//! Deterministic evidence checks for MPTCP best-path improvement and TCP coexistence.
//!
//! This module evaluates paired *measured* summaries. It does not simulate TCP,
//! schedule packets, infer congestion-control behavior or actuate a kernel.
//! All qualification ratios use checked integer arithmetic in parts per million.

use std::convert::TryFrom;

const PPM: u128 = 1_000_000;
const MAX_SUBFLOWS: u16 = 64;
const MAX_RATIO_PPM: u64 = 10_000_000;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExperimentIdentity {
    pub kernel_build_sha256: String,
    pub topology_sha256: String,
    pub qdisc_config_sha256: String,
    pub workload_sha256: String,
    pub impairment_sha256: String,
    pub seed: u64,
    pub warmup_us: u64,
    pub measurement_duration_us: u64,
    /// Exact ordinary TCP congestion-control name used by the TCP references.
    pub tcp_congestion_control: String,
    /// Exact coupled MPTCP congestion-control identity under test.
    pub mptcp_congestion_control: String,
    /// Exact packet scheduler identity under test (including source/build identity elsewhere).
    pub mptcp_scheduler: String,
}

impl ExperimentIdentity {
    pub fn validate(&self) -> Result<(), EvidenceError> {
        for (name, digest) in [
            ("kernel_build_sha256", &self.kernel_build_sha256),
            ("topology_sha256", &self.topology_sha256),
            ("qdisc_config_sha256", &self.qdisc_config_sha256),
            ("workload_sha256", &self.workload_sha256),
            ("impairment_sha256", &self.impairment_sha256),
        ] {
            if !is_sha256(digest) {
                return Err(EvidenceError::InvalidDigest(name));
            }
        }
        if self.measurement_duration_us == 0 {
            return Err(EvidenceError::ZeroMeasurementDuration);
        }
        if self.warmup_us >= self.measurement_duration_us {
            return Err(EvidenceError::InvalidWarmupDuration);
        }
        if self.tcp_congestion_control.trim().is_empty()
            || self.mptcp_congestion_control.trim().is_empty()
            || self.mptcp_scheduler.trim().is_empty()
        {
            return Err(EvidenceError::MissingImplementationIdentity);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GateDisposition {
    Pass,
    Fail,
    Inconclusive,
    Unsupported,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvidenceError {
    InvalidDigest(&'static str),
    ZeroMeasurementDuration,
    InvalidWarmupDuration,
    MissingImplementationIdentity,
    InvalidTarget,
    ArithmeticOverflow,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GateResult {
    pub gate_id: &'static str,
    pub disposition: GateDisposition,
    pub reason: &'static str,
    pub target_ratio_ppm: Option<u64>,
    pub primary_ratio_ppm: Option<u64>,
    pub secondary_ratio_ppm: Option<u64>,
    pub total_goodput_ratio_ppm: Option<u64>,
}

impl GateResult {
    fn new(
        gate_id: &'static str,
        disposition: GateDisposition,
        reason: &'static str,
        target_ratio_ppm: Option<u64>,
    ) -> Self {
        Self {
            gate_id,
            disposition,
            reason,
            target_ratio_ppm,
            primary_ratio_ppm: None,
            secondary_ratio_ppm: None,
            total_goodput_ratio_ppm: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MptcpEvidence {
    pub kernel_mptcp_supported: bool,
    pub scheduler_supported: bool,
    /// True only when authoritative evidence verifies MPTCP at both endpoints.
    pub negotiation_verified_at_both_endpoints: bool,
    pub negotiated_subflow_count: u16,
    pub all_required_counters_present: bool,
    pub cleanup_verified: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BestPathComparison {
    pub path_a_tcp_goodput_bps: u64,
    pub path_b_tcp_goodput_bps: u64,
    pub mptcp_goodput_bps: u64,
    /// Each arm carries its own exact identity; the evaluator verifies equality.
    pub path_a_identity: ExperimentIdentity,
    pub path_b_identity: ExperimentIdentity,
    pub mptcp_identity: ExperimentIdentity,
    pub path_a_counters_present: bool,
    pub path_b_counters_present: bool,
    pub reference_cleanup_verified: bool,
    pub evidence: MptcpEvidence,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TcpTcpControl {
    pub identity: ExperimentIdentity,
    pub bottleneck_capacity_bps: u64,
    pub tcp_flow_a_goodput_bps: u64,
    pub tcp_flow_b_goodput_bps: u64,
    pub all_required_counters_present: bool,
    pub cleanup_verified: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MptcpTcpTreatment {
    pub identity: ExperimentIdentity,
    pub bottleneck_capacity_bps: u64,
    pub mptcp_goodput_bps: u64,
    pub companion_tcp_goodput_bps: u64,
    pub evidence: MptcpEvidence,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SharedBottleneckComparison {
    pub control: TcpTcpControl,
    pub treatment: MptcpTcpTreatment,
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn fixed_ratio_ppm(numerator: u128, denominator: u128) -> Result<u64, EvidenceError> {
    if denominator == 0 {
        return Err(EvidenceError::ArithmeticOverflow);
    }
    let scaled = numerator
        .checked_mul(PPM)
        .ok_or(EvidenceError::ArithmeticOverflow)?
        / denominator;
    u64::try_from(scaled).map_err(|_| EvidenceError::ArithmeticOverflow)
}

fn identity_pair_valid(
    first: &ExperimentIdentity,
    second: &ExperimentIdentity,
) -> Result<bool, EvidenceError> {
    first.validate()?;
    second.validate()?;
    Ok(first == second)
}

fn evidence_disposition(
    gate_id: &'static str,
    evidence: MptcpEvidence,
    target: Option<u64>,
) -> Option<GateResult> {
    if !evidence.kernel_mptcp_supported || !evidence.scheduler_supported {
        return Some(GateResult::new(
            gate_id,
            GateDisposition::Unsupported,
            "pinned_kernel_or_scheduler_does_not_support_requested_mptcp_arm",
            target,
        ));
    }
    if !evidence.all_required_counters_present {
        return Some(GateResult::new(
            gate_id,
            GateDisposition::Inconclusive,
            "required_mptcp_or_path_counters_missing",
            target,
        ));
    }
    if !evidence.cleanup_verified {
        return Some(GateResult::new(
            gate_id,
            GateDisposition::Fail,
            "namespace_or_resource_cleanup_not_verified",
            target,
        ));
    }
    if !evidence.negotiation_verified_at_both_endpoints {
        return Some(GateResult::new(
            gate_id,
            GateDisposition::Fail,
            "mptcp_negotiation_not_verified_at_both_endpoints",
            target,
        ));
    }
    if evidence.negotiated_subflow_count < 2 {
        return Some(GateResult::new(
            gate_id,
            GateDisposition::Fail,
            "fewer_than_two_negotiated_subflows",
            target,
        ));
    }
    if evidence.negotiated_subflow_count > MAX_SUBFLOWS {
        return Some(GateResult::new(
            gate_id,
            GateDisposition::Inconclusive,
            "negotiated_subflow_count_exceeds_bounded_evaluator_limit",
            target,
        ));
    }
    None
}

/// Evaluate MPTCP application goodput against the better of two matched
/// single-path TCP references over genuinely independent capacity domains.
///
/// The caller is responsible for ensuring the frozen topology actually has
/// distinct capacity domains; this evaluator binds the trial to its topology
/// digest but does not discover physical bottlenecks.
pub fn evaluate_best_path_improvement(
    comparison: &BestPathComparison,
    minimum_ratio_ppm: u64,
) -> Result<GateResult, EvidenceError> {
    if minimum_ratio_ppm == 0 || minimum_ratio_ppm > MAX_RATIO_PPM {
        return Err(EvidenceError::InvalidTarget);
    }
    let identities_match = identity_pair_valid(
        &comparison.path_a_identity,
        &comparison.path_b_identity,
    )? && identity_pair_valid(
        &comparison.path_a_identity,
        &comparison.mptcp_identity,
    )?;
    if !identities_match {
        return Ok(GateResult::new(
            "MPTCP_GOAL1_BEST_PATH",
            GateDisposition::Inconclusive,
            "single_path_and_mptcp_experiment_identity_mismatch",
            Some(minimum_ratio_ppm),
        ));
    }
    if !comparison.path_a_counters_present || !comparison.path_b_counters_present {
        return Ok(GateResult::new(
            "MPTCP_GOAL1_BEST_PATH",
            GateDisposition::Inconclusive,
            "single_path_tcp_reference_counters_missing",
            Some(minimum_ratio_ppm),
        ));
    }
    if !comparison.reference_cleanup_verified {
        return Ok(GateResult::new(
            "MPTCP_GOAL1_BEST_PATH",
            GateDisposition::Fail,
            "single_path_tcp_reference_cleanup_not_verified",
            Some(minimum_ratio_ppm),
        ));
    }
    if let Some(result) = evidence_disposition(
        "MPTCP_GOAL1_BEST_PATH",
        comparison.evidence,
        Some(minimum_ratio_ppm),
    ) {
        return Ok(result);
    }

    let best_tcp = comparison
        .path_a_tcp_goodput_bps
        .max(comparison.path_b_tcp_goodput_bps);
    if best_tcp == 0 {
        return Ok(GateResult::new(
            "MPTCP_GOAL1_BEST_PATH",
            GateDisposition::Inconclusive,
            "best_single_path_tcp_reference_has_zero_goodput",
            Some(minimum_ratio_ppm),
        ));
    }

    let ratio = fixed_ratio_ppm(
        u128::from(comparison.mptcp_goodput_bps),
        u128::from(best_tcp),
    )?;
    let mut result = GateResult::new(
        "MPTCP_GOAL1_BEST_PATH",
        if ratio >= minimum_ratio_ppm {
            GateDisposition::Pass
        } else {
            GateDisposition::Fail
        },
        if ratio >= minimum_ratio_ppm {
            "mptcp_meets_declared_best_single_path_ratio"
        } else {
            "mptcp_below_declared_best_single_path_ratio"
        },
        Some(minimum_ratio_ppm),
    );
    result.primary_ratio_ppm = Some(ratio);
    Ok(result)
}

/// Evaluate the MPTCP connection and its competing TCP tenant against a
/// matched TCP+TCP shared-bottleneck control.
///
/// The primary ratio compares MPTCP goodput with the mean single-TCP flow
/// goodput in the control. The secondary ratio compares the companion TCP
/// treatment goodput with that same control mean. The total ratio is reported
/// to reveal aggregate utilization changes; it is not by itself a fairness
/// verdict.
pub fn evaluate_shared_bottleneck_do_no_harm(
    comparison: &SharedBottleneckComparison,
    max_mptcp_share_ratio_ppm: u64,
    minimum_companion_retention_ppm: u64,
) -> Result<GateResult, EvidenceError> {
    if max_mptcp_share_ratio_ppm == 0
        || max_mptcp_share_ratio_ppm > MAX_RATIO_PPM
        || minimum_companion_retention_ppm == 0
        || minimum_companion_retention_ppm > PPM as u64
    {
        return Err(EvidenceError::InvalidTarget);
    }

    let gate_id = "MPTCP_GOAL2_SHARED_BOTTLENECK";
    let identities_match = identity_pair_valid(
        &comparison.control.identity,
        &comparison.treatment.identity,
    )?;
    if !identities_match {
        return Ok(GateResult::new(
            gate_id,
            GateDisposition::Inconclusive,
            "control_and_treatment_experiment_identity_mismatch",
            Some(max_mptcp_share_ratio_ppm),
        ));
    }
    if let Some(result) = evidence_disposition(
        gate_id,
        comparison.treatment.evidence,
        Some(max_mptcp_share_ratio_ppm),
    ) {
        return Ok(result);
    }
    if !comparison.control.all_required_counters_present {
        return Ok(GateResult::new(
            gate_id,
            GateDisposition::Inconclusive,
            "tcp_tcp_control_counters_missing",
            Some(max_mptcp_share_ratio_ppm),
        ));
    }
    if !comparison.control.cleanup_verified {
        return Ok(GateResult::new(
            gate_id,
            GateDisposition::Fail,
            "control_namespace_or_resource_cleanup_not_verified",
            Some(max_mptcp_share_ratio_ppm),
        ));
    }
    if comparison.control.bottleneck_capacity_bps == 0
        || comparison.treatment.bottleneck_capacity_bps == 0
    {
        return Ok(GateResult::new(
            gate_id,
            GateDisposition::Inconclusive,
            "shared_bottleneck_capacity_missing",
            Some(max_mptcp_share_ratio_ppm),
        ));
    }
    if comparison.control.bottleneck_capacity_bps
        != comparison.treatment.bottleneck_capacity_bps
    {
        return Ok(GateResult::new(
            gate_id,
            GateDisposition::Inconclusive,
            "control_and_treatment_bottleneck_capacity_mismatch",
            Some(max_mptcp_share_ratio_ppm),
        ));
    }

    let control_total = u128::from(comparison.control.tcp_flow_a_goodput_bps)
        + u128::from(comparison.control.tcp_flow_b_goodput_bps);
    let control_mean = control_total / 2;
    if control_mean == 0 {
        return Ok(GateResult::new(
            gate_id,
            GateDisposition::Inconclusive,
            "tcp_tcp_control_mean_goodput_is_zero",
            Some(max_mptcp_share_ratio_ppm),
        ));
    }

    let mptcp_ratio = fixed_ratio_ppm(
        u128::from(comparison.treatment.mptcp_goodput_bps),
        control_mean,
    )?;
    let companion_ratio = fixed_ratio_ppm(
        u128::from(comparison.treatment.companion_tcp_goodput_bps),
        control_mean,
    )?;
    let treatment_total = u128::from(comparison.treatment.mptcp_goodput_bps)
        + u128::from(comparison.treatment.companion_tcp_goodput_bps);
    let total_ratio = fixed_ratio_ppm(treatment_total, control_total)?;

    let fair_share = mptcp_ratio <= max_mptcp_share_ratio_ppm;
    let companion_retained = companion_ratio >= minimum_companion_retention_ppm;
    let mut result = GateResult::new(
        gate_id,
        if fair_share && companion_retained {
            GateDisposition::Pass
        } else {
            GateDisposition::Fail
        },
        match (fair_share, companion_retained) {
            (true, true) => "mptcp_and_companion_tcp_meet_declared_do_no_harm_targets",
            (false, true) => "mptcp_exceeds_single_tcp_equivalent_share",
            (true, false) => "competing_tcp_retention_below_declared_floor",
            (false, false) => "mptcp_overuses_shared_bottleneck_and_starves_competing_tcp",
        },
        Some(max_mptcp_share_ratio_ppm),
    );
    result.primary_ratio_ppm = Some(mptcp_ratio);
    result.secondary_ratio_ppm = Some(companion_ratio);
    result.total_goodput_ratio_ppm = Some(total_ratio);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity() -> ExperimentIdentity {
        ExperimentIdentity {
            kernel_build_sha256: "a".repeat(64),
            topology_sha256: "b".repeat(64),
            qdisc_config_sha256: "c".repeat(64),
            workload_sha256: "d".repeat(64),
            impairment_sha256: "e".repeat(64),
            seed: 7,
            warmup_us: 2_000_000,
            measurement_duration_us: 30_000_000,
            tcp_congestion_control: "cubic".to_owned(),
            mptcp_congestion_control: "lia".to_owned(),
            mptcp_scheduler: "default".to_owned(),
        }
    }

    fn evidence() -> MptcpEvidence {
        MptcpEvidence {
            kernel_mptcp_supported: true,
            scheduler_supported: true,
            negotiation_verified_at_both_endpoints: true,
            negotiated_subflow_count: 2,
            all_required_counters_present: true,
            cleanup_verified: true,
        }
    }

    fn shared_comparison() -> SharedBottleneckComparison {
        SharedBottleneckComparison {
            control: TcpTcpControl {
                identity: identity(),
                bottleneck_capacity_bps: 100_000_000,
                tcp_flow_a_goodput_bps: 48_000_000,
                tcp_flow_b_goodput_bps: 47_000_000,
                all_required_counters_present: true,
                cleanup_verified: true,
            },
            treatment: MptcpTcpTreatment {
                identity: identity(),
                bottleneck_capacity_bps: 100_000_000,
                mptcp_goodput_bps: 45_000_000,
                companion_tcp_goodput_bps: 44_000_000,
                evidence: evidence(),
            },
        }
    }

    #[test]
    fn independent_mptcp_meets_best_single_path_reference() {
        let result = evaluate_best_path_improvement(
            &BestPathComparison {
                path_a_tcp_goodput_bps: 70_000_000,
                path_b_tcp_goodput_bps: 85_000_000,
                mptcp_goodput_bps: 90_000_000,
                path_a_identity: identity(),
                path_b_identity: identity(),
                mptcp_identity: identity(),
                path_a_counters_present: true,
                path_b_counters_present: true,
                reference_cleanup_verified: true,
                evidence: evidence(),
            },
            1_000_000,
        )
        .expect("valid evidence");
        assert_eq!(result.disposition, GateDisposition::Pass);
        assert_eq!(result.primary_ratio_ppm, Some(1_058_823));
    }

    #[test]
    fn missing_single_path_tcp_counter_is_inconclusive() {
        let result = evaluate_best_path_improvement(
            &BestPathComparison {
                path_a_tcp_goodput_bps: 70_000_000,
                path_b_tcp_goodput_bps: 85_000_000,
                mptcp_goodput_bps: 90_000_000,
                path_a_identity: identity(),
                path_b_identity: identity(),
                mptcp_identity: identity(),
                path_a_counters_present: false,
                path_b_counters_present: true,
                reference_cleanup_verified: true,
                evidence: evidence(),
            },
            1_000_000,
        )
        .expect("missing reference counters are inconclusive");
        assert_eq!(result.disposition, GateDisposition::Inconclusive);
        assert_eq!(result.reason, "single_path_tcp_reference_counters_missing");
    }

    #[test]
    fn independent_comparison_with_mismatched_qdisc_is_inconclusive() {
        let mut path_b_identity = identity();
        path_b_identity.qdisc_config_sha256 = "f".repeat(64);
        let result = evaluate_best_path_improvement(
            &BestPathComparison {
                path_a_tcp_goodput_bps: 70_000_000,
                path_b_tcp_goodput_bps: 85_000_000,
                mptcp_goodput_bps: 90_000_000,
                path_a_identity: identity(),
                path_b_identity,
                mptcp_identity: identity(),
                evidence: evidence(),
            },
            1_000_000,
        )
        .expect("pair with mismatched input identities is classified");
        assert_eq!(result.disposition, GateDisposition::Inconclusive);
        assert_eq!(
            result.reason,
            "single_path_and_mptcp_experiment_identity_mismatch"
        );
    }

    #[test]
    fn independent_mptcp_below_best_path_reference_fails() {
        let result = evaluate_best_path_improvement(
            &BestPathComparison {
                path_a_tcp_goodput_bps: 70_000_000,
                path_b_tcp_goodput_bps: 85_000_000,
                mptcp_goodput_bps: 84_000_000,
                identity: identity(),
                evidence: evidence(),
            },
            1_000_000,
        )
        .expect("valid evidence");
        assert_eq!(result.disposition, GateDisposition::Fail);
        assert_eq!(result.primary_ratio_ppm, Some(988_235));
    }

    #[test]
    fn shared_bottleneck_do_no_harm_passes_matched_tcp_reference() {
        let result = evaluate_shared_bottleneck_do_no_harm(
            &shared_comparison(),
            1_100_000,
            900_000,
        )
        .expect("valid evidence");
        assert_eq!(result.disposition, GateDisposition::Pass);
        assert_eq!(result.primary_ratio_ppm, Some(947_368));
        assert_eq!(result.secondary_ratio_ppm, Some(926_315));
        assert_eq!(result.total_goodput_ratio_ppm, Some(936_842));
    }

    #[test]
    fn mptcp_excess_shared_bottleneck_share_fails() {
        let mut comparison = shared_comparison();
        comparison.treatment.mptcp_goodput_bps = 60_000_000;
        let result = evaluate_shared_bottleneck_do_no_harm(
            &comparison,
            1_100_000,
            900_000,
        )
        .expect("valid evidence");
        assert_eq!(result.disposition, GateDisposition::Fail);
        assert_eq!(
            result.reason,
            "mptcp_exceeds_single_tcp_equivalent_share"
        );
    }

    #[test]
    fn companion_tcp_starvation_fails_even_when_mptcp_ratio_is_bounded() {
        let mut comparison = shared_comparison();
        comparison.treatment.companion_tcp_goodput_bps = 10_000_000;
        let result = evaluate_shared_bottleneck_do_no_harm(
            &comparison,
            1_100_000,
            900_000,
        )
        .expect("valid evidence");
        assert_eq!(result.disposition, GateDisposition::Fail);
        assert_eq!(
            result.reason,
            "competing_tcp_retention_below_declared_floor"
        );
    }

    #[test]
    fn single_subflow_is_not_qualified_as_multipath() {
        let mut comparison = shared_comparison();
        comparison.treatment.evidence.negotiated_subflow_count = 1;
        let result = evaluate_shared_bottleneck_do_no_harm(
            &comparison,
            1_100_000,
            900_000,
        )
        .expect("valid evidence");
        assert_eq!(result.disposition, GateDisposition::Fail);
        assert_eq!(result.reason, "fewer_than_two_negotiated_subflows");
    }

    #[test]
    fn missing_negotiation_counters_are_inconclusive() {
        let mut comparison = shared_comparison();
        comparison.treatment.evidence.all_required_counters_present = false;
        let result = evaluate_shared_bottleneck_do_no_harm(
            &comparison,
            1_100_000,
            900_000,
        )
        .expect("valid evidence");
        assert_eq!(result.disposition, GateDisposition::Inconclusive);
    }

    #[test]
    fn unsupported_kernel_scheduler_is_not_a_pass() {
        let mut comparison = shared_comparison();
        comparison.treatment.evidence.scheduler_supported = false;
        let result = evaluate_shared_bottleneck_do_no_harm(
            &comparison,
            1_100_000,
            900_000,
        )
        .expect("valid evidence");
        assert_eq!(result.disposition, GateDisposition::Unsupported);
    }

    #[test]
    fn mismatched_qdisc_digest_makes_pair_inconclusive() {
        let mut comparison = shared_comparison();
        comparison.treatment.identity.qdisc_config_sha256 = "f".repeat(64);
        let result = evaluate_shared_bottleneck_do_no_harm(
            &comparison,
            1_100_000,
            900_000,
        )
        .expect("valid evidence");
        assert_eq!(result.disposition, GateDisposition::Inconclusive);
        assert_eq!(
            result.reason,
            "control_and_treatment_experiment_identity_mismatch"
        );
    }

    #[test]
    fn different_bottleneck_capacity_makes_pair_inconclusive() {
        let mut comparison = shared_comparison();
        comparison.treatment.bottleneck_capacity_bps = 90_000_000;
        let result = evaluate_shared_bottleneck_do_no_harm(
            &comparison,
            1_100_000,
            900_000,
        )
        .expect("valid evidence");
        assert_eq!(result.disposition, GateDisposition::Inconclusive);
        assert_eq!(
            result.reason,
            "control_and_treatment_bottleneck_capacity_mismatch"
        );
    }

    #[test]
    fn cleanup_failure_cannot_be_a_pass() {
        let mut comparison = shared_comparison();
        comparison.treatment.evidence.cleanup_verified = false;
        let result = evaluate_shared_bottleneck_do_no_harm(
            &comparison,
            1_100_000,
            900_000,
        )
        .expect("valid evidence");
        assert_eq!(result.disposition, GateDisposition::Fail);
        assert_eq!(
            result.reason,
            "namespace_or_resource_cleanup_not_verified"
        );
    }

    #[test]
    fn missing_tcp_control_counters_make_pair_inconclusive() {
        let mut comparison = shared_comparison();
        comparison.control.all_required_counters_present = false;
        let result = evaluate_shared_bottleneck_do_no_harm(
            &comparison,
            1_100_000,
            900_000,
        )
        .expect("valid evidence");
        assert_eq!(result.disposition, GateDisposition::Inconclusive);
    }

    #[test]
    fn malformed_digest_is_rejected() {
        let mut invalid = identity();
        invalid.topology_sha256 = "not-a-digest".to_owned();
        assert_eq!(
            invalid.validate(),
            Err(EvidenceError::InvalidDigest("topology_sha256"))
        );
    }

    #[test]
    fn zero_tcp_reference_is_inconclusive() {
        let mut comparison = shared_comparison();
        comparison.control.tcp_flow_a_goodput_bps = 0;
        comparison.control.tcp_flow_b_goodput_bps = 0;
        let result = evaluate_shared_bottleneck_do_no_harm(
            &comparison,
            1_100_000,
            900_000,
        )
        .expect("valid evidence");
        assert_eq!(result.disposition, GateDisposition::Inconclusive);
    }

    #[test]
    fn invalid_targets_are_rejected() {
        assert_eq!(
            evaluate_best_path_improvement(
                &BestPathComparison {
                    path_a_tcp_goodput_bps: 1,
                    path_b_tcp_goodput_bps: 1,
                    mptcp_goodput_bps: 1,
                    identity: identity(),
                    evidence: evidence(),
                },
                0,
            ),
            Err(EvidenceError::InvalidTarget)
        );
    }

    #[test]
    fn fixed_point_ratio_is_repeatable() {
        let comparison = shared_comparison();
        let first =
            evaluate_shared_bottleneck_do_no_harm(&comparison, 1_100_000, 900_000)
                .expect("valid evidence");
        let second =
            evaluate_shared_bottleneck_do_no_harm(&comparison, 1_100_000, 900_000)
                .expect("same frozen evidence");
        assert_eq!(first, second);
    }
}
