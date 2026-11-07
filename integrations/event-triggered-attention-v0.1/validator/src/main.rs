use chrono::{DateTime, FixedOffset};
use serde_json::Value;

const WATCH_SCHEMA: &str = include_str!("../../watch-request.schema.json");
const RESULT_SCHEMA: &str = include_str!("../../watch-result.schema.json");
const WATCH_POSITIVE: &str = include_str!("../../fixtures/watch-positive.json");
const WATCH_QUIET: &str = include_str!("../../fixtures/watch-quiet-expired.json");
const WATCH_REVOKED: &str = include_str!("../../fixtures/watch-revoked-dormancy.json");
const WATCH_GAP: &str = include_str!("../../fixtures/watch-delivery-gap.json");
const WATCH_PURPOSE: &str = include_str!("../../fixtures/watch-purpose-mismatch.json");
const WATCH_EXECUTABLE: &str =
    include_str!("../../fixtures/watch-executable-callback-rejected.json");
const RESULT_MATCHED: &str = include_str!("../../fixtures/result-event-matched.json");
const RESULT_QUIET: &str = include_str!("../../fixtures/result-quiet-expired.json");
const RESULT_REVOKED: &str = include_str!("../../fixtures/result-revoked-dormancy.json");
const RESULT_GAP: &str = include_str!("../../fixtures/result-delivery-gap.json");

fn parse(raw: &str, label: &str) -> Result<Value, String> {
    serde_json::from_str(raw).map_err(|error| format!("{label}: invalid JSON: {error}"))
}

fn validate(raw: &str, schema_raw: &str, label: &str) -> Result<(), String> {
    let instance = parse(raw, label)?;
    let schema = parse(schema_raw, &format!("{label} schema"))?;
    jsonschema::draft202012::meta::validate(&schema)
        .map_err(|error| format!("{label}: invalid Draft 2020-12 schema: {error}"))?;
    let validator = jsonschema::draft202012::options()
        .should_validate_formats(true)
        .build(&schema)
        .map_err(|error| format!("{label}: schema compile failed: {error}"))?;
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

fn at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    path.iter()
        .try_fold(value, |current, key| current.get(*key))
}

fn string_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    at(value, path)?.as_str()
}

fn number_at(value: &Value, path: &[&str]) -> Option<u64> {
    at(value, path)?.as_u64()
}

fn time_at(
    value: &Value,
    path: &[&str],
    label: &str,
) -> Result<Option<DateTime<FixedOffset>>, String> {
    let Some(raw) = string_at(value, path) else {
        return Ok(None);
    };
    DateTime::parse_from_rfc3339(raw)
        .map(Some)
        .map_err(|error| {
            format!(
                "{label}: invalid RFC3339 timestamp at {}: {error}",
                path.join(".")
            )
        })
}

fn required_time(
    value: &Value,
    path: &[&str],
    label: &str,
) -> Result<DateTime<FixedOffset>, String> {
    time_at(value, path, label)?
        .ok_or_else(|| format!("{label}: missing timestamp at {}", path.join(".")))
}

fn audit_watch(raw: &str, label: &str) -> Result<(), String> {
    validate(raw, WATCH_SCHEMA, label)?;
    let watch = parse(raw, label)?;

    if string_at(&watch, &["task_binding", "purpose_ref"])
        != string_at(&watch, &["policy", "requested_purpose_ref"])
    {
        return Err(format!(
            "{label}: requested watch purpose differs from principal-bound purpose"
        ));
    }

    let total_wakes = number_at(&watch, &["resource_budget", "max_wakeups"])
        .ok_or_else(|| format!("{label}: missing total wake budget"))?;
    let llm_wakes = number_at(&watch, &["resource_budget", "max_llm_wakeups"])
        .ok_or_else(|| format!("{label}: missing LLM wake budget"))?;
    if llm_wakes > total_wakes {
        return Err(format!(
            "{label}: LLM wake budget exceeds total wake budget"
        ));
    }

    let starts = required_time(&watch, &["policy", "starts_at"], label)?;
    let expires = required_time(&watch, &["policy", "expires_at"], label)?;
    let capability_expires = required_time(
        &watch,
        &["authority_context", "capability_expires_at"],
        label,
    )?;
    if starts >= expires {
        return Err(format!("{label}: watch start must precede watch expiry"));
    }
    if expires > capability_expires {
        return Err(format!(
            "{label}: watch expiry exceeds canonical capability expiry"
        ));
    }
    Ok(())
}

fn audit_result(raw: &str, label: &str) -> Result<(), String> {
    validate(raw, RESULT_SCHEMA, label)?;
    let result = parse(raw, label)?;
    let admission = string_at(&result, &["admission", "outcome"]);
    let authority = string_at(&result, &["admission", "authority_status"]);
    let budget = string_at(&result, &["admission", "budget_status"]);
    let status = string_at(&result, &["watch_state", "status"]);
    let terminal_authority = string_at(&result, &["watch_state", "terminal_authority_status"]);
    let delivery = string_at(&result, &["event_delivery", "status"]);
    let disposition = string_at(&result, &["wake", "disposition"]);
    let reason = string_at(&result, &["wake", "reason"]);
    let event_count = number_at(&result, &["event_delivery", "event_count"]).unwrap_or(u64::MAX);
    let wake_count = number_at(&result, &["wake", "wake_count"]).unwrap_or(u64::MAX);
    let mutations = number_at(
        &result,
        &["effect_boundary", "consequential_mutation_attempts"],
    )
    .unwrap_or(u64::MAX);

    if mutations != 0 {
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
        let before = number_at(&result, &["admission", "budget_version_before"])
            .ok_or_else(|| format!("{label}: committed budget lacks before-version"))?;
        let after = number_at(&result, &["admission", "budget_version_after"])
            .ok_or_else(|| format!("{label}: committed budget lacks after-version"))?;
        if before.checked_add(1) != Some(after) {
            return Err(format!(
                "{label}: one watch reservation must advance the budget version exactly once"
            ));
        }
    } else if matches!(admission, Some("denied" | "indeterminate")) {
        if status != Some("not_started")
            || delivery != Some("none")
            || disposition != Some("no_wake")
            || wake_count != 0
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
            || event_count == 0
            || wake_count == 0
        {
            return Err(format!(
                "{label}: matched event is not bound to condition result and wake"
            ));
        }
        let first = number_at(&result, &["event_delivery", "first_sequence"]);
        let last = number_at(&result, &["event_delivery", "last_sequence"]);
        if !matches!((first, last), (Some(first), Some(last)) if first <= last) {
            return Err(format!(
                "{label}: matched event sequence range is absent or reversed"
            ));
        }
    }

    if delivery == Some("delivery_gap") {
        if status != Some("delivery_gap")
            || disposition != Some("escalate")
            || reason != Some("delivery_integrity_failure")
        {
            return Err(format!(
                "{label}: delivery gap must remain visible and trigger escalation"
            ));
        }
        let before = string_at(&result, &["event_delivery", "cursor_before"]);
        let after = string_at(&result, &["event_delivery", "cursor_after"]);
        if before.is_none() || after.is_none() || before == after {
            return Err(format!(
                "{label}: delivery gap lacks distinct before/after cursors"
            ));
        }
    }

    if matches!(terminal_authority, Some("revoked" | "expired" | "missing")) {
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
        let changed = required_time(&result, &["watch_state", "authority_changed_at"], label)?;
        let access = required_time(&result, &["watch_state", "last_data_access_at"], label)?;
        if access >= changed {
            return Err(format!(
                "{label}: data access is not strictly before authority loss"
            ));
        }
        if delivery != Some("none") {
            return Err(format!(
                "{label}: delivery after terminal authority loss is not permitted"
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

fn audit_pair(watch_raw: &str, result_raw: &str, label: &str) -> Result<(), String> {
    audit_watch(watch_raw, &format!("{label} watch"))?;
    audit_result(result_raw, &format!("{label} result"))?;
    let watch = parse(watch_raw, label)?;
    let result = parse(result_raw, label)?;

    if string_at(&watch, &["watch_id"]) != string_at(&result, &["watch_id"]) {
        return Err(format!(
            "{label}: result watch ID does not bind the submitted watch"
        ));
    }

    if string_at(&result, &["admission", "outcome"]) == Some("admitted")
        && number_at(&watch, &["authority_context", "authority_epoch"])
            != number_at(&result, &["admission", "authority_epoch"])
    {
        return Err(format!(
            "{label}: admission authority epoch differs from watch request"
        ));
    }

    let delivery = string_at(&result, &["event_delivery", "status"]);
    if matches!(delivery, Some("matched" | "delivery_gap"))
        && string_at(&watch, &["source", "control_domain_ref"])
            != string_at(&result, &["event_delivery", "source_control_domain_ref"])
    {
        return Err(format!(
            "{label}: event source control domain differs from watch source"
        ));
    }

    let max_wakeups = number_at(&watch, &["resource_budget", "max_wakeups"])
        .ok_or_else(|| format!("{label}: missing maximum wake budget"))?;
    let actual_wakeups = number_at(&result, &["wake", "wake_count"])
        .ok_or_else(|| format!("{label}: missing actual wake count"))?;
    if actual_wakeups > max_wakeups {
        return Err(format!(
            "{label}: actual wake count exceeds requested budget"
        ));
    }

    if string_at(&result, &["watch_state", "status"]) != Some("not_started") {
        let request_start = required_time(&watch, &["policy", "starts_at"], label)?;
        let request_expiry = required_time(&watch, &["policy", "expires_at"], label)?;
        let capability_expiry = required_time(
            &watch,
            &["authority_context", "capability_expires_at"],
            label,
        )?;
        let started = required_time(&result, &["watch_state", "started_at"], label)?;
        let terminal = required_time(&result, &["watch_state", "terminal_at"], label)?;
        if started < request_start || started >= request_expiry {
            return Err(format!(
                "{label}: watch started outside its authorized time window"
            ));
        }
        if terminal < started || terminal > request_expiry {
            return Err(format!(
                "{label}: terminal state lies outside the authorized watch window"
            ));
        }
        if terminal > capability_expiry {
            return Err(format!("{label}: watch continued beyond capability expiry"));
        }
        if let Some(access) = time_at(&result, &["watch_state", "last_data_access_at"], label)?
            && (access < started || access > terminal || access > capability_expiry)
        {
            return Err(format!(
                "{label}: data access lies outside the authorized watch window"
            ));
        }
    }

    if delivery == Some("matched") {
        let start = required_time(&watch, &["policy", "starts_at"], label)?;
        let expiry = required_time(&watch, &["policy", "expires_at"], label)?;
        let observed = required_time(
            &result,
            &["event_delivery", "last_event_observed_at"],
            label,
        )?;
        let ingested = required_time(
            &result,
            &["event_delivery", "last_event_ingested_at"],
            label,
        )?;
        let wake = required_time(&result, &["wake", "last_wakeup_at"], label)?;
        if observed > expiry || ingested > expiry || wake > expiry {
            return Err(format!(
                "{label}: matched event or wake occurred after watch expiry"
            ));
        }
        if observed < start && string_at(&watch, &["condition", "persistence"]) == Some("ephemeral")
        {
            return Err(format!(
                "{label}: ephemeral event predates the active watch"
            ));
        }
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

    audit_pair(WATCH_POSITIVE, RESULT_MATCHED, "matched event")?;
    audit_pair(WATCH_QUIET, RESULT_QUIET, "quiet expiry")?;
    audit_pair(WATCH_REVOKED, RESULT_REVOKED, "revocation during dormancy")?;
    audit_pair(WATCH_GAP, RESULT_GAP, "delivery gap")?;
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
    fn all_paired_watch_and_result_fixtures_match_expected_dispositions() {
        run_all().expect("paired watch/result conformance fixtures");
    }

    #[test]
    fn principal_purpose_mismatch_is_structural_but_unauthorized() {
        assert!(validate(WATCH_PURPOSE, WATCH_SCHEMA, "purpose").is_ok());
        assert!(audit_watch(WATCH_PURPOSE, "purpose").is_err());
    }

    #[test]
    fn authority_revocation_comparison_is_timezone_aware() {
        let mut result = parse(RESULT_REVOKED, "revoked").unwrap();
        result["watch_state"]["last_data_access_at"] =
            Value::String("2026-11-07T14:18:59+02:00".into());
        let valid = serde_json::to_string(&result).unwrap();
        assert!(audit_result(&valid, "offset-equivalent-access").is_ok());

        result["watch_state"]["last_data_access_at"] =
            Value::String("2026-11-07T14:19:01+02:00".into());
        let invalid = serde_json::to_string(&result).unwrap();
        assert!(audit_result(&invalid, "post-revocation-access").is_err());
    }

    #[test]
    fn watch_and_result_must_match_identity_epoch_source_and_time_window() {
        let mut result = parse(RESULT_MATCHED, "matched").unwrap();
        result["watch_id"] = Value::String("watch-substituted".into());
        assert!(
            audit_pair(
                WATCH_POSITIVE,
                &serde_json::to_string(&result).unwrap(),
                "id"
            )
            .is_err()
        );

        result = parse(RESULT_MATCHED, "matched").unwrap();
        result["admission"]["authority_epoch"] = Value::from(8);
        assert!(
            audit_pair(
                WATCH_POSITIVE,
                &serde_json::to_string(&result).unwrap(),
                "epoch"
            )
            .is_err()
        );

        result = parse(RESULT_MATCHED, "matched").unwrap();
        result["event_delivery"]["source_control_domain_ref"] =
            Value::String("source-domain-substituted".into());
        assert!(
            audit_pair(
                WATCH_POSITIVE,
                &serde_json::to_string(&result).unwrap(),
                "source"
            )
            .is_err()
        );

        result = parse(RESULT_MATCHED, "matched").unwrap();
        result["wake"]["wake_count"] = Value::from(9);
        assert!(
            audit_pair(
                WATCH_POSITIVE,
                &serde_json::to_string(&result).unwrap(),
                "wake-budget"
            )
            .is_err()
        );

        result = parse(RESULT_MATCHED, "matched").unwrap();
        result["watch_state"]["terminal_at"] = Value::String("2026-11-07T13:00:01Z".into());
        assert!(
            audit_pair(
                WATCH_POSITIVE,
                &serde_json::to_string(&result).unwrap(),
                "expired"
            )
            .is_err()
        );
    }

    #[test]
    fn delivery_gap_cannot_wake_agent_as_if_matched() {
        assert!(audit_pair(WATCH_GAP, RESULT_GAP, "gap").is_ok());
        let mut result = parse(RESULT_GAP, "gap").unwrap();
        result["wake"]["disposition"] = Value::String("wake_agent".into());
        assert!(
            audit_pair(
                WATCH_GAP,
                &serde_json::to_string(&result).unwrap(),
                "gap-wake"
            )
            .is_err()
        );
    }

    #[test]
    fn watch_event_never_authorizes_mutation() {
        let mut result = parse(RESULT_MATCHED, "matched").unwrap();
        result["effect_boundary"]["consequential_mutation_attempts"] = Value::from(1);
        assert!(
            audit_pair(
                WATCH_POSITIVE,
                &serde_json::to_string(&result).unwrap(),
                "mutation"
            )
            .is_err()
        );
    }

    #[test]
    fn watch_interval_must_fit_inside_capability_with_timezone_offsets() {
        let mut watch = parse(WATCH_POSITIVE, "watch").unwrap();
        watch["policy"]["starts_at"] = Value::String("2026-11-07T12:00:00+02:00".into());
        watch["policy"]["expires_at"] = Value::String("2026-11-07T13:00:00+02:00".into());
        watch["authority_context"]["capability_expires_at"] =
            Value::String("2026-11-07T10:30:00Z".into());
        assert!(audit_watch(&serde_json::to_string(&watch).unwrap(), "offset-expiry").is_err());
    }
}
