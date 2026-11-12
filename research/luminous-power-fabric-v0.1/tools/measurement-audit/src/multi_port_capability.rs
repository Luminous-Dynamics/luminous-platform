use crate::AuditError;
use std::collections::BTreeSet;

/// Current evidence and availability state for one already-evaluated power path.
///
/// The multi-path evaluator only credits QualifiedAvailable paths. The upstream
/// single-port evaluator and evidence verifier must establish that state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathAvailability {
    QualifiedAvailable,
    EvidenceBlocked,
    ProtectionNotReady,
    Faulted,
    Unknown,
}

/// A pre-composed source-to-load route. Its continuous power is the already
/// qualified path bottleneck, not a sum of its series components.
#[derive(Debug, Clone, PartialEq)]
pub struct CapabilityPath {
    pub path_id: String,
    /// Exact qualified configuration digest for this source-to-service route.
    pub configuration_sha256: String,
    /// Reference to the upstream single-path capability/evidence result.
    pub capability_evidence_reference: String,
    pub available_power_w: f64,
    pub availability: PathAvailability,
    /// Exact component/dependency IDs in this route: source, converter, bus,
    /// branch protection, cable, connector, control supply, etc.
    pub component_ids: Vec<String>,
    /// Shared dependencies whose failure can remove several otherwise healthy
    /// paths: shared cooling, enclosure, upstream bus, common control, firmware
    /// cohort, common protective device, and similar declared cut sets.
    pub failure_domain_ids: Vec<String>,
}

/// An explicitly declared joint contingency. Single component and single
/// failure-domain scenarios are generated automatically by the evaluator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclaredFailureScenario {
    pub scenario_id: String,
    pub failed_component_ids: Vec<String>,
    pub failed_domain_ids: Vec<String>,
}

/// Input to the common-cause-aware multi-path capability screen.
#[derive(Debug, Clone, PartialEq)]
pub struct MultiPathCapabilityInput {
    /// All route capacities must terminate at this same useful-service boundary.
    /// Examples: one protected DC bus, one AC load bus, or one declared load group.
    pub service_boundary_id: String,
    pub paths: Vec<CapabilityPath>,
    /// Optional N-k / compound scenarios not represented by individual
    /// component or domain failure. These must reference known IDs.
    pub additional_scenarios: Vec<DeclaredFailureScenario>,
    /// Optional useful-service threshold for a simple robust-feasibility flag.
    pub required_service_power_w: Option<f64>,
}

/// Result of replaying one explicit failure scenario.
#[derive(Debug, Clone, PartialEq)]
pub struct ScenarioCapability {
    pub scenario_id: String,
    pub available_power_w: f64,
    pub unavailable_path_ids: Vec<String>,
}

/// Conservative system-level capability under all enumerated single failures
/// plus the caller's declared compound contingencies.
#[derive(Debug, Clone, PartialEq)]
pub struct PathEvidenceReference {
    pub path_id: String,
    pub configuration_sha256: String,
    pub capability_evidence_reference: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MultiPathCapabilityReport {
    pub service_boundary_id: String,
    pub path_evidence: Vec<PathEvidenceReference>,
    pub baseline_available_power_w: f64,
    pub robust_available_power_w: f64,
    pub required_service_power_w: Option<f64>,
    pub meets_required_service: Option<bool>,
    pub limiting_scenario_id: String,
    pub scenarios_evaluated: usize,
    pub scenario_results: Vec<ScenarioCapability>,
    pub claim_ceiling: &'static str,
}

impl MultiPathCapabilityReport {
    /// This is a declared-contingency arithmetic screen, not a reliability
    /// probability, fault-current model, protective-device test or safety case.
    pub const fn claim_ceiling(&self) -> &'static str {
        self.claim_ceiling
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NormalizedScenario {
    scenario_id: String,
    failed_component_ids: BTreeSet<String>,
    failed_domain_ids: BTreeSet<String>,
}

fn validate_id(value: &str, field: &str) -> Result<(), AuditError> {
    if value.trim().is_empty() {
        return Err(AuditError(format!("{field} must not be empty")));
    }
    Ok(())
}

fn validate_unique_ids(values: &[String], field: &str) -> Result<(), AuditError> {
    let mut seen = BTreeSet::new();
    for value in values {
        validate_id(value, field)?;
        if !seen.insert(value) {
            return Err(AuditError(format!(
                "{field} contains duplicate identifier {value:?}"
            )));
        }
    }
    Ok(())
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn validate_power(value: f64, field: &str) -> Result<(), AuditError> {
    if !value.is_finite() || value < 0.0 {
        return Err(AuditError(format!(
            "{field} must be finite and non-negative"
        )));
    }
    Ok(())
}

fn path_is_eligible(path: &CapabilityPath) -> bool {
    path.availability == PathAvailability::QualifiedAvailable
        && path.available_power_w > 0.0
}

fn scenario_capacity(
    scenario: &NormalizedScenario,
    paths: &[CapabilityPath],
) -> Result<ScenarioCapability, AuditError> {
    let mut available_power_w = 0.0_f64;
    let mut unavailable_path_ids = Vec::new();

    for path in paths {
        let component_failed = path
            .component_ids
            .iter()
            .any(|id| scenario.failed_component_ids.contains(id));
        let domain_failed = path
            .failure_domain_ids
            .iter()
            .any(|id| scenario.failed_domain_ids.contains(id));

        if !path_is_eligible(path) || component_failed || domain_failed {
            unavailable_path_ids.push(path.path_id.clone());
        } else {
            available_power_w += path.available_power_w;
        }
    }

    if !available_power_w.is_finite() {
        return Err(AuditError(format!(
            "available power overflow in scenario {:?}",
            scenario.scenario_id
        )));
    }

    unavailable_path_ids.sort();
    Ok(ScenarioCapability {
        scenario_id: scenario.scenario_id.clone(),
        available_power_w,
        unavailable_path_ids,
    })
}

/// Compose pre-qualified, parallel source-to-load paths under declared
/// component and common-cause contingencies.
///
/// The evaluator:
/// - validates unique path/component/domain identifiers and all references;
/// - credits only paths already marked QualifiedAvailable;
/// - calculates the baseline sum of parallel route bottlenecks;
/// - automatically sweeps each unique component failure and each unique
///   shared-domain failure;
/// - evaluates explicitly declared compound contingencies;
/// - sets robust capability to the minimum surviving capacity across the
///   baseline and every evaluated contingency.
///
/// A component or domain shared by multiple paths removes all those paths
/// together. This prevents false N-1 credit from duplicated module ratings.
///
/// The model is deliberately conservative and deterministic. It does not
/// infer failure probabilities, discover undeclared common causes, calculate
/// series-component bottlenecks, model AC power flow, calculate fault current,
/// simulate relay/selectivity behavior, or authorize switching. The caller
/// must provide a complete, reviewed dependency graph and each route's
/// already-qualified continuous power bottleneck.
pub fn evaluate_multi_path_capability(
    input: &MultiPathCapabilityInput,
) -> Result<MultiPathCapabilityReport, AuditError> {
    validate_id(&input.service_boundary_id, "service_boundary_id")?;
    if input.paths.is_empty() {
        return Err(AuditError(
            "at least one capability path is required".to_string(),
        ));
    }

    if let Some(required) = input.required_service_power_w {
        if !required.is_finite() || required <= 0.0 {
            return Err(AuditError(
                "required_service_power_w must be finite and positive".to_string(),
            ));
        }
    }

    let mut ordered_paths = input.paths.clone();
    ordered_paths.sort_by(|left, right| left.path_id.cmp(&right.path_id));

    let mut path_ids = BTreeSet::new();
    let mut component_ids = BTreeSet::new();
    let mut domain_ids = BTreeSet::new();
    let mut baseline_available_power_w = 0.0_f64;

    for path in &ordered_paths {
        validate_id(&path.path_id, "path_id")?;
        if !path_ids.insert(path.path_id.clone()) {
            return Err(AuditError(format!(
                "duplicate path_id {:?}",
                path.path_id
            )));
        }
        if !valid_sha256(&path.configuration_sha256) {
            return Err(AuditError(format!(
                "path {:?} has a malformed configuration SHA-256",
                path.path_id
            )));
        }
        validate_id(
            &path.capability_evidence_reference,
            "capability_evidence_reference",
        )?;
        validate_power(path.available_power_w, "path.available_power_w")?;
        if path.component_ids.is_empty() {
            return Err(AuditError(format!(
                "path {:?} must declare at least one component ID",
                path.path_id
            )));
        }
        validate_unique_ids(&path.component_ids, "component_ids")?;
        validate_unique_ids(&path.failure_domain_ids, "failure_domain_ids")?;

        for id in &path.component_ids {
            component_ids.insert(id.clone());
        }
        for id in &path.failure_domain_ids {
            domain_ids.insert(id.clone());
        }

        match path.availability {
            PathAvailability::QualifiedAvailable => {
                baseline_available_power_w += path.available_power_w;
            }
            PathAvailability::EvidenceBlocked
            | PathAvailability::ProtectionNotReady
            | PathAvailability::Faulted
            | PathAvailability::Unknown => {
                // Blocked paths cannot contribute any capability, even if a
                // stale caller supplied a positive available_power_w.
            }
        }
    }

    if !baseline_available_power_w.is_finite() {
        return Err(AuditError(
            "baseline available power overflow".to_string(),
        ));
    }

    let mut normalized = Vec::new();

    for id in &component_ids {
        normalized.push(NormalizedScenario {
            scenario_id: format!("component:{id}"),
            failed_component_ids: BTreeSet::from([id.clone()]),
            failed_domain_ids: BTreeSet::new(),
        });
    }

    for id in &domain_ids {
        normalized.push(NormalizedScenario {
            scenario_id: format!("domain:{id}"),
            failed_component_ids: BTreeSet::new(),
            failed_domain_ids: BTreeSet::from([id.clone()]),
        });
    }

    let mut scenario_ids: BTreeSet<String> = normalized
        .iter()
        .map(|scenario| scenario.scenario_id.clone())
        .collect();
    // Reserve baseline so caller-defined contingencies cannot shadow it.
    scenario_ids.insert("baseline".to_string());

    for scenario in &input.additional_scenarios {
        validate_id(&scenario.scenario_id, "scenario_id")?;
        if !scenario_ids.insert(scenario.scenario_id.clone()) {
            return Err(AuditError(format!(
                "duplicate or reserved scenario_id {:?}",
                scenario.scenario_id
            )));
        }
        if scenario.failed_component_ids.is_empty()
            && scenario.failed_domain_ids.is_empty()
        {
            return Err(AuditError(format!(
                "scenario {:?} must fail at least one component or domain",
                scenario.scenario_id
            )));
        }
        validate_unique_ids(
            &scenario.failed_component_ids,
            "scenario.failed_component_ids",
        )?;
        validate_unique_ids(
            &scenario.failed_domain_ids,
            "scenario.failed_domain_ids",
        )?;

        for id in &scenario.failed_component_ids {
            if !component_ids.contains(id) {
                return Err(AuditError(format!(
                    "scenario {:?} references unknown component {:?}",
                    scenario.scenario_id, id
                )));
            }
        }
        for id in &scenario.failed_domain_ids {
            if !domain_ids.contains(id) {
                return Err(AuditError(format!(
                    "scenario {:?} references unknown failure domain {:?}",
                    scenario.scenario_id, id
                )));
            }
        }

        normalized.push(NormalizedScenario {
            scenario_id: scenario.scenario_id.clone(),
            failed_component_ids: scenario.failed_component_ids.iter().cloned().collect(),
            failed_domain_ids: scenario.failed_domain_ids.iter().cloned().collect(),
        });
    }

    normalized.sort_by(|left, right| left.scenario_id.cmp(&right.scenario_id));

    let mut scenario_results = Vec::with_capacity(normalized.len() + 1);
    scenario_results.push(ScenarioCapability {
        scenario_id: "baseline".to_string(),
        available_power_w: baseline_available_power_w,
        unavailable_path_ids: ordered_paths
            .iter()
            .filter(|path| !path_is_eligible(path))
            .map(|path| path.path_id.clone())
            .collect(),
    });

    for scenario in &normalized {
        scenario_results.push(scenario_capacity(scenario, &ordered_paths)?);
    }

    let mut limiting = scenario_results
        .iter()
        .min_by(|left, right| {
            left.available_power_w
                .total_cmp(&right.available_power_w)
                .then_with(|| left.scenario_id.cmp(&right.scenario_id))
        })
        .ok_or_else(|| AuditError("no scenarios were evaluated".to_string()))?
        .clone();

    // Baseline is always included. This also handles a topology with no
    // declared dependency IDs without inventing a failure scenario.
    if limiting.scenario_id.is_empty() {
        limiting.scenario_id = "baseline".to_string();
    }

    let robust_available_power_w = limiting.available_power_w;
    let meets_required_service = input
        .required_service_power_w
        .map(|required| robust_available_power_w >= required);

    let path_evidence = ordered_paths
        .iter()
        .map(|path| PathEvidenceReference {
            path_id: path.path_id.clone(),
            configuration_sha256: path.configuration_sha256.clone(),
            capability_evidence_reference: path.capability_evidence_reference.clone(),
        })
        .collect();

    Ok(MultiPathCapabilityReport {
        service_boundary_id: input.service_boundary_id.clone(),
        path_evidence,
        baseline_available_power_w,
        robust_available_power_w,
        required_service_power_w: input.required_service_power_w,
        meets_required_service,
        limiting_scenario_id: limiting.scenario_id,
        scenarios_evaluated: scenario_results.len(),
        scenario_results,
        claim_ceiling:
            "DECLARED_CONTINGENCY_CAPABILITY_SCREEN_ONLY_NOT_RELIABILITY_OR_PHYSICAL_QUALIFICATION",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path(
        id: &str,
        power_w: f64,
        components: &[&str],
        domains: &[&str],
    ) -> CapabilityPath {
        CapabilityPath {
            path_id: id.to_string(),
            configuration_sha256: "a".repeat(64),
            capability_evidence_reference: format!("single-path-capability:{id}"),
            available_power_w: power_w,
            availability: PathAvailability::QualifiedAvailable,
            component_ids: components.iter().map(|x| x.to_string()).collect(),
            failure_domain_ids: domains.iter().map(|x| x.to_string()).collect(),
        }
    }

    fn input(paths: Vec<CapabilityPath>) -> MultiPathCapabilityInput {
        MultiPathCapabilityInput {
            service_boundary_id: "common-useful-service-boundary".to_string(),
            paths,
            additional_scenarios: Vec::new(),
            required_service_power_w: None,
        }
    }

    #[test]
    fn all_path_capacities_are_bound_to_one_service_boundary() {
        let report = evaluate_multi_path_capability(&input(vec![
            path("path-a", 100.0, &["a"], &[]),
            path("path-b", 80.0, &["b"], &[]),
        ]))
        .unwrap();
        assert_eq!(
            report.service_boundary_id,
            "common-useful-service-boundary"
        );
    }

    #[test]
    fn empty_service_boundary_is_rejected() {
        let mut value = input(vec![path("path-a", 10.0, &["a"], &[])]);
        value.service_boundary_id = "  ".to_string();
        assert!(evaluate_multi_path_capability(&value).is_err());
    }

    #[test]
    fn malformed_path_configuration_digest_is_rejected() {
        let mut value = input(vec![path("path-a", 10.0, &["a"], &[])]);
        value.paths[0].configuration_sha256 = "not-a-digest".to_string();
        assert!(evaluate_multi_path_capability(&value).is_err());
    }

    #[test]
    fn missing_path_capability_evidence_reference_is_rejected() {
        let mut value = input(vec![path("path-a", 10.0, &["a"], &[])]);
        value.paths[0].capability_evidence_reference.clear();
        assert!(evaluate_multi_path_capability(&value).is_err());
    }

    #[test]
    fn report_retains_each_path_configuration_and_capability_reference() {
        let report = evaluate_multi_path_capability(&input(vec![
            path("path-b", 80.0, &["b"], &[]),
            path("path-a", 100.0, &["a"], &[]),
        ]))
        .unwrap();
        assert_eq!(report.path_evidence.len(), 2);
        assert_eq!(report.path_evidence[0].path_id, "path-a");
        assert_eq!(report.path_evidence[0].configuration_sha256, "a".repeat(64));
        assert_eq!(
            report.path_evidence[0].capability_evidence_reference,
            "single-path-capability:path-a"
        );
    }

    #[test]
    fn independent_parallel_paths_receive_n_minus_one_credit() {
        let report = evaluate_multi_path_capability(&input(vec![
            path("path-a", 100.0, &["src-a", "conv-a"], &["cool-a"]),
            path("path-b", 80.0, &["src-b", "conv-b"], &["cool-b"]),
        ]))
        .unwrap();

        assert_eq!(report.baseline_available_power_w, 180.0);
        assert_eq!(report.robust_available_power_w, 80.0);
        assert_eq!(report.limiting_scenario_id, "component:conv-a");
    }

    #[test]
    fn shared_source_failure_removes_all_dependent_paths() {
        let report = evaluate_multi_path_capability(&input(vec![
            path("path-a", 100.0, &["shared-source", "conv-a"], &[]),
            path("path-b", 80.0, &["shared-source", "conv-b"], &[]),
        ]))
        .unwrap();

        assert_eq!(report.baseline_available_power_w, 180.0);
        assert_eq!(report.robust_available_power_w, 0.0);
        assert_eq!(report.limiting_scenario_id, "component:shared-source");
    }

    #[test]
    fn shared_cooling_domain_removes_all_dependent_paths() {
        let report = evaluate_multi_path_capability(&input(vec![
            path("path-a", 100.0, &["conv-a"], &["cooling-loop"]),
            path("path-b", 80.0, &["conv-b"], &["cooling-loop"]),
        ]))
        .unwrap();

        assert_eq!(report.robust_available_power_w, 0.0);
        assert_eq!(report.limiting_scenario_id, "domain:cooling-loop");
    }

    #[test]
    fn independent_failure_domains_preserve_the_unaffected_path() {
        let report = evaluate_multi_path_capability(&input(vec![
            path("path-a", 100.0, &["conv-a"], &["cooling-a"]),
            path("path-b", 80.0, &["conv-b"], &["cooling-b"]),
            path("path-c", 60.0, &["conv-c"], &["cooling-c"]),
        ]))
        .unwrap();

        assert_eq!(report.baseline_available_power_w, 240.0);
        assert_eq!(report.robust_available_power_w, 140.0);
    }

    #[test]
    fn blocked_path_never_contributes_positive_power() {
        let mut blocked = path("path-b", 500.0, &["conv-b"], &[]);
        blocked.availability = PathAvailability::ProtectionNotReady;
        let report = evaluate_multi_path_capability(&input(vec![
            path("path-a", 100.0, &["conv-a"], &[]),
            blocked,
        ]))
        .unwrap();

        assert_eq!(report.baseline_available_power_w, 100.0);
        assert_eq!(report.robust_available_power_w, 0.0);
        assert!(report.scenario_results[0]
            .unavailable_path_ids
            .contains(&"path-b".to_string()));
    }

    #[test]
    fn one_faulted_shared_controller_removes_every_path_that_uses_it() {
        let report = evaluate_multi_path_capability(&input(vec![
            path("path-a", 50.0, &["controller", "conv-a"], &[]),
            path("path-b", 50.0, &["controller", "conv-b"], &[]),
            path("path-c", 40.0, &["controller", "conv-c"], &[]),
        ]))
        .unwrap();

        assert_eq!(report.robust_available_power_w, 0.0);
    }

    #[test]
    fn declared_compound_fault_can_reduce_capacity_beyond_n_minus_one() {
        let mut value = input(vec![
            path("path-a", 100.0, &["a"], &["cool-a"]),
            path("path-b", 80.0, &["b"], &["cool-b"]),
            path("path-c", 60.0, &["c"], &["cool-c"]),
        ]);
        value.additional_scenarios.push(DeclaredFailureScenario {
            scenario_id: "two-cooling-failures".to_string(),
            failed_component_ids: Vec::new(),
            failed_domain_ids: vec!["cool-a".to_string(), "cool-b".to_string()],
        });

        let report = evaluate_multi_path_capability(&value).unwrap();
        assert_eq!(report.robust_available_power_w, 60.0);
        assert!(report
            .scenario_results
            .iter()
            .any(|scenario| scenario.scenario_id == "two-cooling-failures"
                && scenario.available_power_w == 60.0));
    }

    #[test]
    fn service_requirement_is_evaluated_against_robust_not_baseline_power() {
        let mut value = input(vec![
            path("path-a", 100.0, &["a"], &[]),
            path("path-b", 80.0, &["b"], &[]),
        ]);
        value.required_service_power_w = Some(90.0);
        let report = evaluate_multi_path_capability(&value).unwrap();

        assert_eq!(report.baseline_available_power_w, 180.0);
        assert_eq!(report.robust_available_power_w, 80.0);
        assert_eq!(report.meets_required_service, Some(false));
    }

    #[test]
    fn no_common_cause_credit_is_given_when_dependency_ids_differ() {
        let report = evaluate_multi_path_capability(&input(vec![
            path("path-a", 100.0, &["source-a", "bus-a"], &["control-a"]),
            path("path-b", 100.0, &["source-b", "bus-b"], &["control-b"]),
        ]))
        .unwrap();

        assert_eq!(report.robust_available_power_w, 100.0);
    }

    #[test]
    fn unavailable_path_states_are_excluded_from_baseline() {
        let mut paths = vec![
            path("path-a", 80.0, &["a"], &[]),
            path("path-b", 80.0, &["b"], &[]),
        ];
        paths[1].availability = PathAvailability::EvidenceBlocked;
        let report = evaluate_multi_path_capability(&input(paths)).unwrap();
        assert_eq!(report.baseline_available_power_w, 80.0);
    }

    #[test]
    fn empty_path_set_is_rejected() {
        assert!(evaluate_multi_path_capability(&input(Vec::new())).is_err());
    }

    #[test]
    fn duplicate_path_ids_are_rejected() {
        assert!(evaluate_multi_path_capability(&input(vec![
            path("same", 10.0, &["a"], &[]),
            path("same", 20.0, &["b"], &[]),
        ]))
        .is_err());
    }

    #[test]
    fn duplicate_component_reference_within_path_is_rejected() {
        assert!(evaluate_multi_path_capability(&input(vec![path(
            "path-a",
            10.0,
            &["a", "a"],
            &[]
        )]))
        .is_err());
    }

    #[test]
    fn empty_component_list_is_rejected() {
        assert!(evaluate_multi_path_capability(&input(vec![CapabilityPath {
            path_id: "path-a".to_string(),
            configuration_sha256: "a".repeat(64),
            capability_evidence_reference: "single-path-capability:path-a".to_string(),
            available_power_w: 10.0,
            availability: PathAvailability::QualifiedAvailable,
            component_ids: Vec::new(),
            failure_domain_ids: Vec::new(),
        }]))
        .is_err());
    }

    #[test]
    fn non_finite_path_power_is_rejected() {
        assert!(evaluate_multi_path_capability(&input(vec![path(
            "path-a",
            f64::INFINITY,
            &["a"],
            &[]
        )]))
        .is_err());
    }

    #[test]
    fn negative_path_power_is_rejected() {
        assert!(evaluate_multi_path_capability(&input(vec![path(
            "path-a",
            -1.0,
            &["a"],
            &[]
        )]))
        .is_err());
    }

    #[test]
    fn baseline_power_overflow_is_rejected() {
        assert!(evaluate_multi_path_capability(&input(vec![
            path("path-a", f64::MAX, &["a"], &[]),
            path("path-b", f64::MAX, &["b"], &[]),
        ]))
        .is_err());
    }

    #[test]
    fn unknown_component_in_additional_scenario_is_rejected() {
        let mut value = input(vec![path("path-a", 10.0, &["a"], &[])]);
        value.additional_scenarios.push(DeclaredFailureScenario {
            scenario_id: "bad".to_string(),
            failed_component_ids: vec!["not-present".to_string()],
            failed_domain_ids: Vec::new(),
        });
        assert!(evaluate_multi_path_capability(&value).is_err());
    }

    #[test]
    fn unknown_domain_in_additional_scenario_is_rejected() {
        let mut value = input(vec![path("path-a", 10.0, &["a"], &[])]);
        value.additional_scenarios.push(DeclaredFailureScenario {
            scenario_id: "bad".to_string(),
            failed_component_ids: Vec::new(),
            failed_domain_ids: vec!["not-present".to_string()],
        });
        assert!(evaluate_multi_path_capability(&value).is_err());
    }

    #[test]
    fn empty_additional_scenario_is_rejected() {
        let mut value = input(vec![path("path-a", 10.0, &["a"], &[])]);
        value.additional_scenarios.push(DeclaredFailureScenario {
            scenario_id: "empty".to_string(),
            failed_component_ids: Vec::new(),
            failed_domain_ids: Vec::new(),
        });
        assert!(evaluate_multi_path_capability(&value).is_err());
    }

    #[test]
    fn duplicate_failure_scenario_ids_are_rejected() {
        let mut value = input(vec![path("path-a", 10.0, &["a"], &[])]);
        value.additional_scenarios.push(DeclaredFailureScenario {
            scenario_id: "compound".to_string(),
            failed_component_ids: vec!["a".to_string()],
            failed_domain_ids: Vec::new(),
        });
        value.additional_scenarios.push(DeclaredFailureScenario {
            scenario_id: "compound".to_string(),
            failed_component_ids: vec!["a".to_string()],
            failed_domain_ids: Vec::new(),
        });
        assert!(evaluate_multi_path_capability(&value).is_err());
    }

    #[test]
    fn baseline_scenario_identifier_is_reserved() {
        let mut value = input(vec![path("path-a", 10.0, &["a"], &[])]);
        value.additional_scenarios.push(DeclaredFailureScenario {
            scenario_id: "baseline".to_string(),
            failed_component_ids: vec!["a".to_string()],
            failed_domain_ids: Vec::new(),
        });
        assert!(evaluate_multi_path_capability(&value).is_err());
    }

    #[test]
    fn zero_power_requirement_is_rejected() {
        let mut value = input(vec![path("path-a", 10.0, &["a"], &[])]);
        value.required_service_power_w = Some(0.0);
        assert!(evaluate_multi_path_capability(&value).is_err());
    }

    #[test]
    fn report_has_explicit_non_qualification_claim_ceiling() {
        let report = evaluate_multi_path_capability(&input(vec![path(
            "path-a",
            10.0,
            &["a"],
            &[]
        )]))
        .unwrap();
        assert_eq!(
            report.claim_ceiling(),
            "DECLARED_CONTINGENCY_CAPABILITY_SCREEN_ONLY_NOT_RELIABILITY_OR_PHYSICAL_QUALIFICATION"
        );
    }

    #[test]
    fn scenario_order_is_deterministic() {
        let first = evaluate_multi_path_capability(&input(vec![
            path("path-b", 80.0, &["b"], &["db"]),
            path("path-a", 100.0, &["a"], &["da"]),
        ]))
        .unwrap();
        let second = evaluate_multi_path_capability(&input(vec![
            path("path-a", 100.0, &["a"], &["da"]),
            path("path-b", 80.0, &["b"], &["db"]),
        ]))
        .unwrap();

        assert_eq!(
            first
                .scenario_results
                .iter()
                .map(|scenario| scenario.scenario_id.clone())
                .collect::<Vec<_>>(),
            second
                .scenario_results
                .iter()
                .map(|scenario| scenario.scenario_id.clone())
                .collect::<Vec<_>>()
        );
        assert_eq!(first, second);
    }
}
