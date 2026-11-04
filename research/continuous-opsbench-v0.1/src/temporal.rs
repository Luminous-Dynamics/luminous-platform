//! Temporal topology and causal-model freshness evaluator.
//!
//! Evaluator truth, agent-visible observations, causal hypotheses, and action
//! attempts are separate inputs. This module validates their bindings and reports
//! stale-model exposure; it does not authenticate publishers or run infrastructure.

use crate::Digest;
use std::collections::{BTreeMap, BTreeSet};

pub const TEMPORAL_TOPOLOGY_SCHEMA: &str = "continuous-opsbench-temporal-topology-v0.1";
pub const MAX_CLOCK_UNCERTAINTY_MS: u64 = 1_000;
pub const MAX_EDGES_PER_SNAPSHOT: usize = 100_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EdgeKnowledge {
    Present,
    Absent,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TemporalEdge {
    pub edge_id: String,
    pub from_service: String,
    pub to_service: String,
    pub relation: String,
    pub knowledge: EdgeKnowledge,
    pub valid_from_ms: u64,
    pub valid_until_ms: Option<u64>,
    pub evidence_digest: Option<Digest>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TopologyObservation {
    pub observation_id: String,
    pub scope_id: String,
    pub epoch: u64,
    pub topology_digest: Digest,
    pub source_id: String,
    pub control_domain: String,
    pub source_sequence: u64,
    /// Time claimed by the source for the underlying change/event.
    pub event_time_ms: u64,
    /// Time the independent collector received the record.
    pub ingested_at_ms: u64,
    /// Time the agent/evaluator incorporated the record.
    pub processed_at_ms: u64,
    pub clock_uncertainty_ms: u64,
    /// Half-open validity interval [valid_from_ms, valid_until_ms).
    pub valid_from_ms: u64,
    pub valid_until_ms: Option<u64>,
    pub edges: Vec<TemporalEdge>,
}

/// Evaluator-owned topology truth. This plane is never exposed to the agent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TopologyTruthEpoch {
    pub scope_id: String,
    pub epoch: u64,
    pub valid_from_ms: u64,
    pub valid_until_ms: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CausalHypothesis {
    pub hypothesis_id: String,
    pub scope_id: String,
    pub topology_epoch: u64,
    pub topology_digest: Digest,
    pub created_at_ms: u64,
    pub confidence_ppm: u32,
    pub evidence_digests: Vec<Digest>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionAttempt {
    pub action_id: String,
    pub hypothesis_id: String,
    pub scope_id: String,
    pub observation_id: String,
    pub required_topology_epoch: u64,
    pub attempted_at_ms: u64,
    pub maximum_observation_age_ms: u64,
    pub committed_effect: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TemporalTopologyBundle {
    pub schema: String,
    pub truth_epochs: Vec<TopologyTruthEpoch>,
    pub observations: Vec<TopologyObservation>,
    pub hypotheses: Vec<CausalHypothesis>,
    pub actions: Vec<ActionAttempt>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TemporalTopologyError {
    WrongSchema,
    EmptyIdentity(&'static str),
    DuplicateIdentity(&'static str, String),
    InvalidInterval(String),
    InvalidClockUncertainty(String),
    TooManyEdges(String),
    DuplicateEdge(String),
    DuplicateSourceSequence(String),
    OverlappingTruthEpochs(String),
    MissingHypothesis(String),
    MissingObservation(String),
    HypothesisObservationMismatch(String),
    InvalidConfidence(String),
    EmptyHypothesisEvidence(String),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TemporalTopologySummary {
    pub truth_epoch_count: usize,
    pub observation_count: usize,
    pub hypothesis_count: usize,
    pub action_attempt_count: usize,
    pub source_sequence_gaps: usize,
    pub unknown_edges: usize,
    pub stale_model_action_attempts: usize,
    pub stale_model_committed_effects: usize,
    pub missing_observation_actions: usize,
    pub stale_observation_actions: usize,
    pub unknown_truth_actions: usize,
    pub hypotheses_requiring_revision: usize,
}

fn interval_contains(start: u64, end: Option<u64>, at: u64) -> bool {
    at >= start && end.map(|end| at < end).unwrap_or(true)
}

fn valid_interval(start: u64, end: Option<u64>) -> bool {
    end.map(|end| start < end).unwrap_or(true)
}

fn truth_epoch_at<'a>(
    epochs: &'a [TopologyTruthEpoch],
    scope: &str,
    at: u64,
) -> Option<&'a TopologyTruthEpoch> {
    epochs
        .iter()
        .find(|epoch| epoch.scope_id == scope && interval_contains(epoch.valid_from_ms, epoch.valid_until_ms, at))
}

pub fn validate_temporal_topology(
    bundle: &TemporalTopologyBundle,
) -> Result<TemporalTopologySummary, TemporalTopologyError> {
    if bundle.schema != TEMPORAL_TOPOLOGY_SCHEMA {
        return Err(TemporalTopologyError::WrongSchema);
    }

    let mut truth_by_scope: BTreeMap<&str, Vec<&TopologyTruthEpoch>> = BTreeMap::new();
    for epoch in &bundle.truth_epochs {
        if epoch.scope_id.is_empty() {
            return Err(TemporalTopologyError::EmptyIdentity("truth scope"));
        }
        if !valid_interval(epoch.valid_from_ms, epoch.valid_until_ms) {
            return Err(TemporalTopologyError::InvalidInterval(epoch.scope_id.clone()));
        }
        truth_by_scope.entry(&epoch.scope_id).or_default().push(epoch);
    }
    for (scope, epochs) in &mut truth_by_scope {
        epochs.sort_by_key(|epoch| epoch.valid_from_ms);
        for pair in epochs.windows(2) {
            if pair[0].valid_until_ms.map(|end| end > pair[1].valid_from_ms).unwrap_or(true) {
                return Err(TemporalTopologyError::OverlappingTruthEpochs((*scope).to_owned()));
            }
        }
    }

    let mut observation_ids = BTreeMap::new();
    let mut source_sequences: BTreeMap<(&str, &str), BTreeSet<u64>> = BTreeMap::new();
    let mut summary = TemporalTopologySummary::default();

    for observation in &bundle.observations {
        for (field, value) in [
            ("observation_id", observation.observation_id.as_str()),
            ("observation_scope", observation.scope_id.as_str()),
            ("observation_source", observation.source_id.as_str()),
            ("observation_control_domain", observation.control_domain.as_str()),
        ] {
            if value.is_empty() {
                return Err(TemporalTopologyError::EmptyIdentity(field));
            }
        }
        if observation_ids.insert(observation.observation_id.as_str(), observation).is_some() {
            return Err(TemporalTopologyError::DuplicateIdentity(
                "observation",
                observation.observation_id.clone(),
            ));
        }
        if !valid_interval(observation.valid_from_ms, observation.valid_until_ms) {
            return Err(TemporalTopologyError::InvalidInterval(observation.observation_id.clone()));
        }
        if observation.clock_uncertainty_ms > MAX_CLOCK_UNCERTAINTY_MS {
            return Err(TemporalTopologyError::InvalidClockUncertainty(observation.observation_id.clone()));
        }
        if observation.edges.len() > MAX_EDGES_PER_SNAPSHOT {
            return Err(TemporalTopologyError::TooManyEdges(observation.observation_id.clone()));
        }
        let mut edge_ids = BTreeSet::new();
        for edge in &observation.edges {
            if edge.edge_id.is_empty() || edge.from_service.is_empty() || edge.to_service.is_empty() || edge.relation.is_empty() {
                return Err(TemporalTopologyError::EmptyIdentity("topology edge"));
            }
            if !edge_ids.insert(edge.edge_id.as_str()) {
                return Err(TemporalTopologyError::DuplicateEdge(edge.edge_id.clone()));
            }
            if !valid_interval(edge.valid_from_ms, edge.valid_until_ms) {
                return Err(TemporalTopologyError::InvalidInterval(edge.edge_id.clone()));
            }
            if edge.knowledge == EdgeKnowledge::Unknown {
                summary.unknown_edges += 1;
            }
        }
        let sequences = source_sequences
            .entry((&observation.source_id, &observation.control_domain))
            .or_default();
        if !sequences.insert(observation.source_sequence) {
            return Err(TemporalTopologyError::DuplicateSourceSequence(observation.source_id.clone()));
        }
    }

    for sequences in source_sequences.values() {
        if let (Some(first), Some(last)) = (sequences.first(), sequences.last()) {
            summary.source_sequence_gaps += (*last - *first + 1).saturating_sub(sequences.len() as u64) as usize;
        }
    }

    let mut hypotheses = BTreeMap::new();
    for hypothesis in &bundle.hypotheses {
        if hypothesis.hypothesis_id.is_empty() || hypothesis.scope_id.is_empty() {
            return Err(TemporalTopologyError::EmptyIdentity("causal hypothesis"));
        }
        if hypotheses.insert(hypothesis.hypothesis_id.as_str(), hypothesis).is_some() {
            return Err(TemporalTopologyError::DuplicateIdentity("hypothesis", hypothesis.hypothesis_id.clone()));
        }
        if hypothesis.confidence_ppm > 1_000_000 {
            return Err(TemporalTopologyError::InvalidConfidence(hypothesis.hypothesis_id.clone()));
        }
        if hypothesis.evidence_digests.is_empty() {
            return Err(TemporalTopologyError::EmptyHypothesisEvidence(hypothesis.hypothesis_id.clone()));
        }
        let bound = bundle.observations.iter().any(|observation| {
            observation.scope_id == hypothesis.scope_id
                && observation.epoch == hypothesis.topology_epoch
                && observation.topology_digest == hypothesis.topology_digest
        });
        if !bound {
            return Err(TemporalTopologyError::HypothesisObservationMismatch(hypothesis.hypothesis_id.clone()));
        }
    }

    let mut action_ids = BTreeSet::new();
    for action in &bundle.actions {
        if action.action_id.is_empty() || action.scope_id.is_empty() {
            return Err(TemporalTopologyError::EmptyIdentity("action"));
        }
        if !action_ids.insert(action.action_id.as_str()) {
            return Err(TemporalTopologyError::DuplicateIdentity("action", action.action_id.clone()));
        }
        let hypothesis = hypotheses
            .get(action.hypothesis_id.as_str())
            .ok_or_else(|| TemporalTopologyError::MissingHypothesis(action.hypothesis_id.clone()))?;
        if hypothesis.scope_id != action.scope_id {
            return Err(TemporalTopologyError::HypothesisObservationMismatch(action.action_id.clone()));
        }

        let observation = match observation_ids.get(action.observation_id.as_str()) {
            Some(observation) => *observation,
            None => {
                summary.missing_observation_actions += 1;
                summary.stale_model_action_attempts += 1;
                if action.committed_effect {
                    summary.stale_model_committed_effects += 1;
                }
                continue;
            }
        };

        let age = action.attempted_at_ms.checked_sub(observation.processed_at_ms);
        let fresh = age.map(|age| {
            age.saturating_add(observation.clock_uncertainty_ms) <= action.maximum_observation_age_ms
        }).unwrap_or(false);
        if !fresh {
            summary.stale_observation_actions += 1;
        }

        let truth_epoch = truth_epoch_at(&bundle.truth_epochs, &action.scope_id, action.attempted_at_ms);
        let truth_matches = truth_epoch.map(|truth| {
            truth.epoch == action.required_topology_epoch
                && truth.epoch == hypothesis.topology_epoch
        }).unwrap_or(false);
        if truth_epoch.is_none() {
            summary.unknown_truth_actions += 1;
        }

        let observation_matches = observation.epoch == action.required_topology_epoch
            && observation.topology_digest == hypothesis.topology_digest
            && observation.scope_id == action.scope_id;

        let stale = !fresh || !truth_matches || !observation_matches;
        if stale {
            summary.stale_model_action_attempts += 1;
            if action.committed_effect {
                summary.stale_model_committed_effects += 1;
            }
        }
        if !truth_matches && truth_epoch.is_some() {
            summary.hypotheses_requiring_revision += 1;
        }
    }

    summary.truth_epoch_count = bundle.truth_epochs.len();
    summary.observation_count = bundle.observations.len();
    summary.hypothesis_count = bundle.hypotheses.len();
    summary.action_attempt_count = bundle.actions.len();
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(n: u8) -> Digest { Digest::from_byte(n) }

    fn fixture() -> TemporalTopologyBundle {
        let edge = TemporalEdge {
            edge_id: "edge-a-db".into(),
            from_service: "api".into(),
            to_service: "db".into(),
            relation: "reads-from".into(),
            knowledge: EdgeKnowledge::Present,
            valid_from_ms: 0,
            valid_until_ms: None,
            evidence_digest: Some(d(3)),
        };
        let observation = TopologyObservation {
            observation_id: "obs-1".into(),
            scope_id: "prod/zone-a".into(),
            epoch: 1,
            topology_digest: d(1),
            source_id: "inventory-observer".into(),
            control_domain: "observer-domain".into(),
            source_sequence: 1,
            event_time_ms: 100,
            ingested_at_ms: 110,
            processed_at_ms: 120,
            clock_uncertainty_ms: 2,
            valid_from_ms: 0,
            valid_until_ms: None,
            edges: vec![edge],
        };
        let hypothesis = CausalHypothesis {
            hypothesis_id: "hyp-1".into(),
            scope_id: "prod/zone-a".into(),
            topology_epoch: 1,
            topology_digest: d(1),
            created_at_ms: 130,
            confidence_ppm: 900_000,
            evidence_digests: vec![d(4)],
        };
        TemporalTopologyBundle {
            schema: TEMPORAL_TOPOLOGY_SCHEMA.into(),
            truth_epochs: vec![TopologyTruthEpoch {
                scope_id: "prod/zone-a".into(),
                epoch: 1,
                valid_from_ms: 0,
                valid_until_ms: None,
            }],
            observations: vec![observation],
            hypotheses: vec![hypothesis],
            actions: vec![ActionAttempt {
                action_id: "act-1".into(),
                hypothesis_id: "hyp-1".into(),
                scope_id: "prod/zone-a".into(),
                observation_id: "obs-1".into(),
                required_topology_epoch: 1,
                attempted_at_ms: 150,
                maximum_observation_age_ms: 100,
                committed_effect: false,
            }],
        }
    }

    #[test]
    fn fresh_action_on_current_epoch_is_not_stale() {
        let result = validate_temporal_topology(&fixture()).unwrap();
        assert_eq!(result.stale_model_action_attempts, 0);
        assert_eq!(result.stale_model_committed_effects, 0);
    }

    #[test]
    fn topology_transition_invalidates_old_hypothesis() {
        let mut bundle = fixture();
        bundle.truth_epochs.push(TopologyTruthEpoch {
            scope_id: "prod/zone-a".into(), epoch: 2, valid_from_ms: 140, valid_until_ms: None,
        });
        bundle.truth_epochs[0].valid_until_ms = Some(140);
        let result = validate_temporal_topology(&bundle).unwrap();
        assert_eq!(result.hypotheses_requiring_revision, 1);
        assert_eq!(result.stale_model_action_attempts, 1);
    }

    #[test]
    fn stale_model_committed_effect_is_counted_separately() {
        let mut bundle = fixture();
        bundle.truth_epochs.push(TopologyTruthEpoch {
            scope_id: "prod/zone-a".into(), epoch: 2, valid_from_ms: 140, valid_until_ms: None,
        });
        bundle.truth_epochs[0].valid_until_ms = Some(140);
        bundle.actions[0].committed_effect = true;
        let result = validate_temporal_topology(&bundle).unwrap();
        assert_eq!(result.stale_model_committed_effects, 1);
    }

    #[test]
    fn unknown_truth_is_not_treated_as_current_or_healthy() {
        let mut bundle = fixture();
        bundle.truth_epochs.clear();
        let result = validate_temporal_topology(&bundle).unwrap();
        assert_eq!(result.unknown_truth_actions, 1);
        assert_eq!(result.stale_model_action_attempts, 1);
    }

    #[test]
    fn missing_observation_is_not_treated_as_success() {
        let mut bundle = fixture();
        bundle.actions[0].observation_id = "missing".into();
        let result = validate_temporal_topology(&bundle).unwrap();
        assert_eq!(result.missing_observation_actions, 1);
        assert_eq!(result.stale_model_action_attempts, 1);
    }

    #[test]
    fn delayed_observation_expires_for_action_precondition() {
        let mut bundle = fixture();
        bundle.actions[0].attempted_at_ms = 500;
        bundle.actions[0].maximum_observation_age_ms = 100;
        let result = validate_temporal_topology(&bundle).unwrap();
        assert_eq!(result.stale_observation_actions, 1);
        assert_eq!(result.stale_model_action_attempts, 1);
    }

    #[test]
    fn clock_uncertainty_is_added_to_observation_age() {
        let mut bundle = fixture();
        bundle.observations[0].clock_uncertainty_ms = 40;
        bundle.actions[0].attempted_at_ms = 190;
        bundle.actions[0].maximum_observation_age_ms = 70;
        let result = validate_temporal_topology(&bundle).unwrap();
        assert_eq!(result.stale_observation_actions, 1);
    }

    #[test]
    fn event_time_and_ingest_time_may_be_out_of_order_without_rewriting_history() {
        let mut bundle = fixture();
        bundle.observations[0].event_time_ms = 300;
        bundle.observations[0].ingested_at_ms = 200;
        bundle.observations[0].processed_at_ms = 210;
        bundle.actions[0].attempted_at_ms = 250;
        let result = validate_temporal_topology(&bundle).unwrap();
        assert_eq!(result.stale_model_action_attempts, 0);
    }

    #[test]
    fn sequence_gaps_are_counted_not_silently_filled() {
        let mut bundle = fixture();
        let mut second = bundle.observations[0].clone();
        second.observation_id = "obs-3".into();
        second.source_sequence = 3;
        second.epoch = 3;
        second.topology_digest = d(8);
        bundle.observations.push(second);
        let result = validate_temporal_topology(&bundle).unwrap();
        assert_eq!(result.source_sequence_gaps, 1);
    }

    #[test]
    fn duplicate_source_sequence_is_rejected() {
        let mut bundle = fixture();
        let mut second = bundle.observations[0].clone();
        second.observation_id = "obs-2".into();
        bundle.observations.push(second);
        assert!(matches!(
            validate_temporal_topology(&bundle),
            Err(TemporalTopologyError::DuplicateSourceSequence(_))
        ));
    }

    #[test]
    fn overlapping_evaluator_truth_epochs_are_rejected() {
        let mut bundle = fixture();
        bundle.truth_epochs.push(TopologyTruthEpoch {
            scope_id: "prod/zone-a".into(), epoch: 2, valid_from_ms: 100, valid_until_ms: None,
        });
        assert!(matches!(
            validate_temporal_topology(&bundle),
            Err(TemporalTopologyError::OverlappingTruthEpochs(_))
        ));
    }

    #[test]
    fn unknown_edge_is_distinct_from_absent_edge() {
        let mut bundle = fixture();
        bundle.observations[0].edges[0].knowledge = EdgeKnowledge::Unknown;
        bundle.observations[0].edges.push(TemporalEdge {
            edge_id: "edge-a-cache".into(),
            from_service: "api".into(),
            to_service: "cache".into(),
            relation: "reads-from".into(),
            knowledge: EdgeKnowledge::Absent,
            valid_from_ms: 0,
            valid_until_ms: None,
            evidence_digest: None,
        });
        let result = validate_temporal_topology(&bundle).unwrap();
        assert_eq!(result.unknown_edges, 1);
    }

    #[test]
    fn hypothesis_must_bind_exact_observed_topology_digest() {
        let mut bundle = fixture();
        bundle.hypotheses[0].topology_digest = d(99);
        assert!(matches!(
            validate_temporal_topology(&bundle),
            Err(TemporalTopologyError::HypothesisObservationMismatch(_))
        ));
    }

    #[test]
    fn invalid_time_intervals_are_rejected() {
        let mut bundle = fixture();
        bundle.observations[0].valid_until_ms = Some(0);
        assert!(matches!(
            validate_temporal_topology(&bundle),
            Err(TemporalTopologyError::InvalidInterval(_))
        ));
    }

    #[test]
    fn confidence_above_one_is_rejected() {
        let mut bundle = fixture();
        bundle.hypotheses[0].confidence_ppm = 1_000_001;
        assert!(matches!(
            validate_temporal_topology(&bundle),
            Err(TemporalTopologyError::InvalidConfidence(_))
        ));
    }
}
