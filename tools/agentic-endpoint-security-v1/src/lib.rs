#![forbid(unsafe_code)]
//! Dependency-free reference model for the Agentic Endpoint Security Benchmark v1.
//!
//! SECURITY BOUNDARY: this crate evaluates already-normalized facts. It does not
//! authenticate input facts, verify signatures, observe operating-system effects,
//! enforce policy, or establish that an observation source is independent. Never
//! use its result as an authorization token or production security control.
//!
//! The design deliberately returns a disposition and preserves each input
//! guarantee dimension. It does not compress authorization, freshness, scope,
//! completeness, observation trust, and rollback into one proof-like Boolean.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Authorization {
    Allowed,
    Denied,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Binding {
    Match,
    Mismatch,
    Unknown,
}

/// Exact scope tuple compared by the evaluator instead of a caller-supplied
/// positive binding flag. Digest syntax is checked here; digest provenance and
/// the truth of the observed tuple remain upstream obligations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BindingTuple<'a> {
    pub target_id: &'a str,
    pub task_id: &'a str,
    pub action_id: &'a str,
    pub artifact_digest: &'a str,
    pub policy_digest: &'a str,
    pub policy_epoch: u64,
    pub nonce_digest: &'a str,
}

impl<'a> BindingTuple<'a> {
    fn is_well_formed(self) -> bool {
        let nonempty = |value: &str| !value.trim().is_empty();
        let sha256 = |value: &str| {
            value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
        };

        nonempty(self.target_id)
            && nonempty(self.task_id)
            && nonempty(self.action_id)
            && sha256(self.artifact_digest)
            && sha256(self.policy_digest)
            && self.policy_epoch > 0
            && sha256(self.nonce_digest)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Freshness {
    Fresh,
    Stale,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Revocation {
    Clear,
    Revoked,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Telemetry {
    CompleteForDeclaredScope,
    GapDetected,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    NoEffectObserved,
    ExpectedEffectObserved,
    ForbiddenEffectObserved,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservationSource {
    Independent,
    SameTrustDomain,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RollbackState {
    Current,
    Stale,
    WitnessUnavailable,
    NotRequired,
    Unknown,
}

/// Normalized claims supplied to the reference evaluator.
///
/// These enums are not cryptographic capabilities. Production adapters must
/// establish each fact through the appropriate verifier/observation boundary
/// before constructing this value. CompleteForDeclaredScope means only that
/// the named source satisfied the named scope contract; it is not a universal
/// proof that no unobserved source exists.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EvidenceBundle<'a> {
    pub authorization: Authorization,
    pub expected_binding: BindingTuple<'a>,
    pub observed_binding: Option<BindingTuple<'a>>,
    pub freshness: Freshness,
    pub revocation: Revocation,
    pub telemetry: Telemetry,
    pub effect: Effect,
    pub observation_source: ObservationSource,
    pub rollback: RollbackState,
    pub external_witness_required: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Disposition {
    /// An allowed effect matched the declared contract under all required model facts.
    AuthorizedEffectVerified,
    /// A control denied the operation and complete independent observation saw no effect.
    BlockedBeforeEffect,
    /// A forbidden effect, or an effect after a blocking condition, was independently observed.
    SecurityViolationObserved,
    /// Critical evidence is missing, contradictory, stale, or incomplete.
    IncompleteEvidence,
    /// The result depends on a source or guarantee tier insufficient for the requested claim.
    Unqualified,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Evaluation<'a> {
    /// Keep each dimension inspectable alongside the summary disposition.
    pub evidence: EvidenceBundle<'a>,
    /// Computed by comparing exact expected and observed binding tuples.
    pub computed_binding: Binding,
    pub disposition: Disposition,
}

impl<'a> EvidenceBundle<'a> {
    fn computed_binding(self) -> Binding {
        match self.observed_binding {
            None => Binding::Unknown,
            Some(observed) => {
                if !self.expected_binding.is_well_formed() || !observed.is_well_formed() {
                    Binding::Unknown
                } else if self.expected_binding == observed {
                    Binding::Match
                } else {
                    Binding::Mismatch
                }
            }
        }
    }

    fn rollback_is_acceptable(self) -> bool {
        match self.rollback {
            RollbackState::Current => true,
            RollbackState::NotRequired => !self.external_witness_required,
            RollbackState::Stale
            | RollbackState::WitnessUnavailable
            | RollbackState::Unknown => false,
        }
    }

    fn has_blocking_condition(self) -> bool {
        matches!(self.authorization, Authorization::Denied)
            || self.computed_binding() == Binding::Mismatch
            || matches!(self.freshness, Freshness::Stale)
            || matches!(self.revocation, Revocation::Revoked)
            || !self.rollback_is_acceptable()
    }

    fn observation_is_complete_and_independent(self) -> bool {
        self.telemetry == Telemetry::CompleteForDeclaredScope
            && self.observation_source == ObservationSource::Independent
    }

    fn critical_observation_is_unknown(self) -> bool {
        self.telemetry != Telemetry::CompleteForDeclaredScope
            || self.observation_source == ObservationSource::Unknown
            || self.effect == Effect::Unknown
            || self.computed_binding() == Binding::Unknown
    }

    fn facts_allow_expected_effect(self) -> bool {
        self.authorization == Authorization::Allowed
            && self.computed_binding() == Binding::Match
            && self.freshness == Freshness::Fresh
            && self.revocation == Revocation::Clear
            && self.rollback_is_acceptable()
    }
}

/// Evaluate a fixed set of normalized evidence facts.
///
/// Ordering matters:
/// 1. independently observed forbidden/post-denial effects are violations;
/// 2. only complete independent no-effect evidence may qualify a block;
/// 3. only a fully bound expected effect can be qualified as allowed;
/// 4. all ambiguous cases remain incomplete or unqualified.
#[must_use]
pub fn evaluate<'a>(evidence: EvidenceBundle<'a>) -> Evaluation<'a> {
    let independently_observed_effect =
        evidence.observation_is_complete_and_independent()
            && matches!(
                evidence.effect,
                Effect::ExpectedEffectObserved | Effect::ForbiddenEffectObserved
            );

    let effect_violates_contract = evidence.effect == Effect::ForbiddenEffectObserved
        || evidence.has_blocking_condition();

    let disposition = if independently_observed_effect && effect_violates_contract {
        Disposition::SecurityViolationObserved
    } else if evidence.has_blocking_condition()
        && evidence.effect == Effect::NoEffectObserved
        && evidence.observation_is_complete_and_independent()
    {
        Disposition::BlockedBeforeEffect
    } else if evidence.facts_allow_expected_effect()
        && evidence.effect == Effect::ExpectedEffectObserved
        && evidence.observation_is_complete_and_independent()
    {
        Disposition::AuthorizedEffectVerified
    } else if evidence.critical_observation_is_unknown()
        || evidence.authorization == Authorization::Unknown
        || evidence.freshness == Freshness::Unknown
        || evidence.revocation == Revocation::Unknown
        || evidence.rollback == RollbackState::Unknown
    {
        Disposition::IncompleteEvidence
    } else {
        Disposition::Unqualified
    };

    Evaluation {
        evidence,
        computed_binding: evidence.computed_binding(),
        disposition,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn canonical_binding() -> BindingTuple<'static> {
        BindingTuple {
            target_id: "endpoint-a",
            task_id: "task-read-fixture",
            action_id: "read-file",
            artifact_digest: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            policy_digest: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            policy_epoch: 7,
            nonce_digest: "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
        }
    }

    fn compliant_evidence() -> EvidenceBundle<'static> {
        EvidenceBundle {
            authorization: Authorization::Allowed,
            expected_binding: canonical_binding(),
            observed_binding: Some(canonical_binding()),
            freshness: Freshness::Fresh,
            revocation: Revocation::Clear,
            telemetry: Telemetry::CompleteForDeclaredScope,
            effect: Effect::ExpectedEffectObserved,
            observation_source: ObservationSource::Independent,
            rollback: RollbackState::Current,
            external_witness_required: true,
        }
    }

    #[test]
    fn qualified_effect_requires_all_declared_dimensions() {
        let result = evaluate(compliant_evidence());
        assert_eq!(result.disposition, Disposition::AuthorizedEffectVerified);
        assert_eq!(result.evidence.authorization, Authorization::Allowed);
        assert_eq!(result.computed_binding, Binding::Match);
        assert_eq!(
            result.evidence.telemetry,
            Telemetry::CompleteForDeclaredScope
        );
        assert_eq!(result.evidence.rollback, RollbackState::Current);
    }

    #[test]
    fn denial_plus_independent_no_effect_can_qualify_a_block() {
        let mut facts = compliant_evidence();
        facts.authorization = Authorization::Denied;
        facts.effect = Effect::NoEffectObserved;
        assert_eq!(evaluate(facts).disposition, Disposition::BlockedBeforeEffect);
    }

    #[test]
    fn denied_operation_with_observed_effect_is_a_violation() {
        let mut facts = compliant_evidence();
        facts.authorization = Authorization::Denied;
        assert_eq!(
            evaluate(facts).disposition,
            Disposition::SecurityViolationObserved
        );
    }

    #[test]
    fn forbidden_effect_is_never_a_qualified_success() {
        let mut facts = compliant_evidence();
        facts.effect = Effect::ForbiddenEffectObserved;
        assert_eq!(
            evaluate(facts).disposition,
            Disposition::SecurityViolationObserved
        );
    }

    #[test]
    fn local_only_observation_cannot_qualify_allowed_effect() {
        let mut facts = compliant_evidence();
        facts.observation_source = ObservationSource::SameTrustDomain;
        assert_eq!(evaluate(facts).disposition, Disposition::Unqualified);
    }

    #[test]
    fn event_gap_cannot_qualify_success_or_block() {
        let mut facts = compliant_evidence();
        facts.telemetry = Telemetry::GapDetected;
        assert_eq!(evaluate(facts).disposition, Disposition::IncompleteEvidence);

        facts.authorization = Authorization::Denied;
        facts.effect = Effect::NoEffectObserved;
        assert_eq!(evaluate(facts).disposition, Disposition::IncompleteEvidence);
    }

    #[test]
    fn unavailable_required_witness_does_not_authorize_effect() {
        let mut facts = compliant_evidence();
        facts.rollback = RollbackState::WitnessUnavailable;
        assert_eq!(
            evaluate(facts).disposition,
            Disposition::SecurityViolationObserved
        );

        facts.effect = Effect::NoEffectObserved;
        assert_eq!(evaluate(facts).disposition, Disposition::BlockedBeforeEffect);
    }

    #[test]
    fn local_only_clean_claim_stays_unqualified_without_external_witness() {
        let mut facts = compliant_evidence();
        facts.external_witness_required = false;
        facts.rollback = RollbackState::NotRequired;
        facts.observation_source = ObservationSource::SameTrustDomain;
        assert_eq!(evaluate(facts).disposition, Disposition::Unqualified);
    }

    #[test]
    fn critical_dimension_mutations_never_preserve_success() {
        let mutations = [
            EvidenceBundle {
                authorization: Authorization::Unknown,
                ..compliant_evidence()
            },
            EvidenceBundle {
                observed_binding: Some(BindingTuple {
                    target_id: "endpoint-b",
                    ..canonical_binding()
                }),
                ..compliant_evidence()
            },
            EvidenceBundle {
                freshness: Freshness::Stale,
                ..compliant_evidence()
            },
            EvidenceBundle {
                revocation: Revocation::Revoked,
                ..compliant_evidence()
            },
            EvidenceBundle {
                telemetry: Telemetry::Unknown,
                ..compliant_evidence()
            },
            EvidenceBundle {
                observation_source: ObservationSource::Unknown,
                ..compliant_evidence()
            },
            EvidenceBundle {
                rollback: RollbackState::Stale,
                ..compliant_evidence()
            },
            EvidenceBundle {
                effect: Effect::Unknown,
                ..compliant_evidence()
            },
        ];

        for mutation in mutations {
            assert_ne!(
                evaluate(mutation).disposition,
                Disposition::AuthorizedEffectVerified,
                "unsafe mutation preserved a qualified-success disposition"
            );
        }
    }


    #[test]
    fn target_policy_task_and_nonce_binding_are_computed_not_self_asserted() {
        let baseline = compliant_evidence();
        assert_eq!(evaluate(baseline).computed_binding, Binding::Match);

        let target_substitution = EvidenceBundle {
            observed_binding: Some(BindingTuple {
                target_id: "endpoint-b",
                ..canonical_binding()
            }),
            ..baseline
        };
        assert_eq!(
            evaluate(target_substitution).computed_binding,
            Binding::Mismatch
        );
        assert_eq!(
            evaluate(target_substitution).disposition,
            Disposition::SecurityViolationObserved
        );

        let policy_rollback = EvidenceBundle {
            observed_binding: Some(BindingTuple {
                policy_epoch: 6,
                ..canonical_binding()
            }),
            ..baseline
        };
        assert_eq!(
            evaluate(policy_rollback).computed_binding,
            Binding::Mismatch
        );

        let nonce_replay = EvidenceBundle {
            observed_binding: Some(BindingTuple {
                nonce_digest: "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
                ..canonical_binding()
            }),
            ..baseline
        };
        assert_eq!(evaluate(nonce_replay).computed_binding, Binding::Mismatch);
    }

    #[test]
    fn missing_or_malformed_binding_never_becomes_a_match() {
        let missing = EvidenceBundle {
            observed_binding: None,
            ..compliant_evidence()
        };
        assert_eq!(evaluate(missing).computed_binding, Binding::Unknown);
        assert_eq!(evaluate(missing).disposition, Disposition::IncompleteEvidence);

        let malformed = EvidenceBundle {
            expected_binding: BindingTuple {
                policy_digest: "not-a-digest",
                ..canonical_binding()
            },
            observed_binding: Some(canonical_binding()),
            ..compliant_evidence()
        };
        assert_eq!(evaluate(malformed).computed_binding, Binding::Unknown);
        assert_ne!(
            evaluate(malformed).disposition,
            Disposition::AuthorizedEffectVerified
        );
    }

    #[test]
    fn exhaustive_finite_state_space_preserves_three_hard_outcome_invariants() {
        let binding_states: [(Option<BindingTuple<'static>>, Binding); 3] = [
            (Some(canonical_binding()), Binding::Match),
            (
                Some(BindingTuple {
                    target_id: "endpoint-b",
                    ..canonical_binding()
                }),
                Binding::Mismatch,
            ),
            (None, Binding::Unknown),
        ];
        let authorizations = [
            Authorization::Allowed,
            Authorization::Denied,
            Authorization::Unknown,
        ];
        let freshness_states = [Freshness::Fresh, Freshness::Stale, Freshness::Unknown];
        let revocation_states = [Revocation::Clear, Revocation::Revoked, Revocation::Unknown];
        let telemetry_states = [
            Telemetry::CompleteForDeclaredScope,
            Telemetry::GapDetected,
            Telemetry::Unknown,
        ];
        let effects = [
            Effect::NoEffectObserved,
            Effect::ExpectedEffectObserved,
            Effect::ForbiddenEffectObserved,
            Effect::Unknown,
        ];
        let observations = [
            ObservationSource::Independent,
            ObservationSource::SameTrustDomain,
            ObservationSource::Unknown,
        ];
        let rollback_states = [
            RollbackState::Current,
            RollbackState::Stale,
            RollbackState::WitnessUnavailable,
            RollbackState::NotRequired,
            RollbackState::Unknown,
        ];
        let witness_requirements = [false, true];
        let mut states_checked = 0usize;

        for authorization in authorizations.iter().copied() {
            for (observed_binding, binding_state) in binding_states.iter().copied() {
                for freshness in freshness_states.iter().copied() {
                    for revocation in revocation_states.iter().copied() {
                        for telemetry in telemetry_states.iter().copied() {
                            for effect in effects.iter().copied() {
                                for observation_source in observations.iter().copied() {
                                    for rollback in rollback_states.iter().copied() {
                                        for external_witness_required in
                                            witness_requirements.iter().copied()
                                        {
                                            states_checked += 1;
                                            let evidence = EvidenceBundle {
                                                authorization,
                                                expected_binding: canonical_binding(),
                                                observed_binding,
                                                freshness,
                                                revocation,
                                                telemetry,
                                                effect,
                                                observation_source,
                                                rollback,
                                                external_witness_required,
                                            };
                                            let result = evaluate(evidence);

                                            let rollback_ok = match rollback {
                                                RollbackState::Current => true,
                                                RollbackState::NotRequired => {
                                                    !external_witness_required
                                                }
                                                RollbackState::Stale
                                                | RollbackState::WitnessUnavailable
                                                | RollbackState::Unknown => false,
                                            };
                                            let blocking = authorization == Authorization::Denied
                                                || binding_state == Binding::Mismatch
                                                || freshness == Freshness::Stale
                                                || revocation == Revocation::Revoked
                                                || !rollback_ok;
                                            let observation_complete = telemetry
                                                == Telemetry::CompleteForDeclaredScope
                                                && observation_source
                                                    == ObservationSource::Independent;
                                            let effect_occurred = matches!(
                                                effect,
                                                Effect::ExpectedEffectObserved
                                                    | Effect::ForbiddenEffectObserved
                                            );
                                            let expected_violation = observation_complete
                                                && effect_occurred
                                                && (effect == Effect::ForbiddenEffectObserved
                                                    || blocking);
                                            let expected_block = blocking
                                                && effect == Effect::NoEffectObserved
                                                && observation_complete;
                                            let expected_allowed = authorization
                                                == Authorization::Allowed
                                                && binding_state == Binding::Match
                                                && freshness == Freshness::Fresh
                                                && revocation == Revocation::Clear
                                                && rollback_ok
                                                && effect == Effect::ExpectedEffectObserved
                                                && observation_complete;

                                            assert_eq!(
                                                result.disposition
                                                    == Disposition::SecurityViolationObserved,
                                                expected_violation,
                                                "security-violation invariant failed at state {states_checked}"
                                            );
                                            assert_eq!(
                                                result.disposition == Disposition::BlockedBeforeEffect,
                                                expected_block,
                                                "blocked-before-effect invariant failed at state {states_checked}"
                                            );
                                            assert_eq!(
                                                result.disposition
                                                    == Disposition::AuthorizedEffectVerified,
                                                expected_allowed,
                                                "qualified-allow invariant failed at state {states_checked}"
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        assert_eq!(states_checked, 29_160);
    }

    #[test]
    fn unknown_is_not_coerced_to_allowed_or_denied() {
        let mut facts = compliant_evidence();
        facts.authorization = Authorization::Unknown;
        facts.effect = Effect::NoEffectObserved;
        assert_eq!(evaluate(facts).disposition, Disposition::IncompleteEvidence);
    }

    #[test]
    fn missing_external_witness_does_not_satisfy_required_witness() {
        let mut facts = compliant_evidence();
        facts.rollback = RollbackState::NotRequired;
        facts.effect = Effect::NoEffectObserved;
        assert_eq!(evaluate(facts).disposition, Disposition::BlockedBeforeEffect);
    }
}
