pub mod effect_protocol;

use serde_json::Value;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

pub fn read_json(path: &Path) -> Result<Value, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    Ok(serde_json::from_slice(&bytes)?)
}

pub fn collect_json_files(root: &Path, out: &mut Vec<PathBuf>) -> Result<(), Box<dyn Error>> {
    let metadata = fs::symlink_metadata(root)?;
    if metadata.file_type().is_symlink() {
        return Err(format!("refusing to traverse symlink fixture root: {}", root.display()).into());
    }
    if !metadata.is_dir() {
        return Err(format!("fixture root is not a directory: {}", root.display()).into());
    }

    collect_json_files_inner(root, out)
}

fn collect_json_files_inner(root: &Path, out: &mut Vec<PathBuf>) -> Result<(), Box<dyn Error>> {
    let mut entries = fs::read_dir(root)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;
    entries.sort();

    for path in entries {
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink() {
            return Err(format!("refusing to traverse symlink in fixture tree: {}", path.display()).into());
        }
        if metadata.is_dir() {
            collect_json_files_inner(&path, out)?;
        } else if metadata.is_file()
            && path.extension().is_some_and(|extension| extension == "json")
        {
            out.push(path);
        }
    }
    Ok(())
}

fn semantic_errors(profile: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    let readiness = profile.pointer("/readiness").and_then(Value::as_str).unwrap_or("");
    let profile_id = profile.pointer("/profile_id").and_then(Value::as_str).unwrap_or("");
    let subject_id = profile.pointer("/subject/id").and_then(Value::as_str).unwrap_or("");
    let subject_hash = profile.pointer("/subject/sha256").and_then(Value::as_str);

    if readiness == "qualified" {
        errors.push(
            "qualified readiness is fail-closed: this tool does not authenticate qualification receipts or independently verify the test subject; use an independently trusted verifier before making this claim"
                .to_owned(),
        );

        let records = profile.pointer("/qualification/records").and_then(Value::as_array);
        let matching_pass = records.into_iter().flatten().any(|record| {
            record.get("result").and_then(Value::as_str) == Some("pass")
                && record.get("profile_id").and_then(Value::as_str) == Some(profile_id)
                && record.get("subject_sha256").and_then(Value::as_str) == subject_hash
        });

        if profile_id.is_empty() || subject_id.is_empty() {
            errors.push("qualified profile must identify both profile_id and subject.id".to_owned());
        }
        if subject_hash.is_none() {
            errors.push("qualified profile must bind an exact subject.sha256".to_owned());
        }
        if !matching_pass {
            errors.push(
                "qualified profile requires a passing receipt bound to this profile_id and exact subject.sha256"
                    .to_owned(),
            );
        }
    }

    for capability in profile.pointer("/capabilities").and_then(Value::as_array).into_iter().flatten() {
        let state = capability.get("state").and_then(Value::as_str).unwrap_or("");
        if state == "qualified" && readiness != "qualified" {
            errors.push("a qualified capability requires qualified readiness and an independently trusted verifier".to_owned());
        }
    }

    if readiness == "design_only" {
        if profile.pointer("/subject/sha256") != Some(&Value::Null) {
            errors.push("design_only profile must not claim a hashed implementation subject".to_owned());
        }
        let records_len = profile
            .pointer("/qualification/records")
            .and_then(Value::as_array)
            .map_or(0, Vec::len);
        if records_len != 0 {
            errors.push("design_only profile must not carry qualification records".to_owned());
        }
        for capability in profile
            .pointer("/capabilities")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let state = capability.get("state").and_then(Value::as_str).unwrap_or("");
            if state == "qualified" || state == "implemented_unqualified" {
                errors.push(format!("design_only profile cannot claim capability state {state}"));
            }
        }
        for level in ["hardware", "firmware", "software", "documentation"] {
            let current = profile
                .pointer(&format!("/openness/{level}/current_level"))
                .and_then(Value::as_str);
            if current != Some("O0") {
                errors.push(format!("design_only profile must report current openness {level}=O0"));
            }
        }
        for field in [
            "/hardware/sku_selected",
            "/hardware/bom_frozen",
            "/hardware/prototype_exists",
            "/hardware/measured_benchmarks",
            "/authority/physical_stop_verified",
            "/recovery/manual_recovery_documented",
            "/recovery/backup_restore_tested",
        ] {
            if profile.pointer(field).and_then(Value::as_bool) == Some(true) {
                errors.push(format!("design_only profile cannot assert {field}=true"));
            }
        }
    }

    let product_class = profile.pointer("/product_class").and_then(Value::as_str).unwrap_or("");
    if matches!(product_class, "robotics_lab" | "robotics_platform")
        && profile.pointer("/authority/physical_stop_required").and_then(Value::as_bool) != Some(true)
    {
        errors.push("robotics profiles must require an independent physical stop".to_owned());
    }

    errors
}

pub fn validate_schema(schema: &Value) -> Result<(), Vec<String>> {
    if !jsonschema::draft202012::meta::is_valid(schema) {
        return Err(vec!["schema is not a valid JSON Schema Draft 2020-12 document".to_owned()]);
    }
    jsonschema::draft202012::options()
        .should_validate_formats(true)
        .build(schema)
        .map(|_| ())
        .map_err(|error| vec![format!("schema compilation failed: {error}")])
}

pub fn validate_structural_document(schema: &Value, document: &Value) -> Result<(), Vec<String>> {
    validate_schema(schema)?;
    let validator = jsonschema::draft202012::options()
        .should_validate_formats(true)
        .build(schema)
        .map_err(|error| vec![format!("schema compilation failed: {error}")])?;

    let errors = validator
        .iter_errors(document)
        .map(|error| format!("schema instance path '{}', schema path '{}': {}", error.instance_path.as_str(), error.schema_path.as_str(), error))
        .collect::<Vec<_>>();

    if errors.is_empty() { Ok(()) } else { Err(errors) }
}

pub fn validate_document(schema: &Value, profile: &Value) -> Result<(), Vec<String>> {
    let mut errors = validate_structural_document(schema, profile).err().unwrap_or_default();
    errors.extend(semantic_errors(profile));

    if errors.is_empty() { Ok(()) } else { Err(errors) }
}

fn capability_semantic_errors(contract: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    let readiness = contract.pointer("/readiness").and_then(Value::as_str).unwrap_or("");
    let mode = contract.pointer("/authority/mode").and_then(Value::as_str).unwrap_or("");
    let physical_effects = contract.pointer("/permissions/physical_effects").and_then(Value::as_str).unwrap_or("");
    let egress_policy = contract.pointer("/network/egress_policy").and_then(Value::as_str).unwrap_or("");
    let transfer_policy = contract.pointer("/data/external_transfer_policy").and_then(Value::as_str).unwrap_or("");

    // No capability manifest can qualify itself. This stays fail-closed until a
    // separately reviewed verifier authenticates and evaluates external evidence.
    if readiness == "qualified" {
        errors.push(
            "qualified capability contract is fail-closed: this validator does not authenticate evidence or apply an independent qualification policy"
                .to_owned(),
        );
    }

    if physical_effects == "bounded_dispatch" && !matches!(mode, "bounded_execution" | "physical_effect_request") {
        errors.push("bounded physical dispatch requires a bounded_execution or physical_effect_request authority mode".to_owned());
    }
    if mode == "physical_effect_request" && physical_effects == "none" {
        errors.push("physical_effect_request authority conflicts with permissions.physical_effects=none".to_owned());
    }
    if transfer_policy == "required_for_declared_function" && egress_policy == "deny" {
        errors.push("required external transfer conflicts with network egress_policy=deny".to_owned());
    }

    errors
}

pub fn validate_capability_contract(schema: &Value, contract: &Value) -> Result<(), Vec<String>> {
    let mut errors = validate_structural_document(schema, contract).err().unwrap_or_default();
    errors.extend(capability_semantic_errors(contract));

    if errors.is_empty() { Ok(()) } else { Err(errors) }
}

fn check_directory_with(
    schema: &Value,
    root: &Path,
    should_be_valid: bool,
    validator: fn(&Value, &Value) -> Result<(), Vec<String>>,
) -> Result<usize, Box<dyn Error>> {
    let mut paths = Vec::new();
    collect_json_files(root, &mut paths)?;
    if paths.is_empty() {
        return Err(format!("{} contains no JSON fixtures", root.display()).into());
    }

    let mut failures = 0;
    for path in &paths {
        let document = match read_json(path) {
            Ok(document) => document,
            Err(error) => {
                eprintln!("ERROR {}: malformed fixture JSON: {error}", path.display());
                failures += 1;
                continue;
            }
        };
        let validation = validator(schema, &document);
        match (should_be_valid, validation) {
            (true, Ok(())) => println!("VALID  {}", path.display()),
            (false, Err(errors)) => {
                println!("REJECT {}", path.display());
                for error in errors {
                    println!("  reason: {error}");
                }
            },
            (true, Err(errors)) => {
                eprintln!("ERROR {}:", path.display());
                for error in errors { eprintln!("  - {error}"); }
                failures += 1;
            }
            (false, Ok(())) => {
                eprintln!("ERROR {}: negative fixture unexpectedly passed", path.display());
                failures += 1;
            }
        }
    }

    if failures > 0 {
        return Err(format!("{failures} fixture contract failure(s) in {}", root.display()).into());
    }
    Ok(paths.len())
}

pub fn check_directory(
    schema: &Value,
    root: &Path,
    should_be_valid: bool,
) -> Result<usize, Box<dyn Error>> {
    check_directory_with(schema, root, should_be_valid, validate_document)
}

pub fn check_capability_directory(
    schema: &Value,
    root: &Path,
    should_be_valid: bool,
) -> Result<usize, Box<dyn Error>> {
    check_directory_with(schema, root, should_be_valid, validate_capability_contract)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    const HEARTH_PROFILE: &str =
        include_str!("../../../profiles/home-systems/valid/hearth-node-design-only.json");
    const ROBOTICS_PROFILE: &str =
        include_str!("../../../profiles/home-systems/valid/robotics-joint-lab-design-only.json");
    const SCHEMA: &str = include_str!("../../../schemas/home-system-profile.schema.json");

    fn parsed_profile(raw: &str) -> Value {
        serde_json::from_str(raw).expect("checked-in test profile must parse")
    }

    fn parsed_schema() -> Value {
        serde_json::from_str(SCHEMA).expect("checked-in schema must parse")
    }

    #[test]
    fn design_only_cannot_claim_existing_hardware_or_tested_recovery() {
        let schema = parsed_schema();
        let mutations = [
            ("/hardware/sku_selected", "sku_selected"),
            ("/hardware/bom_frozen", "bom_frozen"),
            ("/hardware/prototype_exists", "prototype_exists"),
            ("/hardware/measured_benchmarks", "measured_benchmarks"),
            ("/authority/physical_stop_verified", "physical_stop_verified"),
            ("/recovery/manual_recovery_documented", "manual_recovery_documented"),
            ("/recovery/backup_restore_tested", "backup_restore_tested"),
        ];

        for (pointer, field) in mutations {
            let mut profile = parsed_profile(HEARTH_PROFILE);
            let target = profile.pointer_mut(pointer).expect("contract field must exist");
            *target = Value::Bool(true);

            let errors = validate_document(&schema, &profile)
                .expect_err(&format!("design_only profile must reject {field}=true"));
            assert!(
                errors.iter().any(|error| error.contains(pointer)),
                "expected a targeted policy error for {field}; got {errors:#?}"
            );
        }
    }

    #[test]
    fn design_only_must_not_promote_current_openness_level() {
        let schema = parsed_schema();
        let mut profile = parsed_profile(HEARTH_PROFILE);
        profile["openness"]["hardware"]["current_level"] = Value::String("O1".into());

        let errors = validate_document(&schema, &profile)
            .expect_err("design-only profile cannot claim current non-zero openness");
        assert!(
            errors.iter().any(|error| error.contains("current openness hardware=O0")),
            "expected targeted openness error; got {errors:#?}"
        );
    }

    #[test]
    fn robotics_profile_requires_independent_physical_stop() {
        let schema = parsed_schema();
        let mut profile = parsed_profile(ROBOTICS_PROFILE);
        profile["authority"]["physical_stop_required"] = Value::Bool(false);

        let errors = validate_document(&schema, &profile)
            .expect_err("robotics profile without a physical stop must be rejected");
        assert!(
            errors.iter().any(|error| error.contains("robotics profiles must require an independent physical stop")),
            "expected targeted physical-stop error; got {errors:#?}"
        );
    }

    #[test]
    fn qualified_capability_cannot_hide_inside_design_only_profile() {
        let schema = parsed_schema();
        let mut profile = parsed_profile(HEARTH_PROFILE);
        profile["capabilities"][0]["state"] = Value::String("qualified".into());

        let errors = validate_document(&schema, &profile)
            .expect_err("design-only profile cannot claim a qualified capability");
        assert!(
            errors.iter().any(|error| error.contains("qualified capability requires qualified readiness")),
            "expected targeted capability-state error; got {errors:#?}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn fixture_traversal_fails_closed_on_symlinked_directories() {
        use std::os::unix::fs::symlink;

        let root = std::env::temp_dir().join(format!(
            "home-profile-validator-symlink-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("target")).expect("create target directory");
        symlink(root.join("target"), root.join("linked-directory")).expect("create symlink");

        let mut paths = Vec::new();
        let result = collect_json_files(&root, &mut paths);
        let _ = fs::remove_dir_all(&root);

        assert!(result.is_err(), "fixture traversal must reject symlinked directories");
        assert!(paths.is_empty(), "no fixture paths should be emitted on traversal failure");
    }

    const CAPABILITY_SCHEMA: &str =
        include_str!("../../../schemas/home-capability-contract.schema.json");
    const VALID_CAPABILITY: &str =
        include_str!("../../../profiles/home-systems/capabilities/hearth-core-local-status-design-only.json");
    const INVALID_CAPABILITY_EGRESS: &str =
        include_str!("../../../profiles/home-systems/capability-invalid/invalid-allowlist-without-destinations.json");
    const INVALID_CAPABILITY_AUTHORITY: &str =
        include_str!("../../../profiles/home-systems/capability-invalid/invalid-critical-effect-without-safeguards.json");
    const INVALID_CAPABILITY_QUALIFIED: &str =
        include_str!("../../../profiles/home-systems/capability-invalid/invalid-qualified-with-fabricated-evidence.json");

    fn parsed_capability_schema() -> Value {
        serde_json::from_str(CAPABILITY_SCHEMA).expect("capability contract schema must parse")
    }

    #[test]
    fn capability_contract_schema_is_valid_draft_2020_12() {
        let schema = parsed_capability_schema();
        assert!(
            jsonschema::draft202012::meta::is_valid(&schema),
            "capability contract schema must validate against Draft 2020-12 meta-schema"
        );
    }

    #[test]
    fn design_only_capability_contract_passes_structural_schema() {
        let schema = parsed_capability_schema();
        let contract = parsed_profile(VALID_CAPABILITY);
        validate_capability_contract(&schema, &contract)
            .expect("design-only capability contract should pass structural and semantic validation");
    }

    #[test]
    fn capability_contract_rejects_empty_egress_allowlist() {
        let schema = parsed_capability_schema();
        let contract = parsed_profile(INVALID_CAPABILITY_EGRESS);
        let errors = validate_capability_contract(&schema, &contract)
            .expect_err("allowlist policy requires at least one declared destination");
        assert!(!errors.is_empty(), "rejection should include a diagnostic");
    }

    #[test]
    fn capability_contract_rejects_critical_effect_without_safeguards() {
        let schema = parsed_capability_schema();
        let contract = parsed_profile(INVALID_CAPABILITY_AUTHORITY);
        let errors = validate_capability_contract(&schema, &contract)
            .expect_err("critical physical effects require authorization, observation, and a stop");
        assert!(!errors.is_empty(), "rejection should include a diagnostic");
    }
    #[test]
    fn qualified_capability_manifest_cannot_self_attest() {
        let schema = parsed_capability_schema();
        let contract = parsed_profile(INVALID_CAPABILITY_QUALIFIED);
        let errors = validate_capability_contract(&schema, &contract)
            .expect_err("a capability manifest cannot self-promote to qualified");
        assert!(
            errors.iter().any(|error| error.contains("qualified capability contract is fail-closed")),
            "expected fail-closed qualification diagnostic; got {errors:#?}"
        );
    }


    const EFFECT_ENVELOPE_SCHEMA: &str =
        include_str!("../../../schemas/home-effect-envelope.schema.json");
    const VALID_EFFECT_ENVELOPE: &str =
        include_str!("../../../profiles/home-systems/effect-requests/hearth-light-proposal-design-only.json");
    const INVALID_EFFECT_DIRECT: &str =
        include_str!("../../../profiles/home-systems/effect-invalid/design-only-direct-effect-request.json");
    const INVALID_EFFECT_BYPASS: &str =
        include_str!("../../../profiles/home-systems/effect-invalid/envelope-claims-direct-dispatch.json");
    const INVALID_EFFECT_ACK_PROOF: &str =
        include_str!("../../../profiles/home-systems/effect-invalid/provider-ack-as-physical-proof.json");

    fn parsed_effect_schema() -> Value {
        serde_json::from_str(EFFECT_ENVELOPE_SCHEMA)
            .expect("home effect envelope schema must parse")
    }

    #[test]
    fn home_effect_envelope_schema_is_valid_draft_2020_12() {
        let schema = parsed_effect_schema();
        assert!(
            jsonschema::draft202012::meta::is_valid(&schema),
            "home effect envelope schema must validate against Draft 2020-12 meta-schema"
        );
        validate_schema(&schema).expect("home effect envelope schema must compile");
    }

    #[test]
    fn design_only_home_effect_proposal_passes_schema() {
        let schema = parsed_effect_schema();
        let envelope = parsed_profile(VALID_EFFECT_ENVELOPE);
        validate_structural_document(&schema, &envelope)
            .expect("design-only proposal envelope should pass structural validation");
    }

    #[test]
    fn effect_envelope_rejects_design_only_direct_effect() {
        let schema = parsed_effect_schema();
        let envelope = parsed_profile(INVALID_EFFECT_DIRECT);
        let errors = validate_structural_document(&schema, &envelope)
            .expect_err("design-only contract must not request a physical/provider effect");
        assert!(
            errors.iter().any(|error| error.contains("mode")),
            "expected the request mode invariant to reject the fixture; got {errors:#?}"
        );
    }

    #[test]
    fn effect_envelope_rejects_direct_dispatch_authority_bypass() {
        let schema = parsed_effect_schema();
        let envelope = parsed_profile(INVALID_EFFECT_BYPASS);
        let errors = validate_structural_document(&schema, &envelope)
            .expect_err("request envelope must not declare a parallel direct-dispatch route");
        assert!(
            errors.iter().any(|error| error.contains("dispatch_route")
                || error.contains("this_envelope_grants_authority")),
            "expected authority-boundary invariant to reject the fixture; got {errors:#?}"
        );
    }

    #[test]
    fn effect_envelope_rejects_provider_ack_as_physical_proof() {
        let schema = parsed_effect_schema();
        let envelope = parsed_profile(INVALID_EFFECT_ACK_PROOF);
        let errors = validate_structural_document(&schema, &envelope)
            .expect_err("provider response must not be treated as proof of physical outcome");
        assert!(
            errors.iter().any(|error| error.contains("service_response_is_physical_proof")),
            "expected physical-proof invariant to reject the fixture; got {errors:#?}"
        );
    }

}
