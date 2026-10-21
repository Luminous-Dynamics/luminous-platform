use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use coset::{cbor::value::Value, iana, CborSerializable, CoseSign1Builder, HeaderBuilder};
use ed25519_dalek::{Signer, SigningKey};
use srq_boot_reconciliation_model::{
    reconcile, BootObservation, BootObservationChallenge, CommittedPromotion, PromotionIntent,
    Reconciliation, ReconciliationFault,
};
use srq_d1_sqlite_store::{CommittedD1Observation, IssuedChallenge, SqliteD1Store};
use srq_eat_cose_verifier::{
    hex_encode, verify_and_consume, ExpectedChallenge, TrustedKey, PROFILE as D1_PROFILE,
};
use srq_policy_promotion_store::{
    InitialTargetState, IssuedPromotionChallenge, SqlitePolicyPromotionStore, TrustedPromotionKey,
};

const TARGET: &str = "target-token-001";
const AUDIENCE: &str = "srq-cross-layer-integration";
const DECISION: &str = "decision-cross-layer-001";
const PROMOTION_CHALLENGE: &str = "promotion-challenge-001";
const BOOT_CHALLENGE: &str = "boot-observation-001";
const POLICY_KEY_ID: &[u8] = b"srq-policy-key-01";
const BOOT_KEY_ID: &[u8] = b"srq-boot-key-01";
const PREVIOUS_DEPLOYMENT: &str =
    "/nix/store/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-nixos-system-previous";
const NEXT_DEPLOYMENT: &str =
    "/nix/store/0123456789abcdfghijklmnpqrsvwxyz-nixos-system-cross-layer";
const PREVIOUS_APPROVED: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const PREVIOUS_EFFECTIVE: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const NEXT_ARTIFACT: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const NEXT_EFFECTIVE: &str = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
const CORPUS: &str = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
const POLICY_NONCE: &[u8] = b"cross-layer-policy-nonce-00000001";
const BOOT_NONCE: &[u8] = b"cross-layer-boot-nonce-0000000001";
const BOOT_INSTANCE: &str = "boot-instance-cross-layer-0042";

fn now_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock after Unix epoch")
        .as_secs()
}

fn db_path(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "srq-cross-layer-{label}-{}-{}.sqlite",
        std::process::id(),
        now_seconds()
    ))
}

fn remove_db(path: &PathBuf) {
    let _ = fs::remove_file(path);
    let _ = fs::remove_file(format!("{}-wal", path.display()));
    let _ = fs::remove_file(format!("{}-shm", path.display()));
}

fn signed_policy_authorization(
    challenge: &IssuedPromotionChallenge,
    now: u64,
    key: &SigningKey,
) -> Vec<u8> {
    let claims = vec![
        (Value::Integer(10.into()), Value::Bytes(challenge.nonce.clone())),
        (Value::Integer(2.into()), Value::Text(challenge.target_id.clone())),
        (Value::Integer(3.into()), Value::Text(challenge.audience.clone())),
        (Value::Integer(4.into()), Value::Integer(challenge.expires_at_unix_seconds.into())),
        (Value::Integer(6.into()), Value::Integer(now.into())),
        (Value::Text("srq_profile".into()), Value::Text("srq-policy-promotion-v1".into())),
        (Value::Text("srq_challenge_id".into()), Value::Text(challenge.challenge_id.clone())),
        (Value::Text("srq_decision_id".into()), Value::Text(challenge.decision_id.clone())),
        (Value::Text("srq_target_id".into()), Value::Text(challenge.target_id.clone())),
        (Value::Text("srq_previous_generation".into()), Value::Integer(challenge.previous_generation.into())),
        (Value::Text("srq_next_generation".into()), Value::Integer(challenge.next_generation.into())),
        (Value::Text("srq_previous_approved_artifact_sha256".into()), Value::Text(challenge.previous_approved_artifact_sha256.clone())),
        (Value::Text("srq_previous_effective_policy_sha256".into()), Value::Text(challenge.previous_effective_policy_sha256.clone())),
        (Value::Text("srq_approved_artifact_sha256".into()), Value::Text(challenge.approved_artifact_sha256.clone())),
        (Value::Text("srq_next_effective_policy_sha256".into()), Value::Text(challenge.next_effective_policy_sha256.clone())),
        (Value::Text("srq_corpus_sha256".into()), Value::Text(challenge.corpus_sha256.clone())),
        (Value::Text("srq_deployment_id".into()), Value::Text(challenge.deployment_id.clone())),
        (Value::Text("srq_verifier_id".into()), Value::Text(challenge.audience.clone())),
    ];
    let mut payload = Vec::new();
    coset::cbor::ser::into_writer(&Value::Map(claims), &mut payload)
        .expect("serialize promotion authorization claims");
    CoseSign1Builder::new()
        .protected(
            HeaderBuilder::new()
                .algorithm(iana::Algorithm::Ed25519)
                .key_id(POLICY_KEY_ID.to_vec())
                .build(),
        )
        .payload(payload)
        .create_signature(&[], |signed| key.sign(signed).to_bytes().to_vec())
        .build()
        .to_vec()
        .expect("encode signed promotion authorization")
}

fn signed_boot_observation(
    now: u64,
    expires: u64,
    key: &SigningKey,
) -> Vec<u8> {
    let claims = vec![
        (Value::Integer(10.into()), Value::Bytes(BOOT_NONCE.to_vec())),
        (Value::Integer(2.into()), Value::Text(TARGET.into())),
        (Value::Integer(3.into()), Value::Text(AUDIENCE.into())),
        (Value::Integer(4.into()), Value::Integer(expires.into())),
        (Value::Integer(6.into()), Value::Integer(now.into())),
        (Value::Text("srq_profile".into()), Value::Text(D1_PROFILE.into())),
        (Value::Text("srq_challenge_id".into()), Value::Text(BOOT_CHALLENGE.into())),
        (Value::Text("srq_decision_id".into()), Value::Text(DECISION.into())),
        (Value::Text("srq_target_token".into()), Value::Text(TARGET.into())),
        (Value::Text("srq_policy_generation".into()), Value::Integer(42_u64.into())),
        (Value::Text("srq_approved_artifact_sha256".into()), Value::Text(NEXT_ARTIFACT.into())),
        (Value::Text("srq_expected_effective_policy_sha256".into()), Value::Text(NEXT_EFFECTIVE.into())),
        (Value::Text("srq_observed_effective_policy_sha256".into()), Value::Text(NEXT_EFFECTIVE.into())),
        (Value::Text("srq_corpus_sha256".into()), Value::Text(CORPUS.into())),
        (Value::Text("srq_boot_instance_id".into()), Value::Text(BOOT_INSTANCE.into())),
        (Value::Text("srq_expected_deployment_id".into()), Value::Text(NEXT_DEPLOYMENT.into())),
        (Value::Text("srq_booted_deployment_id".into()), Value::Text(NEXT_DEPLOYMENT.into())),
        (Value::Text("srq_current_userspace_deployment_id".into()), Value::Text(NEXT_DEPLOYMENT.into())),
        (Value::Text("srq_observation_status".into()), Value::Text("consistent".into())),
        (Value::Text("srq_assurance_tier".into()), Value::Text("D1-software-key".into())),
        (Value::Text("srq_measurement_profile".into()), Value::Null),
        (Value::Text("srq_measurement_digest".into()), Value::Null),
    ];
    let mut payload = Vec::new();
    coset::cbor::ser::into_writer(&Value::Map(claims), &mut payload)
        .expect("serialize boot observation claims");
    CoseSign1Builder::new()
        .protected(
            HeaderBuilder::new()
                .algorithm(iana::Algorithm::Ed25519)
                .key_id(BOOT_KEY_ID.to_vec())
                .build(),
        )
        .payload(payload)
        .create_signature(&[], |signed| key.sign(signed).to_bytes().to_vec())
        .build()
        .to_vec()
        .expect("encode signed boot observation")
}

fn run_real_lifecycle() -> (
    PromotionIntent,
    CommittedPromotion,
    BootObservationChallenge,
    BootObservation,
    PathBuf,
    PathBuf,
) {
    let now = now_seconds();
    let expires = now + 3_600;
    let policy_key = SigningKey::from_bytes(&[17_u8; 32]);
    let boot_key = SigningKey::from_bytes(&[23_u8; 32]);
    let policy_path = db_path("policy");
    let boot_path = db_path("boot");

    let mut policy_store = SqlitePolicyPromotionStore::open(&policy_path)
        .expect("open actual policy-promotion SQLite store");
    policy_store.initialize_target(&InitialTargetState {
        target_id: TARGET.into(),
        generation: 41,
        approved_artifact_sha256: PREVIOUS_APPROVED.into(),
        effective_policy_sha256: PREVIOUS_EFFECTIVE.into(),
        corpus_sha256: CORPUS.into(),
        deployment_id: PREVIOUS_DEPLOYMENT.into(),
    }).expect("initialize policy target");
    let promotion = IssuedPromotionChallenge {
        challenge_id: PROMOTION_CHALLENGE.into(),
        target_id: TARGET.into(),
        audience: AUDIENCE.into(),
        decision_id: DECISION.into(),
        previous_generation: 41,
        next_generation: 42,
        previous_approved_artifact_sha256: PREVIOUS_APPROVED.into(),
        previous_effective_policy_sha256: PREVIOUS_EFFECTIVE.into(),
        approved_artifact_sha256: NEXT_ARTIFACT.into(),
        next_effective_policy_sha256: NEXT_EFFECTIVE.into(),
        corpus_sha256: CORPUS.into(),
        deployment_id: NEXT_DEPLOYMENT.into(),
        expected_signer_key_id_hex: hex_encode(POLICY_KEY_ID),
        nonce: POLICY_NONCE.to_vec(),
        issued_at_unix_seconds: now - 1,
        expires_at_unix_seconds: expires,
    };
    policy_store.issue_challenge(&promotion).expect("issue policy promotion challenge");
    let policy_trust = TrustedPromotionKey {
        key_id: POLICY_KEY_ID.to_vec(),
        public_key: policy_key.verifying_key().to_bytes(),
        target_id: TARGET.into(),
        audience: AUDIENCE.into(),
    };
    let policy_token = signed_policy_authorization(&promotion, now, &policy_key);
    policy_store.verify_and_promote(&policy_token, &policy_trust)
        .expect("actual signed policy promotion");
    let committed = policy_store.committed_decision_for_challenge(PROMOTION_CHALLENGE)
        .expect("read committed decision")
        .expect("promotion must reconcile after commit");
    policy_store.integrity_check().expect("policy store integrity");

    let mut boot_store = SqliteD1Store::open(&boot_path)
        .expect("open actual D1 boot-observation SQLite store");
    let issued_boot = IssuedChallenge {
        challenge_id: BOOT_CHALLENGE.into(),
        target_token: TARGET.into(),
        audience: AUDIENCE.into(),
        decision_id: DECISION.into(),
        policy_generation: 42,
        approved_artifact_sha256: NEXT_ARTIFACT.into(),
        effective_policy_sha256: NEXT_EFFECTIVE.into(),
        corpus_sha256: CORPUS.into(),
        expected_deployment_id: NEXT_DEPLOYMENT.into(),
        expected_signer_key_id_hex: hex_encode(BOOT_KEY_ID),
        nonce: BOOT_NONCE.to_vec(),
        issued_at_unix_seconds: now - 1,
        expires_at_unix_seconds: expires,
    };
    boot_store.issue_challenge(&issued_boot).expect("issue post-commit boot challenge");
    let trusted_boot_key = TrustedKey {
        key_id: BOOT_KEY_ID.to_vec(),
        public_key: boot_key.verifying_key().to_bytes(),
        target_token: TARGET.into(),
        audience: AUDIENCE.into(),
    };
    let expected_boot = ExpectedChallenge {
        nonce: BOOT_NONCE.to_vec(),
        challenge_id: BOOT_CHALLENGE.into(),
        decision_id: DECISION.into(),
        target_token: TARGET.into(),
        audience: AUDIENCE.into(),
        policy_generation: 42,
        approved_artifact_sha256: NEXT_ARTIFACT.into(),
        expected_effective_policy_sha256: NEXT_EFFECTIVE.into(),
        corpus_sha256: CORPUS.into(),
        expected_deployment_id: NEXT_DEPLOYMENT.into(),
        issued_at_unix_seconds: now - 1,
        expires_at_unix_seconds: expires,
        max_attester_clock_skew_seconds: 60,
    };
    let boot_token = signed_boot_observation(now, expires, &boot_key);
    verify_and_consume(&boot_token, &trusted_boot_key, &expected_boot, now, &mut boot_store)
        .expect("verify and durably consume actual signed boot observation");
    let receipt = boot_store.committed_observation_for_challenge(BOOT_CHALLENGE)
        .expect("read back committed observation")
        .expect("persisted boot observation must be present");
    boot_store.integrity_check().expect("boot store integrity");

    let intent = PromotionIntent {
        decision_id: committed.decision_id.clone(),
        target_id: committed.target_id.clone(),
        promotion_challenge_id: committed.challenge_id.clone(),
        deployment_id: committed.deployment_id.clone(),
        previous_generation: committed.previous_generation,
        next_generation: committed.next_generation,
        // The reconciler's older schema has both approved_policy_digest and
        // artifact_digest; the policy store's approved artifact digest fills
        // both fields until those schemas are normalized.
        approved_policy_digest: committed.approved_artifact_sha256.clone(),
        effective_policy_digest: committed.effective_policy_sha256.clone(),
        artifact_digest: committed.approved_artifact_sha256.clone(),
        corpus_digest: committed.corpus_sha256.clone(),
    };
    let durable = CommittedPromotion {
        decision_id: committed.decision_id.clone(),
        target_id: committed.target_id.clone(),
        consumed_challenge_id: committed.challenge_id.clone(),
        deployment_id: committed.deployment_id.clone(),
        generation: committed.next_generation,
        approved_policy_digest: committed.approved_artifact_sha256.clone(),
        effective_policy_digest: committed.effective_policy_sha256.clone(),
        artifact_digest: committed.approved_artifact_sha256.clone(),
        corpus_digest: committed.corpus_sha256.clone(),
    };
    let challenge = BootObservationChallenge {
        challenge_id: receipt.challenge_id.clone(),
        target_id: receipt.target_token.clone(),
        decision_id: receipt.decision_id.clone(),
        expected_generation: receipt.policy_generation,
        expected_deployment_id: receipt.deployment_id.clone(),
        expected_effective_policy_digest: receipt.effective_policy_sha256.clone(),
    };
    let boot = boot_record(&receipt);

    drop(boot_store);
    drop(policy_store);
    (intent, durable, challenge, boot, policy_path, boot_path)
}

fn boot_record(receipt: &CommittedD1Observation) -> BootObservation {
    BootObservation {
        target_id: receipt.target_token.clone(),
        decision_id: receipt.decision_id.clone(),
        observation_challenge_id: receipt.challenge_id.clone(),
        boot_instance_id: receipt.boot_instance_id.clone(),
        booted_deployment_id: receipt.deployment_id.clone(),
        current_userspace_deployment_id: receipt.deployment_id.clone(),
        generation: receipt.policy_generation,
        effective_policy_digest: receipt.effective_policy_sha256.clone(),
        artifact_digest: receipt.approved_artifact_sha256.clone(),
        // This is the signed envelope digest, not a hardware measurement.
        measurement_digest: receipt.envelope_sha256.clone(),
    }
}

#[test]
fn actual_signed_policy_decision_and_d1_boot_receipt_reconcile_after_database_reopen() {
    let (intent, durable, challenge, boot, policy_path, boot_path) = run_real_lifecycle();
    assert_eq!(
        reconcile(&intent, Some(&durable), Some(&challenge), Some(&boot)),
        Reconciliation::Consistent
    );

    let mut old_generation = boot.clone();
    old_generation.generation = intent.previous_generation;
    assert!(matches!(
        reconcile(&intent, Some(&durable), Some(&challenge), Some(&old_generation)),
        Reconciliation::Blocked(faults)
            if faults.contains(&ReconciliationFault::BootGenerationMismatch)
    ));

    let mut stale_deployment = boot.clone();
    stale_deployment.booted_deployment_id = PREVIOUS_DEPLOYMENT.into();
    assert!(matches!(
        reconcile(&intent, Some(&durable), Some(&challenge), Some(&stale_deployment)),
        Reconciliation::Blocked(faults)
            if faults.contains(&ReconciliationFault::BootedDeploymentMismatch)
    ));

    let mut wrong_decision = durable.clone();
    wrong_decision.decision_id = "decision-other".into();
    assert!(matches!(
        reconcile(&intent, Some(&wrong_decision), Some(&challenge), Some(&boot)),
        Reconciliation::Blocked(faults)
            if faults.contains(&ReconciliationFault::DurableDecisionIdMismatch)
    ));

    let mut wrong_policy = boot.clone();
    wrong_policy.effective_policy_digest = PREVIOUS_EFFECTIVE.into();
    assert!(matches!(
        reconcile(&intent, Some(&durable), Some(&challenge), Some(&wrong_policy)),
        Reconciliation::Blocked(faults)
            if faults.contains(&ReconciliationFault::BootPolicyMismatch)
    ));

    println!("PASS: actual signed policy promotion and persisted COSE D1 boot receipt reconcile; generation/deployment/decision/policy mismatches fail closed");
    remove_db(&policy_path);
    remove_db(&boot_path);
}
