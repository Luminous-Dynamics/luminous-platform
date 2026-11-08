//! Structural failure-domain overlap audit for purported independent observers.
//!
//! Different control-domain labels do not establish independence: two processes
//! may share a host, kernel, clock, identity provider, KMS, network, scheduler,
//! telemetry pipeline, or storage system. This module compares declared
//! transitive dependency closures. A disjoint closure is only "no declared
//! overlap", never proof of operational or cryptographic independence.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InventoryState {
    Complete,
    Missing,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FailureDomainNode {
    pub id: String,
    pub class: String,
    /// Dependencies required for this node to operate or produce trusted output.
    pub depends_on: BTreeSet<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FailureDomainGraph {
    pub revision: String,
    pub nodes: BTreeMap<String, FailureDomainNode>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControlDomainProfile {
    pub subject_id: String,
    /// Digest of the canonical profile bytes supplied by the inventory owner.
    pub profile_digest: String,
    pub graph_revision: String,
    /// One or more roots representing the subject's execution/evidence path.
    pub roots: BTreeSet<String>,
    pub state: InventoryState,
    pub observed_at_tick: u64,
    /// Exclusive validity bound.
    pub valid_until_tick: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IndependenceStatus {
    /// The declared transitive failure-domain closures intersect.
    SharedDeclaredDependency,
    /// No declared dependency overlap was found. This is not proof of independence.
    NoDeclaredOverlap,
    /// Required inventories are missing, unknown, stale, expired, or incomparable.
    Indeterminate,
    /// The supplied graph/profile is malformed or contradictory.
    Invalid,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum IndependenceReason {
    CollectorProfileMissing,
    ObserverProfileMissing,
    CollectorProfileUnknown,
    ObserverProfileUnknown,
    CollectorProfileStale,
    ObserverProfileStale,
    CollectorProfileExpired,
    ObserverProfileExpired,
    CollectorProfileFromFuture,
    ObserverProfileFromFuture,
    CollectorProfileDigestInvalid,
    ObserverProfileDigestInvalid,
    GraphRevisionMismatch,
    MalformedGraph,
    DependencyCycle,
    CollectorRootMissing,
    ObserverRootMissing,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndependenceReport {
    pub status: IndependenceStatus,
    pub collector_subject_id: String,
    pub observer_subject_id: String,
    pub collector_closure_size: usize,
    pub observer_closure_size: usize,
    pub shared_dependencies: Vec<String>,
    pub shared_failure_domain_classes: Vec<String>,
    pub reasons: Vec<IndependenceReason>,
}

impl IndependenceReport {
    fn new(collector_subject_id: &str, observer_subject_id: &str) -> Self {
        Self {
            status: IndependenceStatus::Indeterminate,
            collector_subject_id: collector_subject_id.to_owned(),
            observer_subject_id: observer_subject_id.to_owned(),
            collector_closure_size: 0,
            observer_closure_size: 0,
            shared_dependencies: Vec::new(),
            shared_failure_domain_classes: Vec::new(),
            reasons: Vec::new(),
        }
    }

    fn add_reason(&mut self, reason: IndependenceReason) {
        if !self.reasons.contains(&reason) {
            self.reasons.push(reason);
            self.reasons.sort();
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IndependencePolicy {
    pub maximum_inventory_age_ticks: u64,
}

fn non_empty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn valid_sha256(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|hex| {
        hex.len() == 64
            && hex
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    })
}

fn validate_graph(graph: &FailureDomainGraph) -> Result<(), IndependenceReason> {
    if !non_empty(&graph.revision) || graph.nodes.is_empty() {
        return Err(IndependenceReason::MalformedGraph);
    }

    for (key, node) in &graph.nodes {
        if !non_empty(key)
            || key != &node.id
            || !non_empty(&node.class)
            || node.depends_on.contains(key)
            || node
                .depends_on
                .iter()
                .any(|dependency| !graph.nodes.contains_key(dependency))
        {
            return Err(IndependenceReason::MalformedGraph);
        }
    }

    // Kahn's algorithm detects cycles without recursion or stack growth.
    // Edges point from a component to its dependencies.
    let mut incoming = BTreeMap::<String, usize>::new();
    for id in graph.nodes.keys() {
        incoming.insert(id.clone(), 0);
    }
    for node in graph.nodes.values() {
        for dependency in &node.depends_on {
            let count = incoming
                .get_mut(dependency)
                .ok_or(IndependenceReason::MalformedGraph)?;
            *count = count.saturating_add(1);
        }
    }

    let mut ready: VecDeque<String> = incoming
        .iter()
        .filter_map(|(id, count)| (*count == 0).then_some(id.clone()))
        .collect();
    let mut visited = 0usize;
    while let Some(id) = ready.pop_front() {
        visited += 1;
        let node = graph
            .nodes
            .get(&id)
            .ok_or(IndependenceReason::MalformedGraph)?;
        for dependency in &node.depends_on {
            let count = incoming
                .get_mut(dependency)
                .ok_or(IndependenceReason::MalformedGraph)?;
            *count -= 1;
            if *count == 0 {
                ready.push_back(dependency.clone());
            }
        }
    }
    if visited != graph.nodes.len() {
        return Err(IndependenceReason::DependencyCycle);
    }
    Ok(())
}

fn profile_closure(
    profile: &ControlDomainProfile,
    graph: &FailureDomainGraph,
) -> Result<BTreeSet<String>, IndependenceReason> {
    if profile.roots.is_empty() {
        return Err(IndependenceReason::MalformedGraph);
    }

    let mut closure = BTreeSet::new();
    let mut pending: Vec<String> = profile.roots.iter().cloned().collect();
    while let Some(id) = pending.pop() {
        if !closure.insert(id.clone()) {
            continue;
        }
        let node = graph
            .nodes
            .get(&id)
            .ok_or(IndependenceReason::MalformedGraph)?;
        pending.extend(node.depends_on.iter().cloned());
    }
    Ok(closure)
}

fn validate_profile(
    profile: &ControlDomainProfile,
    graph: &FailureDomainGraph,
    audited_at_tick: u64,
    policy: IndependencePolicy,
    missing_reason: IndependenceReason,
    unknown_reason: IndependenceReason,
    stale_reason: IndependenceReason,
    expired_reason: IndependenceReason,
    future_reason: IndependenceReason,
    digest_reason: IndependenceReason,
) -> Result<(), IndependenceReason> {
    if !non_empty(&profile.subject_id) {
        return Err(IndependenceReason::MalformedGraph);
    }
    match profile.state {
        InventoryState::Complete => {}
        InventoryState::Missing => return Err(missing_reason),
        InventoryState::Unknown => return Err(unknown_reason),
    }
    if !valid_sha256(&profile.profile_digest) {
        return Err(digest_reason);
    }
    if profile.graph_revision != graph.revision {
        return Err(IndependenceReason::GraphRevisionMismatch);
    }
    if profile.observed_at_tick > audited_at_tick {
        return Err(future_reason);
    }
    if profile.valid_until_tick <= audited_at_tick {
        return Err(expired_reason);
    }
    if profile.valid_until_tick <= profile.observed_at_tick
        || audited_at_tick.saturating_sub(profile.observed_at_tick)
            > policy.maximum_inventory_age_ticks
    {
        return Err(stale_reason);
    }
    Ok(())
}

/// Compare the complete declared dependency closure for a collector and an
/// impact observer. A disjoint result means only that the supplied graph shows
/// no common dependency; it does not authenticate the inventory owner, prove
/// the graph complete, or rule out undeclared/latent common causes.
pub fn audit_observer_independence(
    collector: &ControlDomainProfile,
    observer: &ControlDomainProfile,
    graph: &FailureDomainGraph,
    audited_at_tick: u64,
    policy: IndependencePolicy,
) -> IndependenceReport {
    let mut report =
        IndependenceReport::new(&collector.subject_id, &observer.subject_id);

    if let Err(reason) = validate_graph(graph) {
        report.add_reason(reason);
        report.status = IndependenceStatus::Invalid;
        return report;
    }

    let collector_valid = validate_profile(
        collector,
        graph,
        audited_at_tick,
        policy,
        IndependenceReason::CollectorProfileMissing,
        IndependenceReason::CollectorProfileUnknown,
        IndependenceReason::CollectorProfileStale,
        IndependenceReason::CollectorProfileExpired,
        IndependenceReason::CollectorProfileFromFuture,
        IndependenceReason::CollectorProfileDigestInvalid,
    );
    let observer_valid = validate_profile(
        observer,
        graph,
        audited_at_tick,
        policy,
        IndependenceReason::ObserverProfileMissing,
        IndependenceReason::ObserverProfileUnknown,
        IndependenceReason::ObserverProfileStale,
        IndependenceReason::ObserverProfileExpired,
        IndependenceReason::ObserverProfileFromFuture,
        IndependenceReason::ObserverProfileDigestInvalid,
    );

    if let Err(reason) = collector_valid {
        report.add_reason(reason);
    }
    if let Err(reason) = observer_valid {
        report.add_reason(reason);
    }
    if !report.reasons.is_empty() {
        report.status = if report
            .reasons
            .iter()
            .any(|reason| matches!(reason, IndependenceReason::MalformedGraph | IndependenceReason::DependencyCycle))
        {
            IndependenceStatus::Invalid
        } else {
            IndependenceStatus::Indeterminate
        };
        return report;
    }

    let collector_closure = match profile_closure(collector, graph) {
        Ok(closure) => closure,
        Err(_) => {
            report.add_reason(IndependenceReason::CollectorRootMissing);
            report.status = IndependenceStatus::Invalid;
            return report;
        }
    };
    let observer_closure = match profile_closure(observer, graph) {
        Ok(closure) => closure,
        Err(_) => {
            report.add_reason(IndependenceReason::ObserverRootMissing);
            report.status = IndependenceStatus::Invalid;
            return report;
        }
    };

    report.collector_closure_size = collector_closure.len();
    report.observer_closure_size = observer_closure.len();
    report.shared_dependencies = collector_closure
        .intersection(&observer_closure)
        .cloned()
        .collect();
    report.shared_failure_domain_classes = report
        .shared_dependencies
        .iter()
        .filter_map(|id| graph.nodes.get(id).map(|node| node.class.clone()))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    report.status = if report.shared_dependencies.is_empty() {
        IndependenceStatus::NoDeclaredOverlap
    } else {
        IndependenceStatus::SharedDeclaredDependency
    };
    report
}

#[cfg(test)]
mod tests {
    use super::*;

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
            nodes: nodes.into_iter().map(|node| (node.id.clone(), node)).collect(),
        }
    }

    fn profile(
        subject: &str,
        roots: &[&str],
        observed_at_tick: u64,
        valid_until_tick: u64,
    ) -> ControlDomainProfile {
        ControlDomainProfile {
            subject_id: subject.into(),
            profile_digest: format!("sha256:{:064x}", if subject == "collector" { 1 } else { 2 }),
            graph_revision: "registry-v1".into(),
            roots: roots.iter().map(|value| (*value).into()).collect(),
            state: InventoryState::Complete,
            observed_at_tick,
            valid_until_tick,
        }
    }

    fn policy() -> IndependencePolicy {
        IndependencePolicy {
            maximum_inventory_age_ticks: 10,
        }
    }

    #[test]
    fn different_control_labels_sharing_a_host_are_not_independent() {
        let graph = graph(vec![
            node("collector-process", "process", &["collector-host"]),
            node("observer-process", "process", &["observer-host"]),
            node("collector-host", "host", &["shared-host"]),
            node("observer-host", "host", &["shared-host"]),
            node("shared-host", "physical-host", &[]),
        ]);
        let report = audit_observer_independence(
            &profile("collector", &["collector-process"], 2, 30),
            &profile("observer", &["observer-process"], 2, 30),
            &graph,
            5,
            policy(),
        );

        assert_eq!(
            report.status,
            IndependenceStatus::SharedDeclaredDependency
        );
        assert_eq!(report.shared_dependencies, vec!["shared-host"]);
        assert_eq!(report.shared_failure_domain_classes, vec!["physical-host"]);
    }

    #[test]
    fn separate_hosts_can_still_share_identity_clock_and_network_dependencies() {
        let graph = graph(vec![
            node(
                "collector",
                "process",
                &["collector-host", "shared-idp", "shared-clock"],
            ),
            node(
                "observer",
                "process",
                &["observer-host", "shared-idp", "shared-clock", "shared-network"],
            ),
            node("collector-host", "host", &[]),
            node("observer-host", "host", &[]),
            node("shared-idp", "identity-provider", &[]),
            node("shared-clock", "time-source", &[]),
            node("shared-network", "network-control-plane", &[]),
        ]);
        let report = audit_observer_independence(
            &profile("collector", &["collector"], 2, 30),
            &profile("observer", &["observer"], 2, 30),
            &graph,
            5,
            policy(),
        );

        assert_eq!(
            report.status,
            IndependenceStatus::SharedDeclaredDependency
        );
        assert_eq!(
            report.shared_dependencies,
            vec!["shared-clock", "shared-idp"]
        );
    }

    #[test]
    fn disjoint_declared_dependency_closures_are_only_a_candidate() {
        let graph = graph(vec![
            node("collector", "process", &["collector-host", "collector-clock"]),
            node("observer", "process", &["observer-host", "observer-clock"]),
            node("collector-host", "host", &[]),
            node("collector-clock", "clock", &[]),
            node("observer-host", "host", &[]),
            node("observer-clock", "clock", &[]),
        ]);
        let report = audit_observer_independence(
            &profile("collector", &["collector"], 2, 30),
            &profile("observer", &["observer"], 2, 30),
            &graph,
            5,
            policy(),
        );

        assert_eq!(report.status, IndependenceStatus::NoDeclaredOverlap);
        assert!(report.shared_dependencies.is_empty());
    }

    #[test]
    fn missing_inventory_is_indeterminate_not_independent() {
        let graph = graph(vec![
            node("collector", "process", &["collector-host"]),
            node("collector-host", "host", &[]),
            node("observer", "process", &["observer-host"]),
            node("observer-host", "host", &[]),
        ]);
        let mut collector = profile("collector", &["collector"], 2, 30);
        collector.state = InventoryState::Missing;
        let report =
            audit_observer_independence(&collector, &profile("observer", &["observer"], 2, 30), &graph, 5, policy());

        assert_eq!(report.status, IndependenceStatus::Indeterminate);
        assert_eq!(
            report.reasons,
            vec![IndependenceReason::CollectorProfileMissing]
        );
    }

    #[test]
    fn unknown_or_expired_inventory_is_indeterminate() {
        let graph = graph(vec![
            node("collector", "process", &["collector-host"]),
            node("collector-host", "host", &[]),
            node("observer", "process", &["observer-host"]),
            node("observer-host", "host", &[]),
        ]);
        let collector = profile("collector", &["collector"], 2, 30);
        let mut observer = profile("observer", &["observer"], 2, 5);
        observer.state = InventoryState::Unknown;
        let unknown =
            audit_observer_independence(&collector, &observer, &graph, 5, policy());
        assert_eq!(unknown.status, IndependenceStatus::Indeterminate);
        assert!(unknown
            .reasons
            .contains(&IndependenceReason::ObserverProfileUnknown));

        observer.state = InventoryState::Complete;
        let expired =
            audit_observer_independence(&collector, &observer, &graph, 5, policy());
        assert_eq!(expired.status, IndependenceStatus::Indeterminate);
        assert!(expired
            .reasons
            .contains(&IndependenceReason::ObserverProfileExpired));
    }

    #[test]
    fn mismatched_registry_revision_is_indeterminate() {
        let graph = graph(vec![
            node("collector", "process", &["collector-host"]),
            node("collector-host", "host", &[]),
            node("observer", "process", &["observer-host"]),
            node("observer-host", "host", &[]),
        ]);
        let collector = profile("collector", &["collector"], 2, 30);
        let mut observer = profile("observer", &["observer"], 2, 30);
        observer.graph_revision = "registry-v0".into();
        let report =
            audit_observer_independence(&collector, &observer, &graph, 5, policy());

        assert_eq!(report.status, IndependenceStatus::Indeterminate);
        assert!(report
            .reasons
            .contains(&IndependenceReason::GraphRevisionMismatch));
    }

    #[test]
    fn dependency_cycles_and_unknown_edges_are_invalid() {
        let cycle = graph(vec![
            node("collector", "process", &["host"]),
            node("host", "host", &["collector"]),
        ]);
        let good = profile("collector", &["collector"], 2, 30);
        let other = profile("observer", &["collector"], 2, 30);
        assert_eq!(
            audit_observer_independence(&good, &other, &cycle, 5, policy()).status,
            IndependenceStatus::Invalid
        );

        let unknown_edge = graph(vec![node("collector", "process", &["missing-host"])]);
        assert_eq!(
            audit_observer_independence(&good, &other, &unknown_edge, 5, policy()).status,
            IndependenceStatus::Invalid
        );
    }
}
