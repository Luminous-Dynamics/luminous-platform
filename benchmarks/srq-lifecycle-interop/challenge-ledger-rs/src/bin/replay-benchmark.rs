use srq_challenge_ledger::{
    Binding, Challenge, ChallengeLedger, LedgerError, MAX_CHALLENGE_LIFETIME_SECS,
};
use std::error::Error;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    if let Err(error) = run() {
        eprintln!("BLOCKED: {error}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    const SUBMISSIONS: usize = 64;
    let nonce_time = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let private_dir = std::env::temp_dir().join(format!(
        "srq-challenge-benchmark-{}-{nonce_time:x}",
        std::process::id()
    ));
    fs::create_dir(&private_dir)?;
    fs::set_permissions(&private_dir, fs::Permissions::from_mode(0o700))?;
    let path: PathBuf = private_dir.join("ledger.log");

    let challenge = Challenge {
        // Synthetic test fixture only. Production callers must provide CSPRNG bytes.
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
    };

    let ledger = ChallengeLedger::initialize(&path)?;
    ledger.issue(&challenge, 1_001)?;
    let ledger = Arc::new(ledger);
    let barrier = Arc::new(Barrier::new(SUBMISSIONS));
    let mut workers = Vec::with_capacity(SUBMISSIONS);
    for _ in 0..SUBMISSIONS {
        let ledger = Arc::clone(&ledger);
        let barrier = Arc::clone(&barrier);
        let challenge = challenge.clone();
        workers.push(thread::spawn(move || {
            barrier.wait();
            ledger.consume(&challenge.id, &challenge.binding, 1_002)
        }));
    }

    let mut accepted = 0usize;
    let mut replay_rejections = 0usize;
    let mut unexpected_errors = 0usize;
    for worker in workers {
        match worker.join().map_err(|_| "worker panicked")? {
            Ok(()) => accepted += 1,
            Err(LedgerError::AlreadyConsumed) => replay_rejections += 1,
            Err(_) => unexpected_errors += 1,
        }
    }

    drop(ledger);
    let reopened = ChallengeLedger::open(&path)?;
    let persisted_replay_rejected = matches!(
        reopened.consume(&challenge.id, &challenge.binding, 1_003),
        Err(LedgerError::AlreadyConsumed)
    );
    drop(reopened);

    let status = if accepted == 1
        && replay_rejections == SUBMISSIONS - 1
        && unexpected_errors == 0
        && persisted_replay_rejected
    {
        "PASS"
    } else {
        "FAIL"
    };

    println!(
        "{{\"schema\":\"srq-challenge-replay-benchmark-v1\",\"status\":\"{status}\",\"concurrent_submissions\":{SUBMISSIONS},\"accepted_submissions\":{accepted},\"replay_rejections\":{replay_rejections},\"unexpected_errors\":{unexpected_errors},\"persisted_replay_rejected\":{persisted_replay_rejected},\"qualification_status\":\"BLOCKED\",\"claim_ceiling\":\"none\"}}"
    );

    fs::remove_dir_all(&private_dir)?;
    if status == "PASS" {
        Ok(())
    } else {
        Err("challenge replay benchmark did not meet its acceptance criteria".into())
    }
}
