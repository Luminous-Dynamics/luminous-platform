use crate::AuditError;
use std::collections::HashSet;

const CLAIM_CEILING: &str =
    "DISPATCH_EPISODE_OUTCOME_AND_PAIRED_STRUCTURE_ONLY_NOT_SUPERIORITY_OR_CONFORMITY";
const MAX_CAMPAIGN_EPISODES: usize = 100_000;
const ENERGY_ABSOLUTE_TOLERANCE_WH: f64 = 1.0e-6;
const ENERGY_RELATIVE_TOLERANCE: f64 = 1.0e-9;

fn energy_reconciliation_tolerance_wh(required_wh: f64) -> f64 {
    ENERGY_ABSOLUTE_TOLERANCE_WH.max(required_wh.abs() * ENERGY_RELATIVE_TOLERANCE)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchEvidenceClass {
    Synthetic,
    Modelled,
    Measured,
    IndependentlyWitnessed,
    Accredited,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchTestEnvironment {
    Synthetic,
    Simulation,
    ControllerHil,
    PowerHil,
    ControlledLaboratory,
    Commissioning,
    Periodic,
    SiteOperational,
    FieldMission,
}

impl DispatchTestEnvironment {
    const fn is_physical_test(self) -> bool {
        matches!(
            self,
            Self::ControllerHil
                | Self::PowerHil
                | Self::ControlledLaboratory
                | Self::Commissioning
                | Self::Periodic
                | Self::SiteOperational
                | Self::FieldMission
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchGateState {
    Pass,
    Fail,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchEpisodeDisposition {
    Pass,
    Fail,
    Inconclusive,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispatchHardGates {
    pub critical_service: DispatchGateState,
    pub qualified_capability: DispatchGateState,
    pub terminal_reserve: DispatchGateState,
    pub telemetry_integrity: DispatchGateState,
    pub protection_readiness: DispatchGateState,
    pub required_power_events: DispatchGateState,
}

impl DispatchHardGates {
    pub fn disposition(&self) -> DispatchEpisodeDisposition {
        let states = [
            self.critical_service,
            self.qualified_capability,
            self.terminal_reserve,
            self.telemetry_integrity,
            self.protection_readiness,
            self.required_power_events,
        ];
        if states.contains(&DispatchGateState::Fail) {
            DispatchEpisodeDisposition::Fail
        } else if states.contains(&DispatchGateState::Unknown) {
            DispatchEpisodeDisposition::Inconclusive
        } else {
            DispatchEpisodeDisposition::Pass
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DispatchEpisodeMetrics {
    pub required_critical_service_wh: f64,
    pub delivered_critical_service_wh: f64,
    pub unserved_critical_service_wh: f64,
    pub maximum_unserved_critical_service_wh: f64,
    /// Positive means reserve remains above the terminal obligation.
    /// Negative means the terminal reserve obligation was breached.
    pub terminal_reserve_margin_wh: f64,
    pub fuel_input_energy_wh: f64,
    pub grid_import_energy_wh: f64,
    pub auxiliary_energy_wh: f64,
    pub storage_energy_throughput_wh: f64,
    pub generator_start_count: u32,
    pub decision_count: u64,
    pub missed_decision_deadline_count: u64,
    pub decision_latency_sample_count: u64,
    pub decision_latency_p50_ms: Option<f64>,
    pub decision_latency_p95_ms: Option<f64>,
    pub decision_latency_p99_ms: Option<f64>,
    pub time_to_safe_state_ms: Option<f64>,
    pub recovery_duration_s: Option<f64>,
    pub time_to_terminal_reserve_restoration_s: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DispatchEpisodeOutcome {
    pub scenario_id: String,
    pub scenario_sha256: String,
    /// Digest of the complete matched inputs available to both policy arms.
    pub matched_input_sha256: String,
    pub policy_id: String,
    pub policy_sha256: String,
    pub run_id: String,
    pub evidence_class: DispatchEvidenceClass,
    pub test_environment: DispatchTestEnvironment,
    pub evidence_bundle_sha256: String,
    pub raw_trace_sha256: String,
    pub analysis_build_sha256: String,
    pub independent_assessment_reference: Option<String>,
    pub data_rights_reference: String,
    pub hard_gates: DispatchHardGates,
    pub metrics: DispatchEpisodeMetrics,
}

impl DispatchEpisodeOutcome {
    /// Derived only from the six required hard gates. This is not a weighted
    /// score, a statistical result, or an independent physical qualification.
    pub fn disposition(&self) -> DispatchEpisodeDisposition {
        self.hard_gates.disposition()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DispatchOutcomeCampaign {
    pub campaign_id: String,
    pub scenario_corpus_sha256: String,
    pub policy_id: String,
    pub policy_sha256: String,
    pub episodes: Vec<DispatchEpisodeOutcome>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispatchGateCounts {
    pub pass: usize,
    pub fail: usize,
    pub unknown: usize,
}

impl DispatchGateCounts {
    fn empty() -> Self {
        Self { pass: 0, fail: 0, unknown: 0 }
    }

    fn add(&mut self, state: DispatchGateState) {
        match state {
            DispatchGateState::Pass => self.pass += 1,
            DispatchGateState::Fail => self.fail += 1,
            DispatchGateState::Unknown => self.unknown += 1,
        }
    }

    fn total(&self) -> usize {
        self.pass + self.fail + self.unknown
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispatchCampaignSummary {
    pub episode_count: usize,
    pub pass_count: usize,
    pub fail_count: usize,
    pub inconclusive_count: usize,
    pub critical_service: DispatchGateCounts,
    pub qualified_capability: DispatchGateCounts,
    pub terminal_reserve: DispatchGateCounts,
    pub telemetry_integrity: DispatchGateCounts,
    pub protection_readiness: DispatchGateCounts,
    pub required_power_events: DispatchGateCounts,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DispatchPairedEpisode {
    pub scenario_id: String,
    pub reference: DispatchEpisodeOutcome,
    pub candidate: DispatchEpisodeOutcome,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DispatchCampaignComparisonReport {
    pub reference_campaign: DispatchOutcomeCampaign,
    pub candidate_campaign: DispatchOutcomeCampaign,
    pub structural_match: bool,
    pub blockers: Vec<String>,
    pub reference_summary: DispatchCampaignSummary,
    pub candidate_summary: DispatchCampaignSummary,
    pub paired_episodes: Vec<DispatchPairedEpisode>,
    pub claim_ceiling: &'static str,
}

impl DispatchCampaignComparisonReport {
    /// No aggregate rank or performance direction is inferred.
    pub const fn claim_ceiling(&self) -> &'static str {
        CLAIM_CEILING
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispatchOutcomeValidationReport {
    pub episode_count: usize,
    pub pass_count: usize,
    pub fail_count: usize,
    pub inconclusive_count: usize,
    pub claim_ceiling: &'static str,
}

fn nonblank(value: &str) -> bool {
    !value.trim().is_empty()
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn finite_nonnegative(value: f64) -> bool {
    value.is_finite() && value >= 0.0
}

fn valid_environment_classification(
    evidence: DispatchEvidenceClass,
    environment: DispatchTestEnvironment,
) -> bool {
    match evidence {
        DispatchEvidenceClass::Synthetic => environment == DispatchTestEnvironment::Synthetic,
        DispatchEvidenceClass::Modelled => environment == DispatchTestEnvironment::Simulation,
        DispatchEvidenceClass::Measured
        | DispatchEvidenceClass::IndependentlyWitnessed
        | DispatchEvidenceClass::Accredited => environment.is_physical_test(),
    }
}

fn optional_finite_nonnegative(value: Option<f64>) -> bool {
    value.is_none_or(finite_nonnegative)
}

fn validate_metrics(metrics: &DispatchEpisodeMetrics) -> Result<(), AuditError> {
    let required = metrics.required_critical_service_wh;
    let delivered = metrics.delivered_critical_service_wh;
    let unserved = metrics.unserved_critical_service_wh;
    let max_unserved = metrics.maximum_unserved_critical_service_wh;
    if !finite_nonnegative(required)
        || !finite_nonnegative(delivered)
        || !finite_nonnegative(unserved)
        || !finite_nonnegative(max_unserved)
        || !finite_nonnegative(tolerance)
    {
        return Err(AuditError(
            "critical-service energy and reconciliation tolerance must be finite and non-negative"
                .to_string(),
        ));
    }
    let reconciled = delivered + unserved;
    let tolerance = energy_reconciliation_tolerance_wh(required);
    if !reconciled.is_finite() || (reconciled - required).abs() > tolerance {
        return Err(AuditError(
            "required critical energy must reconcile to delivered plus unserved energy within the declared tolerance"
                .to_string(),
        ));
    }
    if unserved > required + tolerance
        || max_unserved > required + tolerance
    {
        return Err(AuditError(
            "unserved critical energy or its allowance cannot exceed required service energy"
                .to_string(),
        ));
    }

    if !metrics.terminal_reserve_margin_wh.is_finite() {
        return Err(AuditError(
            "terminal reserve margin must be finite".to_string(),
        ));
    }
    for value in [
        metrics.fuel_input_energy_wh,
        metrics.grid_import_energy_wh,
        metrics.auxiliary_energy_wh,
        metrics.storage_energy_throughput_wh,
    ] {
        if !finite_nonnegative(value) {
            return Err(AuditError(
                "resource energy channels must be finite and non-negative".to_string(),
            ));
        }
    }
    if metrics.missed_decision_deadline_count > metrics.decision_count
        || metrics.decision_latency_sample_count > metrics.decision_count
    {
        return Err(AuditError(
            "missed deadlines and latency samples cannot exceed decision count".to_string(),
        ));
    }

    let latency = [
        metrics.decision_latency_p50_ms,
        metrics.decision_latency_p95_ms,
        metrics.decision_latency_p99_ms,
    ];
    let present = latency.iter().filter(|value| value.is_some()).count();
    if present != 0 && present != 3 {
        return Err(AuditError(
            "decision latency quantiles must be either all present or all absent".to_string(),
        ));
    }
    if let [Some(p50), Some(p95), Some(p99)] = latency {
        if !finite_nonnegative(p50)
            || !finite_nonnegative(p95)
            || !finite_nonnegative(p99)
            || p50 > p95
            || p95 > p99
            || metrics.decision_latency_sample_count == 0
        {
            return Err(AuditError(
                "decision latency quantiles or their sample count are invalid".to_string(),
            ));
        }
    } else if metrics.decision_latency_sample_count != 0 {
        return Err(AuditError(
            "latency sample count is non-zero but quantiles are missing".to_string(),
        ));
    }

    for value in [
        metrics.time_to_safe_state_ms,
        metrics.recovery_duration_s,
        metrics.time_to_terminal_reserve_restoration_s,
    ] {
        if !optional_finite_nonnegative(value) {
            return Err(AuditError(
                "recovery and safe-state timing values must be finite and non-negative"
                    .to_string(),
            ));
        }
    }
    Ok(())
}

pub fn validate_dispatch_episode_outcome(
    outcome: &DispatchEpisodeOutcome,
) -> Result<DispatchEpisodeDisposition, AuditError> {
    if !nonblank(&outcome.scenario_id)
        || !nonblank(&outcome.policy_id)
        || !nonblank(&outcome.run_id)
        || !nonblank(&outcome.data_rights_reference)
    {
        return Err(AuditError(
            "scenario, policy, run and data-rights identities must be non-empty".to_string(),
        ));
    }
    for digest in [
        outcome.scenario_sha256.as_str(),
        outcome.matched_input_sha256.as_str(),
        outcome.policy_sha256.as_str(),
        outcome.evidence_bundle_sha256.as_str(),
        outcome.raw_trace_sha256.as_str(),
        outcome.analysis_build_sha256.as_str(),
    ] {
        if !valid_sha256(digest) {
            return Err(AuditError(
                "episode result contains an invalid SHA-256 digest".to_string(),
            ));
        }
    }
    if !valid_environment_classification(outcome.evidence_class, outcome.test_environment) {
        return Err(AuditError(
            "evidence class and test environment are contradictory".to_string(),
        ));
    }
    if matches!(
        outcome.evidence_class,
        DispatchEvidenceClass::IndependentlyWitnessed | DispatchEvidenceClass::Accredited
    ) && outcome
        .independent_assessment_reference
        .as_deref()
        .is_none_or(|value| !nonblank(value))
    {
        return Err(AuditError(
            "independently witnessed or accredited outcomes require an assessment reference"
                .to_string(),
        ));
    }
    if outcome
        .independent_assessment_reference
        .as_deref()
        .is_some_and(|value| !nonblank(value))
    {
        return Err(AuditError(
            "independent assessment reference cannot be blank".to_string(),
        ));
    }
    validate_metrics(&outcome.metrics)?;

    if outcome.hard_gates.critical_service == DispatchGateState::Pass
        && outcome.metrics.unserved_critical_service_wh
            > outcome.metrics.maximum_unserved_critical_service_wh
                + energy_reconciliation_tolerance_wh(outcome.metrics.required_critical_service_wh)
    {
        return Err(AuditError(
            "critical-service gate cannot pass when unserved energy exceeds its allowance"
                .to_string(),
        ));
    }
    if outcome.hard_gates.terminal_reserve == DispatchGateState::Pass
        && outcome.metrics.terminal_reserve_margin_wh < -ENERGY_ABSOLUTE_TOLERANCE_WH
    {
        return Err(AuditError(
            "terminal-reserve gate cannot pass with a negative reserve margin".to_string(),
        ));
    }

    Ok(outcome.disposition())
}

fn summary(campaign: &DispatchOutcomeCampaign) -> DispatchCampaignSummary {
    let mut result = DispatchCampaignSummary {
        episode_count: campaign.episodes.len(),
        pass_count: 0,
        fail_count: 0,
        inconclusive_count: 0,
        critical_service: DispatchGateCounts::empty(),
        qualified_capability: DispatchGateCounts::empty(),
        terminal_reserve: DispatchGateCounts::empty(),
        telemetry_integrity: DispatchGateCounts::empty(),
        protection_readiness: DispatchGateCounts::empty(),
        required_power_events: DispatchGateCounts::empty(),
    };
    for episode in &campaign.episodes {
        match episode.disposition() {
            DispatchEpisodeDisposition::Pass => result.pass_count += 1,
            DispatchEpisodeDisposition::Fail => result.fail_count += 1,
            DispatchEpisodeDisposition::Inconclusive => result.inconclusive_count += 1,
        }
        result.critical_service.add(episode.hard_gates.critical_service);
        result.qualified_capability.add(episode.hard_gates.qualified_capability);
        result.terminal_reserve.add(episode.hard_gates.terminal_reserve);
        result.telemetry_integrity.add(episode.hard_gates.telemetry_integrity);
        result.protection_readiness.add(episode.hard_gates.protection_readiness);
        result.required_power_events.add(episode.hard_gates.required_power_events);
    }
    result
}

fn validate_campaign(campaign: &DispatchOutcomeCampaign) -> Result<(), AuditError> {
    if !nonblank(&campaign.campaign_id) || !nonblank(&campaign.policy_id)
        || !valid_sha256(&campaign.policy_sha256)
        || !valid_sha256(&campaign.scenario_corpus_sha256)
        || campaign.episodes.is_empty()
        || campaign.episodes.len() > MAX_CAMPAIGN_EPISODES
    {
        return Err(AuditError(
            "campaign identity, digest or episode count is invalid".to_string(),
        ));
    }
    let mut scenario_ids = HashSet::new();
    let mut run_ids = HashSet::new();
    for episode in &campaign.episodes {
        validate_dispatch_episode_outcome(episode)?;
        if episode.policy_id != campaign.policy_id || episode.policy_sha256 != campaign.policy_sha256 {
            return Err(AuditError(
                "episode policy identity does not match its campaign".to_string(),
            ));
        }
        if !scenario_ids.insert(episode.scenario_id.as_str()) {
            return Err(AuditError(
                "campaign contains a duplicate scenario ID".to_string(),
            ));
        }
        if !run_ids.insert(episode.run_id.as_str()) {
            return Err(AuditError(
                "campaign contains a duplicate run ID".to_string(),
            ));
        }
    }
    Ok(())
}

pub fn validate_dispatch_outcome_campaign(
    campaign: &DispatchOutcomeCampaign,
) -> Result<DispatchOutcomeValidationReport, AuditError> {
    validate_campaign(campaign)?;
    let s = summary(campaign);
    for gate in [
        &s.critical_service,
        &s.qualified_capability,
        &s.terminal_reserve,
        &s.telemetry_integrity,
        &s.protection_readiness,
        &s.required_power_events,
    ] {
        if gate.total() != s.episode_count {
            return Err(AuditError(
                "per-gate counts do not reconcile to campaign episode count".to_string(),
            ));
        }
    }
    if s.pass_count + s.fail_count + s.inconclusive_count != s.episode_count {
        return Err(AuditError(
            "episode disposition counts do not reconcile to campaign episode count".to_string(),
        ));
    }
    Ok(DispatchOutcomeValidationReport {
        episode_count: s.episode_count,
        pass_count: s.pass_count,
        fail_count: s.fail_count,
        inconclusive_count: s.inconclusive_count,
        claim_ceiling: CLAIM_CEILING,
    })
}

/// Compare two complete campaigns over exactly the same scenario corpus and
/// per-scenario matched input roots. No scalar performance rank is computed.
pub fn compare_dispatch_outcome_campaigns(
    reference: &DispatchOutcomeCampaign,
    candidate: &DispatchOutcomeCampaign,
) -> Result<DispatchCampaignComparisonReport, AuditError> {
    validate_campaign(reference)?;
    validate_campaign(candidate)?;
    if reference.campaign_id == candidate.campaign_id
        || reference.policy_id == candidate.policy_id
        || reference.policy_sha256 == candidate.policy_sha256
    {
        return Err(AuditError(
            "reference and candidate campaigns and policy revisions must be distinct".to_string(),
        ));
    }

    let mut blockers = Vec::new();
    if reference.scenario_corpus_sha256 != candidate.scenario_corpus_sha256 {
        blockers.push("mismatch:scenario_corpus".to_string());
    }
    let mut paired_episodes = Vec::new();

    for reference_episode in &reference.episodes {
        let Some(candidate_episode) = candidate
            .episodes
            .iter()
            .find(|episode| episode.scenario_id == reference_episode.scenario_id)
        else {
            blockers.push(format!(
                "candidate_scenario_missing:{}",
                reference_episode.scenario_id
            ));
            continue;
        };
        if reference_episode.scenario_sha256 != candidate_episode.scenario_sha256 {
            blockers.push(format!(
                "mismatch:scenario_digest:{}",
                reference_episode.scenario_id
            ));
        }
        if reference_episode.matched_input_sha256 != candidate_episode.matched_input_sha256 {
            blockers.push(format!(
                "mismatch:matched_input:{}",
                reference_episode.scenario_id
            ));
        }
        paired_episodes.push(DispatchPairedEpisode {
            scenario_id: reference_episode.scenario_id.clone(),
            reference: (*reference_episode).clone(),
            candidate: (*candidate_episode).clone(),
        });
    }
    for candidate_episode in &candidate.episodes {
        if !reference
            .episodes
            .iter()
            .any(|episode| episode.scenario_id == candidate_episode.scenario_id)
        {
            blockers.push(format!(
                "reference_scenario_missing:{}",
                candidate_episode.scenario_id
            ));
        }
    }
    paired_episodes.sort_by(|a, b| a.scenario_id.cmp(&b.scenario_id));

    Ok(DispatchCampaignComparisonReport {
        reference_campaign: reference.clone(),
        candidate_campaign: candidate.clone(),
        structural_match: blockers.is_empty(),
        blockers,
        reference_summary: summary(reference),
        candidate_summary: summary(candidate),
        paired_episodes,
        claim_ceiling: CLAIM_CEILING,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        compare_dispatch_outcome_campaigns, validate_dispatch_episode_outcome,
        validate_dispatch_outcome_campaign, DispatchEvidenceClass, DispatchEpisodeDisposition,
        DispatchEpisodeMetrics, DispatchEpisodeOutcome, DispatchGateState, DispatchHardGates,
        DispatchOutcomeCampaign, DispatchTestEnvironment,
    };

    fn digest(ch: char) -> String {
        std::iter::repeat(ch).take(64).collect()
    }

    fn gates() -> DispatchHardGates {
        DispatchHardGates {
            critical_service: DispatchGateState::Pass,
            qualified_capability: DispatchGateState::Pass,
            terminal_reserve: DispatchGateState::Pass,
            telemetry_integrity: DispatchGateState::Pass,
            protection_readiness: DispatchGateState::Pass,
            required_power_events: DispatchGateState::Pass,
        }
    }

    fn metrics() -> DispatchEpisodeMetrics {
        DispatchEpisodeMetrics {
            required_critical_service_wh: 1_000.0,
            delivered_critical_service_wh: 1_000.0,
            unserved_critical_service_wh: 0.0,
            maximum_unserved_critical_service_wh: 0.0,
            terminal_reserve_margin_wh: 50.0,
            fuel_input_energy_wh: 1_200.0,
            grid_import_energy_wh: 0.0,
            auxiliary_energy_wh: 30.0,
            storage_energy_throughput_wh: 500.0,
            generator_start_count: 1,
            decision_count: 100,
            missed_decision_deadline_count: 0,
            decision_latency_sample_count: 100,
            decision_latency_p50_ms: Some(10.0),
            decision_latency_p95_ms: Some(20.0),
            decision_latency_p99_ms: Some(30.0),
            time_to_safe_state_ms: Some(50.0),
            recovery_duration_s: Some(60.0),
            time_to_terminal_reserve_restoration_s: Some(300.0),
        }
    }

    fn episode(
        scenario: &str,
        policy: &str,
        run: &str,
        input_digest: char,
    ) -> DispatchEpisodeOutcome {
        DispatchEpisodeOutcome {
            scenario_id: scenario.to_string(),
            scenario_sha256: digest('1'),
            matched_input_sha256: digest(input_digest),
            policy_id: policy.to_string(),
            policy_sha256: digest(if policy == "fixed" { 'a' } else { 'b' }),
            run_id: run.to_string(),
            evidence_class: DispatchEvidenceClass::Synthetic,
            test_environment: DispatchTestEnvironment::Synthetic,
            evidence_bundle_sha256: digest('2'),
            raw_trace_sha256: digest('3'),
            analysis_build_sha256: digest('4'),
            independent_assessment_reference: None,
            data_rights_reference: "synthetic-fixture-license".to_string(),
            hard_gates: gates(),
            metrics: metrics(),
        }
    }

    fn campaign(policy: &str, run_prefix: &str) -> DispatchOutcomeCampaign {
        DispatchOutcomeCampaign {
            campaign_id: format!("campaign-{policy}"),
            scenario_corpus_sha256: digest('5'),
            policy_id: policy.to_string(),
            policy_sha256: digest(if policy == "fixed" { 'a' } else { 'b' }),
            episodes: vec![
                episode("baseline", policy, &format!("{run_prefix}-1"), '6'),
                episode("outage", policy, &format!("{run_prefix}-2"), '7'),
            ],
        }
    }

    #[test]
    fn all_pass_gates_produce_pass() {
        assert_eq!(
            validate_dispatch_episode_outcome(&episode("baseline", "fixed", "run-1", '6')).unwrap(),
            DispatchEpisodeDisposition::Pass
        );
    }

    #[test]
    fn any_hard_failure_dominates_unknowns() {
        let mut value = episode("baseline", "fixed", "run-1", '6');
        value.hard_gates.telemetry_integrity = DispatchGateState::Unknown;
        value.hard_gates.critical_service = DispatchGateState::Fail;
        assert_eq!(value.disposition(), DispatchEpisodeDisposition::Fail);
    }

    #[test]
    fn unknown_without_failure_is_inconclusive() {
        let mut value = episode("baseline", "fixed", "run-1", '6');
        value.hard_gates.telemetry_integrity = DispatchGateState::Unknown;
        assert_eq!(value.disposition(), DispatchEpisodeDisposition::Inconclusive);
    }

    #[test]
    fn efficiency_cannot_compensate_for_critical_service_failure() {
        let mut value = episode("outage", "mpc", "run-2", '7');
        value.metrics.fuel_input_energy_wh = 1.0;
        value.metrics.unserved_critical_service_wh = 200.0;
        value.metrics.delivered_critical_service_wh = 800.0;
        value.metrics.maximum_unserved_critical_service_wh = 20.0;
        value.hard_gates.critical_service = DispatchGateState::Fail;
        assert_eq!(validate_dispatch_episode_outcome(&value).unwrap(), DispatchEpisodeDisposition::Fail);
    }

    #[test]
    fn rejects_critical_energy_non_reconciliation() {
        let mut value = episode("baseline", "fixed", "run-1", '6');
        value.metrics.delivered_critical_service_wh = 900.0;
        assert!(validate_dispatch_episode_outcome(&value).is_err());
    }

    #[test]
    fn rejects_false_critical_service_pass() {
        let mut value = episode("outage", "fixed", "run-1", '6');
        value.metrics.delivered_critical_service_wh = 800.0;
        value.metrics.unserved_critical_service_wh = 200.0;
        value.metrics.maximum_unserved_critical_service_wh = 10.0;
        assert!(validate_dispatch_episode_outcome(&value).is_err());
    }

    #[test]
    fn rejects_false_terminal_reserve_pass() {
        let mut value = episode("outage", "fixed", "run-1", '6');
        value.metrics.terminal_reserve_margin_wh = -1.0;
        assert!(validate_dispatch_episode_outcome(&value).is_err());
    }

    #[test]
    fn rejects_negative_resource_energy() {
        let mut value = episode("baseline", "fixed", "run-1", '6');
        value.metrics.auxiliary_energy_wh = -0.1;
        assert!(validate_dispatch_episode_outcome(&value).is_err());
    }

    #[test]
    fn rejects_deadline_count_above_decision_count() {
        let mut value = episode("baseline", "fixed", "run-1", '6');
        value.metrics.missed_decision_deadline_count = 101;
        assert!(validate_dispatch_episode_outcome(&value).is_err());
    }

    #[test]
    fn rejects_partial_latency_quantiles() {
        let mut value = episode("baseline", "fixed", "run-1", '6');
        value.metrics.decision_latency_p99_ms = None;
        assert!(validate_dispatch_episode_outcome(&value).is_err());
    }

    #[test]
    fn rejects_non_monotonic_latency_quantiles() {
        let mut value = episode("baseline", "fixed", "run-1", '6');
        value.metrics.decision_latency_p95_ms = Some(5.0);
        assert!(validate_dispatch_episode_outcome(&value).is_err());
    }

    #[test]
    fn rejects_synthetic_evidence_in_simulation_environment() {
        let mut value = episode("baseline", "fixed", "run-1", '6');
        value.test_environment = DispatchTestEnvironment::Simulation;
        assert!(validate_dispatch_episode_outcome(&value).is_err());
    }

    #[test]
    fn independent_evidence_requires_assessment_reference() {
        let mut value = episode("baseline", "fixed", "run-1", '6');
        value.evidence_class = DispatchEvidenceClass::Accredited;
        value.test_environment = DispatchTestEnvironment::ControlledLaboratory;
        assert!(validate_dispatch_episode_outcome(&value).is_err());
    }

    #[test]
    fn valid_campaign_summary_reconciles() {
        let result = validate_dispatch_outcome_campaign(&campaign("fixed", "r")).unwrap();
        assert_eq!(result.episode_count, 2);
        assert_eq!(result.pass_count, 2);
        assert_eq!(result.fail_count, 0);
    }

    #[test]
    fn rejects_duplicate_scenario_ids_in_campaign() {
        let mut value = campaign("fixed", "r");
        value.episodes[1].scenario_id = value.episodes[0].scenario_id.clone();
        assert!(validate_dispatch_outcome_campaign(&value).is_err());
    }

    #[test]
    fn rejects_episode_policy_identity_mismatch() {
        let mut value = campaign("fixed", "r");
        value.episodes[0].policy_sha256 = digest('f');
        assert!(validate_dispatch_outcome_campaign(&value).is_err());
    }

    #[test]
    fn matched_campaigns_preserve_every_episode() {
        let reference = campaign("fixed", "r");
        let candidate = campaign("mpc", "c");
        let report = compare_dispatch_outcome_campaigns(&reference, &candidate).unwrap();
        assert!(report.structural_match);
        assert_eq!(report.paired_episodes.len(), 2);
        assert_eq!(report.reference_summary.episode_count, 2);
        assert_eq!(report.candidate_summary.episode_count, 2);
    }

    #[test]
    fn rejects_comparison_with_different_input_roots() {
        let reference = campaign("fixed", "r");
        let mut candidate = campaign("mpc", "c");
        candidate.episodes[0].matched_input_sha256 = digest('f');
        let report = compare_dispatch_outcome_campaigns(&reference, &candidate).unwrap();
        assert!(!report.structural_match);
        assert!(report.blockers.iter().any(|item| item.starts_with("mismatch:matched_input")));
    }

    #[test]
    fn rejects_comparison_with_missing_scenario() {
        let reference = campaign("fixed", "r");
        let mut candidate = campaign("mpc", "c");
        candidate.episodes.pop();
        let report = compare_dispatch_outcome_campaigns(&reference, &candidate).unwrap();
        assert!(!report.structural_match);
        assert!(report.blockers.iter().any(|item| item.starts_with("candidate_scenario_missing")));
    }

    #[test]
    fn comparison_contains_full_candidate_and_reference_snapshots() {
        let reference = campaign("fixed", "r");
        let candidate = campaign("mpc", "c");
        let report = compare_dispatch_outcome_campaigns(&reference, &candidate).unwrap();
        assert_eq!(report.reference_campaign, reference);
        assert_eq!(report.candidate_campaign, candidate);
        assert_eq!(report.paired_episodes[0].reference.run_id, "r-1");
        assert_eq!(report.paired_episodes[0].candidate.run_id, "c-1");
    }
}
