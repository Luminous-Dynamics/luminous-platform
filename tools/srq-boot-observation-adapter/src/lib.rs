//! D0 local boot observation for NixOS.
//!
//! This crate intentionally has no crypto dependencies and makes no attestation
//! claim. It samples the boot instance ID and the symlink targets of
//! /run/booted-system and /run/current-system. A result can be useful for
//! diagnostics but MUST NOT independently satisfy a release or trust gate.

use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const SCHEMA: &str = "srq-boot-observation-diagnostic-v1";
pub const EVIDENCE_TIER: &str = "D0-local-diagnostic";
pub const AUTHENTICITY: &str = "NOT_AUTHENTICATED";
pub const CONSISTENT: &str = "OBSERVED_CONSISTENT_DIAGNOSTIC";
pub const BLOCKED: &str = "BLOCKED";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Observation {
    pub boot_id: Option<String>,
    pub booted_deployment_id: Option<String>,
    pub current_userspace_deployment_id: Option<String>,
    pub status: String,
    pub reason_codes: Vec<String>,
    pub captured_at_unix_ms: Option<u128>,
}

impl Observation {
    pub fn collect(root: &Path) -> Self {
        let mut reason_codes = Vec::new();

        let boot_id_path = root.join("proc/sys/kernel/random/boot_id");
        let boot_id = match fs::read_to_string(&boot_id_path) {
            Ok(value) => {
                let value = value.trim().to_owned();
                if valid_boot_id(&value) {
                    Some(value)
                } else {
                    reason_codes.push("BOOT_ID_INVALID".to_owned());
                    None
                }
            }
            Err(_) => {
                reason_codes.push("BOOT_ID_UNREADABLE".to_owned());
                None
            }
        };

        let booted_path = root.join("run/booted-system");
        let booted_deployment_id = read_deployment_link(&booted_path, "BOOTED_DEPLOYMENT_UNREADABLE", &mut reason_codes);

        let current_path = root.join("run/current-system");
        let current_userspace_deployment_id =
            read_deployment_link(&current_path, "CURRENT_DEPLOYMENT_UNREADABLE", &mut reason_codes);

        if let (Some(booted), Some(current)) = (
            booted_deployment_id.as_deref(),
            current_userspace_deployment_id.as_deref(),
        ) {
            if booted != current {
                reason_codes.push("BOOTED_CURRENT_DIVERGENCE".to_owned());
            }
        }

        reason_codes.sort();
        reason_codes.dedup();
        let status = if reason_codes.is_empty() {
            CONSISTENT.to_owned()
        } else {
            BLOCKED.to_owned()
        };
        let captured_at_unix_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .map(|duration| duration.as_millis());

        Self {
            boot_id,
            booted_deployment_id,
            current_userspace_deployment_id,
            status,
            reason_codes,
            captured_at_unix_ms,
        }
    }

    /// Serialize a diagnostic record. The output deliberately contains no
    /// field called "verified", "trusted", or "release_pass".
    pub fn to_json(&self) -> String {
        let boot_id = json_optional_string(self.boot_id.as_deref());
        let booted = json_optional_string(self.booted_deployment_id.as_deref());
        let current = json_optional_string(self.current_userspace_deployment_id.as_deref());
        let reasons = self
            .reason_codes
            .iter()
            .map(|reason| json_string(reason))
            .collect::<Vec<_>>()
            .join(",");
        let captured = self
            .captured_at_unix_ms
            .map(|value| value.to_string())
            .unwrap_or_else(|| "null".to_owned());

        format!(
            r#"{{
  "schema": {},
  "evidence_tier": {},
  "authenticity_status": {},
  "status": {},
  "boot_id": {},
  "booted_deployment_id": {},
  "current_userspace_deployment_id": {},
  "promotion_binding_status": "NOT_PROVIDED",
  "measurement_status": "NOT_MEASURED",
  "signature_status": "NOT_AUTHENTICATED",
  "source_paths": {{
    "boot_id": "/proc/sys/kernel/random/boot_id",
    "booted_deployment": "/run/booted-system (readlink target; not a filesystem measurement)",
    "current_userspace_deployment": "/run/current-system (readlink target; not a filesystem measurement)"
  }},
  "reason_codes": [{}],
  "captured_at_unix_ms_untrusted": {}
}}"#,
            json_string(SCHEMA),
            json_string(EVIDENCE_TIER),
            json_string(AUTHENTICITY),
            json_string(&self.status),
            boot_id,
            booted,
            current,
            reasons,
            captured,
        )
    }
}

fn read_deployment_link(path: &Path, unreadable_code: &str, reasons: &mut Vec<String>) -> Option<String> {
    match fs::read_link(path) {
        Ok(target) => {
            let text = target.to_str().map(str::to_owned);
            match text {
                Some(value) if valid_nix_store_path(&value) => Some(value),
                _ => {
                    reasons.push(if target.is_absolute() {
                        "DEPLOYMENT_ID_INVALID".to_owned()
                    } else {
                        "DEPLOYMENT_ID_NOT_ABSOLUTE".to_owned()
                    });
                    None
                }
            }
        }
        Err(_) => {
            reasons.push(unreadable_code.to_owned());
            None
        }
    }
}

fn valid_boot_id(value: &str) -> bool {
    if value.len() != 36 {
        return false;
    }
    for (index, byte) in value.bytes().enumerate() {
        if [8, 13, 18, 23].contains(&index) {
            if byte != b'-' {
                return false;
            }
        } else if !byte.is_ascii_digit() && !(b'a'..=b'f').contains(&byte) {
            return false;
        }
    }
    true
}

fn valid_nix_store_path(value: &str) -> bool {
    if !value.starts_with("/nix/store/") || value.contains('\0') || value.contains('\\') {
        return false;
    }
    let path = PathBuf::from(value);
    let components = path.components().collect::<Vec<_>>();
    if components.len() != 4
        || components[0] != Component::RootDir
        || components[1] != Component::Normal(std::ffi::OsStr::new("nix"))
        || components[2] != Component::Normal(std::ffi::OsStr::new("store"))
    {
        return false;
    }
    let Component::Normal(final_component) = components[3] else {
        return false;
    };
    let Some(component) = final_component.to_str() else {
        return false;
    };
    let Some((hash, name)) = component.split_once('-') else {
        return false;
    };
    hash.len() == 32
        && hash.bytes().all(|byte| b"0123456789abcdfghijklmnpqrsvwxyz".contains(&byte))
        && !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"+._?-".contains(&byte))
}

fn json_optional_string(value: Option<&str>) -> String {
    value.map(json_string).unwrap_or_else(|| "null".to_owned())
}

fn json_string(value: &str) -> String {
    let mut output = String::with_capacity(value.len() + 2);
    output.push('"');
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            ch if ch <= '\u{1f}' => {
                output.push_str(&format!("\\u{:04x}", ch as u32));
            }
            ch => output.push(ch),
        }
    }
    output.push('"');
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;
    use std::time::{SystemTime, UNIX_EPOCH};

    const BOOT_ID: &str = "01234567-89ab-cdef-0123-456789abcdef";
    const STORE_A: &str = "/nix/store/0123456789abcdfghijklmnpqrsvwxyz-nixos-system-test-a";
    const STORE_B: &str = "/nix/store/1123456789abcdfghijklmnpqrsvwxyz-nixos-system-test-b";

    struct Fixture {
        root: PathBuf,
    }

    impl Fixture {
        fn new() -> Self {
            let suffix = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let root = std::env::temp_dir().join(format!("srq-boot-observation-{}-{suffix}", std::process::id()));
            fs::create_dir_all(root.join("proc/sys/kernel/random")).unwrap();
            fs::create_dir_all(root.join("run")).unwrap();
            fs::write(root.join("proc/sys/kernel/random/boot_id"), BOOT_ID).unwrap();
            symlink(STORE_A, root.join("run/booted-system")).unwrap();
            symlink(STORE_A, root.join("run/current-system")).unwrap();
            Self { root }
        }

        fn observe(&self) -> Observation {
            Observation::collect(&self.root)
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn matching_booted_and_current_paths_emit_only_diagnostic_consistency() {
        let fixture = Fixture::new();
        let observation = fixture.observe();
        assert_eq!(observation.status, CONSISTENT);
        assert!(observation.reason_codes.is_empty());
        let json = observation.to_json();
        assert!(json.contains("\"authenticity_status\": \"NOT_AUTHENTICATED\""));
        assert!(json.contains("\"evidence_tier\": \"D0-local-diagnostic\""));
        assert!(json.contains("\"signature_status\": \"NOT_AUTHENTICATED\""));
        assert!(!json.contains("\"release_pass\""));
        assert!(!json.contains("PASS"));
    }

    #[test]
    fn live_userspace_switch_without_reboot_is_blocked() {
        let fixture = Fixture::new();
        fs::remove_file(fixture.root.join("run/current-system")).unwrap();
        symlink(STORE_B, fixture.root.join("run/current-system")).unwrap();
        let observation = fixture.observe();
        assert_eq!(observation.status, BLOCKED);
        assert!(observation.reason_codes.contains(&"BOOTED_CURRENT_DIVERGENCE".to_owned()));
    }

    #[test]
    fn previous_booted_path_is_not_replaced_by_the_expected_current_path() {
        let fixture = Fixture::new();
        fs::remove_file(fixture.root.join("run/current-system")).unwrap();
        symlink(STORE_B, fixture.root.join("run/current-system")).unwrap();
        let observation = fixture.observe();
        assert_eq!(observation.booted_deployment_id.as_deref(), Some(STORE_A));
        assert_eq!(observation.status, BLOCKED);
    }

    #[test]
    fn missing_boot_id_fails_closed() {
        let fixture = Fixture::new();
        fs::remove_file(fixture.root.join("proc/sys/kernel/random/boot_id")).unwrap();
        let observation = fixture.observe();
        assert_eq!(observation.status, BLOCKED);
        assert!(observation.reason_codes.contains(&"BOOT_ID_UNREADABLE".to_owned()));
    }

    #[test]
    fn malformed_boot_id_fails_closed() {
        let fixture = Fixture::new();
        fs::write(fixture.root.join("proc/sys/kernel/random/boot_id"), "not-a-boot-id").unwrap();
        let observation = fixture.observe();
        assert_eq!(observation.status, BLOCKED);
        assert!(observation.reason_codes.contains(&"BOOT_ID_INVALID".to_owned()));
    }

    #[test]
    fn missing_deployment_symlink_fails_closed() {
        let fixture = Fixture::new();
        fs::remove_file(fixture.root.join("run/booted-system")).unwrap();
        let observation = fixture.observe();
        assert_eq!(observation.status, BLOCKED);
        assert!(observation.reason_codes.contains(&"BOOTED_DEPLOYMENT_UNREADABLE".to_owned()));
    }

    #[test]
    fn non_store_and_malformed_store_targets_fail_closed() {
        let fixture = Fixture::new();
        fs::remove_file(fixture.root.join("run/booted-system")).unwrap();
        symlink("/run/evil", fixture.root.join("run/booted-system")).unwrap();
        let observation = fixture.observe();
        assert_eq!(observation.status, BLOCKED);
        assert!(observation.reason_codes.contains(&"DEPLOYMENT_ID_INVALID".to_owned()));
    }

    #[test]
    fn relative_symlink_targets_fail_closed() {
        let fixture = Fixture::new();
        fs::remove_file(fixture.root.join("run/booted-system")).unwrap();
        symlink("../../nix/store/0123456789abcdfghijklmnpqrsvwxyz-nixos-system-test-a", fixture.root.join("run/booted-system")).unwrap();
        let observation = fixture.observe();
        assert_eq!(observation.status, BLOCKED);
        assert!(observation.reason_codes.contains(&"DEPLOYMENT_ID_NOT_ABSOLUTE".to_owned()));
    }

    #[test]
    fn observer_never_supplies_policy_artifact_or_measurement_proof() {
        let fixture = Fixture::new();
        let json = fixture.observe().to_json();
        assert!(json.contains("\"promotion_binding_status\": \"NOT_PROVIDED\""));
        assert!(json.contains("\"measurement_status\": \"NOT_MEASURED\""));
        assert!(json.contains("\"captured_at_unix_ms_untrusted\""));
    }

    #[test]
    fn exactly_one_divergence_reason_is_reported_deterministically() {
        let fixture = Fixture::new();
        fs::remove_file(fixture.root.join("run/current-system")).unwrap();
        symlink(STORE_B, fixture.root.join("run/current-system")).unwrap();
        let observation = fixture.observe();
        assert_eq!(observation.reason_codes, vec!["BOOTED_CURRENT_DIVERGENCE"]);
    }
}
