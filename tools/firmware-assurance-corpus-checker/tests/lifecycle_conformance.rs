use firmware_assurance_corpus_checker::lifecycle::{
    evaluate, Decision, LifecycleInput, Method, ObservedState, Stage,
};
use jsonschema::{meta, validator_for};
use serde_json::Value;
use std::collections::BTreeSet;

const CORPUS_TEXT: &str =
    include_str!("../../../docs/fixtures/secure-boot-lifecycle-corpus-v1.json");
const SCHEMA_TEXT: &str =
    include_str!("../../../docs/schemas/secure-boot-lifecycle-corpus-v1.schema.json");

fn corpus() -> Value {
    serde_json::from_str(CORPUS_TEXT).expect("lifecycle corpus JSON parses")
}

#[test]
fn lifecycle_fixture_matches_its_strict_schema() {
    let corpus = corpus();
    let schema: Value = serde_json::from_str(SCHEMA_TEXT).expect("lifecycle schema JSON parses");
    meta::validate(&schema).expect("lifecycle schema meta-validates");
    let validator = validator_for(&schema).expect("lifecycle schema compiles");
    let errors = validator
        .iter_errors(&corpus)
        .map(|error| error.to_string())
        .collect::<Vec<_>>();
    assert!(errors.is_empty(), "lifecycle fixture schema errors: {errors:#?}");
}

#[test]
fn frozen_lifecycle_cases_match_expected_dispositions() {
    let corpus = corpus();
    let cases = corpus["cases"].as_array().expect("cases array exists");
    assert_eq!(corpus["case_count"].as_u64(), Some(cases.len() as u64));
    assert_eq!(cases.len(), 32);

    let mut seen = BTreeSet::new();
    for (index, case) in cases.iter().enumerate() {
        let case_id = case["case_id"].as_str().expect("case ID is a string");
        assert_eq!(case_id, format!("SBL-{:03}", index + 1));
        assert!(seen.insert(case_id.to_string()), "duplicate case {case_id}");
        let input = LifecycleInput::from_json(case).expect("fixture case contains typed inputs");
        assert_eq!(
            evaluate(&input).as_str(),
            case["expected_decision"].as_str().expect("expected decision is a string"),
            "case {case_id}"
        );
    }
}

#[test]
fn unknown_state_never_authorizes_a_transition() {
    let cases = corpus()["cases"].as_array().unwrap().clone();
    for case in cases {
        let mut changed = case.clone();
        changed["observed_state"] = serde_json::json!("unknown");
        changed["expected_decision"] = serde_json::json!("indeterminate");
        let input = LifecycleInput::from_json(&changed).unwrap();
        assert_eq!(evaluate(&input), Decision::Indeterminate, "case {}", case["case_id"]);
    }
}

#[test]
fn verified_secure_boot_requires_every_post_reboot_claim() {
    let cases = corpus()["cases"].as_array().unwrap();
    let verified = cases
        .iter()
        .find(|case| case["expected_decision"] == "secure_boot_verified")
        .expect("corpus includes one successful post-reboot case");
    let baseline = LifecycleInput::from_json(verified).unwrap();
    assert_eq!(evaluate(&baseline), Decision::SecureBootVerified);

    let mut mutations = Vec::new();
    let mut value = baseline.clone();
    value.configuration_prepared = false;
    mutations.push(value);
    let mut value = baseline.clone();
    value.enrollment_attempted = false;
    mutations.push(value);
    let mut value = baseline.clone();
    value.observed_state = ObservedState::SetupMode;
    mutations.push(value);
    let mut value = baseline.clone();
    value.secure_boot_user_mode = false;
    mutations.push(value);
    let mut value = baseline.clone();
    value.expected_boot_target_selected = false;
    mutations.push(value);
    let mut value = baseline.clone();
    value.post_boot_health_observed = false;
    mutations.push(value);
    let mut value = baseline;
    value.recovery_retested = false;
    mutations.push(value);

    for (index, input) in mutations.iter().enumerate() {
        assert_ne!(
            evaluate(input),
            Decision::SecureBootVerified,
            "post-reboot evidence mutation {index} must invalidate verification"
        );
    }
}

#[test]
fn deterministic_state_mutations_preserve_fail_closed_invariants() {
    const STATES: [ObservedState; 4] = [
        ObservedState::Enabled,
        ObservedState::Disabled,
        ObservedState::SetupMode,
        ObservedState::Unknown,
    ];
    const METHODS: [Method; 3] = [Method::SecureBoot, Method::StandardBoot, Method::Unresolved];
    const STAGES: [Stage; 3] = [Stage::Prepare, Stage::Enroll, Stage::VerifyPostReboot];

    let valid = LifecycleInput::from_json(
        corpus()["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|case| case["case_id"] == "SBL-001")
            .unwrap(),
    )
    .unwrap();

    let mut seed = 0x5342_4C31_u32;
    let mut next = |limit: usize| {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        ((seed >> 16) as usize) % limit
    };

    for trial in 0..10_000 {
        let mut input = valid.clone();
        input.observed_state = STATES[next(STATES.len())];
        input.method = METHODS[next(METHODS.len())];
        input.stage = STAGES[next(STAGES.len())];
        input.profile_id_present = next(2) == 1;
        input.version_pins_present = next(2) == 1;
        input.user_confirmed = next(2) == 1;
        input.exact_profile_qualified = next(2) == 1;
        input.recovery_path_verified = next(2) == 1;
        input.hardware_profile_matches = next(2) == 1;
        input.state_matches_approval = next(2) == 1;
        input.pki_bundle_matches = next(2) == 1;
        input.key_generation_enabled = next(2) == 1;
        input.configuration_prepared = next(2) == 1;
        input.signing_key_verified = next(2) == 1;
        input.boot_artifacts_verified = next(2) == 1;
        input.operator_reviewed_command = next(2) == 1;
        input.enrollment_attempted = next(2) == 1;
        input.secure_boot_user_mode = next(2) == 1;
        input.expected_boot_target_selected = next(2) == 1;
        input.post_boot_health_observed = next(2) == 1;
        input.recovery_retested = next(2) == 1;

        let decision = evaluate(&input);
        if input.observed_state == ObservedState::Unknown {
            assert_eq!(decision, Decision::Indeterminate, "trial {trial}");
        }
        if !input.user_confirmed && input.observed_state != ObservedState::Unknown {
            assert_eq!(decision, Decision::Blocked, "trial {trial}: no consent");
        }
        if decision == Decision::SecureBootVerified {
            assert_eq!(input.stage, Stage::VerifyPostReboot, "trial {trial}");
            assert_eq!(input.method, Method::SecureBoot, "trial {trial}");
            assert_eq!(input.observed_state, ObservedState::Enabled, "trial {trial}");
            assert!(input.user_confirmed, "trial {trial}");
            assert!(input.exact_profile_qualified, "trial {trial}");
            assert!(input.recovery_path_verified, "trial {trial}");
            assert!(input.hardware_profile_matches, "trial {trial}");
            assert!(input.state_matches_approval, "trial {trial}");
            assert!(input.pki_bundle_matches, "trial {trial}");
            assert!(input.configuration_prepared, "trial {trial}");
            assert!(input.enrollment_attempted, "trial {trial}");
            assert!(input.secure_boot_user_mode, "trial {trial}");
            assert!(input.expected_boot_target_selected, "trial {trial}");
            assert!(input.post_boot_health_observed, "trial {trial}");
            assert!(input.recovery_retested, "trial {trial}");
        }
        if decision == Decision::ManualEnrollmentReviewAllowed {
            assert_eq!(input.stage, Stage::Enroll, "trial {trial}");
            assert_eq!(input.observed_state, ObservedState::SetupMode, "trial {trial}");
            assert!(input.configuration_prepared, "trial {trial}");
            assert!(input.signing_key_verified, "trial {trial}");
            assert!(input.boot_artifacts_verified, "trial {trial}");
            assert!(input.operator_reviewed_command, "trial {trial}");
            assert!(input.recovery_path_verified, "trial {trial}");
        }
    }
}
