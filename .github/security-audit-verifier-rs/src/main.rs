use chrono::{DateTime, Utc};
use reqwest::blocking::{Client, Response};
use reqwest::redirect::Policy as RedirectPolicy;
use reqwest::{Method, StatusCode, Url};
use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::env;
use std::fmt;
use std::io::{Cursor, Read};
use std::time::Duration;
use zip::ZipArchive;

const API: &str = "https://api.github.com";
const API_VERSION: &str = "2022-11-28";
const STATUS_CONTEXT: &str = "Security Audit / Independent Verifier";
const ENGINE_REPO: &str = "Luminous-Dynamics/luminous-platform";
const ENGINE_SHA: &str = "15f8135368e6162ca9b07861713a567f95fe765a";
const ENGINE_PATH: &str = ".github/workflows/security-audit.yml";
const ENGINE_BLOB: &str = "6d8f9981822a65835550895b4964337182cf3fb5";
const MAX_API_BODY: usize = 5 * 1024 * 1024;
const MAX_ZIP_BYTES: usize = 100 * 1024 * 1024;
const MAX_ZIP_ENTRIES: usize = 4096;
const MAX_ZIP_UNPACKED: u64 = 50 * 1024 * 1024;
const MAX_VERDICT_BYTES: u64 = 1024 * 1024;
const MAX_VERDICT_AGE_SECS: i64 = 7 * 24 * 60 * 60;
const FUTURE_SKEW_SECS: i64 = 5 * 60;

type VerifyResult<T> = Result<T, String>;

#[derive(Clone, Copy)]
struct Policy {
    workflow_id: u64,
    workflow_name: &'static str,
    workflow_path: &'static str,
    workflow_blob: &'static str,
    engine_sha: Option<&'static str>,
    audit_rust: bool,
    audit_node: bool,
}

fn policy(repo: &str) -> Option<Policy> {
    match repo {
        "Luminous-Dynamics/mycelix" => Some(Policy {
            workflow_id: 379572428,
            workflow_name: "Security Audit",
            workflow_path: ".github/workflows/security-audit.yml",
            workflow_blob: "d7660286694f4f0369120506c6089e544cdc54cb",
            engine_sha: Some(ENGINE_SHA),
            audit_rust: true,
            audit_node: true,
        }),
        "Luminous-Dynamics/symthaea" => Some(Policy {
            workflow_id: 379572736,
            workflow_name: "Security Audit",
            workflow_path: ".github/workflows/security-audit.yml",
            workflow_blob: "f24de1c8d249c365498b78c8ad97557e666d97b0",
            engine_sha: Some(ENGINE_SHA),
            audit_rust: true,
            audit_node: false,
        }),
        "Luminous-Dynamics/luminous-platform" => Some(Policy {
            workflow_id: 379571857,
            workflow_name: "Security Audit — Platform Self-Check",
            workflow_path: ".github/workflows/security-audit-self.yml",
            workflow_blob: "8f621751d2c6ac255250f47d2ea4b71b99a9c6c4",
            engine_sha: None,
            audit_rust: false,
            audit_node: false,
        }),
        _ => None,
    }
}

fn canonical_sha(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn sha_or_error(value: &str, label: &str) -> VerifyResult<String> {
    if canonical_sha(value) {
        Ok(value.to_owned())
    } else {
        Err(format!("{label} is not a canonical lowercase Git SHA"))
    }
}

fn digest_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn api_json(method: Method, path: &str, token: &str, body: Option<Value>) -> VerifyResult<Value> {
    let client = Client::builder()
        .timeout(Duration::from_secs(25))
        .redirect(RedirectPolicy::none())
        .user_agent("luminous-security-audit-verifier-rs/1")
        .build()
        .map_err(|e| format!("could not build GitHub API client: {e}"))?;
    let url = format!("{API}{path}");
    let mut request = client
        .request(method, &url)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", API_VERSION)
        .bearer_auth(token);
    if let Some(payload) = body {
        request = request.json(&payload);
    }
    let response = request
        .send()
        .map_err(|e| format!("GitHub API request failed for {path}: {e}"))?;
    read_api_json(response, path)
}

fn read_api_json(response: Response, path: &str) -> VerifyResult<Value> {
    let status = response.status();
    let mut limited = response.take((MAX_API_BODY + 1) as u64);
    let mut bytes = Vec::new();
    limited
        .read_to_end(&mut bytes)
        .map_err(|e| format!("could not read GitHub API response for {path}: {e}"))?;
    if bytes.len() > MAX_API_BODY {
        return Err(format!("GitHub API response for {path} exceeded 5 MiB"));
    }
    if !status.is_success() {
        let detail = String::from_utf8_lossy(&bytes);
        return Err(format!(
            "GitHub API {path} returned HTTP {}: {}",
            status.as_u16(),
            detail.chars().take(1200).collect::<String>()
        ));
    }
    if bytes.is_empty() {
        return Ok(Value::Null);
    }
    serde_json::from_slice(&bytes)
        .map_err(|e| format!("GitHub API returned invalid JSON for {path}: {e}"))
}

fn get_field_str<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn find_open_pr(
    repo: &str,
    subject: &str,
    branch: &str,
    default_branch: &str,
    token: &str,
) -> VerifyResult<Option<Value>> {
    let path = format!("/repos/{repo}/commits/{subject}/pulls?per_page=100");
    let rows = api_json(Method::GET, &path, token, None)?;
    let rows = rows
        .as_array()
        .ok_or_else(|| "commit-to-PR response has an unexpected shape".to_string())?;
    let mut matches = Vec::new();
    for pr in rows {
        let head = pr.get("head").unwrap_or(&Value::Null);
        let base = pr.get("base").unwrap_or(&Value::Null);
        let head_repo = head.get("repo").unwrap_or(&Value::Null);
        let base_repo = base.get("repo").unwrap_or(&Value::Null);
        let same_identity = get_field_str(pr, "state") == Some("open")
            && get_field_str(head, "sha") == Some(subject)
            && get_field_str(head, "ref") == Some(branch)
            && get_field_str(head_repo, "full_name")
                .is_some_and(|v| v.eq_ignore_ascii_case(repo))
            && get_field_str(base, "ref") == Some(default_branch)
            && get_field_str(base_repo, "full_name")
                .is_some_and(|v| v.eq_ignore_ascii_case(repo));
        if same_identity {
            matches.push(pr.clone());
        }
    }
    if matches.len() > 1 {
        return Err("ambiguous open same-repository PRs match the exact subject".to_string());
    }
    Ok(matches.pop())
}

fn is_latest_run(candidate: &Value, runs: &[Value], policy: Policy, subject: &str) -> bool {
    let eligible: Vec<&Value> = runs
        .iter()
        .filter(|run| {
            run.get("workflow_id").and_then(Value::as_u64) == Some(policy.workflow_id)
                && get_field_str(run, "event") == Some("pull_request")
                && get_field_str(run, "head_sha") == Some(subject)
                && run.get("id").and_then(Value::as_u64).is_some()
        })
        .collect();
    let latest = eligible.into_iter().max_by_key(|run| {
        (
            run.get("run_number").and_then(Value::as_u64).unwrap_or(0),
            run.get("run_attempt").and_then(Value::as_u64).unwrap_or(0),
            get_field_str(run, "created_at").unwrap_or("").to_owned(),
            run.get("id").and_then(Value::as_u64).unwrap_or(0),
        )
    });
    latest.is_some_and(|latest| {
        latest.get("id") == candidate.get("id")
            && latest.get("run_attempt") == candidate.get("run_attempt")
    })
}

fn check_run_binding(
    run: &Value,
    repo: &str,
    policy: Policy,
    branch: &str,
    expected_sha: &str,
    expected_attempt: Option<u64>,
) -> VerifyResult<String> {
    if run.get("workflow_id").and_then(Value::as_u64) != Some(policy.workflow_id)
        || get_field_str(run, "name") != Some(policy.workflow_name)
    {
        return Err("workflow ID/name does not match base-owned policy".to_string());
    }
    let path = get_field_str(run, "path").unwrap_or("");
    if path.split('@').next() != Some(policy.workflow_path) {
        return Err("workflow path does not match base-owned policy".to_string());
    }
    if get_field_str(run, "event") != Some("pull_request") {
        return Err("only pull_request runs can qualify a merge".to_string());
    }
    if get_field_str(run, "head_branch") != Some(branch) {
        return Err("run branch does not match the exact open PR head".to_string());
    }
    let subject = sha_or_error(get_field_str(run, "head_sha").unwrap_or(""), "run.head_sha")?;
    if subject != expected_sha {
        return Err("run head SHA differs from event's exact SHA".to_string());
    }
    if let Some(expected) = expected_attempt {
        if run.get("run_attempt").and_then(Value::as_u64) != Some(expected) {
            return Err("run attempt differs from triggering event".to_string());
        }
    }
    let head_repo = run.get("head_repository").unwrap_or(&Value::Null);
    let base_repo = run.get("repository").unwrap_or(&Value::Null);
    if !get_field_str(head_repo, "full_name").is_some_and(|v| v.eq_ignore_ascii_case(repo)) {
        return Err("fork-originated runs are not eligible".to_string());
    }
    if !get_field_str(base_repo, "full_name").is_some_and(|v| v.eq_ignore_ascii_case(repo)) {
        return Err("run repository differs from policy repository".to_string());
    }
    Ok(subject)
}

fn check_blob(repo: &str, path: &str, reference: &str, expected_blob: &str, token: &str) -> VerifyResult<()> {
    let path = path.split('/').map(|part| url_encode_path_segment(part)).collect::<Vec<_>>().join("/");
    let url = format!("/repos/{repo}/contents/{path}?ref={reference}");
    let item = api_json(Method::GET, &url, token, None)?;
    if get_field_str(&item, "type") != Some("file")
        || get_field_str(&item, "sha") != Some(expected_blob)
    {
        return Err(format!("source blob missing or differs from reviewed pin: {repo}:{path}"));
    }
    Ok(())
}

fn url_encode_path_segment(input: &str) -> String {
    let mut out = String::new();
    for byte in input.bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

fn post_status(
    repo: &str,
    subject: &str,
    token: &str,
    state: &str,
    description: &str,
    target: Option<&str>,
) -> VerifyResult<()> {
    let mut payload = json!({
        "state": state,
        "context": STATUS_CONTEXT,
        "description": description.chars().take(140).collect::<String>()
    });
    if let Some(target) = target.filter(|s| s.starts_with("https://")) {
        payload["target_url"] = Value::String(target.to_owned());
    }
    let path = format!("/repos/{repo}/statuses/{subject}");
    api_json(Method::POST, &path, token, Some(payload)).map(|_| ())
}

fn is_safe_signed_url(raw_url: &str) -> VerifyResult<Url> {
    let url = Url::parse(raw_url).map_err(|e| format!("artifact signed URL is invalid: {e}"))?;
    if url.scheme() != "https" || !url.username().is_empty() || url.password().is_some() {
        return Err("artifact signed URL must be HTTPS and contain no URL credentials".to_string());
    }
    let host = url.host_str().unwrap_or("").to_ascii_lowercase();
    let trusted_host = host.ends_with(".blob.core.windows.net")
        || host.ends_with(".amazonaws.com")
        || host.ends_with(".actions.githubusercontent.com")
        || host.ends_with(".githubusercontent.com")
        || host == "github.com";
    if !trusted_host {
        return Err("artifact signed URL host is outside the allowed GitHub artifact-storage domains".to_string());
    }
    Ok(url)
}

fn download_artifact_zip(repo: &str, artifact_id: u64, token: &str) -> VerifyResult<Vec<u8>> {
    let url = format!("{API}/repos/{repo}/actions/artifacts/{artifact_id}/zip");
    let api_client = Client::builder()
        .timeout(Duration::from_secs(25))
        .redirect(RedirectPolicy::none())
        .user_agent("luminous-security-audit-verifier-rs/1")
        .build()
        .map_err(|e| format!("could not build artifact API client: {e}"))?;
    let response = api_client
        .get(url)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", API_VERSION)
        .bearer_auth(token)
        .send()
        .map_err(|e| format!("artifact download API unavailable: {e}"))?;
    if response.status() != StatusCode::FOUND {
        let status = response.status();
        let detail = response
            .text()
            .unwrap_or_else(|_| String::new())
            .chars()
            .take(1200)
            .collect::<String>();
        return Err(format!("artifact download API returned HTTP {}: {detail}", status.as_u16()));
    }
    let location = response
        .headers()
        .get(reqwest::header::LOCATION)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| "artifact API omitted a valid signed redirect URL".to_string())?;
    let signed_url = is_safe_signed_url(location)?;
    // Deliberately use a separate client with no GitHub token for the signed URL.
    let storage_client = Client::builder()
        .timeout(Duration::from_secs(60))
        .user_agent("luminous-security-audit-verifier-rs/1")
        .build()
        .map_err(|e| format!("could not build signed storage client: {e}"))?;
    let response = storage_client
        .get(signed_url)
        .send()
        .map_err(|e| format!("artifact archive download failed: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("artifact storage returned HTTP {}", response.status().as_u16()));
    }
    if response.content_length().is_some_and(|len| len > MAX_ZIP_BYTES as u64) {
        return Err("artifact archive exceeds the 100 MiB verifier limit".to_string());
    }
    let mut limited = response.take((MAX_ZIP_BYTES + 1) as u64);
    let mut bytes = Vec::new();
    limited
        .read_to_end(&mut bytes)
        .map_err(|e| format!("could not read artifact archive: {e}"))?;
    if bytes.len() > MAX_ZIP_BYTES {
        return Err("artifact archive exceeds the 100 MiB verifier limit".to_string());
    }
    Ok(bytes)
}

struct UniqueJson(Value);

struct UniqueVisitor;

impl<'de> Visitor<'de> for UniqueVisitor {
    type Value = UniqueJson;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("valid JSON with unique object keys")
    }

    fn visit_bool<E: de::Error>(self, value: bool) -> Result<Self::Value, E> {
        Ok(UniqueJson(Value::Bool(value)))
    }

    fn visit_i64<E: de::Error>(self, value: i64) -> Result<Self::Value, E> {
        Ok(UniqueJson(Value::Number(value.into())))
    }

    fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
        Ok(UniqueJson(Value::Number(value.into())))
    }

    fn visit_f64<E: de::Error>(self, value: f64) -> Result<Self::Value, E> {
        let number = serde_json::Number::from_f64(value)
            .ok_or_else(|| E::custom("non-finite JSON number"))?;
        Ok(UniqueJson(Value::Number(number)))
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
        Ok(UniqueJson(Value::String(value.to_owned())))
    }

    fn visit_string<E: de::Error>(self, value: String) -> Result<Self::Value, E> {
        Ok(UniqueJson(Value::String(value)))
    }

    fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(UniqueJson(Value::Null))
    }

    fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(UniqueJson(Value::Null))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut values = Vec::new();
        while let Some(value) = seq.next_element::<UniqueJson>()? {
            values.push(value.0);
        }
        Ok(UniqueJson(Value::Array(values)))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut values = Map::new();
        while let Some(key) = map.next_key::<String>()? {
            if values.contains_key(&key) {
                return Err(de::Error::custom(format!("duplicate JSON object key: {key}")));
            }
            let value = map.next_value::<UniqueJson>()?;
            values.insert(key, value.0);
        }
        Ok(UniqueJson(Value::Object(values)))
    }
}

impl<'de> Deserialize<'de> for UniqueJson {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(UniqueVisitor)
    }
}

fn parse_unique_json(bytes: &[u8]) -> VerifyResult<Value> {
    serde_json::from_slice::<UniqueJson>(bytes)
        .map(|value| value.0)
        .map_err(|e| format!("verdict JSON is invalid or ambiguous: {e}"))
}

fn canonical_relative_path(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.contains('\\')
        && !path.chars().any(char::is_control)
        && path.split('/').all(|part| !part.is_empty() && part != "." && part != "..")
}

fn evidence_archive_path(raw: &str) -> VerifyResult<String> {
    if !canonical_relative_path(raw) {
        return Err("artifact ZIP contains an unsafe or non-canonical path".to_string());
    }
    if raw.rsplit('/').next() == Some("verdict.json") {
        return Err("verdict.json must be stored at the artifact ZIP root".to_string());
    }
    let normalized = raw.strip_prefix("audit-evidence/").unwrap_or(raw);
    if !canonical_relative_path(normalized) || normalized == "verdict.json" {
        return Err("artifact ZIP contains an invalid evidence path".to_string());
    }
    Ok(normalized.to_owned())
}

fn validate_evidence_files(
    archive: &mut ZipArchive<Cursor<Vec<u8>>>,
    evidence_records: &Value,
    actual_files: &BTreeMap<String, String>,
) -> VerifyResult<()> {
    let records = evidence_records
        .as_array()
        .ok_or_else(|| "verdict evidence manifest is missing or malformed".to_string())?;
    if records.is_empty() {
        return Err("verdict evidence manifest is empty".to_string());
    }
    let mut expected = BTreeMap::<String, String>::new();
    for record in records {
        let object = record
            .as_object()
            .ok_or_else(|| "evidence manifest entry is not an object".to_string())?;
        if object.len() != 2 || !object.contains_key("path") || !object.contains_key("sha256") {
            return Err("evidence manifest entry has unexpected or missing fields".to_string());
        }
        let path = get_field_str(record, "path").unwrap_or("");
        let digest = get_field_str(record, "sha256").unwrap_or("");
        if !canonical_relative_path(path) || path == "verdict.json" {
            return Err("evidence manifest contains an unsafe or non-canonical path".to_string());
        }
        if digest.len() != 64
            || !digest.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(format!("evidence manifest has an invalid digest for {path}"));
        }
        if expected.insert(path.to_owned(), digest.to_owned()).is_some() {
            return Err(format!("evidence manifest duplicates path {path}"));
        }
    }
    let expected_paths: BTreeSet<&str> = expected.keys().map(String::as_str).collect();
    let actual_paths: BTreeSet<&str> = actual_files.keys().map(String::as_str).collect();
    if expected_paths != actual_paths {
        return Err("evidence manifest does not exactly cover the archive's evidence files".to_string());
    }
    for (path, expected_digest) in expected {
        let archive_name = actual_files
            .get(&path)
            .ok_or_else(|| format!("evidence file {path} is missing"))?;
        let mut file = archive
            .by_name(archive_name)
            .map_err(|_| format!("evidence file {path} cannot be read"))?;
        let declared_size = file.size();
        if declared_size > MAX_ZIP_UNPACKED {
            return Err(format!("evidence file {path} exceeds the unpacked-size limit"));
        }
        let mut content = Vec::with_capacity(declared_size as usize);
        file.by_ref()
            .take(declared_size + 1)
            .read_to_end(&mut content)
            .map_err(|e| format!("evidence file {path} cannot be decoded: {e}"))?;
        if content.len() as u64 != declared_size {
            return Err(format!("evidence file {path} size differs from ZIP metadata"));
        }
        if digest_hex(&content) != expected_digest {
            return Err(format!("evidence file digest mismatch for {path}"));
        }
    }
    Ok(())
}

fn verify_artifact(repo: &str, policy: Policy, run: &Value, expected_pr: u64, token: &str) -> VerifyResult<()> {
    let run_id = run.get("id").and_then(Value::as_u64)
        .ok_or_else(|| "workflow run ID is missing or invalid".to_string())?;
    let subject = sha_or_error(get_field_str(run, "head_sha").unwrap_or(""), "run.head_sha")?;
    let repo_name = repo.rsplit('/').next().unwrap_or(repo);
    let expected_name = format!("security-audit-{repo_name}-{subject}-verdict");
    let path = format!("/repos/{repo}/actions/runs/{run_id}/artifacts?per_page=100&page=1");
    let payload = api_json(Method::GET, &path, token, None)?;
    let artifacts = payload.get("artifacts").and_then(Value::as_array)
        .ok_or_else(|| "artifact listing response has an unexpected shape".to_string())?;
    let matches: Vec<&Value> = artifacts.iter()
        .filter(|artifact| get_field_str(artifact, "name") == Some(expected_name.as_str()))
        .collect();
    if matches.len() != 1 {
        return Err(format!("expected exactly one aggregate verdict artifact, found {}", matches.len()));
    }
    let artifact = matches[0];
    let artifact_run = artifact.get("workflow_run").unwrap_or(&Value::Null);
    if artifact.get("expired").and_then(Value::as_bool) != Some(false)
        || artifact_run.get("id").and_then(Value::as_u64) != Some(run_id)
        || get_field_str(artifact_run, "head_sha") != Some(subject.as_str())
    {
        return Err("verdict artifact is expired or bound to a different run/head".to_string());
    }
    let expected_digest = get_field_str(artifact, "digest").unwrap_or("");
    if expected_digest.len() != 71 || !expected_digest.starts_with("sha256:")
        || !expected_digest[7..].bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err("artifact metadata has no canonical SHA-256 digest".to_string());
    }
    let artifact_id = artifact.get("id").and_then(Value::as_u64)
        .filter(|id| *id > 0)
        .ok_or_else(|| "artifact ID is invalid".to_string())?;
    let bytes = download_artifact_zip(repo, artifact_id, token)?;
    if format!("sha256:{}", digest_hex(&bytes)) != expected_digest {
        return Err("downloaded verdict artifact digest differs from GitHub artifact metadata".to_string());
    }
    let mut archive = ZipArchive::new(Cursor::new(bytes))
        .map_err(|e| format!("artifact ZIP is invalid: {e}"))?;
    if archive.is_empty() || archive.len() > MAX_ZIP_ENTRIES {
        return Err("verdict artifact contains an invalid number of entries".to_string());
    }
    let mut total_size = 0u64;
    let mut verdict_count = 0usize;
    let mut actual_files = BTreeMap::<String, String>::new();
    for index in 0..archive.len() {
        let file = archive.by_index(index)
            .map_err(|e| format!("artifact ZIP entry is invalid: {e}"))?;
        if file.is_dir() {
            continue;
        }
        let archive_name = file.name().to_owned();
        if !canonical_relative_path(&archive_name) {
            return Err("artifact ZIP contains an unsafe path".to_string());
        }
        total_size = total_size.saturating_add(file.size());
        if total_size > MAX_ZIP_UNPACKED {
            return Err("artifact ZIP unpacked size exceeds 50 MiB".to_string());
        }
        if archive_name == "verdict.json" {
            verdict_count += 1;
            continue;
        }
        let normalized = evidence_archive_path(&archive_name)?;
        if actual_files.insert(normalized.clone(), archive_name).is_some() {
            return Err(format!("artifact ZIP duplicates evidence path {normalized}"));
        }
    }
    if verdict_count != 1 {
        return Err("artifact must contain exactly one root verdict.json".to_string());
    }
    let mut verdict_file = archive.by_name("verdict.json")
        .map_err(|_| "artifact is missing root verdict.json".to_string())?;
    if verdict_file.size() > MAX_VERDICT_BYTES {
        return Err("verdict.json exceeds the 1 MiB parsing limit".to_string());
    }
    let declared_verdict_size = verdict_file.size();
    let mut verdict_bytes = Vec::with_capacity(declared_verdict_size as usize);
    verdict_file.by_ref().take(MAX_VERDICT_BYTES + 1).read_to_end(&mut verdict_bytes)
        .map_err(|e| format!("verdict.json cannot be read: {e}"))?;
    if verdict_bytes.len() as u64 != declared_verdict_size {
        return Err("verdict.json size differs from ZIP metadata".to_string());
    }
    let verdict = parse_unique_json(&verdict_bytes)?;
    validate_evidence_files(&mut archive, verdict.get("evidence_files").unwrap_or(&Value::Null), &actual_files)?;
    validate_verdict(&verdict, repo, policy, run, expected_pr)?;
    Ok(())
}

fn valid_canonical_utc(raw: &str) -> VerifyResult<DateTime<chrono::FixedOffset>> {
    let bytes = raw.as_bytes();
    let canonical_shape = raw.len() == 20
        && bytes.get(4) == Some(&b'-')
        && bytes.get(7) == Some(&b'-')
        && bytes.get(10) == Some(&b'T')
        && bytes.get(13) == Some(&b':')
        && bytes.get(16) == Some(&b':')
        && bytes.get(19) == Some(&b'Z')
        && bytes.iter().enumerate().all(|(i, b)| {
            matches!(i, 4 | 7 | 10 | 13 | 16 | 19) || b.is_ascii_digit()
        });
    if !canonical_shape {
        return Err("verdict timestamp is not canonical UTC".to_string());
    }
    DateTime::parse_from_rfc3339(raw)
        .map_err(|_| "verdict timestamp is not a valid UTC date-time".to_string())
}

fn validate_workflow_ref(raw: &str, repo: &str, path: &str, expected_pr: u64) -> VerifyResult<()> {
    let expected = format!("{repo}/{path}@refs/pull/{expected_pr}/merge");
    if raw != expected {
        return Err("verdict workflow_ref does not match the policy caller and exact PR number".to_string());
    }
    Ok(())
}

fn validate_verdict(verdict: &Value, repo: &str, policy: Policy, run: &Value, expected_pr: u64) -> VerifyResult<()> {
    let object = verdict.as_object().ok_or_else(|| "verdict artifact is not a JSON object".to_string())?;
    let expected_keys: HashSet<&str> = [
        "schema", "repository", "subject_sha", "audit_engine_sha", "workflow_ref", "workflow_sha",
        "workflow_run_url", "generated_at_utc", "aggregate_artifact_retention_days", "run_id",
        "run_attempt", "workflow_job_result", "rustsec_job_result", "npm_job_result", "audit_rust",
        "audit_node", "required_lanes", "non_blocking_findings_present", "non_blocking_finding_sources",
        "status", "failure_reasons", "evidence_files",
    ].into_iter().collect();
    let actual_keys: HashSet<&str> = object.keys().map(String::as_str).collect();
    if actual_keys != expected_keys {
        return Err("verdict fields differ from the exact v1 contract".to_string());
    }
    let subject = sha_or_error(get_field_str(run, "head_sha").unwrap_or(""), "run.head_sha")?;
    let run_id = run.get("id").and_then(Value::as_u64)
        .ok_or_else(|| "authoritative run ID is invalid".to_string())?;
    let attempt = run.get("run_attempt").and_then(Value::as_u64)
        .ok_or_else(|| "authoritative run attempt is invalid".to_string())?;
    if get_field_str(verdict, "schema") != Some("luminous.security-audit.verdict.v1")
        || get_field_str(verdict, "repository") != Some(repo)
        || get_field_str(verdict, "subject_sha") != Some(subject.as_str())
    {
        return Err("verdict schema/repository/subject does not match the authoritative run".to_string());
    }
    let run_id_text = run_id.to_string();
    let attempt_text = attempt.to_string();
    if get_field_str(verdict, "run_id") != Some(run_id_text.as_str())
        || get_field_str(verdict, "run_attempt") != Some(attempt_text.as_str())
        || get_field_str(verdict, "workflow_job_result") != Some("success")
    {
        return Err("verdict run ID/attempt or workflow-security result is inconsistent".to_string());
    }
    validate_workflow_ref(get_field_str(verdict, "workflow_ref").unwrap_or(""), repo, policy.workflow_path, expected_pr)?;
    sha_or_error(get_field_str(verdict, "workflow_sha").unwrap_or(""), "verdict.workflow_sha")?;
    let run_url = get_field_str(run, "html_url").ok_or_else(|| "authoritative run URL is missing".to_string())?;
    if !run_url.starts_with("https://") || get_field_str(verdict, "workflow_run_url") != Some(run_url) {
        return Err("verdict run URL does not match authoritative GitHub run".to_string());
    }
    let generated = valid_canonical_utc(get_field_str(verdict, "generated_at_utc").unwrap_or(""))?;
    let age = Utc::now().timestamp() - generated.timestamp();
    if age < -FUTURE_SKEW_SECS {
        return Err("verdict timestamp is too far in the future".to_string());
    }
    if age > MAX_VERDICT_AGE_SECS {
        return Err("verdict evidence is older than the seven-day freshness limit".to_string());
    }
    if verdict.get("aggregate_artifact_retention_days").and_then(Value::as_u64) != Some(30) {
        return Err("verdict retention contract differs from v1 policy".to_string());
    }
    if verdict.get("audit_rust").and_then(Value::as_bool) != Some(policy.audit_rust)
        || verdict.get("audit_node").and_then(Value::as_bool) != Some(policy.audit_node)
    {
        return Err("verdict requested-lane flags differ from base-owned coverage policy".to_string());
    }
    let required = verdict.get("required_lanes").and_then(Value::as_object)
        .ok_or_else(|| "verdict required_lanes is missing or malformed".to_string())?;
    let expected_lane_keys: HashSet<&str> = ["workflow_security", "rustsec", "npm"].into_iter().collect();
    if required.keys().map(String::as_str).collect::<HashSet<_>>() != expected_lane_keys {
        return Err("verdict required_lanes differs from the exact v1 lane contract".to_string());
    }
    let expected_rust = if policy.audit_rust { "PASS" } else { "SKIPPED_NOT_REQUESTED" };
    let expected_node = if policy.audit_node { "PASS" } else { "SKIPPED_NOT_REQUESTED" };
    if get_field_str(verdict, "required_lanes").and_then(|_| required.get("workflow_security")).and_then(Value::as_str) != Some("PASS")
        || required.get("rustsec").and_then(Value::as_str) != Some(expected_rust)
        || required.get("npm").and_then(Value::as_str) != Some(expected_node)
    {
        return Err("one or more required dependency-audit lanes are not qualified".to_string());
    }
    let expected_rust_result = if policy.audit_rust { "success" } else { "skipped" };
    let expected_node_result = if policy.audit_node { "success" } else { "skipped" };
    if get_field_str(verdict, "rustsec_job_result") != Some(expected_rust_result)
        || get_field_str(verdict, "npm_job_result") != Some(expected_node_result)
    {
        return Err("dependency-audit job results differ from the required-lane policy".to_string());
    }
    let engine = get_field_str(verdict, "audit_engine_sha").unwrap_or("");
    if let Some(expected_engine) = policy.engine_sha {
        if engine != expected_engine {
            return Err("verdict engine SHA differs from immutable engine pin".to_string());
        }
    } else {
        sha_or_error(engine, "verdict.audit_engine_sha")?;
    }
    let status = get_field_str(verdict, "status").unwrap_or("");
    if status != "PASS" && status != "PASS_WITH_FINDINGS" {
        return Err(format!("producer verdict is {status:?}, not a passing verdict"));
    }
    let failures = verdict.get("failure_reasons").and_then(Value::as_array)
        .ok_or_else(|| "verdict failure_reasons is malformed".to_string())?;
    if !failures.is_empty() {
        return Err("passing verdict has non-empty failure reasons".to_string());
    }
    let sources = verdict.get("non_blocking_finding_sources").and_then(Value::as_array)
        .ok_or_else(|| "non-blocking finding sources are malformed".to_string())?;
    let mut source_names = Vec::new();
    for source in sources {
        let name = source.as_str().ok_or_else(|| "non-blocking finding source is not a string".to_string())?;
        if name != "rustsec_warnings" && name != "npm_below_threshold" {
            return Err("verdict has an unknown non-blocking finding source".to_string());
        }
        if source_names.iter().any(|current| current == name) {
            return Err("verdict duplicates a non-blocking finding source".to_string());
        }
        source_names.push(name.to_owned());
    }
    let nonblocking = verdict.get("non_blocking_findings_present").and_then(Value::as_bool)
        .ok_or_else(|| "non-blocking findings flag is malformed".to_string())?;
    if nonblocking != !source_names.is_empty() {
        return Err("non-blocking findings flag and sources disagree".to_string());
    }
    if (status == "PASS" && nonblocking) || (status == "PASS_WITH_FINDINGS" && !nonblocking) {
        return Err("verdict status is inconsistent with non-blocking findings".to_string());
    }
    if source_names.contains(&"rustsec_warnings".to_string()) && !policy.audit_rust {
        return Err("RustSec finding source was reported although Rust auditing was not requested".to_string());
    }
    if source_names.contains(&"npm_below_threshold".to_string()) && !policy.audit_node {
        return Err("npm finding source was reported although npm auditing was not requested".to_string());
    }
    Ok(())
}

fn should_publish_pending_status(mode: &str, activity: &str, run: &Value) -> bool {
    mode == "workflow_run"
        && (activity == "requested" || activity == "in_progress")
        && matches!(get_field_str(run, "status"), Some("queued" | "in_progress" | "waiting" | "pending" | "requested"))
        && run.get("conclusion").is_none_or(Value::is_null)
}

fn best_effort_failure_status(repo: &str, subject: &str, token: &str, reason: &str) {
    let Some(policy) = policy(repo) else { return; };
    if token.is_empty() || !canonical_sha(subject) {
        return;
    }
    let workflow_id = env::var("TRIGGER_RUN_WORKFLOW_ID").ok().and_then(|v| v.parse::<u64>().ok());
    let event = env::var("TRIGGER_RUN_EVENT").unwrap_or_default();
    let source_repo = env::var("TRIGGER_RUN_REPOSITORY_ID").unwrap_or_default();
    let head_repo = env::var("TRIGGER_RUN_HEAD_REPOSITORY_ID").unwrap_or_default();
    let current_repo = env::var("CURRENT_REPOSITORY_ID").unwrap_or_default();
    let base_ref = env::var("TRIGGER_RUN_PR_BASE_REF").unwrap_or_default();
    let default_branch = env::var("DEFAULT_BRANCH").unwrap_or_else(|_| "main".to_string());
    if workflow_id != Some(policy.workflow_id)
        || event != "pull_request"
        || base_ref != default_branch
        || source_repo.is_empty()
        || source_repo != head_repo
        || source_repo != current_repo
    {
        return;
    }
    let run_id = env::var("TRIGGER_RUN_ID").unwrap_or_default();
    let target = if run_id.chars().all(|c| c.is_ascii_digit()) && !run_id.is_empty() {
        Some(format!("https://github.com/{repo}/actions/runs/{run_id}"))
    } else {
        None
    };
    if let Err(error) = post_status(repo, subject, token, "failure", &format!("Independent verifier incomplete: {reason}"), target.as_deref()) {
        eprintln!("Unable to publish fail-closed status: {error}");
    }
}

fn run_verifier() -> VerifyResult<i32> {
    let repo = env::var("REPOSITORY").unwrap_or_default().trim().to_owned();
    let token = env::var("GITHUB_TOKEN").unwrap_or_default().trim().to_owned();
    let mode = env::var("TRUST_ANCHOR_MODE").unwrap_or_default();
    let default_branch = env::var("DEFAULT_BRANCH").unwrap_or_else(|_| "main".to_string());
    let policy = policy(&repo).ok_or_else(|| format!("no base-owned policy for repository {repo:?}"))?;
    if token.is_empty() {
        return Err("GITHUB_TOKEN is required".to_string());
    }
    if mode != "workflow_run" && mode != "workflow_dispatch" {
        return Err(format!("unsupported verifier mode {mode:?}"));
    }
    let (run_id, expected_sha, expected_attempt, activity) = if mode == "workflow_dispatch" {
        let id = env::var("MANUAL_WORKFLOW_RUN_ID").unwrap_or_default();
        let id = id.parse::<u64>().ok().filter(|v| *v > 0)
            .ok_or_else(|| "manual replay requires a positive workflow_run_id".to_string())?;
        (id, None, None, "manual_replay".to_string())
    } else {
        let id = env::var("TRIGGER_RUN_ID").unwrap_or_default();
        let id = id.parse::<u64>().ok().filter(|v| *v > 0)
            .ok_or_else(|| "workflow_run event has no positive run ID".to_string())?;
        let subject = sha_or_error(&env::var("TRIGGER_RUN_HEAD_SHA").unwrap_or_default(), "event.head_sha")?;
        let attempt = env::var("TRIGGER_RUN_ATTEMPT").unwrap_or_default();
        let attempt = attempt.parse::<u64>().ok().filter(|v| *v > 0)
            .ok_or_else(|| "workflow_run event has no positive run attempt".to_string())?;
        (id, Some(subject), Some(attempt), env::var("TRIGGER_ACTIVITY_TYPE").unwrap_or_default())
    };
    let run_path = format!("/repos/{repo}/actions/runs/{run_id}");
    let run = api_json(Method::GET, &run_path, &token, None)?;
    if run.get("id").and_then(Value::as_u64) != Some(run_id) {
        return Err("run lookup returned missing or mismatched run ID".to_string());
    }
    let subject = sha_or_error(get_field_str(&run, "head_sha").unwrap_or(""), "run.head_sha")?;
    if expected_sha.as_deref().is_some_and(|expected| expected != subject) {
        return Err("run SHA differs from triggering event".to_string());
    }
    let branch = get_field_str(&run, "head_branch").unwrap_or("").to_owned();
    let pr = find_open_pr(&repo, &subject, &branch, &default_branch, &token)?;
    let Some(pr) = pr else {
        println!("No open same-repository PR targets the default branch; no merge status applies.");
        return Ok(0);
    };
    check_run_binding(&run, &repo, policy, &branch, &subject, expected_attempt)?;
    let workflow_runs_path = format!(
        "/repos/{repo}/actions/workflows/{}/runs?head_sha={subject}&event=pull_request&per_page=100&page=1",
        policy.workflow_id
    );
    let runs_payload = api_json(Method::GET, &workflow_runs_path, &token, None)?;
    let runs = runs_payload.get("workflow_runs").and_then(Value::as_array)
        .ok_or_else(|| "latest-run API response has an unexpected shape".to_string())?;
    if !is_latest_run(&run, runs, policy, &subject) {
        println!("Run/attempt superseded by a newer exact-head attempt; status left unchanged.");
        return Ok(0);
    }
    let target = get_field_str(&run, "html_url").unwrap_or("");
    let self_test_outcome = env::var("VERIFIER_TEST_OUTCOME").unwrap_or_else(|_| "success".to_string());
    if self_test_outcome != "success" {
        post_status(&repo, &subject, &token, "failure", &format!("Independent verifier self-tests did not pass: {self_test_outcome}"), Some(target))?;
        eprintln!("FAIL: independent verifier self-tests did not pass");
        return Ok(1);
    }
    if should_publish_pending_status(&mode, &activity, &run) {
        post_status(&repo, &subject, &token, "pending", "Exact-head security audit is running; no pass is implied.", Some(target))?;
        println!("PENDING: {repo}@{subject} run {run_id}.");
        return Ok(0);
    }
    let verification = (|| -> VerifyResult<()> {
        if get_field_str(&run, "status") != Some("completed") {
            return Err("workflow run is not completed".to_string());
        }
        if get_field_str(&run, "conclusion") != Some("success") {
            return Err(format!("producer workflow conclusion is {:?}, not success", get_field_str(&run, "conclusion")));
        }
        check_blob(&repo, policy.workflow_path, &subject, policy.workflow_blob, &token)?;
        if let Some(engine_sha) = policy.engine_sha {
            check_blob(ENGINE_REPO, ENGINE_PATH, ENGINE_SHA, ENGINE_BLOB, &token)?;
            if engine_sha != ENGINE_SHA {
                return Err("caller engine commit differs from trusted policy pin".to_string());
            }
        } else {
            check_blob(&repo, ENGINE_PATH, &subject, ENGINE_BLOB, &token)?;
        }
        let pr_number = pr.get("number").and_then(Value::as_u64)
            .filter(|n| *n > 0).ok_or_else(|| "matched open PR has an invalid number".to_string())?;
        verify_artifact(&repo, policy, &run, pr_number, &token)?;
        Ok(())
    })();
    match verification {
        Ok(()) => {
            post_status(&repo, &subject, &token, "success", "Exact run/attempt, PR, caller workflow blob, engine and verdict artifact verified.", Some(target))?;
            println!("PASS: independently verified {repo}@{subject}, run {run_id}, attempt {}.", run.get("run_attempt").and_then(Value::as_u64).unwrap_or(0));
            Ok(0)
        }
        Err(reason) => {
            post_status(&repo, &subject, &token, "failure", &format!("Independent security audit verification failed: {reason}"), Some(target))?;
            eprintln!("FAIL: {reason}");
            Ok(1)
        }
    }
}

fn main() {
    let repo = env::var("REPOSITORY").unwrap_or_default().trim().to_owned();
    let token = env::var("GITHUB_TOKEN").unwrap_or_default().trim().to_owned();
    match run_verifier() {
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("INCOMPLETE: {error}");
            if env::var("TRUST_ANCHOR_MODE").as_deref() == Ok("workflow_run") {
                let subject = env::var("TRIGGER_RUN_HEAD_SHA").unwrap_or_default();
                best_effort_failure_status(&repo, &subject, &token, &error);
            }
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_verdict(repo: &str, expected_pr: u64) -> (Value, Value, Policy) {
        let policy = policy(repo).unwrap();
        let subject = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let run_id = 123456u64;
        let attempt = 2u64;
        let run_url = format!("https://github.com/{repo}/actions/runs/{run_id}");
        let engine_sha = policy.engine_sha.unwrap_or("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
        let rustsec = if policy.audit_rust { "PASS" } else { "SKIPPED_NOT_REQUESTED" };
        let npm = if policy.audit_node { "PASS" } else { "SKIPPED_NOT_REQUESTED" };
        let verdict = json!({
            "schema": "luminous.security-audit.verdict.v1",
            "repository": repo,
            "subject_sha": subject,
            "audit_engine_sha": engine_sha,
            "workflow_ref": format!("{repo}/{}@refs/pull/{expected_pr}/merge", policy.workflow_path),
            "workflow_sha": "cccccccccccccccccccccccccccccccccccccccc",
            "workflow_run_url": run_url,
            "generated_at_utc": Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            "aggregate_artifact_retention_days": 30,
            "run_id": run_id.to_string(),
            "run_attempt": attempt.to_string(),
            "workflow_job_result": "success",
            "rustsec_job_result": if policy.audit_rust {"success"} else {"skipped"},
            "npm_job_result": if policy.audit_node {"success"} else {"skipped"},
            "audit_rust": policy.audit_rust,
            "audit_node": policy.audit_node,
            "required_lanes": {
                "workflow_security": "PASS",
                "rustsec": rustsec,
                "npm": npm
            },
            "non_blocking_findings_present": false,
            "non_blocking_finding_sources": [],
            "status": "PASS",
            "failure_reasons": [],
            "evidence_files": [{"path":"workflows/test.txt","sha256":digest_hex(b"evidence")}]
        });
        let run = json!({
            "id": run_id, "run_attempt": attempt, "head_sha": subject, "html_url": run_url
        });
        (verdict, run, policy)
    }

    #[test]
    fn accepts_only_canonical_lowercase_git_shas() {
        assert!(canonical_sha(&"a".repeat(40)));
        assert!(!canonical_sha(&"A".repeat(40)));
        assert!(!canonical_sha(&"g".repeat(40)));
        assert!(!canonical_sha(&"a".repeat(39)));
    }

    #[test]
    fn pins_workflow_reference_to_exact_pull_request() {
        assert!(validate_workflow_ref(
            "Luminous-Dynamics/mycelix/.github/workflows/security-audit.yml@refs/pull/4911/merge",
            "Luminous-Dynamics/mycelix",
            ".github/workflows/security-audit.yml",
            4911
        ).is_ok());
        assert!(validate_workflow_ref(
            "Luminous-Dynamics/mycelix/.github/workflows/security-audit.yml@refs/pull/4912/merge",
            "Luminous-Dynamics/mycelix",
            ".github/workflows/security-audit.yml",
            4911
        ).is_err());
    }

    #[test]
    fn pending_requires_an_unfinished_live_run() {
        assert!(should_publish_pending_status("workflow_run", "requested", &json!({"status":"queued"})));
        assert!(should_publish_pending_status("workflow_run", "in_progress", &json!({"status":"in_progress","conclusion":null})));
        assert!(!should_publish_pending_status("workflow_run", "in_progress", &json!({"status":"completed","conclusion":"success"})));
        assert!(!should_publish_pending_status("workflow_run", "in_progress", &json!({"status":"unexpected"})));
        assert!(!should_publish_pending_status("workflow_dispatch", "requested", &json!({"status":"queued"})));
    }

    #[test]
    fn rejects_duplicate_json_object_keys_at_any_depth() {
        assert!(parse_unique_json(br#"{"status":"PASS","nested":{"k":1,"k":2}}"#).is_err());
        assert!(parse_unique_json(br#"{"status":"PASS","nested":{"k":1,"other":2}}"#).is_ok());
    }

    #[test]
    fn rejects_unsafe_and_nested_verdict_paths() {
        assert!(!canonical_relative_path("../verdict.json"));
        assert!(!canonical_relative_path("a\\b"));
        assert!(evidence_archive_path("../x").is_err());
        assert!(evidence_archive_path("nested/verdict.json").is_err());
        assert!(evidence_archive_path("audit-evidence/workflows/test.txt").is_ok());
    }

    #[test]
    fn enforces_timestamp_format_and_calendar_validity() {
        assert!(valid_canonical_utc("2026-10-10T12:00:00Z").is_ok());
        assert!(valid_canonical_utc("2026-02-30T12:00:00Z").is_err());
        assert!(valid_canonical_utc("2026-10-10T12:00:00+00:00").is_err());
    }

    #[test]
    fn valid_verdict_matches_policy_and_exact_pr() {
        let (verdict, run, policy) = sample_verdict("Luminous-Dynamics/mycelix", 4911);
        assert!(validate_verdict(&verdict, "Luminous-Dynamics/mycelix", policy, &run, 4911).is_ok());
        assert!(validate_verdict(&verdict, "Luminous-Dynamics/mycelix", policy, &run, 4912).is_err());
    }

    #[test]
    fn verdict_must_match_engine_and_required_lanes() {
        let (mut verdict, run, policy) = sample_verdict("Luminous-Dynamics/mycelix", 4911);
        verdict["audit_engine_sha"] = Value::String("dddddddddddddddddddddddddddddddddddddddd".to_string());
        assert!(validate_verdict(&verdict, "Luminous-Dynamics/mycelix", policy, &run, 4911).is_err());
        let (mut verdict, run, policy) = sample_verdict("Luminous-Dynamics/mycelix", 4911);
        verdict["required_lanes"]["npm"] = Value::String("SKIPPED_NOT_REQUESTED".to_string());
        assert!(validate_verdict(&verdict, "Luminous-Dynamics/mycelix", policy, &run, 4911).is_err());
    }

    #[test]
    fn verdict_rejects_unknown_fields_and_failure_reasons() {
        let (mut verdict, run, policy) = sample_verdict("Luminous-Dynamics/mycelix", 4911);
        verdict["extra"] = json!(true);
        assert!(validate_verdict(&verdict, "Luminous-Dynamics/mycelix", policy, &run, 4911).is_err());
        let (mut verdict, run, policy) = sample_verdict("Luminous-Dynamics/mycelix", 4911);
        verdict["failure_reasons"] = json!(["unexpected issue"]);
        assert!(validate_verdict(&verdict, "Luminous-Dynamics/mycelix", policy, &run, 4911).is_err());
    }

    #[test]
    fn clean_pass_cannot_hide_non_blocking_findings() {
        let (mut verdict, run, policy) = sample_verdict("Luminous-Dynamics/mycelix", 4911);
        verdict["non_blocking_findings_present"] = json!(true);
        verdict["non_blocking_finding_sources"] = json!(["npm_below_threshold"]);
        assert!(validate_verdict(&verdict, "Luminous-Dynamics/mycelix", policy, &run, 4911).is_err());
    }

    #[test]
    fn pass_with_findings_must_name_a_permitted_lane() {
        let (mut verdict, run, policy) = sample_verdict("Luminous-Dynamics/mycelix", 4911);
        verdict["status"] = json!("PASS_WITH_FINDINGS");
        verdict["non_blocking_findings_present"] = json!(true);
        verdict["non_blocking_finding_sources"] = json!(["npm_below_threshold"]);
        assert!(validate_verdict(&verdict, "Luminous-Dynamics/mycelix", policy, &run, 4911).is_ok());
        verdict["non_blocking_finding_sources"] = json!(["unreviewed_source"]);
        assert!(validate_verdict(&verdict, "Luminous-Dynamics/mycelix", policy, &run, 4911).is_err());
    }
}
