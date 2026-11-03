//! Paired evidence qualification for mixed L4S / Classic traffic.
//!
//! This module evaluates a single frozen Classic-only control versus mixed
//! Classic + L4S treatment. It does not implement TCP Prague, a qdisc, ECN,
//! packet classification, or network actuation. A verdict is scoped to one
//! pair; programme-level conclusions require external repeated-trial analysis.

const PPM: u128 = 1_000_000;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ImplementationProvenance {
    InTreeKernel,
    ExternalArtifact { artifact_sha256: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImplementationIdentity {
    pub name: String,
    pub provenance: ImplementationProvenance,
}

impl ImplementationIdentity {
    fn validate(&self) -> Result<(), L4sEvidenceError> {
        if self.name.trim().is_empty() {
            return Err(L4sEvidenceError::MissingImplementationIdentity);
        }
        if let ImplementationProvenance::ExternalArtifact { artifact_sha256 } =
            &self.provenance
        {
            if !is_sha256(artifact_sha256) {
                return Err(L4sEvidenceError::InvalidImplementationArtifact);
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AqmKind {
    DualPi2,
    FqL4s,
    ClassicOnly,
    Unsupported,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SenderKind {
    TcpPrague,
    QuicPrague,
    TcpCubic,
    Dctcp,
    Other,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GateDisposition {
    Pass,
    Fail,
    Inconclusive,
    Unsupported,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum L4sEvidenceError {
    InvalidDigest(&'static str),
    InvalidDecisionDuration,
    InvalidThreshold,
    MissingImplementationIdentity,
    InvalidImplementationArtifact,
    ArithmeticOverflow,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PairIdentity {
    pub kernel_build_sha256: String,
    pub topology_sha256: String,
    pub qdisc_config_sha256: String,
    pub classic_workload_sha256: String,
    /// Planned L4S workload identity; the Classic-only control records the
    /// same intended treatment workload so paired runs remain reproducible.
    pub l4s_workload_sha256: String,
    pub path_impairment_sha256: String,
    pub endpoint_config_sha256: String,
    pub ecn_path_sha256: String,
    pub seed: u64,
    pub warmup_us: u64,
    pub measurement_duration_us: u64,
    pub link_rate_bps: u64,
    pub base_rtt_us: u64,
}

impl PairIdentity {
    pub fn validate(&self) -> Result<(), L4sEvidenceError> {
        for (name, digest) in [
            ("kernel_build_sha256", &self.kernel_build_sha256),
            ("topology_sha256", &self.topology_sha256),
            ("qdisc_config_sha256", &self.qdisc_config_sha256),
            ("classic_workload_sha256", &self.classic_workload_sha256),
            ("l4s_workload_sha256", &self.l4s_workload_sha256),
            ("path_impairment_sha256", &self.path_impairment_sha256),
            ("endpoint_config_sha256", &self.endpoint_config_sha256),
            ("ecn_path_sha256", &self.ecn_path_sha256),
        ] {
            if !is_sha256(digest) {
                return Err(L4sEvidenceError::InvalidDigest(name));
            }
        }
        if self.measurement_duration_us == 0
            || self.warmup_us >= self.measurement_duration_us
            || self.link_rate_bps == 0
            || self.base_rtt_us == 0
        {
            return Err(L4sEvidenceError::InvalidDecisionDuration);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClassicOnlyControl {
    pub identity: PairIdentity,
    pub aqm_kind: AqmKind,
    pub aqm: ImplementationIdentity,
    pub classic_sender: ImplementationIdentity,
    pub classic_sender_kind: SenderKind,
    pub qdisc_readback_verified: bool,
    pub queue_counters_present: bool,
    pub cleanup_verified: bool,
    pub classic_goodput_bps: u64,
    pub classic_backlogged_flow_count: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EcnFeedbackMode {
    /// TCP AccECN negotiated and observed at both endpoints.
    TcpAccEcn,
    /// QUIC ECN validation and per-codepoint feedback observed on the path.
    QuicEcnCounts,
    /// Classic RFC 3168 feedback is not sufficient evidence for scalable TCP.
    ClassicEcnOnly,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EcnPathEvidence {
    /// Negotiation is recorded independently at sender and receiver.
    pub ecn_negotiated_at_sender: bool,
    pub ecn_negotiated_at_receiver: bool,
    pub feedback_mode: EcnFeedbackMode,
    /// Packet/mark counters are taken at the named observation points.
    pub ect1_packets_sent_by_sender: u64,
    pub ect1_packets_received_at_receiver: u64,
    pub ect1_packets_observed_at_aqm: u64,
    pub ce_marks_at_aqm: u64,
    pub ce_feedback_received_at_sender: u64,
    /// A frozen offered-load challenge is required before zero CE marks
    /// can be interpreted as a failure rather than an unchallenged path.
    pub congestion_challenge_verified: bool,
    pub counters_complete: bool,
    pub sender_fallback_detected: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClassicFlowPair {
    pub flow_id: String,
    pub backlogged: bool,
    pub control_goodput_bps: u64,
    pub treatment_goodput_bps: u64,
    pub zero_service_duration_us: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MixedL4sTreatment {
    pub identity: PairIdentity,
    pub aqm_kind: AqmKind,
    pub aqm: ImplementationIdentity,
    pub classic_sender: ImplementationIdentity,
    pub classic_sender_kind: SenderKind,
    pub l4s_sender: ImplementationIdentity,
    pub l4s_sender_kind: SenderKind,
    pub qdisc_readback_verified: bool,
    pub cleanup_verified: bool,
    pub queue_counters_present: bool,
    /// ECT(1) packets observed in the L4S queue, not merely total queue depth.
    pub ect1_packets_classified_to_l4s_queue: u64,
    /// Classic packets observed in the Classic queue.
    pub classic_packets_classified_to_classic_queue: u64,
    pub classic_backlogged_flow_count: u32,
    pub classic_flow_pairs: Vec<ClassicFlowPair>,
    pub ecn: EcnPathEvidence,
    pub classic_goodput_bps: u64,
    pub l4s_goodput_bps: u64,
    pub l4s_queue_delay_average_us: u64,
    pub l4s_queue_delay_p99_us: u64,
    pub classic_queue_delay_p99_us: u64,
    pub classic_zero_service_duration_us: u64,
    /// True only when rate, RTT, workload and measurement conditions match
    /// the frozen RFC 9332 reproduction profile for which delay targets apply.
    pub rfc9332_reproduction_profile_verified: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct QualificationThresholds {
    pub classic_retention_target_ppm: u64,
    pub classic_retention_investigation_floor_ppm: u64,
    pub l4s_average_queue_delay_target_us: u64,
    pub l4s_p99_queue_delay_target_us: u64,
}

impl Default for QualificationThresholds {
    fn default() -> Self {
        Self {
            classic_retention_target_ppm: 950_000,
            classic_retention_investigation_floor_ppm: 900_000,
            l4s_average_queue_delay_target_us: 1_000,
            l4s_p99_queue_delay_target_us: 2_000,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GateResult {
    pub gate_id: &'static str,
    /// A gate is evaluated for one matched trial/pair only.
    pub scope: &'static str,
    pub disposition: GateDisposition,
    pub reason: &'static str,
    pub target: Option<u64>,
    pub observed: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct L4sPairReport {
    pub ecn_path: GateResult,
    pub queue_classification: GateResult,
    pub l4s_queue_delay: GateResult,
    pub classic_retention: GateResult,
    pub classic_flow_retention: GateResult,
    pub classic_starvation: GateResult,
    pub classic_retention_ratio_ppm: Option<u64>,
    pub worst_backlogged_classic_flow_retention_ppm: Option<u64>,
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn gate(
    gate_id: &'static str,
    disposition: GateDisposition,
    reason: &'static str,
    target: Option<u64>,
    observed: Option<u64>,
) -> GateResult {
    GateResult {
        gate_id,
        scope: "single_paired_trial",
        disposition,
        reason,
        target,
        observed,
    }
}

fn paired_identity_matches(
    control: &PairIdentity,
    treatment: &PairIdentity,
) -> Result<bool, L4sEvidenceError> {
    control.validate()?;
    treatment.validate()?;
    Ok(control == treatment)
}

fn implementation_is_pinned(
    implementation: &ImplementationIdentity,
    kernel_build_sha256: &str,
) -> Result<bool, L4sEvidenceError> {
    implementation.validate()?;
    match &implementation.provenance {
        ImplementationProvenance::InTreeKernel => Ok(is_sha256(kernel_build_sha256)),
        ImplementationProvenance::ExternalArtifact { artifact_sha256 } => {
            Ok(is_sha256(artifact_sha256))
        }
    }
}

fn validate_thresholds(
    thresholds: QualificationThresholds,
) -> Result<(), L4sEvidenceError> {
    if thresholds.classic_retention_target_ppm == 0
        || thresholds.classic_retention_target_ppm > PPM as u64
        || thresholds.classic_retention_investigation_floor_ppm == 0
        || thresholds.classic_retention_investigation_floor_ppm
            > thresholds.classic_retention_target_ppm
        || thresholds.l4s_average_queue_delay_target_us == 0
        || thresholds.l4s_p99_queue_delay_target_us == 0
    {
        return Err(L4sEvidenceError::InvalidThreshold);
    }
    Ok(())
}

fn unsupported_sender(kind: SenderKind) -> bool {
    !matches!(kind, SenderKind::TcpPrague | SenderKind::QuicPrague)
}

/// Evaluate whether the mixed treatment actually exercised an L4S-capable
/// sender and a supported L4S AQM/queueing path.
///
/// DCTCP is deliberately not accepted as a public-Internet Prague substitute;
/// controlled DCTCP experiments require a separate explicitly labelled profile.
pub fn evaluate_l4s_ecn_and_aqm(
    treatment: &MixedL4sTreatment,
) -> Result<(GateResult, GateResult), L4sEvidenceError> {
    treatment.identity.validate()?;
    treatment.aqm.validate()?;
    treatment.classic_sender.validate()?;
    treatment.l4s_sender.validate()?;

    let aqm_supported = matches!(treatment.aqm_kind, AqmKind::DualPi2 | AqmKind::FqL4s);
    if !aqm_supported {
        let unsupported = gate(
            "L4S_AQM_SUPPORT_001",
            GateDisposition::Unsupported,
            "requested_aqm_is_not_a_supported_l4s_aqm",
            None,
            None,
        );
        let ecn = gate(
            "L4S_ECN_PATH_001",
            GateDisposition::Unsupported,
            "l4s_aqm_arm_unsupported",
            None,
            None,
        );
        return Ok((ecn, unsupported));
    }
    if unsupported_sender(treatment.l4s_sender_kind) {
        return Ok((
            gate(
                "L4S_ECN_PATH_001",
                GateDisposition::Unsupported,
                "sender_is_not_a_prague_compliant_scalable_sender",
                None,
                None,
            ),
            gate(
                "L4S_AQM_SUPPORT_001",
                GateDisposition::Inconclusive,
                "queue_classification_cannot_qualify_without_supported_scalable_sender",
                None,
                None,
            ),
        ));
    }

    let aqm_pinned = implementation_is_pinned(&treatment.aqm, &treatment.identity.kernel_build_sha256)?;
    let sender_pinned = implementation_is_pinned(
        &treatment.l4s_sender,
        &treatment.identity.kernel_build_sha256,
    )?;
    if !aqm_pinned || !sender_pinned {
        return Ok((
            gate(
                "L4S_ECN_PATH_001",
                GateDisposition::Inconclusive,
                "aqm_or_scalable_sender_implementation_not_pinned",
                None,
                None,
            ),
            gate(
                "L4S_AQM_SUPPORT_001",
                GateDisposition::Inconclusive,
                "aqm_or_scalable_sender_implementation_not_pinned",
                None,
                None,
            ),
        ));
    }
    if !treatment.queue_counters_present || !treatment.ecn.counters_complete {
        return Ok((
            gate(
                "L4S_ECN_PATH_001",
                GateDisposition::Inconclusive,
                "ecn_or_queue_counters_missing",
                None,
                None,
            ),
            gate(
                "L4S_AQM_SUPPORT_001",
                GateDisposition::Inconclusive,
                "ecn_or_queue_counters_missing",
                None,
                None,
            ),
        ));
    }
    if !treatment.qdisc_readback_verified || !treatment.cleanup_verified {
        return Ok((
            gate(
                "L4S_ECN_PATH_001",
                GateDisposition::Fail,
                "qdisc_readback_or_cleanup_not_verified",
                None,
                None,
            ),
            gate(
                "L4S_AQM_SUPPORT_001",
                GateDisposition::Fail,
                "qdisc_readback_or_cleanup_not_verified",
                None,
                None,
            ),
        ));
    }
    if treatment.ecn.sender_fallback_detected {
        return Ok((
            gate(
                "L4S_ECN_PATH_001",
                GateDisposition::Fail,
                "sender_fell_back_from_requested_l4s_controller",
                None,
                None,
            ),
            gate(
                "L4S_AQM_SUPPORT_001",
                GateDisposition::Fail,
                "sender_fell_back_from_requested_l4s_controller",
                None,
                None,
            ),
        ));
    }

    let sender_and_receiver_negotiated =
        treatment.ecn.ecn_negotiated_at_sender && treatment.ecn.ecn_negotiated_at_receiver;
    let feedback_mode_matches = match treatment.l4s_sender_kind {
        SenderKind::TcpPrague => treatment.ecn.feedback_mode == EcnFeedbackMode::TcpAccEcn,
        SenderKind::QuicPrague => treatment.ecn.feedback_mode == EcnFeedbackMode::QuicEcnCounts,
        _ => false,
    };
    let ect1_reached_receiver = treatment.ecn.ect1_packets_sent_by_sender > 0
        && treatment.ecn.ect1_packets_received_at_receiver > 0;
    let ect1_reached_aqm = treatment.ecn.ect1_packets_observed_at_aqm > 0;
    let ce_feedback_path = treatment.ecn.ce_marks_at_aqm > 0
        && treatment.ecn.ce_feedback_received_at_sender > 0;
    let ecn_counters_consistent =
        treatment.ecn.ce_marks_at_aqm <= treatment.ecn.ect1_packets_observed_at_aqm;
    let ecn_ok = sender_and_receiver_negotiated
        && feedback_mode_matches
        && ect1_reached_receiver
        && ect1_reached_aqm
        && ce_feedback_path
        && ecn_counters_consistent;

    let ecn_result = if !sender_and_receiver_negotiated {
        gate(
            "L4S_ECN_PATH_001",
            GateDisposition::Fail,
            "ecn_not_negotiated_at_both_endpoints",
            None,
            None,
        )
    } else if !feedback_mode_matches {
        let (disposition, reason) = match treatment.ecn.feedback_mode {
            EcnFeedbackMode::Unknown => (
                GateDisposition::Inconclusive,
                "transport_specific_accurate_ecn_feedback_evidence_missing",
            ),
            EcnFeedbackMode::ClassicEcnOnly => (
                GateDisposition::Fail,
                "classic_ecn_feedback_is_not_sufficient_for_scalable_sender",
            ),
            _ => (
                GateDisposition::Fail,
                "ecn_feedback_mode_does_not_match_sender_transport",
            ),
        };
        gate("L4S_ECN_PATH_001", disposition, reason, None, None)
    } else if !ect1_reached_receiver || !ect1_reached_aqm {
        gate(
            "L4S_ECN_PATH_001",
            GateDisposition::Fail,
            "ect1_not_observed_at_receiver_and_aqm",
            None,
            None,
        )
    } else if !ecn_counters_consistent {
        gate(
            "L4S_ECN_PATH_001",
            GateDisposition::Fail,
            "ce_marks_exceed_ect1_packets_observed_at_aqm",
            None,
            None,
        )
    } else if !treatment.ecn.congestion_challenge_verified {
        gate(
            "L4S_ECN_PATH_001",
            GateDisposition::Inconclusive,
            "no_frozen_congestion_challenge_to_verify_ce_feedback",
            None,
            None,
        )
    } else if !ce_feedback_path {
        gate(
            "L4S_ECN_PATH_001",
            GateDisposition::Fail,
            "ce_marks_or_sender_feedback_missing_under_congestion_challenge",
            None,
            None,
        )
    } else {
        gate(
            "L4S_ECN_PATH_001",
            GateDisposition::Pass,
            "prague_sender_ect1_and_ce_feedback_verified",
            Some(1),
            Some(u64::from(ecn_ok)),
        )
    };

    let classification_ok = treatment.ect1_packets_classified_to_l4s_queue > 0
        && treatment.classic_packets_classified_to_classic_queue > 0
        && treatment.classic_backlogged_flow_count > 0;
    let classification_result = if classification_ok {
        gate(
            "L4S_AQM_SUPPORT_001",
            GateDisposition::Pass,
            "both_dual_queue_classes_observed_under_mixed_load",
            Some(1),
            Some(1),
        )
    } else {
        gate(
            "L4S_AQM_SUPPORT_001",
            GateDisposition::Fail,
            "l4s_or_classic_queue_missing_under_declared_mixed_load",
            Some(1),
            Some(0),
        )
    };
    Ok((ecn_result, classification_result))
}

/// Evaluate latency and Classic-flow coexistence against a matched
/// Classic-only control. Delay targets only apply to an explicitly verified
/// RFC 9332-like reproduction profile; they are not universal guarantees.
pub fn evaluate_l4s_pair(
    control: &ClassicOnlyControl,
    treatment: &MixedL4sTreatment,
    thresholds: QualificationThresholds,
) -> Result<L4sPairReport, L4sEvidenceError> {
    validate_thresholds(thresholds)?;
    let identities_match = paired_identity_matches(&control.identity, &treatment.identity)?;
    if !identities_match {
        let mismatch = gate(
            "L4S_PAIR_IDENTITY_001",
            GateDisposition::Inconclusive,
            "control_and_treatment_identity_mismatch",
            None,
            None,
        );
        return Ok(L4sPairReport {
            ecn_path: mismatch.clone(),
            queue_classification: mismatch.clone(),
            l4s_queue_delay: mismatch.clone(),
            classic_retention: mismatch.clone(),
            classic_flow_retention: mismatch.clone(),
            classic_starvation: mismatch,
            classic_retention_ratio_ppm: None,
            worst_backlogged_classic_flow_retention_ppm: None,
        });
    }

    control.aqm.validate()?;
    control.classic_sender.validate()?;
    treatment.classic_sender.validate()?;
    if !matches!(control.classic_sender_kind, SenderKind::TcpCubic)
        || !matches!(treatment.classic_sender_kind, SenderKind::TcpCubic)
    {
        let mismatch = gate(
            "L4S_CLASSIC_BASELINE_001",
            GateDisposition::Unsupported,
            "classic_control_is_not_pinned_cubic",
            None,
            None,
        );
        return Ok(L4sPairReport {
            ecn_path: mismatch.clone(),
            queue_classification: mismatch.clone(),
            l4s_queue_delay: mismatch.clone(),
            classic_retention: mismatch.clone(),
            classic_flow_retention: mismatch.clone(),
            classic_starvation: mismatch,
            classic_retention_ratio_ppm: None,
            worst_backlogged_classic_flow_retention_ppm: None,
        });
    }
    let implementations_pinned = implementation_is_pinned(
        &control.aqm,
        &control.identity.kernel_build_sha256,
    )? && implementation_is_pinned(
        &control.classic_sender,
        &control.identity.kernel_build_sha256,
    )? && implementation_is_pinned(
        &treatment.aqm,
        &treatment.identity.kernel_build_sha256,
    )? && implementation_is_pinned(
        &treatment.classic_sender,
        &treatment.identity.kernel_build_sha256,
    )?;
    if !implementations_pinned {
        let mismatch = gate(
            "L4S_PAIR_IMPLEMENTATION_001",
            GateDisposition::Inconclusive,
            "in_tree_or_external_classic_aqm_implementation_not_pinned",
            None,
            None,
        );
        return Ok(L4sPairReport {
            ecn_path: mismatch.clone(),
            queue_classification: mismatch.clone(),
            l4s_queue_delay: mismatch.clone(),
            classic_retention: mismatch.clone(),
            classic_flow_retention: mismatch.clone(),
            classic_starvation: mismatch,
            classic_retention_ratio_ppm: None,
            worst_backlogged_classic_flow_retention_ppm: None,
        });
    }

    if control.aqm_kind != treatment.aqm_kind
        || control.aqm != treatment.aqm
        || control.classic_sender != treatment.classic_sender
    {
        let mismatch = gate(
            "L4S_PAIR_IMPLEMENTATION_001",
            GateDisposition::Inconclusive,
            "paired_aqm_or_classic_sender_implementation_mismatch",
            None,
            None,
        );
        return Ok(L4sPairReport {
            ecn_path: mismatch.clone(),
            queue_classification: mismatch.clone(),
            l4s_queue_delay: mismatch.clone(),
            classic_retention: mismatch.clone(),
            classic_flow_retention: mismatch.clone(),
            classic_starvation: mismatch,
            classic_retention_ratio_ppm: None,
            worst_backlogged_classic_flow_retention_ppm: None,
        });
    }
    if !control.qdisc_readback_verified
        || !control.queue_counters_present
        || !control.cleanup_verified
        || !treatment.qdisc_readback_verified
        || !treatment.queue_counters_present
        || !treatment.cleanup_verified
    {
        let failure = gate(
            "L4S_PAIR_EVIDENCE_001",
            GateDisposition::Inconclusive,
            "paired_queue_readback_counters_or_cleanup_evidence_missing",
            None,
            None,
        );
        return Ok(L4sPairReport {
            ecn_path: failure.clone(),
            queue_classification: failure.clone(),
            l4s_queue_delay: failure.clone(),
            classic_retention: failure.clone(),
            classic_starvation: failure,
            classic_retention_ratio_ppm: None,
        });
    }
    if control.classic_goodput_bps == 0
        || control.classic_backlogged_flow_count == 0
        || control.classic_backlogged_flow_count != treatment.classic_backlogged_flow_count
    {
        let failure = gate(
            "L4S_CLASSIC_RETENTION_001",
            GateDisposition::Inconclusive,
            "classic_control_has_no_backlogged_goodput_reference",
            Some(thresholds.classic_retention_target_ppm),
            None,
        );
        return Ok(L4sPairReport {
            ecn_path: gate(
                "L4S_ECN_PATH_001",
                GateDisposition::Inconclusive,
                "classic_control_reference_unavailable",
                None,
                None,
            ),
            queue_classification: gate(
                "L4S_AQM_SUPPORT_001",
                GateDisposition::Inconclusive,
                "classic_control_reference_unavailable",
                None,
                None,
            ),
            l4s_queue_delay: gate(
                "L4S_QUEUE_DELAY_001",
                GateDisposition::Inconclusive,
                "classic_control_reference_unavailable",
                None,
                None,
            ),
            classic_retention: failure.clone(),
            classic_flow_retention: failure.clone(),
            classic_starvation: failure,
            classic_retention_ratio_ppm: None,
            worst_backlogged_classic_flow_retention_ppm: None,
        });
    }

    let (ecn_path, queue_classification) = evaluate_l4s_ecn_and_aqm(treatment)?;
    let retention_ratio = u64::try_from(
        u128::from(treatment.classic_goodput_bps)
            .checked_mul(PPM)
            .ok_or(L4sEvidenceError::ArithmeticOverflow)?
            / u128::from(control.classic_goodput_bps),
    )
    .map_err(|_| L4sEvidenceError::ArithmeticOverflow)?;
    let retention = if retention_ratio >= thresholds.classic_retention_target_ppm {
        gate(
            "L4S_CLASSIC_RETENTION_001",
            GateDisposition::Pass,
            "classic_cubic_retention_meets_target",
            Some(thresholds.classic_retention_target_ppm),
            Some(retention_ratio),
        )
    } else if retention_ratio < thresholds.classic_retention_investigation_floor_ppm {
        gate(
            "L4S_CLASSIC_RETENTION_001",
            GateDisposition::Fail,
            "classic_cubic_retention_below_investigation_floor",
            Some(thresholds.classic_retention_target_ppm),
            Some(retention_ratio),
        )
    } else {
        gate(
            "L4S_CLASSIC_RETENTION_001",
            GateDisposition::Inconclusive,
            "classic_cubic_retention_below_target_but_above_investigation_floor",
            Some(thresholds.classic_retention_target_ppm),
            Some(retention_ratio),
        )
    };

    let mut seen_classic_flow_ids = std::collections::BTreeSet::new();
    let mut paired_control_goodput = 0_u64;
    let mut paired_treatment_goodput = 0_u64;
    let mut worst_backlogged_classic_flow_retention_ppm: Option<u64> = None;
    let mut per_flow_starvation = false;
    for flow in &treatment.classic_flow_pairs {
        if flow.flow_id.trim().is_empty() || !seen_classic_flow_ids.insert(flow.flow_id.as_str()) {
            let failure = gate(
                "L4S_CLASSIC_FLOW_RETENTION_001",
                GateDisposition::Inconclusive,
                "classic_flow_pair_identity_missing_or_duplicated",
                None,
                None,
            );
            return Ok(L4sPairReport {
                ecn_path: ecn_path.clone(),
                queue_classification: queue_classification.clone(),
                l4s_queue_delay: gate(
                    "L4S_QUEUE_DELAY_001",
                    GateDisposition::Inconclusive,
                    "classic_flow_pair_identity_invalid",
                    None,
                    None,
                ),
                classic_retention: retention.clone(),
                classic_flow_retention: failure.clone(),
                classic_starvation: failure,
                classic_retention_ratio_ppm: Some(retention_ratio),
                worst_backlogged_classic_flow_retention_ppm: None,
            });
        }
        paired_control_goodput = paired_control_goodput
            .checked_add(flow.control_goodput_bps)
            .ok_or(L4sEvidenceError::ArithmeticOverflow)?;
        paired_treatment_goodput = paired_treatment_goodput
            .checked_add(flow.treatment_goodput_bps)
            .ok_or(L4sEvidenceError::ArithmeticOverflow)?;
        if flow.backlogged {
            if flow.control_goodput_bps == 0 {
                let failure = gate(
                    "L4S_CLASSIC_FLOW_RETENTION_001",
                    GateDisposition::Inconclusive,
                    "backlogged_classic_flow_has_no_control_goodput_reference",
                    Some(thresholds.classic_retention_target_ppm),
                    None,
                );
                return Ok(L4sPairReport {
                    ecn_path: ecn_path.clone(),
                    queue_classification: queue_classification.clone(),
                    l4s_queue_delay: latency_placeholder(),
                    classic_retention: retention.clone(),
                    classic_flow_retention: failure.clone(),
                    classic_starvation: failure,
                    classic_retention_ratio_ppm: Some(retention_ratio),
                    worst_backlogged_classic_flow_retention_ppm: None,
                });
            }
            let ratio = u64::try_from(
                u128::from(flow.treatment_goodput_bps)
                    .checked_mul(PPM)
                    .ok_or(L4sEvidenceError::ArithmeticOverflow)?
                    / u128::from(flow.control_goodput_bps),
            )
            .map_err(|_| L4sEvidenceError::ArithmeticOverflow)?;
            worst_backlogged_classic_flow_retention_ppm =
                Some(worst_backlogged_classic_flow_retention_ppm.map_or(ratio, |current| current.min(ratio)));
            per_flow_starvation |= flow.zero_service_duration_us > 0;
        } else if flow.zero_service_duration_us > 0 {
            per_flow_starvation = true;
        }
    }

    if paired_control_goodput != control.classic_goodput_bps
        || paired_treatment_goodput != treatment.classic_goodput_bps
        || treatment.classic_flow_pairs
            .iter()
            .filter(|flow| flow.backlogged)
            .count() as u32
            != control.classic_backlogged_flow_count
    {
        let failure = gate(
            "L4S_CLASSIC_FLOW_RETENTION_001",
            GateDisposition::Inconclusive,
            "per_flow_records_do_not_reconcile_to_aggregate_goodput_or_backlog",
            None,
            None,
        );
        return Ok(L4sPairReport {
            ecn_path: ecn_path.clone(),
            queue_classification: queue_classification.clone(),
            l4s_queue_delay: latency_placeholder(),
            classic_retention: retention.clone(),
            classic_flow_retention: failure.clone(),
            classic_starvation: failure,
            classic_retention_ratio_ppm: Some(retention_ratio),
            worst_backlogged_classic_flow_retention_ppm,
        });
    }

    let classic_flow_retention = match worst_backlogged_classic_flow_retention_ppm {
        Some(ratio) if ratio >= thresholds.classic_retention_target_ppm => gate(
            "L4S_CLASSIC_FLOW_RETENTION_001",
            GateDisposition::Pass,
            "worst_backlogged_classic_flow_retention_meets_target",
            Some(thresholds.classic_retention_target_ppm),
            Some(ratio),
        ),
        Some(ratio) if ratio < thresholds.classic_retention_investigation_floor_ppm => gate(
            "L4S_CLASSIC_FLOW_RETENTION_001",
            GateDisposition::Fail,
            "worst_backlogged_classic_flow_retention_below_investigation_floor",
            Some(thresholds.classic_retention_target_ppm),
            Some(ratio),
        ),
        Some(ratio) => gate(
            "L4S_CLASSIC_FLOW_RETENTION_001",
            GateDisposition::Inconclusive,
            "worst_backlogged_classic_flow_retention_below_target",
            Some(thresholds.classic_retention_target_ppm),
            Some(ratio),
        ),
        None => gate(
            "L4S_CLASSIC_FLOW_RETENTION_001",
            GateDisposition::Inconclusive,
            "no_backlogged_classic_flow_measurements",
            Some(thresholds.classic_retention_target_ppm),
            None,
        ),
    };

    let starvation = if per_flow_starvation || treatment.classic_zero_service_duration_us > 0
    {
        gate(
            "L4S_CLASSIC_STARVATION_001",
            GateDisposition::Fail,
            "backlogged_classic_flow_experienced_zero_service",
            Some(0),
            Some(treatment.classic_zero_service_duration_us),
        )
    } else {
        gate(
            "L4S_CLASSIC_STARVATION_001",
            GateDisposition::Pass,
            "no_zero_service_interval_reported_for_backlogged_classic_flows",
            Some(0),
            Some(treatment.classic_zero_service_duration_us),
        )
    };

    let latency = if !treatment.rfc9332_reproduction_profile_verified {
        gate(
            "L4S_QUEUE_DELAY_001",
            GateDisposition::Inconclusive,
            "rfc9332_reproduction_conditions_not_verified",
            Some(thresholds.l4s_p99_queue_delay_target_us),
            Some(treatment.l4s_queue_delay_p99_us),
        )
    } else if treatment.l4s_queue_delay_average_us
        <= thresholds.l4s_average_queue_delay_target_us
        && treatment.l4s_queue_delay_p99_us <= thresholds.l4s_p99_queue_delay_target_us
    {
        gate(
            "L4S_QUEUE_DELAY_001",
            GateDisposition::Pass,
            "l4s_queue_delay_meets_frozen_reproduction_targets",
            Some(thresholds.l4s_p99_queue_delay_target_us),
            Some(treatment.l4s_queue_delay_p99_us),
        )
    } else {
        gate(
            "L4S_QUEUE_DELAY_001",
            GateDisposition::Fail,
            "l4s_queue_delay_exceeds_frozen_reproduction_target",
            Some(thresholds.l4s_p99_queue_delay_target_us),
            Some(treatment.l4s_queue_delay_p99_us),
        )
    };

    Ok(L4sPairReport {
        ecn_path,
        queue_classification,
        l4s_queue_delay: latency,
        classic_retention: retention,
        classic_flow_retention,
        classic_starvation: starvation,
        classic_retention_ratio_ppm: Some(retention_ratio),
        worst_backlogged_classic_flow_retention_ppm,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity() -> PairIdentity {
        PairIdentity {
            kernel_build_sha256: "a".repeat(64),
            topology_sha256: "b".repeat(64),
            qdisc_config_sha256: "c".repeat(64),
            classic_workload_sha256: "d".repeat(64),
            l4s_workload_sha256: "e".repeat(64),
            path_impairment_sha256: "f".repeat(64),
            endpoint_config_sha256: "1".repeat(64),
            ecn_path_sha256: "2".repeat(64),
            seed: 17,
            warmup_us: 2_000_000,
            measurement_duration_us: 30_000_000,
            link_rate_bps: 100_000_000,
            base_rtt_us: 15_000,
        }
    }

    fn in_tree(name: &str) -> ImplementationIdentity {
        ImplementationIdentity {
            name: name.to_owned(),
            provenance: ImplementationProvenance::InTreeKernel,
        }
    }

    fn external(name: &str) -> ImplementationIdentity {
        ImplementationIdentity {
            name: name.to_owned(),
            provenance: ImplementationProvenance::ExternalArtifact {
                artifact_sha256: "3".repeat(64),
            },
        }
    }

    fn control() -> ClassicOnlyControl {
        ClassicOnlyControl {
            identity: identity(),
            aqm_kind: AqmKind::DualPi2,
            aqm: in_tree("dualpi2"),
            classic_sender: in_tree("cubic"),
            classic_sender_kind: SenderKind::TcpCubic,
            qdisc_readback_verified: true,
            queue_counters_present: true,
            cleanup_verified: true,
            classic_goodput_bps: 45_000_000,
            classic_backlogged_flow_count: 2,
        }
    }

    fn treatment() -> MixedL4sTreatment {
        MixedL4sTreatment {
            identity: identity(),
            aqm_kind: AqmKind::DualPi2,
            aqm: in_tree("dualpi2"),
            classic_sender: in_tree("cubic"),
            classic_sender_kind: SenderKind::TcpCubic,
            l4s_sender: external("tcp-prague"),
            l4s_sender_kind: SenderKind::TcpPrague,
            qdisc_readback_verified: true,
            cleanup_verified: true,
            queue_counters_present: true,
            ect1_packets_classified_to_l4s_queue: 10_000,
            classic_packets_classified_to_classic_queue: 12_000,
            classic_backlogged_flow_count: 2,
            classic_flow_pairs: vec![
                ClassicFlowPair {
                    flow_id: "classic-a".to_owned(),
                    backlogged: true,
                    control_goodput_bps: 22_500_000,
                    treatment_goodput_bps: 22_000_000,
                    zero_service_duration_us: 0,
                },
                ClassicFlowPair {
                    flow_id: "classic-b".to_owned(),
                    backlogged: true,
                    control_goodput_bps: 22_500_000,
                    treatment_goodput_bps: 22_000_000,
                    zero_service_duration_us: 0,
                },
            ],
            ecn: EcnPathEvidence {
                ecn_negotiated_at_sender: true,
                ecn_negotiated_at_receiver: true,
                feedback_mode: EcnFeedbackMode::TcpAccEcn,
                ect1_packets_sent_by_sender: 10_000,
                ect1_packets_received_at_receiver: 9_950,
                ect1_packets_observed_at_aqm: 9_900,
                ce_marks_at_aqm: 1_000,
                ce_feedback_received_at_sender: 950,
                congestion_challenge_verified: true,
                counters_complete: true,
                sender_fallback_detected: false,
            },
            classic_goodput_bps: 44_000_000,
            l4s_goodput_bps: 30_000_000,
            l4s_queue_delay_average_us: 700,
            l4s_queue_delay_p99_us: 1_800,
            classic_queue_delay_p99_us: 12_000,
            classic_zero_service_duration_us: 0,
            rfc9332_reproduction_profile_verified: true,
        }
    }

    #[test]
    fn matched_mixed_prague_dualpi2_pair_passes_authenticity_and_targets() {
        let result =
            evaluate_l4s_pair(&control(), &treatment(), QualificationThresholds::default())
                .expect("valid pair");
        assert_eq!(result.ecn_path.disposition, GateDisposition::Pass);
        assert_eq!(result.queue_classification.disposition, GateDisposition::Pass);
        assert_eq!(result.l4s_queue_delay.disposition, GateDisposition::Pass);
        assert_eq!(result.classic_retention.disposition, GateDisposition::Pass);
        assert_eq!(result.classic_flow_retention.disposition, GateDisposition::Pass);
        assert_eq!(result.classic_retention_ratio_ppm, Some(977_777));
        assert_eq!(result.worst_backlogged_classic_flow_retention_ppm, Some(977_777));
        assert_eq!(result.classic_starvation.disposition, GateDisposition::Pass);
    }

    #[test]
    fn dctcp_is_not_accepted_as_a_public_internet_prague_sender() {
        let mut trial = treatment();
        trial.l4s_sender_kind = SenderKind::Dctcp;
        let (ecn, aqm) = evaluate_l4s_ecn_and_aqm(&trial).expect("unsupported arm classified");
        assert_eq!(ecn.disposition, GateDisposition::Unsupported);
        assert_eq!(aqm.disposition, GateDisposition::Unsupported);
    }

    #[test]
    fn classic_only_qdisc_is_not_an_l4s_aqm() {
        let mut trial = treatment();
        trial.aqm_kind = AqmKind::ClassicOnly;
        let (ecn, aqm) = evaluate_l4s_ecn_and_aqm(&trial).expect("unsupported arm classified");
        assert_eq!(ecn.disposition, GateDisposition::Unsupported);
        assert_eq!(aqm.disposition, GateDisposition::Unsupported);
    }

    #[test]
    fn ect1_bleaching_is_a_failure() {
        let mut trial = treatment();
        trial.ecn.ect1_packets_received_at_receiver = 0;
        let (ecn, _) = evaluate_l4s_ecn_and_aqm(&trial).expect("valid evidence shape");
        assert_eq!(ecn.disposition, GateDisposition::Fail);
        assert_eq!(
            ecn.reason,
            "ect1_not_observed_at_both_sender_and_receiver"
        );
    }

    #[test]
    fn ce_feedback_missing_under_verified_congestion_is_a_failure() {
        let mut trial = treatment();
        trial.ecn.ce_feedback_received_at_sender = 0;
        let (ecn, _) = evaluate_l4s_ecn_and_aqm(&trial).expect("valid evidence shape");
        assert_eq!(ecn.disposition, GateDisposition::Fail);
    }

    #[test]
    fn absent_congestion_challenge_is_inconclusive() {
        let mut trial = treatment();
        trial.ecn.congestion_challenge_verified = false;
        let (ecn, _) = evaluate_l4s_ecn_and_aqm(&trial).expect("valid evidence shape");
        assert_eq!(ecn.disposition, GateDisposition::Inconclusive);
    }

    #[test]
    fn sender_fallback_cannot_pass() {
        let mut trial = treatment();
        trial.ecn.sender_fallback_detected = true;
        let (ecn, _) = evaluate_l4s_ecn_and_aqm(&trial).expect("valid evidence shape");
        assert_eq!(ecn.disposition, GateDisposition::Fail);
    }

    #[test]
    fn one_missing_queue_class_fails_mixed_classification() {
        let mut trial = treatment();
        trial.ect1_packets_classified_to_l4s_queue = 0;
        let (_, classification) =
            evaluate_l4s_ecn_and_aqm(&trial).expect("valid evidence shape");
        assert_eq!(classification.disposition, GateDisposition::Fail);
    }

    #[test]
    fn mismatched_qdisc_or_workload_identity_is_inconclusive() {
        let mut mixed = treatment();
        mixed.identity.qdisc_config_sha256 = "9".repeat(64);
        let result = evaluate_l4s_pair(&control(), &mixed, QualificationThresholds::default())
            .expect("mismatched pair classified");
        assert_eq!(
            result.classic_retention.disposition,
            GateDisposition::Inconclusive
        );
    }

    #[test]
    fn classic_retention_between_target_and_floor_is_inconclusive() {
        let mut mixed = treatment();
        mixed.classic_goodput_bps = 42_000_000;
        mixed.classic_flow_pairs[0].treatment_goodput_bps = 21_000_000;
        mixed.classic_flow_pairs[1].treatment_goodput_bps = 21_000_000;
        let result = evaluate_l4s_pair(&control(), &mixed, QualificationThresholds::default())
            .expect("valid pair");
        assert_eq!(
            result.classic_retention.disposition,
            GateDisposition::Inconclusive
        );
    }

    #[test]
    fn classic_retention_below_investigation_floor_fails() {
        let mut mixed = treatment();
        mixed.classic_goodput_bps = 30_000_000;
        mixed.classic_flow_pairs[0].treatment_goodput_bps = 15_000_000;
        mixed.classic_flow_pairs[1].treatment_goodput_bps = 15_000_000;
        let result = evaluate_l4s_pair(&control(), &mixed, QualificationThresholds::default())
            .expect("valid pair");
        assert_eq!(result.classic_retention.disposition, GateDisposition::Fail);
    }

    #[test]
    fn no_zero_service_is_required_for_backlogged_classic_flows() {
        let mut mixed = treatment();
        mixed.classic_zero_service_duration_us = 10;
        mixed.classic_flow_pairs[0].zero_service_duration_us = 10;
        let result = evaluate_l4s_pair(&control(), &mixed, QualificationThresholds::default())
            .expect("valid pair");
        assert_eq!(result.classic_starvation.disposition, GateDisposition::Fail);
    }

    #[test]
    fn queue_delay_targets_are_not_universal_without_reproduction_conditions() {
        let mut mixed = treatment();
        mixed.rfc9332_reproduction_profile_verified = false;
        let result = evaluate_l4s_pair(&control(), &mixed, QualificationThresholds::default())
            .expect("valid pair");
        assert_eq!(
            result.l4s_queue_delay.disposition,
            GateDisposition::Inconclusive
        );
    }

    #[test]
    fn external_prague_sender_requires_artifact_digest() {
        let mut mixed = treatment();
        mixed.l4s_sender.provenance = ImplementationProvenance::ExternalArtifact {
            artifact_sha256: "not-a-digest".to_owned(),
        };
        assert_eq!(
            evaluate_l4s_ecn_and_aqm(&mixed),
            Err(L4sEvidenceError::InvalidImplementationArtifact)
        );
    }

    #[test]
    fn tcp_prague_requires_accurate_ecn_feedback_not_only_classic_ecn() {
        let mut trial = treatment();
        trial.ecn.feedback_mode = EcnFeedbackMode::ClassicEcnOnly;
        let (ecn, _) = evaluate_l4s_ecn_and_aqm(&trial).expect("feedback mode classified");
        assert_eq!(ecn.disposition, GateDisposition::Fail);
        assert_eq!(
            ecn.reason,
            "classic_ecn_feedback_is_not_sufficient_for_scalable_sender"
        );
    }

    #[test]
    fn ce_counter_cannot_exceed_ect1_packets_observed_at_aqm() {
        let mut trial = treatment();
        trial.ecn.ce_marks_at_aqm = 10_001;
        let (ecn, _) = evaluate_l4s_ecn_and_aqm(&trial).expect("counter consistency checked");
        assert_eq!(ecn.disposition, GateDisposition::Fail);
        assert_eq!(ecn.reason, "ce_marks_exceed_ect1_packets_observed_at_aqm");
    }

    #[test]
    fn per_flow_retention_catches_starvation_hidden_by_aggregate_goodput() {
        let mut mixed = treatment();
        mixed.classic_goodput_bps = 44_000_000;
        mixed.classic_flow_pairs[0].treatment_goodput_bps = 0;
        mixed.classic_flow_pairs[0].zero_service_duration_us = 100_000;
        mixed.classic_flow_pairs[1].treatment_goodput_bps = 44_000_000;
        let result = evaluate_l4s_pair(&control(), &mixed, QualificationThresholds::default())
            .expect("paired per-flow evidence");
        assert_eq!(result.classic_retention.disposition, GateDisposition::Pass);
        assert_eq!(result.classic_flow_retention.disposition, GateDisposition::Fail);
        assert_eq!(result.classic_starvation.disposition, GateDisposition::Fail);
    }

    #[test]
    fn quic_prague_requires_quic_ecn_count_feedback() {
        let mut trial = treatment();
        trial.l4s_sender_kind = SenderKind::QuicPrague;
        trial.ecn.feedback_mode = EcnFeedbackMode::QuicEcnCounts;
        let (ecn, aqm) = evaluate_l4s_ecn_and_aqm(&trial).expect("QUIC feedback mode");
        assert_eq!(ecn.disposition, GateDisposition::Pass);
        assert_eq!(aqm.disposition, GateDisposition::Pass);
    }

    #[test]
    fn in_tree_classic_sender_and_aqm_provenance_is_checked_on_both_arms() {
        let mut control_trial = control();
        control_trial.aqm.provenance = ImplementationProvenance::ExternalArtifact {
            artifact_sha256: "bad".to_owned(),
        };
        assert_eq!(
            evaluate_l4s_pair(&control_trial, &treatment(), QualificationThresholds::default()),
            Err(L4sEvidenceError::InvalidImplementationArtifact)
        );
    }

    #[test]
    fn exact_pair_has_single_paired_trial_scope() {
        let result =
            evaluate_l4s_pair(&control(), &treatment(), QualificationThresholds::default())
                .expect("valid pair");
        assert_eq!(result.ecn_path.scope, "single_paired_trial");
    }
}
