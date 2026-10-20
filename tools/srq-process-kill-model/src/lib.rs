//! Linux process-kill harness for an atomic challenge/policy snapshot.
 //!
 //! This is intentionally a dependency-free contract model. A parent test
 //! SIGKILLs a child process at named persistence boundaries, then reopens the
 //! same directory and requires one complete state. It tests process death on
 //! a running kernel; it is NOT a kernel-crash or physical power-loss test.

use std::collections::BTreeMap;
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const LEGACY_POLICY: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const NEXT_POLICY: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const CHALLENGE: &str = "challenge-2026-0001";
const DECISION: &str = "decision-2026-0001";
const ACTIVE_NAME: &str = "active.state";
const CHECKPOINTS: [&str; 5] = [
    "after_temp_create",
    "during_temp_write",
    "after_temp_sync",
    "after_rename",
    "after_dir_sync",
];

#[derive(Clone, Debug, PartialEq, Eq)]
struct Snapshot {
    generation: u64,
    policy_digest: String,
    last_challenge: String,
    last_decision: String,
}

impl Snapshot {
    fn legacy() -> Self {
        Self {
            generation: 0,
            policy_digest: LEGACY_POLICY.to_owned(),
            last_challenge: String::new(),
            last_decision: String::new(),
        }
    }

    fn promoted_from(current: &Self, challenge: &str, decision: &str) -> Result<Self, String> {
        if current.generation != 0 || !current.last_challenge.is_empty() {
            return Err("challenge was already consumed or generation is not promotable".into());
        }
        if challenge != CHALLENGE || decision != DECISION {
            return Err("challenge or decision does not match the target-bound fixture".into());
        }
        Ok(Self {
            generation: 1,
            policy_digest: NEXT_POLICY.to_owned(),
            last_challenge: challenge.to_owned(),
            last_decision: decision.to_owned(),
        })
    }

    fn validate(&self) -> Result<(), String> {
        let digest_is_sha256 = |value: &str| {
            value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        };
        if !digest_is_sha256(&self.policy_digest) {
            return Err("policy digest is not a lowercase SHA-256-shaped value".into());
        }
        match self.generation {
            0 if self.policy_digest == LEGACY_POLICY
                && self.last_challenge.is_empty()
                && self.last_decision.is_empty() => Ok(()),
            1 if self.policy_digest == NEXT_POLICY
                && self.last_challenge == CHALLENGE
                && self.last_decision == DECISION => Ok(()),
            _ => Err("snapshot is neither the complete previous nor complete promoted state".into()),
        }
    }

    fn encode(&self) -> Vec<u8> {
        format!(
            "schema=1\ngeneration={}\npolicy_digest={}\nlast_challenge={}\nlast_decision={}\n",
            self.generation, self.policy_digest, self.last_challenge, self.last_decision
        )
        .into_bytes()
    }

    fn decode(bytes: &[u8]) -> Result<Self, String> {
        let text = std::str::from_utf8(bytes).map_err(|_| "snapshot is not UTF-8")?;
        let mut fields = BTreeMap::new();
        for line in text.lines() {
            let (key, value) = line.split_once('=').ok_or("snapshot line lacks '='")?;
            if fields.insert(key, value).is_some() {
                return Err(format!("duplicate snapshot field: {key}"));
            }
        }
        if fields.len() != 5 || fields.get("schema") != Some(&"1") {
            return Err("snapshot schema or field inventory is invalid".into());
        }
        let generation = fields
            .get("generation")
            .ok_or("generation missing")?
            .parse::<u64>()
            .map_err(|_| "generation is invalid")?;
        let snapshot = Self {
            generation,
            policy_digest: fields.get("policy_digest").ok_or("policy_digest missing")?.to_string(),
            last_challenge: fields.get("last_challenge").ok_or("last_challenge missing")?.to_string(),
            last_decision: fields.get("last_decision").ok_or("last_decision missing")?.to_string(),
        };
        snapshot.validate()?;
        Ok(snapshot)
    }
}

fn block_at_checkpoint(checkpoint: &str, marker: &Path) -> ! {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(marker)
        .expect("create process-kill checkpoint marker");
    writeln!(file, "{checkpoint}").expect("write checkpoint marker");
    file.sync_all().expect("sync checkpoint marker");
    drop(file);
    // The parent observes the durable marker and sends SIGKILL to this child.
    loop {
        thread::sleep(Duration::from_millis(25));
    }
}

fn atomic_replace(
    directory: &Path,
    next: &Snapshot,
    checkpoint: Option<&str>,
    marker: Option<&Path>,
) -> Result<(), String> {
    next.validate()?;
    let temporary = directory.join(format!(".txn-{}.tmp", std::process::id()));
    let active = directory.join(ACTIVE_NAME);
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|error| format!("create transaction file: {error}"))?;

    if checkpoint == Some("after_temp_create") {
        block_at_checkpoint(checkpoint.expect("checked"), marker.expect("checkpoint marker"));
    }

    let encoded = next.encode();
    if checkpoint == Some("during_temp_write") {
        let halfway = (encoded.len() / 2).max(1);
        file.write_all(&encoded[..halfway])
            .map_err(|error| format!("write partial snapshot: {error}"))?;
        file.sync_all().map_err(|error| format!("sync partial snapshot: {error}"))?;
        block_at_checkpoint(checkpoint.expect("checked"), marker.expect("checkpoint marker"));
    }

    file.write_all(&encoded)
        .map_err(|error| format!("write full snapshot: {error}"))?;
    file.sync_all().map_err(|error| format!("sync transaction file: {error}"))?;
    if checkpoint == Some("after_temp_sync") {
        block_at_checkpoint(checkpoint.expect("checked"), marker.expect("checkpoint marker"));
    }
    drop(file);

    fs::rename(&temporary, &active).map_err(|error| format!("atomically replace active state: {error}"))?;
    if checkpoint == Some("after_rename") {
        block_at_checkpoint(checkpoint.expect("checked"), marker.expect("checkpoint marker"));
    }

    File::open(directory)
        .and_then(|directory_file| directory_file.sync_all())
        .map_err(|error| format!("sync parent directory: {error}"))?;
    if checkpoint == Some("after_dir_sync") {
        block_at_checkpoint(checkpoint.expect("checked"), marker.expect("checkpoint marker"));
    }
    Ok(())
}

fn initialize(directory: &Path) -> Result<(), String> {
    fs::create_dir_all(directory).map_err(|error| format!("create state directory: {error}"))?;
    atomic_replace(directory, &Snapshot::legacy(), None, None)
}

fn recover(directory: &Path) -> Result<Snapshot, String> {
    let active_path = directory.join(ACTIVE_NAME);
    let bytes = fs::read(&active_path).map_err(|error| format!("read active snapshot: {error}"))?;
    let snapshot = Snapshot::decode(&bytes)?;

    // Only the active, validated snapshot is authoritative. Uncommitted temp
    // files cannot cause promotion; clean them after choosing the active state.
    let mut removed = false;
    for entry in fs::read_dir(directory).map_err(|error| format!("scan state directory: {error}"))? {
        let entry = entry.map_err(|error| format!("read state directory entry: {error}"))?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with(".txn-") && name.ends_with(".tmp") {
            fs::remove_file(entry.path()).map_err(|error| format!("remove abandoned temp: {error}"))?;
            removed = true;
        }
    }
    if removed {
        File::open(directory)
            .and_then(|directory_file| directory_file.sync_all())
            .map_err(|error| format!("sync recovery cleanup: {error}"))?;
    }
    Ok(snapshot)
}

fn unique_temp(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after Unix epoch")
        .as_nanos();
    env::temp_dir().join(format!("srq-process-kill-{label}-{}-{nanos}", std::process::id()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crash_worker_entry() {
        let Ok(directory) = env::var("SRQ_PROCESS_KILL_DIR") else {
            return;
        };
        let checkpoint = env::var("SRQ_PROCESS_KILL_CHECKPOINT").expect("checkpoint env");
        let marker = env::var("SRQ_PROCESS_KILL_MARKER").expect("marker env");
        let directory = PathBuf::from(directory);
        let marker = PathBuf::from(marker);
        let before = recover(&directory).expect("worker recovers initial state");
        assert_eq!(before, Snapshot::legacy());
        let next = Snapshot::promoted_from(&before, CHALLENGE, DECISION).expect("valid promotion");
        atomic_replace(&directory, &next, Some(&checkpoint), Some(&marker))
            .expect("child should remain alive at injected checkpoint only");
        panic!("child unexpectedly crossed its designated kill checkpoint");
    }

    #[test]
    fn sigkill_at_each_persistence_boundary_recovers_old_or_complete_new_snapshot() {
        assert_eq!(CHECKPOINTS.len(), 5);
        let executable = env::current_exe().expect("current test executable");

        for checkpoint in CHECKPOINTS {
            let directory = unique_temp(checkpoint);
            fs::create_dir_all(&directory).expect("make isolated state directory");
            initialize(&directory).expect("durably initialize previous generation");
            let marker = directory.join("checkpoint.marker");

            let mut child = Command::new(&executable)
                .args(["--exact", "tests::crash_worker_entry", "--nocapture"])
                .env("SRQ_PROCESS_KILL_DIR", &directory)
                .env("SRQ_PROCESS_KILL_CHECKPOINT", checkpoint)
                .env("SRQ_PROCESS_KILL_MARKER", &marker)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .expect("spawn transaction worker");

            let deadline = Instant::now() + Duration::from_secs(10);
            while !marker.exists() {
                if let Some(status) = child.try_wait().expect("inspect child process") {
                    panic!("worker exited before checkpoint {checkpoint}: {status}");
                }
                assert!(Instant::now() < deadline, "worker did not reach checkpoint {checkpoint}");
                thread::sleep(Duration::from_millis(5));
            }

            let killed = Command::new("kill")
                .args(["-KILL", &child.id().to_string()])
                .status()
                .expect("invoke Linux kill utility");
            assert!(killed.success(), "SIGKILL failed at {checkpoint}");
            let status = child.wait().expect("wait for killed worker");
            assert!(!status.success(), "worker must be terminated at checkpoint {checkpoint}");

            let reopened = recover(&directory).unwrap_or_else(|error| {
                panic!("recovery failed after SIGKILL at {checkpoint}: {error}")
            });
            let previous = Snapshot::legacy();
            let promoted = Snapshot::promoted_from(&previous, CHALLENGE, DECISION).unwrap();
            assert!(
                reopened == previous || reopened == promoted,
                "recovery exposed a mixed state at {checkpoint}: {reopened:?}"
            );
            if matches!(checkpoint, "after_temp_create" | "during_temp_write" | "after_temp_sync") {
                assert_eq!(reopened, previous, "unrenamed state must remain inactive at {checkpoint}");
            }
            if matches!(checkpoint, "after_rename" | "after_dir_sync") {
                assert_eq!(reopened, promoted, "visible atomic replacement must be complete at {checkpoint}");
                assert!(
                    Snapshot::promoted_from(&reopened, CHALLENGE, DECISION).is_err(),
                    "a committed challenge must not be accepted again after reopening"
                );
            }

            let orphaned_temps: Vec<_> = fs::read_dir(&directory)
                .expect("read directory after recovery")
                .map(|entry| entry.expect("directory entry").file_name().to_string_lossy().to_string())
                .filter(|name| name.starts_with(".txn-") && name.ends_with(".tmp"))
                .collect();
            assert!(orphaned_temps.is_empty(), "recovery left orphan transaction files at {checkpoint}");
            println!("PASS: SIGKILL at {checkpoint} recovered generation {}", reopened.generation);
            fs::remove_dir_all(&directory).expect("remove model fixture");
        }
    }

    #[test]
    fn incomplete_or_mixed_snapshots_fail_closed() {
        let partial = b"schema=1\ngeneration=1\npolicy_digest=bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\n";
        assert!(Snapshot::decode(partial).is_err());
        let mixed = format!(
            "schema=1\ngeneration=1\npolicy_digest={}\nlast_challenge={}\nlast_decision=\n",
            NEXT_POLICY, CHALLENGE
        );
        assert!(Snapshot::decode(mixed.as_bytes()).is_err());
        let duplicate = b"schema=1\ngeneration=0\ngeneration=1\npolicy_digest=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\nlast_challenge=\nlast_decision=\n";
        assert!(Snapshot::decode(duplicate).is_err());
    }
}
