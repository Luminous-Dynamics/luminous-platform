use srq_challenge_ledger::{
    Binding, Challenge, ChallengeLedger, LedgerError, MAX_CHALLENGE_LIFETIME_SECS,
};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const WORKERS: usize = 64;

struct PrivateTempDir(PathBuf);

impl PrivateTempDir {
    fn new() -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before Unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "srq-challenge-process-test-{}-{now:x}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("create private test directory");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700))
            .expect("chmod private test directory");
        Self(path)
    }

    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for PrivateTempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn sample_challenge() -> Challenge {
    Challenge {
        id: "a".repeat(64),
        binding: Binding {
            transaction_id: "b".repeat(64),
            target_digest: "c".repeat(64),
            plan_digest: "d".repeat(64),
            artifact_digest: "e".repeat(64),
            policy_digest: "f".repeat(64),
        },
        issued_at_unix_secs: 1_000,
        expires_at_unix_secs: 1_000 + MAX_CHALLENGE_LIFETIME_SECS,
    }
}

fn stop_children(children: &mut [Child]) {
    for child in children.iter_mut() {
        let _ = child.kill();
    }
    for child in children.iter_mut() {
        let _ = child.wait();
    }
}

#[test]
fn concurrent_processes_can_consume_one_challenge_exactly_once() {
    let dir = PrivateTempDir::new();
    let ledger_path = dir.path("ledger.log");
    let start_path = dir.path("START");
    let challenge = sample_challenge();
    let ledger = ChallengeLedger::initialize(&ledger_path).expect("initialize ledger");
    ledger.issue(&challenge, 1_001).expect("issue challenge");
    drop(ledger);

    let ready_paths: Vec<PathBuf> = (0..WORKERS)
        .map(|index| dir.path(&format!("ready-{index:02}")))
        .collect();
    let mut children = Vec::with_capacity(WORKERS);
    for ready_path in &ready_paths {
        let child = Command::new(env!("CARGO_BIN_EXE_replay-benchmark"))
            .arg("--consume-worker")
            .arg(&ledger_path)
            .arg(ready_path)
            .arg(&start_path)
            .spawn()
            .expect("spawn challenge consumer process");
        children.push(child);
    }

    let deadline = Instant::now() + Duration::from_secs(30);
    while !ready_paths.iter().all(|path| path.exists()) {
        if Instant::now() >= deadline {
            stop_children(&mut children);
            panic!("not all consumer processes reached the start barrier");
        }
        thread::sleep(Duration::from_millis(5));
    }

    let mut start_marker = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&start_path)
        .expect("release all consumer processes");
    start_marker.write_all(b"go").expect("write start marker");
    start_marker.sync_all().expect("sync start marker");
    drop(start_marker);

    let mut accepted = 0;
    let mut replay_rejected = 0;
    let mut unexpected = Vec::new();
    for (index, mut child) in children.into_iter().enumerate() {
        match child.wait().expect("wait for consumer process").code() {
            Some(0) => accepted += 1,
            Some(3) => replay_rejected += 1,
            other => unexpected.push((index, other)),
        }
    }
    assert!(unexpected.is_empty(), "unexpected child exits: {unexpected:?}");
    assert_eq!(accepted, 1, "exactly one process must consume the challenge");
    assert_eq!(replay_rejected, WORKERS - 1, "every competing process must be rejected as replay");

    let reopened = ChallengeLedger::open(&ledger_path).expect("reopen consumed journal");
    assert!(matches!(
        reopened.consume(&challenge.id, &challenge.binding, 1_003),
        Err(LedgerError::AlreadyConsumed)
    ));
}

#[test]
fn kill_after_durable_consumption_before_side_effect_keeps_challenge_spent() {
    let dir = PrivateTempDir::new();
    let ledger_path = dir.path("ledger.log");
    let consumed_marker = dir.path("consumed.marker");
    let challenge = sample_challenge();
    let ledger = ChallengeLedger::initialize(&ledger_path).expect("initialize ledger");
    ledger.issue(&challenge, 1_001).expect("issue challenge");
    drop(ledger);

    let mut child = Command::new(env!("CARGO_BIN_EXE_replay-benchmark"))
        .arg("--consume-crash-window")
        .arg(&ledger_path)
        .arg(&consumed_marker)
        .spawn()
        .expect("spawn crash-window worker");

    let deadline = Instant::now() + Duration::from_secs(15);
    while !consumed_marker.exists() {
        if let Some(status) = child.try_wait().expect("poll crash-window worker") {
            panic!("worker exited before the durable-consume marker: {status}");
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("worker did not reach the post-consume, pre-side-effect window");
        }
        thread::sleep(Duration::from_millis(5));
    }

    child.kill().expect("kill worker before side effect");
    let _killed_status = child.wait().expect("wait for killed worker");

    let reopened = ChallengeLedger::open(&ledger_path).expect("reopen journal after worker kill");
    assert!(matches!(
        reopened.consume(&challenge.id, &challenge.binding, 1_003),
        Err(LedgerError::AlreadyConsumed)
    ));
}
