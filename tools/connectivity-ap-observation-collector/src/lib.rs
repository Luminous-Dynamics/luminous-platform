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

/// Parse only non-identifying radio metadata from `iw dev IFACE info`.
/// SSID, MAC address, wiphy IDs and other fields are intentionally discarded.
pub fn parse_interface_info(source: &str) -> Value {
    let mut interface_type = None;
    let mut channel_number = None;
    let mut frequency_mhz = None;
    let mut channel_width_mhz = None;
    let mut tx_power_centi_dbm = None;

    for line in source.lines().map(str::trim) {
        if let Some(value) = line.strip_prefix("type ") {
            interface_type = Some(value.to_owned());
        } else if line.starts_with("channel ") {
            let rest = line.trim_start_matches("channel ");
            channel_number = rest
                .split_whitespace()
                .next()
                .and_then(|value| value.parse::<u64>().ok());

            if let Some(start) = rest.find('(') {
                frequency_mhz = rest[start + 1..]
                    .split_whitespace()
                    .next()
                    .and_then(|value| value.parse::<u64>().ok());
            }
            if let Some(width) = line.split_once("width:") {
                channel_width_mhz = width
                    .1
                    .split_whitespace()
                    .next()
                    .and_then(|value| value.parse::<u64>().ok());
            }
        } else if let Some(value) = line.strip_prefix("txpower ") {
            tx_power_centi_dbm = value
                .split_whitespace()
                .next()
                .and_then(|number| number.parse::<f64>().ok())
                .filter(|number| (-50.0..=60.0).contains(number))
                .map(|number| (number * 100.0).round() as i64);
        }
    }

    json!({
        "interface_type": interface_type,
        "channel_number": channel_number,
        "frequency_mhz": frequency_mhz,
        "channel_width_mhz": channel_width_mhz,
        "tx_power_centi_dbm": tx_power_centi_dbm
    })
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

#[derive(Clone, Copy, Debug, Default)]
struct SurveyChannel {
    frequency_mhz: Option<u64>,
    active_time_ms: Option<u64>,
    busy_time_ms: Option<u64>,
    noise_dbm: Option<i64>,
}

impl SurveyChannel {
    fn into_json(self) -> Value {
        json!({
            "frequency_mhz": self.frequency_mhz,
            "channel_active_time_ms": self.active_time_ms,
            "channel_busy_time_ms": self.busy_time_ms,
            "noise_dbm": self.noise_dbm
        })
    }
}

fn parse_survey(source: &str) -> Vec<Value> {
    let mut channels: Vec<Value> = Vec::new();
    let mut current: Option<SurveyChannel> = None;

    for line in source.lines().map(str::trim) {
        if line.starts_with("frequency:") {
            if let Some(channel) = current.take() {
                channels.push(channel.into_json());
            }
            current = Some(SurveyChannel {
                frequency_mhz: parse_frequency_mhz(line),
                ..Default::default()
            });
            continue;
        }
        if let Some(channel) = current.as_mut() {
            if line.starts_with("channel active time:") {
                channel.active_time_ms = parse_u64_after_colon(line);
            } else if line.starts_with("channel busy time:") {
                channel.busy_time_ms = parse_u64_after_colon(line);
            } else if line.starts_with("noise:") {
                channel.noise_dbm =
                    parse_i64_after_colon(line).filter(|value| (-150..=0).contains(value));
            }
        }
    }
    if let Some(channel) = current {
        channels.push(channel.into_json());
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
            let normalized = line.replace([',', '(', ')'], " ");
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
        return Err(vec![
            "embedded output schema exceeds the safety bound".to_owned(),
        ]);
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
        return Err(vec![
            "manifest schema or manifest exceeds the configured size bound".to_owned(),
        ]);
    }
    let manifest_source = std::str::from_utf8(manifest_bytes)
        .map_err(|error| vec![format!("manifest is not UTF-8: {error}")])?;
    let manifest = validate_manifest(schema_source, manifest_source)?;
    if manifest.get("evidence_stage").and_then(Value::as_str) == Some("planned") {
        return Err(vec![
            "refusing observation with a planned manifest; record actual hardware and instrumentation first".to_owned(),
        ]);
    }
    if manifest
        .pointer("/run_controls/dedicated_test_network")
        .and_then(Value::as_bool)
        != Some(true)
        || manifest
            .pointer("/run_controls/public_users_present")
            .and_then(Value::as_bool)
            != Some(false)
        || manifest
            .pointer("/run_controls/network_authorization")
            .and_then(Value::as_str)
            != Some("owner_authorized_lab")
    {
        return Err(vec![
            "observation requires an owner-authorized isolated test network with no public users"
                .to_owned(),
        ]);
    }
    for pointer in [
        "/access_point/hardware_revision",
        "/access_point/firmware_version",
        "/access_point/firmware_sha256",
        "/access_point/radio_config_sha256",
        "/access_point/channel",
        "/access_point/channel_width_mhz",
        "/access_point/tx_power_dbm",
        "/instrumentation/iw_version",
        "/run_controls/test_profile_sha256",
        "/software/harness_revision",
        "/software/profile_sha256",
    ] {
        if manifest.pointer(pointer).is_none_or(Value::is_null) {
            return Err(vec![format!("{pointer} is required for AP observation")]);
        }
    }
    if manifest
        .pointer("/access_point/radio_band")
        .and_then(Value::as_str)
        == Some("unknown")
    {
        return Err(vec![
            "radio_band must be recorded for AP observation".to_owned(),
        ]);
    }
    Ok(manifest)
}

/// Outputs of four allow-listed read-only observation commands.
#[derive(Clone, Copy, Debug, Default)]
pub struct CapturedOutputs<'a> {
    pub info: Option<&'a str>,
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
    let interface_radio = outputs
        .info
        .map(parse_interface_info)
        .ok_or_else(|| vec!["iw interface-info output is required".to_owned()])?;
    if interface_radio
        .get("interface_type")
        .and_then(Value::as_str)
        != Some("AP")
    {
        return Err(vec![
            "refusing to emit AP snapshot because the interface mode is not confirmed as AP"
                .to_owned(),
        ]);
    }
    if !valid_token(run_id) {
        return Err(vec![
            "run_id must be a non-empty ASCII token of at most 128 bytes".to_owned(),
        ]);
    }
    if manifest_bytes.len() > MAX_SCHEMA_BYTES {
        return Err(vec![
            "manifest exceeds the configured size bound".to_owned(),
        ]);
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
        "interface_radio": interface_radio,
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
            "iw_info": {"status":"available","exit_code":0},
            "iw_station": {"status":"available","exit_code":0},
            "iw_survey": {"status":"available","exit_code":0},
            "tc_qdisc": {"status":"available","exit_code":0},
            "ip_link": {"status":"available","exit_code":0}
        })
    }

    #[test]
    fn interface_info_confirms_ap_mode_without_exporting_mac_or_ssid() {
        let info = "Interface wlan0\n\tifindex 3\n\taddr aa:bb:cc:dd:ee:01\n\tssid household-secret\n\ttype AP\n\tchannel 36 (5180 MHz), width: 80 MHz, center1: 5210 MHz\n\ttxpower 20.00 dBm\n";
        let parsed = parse_interface_info(info);
        assert_eq!(parsed["interface_type"], "AP");
        assert_eq!(parsed["channel_number"], 36);
        assert_eq!(parsed["frequency_mhz"], 5180);
        assert_eq!(parsed["channel_width_mhz"], 80);
        assert_eq!(parsed["tx_power_centi_dbm"], 2000);
        let exported = serde_json::to_string(&parsed).unwrap();
        assert!(!exported.contains("aa:bb:cc:dd:ee:01"));
        assert!(!exported.contains("household-secret"));
    }

    #[test]
    fn client_interface_cannot_be_labelled_as_ap_snapshot() {
        let error = build_snapshot(
            "run-client-mode",
            "wlan0",
            b"manifest",
            0,
            CapturedOutputs {
                info: Some("Interface wlan0\n\ttype managed\n"),
                ..Default::default()
            },
            statuses(),
        )
        .expect_err("client-mode interface must not claim AP observation");
        assert!(
            error
                .iter()
                .any(|message| message.contains("mode is not confirmed as AP"))
        );
    }

    #[test]
    fn parses_station_counters_without_exporting_station_addresses() {
        let stations = "Station aa:bb:cc:dd:ee:01 (on wlan0)\n\trx bytes: 100\n\ttx bytes: 200\n\ttx retries: 3\n\ttx failed: 1\n\tsignal: -41 dBm\nStation aa:bb:cc:dd:ee:02 (on wlan0)\n\trx bytes: 400\n\ttx bytes: 500\n\ttx retries: 4\n\ttx failed: 2\n\tsignal: -61 dBm\n";
        let snapshot = build_snapshot(
            "run-001",
            "wlan0",
            b"manifest bytes",
            123,
            CapturedOutputs {
                info: Some("Interface wlan0\n\ttype AP\n\tchannel 36 (5180 MHz), width: 80 MHz\n\ttxpower 20.00 dBm\n"),
                station: Some(stations),
                ..Default::default()
            },
            statuses(),
        )
        .unwrap();
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
            "run-partial",
            "wlan0",
            b"manifest",
            0,
            CapturedOutputs {
                info: Some("Interface wlan0\n\ttype AP\n"),
                ..Default::default()
            },
            json!({
                "iw_info": {"status":"available","exit_code":0},
                "iw_station": {"status":"unavailable","exit_code":null},
                "iw_survey": {"status":"error","exit_code":1},
                "tc_qdisc": {"status":"unavailable","exit_code":null},
                "ip_link": {"status":"available","exit_code":0}
            }),
        )
        .unwrap();
        assert_eq!(value["station_metrics"]["station_count"], Value::Null);
        assert_eq!(value["qdisc_metrics"]["sent_bytes_total"], Value::Null);
        assert_eq!(value["privacy"]["raw_command_output_exported"], false);
    }

    #[test]
    fn planned_testbed_manifest_is_not_admitted_for_host_observation() {
        let schema =
            include_str!("../../../schemas/connectivity-ap-testbed-manifest-v1.schema.json");
        let manifest =
            include_str!("../../../examples/connectivity-ap-testbed-manifest-planned-v1.json");
        let error = validate_observation_manifest(schema, manifest.as_bytes())
            .expect_err("planned equipment is not a measured testbed");
        assert!(
            error
                .iter()
                .any(|message| message.contains("planned manifest"))
        );
    }

    #[test]
    fn invalid_interface_and_run_ids_fail_closed() {
        assert!(
            build_snapshot(
                "run-001",
                "wlan0;ip",
                b"manifest",
                0,
                CapturedOutputs::default(),
                statuses(),
            )
            .is_err()
        );
        assert!(
            build_snapshot(
                "run has spaces",
                "wlan0",
                b"manifest",
                0,
                CapturedOutputs {
                    info: Some("Interface wlan0\n\ttype AP\n"),
                    ..Default::default()
                },
                statuses(),
            )
            .is_err()
        );
    }

    #[test]
    fn station_identifier_injection_is_not_copied_to_output() {
        let stations = "Station peer-a-secret (on wlan0)\n\trx bytes: 1\n\ttx bytes: 2\n\ttx retries: 0\n\ttx failed: 0\n";
        let snapshot = build_snapshot(
            "run-privacy",
            "wlan0",
            b"manifest",
            0,
            CapturedOutputs {
                info: Some("Interface wlan0\n\ttype AP\n"),
                station: Some(stations),
                ..Default::default()
            },
            statuses(),
        )
        .unwrap();
        assert!(
            !serde_json::to_string(&snapshot)
                .unwrap()
                .contains("peer-a-secret")
        );
    }
}
