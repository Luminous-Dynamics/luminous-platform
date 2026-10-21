use std::env;
use std::fs;
use std::process::ExitCode;

use serde::Serialize;
use srq_eat_cose_verifier::{
    verify_cose_eat, ChallengeFile, ExpectedChallenge, TrustedKey, TrustPolicyFile,
};

#[derive(Serialize)]
struct CliResult<'a> {
    schema: &'static str,
    status: &'static str,
    release_gate_eligible: bool,
    replay_protection: &'static str,
    hardware_measurement: &'static str,
    observation: &'a srq_eat_cose_verifier::VerifiedObservation,
}

fn usage() {
    eprintln!("Usage: srq-eat-cose-verify --token FILE --trust-policy JSON --challenge JSON --now-epoch UNIX_SECONDS");
    eprintln!("Checks D1 EAT/COSE Ed25519 signature and verifier-bound claims.");
    eprintln!("Output is not replay-safe until a durable challenge store consumes the nonce.");
    eprintln!("D1 does not claim TPM, secure-boot measurement or physical qualification.");
}

fn read_limited(path: &str, limit: usize) -> Result<Vec<u8>, String> {
    let bytes = fs::read(path).map_err(|e| format!("read {path}: {e}"))?;
    if bytes.is_empty() || bytes.len() > limit {
        return Err(format!("{path} is empty or exceeds the {}-byte limit", limit));
    }
    Ok(bytes)
}

fn run() -> Result<(), String> {
    let mut token_path = None;
    let mut trust_path = None;
    let mut challenge_path = None;
    let mut now = None;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--token" => token_path = args.next(),
            "--trust-policy" => trust_path = args.next(),
            "--challenge" => challenge_path = args.next(),
            "--now-epoch" => {
                let value = args.next().ok_or("--now-epoch requires a value")?;
                now = Some(value.parse::<u64>().map_err(|_| "--now-epoch must be an unsigned integer")?);
            }
            "--help" | "-h" => {
                usage();
                return Ok(());
            }
            _ => return Err(format!("unknown argument: {arg}")),
        }
    }

    let token_path = token_path.ok_or("missing --token")?;
    let trust_path = trust_path.ok_or("missing --trust-policy")?;
    let challenge_path = challenge_path.ok_or("missing --challenge")?;
    let now = now.ok_or("missing --now-epoch; provide verifier-controlled time")?;

    let token = read_limited(&token_path, srq_eat_cose_verifier::MAX_TOKEN_BYTES)?;
    let trust_bytes = read_limited(&trust_path, 16 * 1024)?;
    let challenge_bytes = read_limited(&challenge_path, 16 * 1024)?;
    let trust: TrustPolicyFile = serde_json::from_slice(&trust_bytes)
        .map_err(|e| format!("trust policy JSON invalid: {e}"))?;
    let challenge_file: ChallengeFile = serde_json::from_slice(&challenge_bytes)
        .map_err(|e| format!("challenge JSON invalid: {e}"))?;
    let trust = TrustedKey::try_from(trust)?;
    let challenge = ExpectedChallenge::try_from(challenge_file)?;

    let observation = verify_cose_eat(&token, &trust, &challenge, now)?;
    let out = CliResult {
        schema: "srq-eat-cose-cli-result-v1",
        status: "ENVELOPE_AND_BINDINGS_VERIFIED_NOT_REPLAY_SAFE",
        release_gate_eligible: false,
        replay_protection: "NOT_PERFORMED_DURABLE_CONSUMER_REQUIRED",
        hardware_measurement: "NOT_CLAIMED_D1_SOFTWARE_KEY_ONLY",
        observation: &observation,
    };
    println!("{}", serde_json::to_string_pretty(&out).map_err(|e| e.to_string())?);
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("BLOCKED: {error}");
            ExitCode::from(1)
        }
    }
}
