use crate::replay::{create_replay_envelope, verify_replay_envelope};
use omsi_manufacturing_validator::evaluate_input_value;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::hint::black_box;
use std::time::Instant;

const WARMUP_ITERATIONS_PER_CASE: usize = 5;
const MAX_ITERATIONS_PER_CASE: usize = 100_000;
const MAX_BENCHMARK_CASES: usize = 10_000;
const MAX_TOTAL_SAMPLES_PER_PHASE: usize = 1_000_000;

struct Case {
    id: String,
    input: Value,
    bytes: Vec<u8>,
    envelope: Value,
}

/// Run a local, non-gating microbenchmark over the exact inputs embedded in a
/// conformance-vector document. The result is an observation, not a pass claim.
pub fn run(
    vector_document: &Value,
    vector_document_bytes: &[u8],
    iterations_per_case: usize,
) -> Result<Value, String> {
    if iterations_per_case == 0 || iterations_per_case > MAX_ITERATIONS_PER_CASE {
        return Err(format!(
            "iterations_per_case must be between 1 and {MAX_ITERATIONS_PER_CASE}"
        ));
    }

    let vectors = vector_document
        .get("vectors")
        .and_then(Value::as_array)
        .ok_or_else(|| "benchmark corpus must contain a vectors array".to_owned())?;
    if vectors.is_empty() {
        return Err("benchmark corpus must contain at least one vector".to_owned());
    }
    if vectors.len() > MAX_BENCHMARK_CASES {
        return Err(format!(
            "benchmark corpus has {} cases; maximum is {MAX_BENCHMARK_CASES}",
            vectors.len()
        ));
    }
    let sample_count_per_phase = vectors
        .len()
        .checked_mul(iterations_per_case)
        .ok_or_else(|| "benchmark sample-count arithmetic overflowed".to_owned())?;
    if sample_count_per_phase > MAX_TOTAL_SAMPLES_PER_PHASE {
        return Err(format!(
            "benchmark requests {sample_count_per_phase} samples per phase; maximum is {MAX_TOTAL_SAMPLES_PER_PHASE}"
        ));
    }

    let mut cases = Vec::with_capacity(vectors.len());
    for vector in vectors {
        let id = vector
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| "every benchmark vector must have a string id".to_owned())?
            .to_owned();
        let input = vector
            .get("input")
            .cloned()
            .ok_or_else(|| format!("benchmark vector {id} is missing input"))?;
        let bytes = serde_json::to_vec(&input)
            .map_err(|error| format!("cannot serialize input for {id}: {error}"))?;
        let envelope = create_replay_envelope(&bytes)
            .map_err(|error| format!("cannot create replay envelope for {id}: {error}"))?;
        cases.push(Case {
            id,
            input,
            bytes,
            envelope,
        });
    }

    for _ in 0..WARMUP_ITERATIONS_PER_CASE {
        for case in &cases {
            black_box(
                serde_json::from_slice::<Value>(black_box(case.bytes.as_slice()))
                    .map_err(|error| format!("warmup parse failed for {}: {error}", case.id))?,
            );
            black_box(evaluate_input_value(black_box(&case.input)));
            black_box(
                create_replay_envelope(black_box(case.bytes.as_slice()))
                    .map_err(|error| format!("warmup envelope failed for {}: {error}", case.id))?,
            );
            let verification =
                verify_replay_envelope(black_box(&case.envelope), black_box(case.bytes.as_slice()));
            if verification.get("status").and_then(Value::as_str) != Some("replay_consistent") {
                return Err(format!("warmup replay did not verify for {}", case.id));
            }
            black_box(verification);
        }
    }

    let corpus_input_bytes: usize = cases.iter().map(|case| case.bytes.len()).sum();
    let mut parse_samples = Vec::with_capacity(sample_count_per_phase);
    let mut evaluation_samples = Vec::with_capacity(sample_count_per_phase);
    let mut envelope_create_samples = Vec::with_capacity(sample_count_per_phase);
    let mut envelope_verify_samples = Vec::with_capacity(sample_count_per_phase);
    let mut outcome_counts = serde_json::Map::new();

    for case in &cases {
        let outcome = evaluate_input_value(&case.input)
            .get("outcome")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_owned();
        let counter = outcome_counts.entry(outcome).or_insert_with(|| json!(0_u64));
        *counter = json!(counter.as_u64().unwrap_or(0) + 1);
    }

    // Pass A collects per-operation latency samples. Pass B measures wall-clock
    // throughput separately, without the per-operation timer/aggregation overhead.
    for _ in 0..iterations_per_case {
        for case in &cases {
            let started = Instant::now();
            let parsed = serde_json::from_slice::<Value>(black_box(case.bytes.as_slice()))
                .map_err(|error| format!("benchmark parse failed for {}: {error}", case.id))?;
            parse_samples.push(elapsed_nanos(started));
            black_box(parsed);
        }
    }
    let parse_phase_started = Instant::now();
    for _ in 0..iterations_per_case {
        for case in &cases {
            let parsed = serde_json::from_slice::<Value>(black_box(case.bytes.as_slice()))
                .map_err(|error| format!("throughput parse failed for {}: {error}", case.id))?;
            black_box(parsed);
        }
    }
    let parse_phase_wall_nanos = elapsed_nanos(parse_phase_started);

    for _ in 0..iterations_per_case {
        for case in &cases {
            let started = Instant::now();
            let evaluated = evaluate_input_value(black_box(&case.input));
            evaluation_samples.push(elapsed_nanos(started));
            black_box(evaluated);
        }
    }
    let evaluation_phase_started = Instant::now();
    for _ in 0..iterations_per_case {
        for case in &cases {
            let evaluated = evaluate_input_value(black_box(&case.input));
            black_box(evaluated);
        }
    }
    let evaluation_phase_wall_nanos = elapsed_nanos(evaluation_phase_started);

    for _ in 0..iterations_per_case {
        for case in &cases {
            let started = Instant::now();
            let envelope = create_replay_envelope(black_box(case.bytes.as_slice()))
                .map_err(|error| format!("benchmark envelope failed for {}: {error}", case.id))?;
            envelope_create_samples.push(elapsed_nanos(started));
            black_box(envelope);
        }
    }
    let envelope_create_phase_started = Instant::now();
    for _ in 0..iterations_per_case {
        for case in &cases {
            let envelope = create_replay_envelope(black_box(case.bytes.as_slice()))
                .map_err(|error| format!("throughput envelope creation failed for {}: {error}", case.id))?;
            black_box(envelope);
        }
    }
    let envelope_create_phase_wall_nanos = elapsed_nanos(envelope_create_phase_started);

    for _ in 0..iterations_per_case {
        for case in &cases {
            let started = Instant::now();
            let verification =
                verify_replay_envelope(black_box(&case.envelope), black_box(case.bytes.as_slice()));
            envelope_verify_samples.push(elapsed_nanos(started));
            if verification.get("status").and_then(Value::as_str) != Some("replay_consistent") {
                return Err(format!(
                    "replay consistency failed while benchmarking {}",
                    case.id
                ));
            }
            black_box(verification);
        }
    }
    let envelope_verify_phase_started = Instant::now();
    for _ in 0..iterations_per_case {
        for case in &cases {
            let verification =
                verify_replay_envelope(black_box(&case.envelope), black_box(case.bytes.as_slice()));
            if verification.get("status").and_then(Value::as_str) != Some("replay_consistent") {
                return Err(format!(
                    "replay consistency failed while measuring throughput for {}",
                    case.id
                ));
            }
            black_box(verification);
        }
    }
    let envelope_verify_phase_wall_nanos = elapsed_nanos(envelope_verify_phase_started);

    let total_samples = sample_count_per_phase;
    let corpus_sha256 = sha256_hex(vector_document_bytes);
    let total_input_bytes = corpus_input_bytes.saturating_mul(iterations_per_case);
    let phases = json!({
        "json_parse": summarize(
            "json_parse",
            parse_samples,
            total_input_bytes,
            parse_phase_wall_nanos
        ),
        "semantic_evaluation": summarize(
            "semantic_evaluation",
            evaluation_samples,
            total_input_bytes,
            evaluation_phase_wall_nanos
        ),
        "envelope_creation": summarize(
            "envelope_creation",
            envelope_create_samples,
            total_input_bytes,
            envelope_create_phase_wall_nanos
        ),
        "envelope_verification": summarize(
            "envelope_verification",
            envelope_verify_samples,
            total_input_bytes,
            envelope_verify_phase_wall_nanos
        )
    });

    Ok(json!({
        "benchmark_contract": "omsi-manufacturing-benchmark-report/0.1",
        "status": "measured_no_acceptance_claim",
        "authority": "performance_observation_only",
        "validator_version": env!("CARGO_PKG_VERSION"),
        "platform": {
            "os": std::env::consts::OS,
            "architecture": std::env::consts::ARCH,
            "build_profile": if cfg!(debug_assertions) { "debug" } else { "release" },
            "rustc_version": option_env!("RUSTC_VERSION").unwrap_or("not_recorded"),
            "host_label": environment_label("OMSI_BENCHMARK_HOST_LABEL"),
            "cpu_label": environment_label("OMSI_BENCHMARK_CPU_LABEL")
        },
        "corpus": {
            "format": vector_document.get("format").and_then(Value::as_str).unwrap_or("unknown"),
            "format_version": vector_document.get("format_version").and_then(Value::as_str).unwrap_or("unknown"),
            "sha256": corpus_sha256,
            "digest_scope": "exact_vector_document_bytes",
            "canonicalization": "none",
            "case_count": cases.len(),
            "case_ids": cases.iter().map(|case| case.id.as_str()).collect::<Vec<_>>(),
            "serialized_input_bytes_per_corpus_pass": corpus_input_bytes,
            "outcome_counts_per_corpus_pass": outcome_counts
        },
        "run": {
            "throughput_measurement_mode": "separate_batch_pass_without_per_operation_timers",
            "iterations_per_case": iterations_per_case,
            "warmup_iterations_per_case": WARMUP_ITERATIONS_PER_CASE,
            "max_cases": MAX_BENCHMARK_CASES,
            "max_samples_per_phase": MAX_TOTAL_SAMPLES_PER_PHASE,
            "sample_count_per_phase": total_samples,
            "input_bytes_per_phase": total_input_bytes
        },
        "phases": phases,
        "limitations": [
            "Microbenchmark only; no performance threshold is enforced and no measured value implies manufacturing correctness.",
            "Hardware, operating-system scheduling, power state, compiler profile, and background load affect results; compare only suitably controlled runs.",
            "Wall-clock throughput is timed over a separate full phase batch without per-operation timers; it includes loop/dispatch and black_box overhead but excludes process startup. Sampled-time rates are inverse-mean-latency equivalents, not end-to-end throughput.",
            "Inputs are the committed vector inputs serialized with the current serde_json build; the corpus SHA-256 binds the exact source vector document bytes, not a canonical JSON representation.",
            "This harness does not measure Holochain DHT latency, source-adapter latency, signature verification, factory telemetry throughput, or product-release workflows."
        ]
    }))
}

fn environment_label(name: &str) -> String {
    std::env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "not_recorded".to_owned())
}

fn elapsed_nanos(started: Instant) -> u64 {
    started.elapsed().as_nanos().max(1).min(u64::MAX as u128) as u64
}

fn summarize(
    phase: &str,
    mut samples: Vec<u64>,
    total_input_bytes: usize,
    wall_clock_elapsed_ns: u64,
) -> Value {
    samples.sort_unstable();
    let count = samples.len();
    let total_nanos: u128 = samples.iter().map(|value| u128::from(*value)).sum();
    let mean_nanos = total_nanos as f64 / count as f64;
    // This is an inverse-mean-latency equivalent, not measured batch throughput.
    let sampled_seconds = total_nanos as f64 / 1_000_000_000.0;
    let sampled_time_rate = count as f64 / sampled_seconds;
    let sampled_time_mib_per_second =
        total_input_bytes as f64 / (1024.0 * 1024.0) / sampled_seconds;
    // The outer timer measures actual full-phase elapsed wall time.
    let wall_seconds = wall_clock_elapsed_ns as f64 / 1_000_000_000.0;
    let wall_clock_operations_per_second = count as f64 / wall_seconds;
    let wall_clock_mib_per_second =
        total_input_bytes as f64 / (1024.0 * 1024.0) / wall_seconds;

    json!({
        "phase": phase,
        "sample_count": count,
        "total_sampled_time_ns": total_nanos.to_string(),
        "wall_clock_elapsed_ns": wall_clock_elapsed_ns,
        "wall_clock_operations_per_second": wall_clock_operations_per_second,
        "wall_clock_serialized_input_mib_per_second": wall_clock_mib_per_second,
        "min_ns": samples[0],
        "p50_ns": percentile(&samples, 0.50),
        "p95_ns": percentile(&samples, 0.95),
        "p99_ns": percentile(&samples, 0.99),
        "max_ns": samples[count - 1],
        "mean_ns": mean_nanos,
        "observations_per_second_from_sampled_time": sampled_time_rate,
        "serialized_input_mib_per_second_from_sampled_time": sampled_time_mib_per_second
    })
}

fn percentile(sorted_samples: &[u64], quantile: f64) -> u64 {
    let nearest_rank = ((sorted_samples.len() as f64) * quantile).ceil() as usize;
    let index = nearest_rank.saturating_sub(1).min(sorted_samples.len() - 1);
    sorted_samples[index]
}

fn sha256_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = Sha256::digest(bytes);
    let mut encoded = String::with_capacity(64);
    for byte in digest {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::{percentile, run};
    use serde_json::json;

    #[test]
    fn nearest_rank_percentiles_are_bounded_and_deterministic() {
        let values = [10, 20, 30, 40, 50];
        assert_eq!(percentile(&values, 0.50), 30);
        assert_eq!(percentile(&values, 0.95), 50);
        assert_eq!(percentile(&values, 0.99), 50);
    }

    #[test]
    fn empty_and_unbounded_benchmark_requests_are_rejected() {
        assert!(run(&json!({"vectors": []}), b"{}", 1).is_err());
        assert!(run(&json!({"vectors": []}), b"{}", 0).is_err());
        assert!(run(&json!({"vectors": []}), b"{}", 100_001).is_err());
    }

    #[test]
    fn excessive_total_sample_count_is_rejected_before_case_preparation() {
        let vectors = (0..11)
            .map(|index| json!({"id": format!("case-{index}"), "input": {"example": true}}))
            .collect::<Vec<_>>();
        assert!(run(&json!({"vectors": vectors}), b"{}", 100_000).is_err());
    }

    #[test]
    fn benchmark_report_binds_the_exact_corpus_and_disclaims_authority() {
        let document = json!({
            "format": "benchmark-test",
            "format_version": "0.1",
            "vectors": [{
                "id": "test-vector",
                "input": { "example": true }
            }]
        });
        let bytes = br#"{"format":"benchmark-test","format_version":"0.1","vectors":[{"id":"test-vector","input":{"example":true}}]}"#;
        let report = run(&document, bytes, 1).expect("one synthetic case should benchmark");
        assert_eq!(
            report["benchmark_contract"],
            "omsi-manufacturing-benchmark-report/0.1"
        );
        assert_eq!(report["status"], "measured_no_acceptance_claim");
        assert_eq!(report["authority"], "performance_observation_only");
        assert_eq!(report["corpus"]["digest_scope"], "exact_vector_document_bytes");
        assert_eq!(report["corpus"]["case_ids"][0], "test-vector");
        assert_eq!(report["run"]["sample_count_per_phase"], 1);
        assert_eq!(
            report["run"]["throughput_measurement_mode"],
            "separate_batch_pass_without_per_operation_timers"
        );
        assert_eq!(
            report["phases"]["envelope_verification"]["phase"],
            "envelope_verification"
        );
        assert!(
            report["phases"]["semantic_evaluation"]["wall_clock_elapsed_ns"]
                .as_u64()
                .is_some_and(|value| value > 0)
        );
        assert!(
            report["phases"]["semantic_evaluation"]["wall_clock_operations_per_second"]
                .as_f64()
                .is_some_and(|value| value > 0.0)
        );
    }
}
