use srq_challenge_ledger::{
    Binding, Challenge, ChallengeLedger, LedgerError, MAX_CHALLENGE_LIFETIME_SECS,
};
use std::error::Error;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

fn main() {
    if let Err(error) = run() {
        eprintln!("BLOCKED: {error}");
        std::process::exit(2);
    }
}

fn percentile(sorted: &[u128], percentile: usize) -> u128 {
    let rank = (percentile * sorted.len()).div_ceil(100).max(1);
    sorted[rank - 1]
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
    let barrier = Arc::new(Barrier::new(SUBMISSIONS + 1));
    let mut workers = Vec::with_capacity(SUBMISSIONS);
    for _ in 0..SUBMISSIONS {
        let ledger = Arc::clone(&ledger);
        let barrier = Arc::clone(&barrier);
        let challenge = challenge.clone();
        workers.push(thread::spawn(move || {
            barrier.wait();
            let started = Instant::now();
            let result = ledger.consume(&challenge.id, &challenge.binding, 1_002);
            (result, started.elapsed().as_nanos())
        }));
    }

    // Workers are already waiting; measure only the concurrent execution window.
    let wall_started = Instant::now();
    barrier.wait();
    let mut accepted = 0usize;
    let mut replay_rejections = 0usize;
    let mut unexpected_errors = 0usize;
    let mut latencies_ns = Vec::with_capacity(SUBMISSIONS);
    for worker in workers {
        let (result, latency_ns) = worker.join().map_err(|_| "worker panicked")?;
        latencies_ns.push(latency_ns);
        match result {
            Ok(()) => accepted += 1,
            Err(LedgerError::AlreadyConsumed) => replay_rejections += 1,
            Err(_) => unexpected_errors += 1,
        }
    }
    let concurrency_wall_ns = wall_started.elapsed().as_nanos();
    latencies_ns.sort_unstable();
    let p50_ns = percentile(&latencies_ns, 50);
    let p95_ns = percentile(&latencies_ns, 95);
    let p99_ns = percentile(&latencies_ns, 99);
    let throughput_per_second =
        (SUBMISSIONS as f64) * 1_000_000_000.0 / (concurrency_wall_ns as f64);

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
        "{{\"schema\":\"srq-challenge-replay-benchmark-v1\",\"status\":\"{status}\",\"concurrent_submissions\":{SUBMISSIONS},\"accepted_submissions\":{accepted},\"replay_rejections\":{replay_rejections},\"unexpected_errors\":{unexpected_errors},\"persisted_replay_rejected\":{persisted_replay_rejected},\"concurrency_wall_ns\":{concurrency_wall_ns},\"latency_p50_ns\":{p50_ns},\"latency_p95_ns\":{p95_ns},\"latency_p99_ns\":{p99_ns},\"throughput_submissions_per_second\":{throughput_per_second:.3},\"qualification_status\":\"BLOCKED\",\"claim_ceiling\":\"none\"}}"
    );

    fs::remove_dir_all(&private_dir)?;
    if status == "PASS" {
        Ok(())
    } else {
        Err("challenge replay benchmark did not meet its acceptance criteria".into())
    }
}
