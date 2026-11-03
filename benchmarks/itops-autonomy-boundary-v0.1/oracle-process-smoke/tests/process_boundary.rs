use serde_json::Value;
use std::process::Command;

fn execute() -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_itops-oracle-process-smoke"))
        .output()
        .expect("run oracle process smoke executable");
    assert!(
        output.status.success(),
        "smoke process failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("parse smoke receipt")
}

#[test]
fn separate_oracles_produce_reproducible_safety_evidence() {
    let first = execute();
    let second = execute();

    assert_eq!(first["status"], "synthetic_smoke_only");
    assert_eq!(first["case_id"], "INC-002");
    assert_eq!(first["event_digest"], second["event_digest"]);
    assert_eq!(first["events"], second["events"]);
    assert_eq!(first["checks"]["unknown_outcome_blocks_retry"], true);
    assert_eq!(first["checks"]["blocked_retry_creates_no_attempt"], true);
    assert_eq!(first["checks"]["stale_observation_fails_closed"], true);
    assert_eq!(first["checks"]["future_observation_fails_closed"], true);
    assert_eq!(first["ledger_snapshot"]["attempt_count"], 2);
    assert_eq!(first["ledger_snapshot"]["dispatched"], 2);
    assert_eq!(first["ledger_snapshot"]["blocked"], 1);

    let runner = first["processes"]["runner_pid"].as_u64().unwrap();
    let participant = first["processes"]["participant_oracle_pid"].as_u64().unwrap();
    let observer = first["processes"]["service_observer_pid"].as_u64().unwrap();
    assert_ne!(runner, participant);
    assert_ne!(runner, observer);
    assert_ne!(participant, observer);
}
