//! Reproducible timing harness for the public adapter-qualified rollout gate.
//!
//! Results characterize this runner and code revision only. They are not a
//! deployment SLA or a vendor comparison.

#![forbid(unsafe_code)]

use std::error::Error;
use std::time::Instant;

use edge_policy_observation_reconciler::adapter_capability::{
    AdapterQualificationPolicy, CommitSemantics, DeviceAdapterCapabilities, RollbackSemantics,
};
use edge_policy_observation_reconciler::rollout_gate::{
    RolloutGateDecision, RolloutTarget, StagedRolloutPlan,
    authorize_next_wave_with_adapter_capabilities,
};
use serde_json::{Value, json};

const CANDIDATE: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const SAMPLE_COUNT: usize = 5;

fn build_plan(target_count: usize) -> (StagedRolloutPlan, Vec<DeviceAdapterCapabilities>) {
    let targets_per_wave = target_count.div_ceil(100);
    let mut targets = Vec::with_capacity(target_count);
    let mut profiles = Vec::with_capacity(target_count);

    for index in 0..target_count {
        let target_token = format!("target-node-{index}");
        let failure_domain = format!("site-{}", index % targets_per_wave);
        targets.push(RolloutTarget {
            target_token: target_token.clone(),
            failure_domain,
            policy_generation: 7,
            wave_index: index / targets_per_wave,
        });
        profiles.push(DeviceAdapterCapabilities {
            target_token,
            adapter_id: "bench-netconf".to_owned(),
            adapter_version: "bench-v1".to_owned(),
            commit_semantics: CommitSemantics::NetconfConfirmedCommit {
                confirm_timeout_secs: 180,
                cancel_commit_supported: true,
                candidate_lock_supported: true,
                reverts_on_session_loss: true,
            },
            rollback_semantics: RollbackSemantics::NativeConfirmedCommitTimeout,
            supports_effective_policy_readback: true,
            readback_digest_is_effective_policy: true,
            supports_independent_dataplane_probe: true,
            supports_positive_control_probe: true,
            supports_device_side_generation_fence: true,
            supports_exclusive_config_lock: true,
        });
    }

    (
        StagedRolloutPlan {
            rollout_id: format!("rollout-benchmark-{target_count}"),
            candidate_policy_sha256: CANDIDATE.to_owned(),
            targets,
            max_targets_per_wave: targets_per_wave,
            max_targets_per_failure_domain_per_wave: 1,
            max_evidence_age_ms: 60_000,
        },
        profiles,
    )
}

fn percentile(sorted: &[u64], numerator: usize, denominator: usize) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let rank = sorted.len().saturating_mul(numerator).div_ceil(denominator);
    sorted[rank.saturating_sub(1).min(sorted.len() - 1)]
}

fn benchmark_size(target_count: usize) -> Result<Value, Box<dyn Error>> {
    let (plan, profiles) = build_plan(target_count);
    let policy = AdapterQualificationPolicy {
        minimum_confirmed_commit_timeout_secs: 120,
        require_device_side_generation_fence: false,
        allow_explicit_rollback: true,
    };
    let mut samples_ns = Vec::with_capacity(SAMPLE_COUNT);

    for _ in 0..SAMPLE_COUNT {
        let started = Instant::now();
        let decision =
            authorize_next_wave_with_adapter_capabilities(&plan, &[], &profiles, &policy, 1_100)
                .map_err(|errors| {
                    format!(
                        "adapter-qualified rollout gate failed for {target_count} targets: {}",
                        errors.join("; ")
                    )
                })?;
        let elapsed_nanos = u64::try_from(started.elapsed().as_nanos())?;
        match decision {
            RolloutGateDecision::WaveAuthorized { wave_index: 0, .. } => {}
            unexpected => {
                return Err(format!(
                    "public gate did not authorize the first wave for {target_count} targets: {unexpected:?}"
                )
                .into());
            }
        }
        samples_ns.push(elapsed_nanos);
    }
    samples_ns.sort_unstable();

    Ok(json!({
        "target_count": target_count,
        "adapter_profile_count": profiles.len(),
        "planned_wave_count": plan.targets.iter().map(|target| target.wave_index).max().map_or(0, |wave| wave + 1),
        "iterations": SAMPLE_COUNT,
        "decision": "wave_0_authorized",
        "elapsed_ns": {
            "min": samples_ns[0],
            "p50": percentile(&samples_ns, 50, 100),
            "p95": percentile(&samples_ns, 95, 100),
            "max": samples_ns[samples_ns.len() - 1]
        }
    }))
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut measurements = Vec::new();
    for target_count in [100, 1_000, 10_000, 100_000] {
        measurements.push(benchmark_size(target_count)?);
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schema_version": "1.0.0",
            "benchmark": "edge-policy-adapter-qualified-rollout-gate-v1",
            "scope": "single_ci_runner_decision_latency_not_a_production_sla",
            "candidate_policy_sha256": CANDIDATE,
            "measurements": measurements
        }))?
    );
    Ok(())
}
