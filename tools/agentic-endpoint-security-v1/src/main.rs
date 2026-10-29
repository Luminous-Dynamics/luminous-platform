use agentic_endpoint_assurance_model::{
    evaluate, Authorization, BindingTuple, Disposition, Effect, EvidenceBundle, Freshness,
    ObservationSource, Revocation, RollbackState, Telemetry,
};

fn example<'a>(label: &str, evidence: EvidenceBundle<'a>) {
    let result = evaluate(evidence);
    println!(
        concat!(
            "{}: disposition={:?}; authorization={:?}; binding={:?}; ",
            "freshness={:?}; revocation={:?}; telemetry={:?}; effect={:?}; ",
            "observation={:?}; rollback={:?}"
        ),
        label,
        result.disposition,
        result.evidence.authorization,
        result.computed_binding,
        result.evidence.freshness,
        result.evidence.revocation,
        result.evidence.telemetry,
        result.evidence.effect,
        result.evidence.observation_source,
        result.evidence.rollback,
    );
}

fn main() {
    let binding = BindingTuple {
        target_id: "endpoint-a",
        task_id: "task-read-fixture",
        action_id: "read-file",
        artifact_digest: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        policy_digest: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        policy_epoch: 7,
        nonce_digest: "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
    };
    let baseline = EvidenceBundle {
        authorization: Authorization::Allowed,
        expected_binding: binding,
        observed_binding: Some(binding),
        freshness: Freshness::Fresh,
        revocation: Revocation::Clear,
        telemetry: Telemetry::CompleteForDeclaredScope,
        effect: Effect::ExpectedEffectObserved,
        observation_source: ObservationSource::Independent,
        rollback: RollbackState::Current,
        external_witness_required: true,
    };

    println!(
        "REFERENCE MODEL ONLY: examples are synthetic facts, not endpoint tests or security proof."
    );
    example("conforming fixture", baseline);
    example(
        "denied fixture with no independently observed effect",
        EvidenceBundle {
            authorization: Authorization::Denied,
            effect: Effect::NoEffectObserved,
            ..baseline
        },
    );
    example(
        "telemetry-gap fixture",
        EvidenceBundle {
            telemetry: Telemetry::GapDetected,
            ..baseline
        },
    );
    example(
        "forbidden-effect fixture",
        EvidenceBundle {
            effect: Effect::ForbiddenEffectObserved,
            ..baseline
        },
    );

    let expected = evaluate(baseline).disposition;
    assert_eq!(expected, Disposition::AuthorizedEffectVerified);
}
