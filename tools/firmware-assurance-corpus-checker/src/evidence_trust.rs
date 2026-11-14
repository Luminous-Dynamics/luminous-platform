//! Synthetic evidence-envelope trust policy for adversarial contract tests.
//!
//! This is not a cryptographic verifier. It consumes already-classified results
//! from separately qualified verifiers and tests whether the policy composes them
//! without promoting unknown, stale, replayed, or unbound evidence to acceptance.

use serde_json::Value;

pub const EVIDENCE_FIELDS: [&str; 10] = [
    "signature_status",
    "signer_status",
    "subject_status",
    "artifact_status",
    "policy_status",
    "revocation_status",
    "freshness_status",
    "nonce_status",
    "counter_status",
    "provenance_status",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Accepted,
    Rejected,
    Indeterminate,
}

impl Decision {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Rejected => "rejected",
            Self::Indeterminate => "indeterminate",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceTrustInput {
    pub signature_status: String,
    pub signer_status: String,
    pub subject_status: String,
    pub artifact_status: String,
    pub policy_status: String,
    pub revocation_status: String,
    pub freshness_status: String,
    pub nonce_status: String,
    pub counter_status: String,
    pub provenance_status: String,
}

fn get_status(value: &Value, field: &str) -> Result<String, String> {
    let status = value
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{field} must be a string"))?;
    let allowed = match field {
        "signature_status" => &["valid", "invalid", "unknown"],
        "signer_status" => &["authorized", "unauthorized", "unknown"],
        "subject_status" => &["match", "mismatch", "unknown"],
        "artifact_status" => &["match", "mismatch", "unknown"],
        "policy_status" => &["match", "mismatch", "unknown"],
        "revocation_status" => &["clear", "revoked", "unknown"],
        "freshness_status" => &["fresh", "expired", "unknown"],
        "nonce_status" => &["current", "replay", "unknown"],
        "counter_status" => &["monotonic", "rollback", "unknown"],
        "provenance_status" => &["authenticated", "unauthenticated", "unknown"],
        _ => return Err(format!("unsupported evidence field {field}")),
    };
    if !allowed.contains(&status) {
        return Err(format!("unsupported status {status:?} for {field}"));
    }
    Ok(status.to_string())
}

impl EvidenceTrustInput {
    /// Parse all trust inputs strictly; missing fields never default to success.
    pub fn from_json(value: &Value) -> Result<Self, String> {
        if !value.is_object() {
            return Err("evidence case must be a JSON object".to_string());
        }
        Ok(Self {
            signature_status: get_status(value, "signature_status")?,
            signer_status: get_status(value, "signer_status")?,
            subject_status: get_status(value, "subject_status")?,
            artifact_status: get_status(value, "artifact_status")?,
            policy_status: get_status(value, "policy_status")?,
            revocation_status: get_status(value, "revocation_status")?,
            freshness_status: get_status(value, "freshness_status")?,
            nonce_status: get_status(value, "nonce_status")?,
            counter_status: get_status(value, "counter_status")?,
            provenance_status: get_status(value, "provenance_status")?,
        })
    }

    /// Compose independent verifier outcomes with explicit fail-closed precedence.
    ///
    /// Any known violation rejects even if another field is unknown. In the
    /// absence of a known violation, any unknown field is indeterminate. Only
    /// the exact all-positive tuple is accepted.
    pub fn evaluate(&self) -> Decision {
        let statuses = [
            (&self.signature_status, "valid", "invalid"),
            (&self.signer_status, "authorized", "unauthorized"),
            (&self.subject_status, "match", "mismatch"),
            (&self.artifact_status, "match", "mismatch"),
            (&self.policy_status, "match", "mismatch"),
            (&self.revocation_status, "clear", "revoked"),
            (&self.freshness_status, "fresh", "expired"),
            (&self.nonce_status, "current", "replay"),
            (&self.counter_status, "monotonic", "rollback"),
            (&self.provenance_status, "authenticated", "unauthenticated"),
        ];
        if statuses.iter().any(|(actual, _, negative)| actual.as_str() == *negative) {
            return Decision::Rejected;
        }
        if statuses.iter().all(|(actual, positive, _)| actual.as_str() == *positive) {
            return Decision::Accepted;
        }
        Decision::Indeterminate
    }
}
