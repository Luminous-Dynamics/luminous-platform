//! Evidence-source corroboration using the existing failure-domain dependency graph.
//!
//! Source IDs and control-domain labels are descriptive metadata, not independent
//! votes. A candidate is marked PairwiseNoDeclaredOverlap only when every pair
//! of current control domains has a complete, fresh, revision-matched inventory
//! and disjoint declared dependency closure. This remains structural screening,
//! not proof of operational or cryptographic independence.

use std::collections::{BTreeMap, BTreeSet};

use crate::failure_domains::{
    audit_observer_independence, ControlDomainProfile, FailureDomainGraph, IndependencePolicy,
    IndependenceReason, IndependenceStatus,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvidenceFreshness {
    Current,
    Stale,
    FutureDated,
    Expired,
    Absent,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceAvailability {
    Present,
    Absent,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceEvidence {
    pub evidence_id: String,
    pub object_ref: String,
    pub source_id: String,
    /// A declared label used to look up a profile; it is not an independence claim.
    pub control_domain: String,
    pub availability: EvidenceAvailability,
    pub event_tick: u64,
    pub observed_at_tick: u64,
    /// Exclusive validity bound. None means the source did not provide one.
    pub valid_until_tick: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PairwiseStatus {
    NoDeclaredOverlap,
    SharedDeclaredDependency,
    Indeterminate,
    Invalid,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CandidateCorroborationStatus {
    NoCurrentEvidence,
    SingleDeclaredControlDomain,
    /// All current-domain pairs have disjoint declared closures; not proof.
    PairwiseNoDeclaredOverlap,
    SharedDeclaredDependency,
    Indeterminate,
    Invalid,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CorroborationReason {
    MissingFailureDomainProfile,
    PairwiseAuditIndeterminate,
    PairwiseAuditInvalid,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PairwiseCorroboration {
    pub left_control_domain: String,
    pub right_control_domain: String,
    pub status: PairwiseStatus,
    pub shared_dependencies: Vec<String>,
    pub shared_failure_domain_classes: Vec<String>,
    pub independence_reasons: Vec<IndependenceReason>,
    pub reasons: Vec<CorroborationReason>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CandidateCorroboration {
    pub object_ref: String,
    /// Unique source IDs over all records, including stale and unknown records.
    pub declared_source_count: usize,
    /// Unique control-domain labels over all records, including stale records.
    pub declared_control_domain_count: usize,
    pub current_evidence_count: usize,
    pub current_source_count: usize,
    pub current_control_domain_count: usize,
    /// Count of pairwise no-declared-overlap results, never a count of proven-independent sources.
    pub no_declared_overlap_pair_count: usize,
    pub shared_dependency_pair_count: usize,
    pub indeterminate_pair_count: usize,
    pub invalid_pair_count: usize,
    pub status: CandidateCorroborationStatus,
    pub pairwise: Vec<PairwiseCorroboration>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CorroborationError {
    EmptyEvidenceId,
    EmptyObjectRef,
    EmptySourceId,
    EmptyControlDomain,
    DuplicateEvidenceId,
    InvalidProfileBinding,
    InvalidEvidenceInterval,
}

fn non_empty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn classify_freshness(
    item: &SourceEvidence,
    audited_at_tick: u64,
    maximum_evidence_age_ticks: u64,
) -> EvidenceFreshness {
    match item.availability {
        EvidenceAvailability::Absent => EvidenceFreshness::Absent,
        EvidenceAvailability::Unknown => EvidenceFreshness::Unknown,
        EvidenceAvailability::Present => {
            if item.observed_at_tick > audited_at_tick {
                EvidenceFreshness::FutureDated
            } else if item
                .valid_until_tick
                .is_some_and(|valid_until| valid_until <= audited_at_tick)
            {
                EvidenceFreshness::Expired
            } else if audited_at_tick.saturating_sub(item.observed_at_tick)
                > maximum_evidence_age_ticks
            {
                EvidenceFreshness::Stale
            } else {
                EvidenceFreshness::Current
            }
        }
    }
}

fn validate_evidence(evidence: &[SourceEvidence]) -> Result<(), CorroborationError> {
    let mut identities = BTreeSet::new();
    for item in evidence {
        if !non_empty(&item.evidence_id) {
            return Err(CorroborationError::EmptyEvidenceId);
        }
        if !non_empty(&item.object_ref) {
            return Err(CorroborationError::EmptyObjectRef);
        }
        if !non_empty(&item.source_id) {
            return Err(CorroborationError::EmptySourceId);
        }
        if !non_empty(&item.control_domain) {
            return Err(CorroborationError::EmptyControlDomain);
        }
        if !identities.insert(item.evidence_id.as_str()) {
            return Err(CorroborationError::DuplicateEvidenceId);
        }
        if item
            .valid_until_tick
            .is_some_and(|valid_until| valid_until <= item.observed_at_tick)
        {
            return Err(CorroborationError::InvalidEvidenceInterval);
        }
    }
    Ok(())
}

fn compare_pair(
    left: &str,
    right: &str,
    profiles: &BTreeMap<String, ControlDomainProfile>,
    graph: &FailureDomainGraph,
    audited_at_tick: u64,
    policy: IndependencePolicy,
) -> Result<PairwiseCorroboration, CorroborationError> {
    let mut pair = PairwiseCorroboration {
        left_control_domain: left.to_owned(),
        right_control_domain: right.to_owned(),
        status: PairwiseStatus::Indeterminate,
        shared_dependencies: Vec::new(),
        shared_failure_domain_classes: Vec::new(),
        independence_reasons: Vec::new(),
        reasons: Vec::new(),
    };

    let (Some(left_profile), Some(right_profile)) = (profiles.get(left), profiles.get(right))
    else {
        pair.reasons
            .push(CorroborationReason::MissingFailureDomainProfile);
        return Ok(pair);
    };

    if left_profile.subject_id != left || right_profile.subject_id != right {
        return Err(CorroborationError::InvalidProfileBinding);
    }

    let report =
        audit_observer_independence(left_profile, right_profile, graph, audited_at_tick, policy);
    pair.shared_dependencies = report.shared_dependencies;
    pair.shared_failure_domain_classes = report.shared_failure_domain_classes;
    pair.independence_reasons = report.reasons;
    pair.status = match report.status {
        IndependenceStatus::NoDeclaredOverlap => PairwiseStatus::NoDeclaredOverlap,
        IndependenceStatus::SharedDeclaredDependency => PairwiseStatus::SharedDeclaredDependency,
        IndependenceStatus::Indeterminate => {
            pair.reasons
                .push(CorroborationReason::PairwiseAuditIndeterminate);
            PairwiseStatus::Indeterminate
        }
        IndependenceStatus::Invalid => {
            pair.reasons.push(CorroborationReason::PairwiseAuditInvalid);
            PairwiseStatus::Invalid
        }
    };
    Ok(pair)
}

fn summarize_status(
    current_evidence_count: usize,
    current_domain_count: usize,
    pairs: &[PairwiseCorroboration],
) -> CandidateCorroborationStatus {
    if current_evidence_count == 0 {
        return CandidateCorroborationStatus::NoCurrentEvidence;
    }
    if current_domain_count < 2 {
        return CandidateCorroborationStatus::SingleDeclaredControlDomain;
    }
    if pairs
        .iter()
        .any(|pair| pair.status == PairwiseStatus::Invalid)
    {
        CandidateCorroborationStatus::Invalid
    } else if pairs
        .iter()
        .any(|pair| pair.status == PairwiseStatus::SharedDeclaredDependency)
    {
        CandidateCorroborationStatus::SharedDeclaredDependency
    } else if pairs
        .iter()
        .any(|pair| pair.status == PairwiseStatus::Indeterminate)
    {
        CandidateCorroborationStatus::Indeterminate
    } else if !pairs.is_empty()
        && pairs
            .iter()
            .all(|pair| pair.status == PairwiseStatus::NoDeclaredOverlap)
    {
        CandidateCorroborationStatus::PairwiseNoDeclaredOverlap
    } else {
        CandidateCorroborationStatus::Indeterminate
    }
}

/// Assess source corroboration independently for every candidate object.
///
/// Profiles are keyed by exact control-domain label. Only Current evidence is
/// compared. Missing, unknown, stale, expired, future-dated and incomparable
/// profiles never create a no-overlap result. Raw label counts are descriptive.
pub fn evaluate_evidence_corroboration(
    evidence: &[SourceEvidence],
    profiles: &BTreeMap<String, ControlDomainProfile>,
    graph: &FailureDomainGraph,
    audited_at_tick: u64,
    maximum_evidence_age_ticks: u64,
    policy: IndependencePolicy,
) -> Result<Vec<CandidateCorroboration>, CorroborationError> {
    validate_evidence(evidence)?;

    let mut by_object: BTreeMap<&str, Vec<&SourceEvidence>> = BTreeMap::new();
    for item in evidence {
        by_object
            .entry(item.object_ref.as_str())
            .or_default()
            .push(item);
    }

    let mut output = Vec::with_capacity(by_object.len());
    for (object_ref, records) in by_object {
        let declared_source_count = records
            .iter()
            .map(|item| item.source_id.as_str())
            .collect::<BTreeSet<_>>()
            .len();
        let declared_control_domain_count = records
            .iter()
            .map(|item| item.control_domain.as_str())
            .collect::<BTreeSet<_>>()
            .len();

        let current: Vec<_> = records
            .iter()
            .copied()
            .filter(|item| {
                classify_freshness(item, audited_at_tick, maximum_evidence_age_ticks)
                    == EvidenceFreshness::Current
            })
            .collect();
        let current_source_count = current
            .iter()
            .map(|item| item.source_id.as_str())
            .collect::<BTreeSet<_>>()
            .len();
        let current_domains: Vec<String> = current
            .iter()
            .map(|item| item.control_domain.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();

        let mut pairwise = Vec::new();
        for left_index in 0..current_domains.len() {
            for right_index in (left_index + 1)..current_domains.len() {
                pairwise.push(compare_pair(
                    &current_domains[left_index],
                    &current_domains[right_index],
                    profiles,
                    graph,
                    audited_at_tick,
                    policy,
                )?);
            }
        }

        let no_declared_overlap_pair_count = pairwise
            .iter()
            .filter(|pair| pair.status == PairwiseStatus::NoDeclaredOverlap)
            .count();
        let shared_dependency_pair_count = pairwise
            .iter()
            .filter(|pair| pair.status == PairwiseStatus::SharedDeclaredDependency)
            .count();
        let indeterminate_pair_count = pairwise
            .iter()
            .filter(|pair| pair.status == PairwiseStatus::Indeterminate)
            .count();
        let invalid_pair_count = pairwise
            .iter()
            .filter(|pair| pair.status == PairwiseStatus::Invalid)
            .count();
        let current_evidence_count = current.len();
        let current_control_domain_count = current_domains.len();
        let status = summarize_status(
            current_evidence_count,
            current_control_domain_count,
            &pairwise,
        );

        output.push(CandidateCorroboration {
            object_ref: object_ref.to_owned(),
            declared_source_count,
            declared_control_domain_count,
            current_evidence_count,
            current_source_count,
            current_control_domain_count,
            no_declared_overlap_pair_count,
            shared_dependency_pair_count,
            indeterminate_pair_count,
            invalid_pair_count,
            status,
            pairwise,
        });
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::failure_domains::{FailureDomainNode, InventoryState};

    fn node(id: &str, class: &str, dependencies: &[&str]) -> FailureDomainNode {
        FailureDomainNode {
            id: id.into(),
            class: class.into(),
            depends_on: dependencies.iter().map(|value| (*value).into()).collect(),
        }
    }

    fn graph(nodes: Vec<FailureDomainNode>) -> FailureDomainGraph {
        FailureDomainGraph {
            revision: "registry-v1".into(),
            nodes: nodes
                .into_iter()
                .map(|node| (node.id.clone(), node))
                .collect(),
        }
    }

    fn profile(subject: &str, root: &str, observed: u64, expires: u64) -> ControlDomainProfile {
        ControlDomainProfile {
            subject_id: subject.into(),
            profile_digest: format!(
                "sha256:{:064x}",
                subject
                    .bytes()
                    .fold(0_u64, |sum, byte| sum + u64::from(byte))
            ),
            graph_revision: "registry-v1".into(),
            roots: [root.to_owned()].into_iter().collect(),
            state: InventoryState::Complete,
            observed_at_tick: observed,
            valid_until_tick: expires,
        }
    }

    fn policy() -> IndependencePolicy {
        IndependencePolicy {
            maximum_inventory_age_ticks: 10,
        }
    }

    fn evidence(id: &str, object: &str, source: &str, domain: &str) -> SourceEvidence {
        SourceEvidence {
            evidence_id: id.into(),
            object_ref: object.into(),
            source_id: source.into(),
            control_domain: domain.into(),
            availability: EvidenceAvailability::Present,
            event_tick: 1,
            observed_at_tick: 2,
            valid_until_tick: Some(30),
        }
    }

    fn separated_graph() -> FailureDomainGraph {
        graph(vec![
            node("process-a", "process", &["host-a", "clock-a"]),
            node("process-b", "process", &["host-b", "clock-b"]),
            node("host-a", "host", &[]),
            node("clock-a", "clock", &[]),
            node("host-b", "host", &[]),
            node("clock-b", "clock", &[]),
        ])
    }

    #[test]
    fn multiple_source_ids_in_one_control_domain_are_not_independent_votes() {
        let graph = graph(vec![
            node("process-a", "process", &["shared-host"]),
            node("shared-host", "host", &[]),
        ]);
        let evidence = vec![
            evidence("ev-1", "checkout-api", "metrics-a", "domain-a"),
            evidence("ev-2", "checkout-api", "logs-a", "domain-a"),
            evidence("ev-3", "checkout-api", "traces-a", "domain-a"),
        ];
        let profiles = [(
            "domain-a".to_owned(),
            profile("domain-a", "process-a", 2, 30),
        )]
        .into_iter()
        .collect();

        let rows =
            evaluate_evidence_corroboration(&evidence, &profiles, &graph, 5, 10, policy()).unwrap();
        assert_eq!(rows[0].declared_source_count, 3);
        assert_eq!(rows[0].declared_control_domain_count, 1);
        assert_eq!(rows[0].current_control_domain_count, 1);
        assert_eq!(rows[0].no_declared_overlap_pair_count, 0);
        assert_eq!(
            rows[0].status,
            CandidateCorroborationStatus::SingleDeclaredControlDomain
        );
    }

    #[test]
    fn different_control_domain_labels_sharing_a_host_are_not_corroboration() {
        let graph = graph(vec![
            node("process-a", "process", &["host-a"]),
            node("process-b", "process", &["host-b"]),
            node("host-a", "host", &["shared-host"]),
            node("host-b", "host", &["shared-host"]),
            node("shared-host", "physical-host", &[]),
        ]);
        let evidence = vec![
            evidence("ev-1", "checkout-api", "metrics-a", "domain-a"),
            evidence("ev-2", "checkout-api", "logs-b", "domain-b"),
        ];
        let profiles = [
            (
                "domain-a".to_owned(),
                profile("domain-a", "process-a", 2, 30),
            ),
            (
                "domain-b".to_owned(),
                profile("domain-b", "process-b", 2, 30),
            ),
        ]
        .into_iter()
        .collect();

        let rows =
            evaluate_evidence_corroboration(&evidence, &profiles, &graph, 5, 10, policy()).unwrap();
        assert_eq!(
            rows[0].status,
            CandidateCorroborationStatus::SharedDeclaredDependency
        );
        assert_eq!(rows[0].shared_dependency_pair_count, 1);
        assert_eq!(rows[0].no_declared_overlap_pair_count, 0);
        assert_eq!(rows[0].pairwise[0].shared_dependencies, vec!["shared-host"]);
    }

    #[test]
    fn disjoint_complete_profiles_produce_only_a_structural_no_overlap_result() {
        let graph = separated_graph();
        let evidence = vec![
            evidence("ev-1", "checkout-api", "metrics-a", "domain-a"),
            evidence("ev-2", "checkout-api", "logs-b", "domain-b"),
        ];
        let profiles = [
            (
                "domain-a".to_owned(),
                profile("domain-a", "process-a", 2, 30),
            ),
            (
                "domain-b".to_owned(),
                profile("domain-b", "process-b", 2, 30),
            ),
        ]
        .into_iter()
        .collect();

        let rows =
            evaluate_evidence_corroboration(&evidence, &profiles, &graph, 5, 10, policy()).unwrap();
        assert_eq!(
            rows[0].status,
            CandidateCorroborationStatus::PairwiseNoDeclaredOverlap
        );
        assert_eq!(rows[0].no_declared_overlap_pair_count, 1);
        assert_eq!(rows[0].shared_dependency_pair_count, 0);
    }

    #[test]
    fn missing_inventory_is_indeterminate_not_independent() {
        let graph = separated_graph();
        let evidence = vec![
            evidence("ev-1", "checkout-api", "metrics-a", "domain-a"),
            evidence("ev-2", "checkout-api", "logs-b", "domain-b"),
        ];
        let profiles = [(
            "domain-a".to_owned(),
            profile("domain-a", "process-a", 2, 30),
        )]
        .into_iter()
        .collect();

        let rows =
            evaluate_evidence_corroboration(&evidence, &profiles, &graph, 5, 10, policy()).unwrap();
        assert_eq!(rows[0].status, CandidateCorroborationStatus::Indeterminate);
        assert_eq!(rows[0].indeterminate_pair_count, 1);
        assert_eq!(
            rows[0].pairwise[0].reasons,
            vec![CorroborationReason::MissingFailureDomainProfile]
        );
    }

    #[test]
    fn unknown_and_stale_profiles_are_indeterminate_not_a_bonus() {
        let graph = separated_graph();
        let evidence = vec![
            evidence("ev-1", "checkout-api", "metrics-a", "domain-a"),
            evidence("ev-2", "checkout-api", "logs-b", "domain-b"),
        ];
        let mut unknown = profile("domain-b", "process-b", 2, 30);
        unknown.state = InventoryState::Unknown;
        let profiles = [
            (
                "domain-a".to_owned(),
                profile("domain-a", "process-a", 2, 30),
            ),
            ("domain-b".to_owned(), unknown),
        ]
        .into_iter()
        .collect();
        let unknown_rows =
            evaluate_evidence_corroboration(&evidence, &profiles, &graph, 5, 10, policy()).unwrap();
        assert_eq!(
            unknown_rows[0].status,
            CandidateCorroborationStatus::Indeterminate
        );

        let stale_profiles = [
            (
                "domain-a".to_owned(),
                profile("domain-a", "process-a", 2, 30),
            ),
            (
                "domain-b".to_owned(),
                profile("domain-b", "process-b", 0, 30),
            ),
        ]
        .into_iter()
        .collect();
        let stale_rows =
            evaluate_evidence_corroboration(&evidence, &stale_profiles, &graph, 20, policy())
                .unwrap();
        assert_eq!(
            stale_rows[0].status,
            CandidateCorroborationStatus::Indeterminate
        );
    }

    #[test]
    fn evidence_freshness_is_computed_from_source_time_not_a_caller_label() {
        let stale = SourceEvidence {
            evidence_id: "stale".into(),
            object_ref: "service".into(),
            source_id: "logs".into(),
            control_domain: "domain-a".into(),
            availability: EvidenceAvailability::Present,
            event_tick: 1,
            observed_at_tick: 1,
            valid_until_tick: Some(50),
        };
        let future = SourceEvidence {
            evidence_id: "future".into(),
            observed_at_tick: 11,
            ..stale.clone()
        };
        let expired = SourceEvidence {
            evidence_id: "expired".into(),
            observed_at_tick: 2,
            valid_until_tick: Some(5),
            ..stale.clone()
        };
        let absent = SourceEvidence {
            evidence_id: "absent".into(),
            availability: EvidenceAvailability::Absent,
            ..stale.clone()
        };
        let unknown = SourceEvidence {
            evidence_id: "unknown".into(),
            availability: EvidenceAvailability::Unknown,
            ..stale.clone()
        };
        assert_eq!(classify_freshness(&stale, 20, 10), EvidenceFreshness::Stale);
        assert_eq!(classify_freshness(&future, 10, 10), EvidenceFreshness::FutureDated);
        assert_eq!(classify_freshness(&expired, 10, 10), EvidenceFreshness::Expired);
        assert_eq!(classify_freshness(&absent, 10, 10), EvidenceFreshness::Absent);
        assert_eq!(classify_freshness(&unknown, 10, 10), EvidenceFreshness::Unknown);
        assert_eq!(
            classify_freshness(&stale, 10, 10),
            EvidenceFreshness::Current
        );
    }

    #[test]
    fn malformed_evidence_validity_interval_is_rejected() {
        let mut bad = evidence("ev-1", "checkout-api", "metrics-a", "domain-a");
        bad.valid_until_tick = Some(bad.observed_at_tick);
        assert_eq!(
            evaluate_evidence_corroboration(
                &[bad],
                &BTreeMap::new(),
                &separated_graph(),
                5,
                10,
                policy(),
            ),
            Err(CorroborationError::InvalidEvidenceInterval)
        );
    }

    #[test]
    fn only_current_evidence_participates_in_pairwise_comparisons() {
        let graph = separated_graph();
        let evidence = vec![
            evidence("ev-1", "checkout-api", "metrics-a", "domain-a"),
            SourceEvidence {
                evidence_id: "ev-2".into(),
                object_ref: "checkout-api".into(),
                source_id: "logs-b".into(),
                control_domain: "domain-b".into(),
                availability: EvidenceAvailability::Present,
                event_tick: 1,
                observed_at_tick: 0,
                valid_until_tick: Some(30),
            },
        ];
        let profiles = [
            (
                "domain-a".to_owned(),
                profile("domain-a", "process-a", 2, 30),
            ),
            (
                "domain-b".to_owned(),
                profile("domain-b", "process-b", 2, 30),
            ),
        ]
        .into_iter()
        .collect();

        let rows =
            evaluate_evidence_corroboration(&evidence, &profiles, &graph, 5, 10, policy()).unwrap();
        assert_eq!(rows[0].declared_control_domain_count, 2);
        assert_eq!(rows[0].current_control_domain_count, 1);
        assert_eq!(
            rows[0].status,
            CandidateCorroborationStatus::SingleDeclaredControlDomain
        );
        assert!(rows[0].pairwise.is_empty());
    }

    #[test]
    fn every_pair_must_be_disjoint_before_candidate_is_marked_no_overlap() {
        let graph = graph(vec![
            node("process-a", "process", &["host-a"]),
            node("process-b", "process", &["host-b"]),
            node("process-c", "process", &["host-c"]),
            node("host-a", "host", &[]),
            node("host-b", "host", &[]),
            node("host-c", "host", &["shared-host"]),
            node("shared-host", "physical-host", &[]),
        ]);
        let evidence = vec![
            evidence("ev-a", "checkout-api", "source-a", "domain-a"),
            evidence("ev-b", "checkout-api", "source-b", "domain-b"),
            evidence("ev-c", "checkout-api", "source-c", "domain-c"),
        ];
        let profiles = [
            (
                "domain-a".to_owned(),
                profile("domain-a", "process-a", 2, 30),
            ),
            (
                "domain-b".to_owned(),
                profile("domain-b", "process-b", 2, 30),
            ),
            (
                "domain-c".to_owned(),
                profile("domain-c", "process-c", 2, 30),
            ),
        ]
        .into_iter()
        .collect();

        let rows =
            evaluate_evidence_corroboration(&evidence, &profiles, &graph, 5, 10, policy()).unwrap();
        assert_eq!(
            rows[0].status,
            CandidateCorroborationStatus::SharedDeclaredDependency
        );
        assert_eq!(rows[0].no_declared_overlap_pair_count, 1);
        assert_eq!(rows[0].shared_dependency_pair_count, 2);
        assert_eq!(rows[0].pairwise.len(), 3);
    }

    #[test]
    fn profile_subject_substitution_is_rejected() {
        let graph = separated_graph();
        let evidence = vec![
            evidence("ev-1", "checkout-api", "metrics-a", "domain-a"),
            evidence("ev-2", "checkout-api", "logs-b", "domain-b"),
        ];
        let profiles = [
            (
                "domain-a".to_owned(),
                profile("domain-a", "process-a", 2, 30),
            ),
            (
                "domain-b".to_owned(),
                profile("wrong-subject", "process-b", 2, 30),
            ),
        ]
        .into_iter()
        .collect();

        assert_eq!(
            evaluate_evidence_corroboration(&evidence, &profiles, &graph, 5, 10, policy()),
            Err(CorroborationError::InvalidProfileBinding)
        );
    }

    #[test]
    fn duplicate_evidence_identity_is_rejected() {
        let graph = separated_graph();
        let item = evidence("ev-1", "checkout-api", "metrics-a", "domain-a");
        assert_eq!(
            evaluate_evidence_corroboration(
                &[item.clone(), item],
                &BTreeMap::new(),
                &graph,
                5,
                10,
                policy(),
            ),
            Err(CorroborationError::DuplicateEvidenceId)
        );
    }

    #[test]
    fn invalid_graph_is_not_converted_into_no_overlap() {
        let graph = graph(vec![
            node("process-a", "process", &["host"]),
            node("host", "host", &["process-a"]),
        ]);
        let evidence = vec![
            evidence("ev-1", "checkout-api", "metrics-a", "domain-a"),
            evidence("ev-2", "checkout-api", "logs-b", "domain-b"),
        ];
        let profiles = [
            (
                "domain-a".to_owned(),
                profile("domain-a", "process-a", 2, 30),
            ),
            ("domain-b".to_owned(), profile("domain-b", "host", 2, 30)),
        ]
        .into_iter()
        .collect();

        let rows =
            evaluate_evidence_corroboration(&evidence, &profiles, &graph, 5, 10, policy()).unwrap();
        assert_eq!(rows[0].status, CandidateCorroborationStatus::Invalid);
        assert_eq!(rows[0].invalid_pair_count, 1);
    }

    #[test]
    fn absent_and_unknown_evidence_are_not_credited_as_current() {
        let graph = separated_graph();
        let evidence = vec![
            SourceEvidence {
                evidence_id: "ev-absent".into(),
                object_ref: "checkout-api".into(),
                source_id: "metrics-a".into(),
                control_domain: "domain-a".into(),
                availability: EvidenceAvailability::Absent,
                event_tick: 1,
                observed_at_tick: 2,
                valid_until_tick: None,
            },
            SourceEvidence {
                evidence_id: "ev-unknown".into(),
                object_ref: "checkout-api".into(),
                source_id: "logs-b".into(),
                control_domain: "domain-b".into(),
                availability: EvidenceAvailability::Unknown,
                event_tick: 1,
                observed_at_tick: 2,
                valid_until_tick: None,
            },
        ];

        let rows =
            evaluate_evidence_corroboration(&evidence, &BTreeMap::new(), &graph, 5, 10, policy())
                .unwrap();
        assert_eq!(rows[0].current_evidence_count, 0);
        assert_eq!(
            rows[0].status,
            CandidateCorroborationStatus::NoCurrentEvidence
        );
    }

    #[test]
    fn candidates_and_pairs_are_emitted_in_deterministic_order() {
        let graph = separated_graph();
        let evidence = vec![
            evidence("z-1", "z-object", "source-z", "domain-b"),
            evidence("a-1", "a-object", "source-a", "domain-a"),
            evidence("z-2", "z-object", "source-b", "domain-a"),
        ];
        let profiles = [
            (
                "domain-a".to_owned(),
                profile("domain-a", "process-a", 2, 30),
            ),
            (
                "domain-b".to_owned(),
                profile("domain-b", "process-b", 2, 30),
            ),
        ]
        .into_iter()
        .collect();

        let rows =
            evaluate_evidence_corroboration(&evidence, &profiles, &graph, 5, 10, policy()).unwrap();
        assert_eq!(
            rows.iter()
                .map(|row| row.object_ref.as_str())
                .collect::<Vec<_>>(),
            vec!["a-object", "z-object"]
        );
        assert_eq!(rows[1].pairwise.len(), 1);
        assert_eq!(rows[1].pairwise[0].left_control_domain, "domain-a");
        assert_eq!(rows[1].pairwise[0].right_control_domain, "domain-b");
    }
}
