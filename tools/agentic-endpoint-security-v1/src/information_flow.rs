#![forbid(unsafe_code)]
//! Typed, dependency-free reference model for the information-flow lifecycle contract.
//!
//! This module evaluates normalized synthetic facts. It does not authenticate
//! connector receipts, inspect real model internals, observe real sinks, or
//! enforce a production data-loss-prevention policy.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Sensitivity {
    Public,
    Internal,
    Confidential,
    Secret,
    Unknown,
}

impl Sensitivity {
    fn rank(self) -> u8 {
        match self {
            Self::Public => 0,
            Self::Internal => 1,
            Self::Confidential => 2,
            Self::Secret => 3,
            Self::Unknown => 4,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Integrity {
    AuthenticatedTask,
    AuthenticatedConnector,
    Derived,
    UntrustedContent,
    Unknown,
}

impl Integrity {
    fn rank(self) -> u8 {
        match self {
            Self::AuthenticatedTask => 0,
            Self::AuthenticatedConnector => 1,
            Self::Derived => 2,
            Self::UntrustedContent => 3,
            Self::Unknown => 4,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceClass {
    UserTask,
    AllowlistedConnector,
    ExternalUntrusted,
    Memory,
    AgentGenerated,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceRecord {
    pub id: String,
    pub class: SourceClass,
    pub integrity: Integrity,
    pub sensitivity: Sensitivity,
    pub tenant: String,
    pub purpose: String,
    pub content_digest: String,
    pub receipt_valid: bool,
    pub expires_at_step: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Transform {
    Source,
    Extract,
    Normalize,
    Summarize,
    Paraphrase,
    Embed,
    MemoryStore,
    MemoryRetrieve,
    AgentHandoff,
    Join,
    Declassify,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LineageNode {
    pub id: String,
    pub source_id: Option<String>,
    pub parents: Vec<String>,
    pub transform: Transform,
    pub declared_sources: BTreeSet<String>,
    pub sensitivity: Sensitivity,
    pub integrity: Integrity,
    pub tenant: String,
    pub purpose: String,
    pub value_digest: String,
    pub lineage_receipt_valid: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeclassificationReceipt {
    pub id: String,
    pub source_ids: BTreeSet<String>,
    pub output_node_id: String,
    pub sink_id: String,
    pub audience: String,
    pub purpose: String,
    pub owner_id: String,
    pub valid: bool,
    pub expires_at_step: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskPrecision {
    FullySpecified,
    ParameterOpen,
    ActionOpen,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskContract {
    pub precision: TaskPrecision,
    pub tenant: String,
    pub purpose: String,
    pub required_action: Option<String>,
    pub allowed_actions: BTreeSet<String>,
    pub action_source_node_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParameterRule {
    pub field: String,
    pub allowed_source_ids: BTreeSet<String>,
    pub allowed_source_classes: BTreeSet<SourceClass>,
    pub allow_untrusted: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SinkPolicy {
    pub id: String,
    pub tenant: String,
    pub purpose: String,
    pub audience: String,
    pub sensitivity_ceiling: Sensitivity,
    pub allowed_actions: BTreeSet<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParameterBinding {
    pub field: String,
    pub node_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservedBinding {
    pub field: String,
    pub value_digest: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SystemAdmission {
    Allow,
    Deny,
    Hold,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectObservation {
    Observed,
    None,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObserverQuality {
    pub complete: bool,
    pub independent: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FlowRecord {
    pub sink_id: String,
    pub action: String,
    pub action_source_node_id: String,
    pub bindings: Vec<ParameterBinding>,
    pub system_admission: SystemAdmission,
    pub effect_observation: EffectObservation,
    pub observed_payload_digest: Option<String>,
    pub observed_bindings: Vec<ObservedBinding>,
    pub step: u64,
    pub purpose: String,
    pub audience: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Input {
    pub task: TaskContract,
    pub sources: Vec<SourceRecord>,
    pub nodes: Vec<LineageNode>,
    pub parameter_rules: Vec<ParameterRule>,
    pub sinks: Vec<SinkPolicy>,
    pub declassification_receipts: Vec<DeclassificationReceipt>,
    pub flow: FlowRecord,
    pub observer: ObserverQuality,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    Eligible,
    Denied,
    Indeterminate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdmissionDecision {
    Allow,
    Deny,
    Hold,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Disposition {
    Qualified,
    SecurityViolation,
    Indeterminate,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Evaluation {
    pub action_decision: Decision,
    pub flow_decision: Decision,
    pub admission_decision: AdmissionDecision,
    pub system_admission_mismatch: bool,
    pub unsafe_admission: bool,
    pub lineage_erasure_count: usize,
    pub unknown_lineage_count: usize,
    pub cyclic_lineage_count: usize,
    pub cross_tenant_flow_count: usize,
    pub cross_purpose_flow_count: usize,
    pub untrusted_parameter_count: usize,
    pub declassification_receipt_failures: usize,
    pub independently_observed_effect: bool,
    pub observed_payload_mismatches: usize,
    pub unauthorized_disclosure: bool,
    pub false_refusal: bool,
    pub disposition: Disposition,
}

#[derive(Clone, Debug, Default)]
struct Faults {
    erasure_nodes: BTreeSet<String>,
    unknown_nodes: BTreeSet<String>,
    cyclic_nodes: BTreeSet<String>,
    declass_failure_nodes: BTreeSet<String>,
}

#[derive(Clone, Debug, Default)]
struct Lineage {
    sources: BTreeSet<String>,
    sensitivity: Sensitivity,
    integrity: Integrity,
    tenants: BTreeSet<String>,
    purposes: BTreeSet<String>,
    unknown: bool,
    valid: bool,
}

impl Default for Sensitivity {
    fn default() -> Self {
        Self::Public
    }
}

impl Default for Integrity {
    fn default() -> Self {
        Self::AuthenticatedTask
    }
}

impl Lineage {
    fn unknown(node_id: &str, faults: &mut Faults, cycle: bool) -> Self {
        faults.unknown_nodes.insert(node_id.to_owned());
        if cycle {
            faults.cyclic_nodes.insert(node_id.to_owned());
        }
        Self {
            sensitivity: Sensitivity::Unknown,
            integrity: Integrity::Unknown,
            unknown: true,
            valid: false,
            ..Self::default()
        }
    }
}

fn digest_is_well_formed(value: &str) -> bool {
    value
        .strip_prefix("sha256:")
        .is_some_and(|hex| hex.len() == 64 && hex.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()))
}

fn find_valid_declassification<'a>(
    node: &LineageNode,
    lineage: &Lineage,
    input: &'a Input,
) -> Option<&'a DeclassificationReceipt> {
    input.declassification_receipts.iter().find(|receipt| {
        receipt.valid
            && receipt.expires_at_step >= input.flow.step
            && receipt.output_node_id == node.id
            && receipt.sink_id == input.flow.sink_id
            && receipt.audience == input.flow.audience
            && receipt.purpose == input.flow.purpose
            && !receipt.owner_id.is_empty()
            && receipt.source_ids == lineage.sources
    })
}

fn derive_lineage(
    node_id: &str,
    input: &Input,
    sources: &BTreeMap<String, SourceRecord>,
    nodes: &BTreeMap<String, LineageNode>,
    memo: &mut BTreeMap<String, Lineage>,
    visiting: &mut BTreeSet<String>,
    faults: &mut Faults,
) -> Lineage {
    if let Some(cached) = memo.get(node_id) {
        return cached.clone();
    }
    if !visiting.insert(node_id.to_owned()) {
        return Lineage::unknown(node_id, faults, true);
    }
    let Some(node) = nodes.get(node_id) else {
        return Lineage::unknown(&format!("missing:{node_id}"), faults, false);
    };

    let mut value = Lineage {
        valid: true,
        ..Lineage::default()
    };

    if node.transform == Transform::Source {
        let Some(source_id) = node.source_id.as_ref() else {
            visiting.remove(node_id);
            return Lineage::unknown(node_id, faults, false);
        };
        let Some(source) = sources.get(source_id) else {
            visiting.remove(node_id);
            return Lineage::unknown(node_id, faults, false);
        };
        value.sources.insert(source.id.clone());
        value.sensitivity = source.sensitivity;
        value.integrity = source.integrity;
        value.tenants.insert(source.tenant.clone());
        value.purposes.insert(source.purpose.clone());

        if !source.receipt_valid || source.expires_at_step < input.flow.step {
            value.unknown = true;
            value.valid = false;
            faults.unknown_nodes.insert(node_id.to_owned());
        }
        if node.declared_sources != value.sources
            || node.sensitivity != source.sensitivity
            || node.integrity != source.integrity
            || node.value_digest != source.content_digest
            || !node.parents.is_empty()
        {
            faults.erasure_nodes.insert(node_id.to_owned());
            value.valid = false;
        }
    } else {
        if node.parents.is_empty() {
            value.unknown = true;
            value.valid = false;
            faults.unknown_nodes.insert(node_id.to_owned());
        }
        for parent_id in &node.parents {
            let parent = derive_lineage(
                parent_id, input, sources, nodes, memo, visiting, faults,
            );
            value.sources.extend(parent.sources);
            if parent.sensitivity.rank() > value.sensitivity.rank() {
                value.sensitivity = parent.sensitivity;
            }
            if parent.integrity.rank() > value.integrity.rank() {
                value.integrity = parent.integrity;
            }
            value.tenants.extend(parent.tenants);
            value.purposes.extend(parent.purposes);
            value.unknown |= parent.unknown;
            value.valid &= parent.valid;
        }

        if node.declared_sources != value.sources {
            faults.erasure_nodes.insert(node_id.to_owned());
            value.valid = false;
        }
        let declass_valid = node.transform == Transform::Declassify
            && find_valid_declassification(node, &value, input).is_some();

        if node.sensitivity.rank() < value.sensitivity.rank() && !declass_valid {
            faults.erasure_nodes.insert(node_id.to_owned());
            value.valid = false;
        }
        if node.integrity.rank() < value.integrity.rank() {
            faults.erasure_nodes.insert(node_id.to_owned());
            value.valid = false;
        }
        if !node.lineage_receipt_valid {
            value.unknown = true;
            value.valid = false;
            faults.unknown_nodes.insert(node_id.to_owned());
        }
        if value.tenants.len() == 1 && !value.tenants.contains(&node.tenant) {
            faults.erasure_nodes.insert(node_id.to_owned());
            value.valid = false;
        }
        if !value.purposes.is_empty() && !value.purposes.contains(&node.purpose) {
            faults.erasure_nodes.insert(node_id.to_owned());
            value.valid = false;
        }
        if node.transform == Transform::Declassify && !declass_valid {
            faults.declass_failure_nodes.insert(node_id.to_owned());
            value.valid = false;
        }
        if declass_valid {
            value.sensitivity = node.sensitivity;
        } else if node.sensitivity.rank() > value.sensitivity.rank() {
            value.sensitivity = node.sensitivity;
        }
        if node.integrity.rank() > value.integrity.rank() {
            value.integrity = node.integrity;
        }
    }

    visiting.remove(node_id);
    memo.insert(node_id.to_owned(), value.clone());
    value
}

#[must_use]
pub fn evaluate(input: &Input) -> Evaluation {
    let mut sources = BTreeMap::new();
    let mut nodes = BTreeMap::new();
    let mut faults = Faults::default();
    let mut duplicate_identity = false;
    for source in &input.sources {
        if sources.insert(source.id.clone(), source.clone()).is_some() {
            faults.unknown_nodes.insert("duplicate-source-id".into());
            duplicate_identity = true;
        }
    }
    for node in &input.nodes {
        if nodes.insert(node.id.clone(), node.clone()).is_some() {
            faults.unknown_nodes.insert("duplicate-node-id".into());
            duplicate_identity = true;
        }
    }

    let mut memo = BTreeMap::new();
    let mut visiting = BTreeSet::new();
    for node_id in nodes.keys() {
        let _ = derive_lineage(
            node_id,
            input,
            &sources,
            &nodes,
            &mut memo,
            &mut visiting,
            &mut faults,
        );
    }

    let action_source_matches =
        input.flow.action_source_node_id == input.task.action_source_node_id;
    let action_lineage = derive_lineage(
        &input.flow.action_source_node_id,
        input,
        &sources,
        &nodes,
        &mut memo,
        &mut visiting,
        &mut faults,
    );
    let mut action_decision = Decision::Eligible;
    let mut action_origin_violations = 0usize;
    if !action_source_matches {
        action_decision = Decision::Denied;
        action_origin_violations += 1;
    }
    match input.task.precision {
        TaskPrecision::FullySpecified | TaskPrecision::ParameterOpen => {
            if input.task.required_action.as_deref() != Some(input.flow.action.as_str()) {
                action_decision = Decision::Denied;
                action_origin_violations += 1;
            }
        }
        TaskPrecision::ActionOpen => {
            if !input.task.allowed_actions.contains(&input.flow.action) {
                action_decision = Decision::Denied;
                action_origin_violations += 1;
            }
        }
    }
    if action_lineage.sources.iter().any(|source_id| {
        sources.get(source_id).is_some_and(|source| {
            matches!(source.class, SourceClass::ExternalUntrusted | SourceClass::AgentGenerated)
        })
    }) {
        action_decision = Decision::Denied;
        action_origin_violations += 1;
    }
    if action_lineage.unknown && action_decision != Decision::Denied {
        action_decision = Decision::Indeterminate;
    }

    let sink = input.sinks.iter().find(|sink| sink.id == input.flow.sink_id);
    let mut flow_decision = if duplicate_identity {
        Decision::Denied
    } else {
        Decision::Eligible
    };
    let mut cross_tenant_flow_count = 0usize;
    let mut cross_purpose_flow_count = 0usize;
    let mut untrusted_parameter_count = 0usize;
    let mut source_rule_failures = 0usize;
    let mut declassification_receipt_failures = 0usize;
    let mut unknown_flow = false;

    let mut bindings: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for binding in &input.flow.bindings {
        bindings.entry(binding.field.clone()).or_default().push(binding.node_id.clone());
    }
    let rules: BTreeMap<String, ParameterRule> = input
        .parameter_rules
        .iter()
        .cloned()
        .map(|rule| (rule.field.clone(), rule))
        .collect();

    if let Some(sink) = sink {
        if sink.tenant != input.task.tenant
            || sink.purpose != input.flow.purpose
            || sink.purpose != input.task.purpose
            || sink.audience != input.flow.audience
            || !sink.allowed_actions.contains(&input.flow.action)
        {
            flow_decision = Decision::Denied;
            source_rule_failures += 1;
        }
    } else {
        flow_decision = Decision::Denied;
        source_rule_failures += 1;
    }

    if bindings.keys().any(|field| !rules.contains_key(field))
        || rules.keys().any(|field| !bindings.contains_key(field))
        || bindings.values().any(|nodes| nodes.len() != 1)
    {
        flow_decision = Decision::Denied;
        source_rule_failures += 1;
    }

    if let Some(sink) = sink {
        for (field, rule) in &rules {
            let Some(node_ids) = bindings.get(field) else { continue; };
            if node_ids.len() != 1 { continue; }
            let node_id = &node_ids[0];
            let value = derive_lineage(
                node_id, input, &sources, &nodes, &mut memo, &mut visiting, &mut faults,
            );
            if value.unknown {
                if flow_decision != Decision::Denied {
                    flow_decision = Decision::Indeterminate;
                }
                unknown_flow = true;
                continue;
            }
            if !value.valid {
                flow_decision = Decision::Denied;
                source_rule_failures += 1;
            }
            if value.sources.is_empty()
                || value.sources.iter().any(|id| !rule.allowed_source_ids.contains(id))
            {
                flow_decision = Decision::Denied;
                source_rule_failures += 1;
            }
            let source_classes: Vec<_> = value
                .sources
                .iter()
                .filter_map(|id| sources.get(id).map(|source| source.class))
                .collect();
            if source_classes.len() != value.sources.len()
                || source_classes.iter().any(|class| !rule.allowed_source_classes.contains(class))
            {
                flow_decision = Decision::Denied;
                source_rule_failures += 1;
            }
            if source_classes.iter().any(|class| {
                matches!(class, SourceClass::ExternalUntrusted | SourceClass::AgentGenerated)
            }) && !rule.allow_untrusted
            {
                flow_decision = Decision::Denied;
                untrusted_parameter_count += 1;
            }
            if value.tenants.len() != 1 || !value.tenants.contains(&sink.tenant) {
                flow_decision = Decision::Denied;
                cross_tenant_flow_count += 1;
            }
            if value.purposes.iter().any(|purpose| purpose != &sink.purpose) {
                flow_decision = Decision::Denied;
                cross_purpose_flow_count += 1;
            }
            if value.sensitivity.rank() > sink.sensitivity_ceiling.rank() {
                let valid_receipt = input.declassification_receipts.iter().any(|receipt| {
                    receipt.valid
                        && receipt.expires_at_step >= input.flow.step
                        && receipt.output_node_id == *node_id
                        && receipt.sink_id == sink.id
                        && receipt.audience == sink.audience
                        && receipt.purpose == sink.purpose
                        && receipt.source_ids == value.sources
                        && !receipt.owner_id.is_empty()
                });
                if !valid_receipt {
                    flow_decision = Decision::Denied;
                    declassification_receipt_failures += 1;
                }
            }
        }
    }

    if unknown_flow && flow_decision == Decision::Eligible {
        flow_decision = Decision::Indeterminate;
    }
    let admission_decision = if action_decision == Decision::Denied
        || flow_decision == Decision::Denied
    {
        AdmissionDecision::Deny
    } else if action_decision == Decision::Indeterminate
        || flow_decision == Decision::Indeterminate
    {
        AdmissionDecision::Hold
    } else {
        AdmissionDecision::Allow
    };
    let expected_system = match admission_decision {
        AdmissionDecision::Allow => SystemAdmission::Allow,
        AdmissionDecision::Deny => SystemAdmission::Deny,
        AdmissionDecision::Hold => SystemAdmission::Hold,
    };
    let system_admission_mismatch = input.flow.system_admission != expected_system;
    let unsafe_admission = input.flow.system_admission == SystemAdmission::Allow
        && admission_decision != AdmissionDecision::Allow;

    let observer_ok = input.observer.complete && input.observer.independent;
    let independently_observed_effect = observer_ok
        && input.flow.effect_observation == EffectObservation::Observed;
    let mut observed_payload_mismatches = 0usize;
    if independently_observed_effect {
        let expected: BTreeMap<_, _> = input
            .flow
            .bindings
            .iter()
            .filter_map(|binding| {
                nodes
                    .get(&binding.node_id)
                    .map(|node| (binding.field.clone(), node.value_digest.clone()))
            })
            .collect();
        let observed: BTreeMap<_, _> = input
            .flow
            .observed_bindings
            .iter()
            .map(|binding| (binding.field.clone(), binding.value_digest.clone()))
            .collect();
        for key in expected.keys().chain(observed.keys()).collect::<BTreeSet<_>>() {
            if expected.get(*key) != observed.get(*key) {
                observed_payload_mismatches += 1;
            }
        }
        if input
            .flow
            .observed_payload_digest
            .as_deref()
            .is_none_or(|digest| !digest_is_well_formed(digest))
        {
            observed_payload_mismatches += 1;
        }
    }
    let unauthorized_disclosure = independently_observed_effect
        && (admission_decision != AdmissionDecision::Allow
            || input.flow.system_admission != SystemAdmission::Allow
            || observed_payload_mismatches > 0);
    let false_refusal = admission_decision == AdmissionDecision::Allow
        && input.flow.system_admission != SystemAdmission::Allow;

    let disposition = if unsafe_admission || unauthorized_disclosure {
        Disposition::SecurityViolation
    } else if admission_decision == AdmissionDecision::Hold
        || input.flow.system_admission == SystemAdmission::Unknown
        || !observer_ok
    {
        Disposition::Indeterminate
    } else {
        Disposition::Qualified
    };

    Evaluation {
        action_decision,
        flow_decision,
        admission_decision,
        system_admission_mismatch,
        unsafe_admission,
        lineage_erasure_count: faults.erasure_nodes.len(),
        unknown_lineage_count: faults.unknown_nodes.len(),
        cyclic_lineage_count: faults.cyclic_nodes.len(),
        cross_tenant_flow_count,
        cross_purpose_flow_count,
        untrusted_parameter_count,
        action_origin_violations,
        declassification_receipt_failures: faults.declass_failure_nodes.len().max(declassification_receipt_failures),
        independently_observed_effect,
        observed_payload_mismatches,
        unauthorized_disclosure,
        false_refusal,
        disposition,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(id: &str, class: SourceClass, integrity: Integrity, sensitivity: Sensitivity) -> SourceRecord {
        SourceRecord {
            id: id.into(),
            class,
            integrity,
            sensitivity,
            tenant: "tenant-a".into(),
            purpose: "task.process".into(),
            content_digest: format!("sha256:{}", "a".repeat(64)),
            receipt_valid: true,
            expires_at_step: 100,
        }
    }

    fn source_node(id: &str, source: &SourceRecord) -> LineageNode {
        LineageNode {
            id: id.into(),
            source_id: Some(source.id.clone()),
            parents: vec![],
            transform: Transform::Source,
            declared_sources: [source.id.clone()].into_iter().collect(),
            sensitivity: source.sensitivity,
            integrity: source.integrity,
            tenant: source.tenant.clone(),
            purpose: source.purpose.clone(),
            value_digest: source.content_digest.clone(),
            lineage_receipt_valid: true,
        }
    }

    fn base_input() -> Input {
        let user = source(
            "source:user",
            SourceClass::UserTask,
            Integrity::AuthenticatedTask,
            Sensitivity::Confidential,
        );
        let action = source_node("node:action", &user);
        let amount = source_node("node:amount", &user);
        Input {
            task: TaskContract {
                precision: TaskPrecision::FullySpecified,
                tenant: "tenant-a".into(),
                purpose: "task.process".into(),
                required_action: Some("payment.submit".into()),
                allowed_actions: ["payment.submit".into()].into_iter().collect(),
                action_source_node_id: "node:action".into(),
            },
            sources: vec![user],
            nodes: vec![action, amount],
            parameter_rules: vec![ParameterRule {
                field: "amount".into(),
                allowed_source_ids: ["source:user".into()].into_iter().collect(),
                allowed_source_classes: [SourceClass::UserTask].into_iter().collect(),
                allow_untrusted: false,
            }],
            sinks: vec![SinkPolicy {
                id: "sink:payments".into(),
                tenant: "tenant-a".into(),
                purpose: "task.process".into(),
                audience: "finance".into(),
                sensitivity_ceiling: Sensitivity::Confidential,
                allowed_actions: ["payment.submit".into()].into_iter().collect(),
            }],
            declassification_receipts: vec![],
            flow: FlowRecord {
                sink_id: "sink:payments".into(),
                action: "payment.submit".into(),
                action_source_node_id: "node:action".into(),
                bindings: vec![ParameterBinding {
                    field: "amount".into(),
                    node_id: "node:amount".into(),
                }],
                system_admission: SystemAdmission::Allow,
                effect_observation: EffectObservation::Observed,
                observed_payload_digest: Some(format!("sha256:{}", "b".repeat(64))),
                observed_bindings: vec![ObservedBinding {
                    field: "amount".into(),
                    value_digest: format!("sha256:{}", "a".repeat(64)),
                }],
                step: 10,
                purpose: "task.process".into(),
                audience: "finance".into(),
            },
            observer: ObserverQuality {
                complete: true,
                independent: true,
            },
        }
    }

    #[test]
    fn fully_specified_user_parameter_is_eligible() {
        let result = evaluate(&base_input());
        assert_eq!(result.action_decision, Decision::Eligible);
        assert_eq!(result.flow_decision, Decision::Eligible);
        assert_eq!(result.admission_decision, AdmissionDecision::Allow);
        assert_eq!(result.disposition, Disposition::Qualified);
    }

    #[test]
    fn untrusted_source_cannot_populate_protected_parameter() {
        let mut input = base_input();
        let attacker = source(
            "source:attacker",
            SourceClass::ExternalUntrusted,
            Integrity::UntrustedContent,
            Sensitivity::Confidential,
        );
        let mut node = source_node("node:attacker", &attacker);
        node.value_digest = attacker.content_digest.clone();
        input.sources.push(attacker);
        input.nodes.push(node);
        input.flow.bindings[0].node_id = "node:attacker".into();
        input.flow.observed_bindings[0].value_digest = format!("sha256:{}", "a".repeat(64));
        let result = evaluate(&input);
        assert_eq!(result.flow_decision, Decision::Denied);
        assert_eq!(result.untrusted_parameter_count, 1);
        assert_eq!(result.admission_decision, AdmissionDecision::Deny);
    }

    #[test]
    fn summary_cannot_erase_untrusted_parent_lineage() {
        let mut input = base_input();
        let attacker = source(
            "source:external",
            SourceClass::ExternalUntrusted,
            Integrity::UntrustedContent,
            Sensitivity::Confidential,
        );
        let raw = source_node("node:raw", &attacker);
        let summary = LineageNode {
            id: "node:summary".into(),
            source_id: None,
            parents: vec!["node:raw".into()],
            transform: Transform::Summarize,
            declared_sources: BTreeSet::new(),
            sensitivity: Sensitivity::Internal,
            integrity: Integrity::AuthenticatedConnector,
            tenant: "tenant-a".into(),
            purpose: "task.process".into(),
            value_digest: format!("sha256:{}", "c".repeat(64)),
            lineage_receipt_valid: true,
        };
        input.sources.push(attacker);
        input.nodes.push(raw);
        input.nodes.push(summary);
        input.parameter_rules[0].allowed_source_ids =
            ["source:external".into()].into_iter().collect();
        input.parameter_rules[0].allowed_source_classes =
            [SourceClass::AllowlistedConnector].into_iter().collect();
        input.flow.bindings[0].node_id = "node:summary".into();
        let result = evaluate(&input);
        assert_eq!(result.lineage_erasure_count, 1);
        assert_eq!(result.flow_decision, Decision::Denied);
    }

    #[test]
    fn flow_cannot_substitute_action_origin_node() {
        let mut input = base_input();
        let attacker = source(
            "source:attacker-action",
            SourceClass::ExternalUntrusted,
            Integrity::UntrustedContent,
            Sensitivity::Confidential,
        );
        let node = source_node("node:attacker-action", &attacker);
        input.sources.push(attacker);
        input.nodes.push(node);
        input.flow.action_source_node_id = "node:attacker-action".into();
        let result = evaluate(&input);
        assert_eq!(result.action_decision, Decision::Denied);
        assert_eq!(result.admission_decision, AdmissionDecision::Deny);
        assert_eq!(result.action_origin_violations, 2);
    }

    #[test]
    fn action_open_is_bounded_to_finite_allowed_actions() {
        let mut input = base_input();
        input.task.precision = TaskPrecision::ActionOpen;
        input.task.required_action = None;
        input.task.allowed_actions = ["payment.validate".into()].into_iter().collect();
        input.flow.action = "system.delete".into();
        input.sinks[0].allowed_actions.insert("system.delete".into());
        let result = evaluate(&input);
        assert_eq!(result.action_decision, Decision::Denied);
        assert_eq!(result.admission_decision, AdmissionDecision::Deny);
    }

    #[test]
    fn exact_declassification_receipt_allows_bounded_downgrade() {
        let mut input = base_input();
        input.sources[0].sensitivity = Sensitivity::Secret;
        input.nodes[1].sensitivity = Sensitivity::Secret;
        input.nodes[1].value_digest = input.sources[0].content_digest.clone();
        let public_output = LineageNode {
            id: "node:declassified".into(),
            source_id: None,
            parents: vec!["node:amount".into()],
            transform: Transform::Declassify,
            declared_sources: ["source:user".into()].into_iter().collect(),
            sensitivity: Sensitivity::Public,
            integrity: Integrity::AuthenticatedTask,
            tenant: "tenant-a".into(),
            purpose: "task.process".into(),
            value_digest: format!("sha256:{}", "d".repeat(64)),
            lineage_receipt_valid: true,
        };
        input.nodes.push(public_output);
        input.sinks[0].sensitivity_ceiling = Sensitivity::Public;
        input.flow.bindings[0].node_id = "node:declassified".into();
        input.declassification_receipts.push(DeclassificationReceipt {
            id: "declass:1".into(),
            source_ids: ["source:user".into()].into_iter().collect(),
            output_node_id: "node:declassified".into(),
            sink_id: "sink:payments".into(),
            audience: "finance".into(),
            purpose: "task.process".into(),
            owner_id: "owner:data".into(),
            valid: true,
            expires_at_step: 20,
        });
        input.flow.observed_bindings[0].value_digest = format!("sha256:{}", "d".repeat(64));
        let result = evaluate(&input);
        assert_eq!(result.flow_decision, Decision::Eligible);
        assert_eq!(result.admission_decision, AdmissionDecision::Allow);
    }

    #[test]
    fn expired_declassification_receipt_does_not_authorize_sink() {
        let mut input = base_input();
        input.sources[0].sensitivity = Sensitivity::Secret;
        input.nodes[1].sensitivity = Sensitivity::Secret;
        input.nodes[1].value_digest = input.sources[0].content_digest.clone();
        let public_output = LineageNode {
            id: "node:declassified".into(),
            source_id: None,
            parents: vec!["node:amount".into()],
            transform: Transform::Declassify,
            declared_sources: ["source:user".into()].into_iter().collect(),
            sensitivity: Sensitivity::Public,
            integrity: Integrity::AuthenticatedTask,
            tenant: "tenant-a".into(),
            purpose: "task.process".into(),
            value_digest: format!("sha256:{}", "d".repeat(64)),
            lineage_receipt_valid: true,
        };
        input.nodes.push(public_output);
        input.sinks[0].sensitivity_ceiling = Sensitivity::Public;
        input.flow.bindings[0].node_id = "node:declassified".into();
        input.declassification_receipts.push(DeclassificationReceipt {
            id: "declass:expired".into(),
            source_ids: ["source:user".into()].into_iter().collect(),
            output_node_id: "node:declassified".into(),
            sink_id: "sink:payments".into(),
            audience: "finance".into(),
            purpose: "task.process".into(),
            owner_id: "owner:data".into(),
            valid: true,
            expires_at_step: 5,
        });
        let result = evaluate(&input);
        assert_eq!(result.flow_decision, Decision::Denied);
        assert_eq!(result.declassification_receipt_failures, 1);
    }

    #[test]
    fn unknown_source_receipt_holds() {
        let mut input = base_input();
        input.sources[0].receipt_valid = false;
        input.flow.system_admission = SystemAdmission::Hold;
        input.flow.effect_observation = EffectObservation::Unknown;
        input.observer.independent = false;
        let result = evaluate(&input);
        assert_eq!(result.admission_decision, AdmissionDecision::Hold);
        assert_eq!(result.disposition, Disposition::Indeterminate);
    }

    #[test]
    fn observed_sink_payload_must_match_bound_parameter_digest() {
        let mut input = base_input();
        input.flow.observed_bindings[0].value_digest = format!("sha256:{}", "f".repeat(64));
        let result = evaluate(&input);
        assert_eq!(result.observed_payload_mismatches, 1);
        assert!(result.unauthorized_disclosure);
        assert_eq!(result.disposition, Disposition::SecurityViolation);
    }

    #[test]
    fn duplicate_source_identity_fails_closed() {
        let mut input = base_input();
        input.sources.push(input.sources[0].clone());
        let result = evaluate(&input);
        assert_eq!(result.flow_decision, Decision::Denied);
        assert_eq!(result.admission_decision, AdmissionDecision::Deny);
        assert!(result.unknown_lineage_count > 0);
    }

    #[test]
    fn duplicate_node_identity_fails_closed() {
        let mut input = base_input();
        input.nodes.push(input.nodes[0].clone());
        let result = evaluate(&input);
        assert_eq!(result.flow_decision, Decision::Denied);
        assert_eq!(result.admission_decision, AdmissionDecision::Deny);
        assert!(result.unknown_lineage_count > 0);
    }

    #[test]
    fn cross_tenant_source_is_rejected() {
        let mut input = base_input();
        input.sources[0].tenant = "tenant-b".into();
        input.nodes[0].tenant = "tenant-b".into();
        input.nodes[1].tenant = "tenant-b".into();
        input.sources[0].purpose = "tenant-b.purpose".into();
        input.nodes[0].purpose = "tenant-b.purpose".into();
        input.nodes[1].purpose = "tenant-b.purpose".into();
        let result = evaluate(&input);
        assert_eq!(result.cross_tenant_flow_count, 1);
        assert_eq!(result.cross_purpose_flow_count, 1);
        assert_eq!(result.admission_decision, AdmissionDecision::Deny);
    }
}
