//! Executable conformance checker for the decision-aware probe v0.1 schemas.
//!
//! This validator checks structural Draft 2020-12 conformance and selected
//! cross-record invariants over synthetic fixtures. It does not authenticate
//! Xenia references, execute collectors, prove source independence, or grant
//! any authorization.

use serde_json::Value;

const PROPOSAL_SCHEMA: &str =
    include_str!("../../decision-aware-probe-proposal.schema.json");
const BINDING_SCHEMA: &str =
    include_str!("../../decision-aware-probe-binding.schema.json");

const PROPOSAL_POSITIVE: &str = include_str!("../../fixtures/proposal-positive.json");
const PROPOSAL_WRITE: &str =
    include_str!("../../fixtures/proposal-write-capability-rejected.json");
const PROPOSAL_METRIC: &str =
    include_str!("../../fixtures/proposal-metric-mismatch-rejected.json");
const PROPOSAL_PURPOSE: &str =
    include_str!("../../fixtures/proposal-purpose-mismatch-rejected.json");
const PROPOSAL_SINK: &str =
    include_str!("../../fixtures/proposal-external-sink-rejected.json");

const BINDING_COMPLETED: &str = include_str!("../../fixtures/binding-completed.json");
const BINDING_UNKNOWN: &str = include_str!("../../fixtures/binding-unknown-impact.json");
const BINDING_DENIED: &str = include_str!("../../fixtures/binding-denied.json");
const BINDING_MISSING_IMPACT: &str =
    include_str!("../../fixtures/binding-completed-without-impact-rejected.json");
const BINDING_REVOKED_ADMISSION: &str =
    include_str!("../../fixtures/binding-admitted-revoked-authority-rejected.json");

fn parse_json(raw: &str, label: &str) -> Result<Value, String> {
    serde_json::from_str(raw).map_err(|error| format!("{label}: invalid JSON: {error}"))
}

fn validate_draft2020_12(instance_raw: &str, schema_raw: &str, label: &str) -> Result<(), String> {
    let instance = parse_json(instance_raw, label)?;
    let schema = parse_json(schema_raw, &format!("{label} schema"))?;

    jsonschema::draft202012::meta::validate(&schema)
        .map_err(|error| format!("{label}: invalid Draft 2020-12 schema: {error}"))?;

    let validator = jsonschema::draft202012::options()
        .should_validate_formats(true)
        .build(&schema)
        .map_err(|error| format!("{label}: schema compilation failed: {error}"))?;

    let errors: Vec<String> = validator
        .iter_errors(&instance)
        .map(|error| format!("{} at {}", error, error.instance_path()))
        .collect();

    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!("{label}: {}", errors.join("; ")))
    }
}

fn value_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    path.iter().try_fold(value, |current, key| current.get(*key))
}

fn string_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    value_at(value, path)?.as_str()
}

fn audit_proposal_semantics(raw: &str, label: &str) -> Result<(), String> {
    validate_draft2020_12(raw, PROPOSAL_SCHEMA, label)?;
    let proposal = parse_json(raw, label)?;

    if string_at(&proposal, &["proposed_action"]) != Some("READ_ONLY_PROBE") {
        return Err(format!("{label}: proposal is not a read-only probe"));
    }
    if string_at(&proposal, &["probe", "capability"]) != Some("read_only") {
        return Err(format!("{label}: probe is not explicitly read-only"));
    }
    if string_at(&proposal, &["episode", "purpose_ref"])
        != string_at(&proposal, &["probe", "requested_purpose_ref"])
    {
        return Err(format!("{label}: requested probe purpose differs from principal intent"));
    }
    if string_at(&proposal, &["probe", "result_sink_class"])
        != Some("principal_authorized_session")
    {
        return Err(format!("{label}: result sink is outside the principal-authorized session"));
    }
    if string_at(&proposal, &["episode", "target_ref"])
        != string_at(&proposal, &["probe", "target_ref"])
        || string_at(&proposal, &["episode", "target_digest"])
            != string_at(&proposal, &["probe", "target_digest"])
    {
        return Err(format!("{label}: episode and probe target bindings differ"));
    }
    if string_at(&proposal, &["planner", "method"])
        != string_at(&proposal, &["planner", "predicted_quantity", "metric"])
    {
        return Err(format!("{label}: planner method and predicted metric differ"));
    }
    Ok(())
}

fn audit_binding_semantics(raw: &str, label: &str) -> Result<(), String> {
    validate_draft2020_12(raw, BINDING_SCHEMA, label)?;
    let binding = parse_json(raw, label)?;

    let admission = string_at(&binding, &["admission", "outcome"]);
    let authority = string_at(&binding, &["admission", "authority_context", "status"]);
    let collection = string_at(&binding, &["collection", "status"]);
    let impact = string_at(&binding, &["impact", "status"]);
    let budget = string_at(&binding, &["admission", "budget_reservation", "status"]);

    if admission == Some("admitted") {
        if authority != Some("current") || budget != Some("committed") {
            return Err(format!("{label}: admitted probe lacks current Xenia authority or committed budget"));
        }
        if !matches!(collection, Some("completed" | "failed" | "unknown")) {
            return Err(format!("{label}: admitted probe has impossible collection state"));
        }
        if !matches!(impact, Some("observed" | "pending" | "unknown")) {
            return Err(format!("{label}: admitted probe has impossible impact state"));
        }
    } else if matches!(admission, Some("denied" | "indeterminate")) {
        if collection != Some("not_started") || impact != Some("not_measured") {
            return Err(format!("{label}: denied/indeterminate probe must not execute"));
        }
        if !matches!(budget, Some("not_committed" | "unknown")) {
            return Err(format!("{label}: denied/indeterminate probe has an impossible budget state"));
        }
    } else {
        return Err(format!("{label}: unknown admission outcome"));
    }

    if impact == Some("unknown") && value_at(&binding, &["impact", "impact_receipt_ref"]).is_some() {
        return Err(format!("{label}: unknown impact must not carry a purported impact receipt"));
    }
    if collection == Some("completed")
        && value_at(&binding, &["collection", "observation_receipt_ref"])
            == value_at(&binding, &["impact", "impact_receipt_ref"])
    {
        return Err(format!("{label}: observation and impact receipts must be distinct records"));
    }
    Ok(())
}

fn expect_schema_valid(raw: &str, schema: &str, label: &str) -> Result<(), String> {
    validate_draft2020_12(raw, schema, label)
}

fn expect_schema_invalid(raw: &str, schema: &str, label: &str) -> Result<(), String> {
    match validate_draft2020_12(raw, schema, label) {
        Ok(()) => Err(format!("{label}: expected schema rejection, but fixture was accepted")),
        Err(_) => Ok(()),
    }
}

fn run_all() -> Result<(), String> {
    // Positive proposal: schema and cross-record semantic checks.
    audit_proposal_semantics(PROPOSAL_POSITIVE, "proposal-positive")?;

    // Structurally invalid proposal controls.
    expect_schema_invalid(PROPOSAL_WRITE, PROPOSAL_SCHEMA, "proposal-write-capability")?;
    expect_schema_invalid(PROPOSAL_METRIC, PROPOSAL_SCHEMA, "proposal-metric-mismatch")?;
    expect_schema_invalid(PROPOSAL_SINK, PROPOSAL_SCHEMA, "proposal-external-sink")?;

    // Purpose mismatch is structurally well-formed but semantically unauthorized.
    expect_schema_valid(PROPOSAL_PURPOSE, PROPOSAL_SCHEMA, "proposal-purpose-mismatch")?;
    if audit_proposal_semantics(PROPOSAL_PURPOSE, "proposal-purpose-mismatch").is_ok() {
        return Err("proposal-purpose-mismatch: semantic auditor accepted a purpose mismatch".into());
    }

    // Valid binding dispositions.
    audit_binding_semantics(BINDING_COMPLETED, "binding-completed")?;
    audit_binding_semantics(BINDING_UNKNOWN, "binding-unknown-impact")?;
    audit_binding_semantics(BINDING_DENIED, "binding-denied")?;

    // Structurally contradictory binding controls.
    expect_schema_invalid(
        BINDING_MISSING_IMPACT,
        BINDING_SCHEMA,
        "binding-completed-without-impact",
    )?;
    expect_schema_invalid(
        BINDING_REVOKED_ADMISSION,
        BINDING_SCHEMA,
        "binding-admitted-with-revoked-authority",
    )?;

    Ok(())
}

fn main() {
    if let Err(error) = run_all() {
        eprintln!("decision-aware-probe validation failed: {error}");
        std::process::exit(1);
    }
    println!("decision-aware-probe v0.1: schemas and semantic fixture audit passed");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_probe_contract_schemas_and_fixture_dispositions_qualify() {
        run_all().expect("all positive/negative schema and semantic fixture controls must pass");
    }

    #[test]
    fn denied_or_revoked_authority_never_becomes_admitted() {
        let value = parse_json(BINDING_REVOKED_ADMISSION, "revoked-admission").unwrap();
        assert_eq!(
            string_at(&value, &["admission", "outcome"]),
            Some("admitted")
        );
        assert_eq!(
            string_at(&value, &["admission", "authority_context", "status"]),
            Some("revoked")
        );
        assert!(validate_draft2020_12(
            BINDING_REVOKED_ADMISSION,
            BINDING_SCHEMA,
            "revoked-admission"
        )
        .is_err());
    }

    #[test]
    fn purpose_mismatch_is_structural_but_not_semantically_authorized() {
        assert!(validate_draft2020_12(
            PROPOSAL_PURPOSE,
            PROPOSAL_SCHEMA,
            "purpose-mismatch"
        )
        .is_ok());
        assert!(audit_proposal_semantics(PROPOSAL_PURPOSE, "purpose-mismatch").is_err());
    }

    #[test]
    fn unknown_impact_is_not_zero_and_has_no_receipt() {
        let value = parse_json(BINDING_UNKNOWN, "unknown-impact").unwrap();
        assert_eq!(string_at(&value, &["impact", "status"]), Some("unknown"));
        assert!(value_at(&value, &["impact", "impact_receipt_ref"]).is_none());
        assert!(audit_binding_semantics(BINDING_UNKNOWN, "unknown-impact").is_ok());
    }
}
