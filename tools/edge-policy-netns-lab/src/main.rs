#![forbid(unsafe_code)]

//! Fixed, opt-in TCP policy test confined to two temporary Linux network
//! namespaces. It does not change host firewall rules or create an external route.

use std::env;
use std::fmt::Write as _;
use std::io::Write as IoWrite;
use std::process::{Child, Command, ExitCode, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use edge_policy_observation_reconciler::{build_synthetic_receipt, reconcile};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const ACK_ENV: &str = "LUMINOUS_EDGE_NETNS_ACK";
const ACK_VALUE: &str = "EPHEMERAL_NAMESPACES_ONLY";
const CORPUS_SCHEMA: &str =
    include_str!("../../../schemas/edge-policy-netns-dual-stack-corpus-v1.schema.json");
const CORPUS_SOURCE: &str =
    include_str!("../../../examples/edge-policy-netns-dual-stack-corpus-v1.json");
const RECEIPT_SCHEMA: &str =
    include_str!("../../../schemas/edge-policy-observation-receipt-v1.schema.json");

const NFT_POLICY_SCRIPT: &str = r#"table inet luminous_edge_lab {
    chain input {
        type filter hook input priority 0; policy drop;
        iifname "lo" accept
        tcp dport 8080 accept
    }
}
"#;

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

fn json_bytes(value: &Value) -> Result<Vec<u8>, String> {
    serde_json::to_vec(value).map_err(|error| format!("cannot encode evidence JSON: {error}"))
}

fn unix_time_ms() -> Result<u64, String> {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("wall clock is before Unix epoch: {error}"))?;
    u64::try_from(elapsed.as_millis()).map_err(|error| format!("wall timestamp overflow: {error}"))
}

fn ensure_tool(name: &str) -> Result<(), String> {
    Command::new(name)
        .arg("--help")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|_| ())
        .map_err(|error| format!("required lab command {name} is unavailable: {error}"))
}

fn checked(program: &str, args: &[&str]) -> Result<Output, String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|error| format!("could not execute {program}: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "{program} {:?} failed with status {:?}: {}",
            args,
            output.status.code(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(output)
}

fn namespace_output(namespace: &str, args: &[&str]) -> Result<Output, String> {
    let mut full = vec!["netns", "exec", namespace];
    full.extend_from_slice(args);
    checked("ip", &full)
}

fn apply_nft_script(namespace: &str, script: &str) -> Result<(), String> {
    let mut child = Command::new("ip")
        .args(["netns", "exec", namespace, "nft", "-f", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("could not start namespaced nft: {error}"))?;
    child
        .stdin
        .as_mut()
        .ok_or_else(|| "nft script stdin was not available".to_owned())?
        .write_all(script.as_bytes())
        .map_err(|error| format!("could not write fixed nft policy: {error}"))?;
    drop(child.stdin.take());
    let output = child
        .wait_with_output()
        .map_err(|error| format!("could not collect nft result: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "namespaced nft rejected the fixed policy: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(())
}

fn spawn_listener(namespace: &str, ipv6: bool, address: &str, port: &str) -> Result<Child, String> {
    let mut arguments = vec!["netns", "exec", namespace, "nc"];
    if ipv6 {
        arguments.push("-6");
    }
    arguments.extend(["-l", address, port]);
    Command::new("ip")
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("could not start fixed TCP test listener: {error}"))
}

struct LabGuard {
    client_ns: String,
    server_ns: String,
    root_links: Vec<String>,
    listeners: Vec<Child>,
}

impl Drop for LabGuard {
    fn drop(&mut self) {
        for listener in &mut self.listeners {
            let _ = listener.kill();
            let _ = listener.wait();
        }
        for interface in &self.root_links {
            let _ = Command::new("ip")
                .args(["link", "del", "dev", interface.as_str()])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        }
        for namespace in [&self.server_ns, &self.client_ns] {
            let _ = Command::new("ip")
                .args(["netns", "del", namespace.as_str()])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        }
    }
}

fn prepare_namespaces(
    guard: &mut LabGuard,
    client_if: &str,
    server_if: &str,
) -> Result<(), String> {
    checked("ip", &["netns", "add", &guard.client_ns])?;
    checked("ip", &["netns", "add", &guard.server_ns])?;

    guard.root_links.push(client_if.to_owned());
    guard.root_links.push(server_if.to_owned());
    checked(
        "ip",
        &[
            "link", "add", client_if, "type", "veth", "peer", "name", server_if,
        ],
    )?;
    checked(
        "ip",
        &["link", "set", "dev", client_if, "netns", &guard.client_ns],
    )?;
    checked(
        "ip",
        &["link", "set", "dev", server_if, "netns", &guard.server_ns],
    )?;

    for namespace in [&guard.client_ns, &guard.server_ns] {
        checked("ip", &["-n", namespace, "link", "set", "lo", "up"])?;
    }
    checked(
        "ip",
        &[
            "-n",
            &guard.client_ns,
            "address",
            "add",
            "198.18.0.1/30",
            "dev",
            client_if,
        ],
    )?;
    checked(
        "ip",
        &[
            "-n",
            &guard.server_ns,
            "address",
            "add",
            "198.18.0.2/30",
            "dev",
            server_if,
        ],
    )?;
    checked(
        "ip",
        &[
            "-n",
            &guard.client_ns,
            "-6",
            "address",
            "add",
            "2001:db8:1::1/64",
            "dev",
            client_if,
        ],
    )?;
    checked(
        "ip",
        &[
            "-n",
            &guard.server_ns,
            "-6",
            "address",
            "add",
            "2001:db8:1::2/64",
            "dev",
            server_if,
        ],
    )?;
    checked(
        "ip",
        &[
            "-n",
            &guard.client_ns,
            "link",
            "set",
            "dev",
            client_if,
            "up",
        ],
    )?;
    checked(
        "ip",
        &[
            "-n",
            &guard.server_ns,
            "link",
            "set",
            "dev",
            server_if,
            "up",
        ],
    )?;

    for namespace in [&guard.client_ns, &guard.server_ns] {
        for family in ["route", "-6"] {
            let output = if family == "-6" {
                checked("ip", &["-n", namespace, "-6", "route", "show"])?
            } else {
                checked("ip", &["-n", namespace, "route", "show"])?
            };
            let routes = String::from_utf8_lossy(&output.stdout);
            if routes
                .lines()
                .any(|line| line.split_whitespace().next() == Some("default"))
            {
                return Err(format!(
                    "lab namespace unexpectedly has a {family} default route; refusing to probe"
                ));
            }
        }
    }
    Ok(())
}

fn start_and_check_listener(
    guard: &mut LabGuard,
    namespace: &str,
    ipv6: bool,
    address: &str,
    port: &str,
) -> Result<(), String> {
    let mut listener = spawn_listener(namespace, ipv6, address, port)?;
    thread::sleep(Duration::from_millis(150));
    if let Some(status) = listener
        .try_wait()
        .map_err(|error| format!("cannot inspect TCP listener state: {error}"))?
    {
        return Err(format!(
            "TCP listener on fixed port {port} at {address} exited early with {status}"
        ));
    }
    guard.listeners.push(listener);
    Ok(())
}

fn probe_case(
    case: &Value,
    client_ns: &str,
    effective_policy_sha256: &str,
    run_start: Instant,
    wall_anchor_unix_ms: u64,
) -> Result<(Value, Value), String> {
    let case_id = case["case_id"]
        .as_str()
        .ok_or_else(|| "lab corpus case has no string case_id".to_owned())?;
    let protocol = case["protocol"]
        .as_str()
        .ok_or_else(|| format!("{case_id}: protocol is not a string"))?;
    if protocol != "tcp" {
        return Err(format!("{case_id}: only the fixed TCP probe is supported"));
    }
    let port = case["destination_port"]
        .as_u64()
        .ok_or_else(|| format!("{case_id}: destination_port is not an unsigned integer"))?;
    if !matches!(port, 8080 | 8081) {
        return Err(format!(
            "{case_id}: port is not in the fixed allow-listed port set"
        ));
    }
    let address_family = case["address_family"]
        .as_str()
        .ok_or_else(|| format!("{case_id}: address_family is not a string"))?;
    let (ipv6, destination) = match address_family {
        "ipv4" => (false, "198.18.0.2"),
        "ipv6" => (true, "2001:db8:1::2"),
        _ => return Err(format!("{case_id}: address family is not allow-listed")),
    };
    let port_string = port.to_string();
    let probe_start = Instant::now();
    let mut arguments = vec!["netns", "exec", client_ns, "nc"];
    if ipv6 {
        arguments.push("-6");
    }
    arguments.extend(["-z", "-w", "1", destination, port_string.as_str()]);
    let output = Command::new("ip")
        .args(arguments)
        .output()
        .map_err(|error| format!("{case_id}: failed to start TCP probe: {error}"))?;
    let duration_ms = u64::try_from(probe_start.elapsed().as_millis())
        .map_err(|error| format!("{case_id}: probe duration overflow: {error}"))?;
    let exit_code = output
        .status
        .code()
        .ok_or_else(|| format!("{case_id}: TCP probe ended without a process exit code"))?;
    let observed_outcome = if output.status.success() {
        "permitted"
    } else {
        "blocked"
    };
    let monotonic_elapsed_ms = u64::try_from(run_start.elapsed().as_millis())
        .map_err(|error| format!("{case_id}: elapsed-time overflow: {error}"))?;
    let observed_at_unix_ms = wall_anchor_unix_ms.saturating_add(monotonic_elapsed_ms);

    let record = json!({
        "case_id": case_id,
        "protocol": protocol,
        "address_family": address_family,
        "destination_port": port,
        "observed_outcome": observed_outcome,
        "exit_code": exit_code,
        "duration_ms": duration_ms,
        "monotonic_elapsed_ms": monotonic_elapsed_ms,
        "observed_at_unix_ms": observed_at_unix_ms,
        "effective_policy_sha256": effective_policy_sha256,
        "probe_kind": "tcp_connect_no_payload",
        "namespace_scope": "ephemeral_veth_only"
    });
    let evidence_sha256 = sha256_hex(&json_bytes(&record)?);
    let observation = json!({
        "observation_id": format!("OBS-LAB-{case_id}"),
        "case_id": case_id,
        "observed_outcome": observed_outcome,
        "evidence_source": "independent_dataplane_probe",
        "effective_policy_sha256": effective_policy_sha256,
        "observed_at_unix_ms": observed_at_unix_ms,
        "monotonic_elapsed_ms": monotonic_elapsed_ms,
        "duration_ms": duration_ms,
        "evidence_sha256": evidence_sha256
    });
    Ok((
        observation,
        json!({
            "record": record,
            "evidence_sha256": evidence_sha256
        }),
    ))
}

fn run_lab() -> Result<(Value, ExitCode), String> {
    if env::consts::OS != "linux" {
        return Err("this harness only runs on Linux".to_owned());
    }
    if env::var(ACK_ENV).ok().as_deref() != Some(ACK_VALUE) {
        return Err(format!(
            "refusing to run: set {ACK_ENV}={ACK_VALUE} only for an ephemeral namespace lab"
        ));
    }
    for tool in ["ip", "nft", "nc"] {
        ensure_tool(tool)?;
    }

    let pid = std::process::id();
    let client_ns = format!("lp-edge-c-{pid}");
    let server_ns = format!("lp-edge-s-{pid}");
    let client_if = format!("lc{pid}");
    let server_if = format!("ls{pid}");
    let mut guard = LabGuard {
        client_ns,
        server_ns,
        root_links: Vec::new(),
        listeners: Vec::new(),
    };
    prepare_namespaces(&mut guard, &client_if, &server_if)?;
    apply_nft_script(&guard.server_ns, NFT_POLICY_SCRIPT)?;

    let server_ns_for_listeners = guard.server_ns.clone();
    start_and_check_listener(
        &mut guard,
        &server_ns_for_listeners,
        false,
        "198.18.0.2",
        "8080",
    )?;
    start_and_check_listener(
        &mut guard,
        &server_ns_for_listeners,
        false,
        "198.18.0.2",
        "8081",
    )?;
    start_and_check_listener(
        &mut guard,
        &server_ns_for_listeners,
        true,
        "2001:db8:1::2",
        "8080",
    )?;
    start_and_check_listener(
        &mut guard,
        &server_ns_for_listeners,
        true,
        "2001:db8:1::2",
        "8081",
    )?;

    let ruleset = namespace_output(&guard.server_ns, &["nft", "list", "ruleset"])?;
    let effective_ruleset = String::from_utf8_lossy(&ruleset.stdout);
    if !effective_ruleset.contains("table inet luminous_edge_lab")
        || !effective_ruleset.contains("policy drop")
        || !effective_ruleset.contains("tcp dport 8080 accept")
        || effective_ruleset.contains("tcp dport 8081 accept")
        || !effective_ruleset.contains("iifname \"lo\" accept")
        || effective_ruleset
            .lines()
            .filter(|line| line.contains("tcp dport") && line.contains("accept"))
            .count()
            != 1
    {
        return Err(
            "effective namespace ruleset does not match the fixed allow/deny contract".to_owned(),
        );
    }
    let client_routes = checked("ip", &["-n", &guard.client_ns, "route", "show"])?;
    let server_routes = checked("ip", &["-n", &guard.server_ns, "route", "show"])?;
    let client_addresses = checked("ip", &["-n", &guard.client_ns, "-j", "address", "show"])?;
    let server_addresses = checked("ip", &["-n", &guard.server_ns, "-j", "address", "show"])?;
    let effective_policy_sha256 = sha256_hex(&ruleset.stdout);
    let config_material = json!({
        "profile_id": "luminous-edge-netns-dual-stack-v1",
        "client_routes": String::from_utf8_lossy(&client_routes.stdout),
        "server_routes": String::from_utf8_lossy(&server_routes.stdout),
        "client_addresses": String::from_utf8_lossy(&client_addresses.stdout),
        "server_addresses": String::from_utf8_lossy(&server_addresses.stdout),
        "effective_policy_sha256": effective_policy_sha256
    });
    let effective_config_sha256 = sha256_hex(&json_bytes(&config_material)?);
    let approved_policy_sha256 = sha256_hex(NFT_POLICY_SCRIPT.as_bytes());

    let wall_anchor_unix_ms = unix_time_ms()?;
    let run_start = Instant::now();
    let corpus: Value = serde_json::from_str(CORPUS_SOURCE)
        .map_err(|error| format!("embedded lab corpus is invalid JSON: {error}"))?;
    let cases = corpus["cases"]
        .as_array()
        .ok_or_else(|| "embedded lab corpus has no cases array".to_owned())?;
    let mut observations = Vec::with_capacity(cases.len());
    let mut probe_records = Vec::with_capacity(cases.len());
    for case in cases {
        let (observation, probe_record) = probe_case(
            case,
            &guard.client_ns,
            &effective_policy_sha256,
            run_start,
            wall_anchor_unix_ms,
        )?;
        observations.push(observation);
        probe_records.push(probe_record);
    }

    let mut receipt = build_synthetic_receipt(CORPUS_SOURCE).map_err(|errors| errors.join("\n"))?;
    receipt["run_id"] = Value::String(format!("namespace-{pid}"));
    receipt["evidence_class"] = Value::String("namespace_lab_observation".to_owned());
    receipt["approved_policy_sha256"] = Value::String(approved_policy_sha256);
    receipt["effective_config_sha256"] = Value::String(effective_config_sha256);
    receipt["target_token"] = Value::String("target-ephemeral-netns".to_owned());
    receipt["time_basis"] = json!({
        "kind": "monotonic_with_wall_anchor",
        "clock_id": "rust-std-instant-run-local",
        "wall_anchor_unix_ms": wall_anchor_unix_ms,
        "monotonic_origin_ms": 0
    });
    receipt["observations"] = Value::Array(observations);

    let receipt_json = serde_json::to_string(&receipt)
        .map_err(|error| format!("cannot serialize observation receipt: {error}"))?;
    let report = reconcile(CORPUS_SCHEMA, CORPUS_SOURCE, RECEIPT_SCHEMA, &receipt_json)
        .map_err(|errors| errors.join("\n"))?;
    let disposition = report["disposition"].as_str().unwrap_or("");
    let exit_code = if disposition == "reported_observations_consistent_not_authenticated" {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(2)
    };

    // Namespace cleanup runs via LabGuard on return, including all error paths.
    Ok((
        json!({
            "lab_scope": "ephemeral_linux_network_namespaces_ipv4_ipv6_only",
            "host_firewall_mutated": false,
            "external_route_configured": false,
            "payloads_sent": false,
            "receipt": receipt,
            "probe_records": probe_records,
            "report": report
        }),
        exit_code,
    ))
}

fn main() -> ExitCode {
    let arguments: Vec<String> = env::args().collect();
    if arguments.len() != 2 || arguments[1] != "--confirm-ephemeral-netns-only" {
        eprintln!(
            "refusing to run without explicit opt-in; usage: {} --confirm-ephemeral-netns-only",
            arguments
                .first()
                .map(String::as_str)
                .unwrap_or("edge-policy-netns-lab")
        );
        return ExitCode::FAILURE;
    }
    match run_lab() {
        Ok((output, code)) => {
            match serde_json::to_string_pretty(&output) {
                Ok(json) => println!("{json}"),
                Err(error) => {
                    eprintln!("cannot serialize lab report: {error}");
                    return ExitCode::FAILURE;
                }
            }
            code
        }
        Err(error) => {
            eprintln!("edge policy namespace lab failed closed: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CORPUS_SCHEMA, CORPUS_SOURCE, NFT_POLICY_SCRIPT, RECEIPT_SCHEMA};
    use serde_json::Value;

    #[test]
    fn fixed_policy_allows_only_the_registered_tcp_port() {
        assert!(NFT_POLICY_SCRIPT.contains("tcp dport 8080 accept"));
        assert!(!NFT_POLICY_SCRIPT.contains("tcp dport 8081 accept"));
        assert!(NFT_POLICY_SCRIPT.contains("policy drop"));
    }

    #[test]
    fn dual_stack_corpus_has_ipv4_and_ipv6_positive_and_negative_controls() {
        let corpus: Value = serde_json::from_str(CORPUS_SOURCE).expect("corpus JSON");
        let cases = corpus["cases"].as_array().expect("cases");
        assert_eq!(cases.len(), 4);
        assert_eq!(cases[0]["address_family"], "ipv4");
        assert_eq!(cases[0]["expected_probe_outcome"], "permitted");
        assert_eq!(cases[0]["destination_port"], 8080);
        assert_eq!(cases[1]["address_family"], "ipv4");
        assert_eq!(cases[1]["expected_probe_outcome"], "blocked");
        assert_eq!(cases[1]["destination_port"], 8081);
        assert_eq!(cases[2]["address_family"], "ipv6");
        assert_eq!(cases[2]["expected_probe_outcome"], "permitted");
        assert_eq!(cases[2]["destination_port"], 8080);
        assert_eq!(cases[3]["address_family"], "ipv6");
        assert_eq!(cases[3]["expected_probe_outcome"], "blocked");
        assert_eq!(cases[3]["destination_port"], 8081);
    }

    #[test]
    fn embedded_schemas_are_valid_json_and_receipt_is_bound_to_namespace_evidence() {
        let corpus_schema: Value = serde_json::from_str(CORPUS_SCHEMA).expect("corpus schema JSON");
        let receipt_schema: Value =
            serde_json::from_str(RECEIPT_SCHEMA).expect("receipt schema JSON");
        assert_eq!(
            corpus_schema["properties"]["corpus_id"]["const"],
            "luminous-edge-netns-dual-stack-v1"
        );
        assert!(
            receipt_schema["properties"]["evidence_class"]["enum"]
                .as_array()
                .expect("evidence classes")
                .iter()
                .any(|value| value == "namespace_lab_observation")
        );
    }
}
