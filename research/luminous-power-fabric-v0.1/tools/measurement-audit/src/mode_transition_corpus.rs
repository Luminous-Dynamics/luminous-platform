use crate::AuditError;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModeState {
    pub state_id: String,
    pub topology_sha256: String,
    pub configuration_sha256: String,
    pub pcc_breaker_state: String,
    pub grid_state: String,
    pub synchronization_state: String,
    pub grid_forming_state: String,
    pub protection_state: String,
    pub communications_state: String,
    pub critical_service_state: String,
    pub reserve_state: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModeGuard {
    pub guard_id: String,
    pub predicate: String,
    pub enforcement_owner: String,
    pub freshness_required: bool,
    pub failure_behavior: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModeTransition {
    pub transition_id: String,
    pub from_state_ids: Vec<String>,
    pub to_state_id: String,
    pub permitted: bool,
    pub actor: String,
    pub authorization_owner: String,
    pub protection_interlock_required: bool,
    pub guard_ids: Vec<String>,
    pub postcondition_ids: Vec<String>,
    pub failure_disposition: String,
    pub recovery_transition_id: Option<String>,
    pub forbidden_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionTestCase {
    pub test_case_id: String,
    pub transition_id: String,
    pub test_kind: String,
    pub expected_decision: String,
    pub satisfied_guard_ids: Vec<String>,
    pub violated_guard_ids: Vec<String>,
    pub observed_postcondition_ids: Vec<String>,
    pub recovery_transition_id: Option<String>,
    pub evidence_class: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModeTransitionCorpus {
    pub states: Vec<ModeState>,
    pub guards: Vec<ModeGuard>,
    pub transitions: Vec<ModeTransition>,
    pub test_cases: Vec<TransitionTestCase>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionCoverageReport {
    pub state_count: usize,
    pub guard_count: usize,
    pub transition_count: usize,
    pub permitted_transition_count: usize,
    pub forbidden_transition_count: usize,
    pub positive_test_count: usize,
    pub negative_test_count: usize,
    pub recovery_test_count: usize,
    pub claim_ceiling: &'static str,
}

fn nonblank(value: &str, field: &str) -> Result<(), AuditError> {
    if value.trim().is_empty() {
        return Err(AuditError(format!("{field} must not be empty")));
    }
    Ok(())
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn unique_ids<'a>(values: impl IntoIterator<Item = &'a str>, field: &str) -> Result<(), AuditError> {
    let mut seen = BTreeSet::new();
    for value in values {
        nonblank(value, field)?;
        if !seen.insert(value) {
            return Err(AuditError(format!("duplicate {field} {value:?}")));
        }
    }
    Ok(())
}

fn contains_all(have: &[String], required: &[String]) -> bool {
    required.iter().all(|id| have.iter().any(|candidate| candidate == id))
}

fn allowed_authority(value: &str) -> bool {
    matches!(value, "supervisory_mems" | "local_controller" | "independent_protection" | "operator" | "maintenance_authority")
}

fn validate_state(state: &ModeState) -> Result<(), AuditError> {
    nonblank(&state.state_id, "state_id")?;
    if !valid_digest(&state.topology_sha256) || !valid_digest(&state.configuration_sha256) {
        return Err(AuditError(format!("state {:?} has an invalid topology/configuration digest", state.state_id)));
    }
    let checks = [
        ("pcc_breaker_state", state.pcc_breaker_state.as_str(), &["closed", "open", "open_or_unknown", "unknown"][..]),
        ("grid_state", state.grid_state.as_str(), &["present", "absent_or_isolated", "unknown"][..]),
        ("synchronization_state", state.synchronization_state.as_str(), &["synchronised", "not_applicable", "sync_pending", "unknown"][..]),
        ("grid_forming_state", state.grid_forming_state.as_str(), &["available", "unavailable", "starting", "unknown"][..]),
        ("protection_state", state.protection_state.as_str(), &["ready", "tripped", "ready_or_unknown", "unknown"][..]),
        ("communications_state", state.communications_state.as_str(), &["fresh", "stale_or_unknown", "fresh_or_unknown", "unknown"][..]),
        ("critical_service_state", state.critical_service_state.as_str(), &["service_on", "service_off", "staged_pickup", "service_unknown"][..]),
        ("reserve_state", state.reserve_state.as_str(), &["reserve_met", "reserve_unknown"][..]),
    ];
    for (field, value, allowed) in checks {
        if !allowed.contains(&value) {
            return Err(AuditError(format!("state {:?} has unsupported {field} value {value:?}", state.state_id)));
        }
    }
    if state.state_id == "grid_connected"
        && !(state.pcc_breaker_state == "closed" && state.grid_state == "present"
            && state.synchronization_state == "synchronised" && state.protection_state == "ready") {
        return Err(AuditError("grid_connected invariant violated".to_string()));
    }
    if state.state_id == "islanded"
        && !(state.pcc_breaker_state == "open" && state.grid_state == "absent_or_isolated"
            && state.grid_forming_state == "available" && state.protection_state == "ready") {
        return Err(AuditError("islanded invariant violated".to_string()));
    }
    if state.state_id == "dead_bus"
        && !(state.pcc_breaker_state == "open" && state.grid_state == "absent_or_isolated"
            && state.critical_service_state == "service_off") {
        return Err(AuditError("dead_bus invariant violated".to_string()));
    }
    if state.state_id == "protection_tripped" && state.protection_state != "tripped" {
        return Err(AuditError("protection_tripped state must retain tripped protection status".to_string()));
    }
    if state.state_id == "unknown_hold"
        && !(state.pcc_breaker_state == "unknown" && state.grid_state == "unknown") {
        return Err(AuditError("unknown_hold must not assert an unobserved physical PCC/grid state".to_string()));
    }
    Ok(())
}

pub fn validate_mode_transition_corpus(corpus: &ModeTransitionCorpus) -> Result<TransitionCoverageReport, AuditError> {
    if corpus.states.is_empty() || corpus.transitions.is_empty() || corpus.test_cases.is_empty() {
        return Err(AuditError("states, transitions and test cases must be non-empty".to_string()));
    }
    unique_ids(corpus.states.iter().map(|x| x.state_id.as_str()), "state ID")?;
    unique_ids(corpus.guards.iter().map(|x| x.guard_id.as_str()), "guard ID")?;
    unique_ids(corpus.transitions.iter().map(|x| x.transition_id.as_str()), "transition ID")?;
    unique_ids(corpus.test_cases.iter().map(|x| x.test_case_id.as_str()), "test-case ID")?;

    let state_ids: BTreeSet<&str> = corpus.states.iter().map(|x| x.state_id.as_str()).collect();
    let guard_map: BTreeMap<&str, &ModeGuard> = corpus.guards.iter().map(|x| (x.guard_id.as_str(), x)).collect();
    let transition_map: BTreeMap<&str, &ModeTransition> = corpus.transitions.iter().map(|x| (x.transition_id.as_str(), x)).collect();

    for state in &corpus.states { validate_state(state)?; }
    for guard in &corpus.guards {
        nonblank(&guard.predicate, "guard predicate")?;
        if !allowed_authority(&guard.enforcement_owner) {
            return Err(AuditError(format!("guard {:?} has an unsupported enforcement owner", guard.guard_id)));
        }
        if !matches!(guard.failure_behavior.as_str(), "block_transition" | "protection_action" | "safe_hold" | "block_command" | "enter_unknown_hold") {
            return Err(AuditError(format!("guard {:?} has an unsupported failure behavior", guard.guard_id)));
        }
    }

    for transition in &corpus.transitions {
        nonblank(&transition.to_state_id, "transition target state")?;
        nonblank(&transition.actor, "transition actor")?;
        nonblank(&transition.authorization_owner, "transition authorization owner")?;
        nonblank(&transition.failure_disposition, "transition failure disposition")?;
        if !allowed_authority(&transition.actor) || !allowed_authority(&transition.authorization_owner) {
            return Err(AuditError(format!("transition {:?} has an unsupported authority", transition.transition_id)));
        }
        if transition.from_state_ids.is_empty() || transition.guard_ids.is_empty() || transition.postcondition_ids.is_empty() {
            return Err(AuditError(format!("transition {:?} must declare source state, guards and postconditions", transition.transition_id)));
        }
        unique_ids(transition.from_state_ids.iter().map(String::as_str), "transition source state")?;
        unique_ids(transition.guard_ids.iter().map(String::as_str), "transition guard")?;
        unique_ids(transition.postcondition_ids.iter().map(String::as_str), "postcondition")?;
        for source in &transition.from_state_ids {
            if !state_ids.contains(source.as_str()) {
                return Err(AuditError(format!("transition {:?} references unknown source state {:?}", transition.transition_id, source)));
            }
        }
        if !state_ids.contains(transition.to_state_id.as_str()) {
            return Err(AuditError(format!("transition {:?} references unknown target state", transition.transition_id)));
        }
        for guard_id in &transition.guard_ids {
            if !guard_map.contains_key(guard_id.as_str()) {
                return Err(AuditError(format!("transition {:?} references unknown guard {:?}", transition.transition_id, guard_id)));
            }
        }
        if transition.protection_interlock_required {
            if matches!(transition.authorization_owner.as_str(), "supervisory_mems" | "operator") {
                return Err(AuditError(format!("transition {:?} assigns protection-interlocked authorization to a supervisory/request-only owner", transition.transition_id)));
            }
            let has_independent_guard = transition.guard_ids.iter().any(|id| {
                guard_map.get(id.as_str()).is_some_and(|guard| guard.enforcement_owner == "independent_protection")
            });
            if !has_independent_guard {
                return Err(AuditError(format!("transition {:?} lacks an independent-protection guard", transition.transition_id)));
            }
        }
        if transition.permitted && transition.forbidden_reason.is_some() {
            return Err(AuditError(format!("permitted transition {:?} cannot have a forbidden reason", transition.transition_id)));
        }
        if !transition.permitted {
            let reason = transition.forbidden_reason.as_deref().unwrap_or("");
            nonblank(reason, "forbidden transition reason")?;
            if transition.recovery_transition_id.is_some() {
                return Err(AuditError(format!("forbidden transition {:?} cannot declare an automatic recovery route", transition.transition_id)));
            }
        }
    }

    for transition in &corpus.transitions {
        if let Some(recovery_id) = &transition.recovery_transition_id {
            let recovery = transition_map.get(recovery_id.as_str())
                .ok_or_else(|| AuditError(format!("transition {:?} references missing recovery route {:?}", transition.transition_id, recovery_id)))?;
            if !recovery.permitted || !recovery.from_state_ids.iter().any(|id| id == &transition.to_state_id) {
                return Err(AuditError(format!("recovery route {:?} is not permitted from the initiating transition's target state", recovery_id)));
            }
        }
    }

    let mut positive_by_transition = BTreeMap::<String, usize>::new();
    let mut negative_by_transition = BTreeMap::<String, usize>::new();
    let mut negative_guard_coverage = BTreeSet::<(String, String)>::new();
    let mut recovery_by_transition = BTreeMap::<String, usize>::new();
    let mut positive_count = 0;
    let mut negative_count = 0;
    let mut recovery_count = 0;

    for test in &corpus.test_cases {
        let transition = transition_map.get(test.transition_id.as_str())
            .ok_or_else(|| AuditError(format!("test {:?} references unknown transition", test.test_case_id)))?;
        if test.evidence_class != "synthetic" {
            return Err(AuditError(format!("test {:?} exceeds the synthetic corpus evidence class", test.test_case_id)));
        }
        unique_ids(test.satisfied_guard_ids.iter().map(String::as_str), "satisfied guard")?;
        unique_ids(test.violated_guard_ids.iter().map(String::as_str), "violated guard")?;
        unique_ids(test.observed_postcondition_ids.iter().map(String::as_str), "observed postcondition")?;

        let applicable_guard_ids: &[String] = if test.test_kind == "recovery" {
            let recovery_id = transition.recovery_transition_id.as_ref()
                .ok_or_else(|| AuditError(format!("test {:?} is recovery coverage without a recovery route", test.test_case_id)))?;
            &transition_map.get(recovery_id.as_str())
                .ok_or_else(|| AuditError(format!("test {:?} references missing recovery transition", test.test_case_id)))?.guard_ids
        } else {
            &transition.guard_ids
        };
        for id in test.satisfied_guard_ids.iter().chain(test.violated_guard_ids.iter()) {
            if !applicable_guard_ids.contains(id) {
                return Err(AuditError(format!("test {:?} references guard {:?} not declared by the applicable transition", test.test_case_id, id)));
            }
        }
        match test.test_kind.as_str() {
            "positive" => {
                if test.expected_decision != "allow" || !transition.permitted || !test.violated_guard_ids.is_empty()
                    || !contains_all(&test.satisfied_guard_ids, &transition.guard_ids)
                    || !contains_all(&test.observed_postcondition_ids, &transition.postcondition_ids)
                    || test.recovery_transition_id.is_some() {
                    return Err(AuditError(format!("positive test {:?} does not fully cover its permitted transition", test.test_case_id)));
                }
                *positive_by_transition.entry(test.transition_id.clone()).or_default() += 1;
                positive_count += 1;
            }
            "negative" => {
                if test.expected_decision != "block" || test.violated_guard_ids.len() != 1
                    || test.recovery_transition_id.is_some() {
                    return Err(AuditError(format!("negative test {:?} must mutate exactly one declared guard", test.test_case_id)));
                }
                let violated = &test.violated_guard_ids[0];
                let expected_satisfied: Vec<String> = applicable_guard_ids.iter()
                    .filter(|id| *id != violated)
                    .cloned()
                    .collect();
                if !contains_all(&test.satisfied_guard_ids, &expected_satisfied)
                    || test.satisfied_guard_ids.iter().any(|id| id == violated) {
                    return Err(AuditError(format!("negative test {:?} must satisfy every other guard", test.test_case_id)));
                }
                negative_guard_coverage.insert((test.transition_id.clone(), violated.clone()));
                *negative_by_transition.entry(test.transition_id.clone()).or_default() += 1;
                negative_count += 1;
            }
            "recovery" => {
                let expected_recovery = transition.recovery_transition_id.as_ref()
                    .ok_or_else(|| AuditError(format!("test {:?} is recovery coverage for a transition without a recovery route", test.test_case_id)))?;
                if test.expected_decision != "allow" || !test.violated_guard_ids.is_empty()
                    || test.recovery_transition_id.as_ref() != Some(expected_recovery) {
                    return Err(AuditError(format!("recovery test {:?} does not bind the declared recovery route", test.test_case_id)));
                }
                let recovery = transition_map.get(expected_recovery.as_str())
                    .ok_or_else(|| AuditError("validated recovery lookup failed".to_string()))?;
                if !contains_all(&test.satisfied_guard_ids, &recovery.guard_ids)
                    || !contains_all(&test.observed_postcondition_ids, &recovery.postcondition_ids) {
                    return Err(AuditError(format!("recovery test {:?} omits recovery guards or postconditions", test.test_case_id)));
                }
                *recovery_by_transition.entry(test.transition_id.clone()).or_default() += 1;
                recovery_count += 1;
            }
            other => return Err(AuditError(format!("test {:?} has unsupported kind {other:?}", test.test_case_id))),
        }
    }

    for transition in &corpus.transitions {
        let positive = *positive_by_transition.get(&transition.transition_id).unwrap_or(&0);
        let negative = *negative_by_transition.get(&transition.transition_id).unwrap_or(&0);
        let recovery = *recovery_by_transition.get(&transition.transition_id).unwrap_or(&0);
        if negative == 0 {
            return Err(AuditError(format!("transition {:?} lacks negative guard coverage", transition.transition_id)));
        }
        for guard_id in &transition.guard_ids {
            if !negative_guard_coverage.contains(&(transition.transition_id.clone(), guard_id.clone())) {
                return Err(AuditError(format!("transition {:?} lacks an isolated negative mutation for guard {:?}", transition.transition_id, guard_id)));
            }
        }
        if transition.permitted && positive == 0 {
            return Err(AuditError(format!("permitted transition {:?} lacks positive coverage", transition.transition_id)));
        }
        if !transition.permitted && positive != 0 {
            return Err(AuditError(format!("forbidden transition {:?} has positive coverage", transition.transition_id)));
        }
        if transition.recovery_transition_id.is_some() && recovery == 0 {
            return Err(AuditError(format!("transition {:?} lacks its required recovery test", transition.transition_id)));
        }
        if transition.recovery_transition_id.is_none() && recovery != 0 {
            return Err(AuditError(format!("transition {:?} has undeclared recovery coverage", transition.transition_id)));
        }
    }

    Ok(TransitionCoverageReport {
        state_count: corpus.states.len(),
        guard_count: corpus.guards.len(),
        transition_count: corpus.transitions.len(),
        permitted_transition_count: corpus.transitions.iter().filter(|x| x.permitted).count(),
        forbidden_transition_count: corpus.transitions.iter().filter(|x| !x.permitted).count(),
        positive_test_count: positive_count,
        negative_test_count: negative_count,
        recovery_test_count: recovery_count,
        claim_ceiling: "SYNTHETIC_TRANSITION_COVERAGE_ONLY_NOT_HIL_SAFETY_OR_CONFORMITY",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(c: char) -> String { std::iter::repeat(c).take(64).collect() }
    fn state(id: &str) -> ModeState {
        let connected = id == "grid_connected";
        ModeState {
            state_id: id.into(), topology_sha256: digest('a'), configuration_sha256: digest('b'),
            pcc_breaker_state: if connected { "closed" } else { "unknown" }.into(),
            grid_state: if connected { "present" } else { "unknown" }.into(),
            synchronization_state: if connected { "synchronised" } else { "unknown" }.into(),
            grid_forming_state: "unknown".into(), protection_state: "ready_or_unknown".into(),
            communications_state: "stale_or_unknown".into(), critical_service_state: "service_unknown".into(),
            reserve_state: "reserve_unknown".into(),
        }
    }
    fn guard() -> ModeGuard {
        ModeGuard { guard_id:"fresh".into(), predicate:"input is fresh".into(), enforcement_owner:"independent_protection".into(), freshness_required:true, failure_behavior:"block_transition".into() }
    }
    fn transition(id: &str, recovery: Option<&str>) -> ModeTransition {
        ModeTransition { transition_id:id.into(),from_state_ids:vec!["grid_connected".into()],to_state_id:"unknown_hold".into(),
            permitted:true,actor:"local_controller".into(),authorization_owner:"local_controller".into(),
            protection_interlock_required:true,guard_ids:vec!["fresh".into()],postcondition_ids:vec!["state-observed".into()],
            failure_disposition:"remain safe".into(),recovery_transition_id:recovery.map(str::to_string),forbidden_reason:None }
    }
    fn test(id: &str, transition_id: &str, kind: &str) -> TransitionTestCase {
        let positive = kind == "positive";
        TransitionTestCase { test_case_id:id.into(),transition_id:transition_id.into(),test_kind:kind.into(),
            expected_decision:if positive || kind=="recovery" {"allow"} else {"block"}.into(),
            satisfied_guard_ids:if positive || kind=="recovery" {vec!["fresh".into()]} else {vec![]},
            violated_guard_ids:if kind=="negative" {vec!["fresh".into()]} else {vec![]},
            observed_postcondition_ids:if positive || kind=="recovery" {vec!["state-observed".into()]} else {vec![]},
            recovery_transition_id:None,evidence_class:"synthetic".into() }
    }
    fn valid_corpus() -> ModeTransitionCorpus {
        ModeTransitionCorpus {
            states:vec![state("grid_connected"),state("unknown_hold")],
            guards:vec![guard()],
            transitions:vec![transition("T1",None)],
            test_cases:vec![test("P1","T1","positive"),test("N1","T1","negative")],
        }
    }

    #[test] fn accepts_complete_positive_and_negative_coverage() { assert_eq!(validate_mode_transition_corpus(&valid_corpus()).unwrap().positive_test_count,1); }
    #[test] fn rejects_missing_positive_coverage() { let mut c=valid_corpus(); c.test_cases.retain(|x|x.test_kind!="positive"); assert!(validate_mode_transition_corpus(&c).is_err()); }
    #[test] fn rejects_missing_negative_coverage() { let mut c=valid_corpus(); c.test_cases.retain(|x|x.test_kind!="negative"); assert!(validate_mode_transition_corpus(&c).is_err()); }
    #[test] fn rejects_negative_test_without_a_violated_guard() { let mut c=valid_corpus(); c.test_cases[1].violated_guard_ids.clear(); assert!(validate_mode_transition_corpus(&c).is_err()); }
    #[test] fn rejects_unknown_guard_reference() { let mut c=valid_corpus(); c.transitions[0].guard_ids=vec!["missing".into()]; assert!(validate_mode_transition_corpus(&c).is_err()); }
    #[test] fn rejects_unknown_source_state() { let mut c=valid_corpus(); c.transitions[0].from_state_ids=vec!["missing".into()]; assert!(validate_mode_transition_corpus(&c).is_err()); }
    #[test] fn rejects_protection_interlock_owned_by_supervisory_mems() { let mut c=valid_corpus(); c.transitions[0].authorization_owner="supervisory_mems".into(); assert!(validate_mode_transition_corpus(&c).is_err()); }
    #[test] fn rejects_forbidden_transition_positive_test() { let mut c=valid_corpus(); c.transitions[0].permitted=false; c.transitions[0].forbidden_reason=Some("not permitted".into()); assert!(validate_mode_transition_corpus(&c).is_err()); }
    #[test] fn rejects_forbidden_transition_without_reason() { let mut c=valid_corpus(); c.transitions[0].permitted=false; assert!(validate_mode_transition_corpus(&c).is_err()); }
    #[test] fn rejects_non_synthetic_test_evidence() { let mut c=valid_corpus(); c.test_cases[0].evidence_class="measured".into(); assert!(validate_mode_transition_corpus(&c).is_err()); }
    #[test] fn rejects_incomplete_positive_postconditions() { let mut c=valid_corpus(); c.test_cases[0].observed_postcondition_ids.clear(); assert!(validate_mode_transition_corpus(&c).is_err()); }
    #[test] fn rejects_unknown_hold_that_asserts_pcc_is_closed() { let mut c=valid_corpus(); c.states[1].pcc_breaker_state="closed".into(); assert!(validate_mode_transition_corpus(&c).is_err()); }
    #[test] fn rejects_duplicate_state_ids() { let mut c=valid_corpus(); c.states.push(c.states[0].clone()); assert!(validate_mode_transition_corpus(&c).is_err()); }
    #[test] fn rejects_duplicate_test_case_ids() { let mut c=valid_corpus(); c.test_cases[1].test_case_id=c.test_cases[0].test_case_id.clone(); assert!(validate_mode_transition_corpus(&c).is_err()); }
    #[test] fn rejects_recovery_route_that_does_not_start_at_target_state() {
        let mut c=valid_corpus(); let mut t=transition("T2",None); t.from_state_ids=vec!["grid_connected".into()];
        c.transitions.push(t); c.transitions[0].recovery_transition_id=Some("T2".into());
        c.test_cases.push(test("N2","T2","negative"));
        assert!(validate_mode_transition_corpus(&c).is_err());
    }

    #[test]
    fn accepts_recovery_test_against_the_recovery_transition_guards() {
        let mut c = valid_corpus();
        let mut recovery = transition("T2", None);
        recovery.from_state_ids = vec!["unknown_hold".into()];
        recovery.to_state_id = "grid_connected".into();
        recovery.postcondition_ids = vec!["grid-restored".into()];
        c.transitions[0].recovery_transition_id = Some("T2".into());
        c.transitions.push(recovery);
        let mut positive_recovery = test("P2", "T2", "positive");
        positive_recovery.observed_postcondition_ids = vec!["grid-restored".into()];
        c.test_cases.push(positive_recovery);
        c.test_cases.push(test("N2", "T2", "negative"));
        let mut recovery_case = test("R1", "T1", "recovery");
        recovery_case.observed_postcondition_ids = vec!["grid-restored".into()];
        recovery_case.recovery_transition_id = Some("T2".into());
        c.test_cases.push(recovery_case);
        assert!(validate_mode_transition_corpus(&c).is_ok());
    }

    #[test]
    fn rejects_recovery_test_that_omits_recovery_postconditions() {
        let mut c = valid_corpus();
        let mut recovery = transition("T2", None);
        recovery.from_state_ids = vec!["unknown_hold".into()];
        recovery.to_state_id = "grid_connected".into();
        recovery.postcondition_ids = vec!["grid-restored".into()];
        c.transitions[0].recovery_transition_id = Some("T2".into());
        c.transitions.push(recovery);
        let mut positive_recovery = test("P2", "T2", "positive");
        positive_recovery.observed_postcondition_ids = vec!["grid-restored".into()];
        c.test_cases.push(positive_recovery);
        c.test_cases.push(test("N2", "T2", "negative"));
        let mut recovery_case = test("R1", "T1", "recovery");
        recovery_case.recovery_transition_id = Some("T2".into());
        c.test_cases.push(recovery_case);
        assert!(validate_mode_transition_corpus(&c).is_err());
    }

    #[test]
    fn protection_interlock_requires_independent_guard_not_protection_as_request_owner() {
        let mut c = valid_corpus();
        let mut maintenance_guard = guard();
        maintenance_guard.guard_id = "maint".into();
        maintenance_guard.enforcement_owner = "maintenance_authority".into();
        c.guards.push(maintenance_guard);
        c.transitions[0].authorization_owner = "maintenance_authority".into();
        c.transitions[0].guard_ids.push("maint".into());
        c.test_cases[0].satisfied_guard_ids.push("maint".into());
        c.test_cases[1].satisfied_guard_ids.push("maint".into());
        c.test_cases[1].violated_guard_ids = vec!["fresh".into()];
        assert!(validate_mode_transition_corpus(&c).is_ok());
    }


    #[test]
    fn rejects_missing_single_guard_mutation_coverage() {
        let mut c = valid_corpus();
        let mut second = guard();
        second.guard_id = "second".into();
        c.guards.push(second);
        c.transitions[0].guard_ids.push("second".into());
        c.test_cases[0].satisfied_guard_ids.push("second".into());
        c.test_cases[1].satisfied_guard_ids.push("second".into());
        assert!(validate_mode_transition_corpus(&c).is_err());
    }

    #[test]
    fn rejects_negative_test_mutating_multiple_guards_at_once() {
        let mut c = valid_corpus();
        c.guards.push(ModeGuard {
            guard_id: "second".into(), predicate: "second condition".into(),
            enforcement_owner: "independent_protection".into(), freshness_required: true,
            failure_behavior: "block_transition".into()
        });
        c.transitions[0].guard_ids.push("second".into());
        c.test_cases[0].satisfied_guard_ids.push("second".into());
        c.test_cases[1].violated_guard_ids.push("second".into());
        assert!(validate_mode_transition_corpus(&c).is_err());
    }

}
