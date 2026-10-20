#![forbid(unsafe_code)]

//! Opt-in stateful-policy semantics test confined to two temporary Linux namespaces.
//! It distinguishes new-session denial from explicit revocation of an established flow.

use std::env;
use std::io::{self, BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Child, Command, ExitCode, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const ACK_ENV: &str = "LUMINOUS_EDGE_SESSION_LAB_ACK";
const ACK_VALUE: &str = "EPHEMERAL_NAMESPACES_ONLY";
const SERVER_ADDR: &str = "198.18.1.2:8080";
const POLICY_TABLE: &str = "table inet luminous_session_lab";
const ESTABLISHED_RULE: &str = "ct state established,related accept";
const REVOKE_RULE: &str = "ip saddr 198.18.1.1 tcp dport 8080 drop";
const CLIENT_TIMEOUT: Duration = Duration::from_millis(350);

const INITIAL_POLICY: &str = r#"table inet luminous_session_lab {
    chain input {
        type filter hook input priority 0; policy drop;
        iifname "lo" accept
        ct state established,related accept
        tcp dport 8080 accept
    }
}
"#;

const RETAIN_POLICY: &str = r#"flush chain inet luminous_session_lab input
add rule inet luminous_session_lab input iifname "lo" accept
add rule inet luminous_session_lab input ct state established,related accept
"#;

const REVOKE_POLICY: &str = r#"flush chain inet luminous_session_lab input
add rule inet luminous_session_lab input iifname "lo" accept
add rule inet luminous_session_lab input ip saddr 198.18.1.1 tcp dport 8080 drop
add rule inet luminous_session_lab input ct state established,related accept
"#;

#[derive(Clone, Copy, Debug)]
enum PolicyStage {
    Initial,
    RetainEstablished,
    ExplicitRevoke,
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

fn checked(program: &str, args: &[&str]) -> Result<std::process::Output, String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|error| format!("could not execute {program}: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "{program} {:?} exited {:?}: {}",
            args,
            output.status.code(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(output)
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
        .ok_or_else(|| "nft policy stdin was not available".to_owned())?
        .write_all(script.as_bytes())
        .map_err(|error| format!("could not write the fixed nft policy: {error}"))?;
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

fn validate_policy_text(stage: PolicyStage, ruleset: &str) -> Result<(), String> {
    if !ruleset.contains(POLICY_TABLE) || !ruleset.contains("policy drop") {
        return Err("effective policy lacks the expected table or default-drop chain".to_owned());
    }

    let established_at = ruleset.find(ESTABLISHED_RULE);
    let port_allow_at = ruleset.find("tcp dport 8080 accept");
    let revoke_at = ruleset.find(REVOKE_RULE);

    match stage {
        PolicyStage::Initial => {
            let established_at = established_at
                .ok_or_else(|| "initial policy lacks the established-state rule".to_owned())?;
            let port_allow_at = port_allow_at
                .ok_or_else(|| "initial policy lacks the positive TCP/8080 control".to_owned())?;
            if established_at >= port_allow_at {
                return Err("initial policy rule order is unexpected".to_owned());
            }
        }
        PolicyStage::RetainEstablished => {
            if established_at.is_none() || port_allow_at.is_some() || revoke_at.is_some() {
                return Err(
                    "retained-session stage must keep only the established exception, not new-session admission or explicit revocation"
                        .to_owned(),
                );
            }
        }
        PolicyStage::ExplicitRevoke => {
            let revoke_at = revoke_at
                .ok_or_else(|| "explicit-revocation stage lacks the tuple-specific deny".to_owned())?;
            let established_at = established_at
                .ok_or_else(|| "explicit-revocation stage lacks the stateful-rule control".to_owned())?;
            if revoke_at >= established_at {
                return Err(
                    "tuple-specific revocation must precede the generic established-state accept"
                        .to_owned(),
                );
            }
            if port_allow_at.is_some() {
                return Err("explicit-revocation stage unexpectedly permits new TCP/8080 flows".to_owned());
            }
        }
    }
    Ok(())
}

fn effective_policy(namespace: &str, stage: PolicyStage) -> Result<String, String> {
    let output = checked("ip", &["netns", "exec", namespace, "nft", "list", "ruleset"])?;
    let ruleset = String::from_utf8(output.stdout)
        .map_err(|error| format!("effective nft ruleset was not UTF-8: {error}"))?;
    validate_policy_text(stage, &ruleset)?;
    Ok(sha256_hex(ruleset.as_bytes()))
}

struct LabGuard {
    client_ns: String,
    server_ns: String,
    children: Vec<Child>,
    cleaned: bool,
}

impl LabGuard {
    fn cleanup(&mut self) -> Result<(), String> {
        if self.cleaned {
            return Ok(());
        }
        for child in &mut self.children {
            let _ = child.kill();
            let _ = child.wait();
        }
        for namespace in [&self.server_ns, &self.client_ns] {
            let _ = Command::new("ip")
                .args(["netns", "del", namespace])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        }
        let output = checked("ip", &["netns", "list"])?;
        let listing = String::from_utf8_lossy(&output.stdout);
        for namespace in [&self.server_ns, &self.client_ns] {
            if listing
                .lines()
                .any(|line| line.split_whitespace().next() == Some(namespace))
            {
                return Err(format!("temporary namespace {namespace} remains after cleanup"));
            }
        }
        self.cleaned = true;
        Ok(())
    }
}

impl Drop for LabGuard {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

fn prepare_namespaces(guard: &LabGuard, client_if: &str, server_if: &str) -> Result<(), String> {
    checked("ip", &["netns", "add", &guard.client_ns])?;
    checked("ip", &["netns", "add", &guard.server_ns])?;
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
            "198.18.1.1/30",
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
            "198.18.1.2/30",
            "dev",
            server_if,
        ],
    )?;
    checked(
        "ip",
        &["-n", &guard.client_ns, "link", "set", "dev", client_if, "up"],
    )?;
    checked(
        "ip",
        &["-n", &guard.server_ns, "link", "set", "dev", server_if, "up"],
    )?;

    for namespace in [&guard.client_ns, &guard.server_ns] {
        let routes = checked("ip", &["-n", namespace, "route", "show"])?;
        if String::from_utf8_lossy(&routes.stdout)
            .lines()
            .any(|line| line.split_whitespace().next() == Some("default"))
        {
            return Err(format!("{namespace} unexpectedly has an IPv4 default route"));
        }
        let routes_v6 = checked("ip", &["-n", namespace, "-6", "route", "show"])?;
        if String::from_utf8_lossy(&routes_v6.stdout)
            .lines()
            .any(|line| line.split_whitespace().next() == Some("default"))
        {
            return Err(format!("{namespace} unexpectedly has an IPv6 default route"));
        }
    }
    Ok(())
}

fn ensure_server_alive(guard: &mut LabGuard) -> Result<(), String> {
    let server = guard
        .children
        .first_mut()
        .ok_or_else(|| "server process handle was lost".to_owned())?;
    match server
        .try_wait()
        .map_err(|error| format!("could not inspect server process: {error}"))?
    {
        None => Ok(()),
        Some(status) => Err(format!("echo server exited unexpectedly with {status}")),
    }
}

fn read_handshake<R: BufRead>(reader: &mut R, expected: &str) -> Result<(), String> {
    let mut line = String::new();
    let count = reader
        .read_line(&mut line)
        .map_err(|error| format!("could not read lab handshake: {error}"))?;
    if count == 0 || line.trim() != expected {
        return Err(format!(
            "unexpected lab handshake: expected {expected:?}, received {:?}",
            line.trim()
        ));
    }
    Ok(())
}

fn spawn_lab_server(
    guard: &mut LabGuard,
    executable: &std::path::Path,
) -> Result<(), String> {
    let mut child = Command::new("ip")
        .args(["netns", "exec", &guard.server_ns])
        .arg(executable)
        .arg("--server")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|error| format!("could not start the namespaced echo server: {error}"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "server stdout pipe was unavailable".to_owned())?;
    let mut reader = BufReader::new(stdout);
    read_handshake(&mut reader, "READY")?;
    drop(reader);
    guard.children.push(child);
    Ok(())
}

fn spawn_lab_client(
    guard: &mut LabGuard,
    executable: &std::path::Path,
) -> Result<(std::process::ChildStdin, BufReader<std::process::ChildStdout>), String> {
    let mut child = Command::new("ip")
        .args(["netns", "exec", &guard.client_ns])
        .arg(executable)
        .arg("--client")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|error| format!("could not start the namespaced client: {error}"))?;
    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| "client stdin pipe was unavailable".to_owned())?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "client stdout pipe was unavailable".to_owned())?;
    guard.children.push(child);
    let mut reader = BufReader::new(stdout);
    read_handshake(&mut reader, "CONNECTED")?;
    Ok((stdin, reader))
}

fn client_exchange(
    stdin: &mut std::process::ChildStdin,
    stdout: &mut BufReader<std::process::ChildStdout>,
    payload: &str,
) -> Result<String, String> {
    writeln!(stdin, "{payload}").map_err(|error| format!("could not send fixed test payload: {error}"))?;
    stdin
        .flush()
        .map_err(|error| format!("could not flush fixed test payload: {error}"))?;
    let mut reply = String::new();
    let count = stdout
        .read_line(&mut reply)
        .map_err(|error| format!("could not read the client probe result: {error}"))?;
    if count == 0 {
        return Err("client exited without reporting the TCP probe result".to_owned());
    }
    Ok(reply.trim().to_owned())
}

fn check_new_session(namespace: &str, executable: &std::path::Path) -> Result<(bool, u64), String> {
    let started = Instant::now();
    let output = Command::new("ip")
        .args(["netns", "exec", namespace])
        .arg(executable)
        .arg("--connect-check")
        .output()
        .map_err(|error| format!("could not launch the new-session negative control: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "new-session probe failed to run: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let result = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    let elapsed = u64::try_from(started.elapsed().as_millis())
        .map_err(|error| format!("new-session elapsed time overflow: {error}"))?;
    match result.as_str() {
        "BLOCKED" => Ok((true, elapsed)),
        "CONNECTED" => Ok((false, elapsed)),
        _ => Err(format!("unexpected new-session probe output: {result:?}")),
    }
}

fn run_lab() -> Result<ExitCode, String> {
    if env::consts::OS != "linux" {
        return Err("this harness only runs on Linux".to_owned());
    }
    if env::var(ACK_ENV).ok().as_deref() != Some(ACK_VALUE) {
        return Err(format!(
            "refusing to run: set {ACK_ENV}={ACK_VALUE} only for an explicitly authorized disposable namespace lab"
        ));
    }
    for (tool, args) in [("ip", "-Version"), ("nft", "--version")] {
        checked(tool, &[args])?;
    }
    let executable = env::current_exe()
        .map_err(|error| format!("could not determine the lab executable path: {error}"))?;
    let pid = std::process::id();
    let mut guard = LabGuard {
        client_ns: format!("lp-edge-session-c-{pid}"),
        server_ns: format!("lp-edge-session-s-{pid}"),
        children: Vec::new(),
        cleaned: false,
    };
    let client_if = format!("lrc{pid}");
    let server_if = format!("lrs{pid}");
    prepare_namespaces(&guard, &client_if, &server_if)?;
    apply_nft_script(&guard.server_ns, INITIAL_POLICY)?;
    let initial_policy_sha256 = effective_policy(&guard.server_ns, PolicyStage::Initial)?;
    spawn_lab_server(&mut guard, &executable)?;
    let (mut client_stdin, mut client_stdout) = spawn_lab_client(&mut guard, &executable)?;

    let baseline_start = Instant::now();
    let baseline_reply = client_exchange(&mut client_stdin, &mut client_stdout, "baseline")?;
    let baseline_exchange_ms = u64::try_from(baseline_start.elapsed().as_millis())
        .map_err(|error| format!("baseline elapsed time overflow: {error}"))?;
    if baseline_reply != "ACK:baseline" {
        return Err(format!("positive control failed before policy change: {baseline_reply}"));
    }

    let retain_start = Instant::now();
    apply_nft_script(&guard.server_ns, RETAIN_POLICY)?;
    let retain_apply_ms = u64::try_from(retain_start.elapsed().as_millis())
        .map_err(|error| format!("retained-policy apply time overflow: {error}"))?;
    let retained_policy_sha256 =
        effective_policy(&guard.server_ns, PolicyStage::RetainEstablished)?;
    let retained_reply = client_exchange(&mut client_stdin, &mut client_stdout, "retained")?;
    if retained_reply != "ACK:retained" {
        return Err(format!(
            "established-session control should still pass under the retained-state exception, got {retained_reply:?}"
        ));
    }
    let (new_session_blocked_after_admission_removal, new_session_check_ms) =
        check_new_session(&guard.client_ns, &executable)?;
    if !new_session_blocked_after_admission_removal {
        return Err("a fresh TCP/8080 session connected after its admission rule was removed".to_owned());
    }
    ensure_server_alive(&mut guard)?;

    let revoke_start = Instant::now();
    apply_nft_script(&guard.server_ns, REVOKE_POLICY)?;
    let revoke_apply_ms = u64::try_from(revoke_start.elapsed().as_millis())
        .map_err(|error| format!("revocation policy apply time overflow: {error}"))?;
    let revoked_policy_sha256 = effective_policy(&guard.server_ns, PolicyStage::ExplicitRevoke)?;
    let revocation_observation_start = Instant::now();
    let revoked_reply = client_exchange(&mut client_stdin, &mut client_stdout, "revoked")?;
    let revocation_observation_ms =
        u64::try_from(revocation_observation_start.elapsed().as_millis())
            .map_err(|error| format!("revocation observation time overflow: {error}"))?;
    if revoked_reply != "TIMEOUT" {
        return Err(format!(
            "explicit tuple-specific revocation should stop the established flow, got {revoked_reply:?}"
        ));
    }
    let (new_session_blocked_after_explicit_revoke, final_new_session_check_ms) =
        check_new_session(&guard.client_ns, &executable)?;
    if !new_session_blocked_after_explicit_revoke {
        return Err("a fresh TCP/8080 session connected after explicit revocation".to_owned());
    }
    ensure_server_alive(&mut guard)?;

    let summary: Value = json!({
        "schema_version": "edge-policy-session-revocation-observation-v1",
        "evidence_class": "namespace_lab_observation",
        "claim_ceiling": "isolated_virtual_linux_namespace_behavior_not_physical_or_authenticated_evidence",
        "profile": {
            "client_address": "198.18.1.1/30",
            "server_address": "198.18.1.2/30",
            "transport": "tcp",
            "port": 8080,
            "route_scope": "two_ephemeral_namespaces_connected_by_one_veth_without_default_routes",
            "host_firewall_changed": false,
            "external_route_configured": false,
            "packet_capture_used": false
        },
        "observations": [
            {
                "case_id": "ESR-001",
                "condition": "initial_allow_positive_control",
                "expected": "permitted",
                "observed": "permitted",
                "baseline_round_trip_ms": baseline_exchange_ms,
                "effective_policy_sha256": initial_policy_sha256
            },
            {
                "case_id": "ESR-002",
                "condition": "remove_new_session_admission_but_keep_generic_established_exception",
                "expected": "existing_permitted_new_blocked",
                "existing_session_observed": "permitted",
                "new_session_observed": "blocked",
                "new_session_probe_ms": new_session_check_ms,
                "effective_policy_sha256": retained_policy_sha256
            },
            {
                "case_id": "ESR-003",
                "condition": "explicit_source_and_service_deny_precedes_generic_established_exception",
                "expected": "existing_blocked_new_blocked",
                "existing_session_observed": "no_reply_within_timeout_window",
                "new_session_observed": "blocked",
                "response_timeout_window_ms": CLIENT_TIMEOUT.as_millis(),
                "observed_wait_ms": revocation_observation_ms,
                "new_session_probe_ms": final_new_session_check_ms,
                "effective_policy_sha256": revoked_policy_sha256
            }
        ],
        "policy_installation_ms": {
            "remove_admission_keep_established": retain_apply_ms,
            "explicit_revocation": revoke_apply_ms,
            "note": "local nft command duration only; not a fleet rollout or end-to-end SLA"
        },
        "result": "all_three_semantics_checked",
        "cleanup": "pending"
    });
    guard.cleanup()?;
    let mut finalized = summary;
    finalized["cleanup"] = Value::String("temporary_namespaces_removed_and_verified".to_owned());
    println!(
        "{}",
        serde_json::to_string_pretty(&finalized)
            .map_err(|error| format!("could not encode JSON observation: {error}"))?
    );
    Ok(ExitCode::SUCCESS)
}

fn run_server() -> Result<(), String> {
    let listener = TcpListener::bind(SERVER_ADDR)
        .map_err(|error| format!("could not bind fixed echo service at {SERVER_ADDR}: {error}"))?;
    println!("READY");
    io::stdout()
        .flush()
        .map_err(|error| format!("could not flush server readiness marker: {error}"))?;
    for incoming in listener.incoming() {
        let stream = incoming.map_err(|error| format!("listener accept failed: {error}"))?;
        let _ = thread::Builder::new()
            .name("edge-session-echo".to_owned())
            .spawn(move || {
                let _ = serve_connection(stream);
            });
    }
    Ok(())
}

fn serve_connection(stream: TcpStream) -> Result<(), String> {
    let read_stream = stream
        .try_clone()
        .map_err(|error| format!("could not clone echo socket: {error}"))?;
    let mut reader = BufReader::new(read_stream);
    let mut writer = stream;
    loop {
        let mut line = String::new();
        let count = reader
            .read_line(&mut line)
            .map_err(|error| format!("echo server read failed: {error}"))?;
        if count == 0 {
            return Ok(());
        }
        write!(writer, "ACK:{}", line.trim_end())
            .and_then(|()| writeln!(writer))
            .map_err(|error| format!("echo server write failed: {error}"))?;
        writer
            .flush()
            .map_err(|error| format!("echo server flush failed: {error}"))?;
    }
}

fn run_client() -> Result<(), String> {
    let mut stream = TcpStream::connect(SERVER_ADDR)
        .map_err(|error| format!("positive TCP connection could not be established: {error}"))?;
    stream
        .set_read_timeout(Some(CLIENT_TIMEOUT))
        .map_err(|error| format!("could not set bounded client read timeout: {error}"))?;
    let read_stream = stream
        .try_clone()
        .map_err(|error| format!("could not clone client TCP stream: {error}"))?;
    let mut reader = BufReader::new(read_stream);
    println!("CONNECTED");
    io::stdout()
        .flush()
        .map_err(|error| format!("could not flush client connection marker: {error}"))?;

    for input in io::stdin().lock().lines() {
        let payload = input.map_err(|error| format!("could not read fixed test command: {error}"))?;
        writeln!(stream, "{payload}")
            .map_err(|error| format!("could not send fixed test payload: {error}"))?;
        stream
            .flush()
            .map_err(|error| format!("could not flush fixed test payload: {error}"))?;
        let mut reply = String::new();
        match reader.read_line(&mut reply) {
            Ok(count) if count > 0 => println!("{}", reply.trim()),
            Ok(_) => {
                println!("EOF");
                io::stdout()
                    .flush()
                    .map_err(|error| format!("could not flush client EOF result: {error}"))?;
                break;
            }
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) =>
            {
                println!("TIMEOUT");
                io::stdout()
                    .flush()
                    .map_err(|error| format!("could not flush client timeout result: {error}"))?;
                break;
            }
            Err(error) => return Err(format!("client response read failed: {error}")),
        }
        io::stdout()
            .flush()
            .map_err(|error| format!("could not flush client response: {error}"))?;
    }
    Ok(())
}

fn run_connect_check() -> Result<(), String> {
    let address = SERVER_ADDR
        .parse()
        .map_err(|error| format!("fixed server address was invalid: {error}"))?;
    match TcpStream::connect_timeout(&address, Duration::from_millis(300)) {
        Ok(_stream) => println!("CONNECTED"),
        Err(_error) => println!("BLOCKED"),
    }
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<ExitCode, String> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    match arguments.as_slice() {
        [mode] if mode == "--server" => {
            run_server()?;
            Ok(ExitCode::SUCCESS)
        }
        [mode] if mode == "--client" => {
            run_client()?;
            Ok(ExitCode::SUCCESS)
        }
        [mode] if mode == "--connect-check" => {
            run_connect_check()?;
            Ok(ExitCode::SUCCESS)
        }
        [confirmation] if confirmation == "--confirm-session-revocation-lab" => run_lab(),
        [] => Err(
            "refusing to run without --confirm-session-revocation-lab; helper modes are private to the isolated runner"
                .to_owned(),
        ),
        _ => Err("unsupported arguments; caller-selected targets and policy commands are not accepted".to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use super::{PolicyStage, validate_policy_text};

    const HEADER: &str = "table inet luminous_session_lab {\n chain input { type filter hook input priority 0; policy drop;\n iifname \"lo\" accept\n";
    const ESTABLISHED: &str = " ct state established,related accept\n";
    const PORT_ALLOW: &str = " tcp dport 8080 accept\n";
    const REVOKE: &str = " ip saddr 198.18.1.1 tcp dport 8080 drop\n";
    const FOOTER: &str = " }\n}\n";

    #[test]
    fn initial_policy_has_positive_and_established_controls() {
        let ruleset = format!("{HEADER}{ESTABLISHED}{PORT_ALLOW}{FOOTER}");
        assert!(validate_policy_text(PolicyStage::Initial, &ruleset).is_ok());
    }

    #[test]
    fn removing_admission_does_not_claim_to_revoke_existing_sessions() {
        let ruleset = format!("{HEADER}{ESTABLISHED}{FOOTER}");
        assert!(validate_policy_text(PolicyStage::RetainEstablished, &ruleset).is_ok());
        assert!(validate_policy_text(PolicyStage::ExplicitRevoke, &ruleset).is_err());
    }

    #[test]
    fn explicit_deny_must_precede_generic_established_accept() {
        let correct = format!("{HEADER}{REVOKE}{ESTABLISHED}{FOOTER}");
        assert!(validate_policy_text(PolicyStage::ExplicitRevoke, &correct).is_ok());

        let wrong_order = format!("{HEADER}{ESTABLISHED}{REVOKE}{FOOTER}");
        assert!(validate_policy_text(PolicyStage::ExplicitRevoke, &wrong_order).is_err());
    }

    #[test]
    fn retained_policy_must_not_keep_the_new_session_allow_rule() {
        let ruleset = format!("{HEADER}{ESTABLISHED}{PORT_ALLOW}{FOOTER}");
        assert!(validate_policy_text(PolicyStage::RetainEstablished, &ruleset).is_err());
    }

    #[test]
    fn policy_default_must_remain_drop() {
        let ruleset = format!(
            "table inet luminous_session_lab {{\n chain input {{ type filter hook input priority 0; policy accept;\n{ESTABLISHED}{REVOKE}{FOOTER}"
        );
        assert!(validate_policy_text(PolicyStage::ExplicitRevoke, &ruleset).is_err());
    }
}
