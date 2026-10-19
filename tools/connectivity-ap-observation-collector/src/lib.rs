//! Read-only Linux interface observation with aggregate-only output.
//!
//! This module parses bounded diagnostic text. It deliberately has no API to
//! configure devices, generate traffic, change queues, or export identifiers.

#![forbid(unsafe_code)]

use connectivity_ap_testbed_contracts::validate_manifest;
use connectivity_task_outcome_core::sha256_hex;
use serde_json::{Value, json};

const SNAPSHOT_SCHEMA: &str =
    include_str!("../../../schemas/connectivity-ap-observation-snapshot-v1.schema.json");
const MAX_SCHEMA_BYTES: usize = 1_048_576;

fn validate_interface_token(interface: &str) -> Result<(), String> {
    if interface.is_empty()
        || interface.len() > 15
        || !interface
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'-'))
    {
        return Err("interface must match [A-Za-z0-9_.-] and be 1..=15 bytes".to_owned());
    }
    Ok(())
}

fn valid_token(token: &str) -> bool {
    !token.is_empty()
        && token.len() <= 128
        && token
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

fn parse_u64_after_colon(line: &str) -> Option<u64> {
    line.split_once(':')?
        .1
        .split_whitespace()
        .next()?
        .parse::<u64>()
        .ok()
}

fn parse_i64_after_colon(line: &str) -> Option<i64> {
    line.split_once(':')?
        .1
        .split_whitespace()
        .next()?
        .parse::<i64>()
        .ok()
}

fn parse_stations(source: &str) -> Value {
    let mut blocks: Vec<Vec<&str>> = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Station ") {
            blocks.push(Vec::new());
        }
        if let Some(block) = blocks.last_mut() {
            block.push(trimmed);
        }
    }

    let station_count = blocks.len() as u64;
    let sum_required_metric = |label: &str| -> Option<u64> {
        if blocks.is_empty() {
            return Some(0);
        }
        let mut total = 0_u64;
        for block in &blocks {
            let value = block
                .iter()
                .find(|line| line.starts_with(label))
                .and_then(|line| parse_u64_after_colon(line))?;
            total = total.checked_add(value)?;
        }
        Some(total)
    };

    let mut signals = Vec::new();
    for block in &blocks {
        if let Some(value) = block
            .iter()
            .find(|line| line.starts_with("signal:"))
            .and_then(|line| parse_i64_after_colon(line))
            .filter(|value| (-150..=0).contains(value))
        {
            signals.push(value);
        }
    }
    let signal_min = signals.iter().min().copied();
    let signal_max = signals.iter().max().copied();
    let signal_mean_millidbm = if signals.is_empty() {
        None
    } else {
        let sum: i128 = signals.iter().map(|value| i128::from(*value) * 1000).sum();
        Some((sum / signals.len() as i128) as i64)
    };

    json!({
        "station_count": station_count,
        "rx_bytes_total": sum_required_metric("rx bytes:"),
        "tx_bytes_total": sum_required_metric("tx bytes:"),
        "tx_retries_total": sum_required_metric("tx retries:"),
        "tx_failed_total": sum_required_metric("tx failed:"),
        "signal_min_dbm": signal_min,
        "signal_mean_millidbm": signal_mean_millidbm,
        "signal_max_dbm": signal_max,
        "signal_sample_count": signals.len()
    })
}

fn parse_frequency_mhz(line: &str) -> Option<u64> {
    line.split_once(':')?
        .1
        .split_whitespace()
        .next()?
        .parse::<u64>()
        .ok()
}

fn parse_survey(source: &str) -> Vec<Value> {
    let mut channels: Vec<Value> = Vec::new();
    let mut current: Option<(Option<u64>, Option<u64>, Option<u64>, Option<i64>)> = None;

    for line in source.lines().map(str::trim) {
        if line.starts_with("frequency:") {
            if let Some((frequency, active, busy, noise)) = current.take() {
                channels.push(json!({
                    "frequency_mhz": frequency,
                    "channel_active_time_ms": active,
                    "channel_busy_time_ms": busy,
                    "noise_dbm": noise
                }));
            }
            current = Some((parse_frequency_mhz(line), None, None, None));
            continue;
        }
        if let Some((_, active, busy, noise)) = current.as_mut() {
            if line.starts_with("channel active time:") {
                *active = parse_u64_after_colon(line);
            } else if line.starts_with("channel busy time:") {
                *busy = parse_u64_after_colon(line);
            } else if line.starts_with("noise:") {
                *noise = parse_i64_after_colon(line).filter(|value| (-150..=0).contains(value));
            }
        }
    }
    if let Some((frequency, active, busy, noise)) = current {
        channels.push(json!({
            "frequency_mhz": frequency,
            "channel_active_time_ms": active,
            "channel_busy_time_ms": busy,
            "noise_dbm": noise
        }));
    }
    channels
}

fn parse_unsigned_token(value: &str, suffix: Option<char>) -> Option<u64> {
    let value = value.trim_matches(|ch: char| matches!(ch, ',' | '(' | ')'));
    let value = match suffix {
        Some(suffix) => value.strip_suffix(suffix)?,
        None => value,
    };
    value.parse::<u64>().ok()
}

fn token_after(tokens: &[&str], label: &str) -> Option<u64> {
    let index = tokens.iter().position(|token| *token == label)?;
    tokens.get(index + 1)?.parse::<u64>().ok()
}

fn aggregate_optional_totals(current: &mut Option<u64>, value: Option<u64>) {
    if let Some(total) = *current {
        *current = value.and_then(|value| total.checked_add(value));
    }
}

fn parse_qdiscs(source: &str) -> Value {
    let mut qdisc_count = 0_u64;
    let mut sent_rows = 0_u64;
    let mut sent_bytes = Some(0_u64);
    let mut sent_packets = Some(0_u64);
    let mut dropped_packets = Some(0_u64);
    let mut overlimits = Some(0_u64);
    let mut requeues = Some(0_u64);
    let mut backlog_rows = 0_u64;
    let mut backlog_bytes = Some(0_u64);
    let mut backlog_packets = Some(0_u64);

    for line in source.lines().map(str::trim) {
        if line.starts_with("qdisc ") {
            qdisc_count = qdisc_count.saturating_add(1);
        }
        if line.starts_with("Sent ") {
            sent_rows = sent_rows.saturating_add(1);
            let normalized = line.replace(',', " ").replace('(', " ").replace(')', " ");
            let tokens: Vec<&str> = normalized.split_whitespace().collect();
            let bytes = tokens.get(1).and_then(|v| v.parse::<u64>().ok());
            let packets = tokens.get(3).and_then(|v| v.parse::<u64>().ok());
            aggregate_optional_totals(&mut sent_bytes, bytes);
            aggregate_optional_totals(&mut sent_packets, packets);
            aggregate_optional_totals(&mut dropped_packets, token_after(&tokens, "dropped"));
            aggregate_optional_totals(&mut overlimits, token_after(&tokens, "overlimits"));
            aggregate_optional_totals(&mut requeues, token_after(&tokens, "requeues"));
        } else if line.starts_with("backlog ") {
            backlog_rows = backlog_rows.saturating_add(1);
            let tokens: Vec<&str> = line.split_whitespace().collect();
            let bytes = tokens
                .get(1)
                .and_then(|value| parse_unsigned_token(value, Some('b')));
            let packets = tokens
                .get(2)
                .and_then(|value| parse_unsigned_token(value, Some('p')));
            aggregate_optional_totals(&mut backlog_bytes, bytes);
            aggregate_optional_totals(&mut backlog_packets, packets);
        }
    }

    json!({
        "qdisc_count": qdisc_count,
        "sent_bytes_total": if sent_rows == 0 { None } else { sent_bytes },
        "sent_packets_total": if sent_rows == 0 { None } else { sent_packets },
        "dropped_packets_total": if sent_rows == 0 { None } else { dropped_packets },
        "overlimits_total": if sent_rows == 0 { None } else { overlimits },
        "requeues_total": if sent_rows == 0 { None } else { requeues },
        "backlog_bytes_total": if backlog_rows == 0 { None } else { backlog_bytes },
        "backlog_packets_total": if backlog_rows == 0 { None } else { backlog_packets }
    })
}

fn parse_link(source: &str) -> Value {
    let parsed: Result<Value, _> = serde_json::from_str(source);
    let link = parsed
        .ok()
        .and_then(|value| value.as_array().and_then(|items| items.first().cloned()))
        .unwrap_or(Value::Null);
    let mtu = link.get("mtu").and_then(Value::as_u64);
    let operstate = link
        .get("operstate")
        .and_then(Value::as_str)
        .filter(|value| value.len() <= 32);
    let administratively_up = link
        .get("flags")
        .and_then(Value::as_array)
        .map(|flags| flags.iter().any(|flag| flag.as_str() == Some("UP")));
    json!({
        "mtu": mtu,
        "operstate": operstate,
        "administratively_up": administratively_up
    })
}

fn validate_snapshot(snapshot: &Value) -> Result<(), Vec<String>> {
    if SNAPSHOT_SCHEMA.len() > MAX_SCHEMA_BYTES {
        return Err(vec!["embedded output schema exceeds the safety bound".to_owned()]);
    }
    let schema: Value = serde_json::from_str(SNAPSHOT_SCHEMA)
        .map_err(|error| vec![format!("embedded output schema is invalid JSON: {error}")])?;
    let validator = jsonschema::draft202012::options()
        .should_validate_formats(true)
        .build(&schema)
        .map_err(|error| vec![format!("cannot build output schema validator: {error}")])?;
    let errors: Vec<String> = validator
        .iter_errors(snapshot)
        .map(|error| format!("output schema: {error}"))
        .collect();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Admit only a validated measured-stage manifest for a dedicated, authorized
/// test network. A planned fixture is never sufficient to trigger host probing.
pub fn validate_observation_manifest(
    schema_source: &str,
    manifest_bytes: &[u8],
) -> Result<Value, Vec<String>> {
    if schema_source.len() > MAX_SCHEMA_BYTES || manifest_bytes.len() > MAX_SCHEMA_BYTES {
        return Err(vec!["manifest schema or manifest exceeds the configured size bound".to_owned()]);
    }
    let manifest_source = std::str::from_utf8(manifest_bytes)
        .map_err(|error| vec![format!("manifest is not UTF-8: {error}")])?;
    let manifest = validate_manifest(schema_source, manifest_source)?;
    if manifest.get("evidence_stage").and_then(Value::as_str) == Some("planned") {
        return Err(vec![
            "refusing observation with a planned manifest; record actual hardware and instrumentation first".to_owned(),
        ]);
    }
    if manifest.pointer("/run_controls/dedicated_test_network").and_then(Value::as_bool) != Some(true)
        || manifest.pointer("/run_controls/public_users_present").and_then(Value::as_bool) != Some(false)
        || manifest.pointer("/run_controls/network_authorization").and_then(Value::as_str) != Some("owner_authorized_lab")
    {
        return Err(vec![
            "observation requires an owner-authorized isolated test network with no public users".to_owned(),
        ]);
    }
    Ok(manifest)
}

/// Outputs of four allow-listed read-only observation commands.
#[derive(Clone, Copy, Debug, Default)]
pub struct CapturedOutputs<'a> {
    pub station: Option<&'a str>,
    pub survey: Option<&'a str>,
    pub qdisc: Option<&'a str>,
    pub link: Option<&'a str>,
}

/// Build a privacy-safe single-snapshot record from captured command output.
/// Missing/unavailable counters remain null rather than silently becoming zero.
pub fn build_snapshot(
    run_id: &str,
    interface: &str,
    manifest_bytes: &[u8],
    captured_at_unix_ms: u64,
    outputs: CapturedOutputs<'_>,
    command_status: Value,
) -> Result<Value, Vec<String>> {
    validate_interface_token(interface).map_err(|error| vec![error])?;
    if !valid_token(run_id) {
        return Err(vec!["run_id must be a non-empty ASCII token of at most 128 bytes".to_owned()]);
    }
    if manifest_bytes.len() > MAX_SCHEMA_BYTES {
        return Err(vec!["manifest exceeds the configured size bound".to_owned()]);
    }
    let manifest_sha = sha256_hex(manifest_bytes);
    let empty_station = json!({
        "station_count": null, "rx_bytes_total": null, "tx_bytes_total": null,
        "tx_retries_total": null, "tx_failed_total": null,
        "signal_min_dbm": null, "signal_mean_millidbm": null,
        "signal_max_dbm": null, "signal_sample_count": 0
    });
    let empty_qdisc = json!({
        "qdisc_count": null, "sent_bytes_total": null, "sent_packets_total": null,
        "dropped_packets_total": null, "overlimits_total": null, "requeues_total": null,
        "backlog_bytes_total": null, "backlog_packets_total": null
    });
    let snapshot = json!({
        "schema_version": "luminous.connectivity-ap-observation-snapshot.v1",
        "run_id": run_id,
        "evidence_class": "bench_observed_snapshot",
        "captured_at_unix_ms": captured_at_unix_ms,
        "source": {
            "manifest_sha256": manifest_sha,
            "interface_source": "operator_supplied"
        },
        "interface_token": interface,
        "command_status": command_status,
        "station_metrics": outputs.station.map(parse_stations).unwrap_or(empty_station),
        "survey_channels": outputs.survey.map(parse_survey).unwrap_or_default(),
        "qdisc_metrics": outputs.qdisc.map(parse_qdiscs).unwrap_or(empty_qdisc),
        "link_metrics": outputs.link.map(parse_link).unwrap_or(json!({
            "mtu": null, "operstate": null, "administratively_up": null
        })),
        "privacy": {
            "raw_station_identifiers_exported": false,
            "ip_addresses_exported": false,
            "payloads_exported": false,
            "raw_command_output_exported": false
        }
    });
    validate_snapshot(&snapshot)?;
    Ok(snapshot)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn statuses() -> Value {
        json!({
            "iw_station": {"status":"available","exit_code":0},
            "iw_survey": {"status":"available","exit_code":0},
            "tc_qdisc": {"status":"available","exit_code":0},
            "ip_link": {"status":"available","exit_code":0}
        })
    }

    #[test]
    fn parses_station_counters_without_exporting_station_addresses() {
        let stations = "Station aa:bb:cc:dd:ee:01 (on wlan0)\n\trx bytes: 100\n\ttx bytes: 200\n\ttx retries: 3\n\ttx failed: 1\n\tsignal: -41 dBm\nStation aa:bb:cc:dd:ee:02 (on wlan0)\n\trx bytes: 400\n\ttx bytes: 500\n\ttx retries: 4\n\ttx failed: 2\n\tsignal: -61 dBm\n";
        let snapshot = build_snapshot(
            "run-001", "wlan0", b"manifest bytes", 123,
            CapturedOutputs { station: Some(stations), ..Default::default() },
            statuses(),
        ).unwrap();
        let value = snapshot["station_metrics"].clone();
        assert_eq!(value["station_count"], 2);
        assert_eq!(value["rx_bytes_total"], 500);
        assert_eq!(value["tx_bytes_total"], 700);
        assert_eq!(value["tx_retries_total"], 7);
        assert_eq!(value["tx_failed_total"], 3);
        assert_eq!(value["signal_min_dbm"], -61);
        assert_eq!(value["signal_max_dbm"], -41);
        let serialized = serde_json::to_string(&snapshot).unwrap();
        assert!(!serialized.contains("aa:bb:cc:dd:ee:01"));
        assert!(!serialized.contains("aa:bb:cc:dd:ee:02"));
    }

    #[test]
    fn missing_station_counter_stays_null_not_partial_sum() {
        let stations = "Station aa:bb:cc:dd:ee:01 (on wlan0)\n\trx bytes: 100\nStation aa:bb:cc:dd:ee:02 (on wlan0)\n\trx bytes: 400\n\ttx bytes: 500\n";
        let value = parse_stations(stations);
        assert_eq!(value["rx_bytes_total"], 500);
        assert_eq!(value["tx_bytes_total"], Value::Null);
        assert_eq!(value["tx_retries_total"], Value::Null);
    }

    #[test]
    fn survey_parser_retains_per_channel_counter_semantics() {
        let survey = "Survey data from wlan0\n\tfrequency: 2412 MHz [in use]\n\tchannel active time: 1200 ms\n\tchannel busy time: 300 ms\n\tnoise: -94 dBm\n\tfrequency: 5180 MHz\n\tchannel active time: 1200 ms\n\tchannel busy time: 600 ms\n";
        let rows = parse_survey(survey);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["frequency_mhz"], 2412);
        assert_eq!(rows[0]["channel_busy_time_ms"], 300);
        assert_eq!(rows[1]["noise_dbm"], Value::Null);
    }

    #[test]
    fn qdisc_counters_are_aggregated_and_unknowns_remain_null() {
        let qdisc = "qdisc cake 8001: root\n Sent 1234 bytes 12 pkt (dropped 2, overlimits 4 requeues 1)\n backlog 64b 2p requeues 0\nqdisc ingress ffff: parent ffff:fff1\n backlog 0b 0p requeues 0\n";
        let value = parse_qdiscs(qdisc);
        assert_eq!(value["qdisc_count"], 2);
        assert_eq!(value["sent_bytes_total"], 1234);
        assert_eq!(value["sent_packets_total"], 12);
        assert_eq!(value["dropped_packets_total"], 2);
        assert_eq!(value["overlimits_total"], 4);
        assert_eq!(value["requeues_total"], 1);
        assert_eq!(value["backlog_bytes_total"], 64);
        assert_eq!(value["backlog_packets_total"], 2);
    }

    #[test]
    fn link_parser_does_not_export_addresses() {
        let link = r#"[{"ifindex":3,"ifname":"wlan0","flags":["BROADCAST","UP","LOWER_UP"],"mtu":1500,"operstate":"UP","address":"aa:bb:cc:dd:ee:01","addr_info":[{"family":"inet","local":"192.0.2.11"}]}]"#;
        let value = parse_link(link);
        assert_eq!(value["mtu"], 1500);
        assert_eq!(value["operstate"], "UP");
        assert_eq!(value["administratively_up"], true);
        let serialized = serde_json::to_string(&value).unwrap();
        assert!(!serialized.contains("aa:bb:cc:dd:ee:01"));
        assert!(!serialized.contains("192.0.2.11"));
    }

    #[test]
    fn command_failures_and_unavailable_metrics_still_form_valid_snapshot() {
        let value = build_snapshot(
            "run-partial", "wlan0", b"manifest", 0, CapturedOutputs::default(),
            json!({
                "iw_station": {"status":"unavailable","exit_code":null},
                "iw_survey": {"status":"error","exit_code":1},
                "tc_qdisc": {"status":"unavailable","exit_code":null},
                "ip_link": {"status":"available","exit_code":0}
            }),
        ).unwrap();
        assert_eq!(value["station_metrics"]["station_count"], Value::Null);
        assert_eq!(value["qdisc_metrics"]["sent_bytes_total"], Value::Null);
        assert_eq!(value["privacy"]["raw_command_output_exported"], false);
    }

    #[test]
    fn planned_testbed_manifest_is_not_admitted_for_host_observation() {
        let schema = include_str!("../../../schemas/connectivity-ap-testbed-manifest-v1.schema.json");
        let manifest = include_str!("../../../examples/connectivity-ap-testbed-manifest-planned-v1.json");
        let error = validate_observation_manifest(schema, manifest.as_bytes())
            .expect_err("planned equipment is not a measured testbed");
        assert!(error.iter().any(|message| message.contains("planned manifest")));
    }

    #[test]
    fn invalid_interface_and_run_ids_fail_closed() {
        assert!(build_snapshot(
            "run-001", "wlan0;ip", b"manifest", 0, CapturedOutputs::default(), statuses(),
        ).is_err());
        assert!(build_snapshot(
            "run has spaces", "wlan0", b"manifest", 0, CapturedOutputs::default(), statuses(),
        ).is_err());
    }

    #[test]
    fn station_identifier_injection_is_not_copied_to_output() {
        let stations = "Station peer-a-secret (on wlan0)\n\trx bytes: 1\n\ttx bytes: 2\n\ttx retries: 0\n\ttx failed: 0\n";
        let snapshot = build_snapshot(
            "run-privacy", "wlan0", b"manifest", 0,
            CapturedOutputs { station: Some(stations), ..Default::default() },
            statuses(),
        ).unwrap();
        assert!(!serde_json::to_string(&snapshot).unwrap().contains("peer-a-secret"));
    }
}
