use crate::AuditError;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

const MAX_NODES: usize = 4096;
const MAX_ARCS: usize = 32768;
const MAX_SCENARIO_FAILURES: usize = 65536;
const POWER_BASIS: &str = "NET_ACTIVE_POWER_AT_CUSTOMER_POC";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphEvidenceStatus { Current, Missing, Expired, Invalid, Unverified }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapacityNodeType { Source, Junction, Bus, ConverterBoundary, ServiceSink }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapacityGraphNode {
    pub node_id: String,
    pub node_type: CapacityNodeType,
    pub component_id: String,
    pub configuration_sha256: String,
    pub evidence_status: GraphEvidenceStatus,
    pub failure_domain_ids: Vec<String>,
    pub evidence_reference: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapacityGraphArc {
    pub arc_id: String,
    pub from_node_id: String,
    pub to_node_id: String,
    /// Integer watts conservatively normalized at the declared customer POC.
    pub capacity_power_w_at_service_poc: u64,
    pub component_id: String,
    pub configuration_sha256: String,
    pub evidence_status: GraphEvidenceStatus,
    pub failure_domain_ids: Vec<String>,
    pub evidence_reference: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapacityFailureDomain { pub domain_id: String, pub description: String }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopologyCapacityGraph {
    pub graph_id: String,
    pub service_boundary_sha256: String,
    pub power_basis: String,
    pub unit: String,
    pub source_node_ids: Vec<String>,
    pub sink_node_id: String,
    pub nodes: Vec<CapacityGraphNode>,
    pub failure_domains: Vec<CapacityFailureDomain>,
    pub arcs: Vec<CapacityGraphArc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CapacityFailureScenario {
    pub scenario_id: String,
    pub failed_node_ids: Vec<String>,
    pub failed_component_ids: Vec<String>,
    pub failed_domain_ids: Vec<String>,
    pub failed_arc_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapacityGraphScenarioReport {
    pub scenario_id: String,
    pub max_flow_power_w: u64,
    /// Active-capacity arcs crossing the residual source/sink minimum cut.
    /// Their capacity sum is checked against max_flow_power_w.
    pub min_cut_witness_arc_ids: Vec<String>,
    /// Unavailable arcs are reported separately and are not counted as active cut capacity.
    pub blocked_arc_ids: Vec<String>,
    pub residual_source_side_node_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapacityGraphReport {
    pub graph_id: String,
    pub service_boundary_sha256: String,
    pub power_basis: String,
    pub unit: String,
    pub scenario: CapacityGraphScenarioReport,
    pub claim_ceiling: &'static str,
}

impl CapacityGraphReport {
    pub const fn claim_ceiling(&self) -> &'static str {
        "DECLARED_CAPACITY_GRAPH_MAX_FLOW_ONLY_NOT_ELECTRICAL_NETWORK_OR_RELIABILITY_QUALIFICATION"
    }
}

fn nonblank(value: &str, field: &str) -> Result<(), AuditError> {
    if value.trim().is_empty() {
        return Err(AuditError(format!("{field} must not be empty")));
    }
    Ok(())
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn unique_nonblank(values: &[String], field: &str) -> Result<(), AuditError> {
    let mut seen = BTreeSet::new();
    for value in values {
        nonblank(value, field)?;
        if !seen.insert(value.as_str()) {
            return Err(AuditError(format!("{field} contains duplicate identifier {value:?}")));
        }
    }
    Ok(())
}

pub fn validate_topology_capacity_graph(graph: &TopologyCapacityGraph) -> Result<(), AuditError> {
    nonblank(&graph.graph_id, "graph_id")?;
    if !valid_sha256(&graph.service_boundary_sha256) {
        return Err(AuditError("service boundary must be a valid SHA-256 digest".to_string()));
    }
    if graph.power_basis != POWER_BASIS || graph.unit != "W" {
        return Err(AuditError("graph must use NET_ACTIVE_POWER_AT_CUSTOMER_POC in integer watts".to_string()));
    }
    if graph.nodes.is_empty() || graph.nodes.len() > MAX_NODES {
        return Err(AuditError("node count is empty or exceeds the supported graph size".to_string()));
    }
    if graph.arcs.is_empty() || graph.arcs.len() > MAX_ARCS {
        return Err(AuditError("arc count is empty or exceeds the supported graph size".to_string()));
    }
    if graph.source_node_ids.is_empty() {
        return Err(AuditError("at least one source node is required".to_string()));
    }
    unique_nonblank(&graph.source_node_ids, "source_node_ids")?;
    nonblank(&graph.sink_node_id, "sink_node_id")?;

    let mut nodes = BTreeMap::new();
    for node in &graph.nodes {
        nonblank(&node.node_id, "node_id")?;
        nonblank(&node.component_id, "node.component_id")?;
        nonblank(&node.evidence_reference, "node.evidence_reference")?;
        if !valid_sha256(&node.configuration_sha256) {
            return Err(AuditError(format!("node {:?} has an invalid configuration digest", node.node_id)));
        }
        unique_nonblank(&node.failure_domain_ids, "node.failure_domain_ids")?;
        if nodes.insert(node.node_id.as_str(), node).is_some() {
            return Err(AuditError(format!("duplicate node ID {:?}", node.node_id)));
        }
    }

    if graph.failure_domains.len() > MAX_SCENARIO_FAILURES {
        return Err(AuditError("failure-domain count exceeds the supported limit".to_string()));
    }
    let mut domains = BTreeSet::new();
    for domain in &graph.failure_domains {
        nonblank(&domain.domain_id, "failure_domain_id")?;
        nonblank(&domain.description, "failure_domain.description")?;
        if !domains.insert(domain.domain_id.as_str()) {
            return Err(AuditError(format!("duplicate failure domain {:?}", domain.domain_id)));
        }
    }
    for node in &graph.nodes {
        for domain in &node.failure_domain_ids {
            if !domains.contains(domain.as_str()) {
                return Err(AuditError(format!("node {:?} references unknown failure domain {:?}", node.node_id, domain)));
            }
        }
    }

    let sink = nodes.get(graph.sink_node_id.as_str())
        .ok_or_else(|| AuditError("sink node does not exist".to_string()))?;
    if sink.node_type != CapacityNodeType::ServiceSink {
        return Err(AuditError("sink_node_id must identify a service-sink node".to_string()));
    }
    if graph.nodes.iter().filter(|node| node.node_type == CapacityNodeType::ServiceSink).count() != 1 {
        return Err(AuditError("v0.1 requires exactly one service sink".to_string()));
    }
    for source_id in &graph.source_node_ids {
        let source = nodes.get(source_id.as_str())
            .ok_or_else(|| AuditError(format!("source node {source_id:?} does not exist")))?;
        if source.node_type != CapacityNodeType::Source {
            return Err(AuditError(format!("declared source {source_id:?} is not a source node")));
        }
    }
    for node in &graph.nodes {
        if node.node_type == CapacityNodeType::Source
            && !graph.source_node_ids.iter().any(|id| id == &node.node_id) {
            return Err(AuditError(format!("source node {:?} is not declared in source_node_ids", node.node_id)));
        }
    }

    let mut arc_ids = BTreeSet::new();
    for arc in &graph.arcs {
        nonblank(&arc.arc_id, "arc_id")?;
        nonblank(&arc.component_id, "arc.component_id")?;
        nonblank(&arc.evidence_reference, "arc.evidence_reference")?;
        if !valid_sha256(&arc.configuration_sha256) {
            return Err(AuditError(format!("arc {:?} has an invalid configuration digest", arc.arc_id)));
        }
        if arc.from_node_id == arc.to_node_id {
            return Err(AuditError(format!("self-loop arc {:?} is not permitted in v0.1", arc.arc_id)));
        }
        if !nodes.contains_key(arc.from_node_id.as_str()) || !nodes.contains_key(arc.to_node_id.as_str()) {
            return Err(AuditError(format!("arc {:?} references an unknown endpoint", arc.arc_id)));
        }
        unique_nonblank(&arc.failure_domain_ids, "arc.failure_domain_ids")?;
        for domain in &arc.failure_domain_ids {
            if !domains.contains(domain.as_str()) {
                return Err(AuditError(format!("arc {:?} references unknown failure domain {:?}", arc.arc_id, domain)));
            }
        }
        if !arc_ids.insert(arc.arc_id.as_str()) {
            return Err(AuditError(format!("duplicate arc ID {:?}", arc.arc_id)));
        }
    }
    Ok(())
}

fn validate_scenario(graph: &TopologyCapacityGraph, scenario: &CapacityFailureScenario) -> Result<(), AuditError> {
    nonblank(&scenario.scenario_id, "scenario_id")?;
    let count = scenario.failed_node_ids.len()
        .checked_add(scenario.failed_component_ids.len())
        .and_then(|n| n.checked_add(scenario.failed_domain_ids.len()))
        .and_then(|n| n.checked_add(scenario.failed_arc_ids.len()))
        .ok_or_else(|| AuditError("scenario failure count overflowed".to_string()))?;
    if count > MAX_SCENARIO_FAILURES {
        return Err(AuditError("scenario exceeds the supported failure count".to_string()));
    }
    unique_nonblank(&scenario.failed_node_ids, "failed_node_ids")?;
    unique_nonblank(&scenario.failed_component_ids, "failed_component_ids")?;
    unique_nonblank(&scenario.failed_domain_ids, "failed_domain_ids")?;
    unique_nonblank(&scenario.failed_arc_ids, "failed_arc_ids")?;

    let node_ids: BTreeSet<&str> = graph.nodes.iter().map(|node| node.node_id.as_str()).collect();
    let component_ids: BTreeSet<&str> = graph.nodes.iter().map(|node| node.component_id.as_str())
        .chain(graph.arcs.iter().map(|arc| arc.component_id.as_str())).collect();
    let domain_ids: BTreeSet<&str> = graph.failure_domains.iter().map(|domain| domain.domain_id.as_str()).collect();
    let arc_ids: BTreeSet<&str> = graph.arcs.iter().map(|arc| arc.arc_id.as_str()).collect();

    for id in &scenario.failed_node_ids {
        if !node_ids.contains(id.as_str()) { return Err(AuditError(format!("unknown failed node {id:?}"))); }
    }
    for id in &scenario.failed_component_ids {
        if !component_ids.contains(id.as_str()) { return Err(AuditError(format!("unknown failed component {id:?}"))); }
    }
    for id in &scenario.failed_domain_ids {
        if !domain_ids.contains(id.as_str()) { return Err(AuditError(format!("unknown failed domain {id:?}"))); }
    }
    for id in &scenario.failed_arc_ids {
        if !arc_ids.contains(id.as_str()) { return Err(AuditError(format!("unknown failed arc {id:?}"))); }
    }
    Ok(())
}

fn node_is_available(node: &CapacityGraphNode, scenario: &CapacityFailureScenario) -> bool {
    node.evidence_status == GraphEvidenceStatus::Current
        && !scenario.failed_node_ids.iter().any(|id| id == &node.node_id)
        && !scenario.failed_component_ids.iter().any(|id| id == &node.component_id)
        && !node.failure_domain_ids.iter().any(|domain| scenario.failed_domain_ids.iter().any(|failed| failed == domain))
}

fn arc_is_available(
    arc: &CapacityGraphArc,
    from_node: &CapacityGraphNode,
    to_node: &CapacityGraphNode,
    scenario: &CapacityFailureScenario,
) -> bool {
    arc.evidence_status == GraphEvidenceStatus::Current
        && node_is_available(from_node, scenario)
        && node_is_available(to_node, scenario)
        && !scenario.failed_arc_ids.iter().any(|id| id == &arc.arc_id)
        && !scenario.failed_component_ids.iter().any(|id| id == &arc.component_id)
        && !arc.failure_domain_ids.iter().any(|domain| scenario.failed_domain_ids.iter().any(|failed| failed == domain))
}

#[derive(Debug, Clone, Copy)]
struct ResidualEdge { to: usize, reverse_index: usize, residual_capacity: u64 }

#[derive(Debug)]
struct Dinic { adjacency: Vec<Vec<ResidualEdge>> }

impl Dinic {
    fn new(vertex_count: usize) -> Self { Self { adjacency: vec![Vec::new(); vertex_count] } }

    fn add_edge(&mut self, from: usize, to: usize, capacity: u64) {
        let forward_index = self.adjacency[from].len();
        let reverse_index = self.adjacency[to].len();
        self.adjacency[from].push(ResidualEdge { to, reverse_index, residual_capacity: capacity });
        self.adjacency[to].push(ResidualEdge { to: from, reverse_index: forward_index, residual_capacity: 0 });
    }

    fn build_levels(&self, source: usize, sink: usize) -> Option<Vec<i32>> {
        let mut levels = vec![-1_i32; self.adjacency.len()];
        let mut queue = VecDeque::new();
        levels[source] = 0;
        queue.push_back(source);
        while let Some(vertex) = queue.pop_front() {
            for edge in &self.adjacency[vertex] {
                if edge.residual_capacity > 0 && levels[edge.to] < 0 {
                    levels[edge.to] = levels[vertex] + 1;
                    queue.push_back(edge.to);
                }
            }
        }
        (levels[sink] >= 0).then_some(levels)
    }

    fn send_flow(&mut self, vertex: usize, sink: usize, limit: u64, levels: &[i32], next_edge: &mut [usize]) -> u64 {
        if vertex == sink || limit == 0 { return limit; }
        while next_edge[vertex] < self.adjacency[vertex].len() {
            let edge_index = next_edge[vertex];
            let edge = self.adjacency[vertex][edge_index];
            if edge.residual_capacity > 0 && levels[edge.to] == levels[vertex] + 1 {
                let sent = self.send_flow(edge.to, sink, limit.min(edge.residual_capacity), levels, next_edge);
                if sent > 0 {
                    self.adjacency[vertex][edge_index].residual_capacity -= sent;
                    self.adjacency[edge.to][edge.reverse_index].residual_capacity += sent;
                    return sent;
                }
            }
            next_edge[vertex] += 1;
        }
        0
    }

    fn max_flow(&mut self, source: usize, sink: usize) -> Result<u64, AuditError> {
        let mut total = 0_u64;
        while let Some(levels) = self.build_levels(source, sink) {
            let mut next_edge = vec![0_usize; self.adjacency.len()];
            loop {
                let sent = self.send_flow(source, sink, u64::MAX, &levels, &mut next_edge);
                if sent == 0 { break; }
                total = total.checked_add(sent).ok_or_else(|| AuditError("max-flow sum overflowed u64".to_string()))?;
            }
        }
        Ok(total)
    }

    fn residual_reachable(&self, source: usize) -> Vec<bool> {
        let mut reachable = vec![false; self.adjacency.len()];
        let mut queue = VecDeque::new();
        reachable[source] = true;
        queue.push_back(source);
        while let Some(vertex) = queue.pop_front() {
            for edge in &self.adjacency[vertex] {
                if edge.residual_capacity > 0 && !reachable[edge.to] {
                    reachable[edge.to] = true;
                    queue.push_back(edge.to);
                }
            }
        }
        reachable
    }
}

pub fn evaluate_topology_capacity_graph(
    graph: &TopologyCapacityGraph,
    scenario: &CapacityFailureScenario,
) -> Result<CapacityGraphReport, AuditError> {
    validate_topology_capacity_graph(graph)?;
    validate_scenario(graph, scenario)?;

    let mut nodes: Vec<&CapacityGraphNode> = graph.nodes.iter().collect();
    nodes.sort_by(|a, b| a.node_id.cmp(&b.node_id));
    let indices: BTreeMap<&str, usize> = nodes.iter().enumerate().map(|(i, n)| (n.node_id.as_str(), i)).collect();
    let mut arcs: Vec<&CapacityGraphArc> = graph.arcs.iter().collect();
    arcs.sort_by(|a, b| a.arc_id.cmp(&b.arc_id));

    let super_source = nodes.len();
    let sink = *indices.get(graph.sink_node_id.as_str()).ok_or_else(|| AuditError("validated sink lookup failed".to_string()))?;
    let mut total_capacity = 0_u64;
    for arc in &arcs {
        if arc.evidence_status == GraphEvidenceStatus::Current {
            total_capacity = total_capacity.checked_add(arc.capacity_power_w_at_service_poc)
                .ok_or_else(|| AuditError("sum of graph capacities overflowed u64".to_string()))?;
        }
    }

    let mut network = Dinic::new(nodes.len() + 1);
    let mut ordered_source_ids = graph.source_node_ids.clone();
    ordered_source_ids.sort();
    for source_id in &ordered_source_ids {
        let i = *indices.get(source_id.as_str()).ok_or_else(|| AuditError("validated source lookup failed".to_string()))?;
        if node_is_available(nodes[i], scenario) && i != sink {
            // This connector must never become the reported minimum cut. The
            // checked sum of real arc capacities above bounds total possible flow.
            network.add_edge(super_source, i, u64::MAX);
        }
    }

    let mut active_arcs = Vec::new();
    let mut blocked_arc_ids = Vec::new();
    for (arc_index, arc) in arcs.iter().enumerate() {
        let from = *indices.get(arc.from_node_id.as_str()).ok_or_else(|| AuditError("validated from-node lookup failed".to_string()))?;
        let to = *indices.get(arc.to_node_id.as_str()).ok_or_else(|| AuditError("validated to-node lookup failed".to_string()))?;
        if arc_is_available(arc, nodes[from], nodes[to], scenario) {
            if arc.capacity_power_w_at_service_poc > 0 {
                network.add_edge(from, to, arc.capacity_power_w_at_service_poc);
                active_arcs.push((arc_index, from, to));
            }
        } else {
            blocked_arc_ids.push(arc.arc_id.clone());
        }
    }

    let flow = network.max_flow(super_source, sink)?;
    let residual = network.residual_reachable(super_source);
    let mut min_cut_witness_arc_ids = Vec::new();
    for (arc_index, from, to) in active_arcs {
        if residual[from] && !residual[to] {
            min_cut_witness_arc_ids.push(arcs[arc_index].arc_id.clone());
        }
    }
    min_cut_witness_arc_ids.sort();
    blocked_arc_ids.sort();

    let mut residual_source_side_node_ids: Vec<String> = nodes.iter().enumerate()
        .filter(|(i, _)| residual[*i])
        .map(|(_, node)| node.node_id.clone())
        .collect();
    residual_source_side_node_ids.sort();

    let cut_capacity = min_cut_witness_arc_ids.iter().try_fold(0_u64, |sum, id| {
        let arc = arcs.iter().find(|arc| arc.arc_id == *id)
            .ok_or_else(|| AuditError("min-cut witness lookup failed".to_string()))?;
        sum.checked_add(arc.capacity_power_w_at_service_poc)
            .ok_or_else(|| AuditError("min-cut capacity overflowed u64".to_string()))
    })?;
    if cut_capacity != flow {
        return Err(AuditError(format!("max-flow/min-cut mismatch: flow {flow} W, cut {cut_capacity} W")));
    }

    Ok(CapacityGraphReport {
        graph_id: graph.graph_id.clone(),
        service_boundary_sha256: graph.service_boundary_sha256.clone(),
        power_basis: graph.power_basis.clone(),
        unit: graph.unit.clone(),
        scenario: CapacityGraphScenarioReport {
            scenario_id: scenario.scenario_id.clone(),
            max_flow_power_w: flow,
            min_cut_witness_arc_ids,
            blocked_arc_ids,
            residual_source_side_node_ids,
        },
        claim_ceiling: "DECLARED_CAPACITY_GRAPH_MAX_FLOW_ONLY_NOT_ELECTRICAL_NETWORK_OR_RELIABILITY_QUALIFICATION",
    })
}


/// Evaluate baseline, each unique component/domain single failure, and caller-declared
/// compound scenarios. Reports are returned in stable scenario-ID order.
pub fn evaluate_topology_capacity_n1_suite(
    graph: &TopologyCapacityGraph,
    additional_scenarios: &[CapacityFailureScenario],
) -> Result<Vec<CapacityGraphReport>, AuditError> {
    validate_topology_capacity_graph(graph)?;
    let mut scenarios = BTreeMap::<String, CapacityFailureScenario>::new();
    scenarios.insert("baseline".to_string(), CapacityFailureScenario {
        scenario_id: "baseline".to_string(),
        ..CapacityFailureScenario::default()
    });

    let mut component_ids = BTreeSet::new();
    for node in &graph.nodes { component_ids.insert(node.component_id.clone()); }
    for arc in &graph.arcs { component_ids.insert(arc.component_id.clone()); }
    for id in component_ids {
        let scenario_id = format!("component:{id}");
        scenarios.insert(scenario_id.clone(), CapacityFailureScenario {
            scenario_id,
            failed_component_ids: vec![id],
            ..CapacityFailureScenario::default()
        });
    }
    for domain in &graph.failure_domains {
        let scenario_id = format!("domain:{}", domain.domain_id);
        scenarios.insert(scenario_id.clone(), CapacityFailureScenario {
            scenario_id,
            failed_domain_ids: vec![domain.domain_id.clone()],
            ..CapacityFailureScenario::default()
        });
    }
    for scenario in additional_scenarios {
        if scenario.failed_node_ids.is_empty() && scenario.failed_component_ids.is_empty()
            && scenario.failed_domain_ids.is_empty() && scenario.failed_arc_ids.is_empty() {
            return Err(AuditError(format!(
                "additional scenario {:?} must declare at least one failure", scenario.scenario_id
            )));
        }
        if scenarios.contains_key(&scenario.scenario_id) {
            return Err(AuditError(format!("duplicate or reserved scenario ID {:?}", scenario.scenario_id)));
        }
        scenarios.insert(scenario.scenario_id.clone(), scenario.clone());
    }
    if scenarios.len() > MAX_SCENARIO_FAILURES {
        return Err(AuditError("scenario suite exceeds the supported scenario count".to_string()));
    }
    scenarios.values()
        .map(|scenario| evaluate_topology_capacity_graph(graph, scenario))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(ch: char) -> String { std::iter::repeat(ch).take(64).collect() }

    fn node(id: &str, kind: CapacityNodeType, component: &str, domains: &[&str]) -> CapacityGraphNode {
        CapacityGraphNode {
            node_id: id.into(), node_type: kind, component_id: component.into(),
            configuration_sha256: digest('a'), evidence_status: GraphEvidenceStatus::Current,
            failure_domain_ids: domains.iter().map(|x| x.to_string()).collect(),
            evidence_reference: format!("synthetic://{id}"),
        }
    }

    fn arc(id: &str, from: &str, to: &str, capacity: u64, component: &str, domains: &[&str]) -> CapacityGraphArc {
        CapacityGraphArc {
            arc_id: id.into(), from_node_id: from.into(), to_node_id: to.into(),
            capacity_power_w_at_service_poc: capacity, component_id: component.into(),
            configuration_sha256: digest('b'), evidence_status: GraphEvidenceStatus::Current,
            failure_domain_ids: domains.iter().map(|x| x.to_string()).collect(),
            evidence_reference: format!("synthetic://{id}"),
        }
    }

    fn graph() -> TopologyCapacityGraph {
        TopologyCapacityGraph {
            graph_id: "PF-CAPACITY-GRAPH-OVERLAP-001".into(),
            service_boundary_sha256: digest('1'),
            power_basis: "NET_ACTIVE_POWER_AT_CUSTOMER_POC".into(), unit: "W".into(),
            source_node_ids: vec!["source_a".into(), "source_b".into(), "source_c".into()],
            sink_node_id: "customer_poc".into(),
            nodes: vec![
                node("source_a", CapacityNodeType::Source, "source-A", &[]),
                node("source_b", CapacityNodeType::Source, "source-B", &[]),
                node("source_c", CapacityNodeType::Source, "source-C", &[]),
                node("junction", CapacityNodeType::Junction, "junction-1", &[]),
                node("feeder_out", CapacityNodeType::Bus, "feeder-1", &[]),
                node("customer_poc", CapacityNodeType::ServiceSink, "poc-1", &["service-bus"]),
            ],
            failure_domains: vec![
                CapacityFailureDomain { domain_id: "cooling-ab".into(), description: "Cooling shared by A/B".into() },
                CapacityFailureDomain { domain_id: "control-bc".into(), description: "Control power shared by B/C".into() },
                CapacityFailureDomain { domain_id: "common-feeder".into(), description: "One upstream feeder".into() },
                CapacityFailureDomain { domain_id: "service-bus".into(), description: "One downstream service bus".into() },
            ],
            arcs: vec![
                arc("arc-source-a", "source_a", "junction", 80, "converter-A", &["cooling-ab"]),
                arc("arc-source-b", "source_b", "junction", 70, "converter-B", &["cooling-ab", "control-bc"]),
                arc("arc-source-c", "source_c", "junction", 60, "converter-C", &["control-bc"]),
                arc("arc-shared-feeder", "junction", "feeder_out", 170, "shared-feeder-1", &["common-feeder"]),
                arc("arc-service-bus", "feeder_out", "customer_poc", 160, "service-bus-1", &["service-bus"]),
            ],
        }
    }

    fn scenario(id: &str) -> CapacityFailureScenario {
        CapacityFailureScenario { scenario_id: id.into(), ..CapacityFailureScenario::default() }
    }

    fn exhaustive_min_cut(graph: &TopologyCapacityGraph, scenario: &CapacityFailureScenario) -> u64 {
        let source_ids: BTreeSet<String> = graph.source_node_ids.iter().cloned().collect();
        let mut optional: Vec<String> = graph.nodes.iter()
            .filter(|node| node.node_id != graph.sink_node_id && !source_ids.contains(&node.node_id))
            .map(|node| node.node_id.clone())
            .collect();
        optional.sort();
        assert!(optional.len() <= 16);
        let combinations = 1_usize << optional.len();
        let mut best = u64::MAX;
        for mask in 0..combinations {
            let mut source_side = source_ids.clone();
            for (index, node_id) in optional.iter().enumerate() {
                if mask & (1_usize << index) != 0 { source_side.insert(node_id.clone()); }
            }
            assert!(!source_side.contains(&graph.sink_node_id));
            let mut cut = 0_u64;
            for arc in &graph.arcs {
                let from = graph.nodes.iter().find(|node| node.node_id == arc.from_node_id).unwrap();
                let to = graph.nodes.iter().find(|node| node.node_id == arc.to_node_id).unwrap();
                if source_side.contains(&arc.from_node_id)
                    && !source_side.contains(&arc.to_node_id)
                    && arc_is_available(arc, from, to, scenario) {
                    cut = cut.checked_add(arc.capacity_power_w_at_service_poc).unwrap();
                }
            }
            best = best.min(cut);
        }
        best
    }

    #[test]
    fn baseline_flow_is_limited_by_customer_service_bus() {
        let r = evaluate_topology_capacity_graph(&graph(), &scenario("baseline")).unwrap();
        assert_eq!(r.scenario.max_flow_power_w, 160);
        assert_eq!(r.scenario.min_cut_witness_arc_ids, vec!["arc-service-bus".to_string()]);
        assert!(r.scenario.blocked_arc_ids.is_empty());
    }

    #[test]
    fn overlapping_cooling_failure_removes_a_and_b() {
        let mut s = scenario("cooling-ab"); s.failed_domain_ids.push("cooling-ab".into());
        let r = evaluate_topology_capacity_graph(&graph(), &s).unwrap();
        assert_eq!(r.scenario.max_flow_power_w, 60);
        assert_eq!(r.scenario.min_cut_witness_arc_ids, vec!["arc-source-c".to_string()]);
        assert_eq!(r.scenario.blocked_arc_ids, vec!["arc-source-a".to_string(), "arc-source-b".to_string()]);
    }

    #[test]
    fn overlapping_control_failure_removes_b_and_c() {
        let mut s = scenario("control-bc"); s.failed_domain_ids.push("control-bc".into());
        let r = evaluate_topology_capacity_graph(&graph(), &s).unwrap();
        assert_eq!(r.scenario.max_flow_power_w, 80);
        assert_eq!(r.scenario.min_cut_witness_arc_ids, vec!["arc-source-a".to_string()]);
    }

    #[test]
    fn source_component_failure_removes_only_its_route() {
        let mut s = scenario("source-a"); s.failed_component_ids.push("source-A".into());
        let r = evaluate_topology_capacity_graph(&graph(), &s).unwrap();
        assert_eq!(r.scenario.max_flow_power_w, 130);
        assert_eq!(r.scenario.min_cut_witness_arc_ids, vec!["arc-source-b".to_string(), "arc-source-c".to_string()]);
    }

    #[test]
    fn shared_feeder_failure_withdraws_all_paths() {
        let mut s = scenario("feeder"); s.failed_domain_ids.push("common-feeder".into());
        let r = evaluate_topology_capacity_graph(&graph(), &s).unwrap();
        assert_eq!(r.scenario.max_flow_power_w, 0);
        assert_eq!(r.scenario.blocked_arc_ids, vec!["arc-shared-feeder".to_string()]);
        assert!(r.scenario.min_cut_witness_arc_ids.is_empty());
    }

    #[test]
    fn service_bus_failure_withdraws_all_deliverable_power() {
        let mut s = scenario("service-bus"); s.failed_domain_ids.push("service-bus".into());
        let r = evaluate_topology_capacity_graph(&graph(), &s).unwrap();
        assert_eq!(r.scenario.max_flow_power_w, 0);
        assert_eq!(r.scenario.blocked_arc_ids, vec!["arc-service-bus".to_string()]);
    }

    #[test]
    fn unverified_arc_is_excluded_and_retained_as_a_blocker() {
        let mut g = graph();
        g.arcs.iter_mut().find(|a| a.arc_id == "arc-source-c").unwrap().evidence_status = GraphEvidenceStatus::Unverified;
        let r = evaluate_topology_capacity_graph(&g, &scenario("unverified-source-c")).unwrap();
        assert_eq!(r.scenario.max_flow_power_w, 150);
        assert_eq!(r.scenario.min_cut_witness_arc_ids, vec!["arc-source-a".to_string(), "arc-source-b".to_string()]);
        assert_eq!(r.scenario.blocked_arc_ids, vec!["arc-source-c".to_string()]);
    }

    #[test]
    fn rejects_unknown_failure_domain() {
        let mut s = scenario("unknown-domain"); s.failed_domain_ids.push("not-declared".into());
        assert!(evaluate_topology_capacity_graph(&graph(), &s).is_err());
    }

    #[test]
    fn rejects_mismatched_power_basis() {
        let mut g = graph(); g.power_basis = "BATTERY_SIDE_DC_POWER".into();
        assert!(evaluate_topology_capacity_graph(&g, &scenario("bad-basis")).is_err());
    }

    #[test]
    fn rejects_unknown_arc_endpoint() {
        let mut g = graph(); g.arcs[0].to_node_id = "missing-node".into();
        assert!(evaluate_topology_capacity_graph(&g, &scenario("bad-endpoint")).is_err());
    }

    #[test]
    fn rejects_duplicate_arc_ids() {
        let mut g = graph(); g.arcs[1].arc_id = g.arcs[0].arc_id.clone();
        assert!(evaluate_topology_capacity_graph(&g, &scenario("duplicate-arc")).is_err());
    }

    #[test]
    fn rejects_invalid_configuration_digest() {
        let mut g = graph(); g.arcs[0].configuration_sha256 = "bad".into();
        assert!(evaluate_topology_capacity_graph(&g, &scenario("bad-digest")).is_err());
    }

    #[test]
    fn rejects_arc_with_unknown_failure_domain() {
        let mut g = graph(); g.arcs[0].failure_domain_ids.push("undeclared".into());
        assert!(evaluate_topology_capacity_graph(&g, &scenario("bad-arc-domain")).is_err());
    }

    #[test]
    fn rejects_unknown_failed_component() {
        let mut s = scenario("unknown-component"); s.failed_component_ids.push("not-a-component".into());
        assert!(evaluate_topology_capacity_graph(&graph(), &s).is_err());
    }

    #[test]
    fn rejects_undeclared_source_node() {
        let mut g = graph(); g.nodes.push(node("orphan", CapacityNodeType::Source, "orphan", &[]));
        assert!(evaluate_topology_capacity_graph(&g, &scenario("orphan-source")).is_err());
    }

    #[test]
    fn result_is_deterministic_under_node_arc_and_source_permutation() {
        let g1 = graph(); let mut g2 = graph();
        g2.nodes.reverse(); g2.arcs.reverse(); g2.source_node_ids.reverse();
        let a = evaluate_topology_capacity_graph(&g1, &scenario("permutation")).unwrap();
        let b = evaluate_topology_capacity_graph(&g2, &scenario("permutation")).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn min_cut_capacity_matches_flow_with_parallel_and_directed_edges() {
        let mut g = graph();
        g.arcs.push(arc("arc-detour", "source_a", "junction", 9, "detour", &[]));
        let r = evaluate_topology_capacity_graph(&g, &scenario("detour")).unwrap();
        let cut: u64 = r.scenario.min_cut_witness_arc_ids.iter()
            .map(|id| g.arcs.iter().find(|a| &a.arc_id == id).unwrap().capacity_power_w_at_service_poc)
            .sum();
        assert_eq!(cut, r.scenario.max_flow_power_w);
        assert_eq!(r.scenario.max_flow_power_w, 160);
    }

    #[test]
    fn exhaustive_cut_enumerator_matches_flow_for_all_declared_failure_examples() {
        let g = graph();
        let mut cases = vec![scenario("baseline")];
        let mut cooling = scenario("cooling"); cooling.failed_domain_ids.push("cooling-ab".into()); cases.push(cooling);
        let mut control = scenario("control"); control.failed_domain_ids.push("control-bc".into()); cases.push(control);
        let mut source = scenario("source-a"); source.failed_component_ids.push("source-A".into()); cases.push(source);
        let mut feeder = scenario("feeder"); feeder.failed_domain_ids.push("common-feeder".into()); cases.push(feeder);
        let mut bus = scenario("bus"); bus.failed_domain_ids.push("service-bus".into()); cases.push(bus);
        for case in cases {
            let report = evaluate_topology_capacity_graph(&g, &case).unwrap();
            assert_eq!(report.scenario.max_flow_power_w, exhaustive_min_cut(&g, &case));
        }
    }

    #[test]
    fn n1_suite_generates_components_domains_and_keeps_compound_cases() {
        let compound = CapacityFailureScenario {
            scenario_id: "double-fault-a-b".into(),
            failed_component_ids: vec!["source-A".into(), "source-B".into()],
            ..CapacityFailureScenario::default()
        };
        let reports = evaluate_topology_capacity_n1_suite(&graph(), &[compound]).unwrap();
        let ids: BTreeSet<String> = reports.iter().map(|r| r.scenario.scenario_id.clone()).collect();
        assert!(ids.contains("baseline"));
        assert!(ids.contains("component:source-A"));
        assert!(ids.contains("component:converter-A"));
        assert!(ids.contains("domain:cooling-ab"));
        assert!(ids.contains("domain:control-bc"));
        assert!(ids.contains("double-fault-a-b"));
    }

    #[test]
    fn n1_suite_rejects_empty_and_duplicate_scenarios() {
        assert!(evaluate_topology_capacity_n1_suite(&graph(), &[scenario("empty")]).is_err());
        assert!(evaluate_topology_capacity_n1_suite(&graph(), &[scenario("baseline")]).is_err());
    }


    #[test]
    fn single_source_single_edge_cut_witness_excludes_super_source_connector() {
        let g = TopologyCapacityGraph {
            graph_id: "single-edge".into(),
            service_boundary_sha256: digest('1'),
            power_basis: "NET_ACTIVE_POWER_AT_CUSTOMER_POC".into(),
            unit: "W".into(),
            source_node_ids: vec!["source".into()],
            sink_node_id: "sink".into(),
            nodes: vec![
                node("source", CapacityNodeType::Source, "source-component", &[]),
                node("sink", CapacityNodeType::ServiceSink, "sink-component", &[]),
            ],
            failure_domains: vec![],
            arcs: vec![arc("only-arc", "source", "sink", 100, "only-converter", &[])],
        };
        let report = evaluate_topology_capacity_graph(&g, &scenario("single-edge")).unwrap();
        assert_eq!(report.scenario.max_flow_power_w, 100);
        assert_eq!(report.scenario.min_cut_witness_arc_ids, vec!["only-arc".to_string()]);
    }

}
