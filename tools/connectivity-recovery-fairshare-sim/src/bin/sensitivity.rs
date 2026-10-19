use connectivity_recovery_fairshare_sim::validate_and_simulate;
use serde_json::{Value, json};
use std::error::Error;
use std::io;

const LOAD_BUDGETS_BYTES: [u64; 7] = [
    49_152, 24_576, 16_384, 12_288, 9_830, 8_192, 6_144,
];

fn number(value: &Value, pointer: &str) -> Result<u64, io::Error> {
    value
        .pointer(pointer)
        .and_then(Value::as_u64)
        .ok_or_else(|| io::Error::other(format!("benchmark report is missing numeric field {pointer}")))
}

fn nullable_number(value: &Value, pointer: &str) -> String {
    value
        .pointer(pointer)
        .and_then(Value::as_u64)
        .map_or_else(|| "NA".to_owned(), |number| number.to_string())
}

fn main() -> Result<(), Box<dyn Error>> {
    let scenario_schema: Value = serde_json::from_str(include_str!(
        "../../../../schemas/connectivity-recovery-fairshare-scenario-v1.schema.json"
    ))?;
    let run_schema: Value = serde_json::from_str(include_str!(
        "../../../../schemas/connectivity-task-outcome-run-v1.schema.json"
    ))?;
    let profile_schema: Value = serde_json::from_str(include_str!(
        "../../../../schemas/connectivity-task-outcome-profile-v1.schema.json"
    ))?;
    let profile_bytes = include_bytes!(
        "../../../../profiles/connectivity-task-outcome-benchmark-profile-v1.json"
    )
    .to_vec();
    let profile: Value = serde_json::from_slice(&profile_bytes)?;
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../examples/connectivity-recovery-fairshare-scenario-synthetic-v1.json"
    ))?;

    println!(
        "window_budget_bytes,offered_demand_bytes,offered_load_ppm,mode,baseline_successful_tasks,candidate_successful_tasks,baseline_jain_fairness_ppm,candidate_jain_fairness_ppm,baseline_actual_bytes,candidate_actual_bytes,candidate_allocator_granted_bytes,candidate_allocator_unallocated_bytes"
    );

    for budget in LOAD_BUDGETS_BYTES {
        let mut scenario = fixture.clone();
        scenario["window_budget_bytes"] = json!(budget);
        // Keep the essential reserve at approximately one ninth of each
        // window so sensitivity points compare load, not a fixed reserve ratio.
        scenario["essential_reserve_bytes"] = json!(budget / 9);

        let (_, report) = validate_and_simulate(
            &scenario_schema,
            &scenario,
            &run_schema,
            &profile_schema,
            &profile_bytes,
            &profile,
        )
        .map_err(|errors| io::Error::other(errors.join("; ")))?;

        let windows = report["fair_share_recovery_windows"]
            .as_array()
            .ok_or_else(|| io::Error::other("fair-share window report must be an array"))?;
        for window in windows {
            let mode = window["mode"]
                .as_str()
                .ok_or_else(|| io::Error::other("mode label must be a string"))?;
            let demand = scenario["tasks"]
                .as_array()
                .ok_or_else(|| io::Error::other("task list must be an array"))?
                .iter()
                .filter(|task| task["mode"] == mode)
                .try_fold(0_u64, |total, task| {
                    total.checked_add(task["service_bytes"].as_u64()?)
                })
                .ok_or_else(|| io::Error::other("mode demand is missing or overflows"))?;
            let load_ppm =
                (u128::from(demand) * 1_000_000 / u128::from(budget)) as u64;
            println!(
                "{budget},{demand},{load_ppm},{mode},{},{},{},{},{},{},{},{}",
                number(window, "/baseline/successful_tasks")?,
                number(window, "/candidate_fair_share/successful_tasks")?,
                nullable_number(window, "/baseline/jain_peer_task_success_fairness_ppm"),
                nullable_number(window, "/candidate_fair_share/jain_peer_task_success_fairness_ppm"),
                number(window, "/baseline/actual_transferred_bytes")?,
                number(window, "/candidate_fair_share/actual_transferred_bytes")?,
                number(window, "/candidate_fair_share/allocator_granted_bytes")?,
                number(window, "/candidate_fair_share/allocator_unallocated_bytes")?
            );
        }
    }
    Ok(())
}
