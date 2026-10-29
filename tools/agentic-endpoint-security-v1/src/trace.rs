//! Trace-ordering reference model for one scoped endpoint operation.
//!
//! This checks normalized records only. It does not authenticate records, prove
//! that an observer is genuinely independent, inspect OS state, or enforce policy.
//! Adapters must verify provenance and populate records from a trusted, complete
//! trace source before relying on these classifications.
//!
//! The trace contract is deliberately narrow: every record in the scoped trace
//! has a contiguous sequence number, there is exactly one terminal checkpoint,
//! and an effect observation is explicit (absence of an effect record is never
//! interpreted as proof that no effect occurred).

use super::{BindingTuple, Effect, ObservationSource};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TraceEvent {
    AuthorizationGranted,
    AuthorizationDenied,
    ActionAttempted,
    RevocationObserved,
    EffectObserved(Effect),
    Checkpoint {
        complete_through_sequence: u64,
        dropped_records: u64,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TraceRecord<'a> {
    pub sequence: u64,
    pub binding: BindingTuple<'a>,
    pub event: TraceEvent,
    /// Normalized assertion about this record's observation domain.
    /// This enum is not evidence that independence has actually been established.
    pub source: ObservationSource,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TraceEvidence<'a> {
    pub expected_binding: BindingTuple<'a>,
    pub first_sequence: u64,
    pub records: &'a [TraceRecord<'a>],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TraceDisposition {
    /// Exactly one grant preceded exactly one attempt and an independent oracle
    /// observed the expected effect before any applicable revocation or denial.
    AuthorizedEffectVerified,
    /// A complete independent no-effect observation accompanies a denial,
    /// pre-effect revocation, or attempted action with no matching grant.
    BlockedBeforeEffect,
    /// An independent observation exposes a forbidden, replayed, mis-scoped,
    /// denied, or revoked operation that nevertheless produced an effect.
    SecurityViolationObserved,
    /// Missing, malformed, contradictory, gapped, or dropped evidence.
    IncompleteEvidence,
    /// Evidence exists, but its asserted trust tier is insufficient to qualify
    /// either a successful operation or a confirmed block.
    Unqualified,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TraceEvaluation {
    pub disposition: TraceDisposition,
    pub records_checked: usize,
    pub grants_seen: usize,
    pub denials_seen: usize,
    pub attempts_seen: usize,
    pub revocations_seen: usize,
    pub effect_observations_seen: usize,
    pub first_sequence: u64,
    pub last_sequence: Option<u64>,
}

fn record_binding_matches<'a>(record: TraceRecord<'a>, expected: BindingTuple<'a>) -> bool {
    record.binding.is_well_formed() && record.binding == expected
}

fn has_valid_terminal_checkpoint(records: &[TraceRecord<'_>]) -> bool {
    let Some(last) = records.last() else {
        return false;
    };
    if !matches!(
        last.event,
        TraceEvent::Checkpoint {
            complete_through_sequence,
            dropped_records: 0
        } if complete_through_sequence == last.sequence
    ) || last.source != ObservationSource::Independent
    {
        return false;
    }

    // A checkpoint is terminal and unique. Its own sequence is included in the
    // completeness claim; an earlier checkpoint cannot certify later records.
    records[..records.len() - 1]
        .iter()
        .all(|record| !matches!(record.event, TraceEvent::Checkpoint { .. }))
}

fn is_contiguous(records: &[TraceRecord<'_>], first_sequence: u64) -> bool {
    let Some(first) = records.first() else {
        return false;
    };
    if first.sequence != first_sequence {
        return false;
    }
    records.windows(2).all(|pair| {
        pair[0].sequence.checked_add(1) == Some(pair[1].sequence)
    })
}

fn make_evaluation<'a>(
    evidence: TraceEvidence<'a>,
    disposition: TraceDisposition,
) -> TraceEvaluation {
    let mut grants_seen = 0;
    let mut denials_seen = 0;
    let mut attempts_seen = 0;
    let mut revocations_seen = 0;
    let mut effect_observations_seen = 0;
    for record in evidence.records {
        match record.event {
            TraceEvent::AuthorizationGranted => grants_seen += 1,
            TraceEvent::AuthorizationDenied => denials_seen += 1,
            TraceEvent::ActionAttempted => attempts_seen += 1,
            TraceEvent::RevocationObserved => revocations_seen += 1,
            TraceEvent::EffectObserved(_) => effect_observations_seen += 1,
            TraceEvent::Checkpoint { .. } => {}
        }
    }

    TraceEvaluation {
        disposition,
        records_checked: evidence.records.len(),
        grants_seen,
        denials_seen,
        attempts_seen,
        revocations_seen,
        effect_observations_seen,
        first_sequence: evidence.first_sequence,
        last_sequence: evidence.records.last().map(|record| record.sequence),
    }
}

/// Evaluate one operation's explicit authorization/attempt/revocation/effect trace.
///
/// This is a reference classifier, not an authorization mechanism. In particular,
/// the supplied source labels and checkpoint are assumed to have been validated
/// by an upstream adapter. A missing effect observation is never treated as no effect.
#[must_use]
pub fn evaluate_trace(evidence: TraceEvidence<'_>) -> TraceEvaluation {
    let records = evidence.records;
    let bindings_match = evidence.expected_binding.is_well_formed()
        && records
            .iter()
            .copied()
            .all(|record| record_binding_matches(record, evidence.expected_binding));

    let trace_complete = !records.is_empty()
        && is_contiguous(records, evidence.first_sequence)
        && has_valid_terminal_checkpoint(records)
        && evidence.expected_binding.is_well_formed()
        && bindings_match;

    // An independently observed forbidden effect or a well-formed, wrong-scope
    // effect is direct violation evidence. For expected effects, missing records
    // alone are not proof that authorization was absent: incomplete traces must
    // remain incomplete unless they contain a positive contradiction.
    for (effect_index, effect_record) in records.iter().copied().enumerate() {
        let TraceEvent::EffectObserved(effect) = effect_record.event else {
            continue;
        };
        if effect_record.source != ObservationSource::Independent
            || effect == Effect::NoEffectObserved
            || effect == Effect::Unknown
        {
            continue;
        }

        if effect == Effect::ForbiddenEffectObserved {
            return make_evaluation(evidence, TraceDisposition::SecurityViolationObserved);
        }
        if effect_record.binding.is_well_formed()
            && effect_record.binding != evidence.expected_binding
        {
            return make_evaluation(evidence, TraceDisposition::SecurityViolationObserved);
        }
        if !effect_record.binding.is_well_formed() {
            continue;
        }

        let grant_indices: Vec<usize> = records
            .iter()
            .enumerate()
            .filter_map(|(index, record)| {
                (record_binding_matches(*record, evidence.expected_binding)
                    && matches!(record.event, TraceEvent::AuthorizationGranted))
                .then_some(index)
            })
            .collect();
        let attempt_indices: Vec<usize> = records
            .iter()
            .enumerate()
            .filter_map(|(index, record)| {
                (record_binding_matches(*record, evidence.expected_binding)
                    && matches!(record.event, TraceEvent::ActionAttempted))
                .then_some(index)
            })
            .collect();
        let denial_before_effect = records.iter().any(|record| {
            record_binding_matches(*record, evidence.expected_binding)
                && record.sequence < effect_record.sequence
                && matches!(record.event, TraceEvent::AuthorizationDenied)
        });
        let revocation_before_effect = records.iter().any(|record| {
            record_binding_matches(*record, evidence.expected_binding)
                && record.sequence < effect_record.sequence
                && matches!(record.event, TraceEvent::RevocationObserved)
        });
        let explicit_contradiction = denial_before_effect
            || revocation_before_effect
            || grant_indices.len() > 1
            || attempt_indices.len() > 1
            || grant_indices
                .first()
                .zip(attempt_indices.first())
                .is_some_and(|(grant, attempt)| grant >= attempt)
            || attempt_indices
                .first()
                .is_some_and(|attempt| *attempt >= effect_index);

        if explicit_contradiction {
            return make_evaluation(evidence, TraceDisposition::SecurityViolationObserved);
        }

        // Only a complete trace can turn a missing grant/attempt into evidence
        // that the observed effect bypassed the contract.
        if trace_complete
            && (grant_indices.len() != 1
                || attempt_indices.len() != 1
                || !grant_indices
                    .first()
                    .zip(attempt_indices.first())
                    .is_some_and(|(grant, attempt)| grant < attempt && attempt < effect_index))
        {
            return make_evaluation(evidence, TraceDisposition::SecurityViolationObserved);
        }
    }

    if !trace_complete {
        // Preserve an untrusted effect signal as unqualified rather than
        // turning it into either a clean result or an asserted no-effect block.
        if records.iter().any(|record| {
            matches!(record.event, TraceEvent::EffectObserved(_))
                && record.source != ObservationSource::Independent
        }) {
            return make_evaluation(evidence, TraceDisposition::Unqualified);
        }
        return make_evaluation(evidence, TraceDisposition::IncompleteEvidence);
    }

    let effect_records: Vec<(usize, Effect, ObservationSource)> = records
        .iter()
        .enumerate()
        .filter_map(|(index, record)| match record.event {
            TraceEvent::EffectObserved(effect) => Some((index, effect, record.source)),
            _ => None,
        })
        .collect();

    if effect_records.len() != 1 {
        return make_evaluation(evidence, TraceDisposition::IncompleteEvidence);
    }
    let (effect_index, effect, effect_source) = effect_records[0];
    if effect_source != ObservationSource::Independent {
        return make_evaluation(evidence, TraceDisposition::Unqualified);
    }
    if effect == Effect::Unknown {
        return make_evaluation(evidence, TraceDisposition::IncompleteEvidence);
    }

    if effect == Effect::ExpectedEffectObserved {
        // The violation path above handles missing/duplicate/out-of-order grant
        // and attempt records. Only complete, contiguous evidence can qualify success.
        let grant_index = records
            .iter()
            .position(|record| matches!(record.event, TraceEvent::AuthorizationGranted));
        let attempt_index = records
            .iter()
            .position(|record| matches!(record.event, TraceEvent::ActionAttempted));
        if grant_index.zip(attempt_index).is_some_and(|(grant, attempt)| {
            grant < attempt && attempt < effect_index
        }) {
            return make_evaluation(evidence, TraceDisposition::AuthorizedEffectVerified);
        }
        return make_evaluation(evidence, TraceDisposition::SecurityViolationObserved);
    }

    // A no-effect observation only qualifies a block if the trace shows a
    // blocking condition. A clean but non-effectful run is not a security claim.
    let attempt_index = records
        .iter()
        .position(|record| matches!(record.event, TraceEvent::ActionAttempted));
    let grant_index = records
        .iter()
        .position(|record| matches!(record.event, TraceEvent::AuthorizationGranted));
    let denied = records
        .iter()
        .any(|record| matches!(record.event, TraceEvent::AuthorizationDenied));
    let revoked_before_attempt = records.iter().any(|record| {
        matches!(record.event, TraceEvent::RevocationObserved)
            && attempt_index.is_some_and(|attempt| {
                records[attempt].sequence > record.sequence
            })
    });
    let attempted_without_prior_grant = attempt_index.is_some_and(|attempt| {
        grant_index.is_none_or(|grant| grant >= attempt)
    });
    let contradictory_decision = denied && grant_index.is_some();
    if !contradictory_decision && (denied || revoked_before_attempt || attempted_without_prior_grant)
    {
        make_evaluation(evidence, TraceDisposition::BlockedBeforeEffect)
    } else {
        make_evaluation(evidence, TraceDisposition::Unqualified)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn binding() -> BindingTuple<'static> {
        BindingTuple {
            target_id: "endpoint-a",
            task_id: "task-42",
            action_id: "write-synthetic-canary",
            artifact_digest: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            policy_digest: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            policy_epoch: 7,
            nonce_digest: "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
        }
    }

    fn record(sequence: u64, event: TraceEvent) -> TraceRecord<'static> {
        TraceRecord {
            sequence,
            binding: binding(),
            event,
            source: if matches!(
                event,
                TraceEvent::EffectObserved(_) | TraceEvent::Checkpoint { .. }
            ) {
                ObservationSource::Independent
            } else {
                ObservationSource::SameTrustDomain
            },
        }
    }

    fn with_checkpoint(mut records: Vec<TraceRecord<'static>>) -> Vec<TraceRecord<'static>> {
        let next = records.last().map_or(1, |last| last.sequence + 1);
        records.push(TraceRecord {
            sequence: next,
            binding: binding(),
            event: TraceEvent::Checkpoint {
                complete_through_sequence: next,
                dropped_records: 0,
            },
            source: ObservationSource::Independent,
        });
        records
    }

    fn evaluate(records: &[TraceRecord<'static>], first_sequence: u64) -> TraceEvaluation {
        evaluate_trace(TraceEvidence {
            expected_binding: binding(),
            first_sequence,
            records,
        })
    }

    #[test]
    fn grant_attempt_and_independently_observed_effect_qualify() {
        let records = with_checkpoint(vec![
            record(10, TraceEvent::AuthorizationGranted),
            record(11, TraceEvent::ActionAttempted),
            record(12, TraceEvent::EffectObserved(Effect::ExpectedEffectObserved)),
        ]);
        assert_eq!(
            evaluate(&records, 10).disposition,
            TraceDisposition::AuthorizedEffectVerified
        );
    }

    #[test]
    fn explicit_no_effect_plus_denial_qualifies_a_block() {
        let records = with_checkpoint(vec![
            record(10, TraceEvent::AuthorizationDenied),
            record(11, TraceEvent::EffectObserved(Effect::NoEffectObserved)),
        ]);
        assert_eq!(
            evaluate(&records, 10).disposition,
            TraceDisposition::BlockedBeforeEffect
        );
    }

    #[test]
    fn effect_after_revocation_is_a_violation() {
        let records = with_checkpoint(vec![
            record(10, TraceEvent::AuthorizationGranted),
            record(11, TraceEvent::ActionAttempted),
            record(12, TraceEvent::RevocationObserved),
            record(13, TraceEvent::EffectObserved(Effect::ExpectedEffectObserved)),
        ]);
        assert_eq!(
            evaluate(&records, 10).disposition,
            TraceDisposition::SecurityViolationObserved
        );
    }

    #[test]
    fn independently_observed_effect_without_grant_is_a_violation() {
        let records = with_checkpoint(vec![
            record(10, TraceEvent::ActionAttempted),
            record(11, TraceEvent::EffectObserved(Effect::ExpectedEffectObserved)),
        ]);
        assert_eq!(
            evaluate(&records, 10).disposition,
            TraceDisposition::SecurityViolationObserved
        );
    }

    #[test]
    fn repeated_attempt_after_one_grant_is_a_violation_if_effect_occurs() {
        let records = with_checkpoint(vec![
            record(10, TraceEvent::AuthorizationGranted),
            record(11, TraceEvent::ActionAttempted),
            record(12, TraceEvent::ActionAttempted),
            record(13, TraceEvent::EffectObserved(Effect::ExpectedEffectObserved)),
        ]);
        assert_eq!(
            evaluate(&records, 10).disposition,
            TraceDisposition::SecurityViolationObserved
        );
    }

    #[test]
    fn forbidden_effect_is_a_violation_even_when_other_records_are_missing() {
        let records = vec![record(
            10,
            TraceEvent::EffectObserved(Effect::ForbiddenEffectObserved),
        )];
        assert_eq!(
            evaluate(&records, 10).disposition,
            TraceDisposition::SecurityViolationObserved
        );
    }

    #[test]
    fn a_gap_cannot_qualify_a_successful_effect() {
        let records = vec![
            record(10, TraceEvent::AuthorizationGranted),
            record(11, TraceEvent::ActionAttempted),
            record(13, TraceEvent::EffectObserved(Effect::ExpectedEffectObserved)),
            record(
                14,
                TraceEvent::Checkpoint {
                    complete_through_sequence: 14,
                    dropped_records: 0,
                },
            ),
        ];
        assert_eq!(
            evaluate(&records, 10).disposition,
            TraceDisposition::IncompleteEvidence
        );
    }

    #[test]
    fn dropped_records_cannot_qualify_a_block() {
        let records = vec![
            record(10, TraceEvent::AuthorizationDenied),
            record(11, TraceEvent::EffectObserved(Effect::NoEffectObserved)),
            TraceRecord {
                sequence: 12,
                binding: binding(),
                event: TraceEvent::Checkpoint {
                    complete_through_sequence: 12,
                    dropped_records: 1,
                },
                source: ObservationSource::Independent,
            },
        ];
        assert_eq!(
            evaluate(&records, 10).disposition,
            TraceDisposition::IncompleteEvidence
        );
    }

    #[test]
    fn same_trust_domain_effect_is_never_qualified() {
        let mut records = with_checkpoint(vec![
            record(10, TraceEvent::AuthorizationGranted),
            record(11, TraceEvent::ActionAttempted),
            record(12, TraceEvent::EffectObserved(Effect::ExpectedEffectObserved)),
        ]);
        records[2].source = ObservationSource::SameTrustDomain;
        assert_eq!(
            evaluate(&records, 10).disposition,
            TraceDisposition::Unqualified
        );
    }

    #[test]
    fn mismatched_binding_with_independent_effect_is_a_violation() {
        let mut wrong = record(
            12,
            TraceEvent::EffectObserved(Effect::ExpectedEffectObserved),
        );
        wrong.binding = BindingTuple {
            target_id: "endpoint-b",
            ..binding()
        };
        let records = with_checkpoint(vec![
            record(10, TraceEvent::AuthorizationGranted),
            record(11, TraceEvent::ActionAttempted),
            wrong,
        ]);
        assert_eq!(
            evaluate(&records, 10).disposition,
            TraceDisposition::SecurityViolationObserved
        );
    }

    #[test]
    fn missing_explicit_effect_observation_is_incomplete_not_no_effect() {
        let records = with_checkpoint(vec![
            record(10, TraceEvent::AuthorizationDenied),
        ]);
        assert_eq!(
            evaluate(&records, 10).disposition,
            TraceDisposition::IncompleteEvidence
        );
    }

    #[test]
    fn grant_and_denial_contradiction_does_not_qualify_a_block() {
        let records = with_checkpoint(vec![
            record(10, TraceEvent::AuthorizationGranted),
            record(11, TraceEvent::AuthorizationDenied),
            record(12, TraceEvent::EffectObserved(Effect::NoEffectObserved)),
        ]);
        assert_eq!(
            evaluate(&records, 10).disposition,
            TraceDisposition::Unqualified
        );
    }
}
