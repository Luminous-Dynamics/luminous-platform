use serde_json::Value;

const WATCH_SCHEMA: &str = include_str!("../../watch-request.schema.json");
const RESULT_SCHEMA: &str = include_str!("../../watch-result.schema.json");
const WATCH_POSITIVE: &str = include_str!("../../fixtures/watch-positive.json");
const WATCH_PURPOSE: &str = include_str!("../../fixtures/watch-purpose-mismatch.json");
const WATCH_EXECUTABLE: &str =
    include_str!("../../fixtures/watch-executable-callback-rejected.json");
const RESULT_MATCHED: &str = include_str!("../../fixtures/result-event-matched.json");
const RESULT_QUIET: &str = include_str!("../../fixtures/result-quiet-expired.json");
const RESULT_REVOKED: &str = include_str!("../../fixtures/result-revoked-dormancy.json");
const RESULT_GAP: &str = include_str!("../../fixtures/result-delivery-gap.json");

fn parse(raw: &str, label: &str) -> Result<Value, String> {
    serde_json::from_str(raw).map_err(|e| format!("{label}: invalid JSON: {e}"))
}

fn validate(raw: &str, schema_raw: &str, label: &str) -> Result<(), String> {
    let instance = parse(raw, label)?;
    let schema = parse(schema_raw, &format!("{label} schema"))?;
    jsonschema::draft202012::meta::validate(&schema)
        .map_err(|e| format!("{label}: invalid Draft 2020-12 schema: {e}"))?;
    let validator = jsonschema::draft202012::options()
        .should_validate_formats(true)
        .build(&schema)
        .map_err(|e| format!("{label}: schema compile failed: {e}"))?;
    let errors: Vec<String> = validator
        .iter_errors(&instance)
        .map(|e| format!("{} at {}", e, e.instance_path()))
        .collect();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!("{label}: {}", errors.join("; ")))
    }
}

fn at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    path.iter()
        .try_fold(value, |current, key| current.get(*key))
}
fn s<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    at(value, path)?.as_str()
}
fn n(value: &Value, path: &[&str]) -> Option<u64> {
    at(value, path)?.as_u64()
}

fn audit_watch(raw: &str, label: &str) -> Result<(), String> {
    validate(raw, WATCH_SCHEMA, label)?;
    let v = parse(raw, label)?;
    if s(&v, &["task_binding", "purpose_ref"]) != s(&v, &["policy", "requested_purpose_ref"]) {
        return Err(format!(
            "{label}: requested watch purpose differs from principal-bound purpose"
        ));
    }
    if n(&v, &["resource_budget", "max_llm_wakeups"]).unwrap_or(u64::MAX)
        > n(&v, &["resource_budget", "max_wakeups"]).unwrap_or(0)
    {
        return Err(format!(
            "{label}: LLM wake budget exceeds total wake budget"
        ));
    }
    Ok(())
}

fn audit_result(raw: &str, label: &str) -> Result<(), String> {
    validate(raw, RESULT_SCHEMA, label)?;
    let v = parse(raw, label)?;
    let admission = s(&v, &["admission", "outcome"]);
    let authority = s(&v, &["admission", "authority_status"]);
    let budget = s(&v, &["admission", "budget_status"]);
    let status = s(&v, &["watch_state", "status"]);
    let terminal = s(&v, &["watch_state", "terminal_authority_status"]);
    let delivery = s(&v, &["event_delivery", "status"]);
    let disposition = s(&v, &["wake", "disposition"]);
    let reason = s(&v, &["wake", "reason"]);
    let count = n(&v, &["event_delivery", "event_count"]).unwrap_or(u64::MAX);
    let wakes = n(&v, &["wake", "wake_count"]).unwrap_or(u64::MAX);
    if n(&v, &["effect_boundary", "consequential_mutation_attempts"]) != Some(0) {
        return Err(format!(
            "{label}: watch result reports consequential mutation attempts"
        ));
    }
    if admission == Some("admitted") {
        if authority != Some("current") || budget != Some("committed") {
            return Err(format!(
                "{label}: admitted watch lacks current authority or committed budget"
            ));
        }
    } else if matches!(admission, Some("denied" | "indeterminate")) {
        if status != Some("not_started")
            || delivery != Some("none")
            || disposition != Some("no_wake")
            || wakes != 0
        {
            return Err(format!(
                "{label}: denied/indeterminate watch acquired data or woke an agent"
            ));
        }
        return Ok(());
    } else {
        return Err(format!("{label}: unknown admission outcome"));
    }

    if delivery == Some("matched") {
        if status != Some("condition_met")
            || !matches!(
                disposition,
                Some("wake_agent" | "notify_principal" | "request_human_approval")
            )
            || reason != Some("event_matched")
            || count == 0
            || wakes == 0
        {
            return Err(format!(
                "{label}: matched event is not bound to condition result and wake"
            ));
        }
        let first = n(&v, &["event_delivery", "first_sequence"]);
        let last = n(&v, &["event_delivery", "last_sequence"]);
        if !matches!((first,last),(Some(a),Some(b)) if a<=b) {
            return Err(format!(
                "{label}: matched event sequence range is absent or reversed"
            ));
        }
    }
    if delivery == Some("delivery_gap")
        && (status != Some("delivery_gap")
            || disposition != Some("escalate")
            || reason != Some("delivery_integrity_failure"))
    {
        return Err(format!(
            "{label}: delivery gap must remain visible and trigger escalation"
        ));
    }
    if matches!(terminal, Some("revoked" | "expired" | "missing")) {
        if !matches!(
            status,
            Some("authority_revoked" | "expired" | "indeterminate")
        ) || !matches!(
            disposition,
            Some("cancel" | "escalate" | "expire" | "no_wake")
        ) {
            return Err(format!(
                "{label}: terminal authority loss did not stop the watch"
            ));
        }
        if let (Some(changed), Some(access)) = (
            s(&v, &["watch_state", "authority_changed_at"]),
            s(&v, &["watch_state", "last_data_access_at"]),
        ) {
            if access >= changed {
                return Err(format!(
                    "{label}: data access is not strictly before authority loss"
                ));
            }
        } else if delivery != Some("none") {
            return Err(format!(
                "{label}: event activity with authority loss lacks comparable access times"
            ));
        }
    }
    if matches!(
        delivery,
        Some("none" | "no_match" | "duplicate_only" | "replayed")
    ) && status == Some("condition_met")
    {
        return Err(format!(
            "{label}: no qualifying event may be promoted to condition_met"
        ));
    }
    Ok(())
}

fn run_all() -> Result<(), String> {
    audit_watch(WATCH_POSITIVE, "watch-positive")?;
    validate(WATCH_PURPOSE, WATCH_SCHEMA, "watch-purpose-mismatch")?;
    if audit_watch(WATCH_PURPOSE, "watch-purpose-mismatch").is_ok() {
        return Err("purpose mismatch accepted by semantic audit".into());
    }
    if validate(WATCH_EXECUTABLE, WATCH_SCHEMA, "watch-executable-callback").is_ok() {
        return Err("schema accepted executable condition/callback".into());
    }
    audit_result(RESULT_MATCHED, "result-event-matched")?;
    audit_result(RESULT_QUIET, "result-quiet-expired")?;
    audit_result(RESULT_REVOKED, "result-revoked-dormancy")?;
    audit_result(RESULT_GAP, "result-delivery-gap")?;
    Ok(())
}

fn main() {
    if let Err(error) = run_all() {
        eprintln!("event-triggered-attention validation failed: {error}");
        std::process::exit(1);
    }
    println!("event-triggered-attention v0.1: schema and semantic audit passed");
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_fixtures_match_expected_dispositions() {
        run_all().expect("conformance fixtures");
    }
    #[test]
    fn principal_purpose_mismatch_is_structural_but_unauthorized() {
        assert!(validate(WATCH_PURPOSE, WATCH_SCHEMA, "purpose").is_ok());
        assert!(audit_watch(WATCH_PURPOSE, "purpose").is_err());
    }
    #[test]
    fn revocation_during_dormancy_stops_later_access() {
        assert!(audit_result(RESULT_REVOKED, "revoked").is_ok());
        let mut v = parse(RESULT_REVOKED, "revoked").unwrap();
        v["watch_state"]["last_data_access_at"] = Value::String("2026-11-07T12:19:01Z".into());
        assert!(
            audit_result(
                &serde_json::to_string(&v).unwrap(),
                "post-revocation-access"
            )
            .is_err()
        );
    }
    #[test]
    fn delivery_gap_cannot_wake_agent_as_if_matched() {
        assert!(audit_result(RESULT_GAP, "gap").is_ok());
        let mut v = parse(RESULT_GAP, "gap").unwrap();
        v["wake"]["disposition"] = Value::String("wake_agent".into());
        assert!(audit_result(&serde_json::to_string(&v).unwrap(), "gap-wake").is_err());
    }
    #[test]
    fn watch_event_never_authorizes_mutation() {
        let mut v = parse(RESULT_MATCHED, "matched").unwrap();
        v["effect_boundary"]["consequential_mutation_attempts"] = Value::from(1);
        assert!(audit_result(&serde_json::to_string(&v).unwrap(), "mutation").is_err());
    }
}
