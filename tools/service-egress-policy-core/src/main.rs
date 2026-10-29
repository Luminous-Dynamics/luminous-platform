use luminous_service_egress_policy_core::{
    evaluate, AuthenticationState, CapabilityId, CapabilityState, ClockState, CredentialHandleId,
    Digest, EgressCapability, EvaluationContext, Fact, OperationId, PolicyId, PolicySnapshot,
    RevocationState, ServiceId, TargetId, ObservedEgressAttempt, RequestId, Decision, DenyReason,
};

fn digest(character: char) -> Digest {
    Digest::parse(character.to_string().repeat(64)).expect("demo digest is valid")
}

fn main() {
    let target = TargetId::parse("agent-demo").expect("constant target id is valid");
    let subject = digest('b');
    let service = ServiceId::parse("code-host").expect("constant service id is valid");
    let credential = CredentialHandleId::parse("cred.readonly").expect("constant handle is valid");
    let policy = PolicySnapshot {
        id: PolicyId::parse("egress-demo").expect("constant policy id is valid"),
        digest: digest('a'),
        target_id: target.clone(),
        epoch: 3,
        valid_from_ms: 1_000,
        expires_at_ms: 9_000,
        authentication: AuthenticationState::Verified,
        revocation: RevocationState::Clear,
        capabilities: vec![EgressCapability {
            id: CapabilityId::parse("repo-read").expect("constant capability id is valid"),
            target_id: target.clone(),
            subject_image_digest: subject.clone(),
            service_id: service.clone(),
            operations: vec![OperationId::parse("repo.read_file").expect("constant operation id is valid")],
            credential_handle_id: Some(credential.clone()),
            valid_from_ms: 1_000,
            expires_at_ms: 8_000,
            state: CapabilityState::Active,
        }],
    };
    let context = EvaluationContext {
        now_ms: Some(2_000),
        clock_state: ClockState::Fresh,
        expected_policy_digest: Some(digest('a')),
        minimum_accepted_epoch: Some(3),
    };
    let mut attempt = ObservedEgressAttempt {
        request_id: RequestId::parse("request-1").expect("constant request id is valid"),
        target_id: target,
        subject_image_digest: subject,
        service_id: service,
        operation_id: OperationId::parse("repo.read_file").expect("constant operation id is valid"),
        credential_handle_id: Some(credential),
        redirect_hop: 0,
    };

    println!("SYNTHETIC POLICY DEMO ONLY — no proxy, socket, or kernel hook is installed.");
    println!("matching service operation: {:?}", evaluate(&policy, &Fact::verified(attempt.clone()), &context));
    assert!(matches!(
        evaluate(&policy, &Fact::verified(attempt.clone()), &context),
        Decision::Allow(_)
    ));

    attempt.operation_id = OperationId::parse("repo.write_file").expect("constant operation id is valid");
    let write_result = evaluate(&policy, &Fact::verified(attempt), &context);
    println!("write operation without an explicit grant: {write_result:?}");
    assert_eq!(write_result, Decision::Deny(DenyReason::OperationNotGranted));
}
