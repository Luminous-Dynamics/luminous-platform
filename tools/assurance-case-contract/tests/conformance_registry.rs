use serde::Deserialize;
use std::collections::BTreeSet;

const PACK: &str = include_str!("../fixtures/assurance-case-cases-0.1.json");

#[derive(Deserialize)]
struct CasePack {
    schema: String,
    status: String,
    claim_ceiling: String,
    case_ids: Vec<String>,
}

#[test]
fn public_case_registry_is_exact_and_nonempty() {
    let pack: CasePack = serde_json::from_str(PACK).expect("assurance case registry JSON");
    assert_eq!(
        pack.schema,
        "luminous.assurance-case-public-conformance/0.1"
    );
    assert_eq!(pack.status, "PUBLIC_SYNTHETIC_SOFTWARE_CONFORMANCE");
    assert!(!pack.claim_ceiling.trim().is_empty());
    assert_eq!(pack.case_ids.len(), 25);
    let unique: BTreeSet<&str> = pack.case_ids.iter().map(String::as_str).collect();
    assert_eq!(unique.len(), pack.case_ids.len());
    assert_eq!(
        pack.case_ids.first().map(String::as_str),
        Some("AC-001-complete-supported-claim")
    );
    assert_eq!(
        pack.case_ids.last().map(String::as_str),
        Some("AC-025-closed-contract-unknown-field")
    );
}
