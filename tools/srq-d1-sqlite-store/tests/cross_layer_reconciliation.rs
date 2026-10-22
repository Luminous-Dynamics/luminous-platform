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
    hex_encode, sha256_hex, verify_and_consume, ExpectedChallenge, TrustedKey, PROFILE as D1_PROFILE,
};
use srq_rollback_witness_model::{
    reconcile as reconcile_witness, Checkpoint, LocalState, Reconciliation as WitnessReconciliation,
    Witness,
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
    let unique_nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock after Unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "srq-cross-layer-{label}-{}-{unique_nanos}.sqlite",
        std::process::id(),
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
    PathBuf,
) {
    let now = now_seconds();
    let expires = now + 3_600;
    let policy_key = SigningKey::from_bytes(&[17_u8; 32]);
    let boot_key = SigningKey::from_bytes(&[23_u8; 32]);
    let policy_path = db_path("policy");
    let policy_backup_path = db_path("policy-generation-41-backup");
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
    policy_store.integrity_check().expect("policy store integrity before snapshot backup");
    // Close the only connection so committed WAL pages are checkpointed before
    // making an intentionally stale but internally consistent SQLite snapshot.
    drop(policy_store);
    {
        let snapshot_connection = rusqlite::Connection::open(&policy_path)
            .expect("open policy snapshot for an explicit WAL checkpoint");
        let (busy, log_frames, checkpointed_frames): (i64, i64, i64) = snapshot_connection
            .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .expect("checkpoint generation-41 policy snapshot before copying");
        assert_eq!(busy, 0, "policy snapshot must not have a busy checkpoint");
        assert_eq!(
            checkpointed_frames, log_frames,
            "all WAL frames must be checkpointed before copying the database file"
        );
        drop(snapshot_connection);
        fs::copy(&policy_path, &policy_backup_path)
            .expect("snapshot the generation-41 policy store before promotion");
    }
    let mut policy_store = SqlitePolicyPromotionStore::open(&policy_path)
        .expect("reopen policy store after generation-41 snapshot");

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
    policy_store.integrity_check().expect("policy store integrity before close");
    drop(policy_store);
    let policy_store = SqlitePolicyPromotionStore::open(&policy_path)
        .expect("reopen policy-promotion database after commit");
    let committed = policy_store.committed_decision_for_challenge(PROMOTION_CHALLENGE)
        .expect("read committed decision after reopen")
        .expect("promotion must reconcile after commit and reopen");
    let promoted_state = policy_store.target_state(TARGET)
        .expect("read current policy target after reopen")
        .expect("current policy target must exist");
    assert_eq!(promoted_state.generation, 42);
    assert_eq!(promoted_state.approved_artifact_sha256, NEXT_ARTIFACT);
    assert_eq!(promoted_state.effective_policy_sha256, NEXT_EFFECTIVE);
    assert_eq!(promoted_state.deployment_id, NEXT_DEPLOYMENT);
    policy_store.integrity_check().expect("policy store integrity after reopen");

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
    boot_store.integrity_check().expect("boot store integrity before close");
    drop(boot_store);
    let boot_store = SqliteD1Store::open(&boot_path)
        .expect("reopen D1 observation database after commit");
    let receipt = boot_store.committed_observation_for_challenge(BOOT_CHALLENGE)
        .expect("read back committed observation after reopen")
        .expect("persisted boot observation must be present after reopen");
    boot_store.integrity_check().expect("boot store integrity after reopen");

    let intent = PromotionIntent {
        decision_id: committed.decision_id.clone(),
        target_id: committed.target_id.clone(),
        promotion_challenge_id: committed.challenge_id.clone(),
        deployment_id: committed.deployment_id.clone(),
        previous_generation: committed.previous_generation,
        next_generation: committed.next_generation,
        approved_artifact_digest: committed.approved_artifact_sha256.clone(),
        effective_policy_digest: committed.effective_policy_sha256.clone(),
        corpus_digest: committed.corpus_sha256.clone(),
    };
    let durable = CommittedPromotion {
        decision_id: committed.decision_id.clone(),
        target_id: committed.target_id.clone(),
        consumed_challenge_id: committed.challenge_id.clone(),
        deployment_id: committed.deployment_id.clone(),
        generation: committed.next_generation,
        approved_artifact_digest: committed.approved_artifact_sha256.clone(),
        effective_policy_digest: committed.effective_policy_sha256.clone(),
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
    (
        intent,
        durable,
        challenge,
        boot,
        policy_path,
        boot_path,
        policy_backup_path,
    )
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

fn canonical_witness_checkpoint_digest(checkpoint: &Checkpoint) -> String {
    // Stable test encoding. A production witness protocol must specify and
    // verify its own canonical serialization before accepting a signed receipt.
    let canonical = format!(
        "srq-external-rollback-witness-v1\nschema=1\ntarget_id={}\nsequence={}\nprevious_checkpoint_sha256={}\ndecision_id={}\nchallenge_id={}\nprevious_generation={}\ngeneration={}\ndeployment_id={}\napproved_artifact_sha256={}\neffective_policy_sha256={}\ncorpus_sha256={}\nissued_at_unix_seconds={}\nexpires_at_unix_seconds={}\n",
        checkpoint.target_id,
        checkpoint.sequence,
        checkpoint.previous_checkpoint_sha256,
        checkpoint.decision_id,
        checkpoint.challenge_id,
        checkpoint.previous_generation,
        checkpoint.generation,
        checkpoint.deployment_id,
        checkpoint.approved_artifact_sha256,
        checkpoint.effective_policy_sha256,
        checkpoint.corpus_sha256,
        checkpoint.issued_at_unix_seconds,
        checkpoint.expires_at_unix_seconds,
    );
    sha256_hex(canonical.as_bytes())
}

#[test]
fn actual_signed_policy_decision_and_d1_boot_receipt_reconcile_after_database_reopen() {
    let (intent, durable, challenge, boot, policy_path, boot_path, policy_backup_path) =
        run_real_lifecycle();
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

    // Mutation campaign starts from records read back from both SQLite stores,
    // not from an all-synthetic fixture. Each field is mutated independently.
    for i in 0..10_000_u64 {
        let mut changed_intent = intent.clone();
        let mut changed_durable = durable.clone();
        let mut changed_challenge = challenge.clone();
        let mut changed_boot = boot.clone();
        let token = format!("mut-{i}");
        let digest = format!("{:064x}", i + 1);
        match i % 33 {
            0 => changed_intent.target_id = token,
            1 => changed_intent.decision_id = token,
            2 => changed_intent.promotion_challenge_id = token,
            3 => changed_intent.deployment_id = token,
            4 => changed_intent.previous_generation = u64::MAX,
            5 => changed_intent.next_generation = 43 + i,
            6 => changed_intent.approved_artifact_digest = digest,
            7 => changed_intent.effective_policy_digest = digest,
            8 => changed_intent.corpus_digest = digest,
            9 => changed_durable.decision_id = token,
            10 => changed_durable.target_id = token,
            11 => changed_durable.consumed_challenge_id = token,
            12 => changed_durable.deployment_id = token,
            13 => changed_durable.generation = 1_000 + i,
            14 => changed_durable.approved_artifact_digest = digest,
            15 => changed_durable.effective_policy_digest = digest,
            16 => changed_durable.corpus_digest = digest,
            17 => changed_challenge.challenge_id = token,
            18 => changed_challenge.target_id = token,
            19 => changed_challenge.decision_id = token,
            20 => changed_challenge.expected_generation = 1_000 + i,
            21 => changed_challenge.expected_deployment_id = token,
            22 => changed_challenge.expected_effective_policy_digest = digest,
            23 => changed_boot.target_id = token,
            24 => changed_boot.decision_id = token,
            25 => changed_boot.observation_challenge_id = token,
            26 => changed_boot.booted_deployment_id = token,
            27 => changed_boot.current_userspace_deployment_id = token,
            28 => changed_boot.generation = 1_000 + i,
            29 => changed_boot.effective_policy_digest = digest,
            30 => changed_boot.artifact_digest = digest,
            31 => changed_boot.boot_instance_id = "../invalid-boot".into(),
            _ => changed_boot.measurement_digest = "not-a-digest".into(),
        }
        assert!(
            !reconcile(
                &changed_intent,
                Some(&changed_durable),
                Some(&changed_challenge),
                Some(&changed_boot),
            ).is_consistent(),
            "actual-store mutation {i} unexpectedly reconciled"
        );
    }
    println!("PASS: actual signed policy promotion and persisted COSE D1 boot receipt reconcile; generation/deployment/decision/policy mismatches and 10,000 mutations fail closed");
    remove_db(&policy_path);
    remove_db(&boot_path);
    remove_db(&policy_backup_path);
}

#[test]
fn restoring_actual_sqlite_policy_snapshot_is_detected_by_independent_witness_state() {
    let (intent, durable, _challenge, _boot, policy_path, boot_path, policy_backup_path) =
        run_real_lifecycle();

    let genesis_checkpoint_sha256 = sha256_hex(format!(
        "srq-external-rollback-witness-genesis-v1\ntarget_id={}\ngeneration={}\napproved_artifact_sha256={}\neffective_policy_sha256={}\ncorpus_sha256={}\ndeployment_id={}\n",
        intent.target_id,
        intent.previous_generation,
        PREVIOUS_APPROVED,
        PREVIOUS_EFFECTIVE,
        CORPUS,
        PREVIOUS_DEPLOYMENT,
    ).as_bytes());

    let mut witness = Witness::new();
    let genesis_tip = witness.initialize_target(
        &intent.target_id,
        &genesis_checkpoint_sha256,
        intent.previous_generation,
    ).expect("initialize independent witness at generation 41");
    let mut witness_local = LocalState::from_genesis(&genesis_tip);

    let issued_at = now_seconds();
    let mut checkpoint = Checkpoint {
        target_id: intent.target_id.clone(),
        sequence: 1,
        previous_checkpoint_sha256: genesis_checkpoint_sha256,
        checkpoint_sha256: String::new(),
        decision_id: durable.decision_id.clone(),
        challenge_id: intent.promotion_challenge_id.clone(),
        previous_generation: intent.previous_generation,
        generation: durable.generation,
        deployment_id: durable.deployment_id.clone(),
        approved_artifact_sha256: durable.approved_artifact_digest.clone(),
        effective_policy_sha256: durable.effective_policy_digest.clone(),
        corpus_sha256: durable.corpus_digest.clone(),
        issued_at_unix_seconds: issued_at,
        expires_at_unix_seconds: issued_at + 3_600,
    };
    checkpoint.checkpoint_sha256 = canonical_witness_checkpoint_digest(&checkpoint);

    witness_local.prepare(checkpoint.clone())
        .expect("prepare the checkpoint that binds to durable signed promotion");
    let witness_receipt = witness.advance_at(&checkpoint, issued_at + 1)
        .expect("external witness model advances to committed generation 42");
    witness_local.finalize(&witness_receipt)
        .expect("finalize local checkpoint after witness commit");
    assert_eq!(reconcile_witness(&witness_local, &witness), WitnessReconciliation::Consistent);

    // Remove all live SQLite files, then restore the exact generation-41
    // snapshot made before the real signed policy promotion. This snapshot is
    // internally consistent; integrity_check alone cannot identify rollback.
    remove_db(&policy_path);
    fs::copy(&policy_backup_path, &policy_path)
        .expect("restore the pre-promotion generation-41 SQLite snapshot");
    let restored_store = SqlitePolicyPromotionStore::open(&policy_path)
        .expect("open restored but internally consistent SQLite snapshot");
    let restored = restored_store.target_state(TARGET)
        .expect("read restored target state")
        .expect("restored target must still exist");
    assert_eq!(restored.generation, 41);
    assert_eq!(restored.approved_artifact_sha256, PREVIOUS_APPROVED);
    assert_eq!(restored.effective_policy_sha256, PREVIOUS_EFFECTIVE);
    assert_eq!(restored.deployment_id, PREVIOUS_DEPLOYMENT);
    assert!(
        restored_store.committed_decision_for_challenge(PROMOTION_CHALLENGE)
            .expect("query restored decision ledger")
            .is_none(),
        "pre-promotion snapshot should not contain the later committed decision"
    );
    restored_store.integrity_check()
        .expect("old database remains internally consistent after restore");
    drop(restored_store);

    // The local trust record restored with the database is generation 41 /
    // witness sequence 0. The independent witness still records sequence 1 /
    // generation 42, so reconciliation must detect rollback despite SQLite OK.
    let restored_local_state = LocalState::from_genesis(&genesis_tip);
    assert_eq!(
        reconcile_witness(&restored_local_state, &witness),
        WitnessReconciliation::LocalStateRollbackDetected,
        "a valid old SQLite snapshot must not be accepted as current after a witness advance"
    );
    println!(
        "PASS: restored generation-41 SQLite snapshot passes integrity_check but is detected as rollback against independent witness sequence 1"
    );

    remove_db(&policy_path);
    remove_db(&boot_path);
    remove_db(&policy_backup_path);
}
