use crate::AuditError;
use std::collections::HashSet;

const CLAIM_CEILING: &str =
    "OFFLINE_CONTROLLER_BENCHMARK_PROTOCOL_ONLY_NOT_PHYSICAL_OR_CONFORMITY_CLAIM";
const MAX_CASES: usize = 100_000;
const MAX_DISTURBANCES_PER_CASE: usize = 10_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScenarioCorpusClass { SyntheticFixture }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScenarioCaseType { OperationalEpisode, PairingNegativeControl }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScenarioClass {
    Baseline, ForecastUncertainty, SourceContingency, StorageContingency,
    AuxiliaryCommonCause, TelemetryIntegrity, Recovery, PairingIntegrity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScenarioGate { ExecuteOfflineEpisode, RejectPairBeforeExecution }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScenarioBehavior {
    PreserveCriticalServiceAndTerminalReserve,
    DegradeWithoutFalsePass,
    EnterBoundedSafeFallback,
    RecoverAndRestoreTerminalObligation,
    RejectMismatchedArmPair,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MismatchDimension {
    MissionProfile, EnvironmentProfile, PlantModelProfile, InitialState,
    TerminalObligation, AuxiliaryBoundary, InformationBudget, ComputeBudget,
    MissionHorizon, DecisionCadence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RequiredInvariant {
    CriticalServiceNoncompensatory, AllComponentLimitsRespected, InitialStateMatched,
    TerminalReserveObligationMet, AuxiliaryLoadAccountedOnce,
    StaleOrUnknownTelemetryNotFresh, BoundedSafeFallback, NoFutureInformationLeakage,
    RecoveryAndReserveRestorationReported, FailuresRemainInDenominator,
    ReplayInputsAndOutputsDigestBound, CommonCauseNotAssumedIndependent,
    PairingMismatchRejectedBeforeScoring,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PairedValue<T> { pub reference: T, pub candidate: T }

#[derive(Debug, Clone, PartialEq)]
pub struct ScenarioMatchedConditions {
    pub mission_profile: PairedValue<String>,
    pub environment_profile: PairedValue<String>,
    pub plant_model_profile: PairedValue<String>,
    pub initial_state: PairedValue<String>,
    pub terminal_obligation: PairedValue<String>,
    pub auxiliary_boundary: PairedValue<String>,
    pub information_budget: PairedValue<String>,
    pub compute_budget: PairedValue<String>,
    pub mission_horizon_s: PairedValue<f64>,
    pub decision_cadence_s: PairedValue<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScenarioDisturbance {
    pub event_id: String,
    pub domain: String,
    pub at_s: f64,
    pub duration_s: f64,
    pub severity: String,
    pub effect: String,
    pub parameters_sha256: String,
    pub common_cause_group: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DispatchControllerScenario {
    pub case_id: String,
    pub case_type: ScenarioCaseType,
    pub scenario_class: ScenarioClass,
    pub seed: u32,
    pub mission_horizon_s: f64,
    pub decision_cadence_s: f64,
    pub reference_policy_sha256: String,
    pub candidate_policy_sha256: String,
    pub matched_conditions: ScenarioMatchedConditions,
    pub disturbances: Vec<ScenarioDisturbance>,
    pub required_invariants: Vec<RequiredInvariant>,
    pub expected_gate: ScenarioGate,
    pub mismatch_dimension: Option<MismatchDimension>,
    pub expected_behavior: ScenarioBehavior,
    pub rationale: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DispatchControllerScenarioCorpus {
    pub schema_version: String,
    pub corpus_id: String,
    pub corpus_class: ScenarioCorpusClass,
    pub cases: Vec<DispatchControllerScenario>,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispatchScenarioValidationReport {
    pub case_count: usize,
    pub operational_episode_count: usize,
    pub pairing_negative_control_count: usize,
    pub disturbance_count: usize,
    pub claim_ceiling: &'static str,
}

fn nonblank(value: &str) -> bool { !value.trim().is_empty() }

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn valid_domain(value: &str) -> bool {
    matches!(value, "forecast" | "load" | "grid" | "generator" | "fuel" | "storage"
        | "auxiliary" | "cooling" | "telemetry" | "timebase" | "communications"
        | "repair" | "power_event" | "price")
}

fn valid_severity(value: &str) -> bool {
    matches!(value, "nominal" | "moderate" | "severe" | "hard_fault")
}

fn valid_effect(value: &str) -> bool {
    matches!(value, "forecast_bias" | "forecast_delay" | "load_ramp" | "availability_loss"
        | "availability_restore" | "start_delay" | "start_failure" | "fuel_limit"
        | "power_derate" | "energy_derate" | "capacity_derate" | "auxiliary_increase"
        | "cooling_degradation" | "stale_data" | "missing_data" | "measurement_bias"
        | "clock_offset" | "partition" | "repair_outage" | "repeated_event"
        | "price_spike" | "recharge_recovery")
}

fn mismatches(c: &ScenarioMatchedConditions) -> Vec<MismatchDimension> {
    let mut out = Vec::new();
    if c.mission_profile.reference != c.mission_profile.candidate { out.push(MismatchDimension::MissionProfile); }
    if c.environment_profile.reference != c.environment_profile.candidate { out.push(MismatchDimension::EnvironmentProfile); }
    if c.plant_model_profile.reference != c.plant_model_profile.candidate { out.push(MismatchDimension::PlantModelProfile); }
    if c.initial_state.reference != c.initial_state.candidate { out.push(MismatchDimension::InitialState); }
    if c.terminal_obligation.reference != c.terminal_obligation.candidate { out.push(MismatchDimension::TerminalObligation); }
    if c.auxiliary_boundary.reference != c.auxiliary_boundary.candidate { out.push(MismatchDimension::AuxiliaryBoundary); }
    if c.information_budget.reference != c.information_budget.candidate { out.push(MismatchDimension::InformationBudget); }
    if c.compute_budget.reference != c.compute_budget.candidate { out.push(MismatchDimension::ComputeBudget); }
    if c.mission_horizon_s.reference != c.mission_horizon_s.candidate { out.push(MismatchDimension::MissionHorizon); }
    if c.decision_cadence_s.reference != c.decision_cadence_s.candidate { out.push(MismatchDimension::DecisionCadence); }
    out
}

fn validate_scenario(s: &DispatchControllerScenario) -> Result<usize, AuditError> {
    if !nonblank(&s.case_id) || !nonblank(&s.rationale) {
        return Err(AuditError("scenario case identity and rationale must be non-empty".to_string()));
    }
    if !s.mission_horizon_s.is_finite() || s.mission_horizon_s <= 0.0
        || !s.decision_cadence_s.is_finite() || s.decision_cadence_s <= 0.0 {
        return Err(AuditError("scenario mission horizon and decision cadence must be finite and positive".to_string()));
    }
    if !valid_sha256(&s.reference_policy_sha256) || !valid_sha256(&s.candidate_policy_sha256)
        || s.reference_policy_sha256 == s.candidate_policy_sha256 {
        return Err(AuditError("policy revisions must be distinct valid SHA-256 digests".to_string()));
    }

    let c = &s.matched_conditions;
    for digest in [
        c.mission_profile.reference.as_str(), c.mission_profile.candidate.as_str(),
        c.environment_profile.reference.as_str(), c.environment_profile.candidate.as_str(),
        c.plant_model_profile.reference.as_str(), c.plant_model_profile.candidate.as_str(),
        c.initial_state.reference.as_str(), c.initial_state.candidate.as_str(),
        c.terminal_obligation.reference.as_str(), c.terminal_obligation.candidate.as_str(),
        c.auxiliary_boundary.reference.as_str(), c.auxiliary_boundary.candidate.as_str(),
        c.information_budget.reference.as_str(), c.information_budget.candidate.as_str(),
        c.compute_budget.reference.as_str(), c.compute_budget.candidate.as_str(),
    ] {
        if !valid_sha256(digest) {
            return Err(AuditError("matched condition contains an invalid SHA-256 digest".to_string()));
        }
    }
    for horizon in [c.mission_horizon_s.reference, c.mission_horizon_s.candidate] {
        if !horizon.is_finite() || horizon <= 0.0 {
            return Err(AuditError("matched mission horizons must be finite and positive".to_string()));
        }
    }
    for cadence in [c.decision_cadence_s.reference, c.decision_cadence_s.candidate] {
        if !cadence.is_finite() || cadence <= 0.0 {
            return Err(AuditError("matched decision cadences must be finite and positive".to_string()));
        }
    }
    if c.mission_horizon_s.reference != s.mission_horizon_s
        || c.decision_cadence_s.reference != s.decision_cadence_s {
        return Err(AuditError("scenario horizon/cadence must bind reference policy conditions".to_string()));
    }

    if s.required_invariants.is_empty() {
        return Err(AuditError("scenario must declare at least one required invariant".to_string()));
    }
    let mut invariants = HashSet::new();
    for item in &s.required_invariants {
        if !invariants.insert(*item) {
            return Err(AuditError("scenario contains a duplicate required invariant".to_string()));
        }
    }
    if s.disturbances.len() > MAX_DISTURBANCES_PER_CASE {
        return Err(AuditError("scenario exceeds the supported disturbance count".to_string()));
    }
    let mut event_ids = HashSet::new();
    for e in &s.disturbances {
        if !nonblank(&e.event_id) || !event_ids.insert(e.event_id.as_str())
            || !valid_domain(&e.domain) || !valid_severity(&e.severity) || !valid_effect(&e.effect)
            || !valid_sha256(&e.parameters_sha256) || !e.at_s.is_finite()
            || !e.duration_s.is_finite() || e.at_s < 0.0 || e.duration_s <= 0.0
            || e.common_cause_group.as_deref().is_some_and(|v| !nonblank(v)) {
            return Err(AuditError("disturbance identity, classification, digest or timing is invalid".to_string()));
        }
        let end_s = e.at_s + e.duration_s;
        if !end_s.is_finite() || end_s > s.mission_horizon_s {
            return Err(AuditError("disturbance must end within the scenario mission horizon".to_string()));
        }
    }

    let mismatched = mismatches(c);
    match s.case_type {
        ScenarioCaseType::OperationalEpisode => {
            if !mismatched.is_empty() || s.mismatch_dimension.is_some()
                || s.expected_gate != ScenarioGate::ExecuteOfflineEpisode
                || s.scenario_class == ScenarioClass::PairingIntegrity
                || s.expected_behavior == ScenarioBehavior::RejectMismatchedArmPair {
                return Err(AuditError("operational episode must be fully matched and executable offline".to_string()));
            }
        }
        ScenarioCaseType::PairingNegativeControl => {
            if mismatched.len() != 1 || s.mismatch_dimension != mismatched.first().copied()
                || s.expected_gate != ScenarioGate::RejectPairBeforeExecution
                || s.scenario_class != ScenarioClass::PairingIntegrity
                || s.expected_behavior != ScenarioBehavior::RejectMismatchedArmPair
                || !s.required_invariants.contains(&RequiredInvariant::PairingMismatchRejectedBeforeScoring) {
                return Err(AuditError("pairing negative control must declare exactly one matching mismatch and reject before execution".to_string()));
            }
        }
    }
    Ok(s.disturbances.len())
}

/// Validates the typed, in-memory v0.1 corpus. It does not parse JSON, run a
/// dispatch policy, simulate physical plant behavior, verify evidence or prove
/// controller performance.
pub fn validate_dispatch_controller_scenario_corpus(
    corpus: &DispatchControllerScenarioCorpus,
) -> Result<DispatchScenarioValidationReport, AuditError> {
    if corpus.schema_version != "0.1" || !corpus.corpus_id.starts_with("PF-DISPATCH-")
        || !nonblank(&corpus.corpus_id) || corpus.corpus_class != ScenarioCorpusClass::SyntheticFixture
        || corpus.claim_ceiling != CLAIM_CEILING {
        return Err(AuditError("scenario corpus metadata or claim ceiling is invalid".to_string()));
    }
    if corpus.cases.is_empty() || corpus.cases.len() > MAX_CASES {
        return Err(AuditError("scenario corpus is empty or exceeds the supported case count".to_string()));
    }
    let mut ids = HashSet::new();
    let mut operational = 0usize;
    let mut negative = 0usize;
    let mut disturbances = 0usize;
    for scenario in &corpus.cases {
        if !ids.insert(scenario.case_id.as_str()) {
            return Err(AuditError("scenario corpus contains a duplicate case id".to_string()));
        }
        disturbances = disturbances.checked_add(validate_scenario(scenario)?)
            .ok_or_else(|| AuditError("scenario disturbance count overflowed".to_string()))?;
        match scenario.case_type {
            ScenarioCaseType::OperationalEpisode => operational += 1,
            ScenarioCaseType::PairingNegativeControl => negative += 1,
        }
    }
    if operational == 0 || negative == 0 {
        return Err(AuditError("corpus requires both operational positive and pairing negative controls".to_string()));
    }
    Ok(DispatchScenarioValidationReport {
        case_count: corpus.cases.len(),
        operational_episode_count: operational,
        pairing_negative_control_count: negative,
        disturbance_count: disturbances,
        claim_ceiling: CLAIM_CEILING,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        validate_dispatch_controller_scenario_corpus, DispatchControllerScenario,
        DispatchControllerScenarioCorpus, MismatchDimension, PairedValue, RequiredInvariant,
        ScenarioBehavior, ScenarioCaseType, ScenarioClass, ScenarioCorpusClass,
        ScenarioDisturbance, ScenarioGate, ScenarioMatchedConditions,
    };

    const CEILING: &str =
        "OFFLINE_CONTROLLER_BENCHMARK_PROTOCOL_ONLY_NOT_PHYSICAL_OR_CONFORMITY_CLAIM";

    fn digest(ch: char) -> String { std::iter::repeat(ch).take(64).collect() }

    fn matched() -> ScenarioMatchedConditions {
        ScenarioMatchedConditions {
            mission_profile: PairedValue { reference: digest('1'), candidate: digest('1') },
            environment_profile: PairedValue { reference: digest('2'), candidate: digest('2') },
            plant_model_profile: PairedValue { reference: digest('3'), candidate: digest('3') },
            initial_state: PairedValue { reference: digest('4'), candidate: digest('4') },
            terminal_obligation: PairedValue { reference: digest('5'), candidate: digest('5') },
            auxiliary_boundary: PairedValue { reference: digest('6'), candidate: digest('6') },
            information_budget: PairedValue { reference: digest('7'), candidate: digest('7') },
            compute_budget: PairedValue { reference: digest('8'), candidate: digest('8') },
            mission_horizon_s: PairedValue { reference: 100.0, candidate: 100.0 },
            decision_cadence_s: PairedValue { reference: 5.0, candidate: 5.0 },
        }
    }

    fn positive() -> DispatchControllerScenario {
        DispatchControllerScenario {
            case_id: "positive".to_string(), case_type: ScenarioCaseType::OperationalEpisode,
            scenario_class: ScenarioClass::Baseline, seed: 7, mission_horizon_s: 100.0,
            decision_cadence_s: 5.0, reference_policy_sha256: digest('a'),
            candidate_policy_sha256: digest('b'), matched_conditions: matched(),
            disturbances: vec![ScenarioDisturbance {
                event_id: "disturbance-1".to_string(), domain: "load".to_string(),
                at_s: 10.0, duration_s: 5.0, severity: "moderate".to_string(),
                effect: "load_ramp".to_string(), parameters_sha256: digest('c'),
                common_cause_group: None,
            }],
            required_invariants: vec![
                RequiredInvariant::CriticalServiceNoncompensatory,
                RequiredInvariant::TerminalReserveObligationMet,
            ],
            expected_gate: ScenarioGate::ExecuteOfflineEpisode, mismatch_dimension: None,
            expected_behavior: ScenarioBehavior::PreserveCriticalServiceAndTerminalReserve,
            rationale: "positive control".to_string(),
        }
    }

    fn negative() -> DispatchControllerScenario {
        let mut s = positive();
        s.case_id = "negative".to_string();
        s.case_type = ScenarioCaseType::PairingNegativeControl;
        s.scenario_class = ScenarioClass::PairingIntegrity;
        s.matched_conditions.initial_state.candidate = digest('9');
        s.required_invariants = vec![RequiredInvariant::PairingMismatchRejectedBeforeScoring];
        s.expected_gate = ScenarioGate::RejectPairBeforeExecution;
        s.mismatch_dimension = Some(MismatchDimension::InitialState);
        s.expected_behavior = ScenarioBehavior::RejectMismatchedArmPair;
        s
    }

    fn corpus() -> DispatchControllerScenarioCorpus {
        DispatchControllerScenarioCorpus {
            schema_version: "0.1".to_string(), corpus_id: "PF-DISPATCH-TEST".to_string(),
            corpus_class: ScenarioCorpusClass::SyntheticFixture,
            cases: vec![positive(), negative()], claim_ceiling: CEILING.to_string(),
        }
    }

    #[test]
    fn accepts_positive_and_single_mismatch_negative_controls() {
        let report = validate_dispatch_controller_scenario_corpus(&corpus()).unwrap();
        assert_eq!(report.case_count, 2);
        assert_eq!(report.operational_episode_count, 1);
        assert_eq!(report.pairing_negative_control_count, 1);
        assert_eq!(report.disturbance_count, 1);
        assert_eq!(report.claim_ceiling, CEILING);
    }

    #[test]
    fn rejects_duplicate_case_ids() {
        let mut c = corpus();
        let duplicate_id = c.cases[0].case_id.clone();
        c.cases[1].case_id = duplicate_id;
        assert!(validate_dispatch_controller_scenario_corpus(&c).is_err());
    }

    #[test]
    fn rejects_duplicate_event_ids() {
        let mut c = corpus();
        let duplicate_event = c.cases[0].disturbances[0].clone();
        c.cases[0].disturbances.push(duplicate_event);
        assert!(validate_dispatch_controller_scenario_corpus(&c).is_err());
    }

    #[test]
    fn rejects_event_outside_horizon() {
        let mut c = corpus(); c.cases[0].disturbances[0].at_s = 99.0;
        c.cases[0].disturbances[0].duration_s = 2.0;
        assert!(validate_dispatch_controller_scenario_corpus(&c).is_err());
    }

    #[test]
    fn rejects_non_finite_event_time() {
        let mut c = corpus(); c.cases[0].disturbances[0].at_s = f64::NAN;
        assert!(validate_dispatch_controller_scenario_corpus(&c).is_err());
    }

    #[test]
    fn rejects_invalid_policy_digest() {
        let mut c = corpus(); c.cases[0].candidate_policy_sha256 = "bad".to_string();
        assert!(validate_dispatch_controller_scenario_corpus(&c).is_err());
    }

    #[test]
    fn rejects_identical_policy_revisions() {
        let mut c = corpus();
        let same_policy = c.cases[0].reference_policy_sha256.clone();
        c.cases[0].candidate_policy_sha256 = same_policy;
        assert!(validate_dispatch_controller_scenario_corpus(&c).is_err());
    }

    #[test]
    fn rejects_mismatched_operational_episode() {
        let mut c = corpus(); c.cases[0].matched_conditions.auxiliary_boundary.candidate = digest('f');
        assert!(validate_dispatch_controller_scenario_corpus(&c).is_err());
    }

    #[test]
    fn rejects_negative_control_without_declared_dimension() {
        let mut c = corpus(); c.cases[1].mismatch_dimension = None;
        assert!(validate_dispatch_controller_scenario_corpus(&c).is_err());
    }

    #[test]
    fn rejects_negative_control_with_two_mismatches() {
        let mut c = corpus(); c.cases[1].matched_conditions.terminal_obligation.candidate = digest('e');
        assert!(validate_dispatch_controller_scenario_corpus(&c).is_err());
    }

    #[test]
    fn rejects_wrong_negative_control_dimension() {
        let mut c = corpus(); c.cases[1].mismatch_dimension = Some(MismatchDimension::AuxiliaryBoundary);
        assert!(validate_dispatch_controller_scenario_corpus(&c).is_err());
    }

    #[test]
    fn rejects_negative_control_that_would_execute() {
        let mut c = corpus(); c.cases[1].expected_gate = ScenarioGate::ExecuteOfflineEpisode;
        assert!(validate_dispatch_controller_scenario_corpus(&c).is_err());
    }

    #[test]
    fn rejects_duplicate_required_invariants() {
        let mut c = corpus(); c.cases[0].required_invariants.push(RequiredInvariant::CriticalServiceNoncompensatory);
        assert!(validate_dispatch_controller_scenario_corpus(&c).is_err());
    }

    #[test]
    fn rejects_empty_required_invariants() {
        let mut c = corpus(); c.cases[0].required_invariants.clear();
        assert!(validate_dispatch_controller_scenario_corpus(&c).is_err());
    }

    #[test]
    fn rejects_unknown_disturbance_domain() {
        let mut c = corpus(); c.cases[0].disturbances[0].domain = "unknown".to_string();
        assert!(validate_dispatch_controller_scenario_corpus(&c).is_err());
    }

    #[test]
    fn rejects_unknown_disturbance_effect() {
        let mut c = corpus(); c.cases[0].disturbances[0].effect = "unknown".to_string();
        assert!(validate_dispatch_controller_scenario_corpus(&c).is_err());
    }

    #[test]
    fn rejects_reference_horizon_mismatch() {
        let mut c = corpus(); c.cases[0].matched_conditions.mission_horizon_s.reference = 200.0;
        assert!(validate_dispatch_controller_scenario_corpus(&c).is_err());
    }

    #[test]
    fn rejects_invalid_corpus_claim_ceiling() {
        let mut c = corpus(); c.claim_ceiling = "PHYSICAL_PASS".to_string();
        assert!(validate_dispatch_controller_scenario_corpus(&c).is_err());
    }

    #[test]
    fn rejects_corpus_with_only_one_control_class() {
        let mut c = corpus(); c.cases.pop();
        assert!(validate_dispatch_controller_scenario_corpus(&c).is_err());
    }
}
