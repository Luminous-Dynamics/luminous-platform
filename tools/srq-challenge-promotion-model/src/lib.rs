//! SRQ-1 challenge/promotion contract model.
//!
//! Fault injection returns at selected persistence boundaries and then reopens
//! the store. These are deterministic state-boundary simulations, not SIGKILL,
//! kernel crash, power-cut, filesystem fault, or distributed database tests.
//! The single-host flock model does not qualify cross-host transaction semantics.

use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, ErrorKind, Write};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};

#[cfg(not(target_os = "linux"))]
compile_error!("This contract model requires Linux flock semantics.");

unsafe extern "C" {
    fn flock(fd: i32, operation: i32) -> i32;
}

const LOCK_EX: i32 = 2;
const LOCK_UN: i32 = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChallengeSpec {
    pub challenge_id: String,
    pub nonce: String,
    pub target_token: String,
    pub expected_generation: u64,
    pub expected_current_policy_sha256: String,
    pub candidate_policy_sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitCheckpoint {
    BeforeStateWrite,
    AfterStateWriteBeforeSync,
    AfterStateSyncBeforeRename,
    AfterRenameBeforeDirectorySync,
    AfterDirectorySync,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitOutcome {
    Committed,
    InterruptedAt(CommitCheckpoint),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    pub challenge: ChallengeSpec,
    pub active_generation: u64,
    pub active_policy_sha256: String,
    pub challenge_consumed: bool,
    pub decision_id: Option<String>,
}

struct FileLock(File);

impl FileLock {
    fn exclusive(file: File) -> io::Result<Self> {
        loop {
            let result = unsafe { flock(file.as_raw_fd(), LOCK_EX) };
            if result == 0 {
                return Ok(Self(file));
            }
            let error = io::Error::last_os_error();
            if error.kind() != ErrorKind::Interrupted {
                return Err(error);
            }
        }
    }
}

impl Drop for FileLock {
    fn drop(&mut self) {
        let _ = unsafe { flock(self.0.as_raw_fd(), LOCK_UN) };
    }
}

#[derive(Debug, Clone)]
pub struct PromotionStore {
    root: PathBuf,
}

impl PromotionStore {
    pub fn open(root: impl Into<PathBuf>) -> io::Result<Self> {
        let root = root.into();
        fs::create_dir_all(&root)?;
        Ok(Self { root })
    }

    pub fn initialize(&self, challenge: ChallengeSpec) -> io::Result<()> {
        validate_challenge(&challenge)?;
        let _lock = self.acquire_lock()?;
        let state_path = self.root.join("ledger.state");
        if state_path.exists() {
            return Err(io::Error::new(
                ErrorKind::AlreadyExists,
                "challenge ledger is already initialized",
            ));
        }
        let snapshot = Snapshot {
            active_generation: challenge.expected_generation,
            active_policy_sha256: challenge.expected_current_policy_sha256.clone(),
            challenge,
            challenge_consumed: false,
            decision_id: None,
        };
        validate_snapshot(&snapshot)?;
        write_synced(&state_path, serialize_snapshot(&snapshot).as_bytes())?;
        sync_directory(&self.root)
    }

    pub fn snapshot(&self) -> io::Result<Snapshot> {
        let _lock = self.acquire_lock()?;
        self.read_snapshot()
    }

    pub fn consume_and_promote(
        &self,
        presented: &ChallengeSpec,
        decision_id: &str,
        interrupt_at: Option<CommitCheckpoint>,
    ) -> io::Result<CommitOutcome> {
        validate_challenge(presented)?;
        validate_token(decision_id, "decision_id")?;
        let _lock = self.acquire_lock()?;
        let current = self.read_snapshot()?;

        if &current.challenge != presented {
            return Err(io::Error::new(
                ErrorKind::PermissionDenied,
                "challenge, nonce, target, generation, or policy binding mismatch",
            ));
        }
        if current.challenge_consumed {
            return Err(io::Error::new(
                ErrorKind::AlreadyExists,
                "challenge has already been consumed",
            ));
        }
        if current.active_generation != presented.expected_generation
            || current.active_policy_sha256 != presented.expected_current_policy_sha256
        {
            return Err(io::Error::new(
                ErrorKind::InvalidData,
                "active generation/policy differs from challenged state",
            ));
        }

        if interrupt_at == Some(CommitCheckpoint::BeforeStateWrite) {
            return Ok(CommitOutcome::InterruptedAt(CommitCheckpoint::BeforeStateWrite));
        }

        let mut next = current;
        next.active_generation = next.active_generation.checked_add(1).ok_or_else(|| {
            io::Error::new(ErrorKind::InvalidData, "policy generation overflow")
        })?;
        next.active_policy_sha256 = presented.candidate_policy_sha256.clone();
        next.challenge_consumed = true;
        next.decision_id = Some(decision_id.to_owned());
        validate_snapshot(&next)?;

        let temporary = self.root.join("ledger.next");
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&temporary)?;
        file.write_all(serialize_snapshot(&next).as_bytes())?;

        if interrupt_at == Some(CommitCheckpoint::AfterStateWriteBeforeSync) {
            return Ok(CommitOutcome::InterruptedAt(CommitCheckpoint::AfterStateWriteBeforeSync));
        }

        file.sync_all()?;
        drop(file);

        if interrupt_at == Some(CommitCheckpoint::AfterStateSyncBeforeRename) {
            return Ok(CommitOutcome::InterruptedAt(CommitCheckpoint::AfterStateSyncBeforeRename));
        }

        fs::rename(&temporary, self.root.join("ledger.state"))?;

        if interrupt_at == Some(CommitCheckpoint::AfterRenameBeforeDirectorySync) {
            return Ok(CommitOutcome::InterruptedAt(CommitCheckpoint::AfterRenameBeforeDirectorySync));
        }

        sync_directory(&self.root)?;

        if interrupt_at == Some(CommitCheckpoint::AfterDirectorySync) {
            return Ok(CommitOutcome::InterruptedAt(CommitCheckpoint::AfterDirectorySync));
        }

        Ok(CommitOutcome::Committed)
    }

    fn acquire_lock(&self) -> io::Result<FileLock> {
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(self.root.join("ledger.lock"))?;
        FileLock::exclusive(file)
    }

    fn read_snapshot(&self) -> io::Result<Snapshot> {
        let text = fs::read_to_string(self.root.join("ledger.state"))?;
        let snapshot = parse_snapshot(&text)?;
        validate_snapshot(&snapshot)?;
        Ok(snapshot)
    }
}

fn validate_token(value: &str, field: &str) -> io::Result<()> {
    let valid = (3..=96).contains(&value.len())
        && value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        && !value.starts_with('-')
        && !value.ends_with('-');
    if valid {
        Ok(())
    } else {
        Err(io::Error::new(
            ErrorKind::InvalidInput,
            format!("{field} must use 3–96 lowercase ASCII letters, digits, or hyphens"),
        ))
    }
}

fn validate_digest(value: &str, field: &str) -> io::Result<()> {
    if value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        Ok(())
    } else {
        Err(io::Error::new(
            ErrorKind::InvalidInput,
            format!("{field} must be 64 lowercase hexadecimal characters"),
        ))
    }
}

fn validate_challenge(challenge: &ChallengeSpec) -> io::Result<()> {
    validate_token(&challenge.challenge_id, "challenge_id")?;
    validate_token(&challenge.target_token, "target_token")?;
    validate_digest(&challenge.nonce, "nonce")?;
    validate_digest(
        &challenge.expected_current_policy_sha256,
        "expected_current_policy_sha256",
    )?;
    validate_digest(&challenge.candidate_policy_sha256, "candidate_policy_sha256")?;
    if challenge.expected_current_policy_sha256 == challenge.candidate_policy_sha256 {
        return Err(io::Error::new(
            ErrorKind::InvalidInput,
            "candidate policy digest must differ from current policy digest",
        ));
    }
    Ok(())
}

fn validate_snapshot(snapshot: &Snapshot) -> io::Result<()> {
    validate_challenge(&snapshot.challenge)?;
    validate_digest(&snapshot.active_policy_sha256, "active_policy_sha256")?;

    match (snapshot.challenge_consumed, snapshot.decision_id.as_deref()) {
        (false, None) => {
            if snapshot.active_generation != snapshot.challenge.expected_generation
                || snapshot.active_policy_sha256 != snapshot.challenge.expected_current_policy_sha256
            {
                return Err(io::Error::new(
                    ErrorKind::InvalidData,
                    "unconsumed challenge cannot coexist with promoted policy state",
                ));
            }
        }
        (true, Some(decision_id)) => {
            validate_token(decision_id, "decision_id")?;
            let next_generation = snapshot.challenge.expected_generation.checked_add(1).ok_or_else(|| {
                io::Error::new(ErrorKind::InvalidData, "policy generation overflow")
            })?;
            if snapshot.active_generation != next_generation
                || snapshot.active_policy_sha256 != snapshot.challenge.candidate_policy_sha256
            {
                return Err(io::Error::new(
                    ErrorKind::InvalidData,
                    "consumed challenge must be atomic with matching promoted policy state",
                ));
            }
        }
        _ => {
            return Err(io::Error::new(
                ErrorKind::InvalidData,
                "challenge consumption and decision ID must be present together",
            ));
        }
    }
    Ok(())
}

fn serialize_snapshot(snapshot: &Snapshot) -> String {
    format!(
        "schema_version=1\nchallenge_id={}\nnonce={}\ntarget_token={}\nexpected_generation={}\nexpected_current_policy_sha256={}\ncandidate_policy_sha256={}\nactive_generation={}\nactive_policy_sha256={}\nchallenge_consumed={}\ndecision_id={}\n",
        snapshot.challenge.challenge_id,
        snapshot.challenge.nonce,
        snapshot.challenge.target_token,
        snapshot.challenge.expected_generation,
        snapshot.challenge.expected_current_policy_sha256,
        snapshot.challenge.candidate_policy_sha256,
        snapshot.active_generation,
        snapshot.active_policy_sha256,
        if snapshot.challenge_consumed { "true" } else { "false" },
        snapshot.decision_id.as_deref().unwrap_or("none"),
    )
}

fn required<'a>(fields: &'a BTreeMap<String, String>, key: &str) -> io::Result<&'a str> {
    fields.get(key).map(String::as_str).ok_or_else(|| {
        io::Error::new(ErrorKind::InvalidData, format!("snapshot is missing {key}"))
    })
}

fn parse_number(fields: &BTreeMap<String, String>, key: &str) -> io::Result<u64> {
    required(fields, key)?.parse::<u64>().map_err(|_| {
        io::Error::new(ErrorKind::InvalidData, format!("snapshot {key} is not an integer"))
    })
}

fn parse_snapshot(source: &str) -> io::Result<Snapshot> {
    let body = source.strip_suffix('\n').ok_or_else(|| {
        io::Error::new(ErrorKind::InvalidData, "snapshot must end with one newline")
    })?;
    let mut fields = BTreeMap::new();
    for line in body.split('\n') {
        let (key, value) = line.split_once('=').ok_or_else(|| {
            io::Error::new(ErrorKind::InvalidData, "snapshot line lacks key=value separator")
        })?;
        if key.is_empty() || value.is_empty() || fields.insert(key.to_owned(), value.to_owned()).is_some() {
            return Err(io::Error::new(ErrorKind::InvalidData, "snapshot has empty or duplicate fields"));
        }
    }
    if fields.len() != 11 || required(&fields, "schema_version")? != "1" {
        return Err(io::Error::new(ErrorKind::InvalidData, "snapshot schema/version is invalid"));
    }
    let decision_value = required(&fields, "decision_id")?;
    let decision_id = if decision_value == "none" {
        None
    } else {
        Some(decision_value.to_owned())
    };
    let consumed = match required(&fields, "challenge_consumed")? {
        "true" => true,
        "false" => false,
        _ => return Err(io::Error::new(ErrorKind::InvalidData, "challenge_consumed is not boolean")),
    };

    Ok(Snapshot {
        challenge: ChallengeSpec {
            challenge_id: required(&fields, "challenge_id")?.to_owned(),
            nonce: required(&fields, "nonce")?.to_owned(),
            target_token: required(&fields, "target_token")?.to_owned(),
            expected_generation: parse_number(&fields, "expected_generation")?,
            expected_current_policy_sha256: required(&fields, "expected_current_policy_sha256")?.to_owned(),
            candidate_policy_sha256: required(&fields, "candidate_policy_sha256")?.to_owned(),
        },
        active_generation: parse_number(&fields, "active_generation")?,
        active_policy_sha256: required(&fields, "active_policy_sha256")?.to_owned(),
        challenge_consumed: consumed,
        decision_id,
    })
}

fn write_synced(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = File::create(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

fn sync_directory(path: &Path) -> io::Result<()> {
    File::open(path)?.sync_all()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::process::{Command, Stdio};
    use std::sync::{Arc, Barrier};
    use std::thread;
    use std::time::{Duration, Instant};

    static NEXT_ID: AtomicU64 = AtomicU64::new(0);

    struct Sandbox(PathBuf);

    impl Sandbox {
        fn new() -> Self {
            loop {
                let serial = NEXT_ID.fetch_add(1, Ordering::Relaxed);
                let path = std::env::temp_dir().join(format!(
                    "srq-challenge-model-{}-{serial}",
                    std::process::id()
                ));
                match fs::create_dir(&path) {
                    Ok(()) => return Self(path),
                    Err(error) if error.kind() == ErrorKind::AlreadyExists => continue,
                    Err(error) => panic!("create sandbox: {error}"),
                }
            }
        }

        fn store(&self) -> PromotionStore {
            PromotionStore::open(self.0.clone()).expect("open store")
        }
    }

    impl Drop for Sandbox {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn spec() -> ChallengeSpec {
        ChallengeSpec {
            challenge_id: "challenge-004".to_owned(),
            nonce: "c".repeat(64),
            target_token: "target-lab-01".to_owned(),
            expected_generation: 7,
            expected_current_policy_sha256: "a".repeat(64),
            candidate_policy_sha256: "b".repeat(64),
        }
    }

    fn seeded_store(sandbox: &Sandbox) -> PromotionStore {
        let store = sandbox.store();
        store.initialize(spec()).expect("initialize challenge ledger");
        store
    }

    #[test]
    fn challenge_consumption_and_promotion_are_one_durable_state_transition() {
        let sandbox = Sandbox::new();
        let store = seeded_store(&sandbox);
        assert_eq!(
            store.consume_and_promote(&spec(), "decision-001", None).expect("promotion"),
            CommitOutcome::Committed
        );

        let reopened = sandbox.store();
        let snapshot = reopened.snapshot().expect("read state after reopen");
        assert!(snapshot.challenge_consumed);
        assert_eq!(snapshot.decision_id.as_deref(), Some("decision-001"));
        assert_eq!(snapshot.active_generation, 8);
        assert_eq!(snapshot.active_policy_sha256, "b".repeat(64));
    }

    #[test]
    fn sixty_four_concurrent_attempts_for_one_challenge_commit_exactly_once() {
        const WORKERS: usize = 64;
        let sandbox = Sandbox::new();
        let store = seeded_store(&sandbox);
        let barrier = Arc::new(Barrier::new(WORKERS));
        let mut handles = Vec::with_capacity(WORKERS);

        for index in 0..WORKERS {
            let root = store.root.clone();
            let barrier = Arc::clone(&barrier);
            let presented = spec();
            handles.push(thread::spawn(move || {
                let store = PromotionStore::open(root).expect("open worker store");
                barrier.wait();
                store.consume_and_promote(&presented, &format!("decision-{index:03}"), None)
            }));
        }

        let mut committed = 0usize;
        let mut rejected = 0usize;
        for handle in handles {
            match handle.join().expect("worker thread") {
                Ok(CommitOutcome::Committed) => committed += 1,
                Err(error) if error.kind() == ErrorKind::AlreadyExists => rejected += 1,
                other => panic!("unexpected concurrent result: {other:?}"),
            }
        }
        assert_eq!(committed, 1, "exactly one promotion can consume a nonce");
        assert_eq!(rejected, WORKERS - 1);
        let snapshot = sandbox.store().snapshot().expect("persisted snapshot");
        assert!(snapshot.challenge_consumed);
        assert_eq!(snapshot.active_generation, 8);
        assert_eq!(snapshot.active_policy_sha256, "b".repeat(64));
    }

    #[test]
    fn eight_independent_processes_share_one_challenge_commit() {
        const WORKERS: usize = 8;
        let sandbox = Sandbox::new();
        let store = seeded_store(&sandbox);
        let executable = std::env::current_exe().expect("test executable path");
        let mut children = Vec::with_capacity(WORKERS);

        for index in 0..WORKERS {
            let child = Command::new(&executable)
                .arg("--exact")
                .arg("tests::subprocess_promotion_worker")
                .arg("--nocapture")
                .env("SRQ_MODEL_WORKER_ROOT", &sandbox.0)
                .env("SRQ_MODEL_WORKER_INDEX", index.to_string())
                .stdout(Stdio::null())
                .stderr(Stdio::inherit())
                .spawn()
                .expect("spawn independent worker process");
            children.push(child);
        }

        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            let ready = (0..WORKERS).all(|index| sandbox.0.join(format!("ready-{index}")).exists());
            if ready {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "timed out waiting for subprocess workers to reach the shared gate"
            );
            thread::sleep(Duration::from_millis(5));
        }

        fs::write(sandbox.0.join("start-gate"), b"go").expect("release concurrent workers");
        for mut child in children {
            assert!(child.wait().expect("wait for worker process").success());
        }

        let mut committed = 0usize;
        let mut rejected = 0usize;
        for index in 0..WORKERS {
            match fs::read_to_string(sandbox.0.join(format!("result-{index}")))
                .expect("worker result file")
                .as_str()
            {
                "committed" => committed += 1,
                "rejected" => rejected += 1,
                other => panic!("unexpected worker outcome: {other}"),
            }
        }
        assert_eq!(committed, 1, "flock must serialize independent processes");
        assert_eq!(rejected, WORKERS - 1);
        let snapshot = sandbox.store().snapshot().expect("durable state after processes exit");
        assert!(snapshot.challenge_consumed);
        assert_eq!(snapshot.active_generation, 8);
    }

    #[test]
    fn subprocess_promotion_worker() {
        let Ok(root) = std::env::var("SRQ_MODEL_WORKER_ROOT") else {
            return;
        };
        let index = std::env::var("SRQ_MODEL_WORKER_INDEX")
            .expect("worker index")
            .parse::<usize>()
            .expect("integer worker index");
        let root = PathBuf::from(root);
        fs::write(root.join(format!("ready-{index}")), b"ready")
            .expect("mark worker ready");

        let deadline = Instant::now() + Duration::from_secs(15);
        while !root.join("start-gate").exists() {
            assert!(
                Instant::now() < deadline,
                "timed out waiting for concurrent worker release"
            );
            thread::sleep(Duration::from_millis(5));
        }

        let store = PromotionStore::open(root.clone()).expect("open worker store");
        let outcome = store.consume_and_promote(
            &spec(),
            &format!("process-decision-{index:03}"),
            None,
        );
        let status = match outcome {
            Ok(CommitOutcome::Committed) => "committed",
            Err(error) if error.kind() == ErrorKind::AlreadyExists => "rejected",
            other => panic!("unexpected worker result: {other:?}"),
        };
        fs::write(root.join(format!("result-{index}")), status.as_bytes())
            .expect("persist worker outcome");
    }

    #[test]
    fn replay_after_restart_is_rejected() {
        let sandbox = Sandbox::new();
        seeded_store(&sandbox)
            .consume_and_promote(&spec(), "decision-001", None)
            .expect("first promotion");

        let error = sandbox.store()
            .consume_and_promote(&spec(), "decision-replay", None)
            .expect_err("replay must fail after reopen");
        assert_eq!(error.kind(), ErrorKind::AlreadyExists);
    }

    #[test]
    fn interruption_boundaries_preserve_old_or_fully_promoted_state() {
        const TRIALS_PER_CHECKPOINT: usize = 20;
        let checkpoints = [
            CommitCheckpoint::BeforeStateWrite,
            CommitCheckpoint::AfterStateWriteBeforeSync,
            CommitCheckpoint::AfterStateSyncBeforeRename,
            CommitCheckpoint::AfterRenameBeforeDirectorySync,
            CommitCheckpoint::AfterDirectorySync,
        ];

        for checkpoint in checkpoints {
            for trial in 0..TRIALS_PER_CHECKPOINT {
                let sandbox = Sandbox::new();
                let store = seeded_store(&sandbox);
                let result = store
                    .consume_and_promote(&spec(), "decision-002", Some(checkpoint))
                    .expect("injected interruption outcome");
                assert_eq!(result, CommitOutcome::InterruptedAt(checkpoint));

                let snapshot = sandbox.store().snapshot().expect("reopen after interruption");
                let should_be_committed = matches!(
                    checkpoint,
                    CommitCheckpoint::AfterRenameBeforeDirectorySync
                        | CommitCheckpoint::AfterDirectorySync
                );
                assert_eq!(
                    snapshot.challenge_consumed, should_be_committed,
                    "consumption state at {checkpoint:?} trial {trial}"
                );
                if should_be_committed {
                    assert_eq!(snapshot.active_generation, 8);
                    assert_eq!(snapshot.active_policy_sha256, "b".repeat(64));
                    assert_eq!(snapshot.decision_id.as_deref(), Some("decision-002"));
                } else {
                    assert_eq!(snapshot.active_generation, 7);
                    assert_eq!(snapshot.active_policy_sha256, "a".repeat(64));
                    assert_eq!(snapshot.decision_id, None);
                }
            }
        }
    }

    #[test]
    fn wrong_target_or_policy_binding_does_not_consume_challenge() {
        let sandbox = Sandbox::new();
        let store = seeded_store(&sandbox);
        let mut wrong_target = spec();
        wrong_target.target_token = "target-other".to_owned();
        assert_eq!(
            store.consume_and_promote(&wrong_target, "decision-003", None).unwrap_err().kind(),
            ErrorKind::PermissionDenied
        );

        let mut wrong_policy = spec();
        wrong_policy.candidate_policy_sha256 = "d".repeat(64);
        assert_eq!(
            store.consume_and_promote(&wrong_policy, "decision-004", None).unwrap_err().kind(),
            ErrorKind::PermissionDenied
        );

        let snapshot = store.snapshot().expect("still unconsumed");
        assert!(!snapshot.challenge_consumed);
        assert_eq!(snapshot.active_generation, 7);
    }

    #[test]
    fn malformed_ledger_fails_closed_without_reinitialization() {
        let sandbox = Sandbox::new();
        let store = seeded_store(&sandbox);
        fs::write(store.root.join("ledger.state"), "schema_version=1\nchallenge_consumed=true\n")
            .expect("corrupt ledger fixture");

        let error = store
            .consume_and_promote(&spec(), "decision-005", None)
            .expect_err("corrupt ledger must block promotion");
        assert_eq!(error.kind(), ErrorKind::InvalidData);
        let current = fs::read_to_string(store.root.join("ledger.state")).expect("preserved bytes");
        assert_eq!(current, "schema_version=1\nchallenge_consumed=true\n");
    }

    #[test]
    fn second_initialization_cannot_overwrite_challenge_state() {
        let sandbox = Sandbox::new();
        let store = seeded_store(&sandbox);
        let error = store.initialize(spec()).expect_err("no silent replacement");
        assert_eq!(error.kind(), ErrorKind::AlreadyExists);
        assert!(!store.snapshot().expect("existing snapshot").challenge_consumed);
    }
}
