use luminous_syscall_policy_core::{
    evaluate, ArgumentFacts, AuthenticationState, Capability, CapabilityState, ClockState, Decision,
    Digest, EvaluationContext, Fact, OpenAccessMode, OpenArguments, Operation, PolicySnapshot,
    ObservedOperation, Request, ResourceIdentity, ResourceKind, RevocationState, Syscall,
    SyscallCall,
};

fn digest(ch: char) -> Digest {
    Digest::parse(ch.to_string().repeat(64)).expect("constant demo digest")
}

fn main() {
    let policy = PolicySnapshot {
        id: "demo-read-only-policy".into(),
        digest: digest('a'),
        target_id: "demo-endpoint".into(),
        epoch: 3,
        valid_from_ms: 1_000,
        expires_at_ms: 9_000,
        authentication: AuthenticationState::Verified,
        revocation: RevocationState::Clear,
        capabilities: vec![Capability {
            id: "read-demo-fixture".into(),
            target_id: "demo-endpoint".into(),
            subject_image_digest: digest('b'),
            resource: ResourceIdentity {
                kind: ResourceKind::File,
                stable_digest: digest('c'),
            },
            operations: vec![Operation::OpenRead],
            valid_from_ms: 1_000,
            expires_at_ms: 8_000,
            state: CapabilityState::Active,
        }],
    };
    let request = Request {
        observed: Fact::verified(ObservedOperation {
            target_id: "demo-endpoint".into(),
            subject_image_digest: digest('b'),
            resource: ResourceIdentity {
                kind: ResourceKind::File,
                stable_digest: digest('c'),
            },
            call: SyscallCall {
                syscall: Syscall::OpenAt,
                arguments: ArgumentFacts::Open(OpenArguments {
                    access: OpenAccessMode::ReadOnly,
                    create: false,
                    truncate: false,
                    append: false,
                    unsupported_flags: false,
                }),
            },
        }),
    };
    let context = EvaluationContext {
        now_ms: Some(2_000),
        clock_state: ClockState::Fresh,
        expected_policy_digest: Some(digest('a')),
        minimum_accepted_epoch: Some(3),
    };

    println!("SYNTHETIC POLICY DEMO ONLY — no kernel hook is installed.");
    println!("matching read capability: {:?}", evaluate(&policy, &request, &context));

    let mut write_request = request;
    let mut write_observation = write_request
        .observed
        .value
        .clone()
        .expect("demo observation is present");
    write_observation.call = SyscallCall {
        syscall: Syscall::OpenAt,
        arguments: ArgumentFacts::Open(OpenArguments {
            access: OpenAccessMode::WriteOnly,
            create: false,
            truncate: false,
            append: false,
            unsupported_flags: false,
        }),
    };
    write_request.observed = Fact::verified(write_observation);
    let result = evaluate(&policy, &write_request, &context);
    println!("write without a write capability: {result:?}");
    assert!(matches!(result, Decision::Deny(_)));
}
