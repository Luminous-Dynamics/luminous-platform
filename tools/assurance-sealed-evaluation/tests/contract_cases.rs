use serde::Deserialize;
use std::collections::BTreeSet;

const CASES: &str = include_str!("../fixtures/sealed-evaluation-cases-0.1.json");

#[derive(Debug, Deserialize)]
struct CasePack {
    schema: String,
    status: String,
    claim_ceiling: String,
    case_ids: Vec<String>,
}

#[test]
fn public_mechanics_pack_has_exact_nonempty_discovery() {
    let pack: CasePack = serde_json::from_str(CASES).expect("case pack JSON");
    assert_eq!(pack.schema, "luminous.sealed-evaluation-contract-cases/0.1");
    assert_eq!(pack.status, "PUBLIC_SYNTHETIC_VERIFIER_MECHANICS_ONLY");
    assert!(!pack.claim_ceiling.trim().is_empty());
    let expected = vec![
        "SEAL-001-valid-reveal".to_owned(),
        "SEAL-002-private-byte-mutation".to_owned(),
        "SEAL-003-salt-mutation".to_owned(),
        "SEAL-004-policy-substitution".to_owned(),
        "SEAL-005-suite-substitution".to_owned(),
        "SEAL-006-commit-after-start".to_owned(),
        "SEAL-007-reveal-before-lock".to_owned(),
        "SEAL-008-missing-monitor-receipt".to_owned(),
        "SEAL-009-incomplete-monitor-coverage".to_owned(),
        "SEAL-010-sequence-gap".to_owned(),
        "SEAL-011-blocked-private-access-attempt".to_owned(),
        "SEAL-012-successful-grader-access".to_owned(),
        "SEAL-013-public-manifest-private-field".to_owned(),
    ];
    assert_eq!(pack.case_ids, expected);
    let unique: BTreeSet<&str> = pack.case_ids.iter().map(String::as_str).collect();
    assert_eq!(unique.len(), pack.case_ids.len());
}
