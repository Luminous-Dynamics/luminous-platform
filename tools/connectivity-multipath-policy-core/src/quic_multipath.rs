//! Evidence verifier for an exact Multipath QUIC endpoint implementation.
//!
//! This module does not implement QUIC, open sockets, generate paths, or
//! schedule packets. It validates adapter-produced evidence from a pinned
//! implementation and classifies one paired trial. It deliberately keeps
//! data-path and ACK-return-path identities separate.

use std::collections::BTreeSet;

pub const MULTIPATH_QUIC_DRAFT: &str = "draft-ietf-quic-multipath-21";
pub const MAX_MULTIPATH_PATHS: u8 = 64;
const MIN_PATH_VALIDATION_PROBE_BYTES: u16 = 1200;
const MAX_PATH_HISTORY_RECORDS: usize = 4096;
const MAX_ACK_PATH_SAMPLES: usize = 1_000_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrialDisposition {
    Pass,
    Fail,
    Inconclusive,
    Unsupported,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApplicationDisposition {
    DeadlineMet,
    DeadlineMissed,
    Incomplete,
    CompletedWithoutDeadline,
    NotMeasured,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CongestionControllerKind {
    /// Default documented by mp-quic-go v0.3.0: independent Reno per path.
    UncoupledRenoPerPath,
    /// Optional coupled OLIA implementation.
    CoupledOlia,
    Other,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SchedulerKind {
    RoundRobin,
    LowLatency,
    MinRtt,
    Custom,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImplementationPin {
    pub repository: String,
    pub source_commit_sha: String,
    pub module_version: String,
    pub base_transport_version: String,
    pub toolchain_version: String,
    pub protocol_revision: String,
    pub build_artifact_sha256: String,
    pub congestion_controller: CongestionControllerKind,
    pub congestion_controller_id: String,
    pub congestion_controller_artifact_sha256: String,
    pub scheduler: SchedulerKind,
    pub scheduler_id: String,
    pub scheduler_artifact_sha256: String,
}

impl ImplementationPin {
    fn validate(&self) -> Result<(), PinError> {
        if self.repository.trim().is_empty()
            || self.module_version.trim().is_empty()
            || self.base_transport_version.trim().is_empty()
            || self.toolchain_version.trim().is_empty()
            || self.congestion_controller_id.trim().is_empty()
            || self.scheduler_id.trim().is_empty()
        {
            return Err(PinError::MissingIdentity);
        }
        if !is_hex_digest(&self.source_commit_sha, 40) {
            return Err(PinError::InvalidSourceCommit);
        }
        for (field, digest) in [
            ("build_artifact_sha256", &self.build_artifact_sha256),
            (
                "congestion_controller_artifact_sha256",
                &self.congestion_controller_artifact_sha256,
            ),
            ("scheduler_artifact_sha256", &self.scheduler_artifact_sha256),
        ] {
            if !is_hex_digest(digest, 64) {
                return Err(PinError::InvalidArtifactDigest(field));
            }
        }
        if self.protocol_revision != MULTIPATH_QUIC_DRAFT {
            return Err(PinError::UnsupportedProtocolRevision);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PinError {
    MissingIdentity,
    InvalidSourceCommit,
    InvalidArtifactDigest(&'static str),
    UnsupportedProtocolRevision,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExperimentIdentity {
    pub topology_sha256: String,
    pub resource_snapshot_sha256: String,
    pub qdisc_config_sha256: String,
    pub impairment_sha256: String,
    pub workload_sha256: String,
    pub seed: u64,
    pub started_at_unix_ms: u64,
    pub warmup_duration_us: u64,
    pub measurement_duration_us: u64,
}

impl ExperimentIdentity {
    fn validate(&self) -> bool {
        [
            &self.topology_sha256,
            &self.resource_snapshot_sha256,
            &self.qdisc_config_sha256,
            &self.impairment_sha256,
            &self.workload_sha256,
        ]
        .iter()
        .all(|digest| is_hex_digest(digest, 64))
            && self.started_at_unix_ms > 0
            && self.measurement_duration_us > 0
            && self.warmup_duration_us < self.measurement_duration_us
    }

    fn ends_at_unix_ms(&self) -> Option<u64> {
        let duration_ms = self.measurement_duration_us.div_ceil(1_000);
        self.started_at_unix_ms.checked_add(duration_ms)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PathState {
    Validating,
    Active,
    Backup,
    Abandoned,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LifecycleEventKind {
    Created,
    Validated,
    Abandoned,
    MaximumPathIdRaised,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PathLifecycleEvent {
    pub sequence: u64,
    pub occurred_at_unix_ms: u64,
    pub path_id: u32,
    /// Present only for MaximumPathIdRaised; the newly observed MAX_PATH_ID.
    pub maximum_path_id: Option<u32>,
    pub kind: LifecycleEventKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PathObservation {
    pub path_id: u32,
    pub state: PathState,
    pub validated: bool,
    /// True only if this path was given a non-zero-length QUIC connection ID.
    pub non_zero_connection_id: bool,
    /// True only if its packet-number space is separately keyed by this path ID.
    pub path_specific_packet_number_space: bool,
    /// For newly opened paths, the observed PATH_CHALLENGE datagram size.
    pub path_validation_probe_bytes: Option<u16>,
    /// For newly opened paths, proves PATH_RESPONSE returned on the challenge path.
    pub path_response_on_challenge_path: Option<bool>,
    /// Path-specific PMTU observation, not assumed equal across paths.
    pub pmtu_bytes: Option<u16>,
    /// None means ECN validation was not observed; Some(false) is a measured negative.
    pub ecn_validation_succeeded: Option<bool>,
    pub packets_sent: u64,
    pub packets_received: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EndpointEvidence {
    pub endpoint_id: String,
    pub implementation: ImplementationPin,
    pub multipath_supported: bool,
    pub multipath_negotiated: bool,
    pub initial_max_path_id: Option<u32>,
    /// Configured concurrent path ceiling, including the handshake path.
    pub configured_max_paths: u8,
    /// Highest peer-advertised MAX_PATH_ID observed so far. This may exceed
    /// initial_max_path_id after path churn, but must never decrease.
    pub maximum_path_id_limit_observed: u32,
    /// Complete path history is retained by the adapter even after Conn.Paths()
    /// removes long-abandoned path state.
    pub paths: Vec<PathObservation>,
    pub lifecycle_events: Vec<PathLifecycleEvent>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RttInterpretation {
    /// The packet and its acknowledgment used the same path.
    SamePath,
    /// The data packet and its acknowledgment used different paths. This is a
    /// composite round trip, not a direct RTT measurement of either path.
    CompositeDataPlusAckPath,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AckPathSample {
    pub sample_id: u64,
    /// Which endpoint emitted this observation (client or server endpoint ID).
    pub endpoint_id: String,
    pub data_path_id: u32,
    pub ack_return_path_id: u32,
    pub acknowledged_packet_count: u32,
    pub rtt_sample_count: u32,
    pub measured_round_trip_us: u64,
    pub observed_at_unix_ms: u64,
    pub rtt_interpretation: RttInterpretation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StreamOutcome {
    pub stream_id: String,
    pub source_bytes: u64,
    pub network_received_unique_bytes: u64,
    pub in_order_application_released_bytes: u64,
    pub receiver_reorder_peak_bytes: u64,
    pub receiver_reorder_limit_bytes: u64,
    pub unreleased_bytes_at_reassembly_timeout: u64,
    /// Duration from measurement start to complete in-order application release.
    pub completion_after_start_us: Option<u64>,
    pub deadline_after_start_us: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PairedTrialEvidence {
    pub identity: ExperimentIdentity,
    pub client: EndpointEvidence,
    pub server: EndpointEvidence,
    pub ack_path_samples: Vec<AckPathSample>,
    pub stream_outcome: Option<StreamOutcome>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QuicEvidenceReport {
    /// A verdict over one matched client/server trial only.
    pub scope: &'static str,
    pub disposition: TrialDisposition,
    pub protocol_disposition: TrialDisposition,
    pub application_disposition: ApplicationDisposition,
    pub reason: &'static str,
    pub validated_path_ids: Vec<u32>,
    /// Client data paths observed in ACK/packet telemetry.
    pub client_observed_data_path_ids: Vec<u32>,
    /// Client ACK-return paths observed, separately from data paths.
    pub client_observed_ack_return_path_ids: Vec<u32>,
    /// Server data paths observed in ACK/packet telemetry.
    pub server_observed_data_path_ids: Vec<u32>,
    /// Server ACK-return paths observed, separately from data paths.
    pub server_observed_ack_return_path_ids: Vec<u32>,
    pub ack_path_sample_count: u32,
    pub cross_path_ack_sample_count: u32,
    pub source_bytes: Option<u64>,
    pub network_received_unique_bytes: Option<u64>,
    pub in_order_application_released_bytes: Option<u64>,
    pub completion_after_start_us: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct EndpointSummary {
    active_path_ids: BTreeSet<u32>,
}

fn is_hex_digest(value: &str, expected_len: usize) -> bool {
    value.len() == expected_len && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn result(
    protocol_disposition: TrialDisposition,
    application_disposition: ApplicationDisposition,
    reason: &'static str,
    path_ids: Vec<u32>,
    ack_samples: usize,
    cross_path_ack_samples: usize,
    outcome: Option<&StreamOutcome>,
) -> QuicEvidenceReport {
    let disposition = match protocol_disposition {
        TrialDisposition::Fail
        | TrialDisposition::Unsupported
        | TrialDisposition::Inconclusive => protocol_disposition,
        TrialDisposition::Pass => match application_disposition {
            ApplicationDisposition::DeadlineMet
            | ApplicationDisposition::CompletedWithoutDeadline => TrialDisposition::Pass,
            ApplicationDisposition::DeadlineMissed => TrialDisposition::Fail,
            ApplicationDisposition::NotMeasured => TrialDisposition::Inconclusive,
        },
    };
    QuicEvidenceReport {
        scope: "single_paired_trial",
        disposition,
        protocol_disposition,
        application_disposition,
        reason,
        validated_path_ids: path_ids,
        client_observed_data_path_ids: Vec::new(),
        client_observed_ack_return_path_ids: Vec::new(),
        server_observed_data_path_ids: Vec::new(),
        server_observed_ack_return_path_ids: Vec::new(),
        ack_path_sample_count: u32::try_from(ack_samples).unwrap_or(u32::MAX),
        cross_path_ack_sample_count: u32::try_from(cross_path_ack_samples).unwrap_or(u32::MAX),
        source_bytes: outcome.map(|item| item.source_bytes),
        network_received_unique_bytes: outcome.map(|item| item.network_received_unique_bytes),
        in_order_application_released_bytes: outcome
            .map(|item| item.in_order_application_released_bytes),
        completion_after_start_us: outcome.and_then(|item| item.completion_after_start_us),
    }
}

fn validate_lifecycle(
    endpoint: &EndpointEvidence,
    start_ms: u64,
    end_ms: u64,
) -> Result<(), &'static str> {
    if endpoint.lifecycle_events.is_empty() {
        return Err("path_lifecycle_events_missing");
    }
    if endpoint.lifecycle_events.len() > MAX_PATH_HISTORY_RECORDS {
        return Err("path_lifecycle_history_exceeds_bound");
    }
    let mut created = BTreeSet::new();
    let mut validated = BTreeSet::new();
    let mut abandoned = BTreeSet::new();
    let mut previous_sequence = None;
    let mut previous_time = None;
    let mut previous_created_path_id = None;
    let mut observed_max_path_id = endpoint
        .initial_max_path_id
        .ok_or("initial_max_path_id_missing")?;

    for event in &endpoint.lifecycle_events {
        if event.sequence == 0 || previous_sequence.is_some_and(|value| event.sequence <= value) {
            return Err("path_lifecycle_sequence_not_strictly_increasing");
        }
        if event.occurred_at_unix_ms < start_ms || event.occurred_at_unix_ms > end_ms {
            return Err("path_lifecycle_event_outside_measurement_window");
        }
        if previous_time.is_some_and(|value| event.occurred_at_unix_ms < value) {
            return Err("path_lifecycle_time_moves_backwards");
        }
        previous_sequence = Some(event.sequence);
        previous_time = Some(event.occurred_at_unix_ms);

        match event.kind {
            LifecycleEventKind::Created => {
                if event.maximum_path_id.is_some() {
                    return Err("unexpected_maximum_path_id_on_lifecycle_event");
                }
                if !created.insert(event.path_id) {
                    return Err("path_id_reused");
                }
                if previous_created_path_id.is_some_and(|value| event.path_id <= value) {
                    return Err("path_ids_not_created_monotonically");
                }
                if created.len() == 1 && event.path_id != 0 {
                    return Err("initial_path_id_is_not_zero");
                }
                previous_created_path_id = Some(event.path_id);
            }
            LifecycleEventKind::Validated => {
                if event.maximum_path_id.is_some() {
                    return Err("unexpected_maximum_path_id_on_lifecycle_event");
                }
                if !created.contains(&event.path_id)
                    || abandoned.contains(&event.path_id)
                    || !validated.insert(event.path_id)
                {
                    return Err("path_validation_event_out_of_lifecycle_order");
                }
            }
            LifecycleEventKind::Abandoned => {
                if event.maximum_path_id.is_some() {
                    return Err("unexpected_maximum_path_id_on_lifecycle_event");
                }
                if !created.contains(&event.path_id) || !abandoned.insert(event.path_id) {
                    return Err("path_abandon_event_out_of_lifecycle_order");
                }
            }
            LifecycleEventKind::MaximumPathIdRaised => {
                let new_limit = event
                    .maximum_path_id
                    .ok_or("maximum_path_id_raise_value_missing")?;
                if new_limit <= observed_max_path_id {
                    return Err("maximum_path_id_limit_did_not_increase");
                }
                observed_max_path_id = new_limit;
            }
        }
    }
    if !created.contains(&0) {
        return Err("initial_path_creation_event_missing");
    }
    if observed_max_path_id != endpoint.maximum_path_id_limit_observed {
        return Err("maximum_path_id_limit_history_disagrees_with_observed_high_water");
    }

    let observed_ids = endpoint
        .paths
        .iter()
        .map(|path| path.path_id)
        .collect::<BTreeSet<_>>();
    if observed_ids != created {
        return Err("path_lifecycle_and_path_inventory_disagree");
    }
    for path in &endpoint.paths {
        if path.state == PathState::Abandoned && !abandoned.contains(&path.path_id) {
            return Err("abandoned_path_missing_lifecycle_event");
        }
        if matches!(path.state, PathState::Active | PathState::Backup)
            && !validated.contains(&path.path_id)
        {
            return Err("active_path_missing_validation_event");
        }
    }
    Ok(())
}

fn validate_endpoint(
    endpoint: &EndpointEvidence,
    start_ms: u64,
    end_ms: u64,
) -> Result<EndpointSummary, EndpointFailure> {
    if endpoint.endpoint_id.trim().is_empty()
        || endpoint.endpoint_id.len() > 128
        || endpoint.endpoint_id.chars().any(char::is_control)
    {
        return Err(EndpointFailure::Inconclusive("endpoint_identity_missing_or_invalid"));
    }
    if !endpoint.multipath_supported {
        return Err(EndpointFailure::Unsupported("endpoint_does_not_support_multipath_quic"));
    }
    if endpoint.implementation.protocol_revision != MULTIPATH_QUIC_DRAFT {
        return Err(EndpointFailure::Unsupported("endpoint_does_not_implement_draft_21"));
    }
    endpoint
        .implementation
        .validate()
        .map_err(|error| match error {
            PinError::UnsupportedProtocolRevision => {
                EndpointFailure::Unsupported("endpoint_does_not_implement_draft_21")
            }
            PinError::MissingIdentity => EndpointFailure::Inconclusive("implementation_identity_missing"),
            PinError::InvalidSourceCommit => EndpointFailure::Inconclusive("implementation_source_commit_invalid"),
            PinError::InvalidArtifactDigest(field) => match field {
                "build_artifact_sha256" => EndpointFailure::Inconclusive("implementation_build_digest_invalid"),
                "congestion_controller_artifact_sha256" => EndpointFailure::Inconclusive("congestion_controller_digest_invalid"),
                _ => EndpointFailure::Inconclusive("scheduler_artifact_digest_invalid"),
            },
        })?;

    if !endpoint.multipath_negotiated {
        return Err(EndpointFailure::Fail("multipath_extension_not_negotiated"));
    }
    let initial_max_path_id = endpoint
        .initial_max_path_id
        .ok_or(EndpointFailure::Fail("initial_max_path_id_missing"))?;
    if endpoint.configured_max_paths == 0
        || endpoint.configured_max_paths > MAX_MULTIPATH_PATHS
        || endpoint.maximum_path_id_limit_observed < initial_max_path_id
    {
        return Err(EndpointFailure::Fail("configured_path_limit_out_of_range"));
    }
    if endpoint.paths.is_empty() || endpoint.paths.len() > MAX_PATH_HISTORY_RECORDS {
        return Err(EndpointFailure::Fail("path_history_exceeds_bound"));
    }

    let mut seen = BTreeSet::new();
    let mut active_path_ids = BTreeSet::new();
    for path in &endpoint.paths {
        if !seen.insert(path.path_id) {
            return Err(EndpointFailure::Fail("duplicate_path_observation"));
        }
        if path.path_id > endpoint.maximum_path_id_limit_observed {
            return Err(EndpointFailure::Fail("path_id_exceeds_observed_advertised_maximum"));
        }
        if path.path_id != 0
            && matches!(path.state, PathState::Active | PathState::Backup)
            && (path.path_validation_probe_bytes.is_none_or(|size| {
                size < MIN_PATH_VALIDATION_PROBE_BYTES
            }) || path.path_response_on_challenge_path != Some(true))
        {
            return Err(EndpointFailure::Fail("secondary_path_validation_not_proven"));
        }
        if matches!(path.state, PathState::Active | PathState::Backup) {
            if !path.validated {
                return Err(EndpointFailure::Fail("active_path_not_validated"));
            }
            if !path.non_zero_connection_id {
                return Err(EndpointFailure::Fail("active_path_uses_zero_length_connection_id"));
            }
            if !path.path_specific_packet_number_space {
                return Err(EndpointFailure::Fail("path_packet_number_space_not_separated"));
            }
            if path.pmtu_bytes.is_none() {
                return Err(EndpointFailure::Inconclusive("active_path_pmtu_missing"));
            }
            if path.ecn_validation_succeeded.is_none() {
                return Err(EndpointFailure::Inconclusive("active_path_ecn_validation_unobserved"));
            }
            active_path_ids.insert(path.path_id);
        }
    }
    if !seen.contains(&0) || !active_path_ids.contains(&0) {
        return Err(EndpointFailure::Fail("initial_handshake_path_not_active"));
    }
    if active_path_ids.len() > usize::from(endpoint.configured_max_paths) {
        return Err(EndpointFailure::Fail("concurrent_path_count_exceeds_configured_limit"));
    }
    if active_path_ids.len() > usize::from(MAX_MULTIPATH_PATHS) {
        return Err(EndpointFailure::Fail("concurrent_path_count_exceeds_protocol_bound"));
    }
    if active_path_ids.len() < 2 {
        return Err(EndpointFailure::Fail("fewer_than_two_active_validated_paths"));
    }
    validate_lifecycle(endpoint, start_ms, end_ms)
        .map_err(EndpointFailure::Fail)?;
    Ok(EndpointSummary { active_path_ids })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EndpointFailure {
    Fail(&'static str),
    Inconclusive(&'static str),
    Unsupported(&'static str),
}

/// Evaluate one paired Multipath QUIC trial. This is a protocol/evidence check,
/// not a performance benchmark or statistical programme-level verdict.
pub fn evaluate_multipath_quic_trial(evidence: &PairedTrialEvidence) -> QuicEvidenceReport {
    if !evidence.identity.validate() {
        return result(
            TrialDisposition::Inconclusive,
            ApplicationDisposition::NotMeasured,
            "experiment_identity_invalid",
            Vec::new(),
            0,
            0,
            evidence.stream_outcome.as_ref(),
        );
    }
    let Some(end_ms) = evidence.identity.ends_at_unix_ms() else {
        return result(
            TrialDisposition::Inconclusive,
            ApplicationDisposition::NotMeasured,
            "measurement_window_overflow",
            Vec::new(),
            0,
            0,
            evidence.stream_outcome.as_ref(),
        );
    };

    let client = match validate_endpoint(
        &evidence.client,
        evidence.identity.started_at_unix_ms,
        end_ms,
    ) {
        Ok(summary) => summary,
        Err(EndpointFailure::Fail(reason)) => {
            return result(
                TrialDisposition::Fail,
                ApplicationDisposition::NotMeasured,
                reason,
                Vec::new(),
                0,
                0,
                evidence.stream_outcome.as_ref(),
            );
        }
        Err(EndpointFailure::Inconclusive(reason)) => {
            return result(
                TrialDisposition::Inconclusive,
                ApplicationDisposition::NotMeasured,
                reason,
                Vec::new(),
                0,
                0,
                evidence.stream_outcome.as_ref(),
            );
        }
        Err(EndpointFailure::Unsupported(reason)) => {
            return result(
                TrialDisposition::Unsupported,
                ApplicationDisposition::NotMeasured,
                reason,
                Vec::new(),
                0,
                0,
                evidence.stream_outcome.as_ref(),
            );
        }
    };
    let server = match validate_endpoint(
        &evidence.server,
        evidence.identity.started_at_unix_ms,
        end_ms,
    ) {
        Ok(summary) => summary,
        Err(EndpointFailure::Fail(reason)) => {
            return result(
                TrialDisposition::Fail,
                ApplicationDisposition::NotMeasured,
                reason,
                Vec::new(),
                0,
                0,
                evidence.stream_outcome.as_ref(),
            );
        }
        Err(EndpointFailure::Inconclusive(reason)) => {
            return result(
                TrialDisposition::Inconclusive,
                ApplicationDisposition::NotMeasured,
                reason,
                Vec::new(),
                0,
                0,
                evidence.stream_outcome.as_ref(),
            );
        }
        Err(EndpointFailure::Unsupported(reason)) => {
            return result(
                TrialDisposition::Unsupported,
                ApplicationDisposition::NotMeasured,
                reason,
                Vec::new(),
                0,
                0,
                evidence.stream_outcome.as_ref(),
            );
        }
    };

    if evidence.client.implementation.protocol_revision
        != evidence.server.implementation.protocol_revision
    {
        return result(
            TrialDisposition::Unsupported,
            ApplicationDisposition::NotMeasured,
            "endpoint_protocol_revision_mismatch",
            Vec::new(),
            0,
            0,
            evidence.stream_outcome.as_ref(),
        );
    }
    if evidence.client.implementation.congestion_controller
        != evidence.server.implementation.congestion_controller
        || evidence.client.implementation.congestion_controller_id
            != evidence.server.implementation.congestion_controller_id
    {
        return result(
            TrialDisposition::Inconclusive,
            ApplicationDisposition::NotMeasured,
            "endpoint_congestion_controller_pair_mismatch",
            Vec::new(),
            0,
            0,
            evidence.stream_outcome.as_ref(),
        );
    }
    if evidence.client.implementation.scheduler != evidence.server.implementation.scheduler
        || evidence.client.implementation.scheduler_id
            != evidence.server.implementation.scheduler_id
    {
        return result(
            TrialDisposition::Inconclusive,
            ApplicationDisposition::NotMeasured,
            "endpoint_scheduler_pair_mismatch",
            Vec::new(),
            0,
            0,
            evidence.stream_outcome.as_ref(),
        );
    }
    if client.active_path_ids != server.active_path_ids {
        return result(
            TrialDisposition::Fail,
            ApplicationDisposition::NotMeasured,
            "client_and_server_active_path_sets_disagree",
            Vec::new(),
            0,
            0,
            evidence.stream_outcome.as_ref(),
        );
    }

    if evidence.ack_path_samples.is_empty() {
        return result(
            TrialDisposition::Inconclusive,
            ApplicationDisposition::NotMeasured,
            "ack_path_observations_missing",
            client.active_path_ids.iter().copied().collect(),
            0,
            0,
            evidence.stream_outcome.as_ref(),
        );
    }

    if evidence.ack_path_samples.len() > MAX_ACK_PATH_SAMPLES {
        return result(
            TrialDisposition::Inconclusive,
            ApplicationDisposition::NotMeasured,
            "ack_path_sample_count_exceeds_bound",
            client.active_path_ids.iter().copied().collect(),
            0,
            0,
            evidence.stream_outcome.as_ref(),
        );
    }
    if evidence.client.endpoint_id == evidence.server.endpoint_id {
        return result(
            TrialDisposition::Inconclusive,
            ApplicationDisposition::NotMeasured,
            "client_and_server_endpoint_ids_are_not_distinct",
            Vec::new(),
            0,
            0,
            evidence.stream_outcome.as_ref(),
        );
    }
    let mut seen_sample_ids = BTreeSet::new();
    let mut sampled_endpoint_ids = BTreeSet::new();
    let mut cross_path_ack_samples = 0_usize;
    for sample in &evidence.ack_path_samples {
        if sample.sample_id == 0 || !seen_sample_ids.insert(sample.sample_id) {
            return result(
                TrialDisposition::Fail,
                ApplicationDisposition::NotMeasured,
                "ack_path_sample_id_invalid_or_duplicate",
                client.active_path_ids.iter().copied().collect(),
                evidence.ack_path_samples.len(),
                cross_path_ack_samples,
                evidence.stream_outcome.as_ref(),
            );
        }
        if sample.observed_at_unix_ms < evidence.identity.started_at_unix_ms
            || sample.observed_at_unix_ms > end_ms
        {
            return result(
                TrialDisposition::Fail,
                ApplicationDisposition::NotMeasured,
                "ack_path_sample_outside_measurement_window",
                client.active_path_ids.iter().copied().collect(),
                evidence.ack_path_samples.len(),
                cross_path_ack_samples,
                evidence.stream_outcome.as_ref(),
            );
        }
        let endpoint_paths = if sample.endpoint_id == evidence.client.endpoint_id {
            sampled_endpoint_ids.insert("client");
            &client.active_path_ids
        } else if sample.endpoint_id == evidence.server.endpoint_id {
            sampled_endpoint_ids.insert("server");
            &server.active_path_ids
        } else {
            return result(
                TrialDisposition::Fail,
                ApplicationDisposition::NotMeasured,
                "ack_path_sample_endpoint_identity_unknown",
                client.active_path_ids.iter().copied().collect(),
                evidence.ack_path_samples.len(),
                cross_path_ack_samples,
                evidence.stream_outcome.as_ref(),
            );
        };
        if !endpoint_paths.contains(&sample.data_path_id)
            || !endpoint_paths.contains(&sample.ack_return_path_id)
            || sample.acknowledged_packet_count == 0
            || sample.rtt_sample_count == 0
            || sample.measured_round_trip_us == 0
        {
            return result(
                TrialDisposition::Fail,
                ApplicationDisposition::NotMeasured,
                "ack_path_sample_references_unknown_path_or_missing_counters",
                client.active_path_ids.iter().copied().collect(),
                evidence.ack_path_samples.len(),
                cross_path_ack_samples,
                evidence.stream_outcome.as_ref(),
            );
        }
        if sample.data_path_id == sample.ack_return_path_id {
            if sample.rtt_interpretation != RttInterpretation::SamePath {
                return result(
                    TrialDisposition::Fail,
                    ApplicationDisposition::NotMeasured,
                    "same_path_ack_sample_has_incorrect_rtt_interpretation",
                    client.active_path_ids.iter().copied().collect(),
                    evidence.ack_path_samples.len(),
                    cross_path_ack_samples,
                    evidence.stream_outcome.as_ref(),
                );
            }
        } else {
            cross_path_ack_samples += 1;
            if sample.rtt_interpretation != RttInterpretation::CompositeDataPlusAckPath {
                return result(
                    TrialDisposition::Fail,
                    ApplicationDisposition::NotMeasured,
                    "cross_path_ack_sample_misrepresented_as_single_path_rtt",
                    client.active_path_ids.iter().copied().collect(),
                    evidence.ack_path_samples.len(),
                    cross_path_ack_samples,
                    evidence.stream_outcome.as_ref(),
                );
            }
        }
    }
    if sampled_endpoint_ids.len() != 2 {
        return result(
            TrialDisposition::Inconclusive,
            ApplicationDisposition::NotMeasured,
            "ack_path_samples_missing_for_one_endpoint",
            client.active_path_ids.iter().copied().collect(),
            evidence.ack_path_samples.len(),
            cross_path_ack_samples,
            evidence.stream_outcome.as_ref(),
        );
    }

    let Some(outcome) = evidence.stream_outcome.as_ref() else {
        return result(
            TrialDisposition::Inconclusive,
            ApplicationDisposition::NotMeasured,
            "application_stream_outcome_missing",
            client.active_path_ids.iter().copied().collect(),
            evidence.ack_path_samples.len(),
            cross_path_ack_samples,
            None,
        );
    };
    if outcome.stream_id.trim().is_empty() || outcome.source_bytes == 0 {
        return result(
            TrialDisposition::Fail,
            ApplicationDisposition::NotMeasured,
            "application_stream_identity_or_source_bytes_invalid",
            client.active_path_ids.iter().copied().collect(),
            evidence.ack_path_samples.len(),
            cross_path_ack_samples,
            Some(outcome),
        );
    }
    if outcome.network_received_unique_bytes > outcome.source_bytes
        || outcome.in_order_application_released_bytes > outcome.network_received_unique_bytes
        || outcome.unreleased_bytes_at_reassembly_timeout
            > outcome.network_received_unique_bytes - outcome.in_order_application_released_bytes
    {
        return result(
            TrialDisposition::Fail,
            ApplicationDisposition::NotMeasured,
            "application_byte_accounting_invariant_violated",
            client.active_path_ids.iter().copied().collect(),
            evidence.ack_path_samples.len(),
            cross_path_ack_samples,
            Some(outcome),
        );
    }
    if outcome.receiver_reorder_peak_bytes > outcome.receiver_reorder_limit_bytes {
        return result(
            TrialDisposition::Fail,
            ApplicationDisposition::NotMeasured,
            "receiver_reorder_buffer_limit_exceeded",
            client.active_path_ids.iter().copied().collect(),
            evidence.ack_path_samples.len(),
            cross_path_ack_samples,
            Some(outcome),
        );
    }

    if outcome
        .completion_after_start_us
        .is_some_and(|completion| completion > evidence.identity.measurement_duration_us)
    {
        return result(
            TrialDisposition::Inconclusive,
            ApplicationDisposition::NotMeasured,
            "application_completion_outside_measurement_window",
            client.active_path_ids.iter().copied().collect(),
            evidence.ack_path_samples.len(),
            cross_path_ack_samples,
            Some(outcome),
        );
    }
    if outcome.deadline_after_start_us.is_some_and(|deadline| {
        deadline > evidence.identity.measurement_duration_us
    }) {
        return result(
            TrialDisposition::Inconclusive,
            ApplicationDisposition::NotMeasured,
            "application_deadline_outside_measurement_window",
            client.active_path_ids.iter().copied().collect(),
            evidence.ack_path_samples.len(),
            cross_path_ack_samples,
            Some(outcome),
        );
    }

    let application_disposition = if outcome.in_order_application_released_bytes
        != outcome.source_bytes
        || outcome.completion_after_start_us.is_none()
    {
        ApplicationDisposition::Incomplete
    } else if let Some(deadline) = outcome.deadline_after_start_us {
        if outcome.completion_after_start_us.is_some_and(|value| value <= deadline) {
            ApplicationDisposition::DeadlineMet
        } else {
            ApplicationDisposition::DeadlineMissed
        }
    } else {
        ApplicationDisposition::CompletedWithoutDeadline
    };

    let reason = match application_disposition {
        ApplicationDisposition::DeadlineMet => "protocol_and_application_deadline_qualified_for_this_pair",
        ApplicationDisposition::DeadlineMissed => "protocol_qualified_but_useful_application_delivery_missed_deadline",
        ApplicationDisposition::Incomplete => "protocol_qualified_but_application_stream_incomplete",
        ApplicationDisposition::CompletedWithoutDeadline => "protocol_qualified_and_application_completed_without_deadline",
        ApplicationDisposition::NotMeasured => "application_outcome_not_measured",
    };
    let mut report = result(
        TrialDisposition::Pass,
        application_disposition,
        reason,
        client.active_path_ids.iter().copied().collect(),
        evidence.ack_path_samples.len(),
        cross_path_ack_samples,
        Some(outcome),
    );
    report.client_observed_data_path_ids = evidence
        .ack_path_samples
        .iter()
        .filter(|sample| sample.endpoint_id == evidence.client.endpoint_id)
        .map(|sample| sample.data_path_id)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    report.client_observed_ack_return_path_ids = evidence
        .ack_path_samples
        .iter()
        .filter(|sample| sample.endpoint_id == evidence.client.endpoint_id)
        .map(|sample| sample.ack_return_path_id)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    report.server_observed_data_path_ids = evidence
        .ack_path_samples
        .iter()
        .filter(|sample| sample.endpoint_id == evidence.server.endpoint_id)
        .map(|sample| sample.data_path_id)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    report.server_observed_ack_return_path_ids = evidence
        .ack_path_samples
        .iter()
        .filter(|sample| sample.endpoint_id == evidence.server.endpoint_id)
        .map(|sample| sample.ack_return_path_id)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pin() -> ImplementationPin {
        ImplementationPin {
            repository: "qoke/mp-quic-go".to_owned(),
            source_commit_sha: "a".repeat(40),
            module_version: "v0.3.0".to_owned(),
            base_transport_version: "quic-go-v0.63.0".to_owned(),
            toolchain_version: "go1.26.0".to_owned(),
            protocol_revision: MULTIPATH_QUIC_DRAFT.to_owned(),
            build_artifact_sha256: "b".repeat(64),
            congestion_controller: CongestionControllerKind::CoupledOlia,
            congestion_controller_id: "mp-quic-go-olia".to_owned(),
            congestion_controller_artifact_sha256: "c".repeat(64),
            scheduler: SchedulerKind::MinRtt,
            scheduler_id: "mp-quic-go-min-rtt".to_owned(),
            scheduler_artifact_sha256: "d".repeat(64),
        }
    }

    fn endpoint(id: &str) -> EndpointEvidence {
        let pin = pin();
        EndpointEvidence {
            endpoint_id: id.to_owned(),
            implementation: pin,
            multipath_supported: true,
            multipath_negotiated: true,
            initial_max_path_id: Some(2),
            configured_max_paths: 3,
            maximum_path_id_limit_observed: 2,
            paths: vec![
                PathObservation {
                    path_id: 0,
                    state: PathState::Active,
                    validated: true,
                    non_zero_connection_id: true,
                    path_specific_packet_number_space: true,
                    path_validation_probe_bytes: None,
                    path_response_on_challenge_path: None,
                    pmtu_bytes: Some(1450),
                    ecn_validation_succeeded: Some(true),
                    packets_sent: 100,
                    packets_received: 100,
                    bytes_sent: 100_000,
                    bytes_received: 100_000,
                },
                PathObservation {
                    path_id: 1,
                    state: PathState::Active,
                    validated: true,
                    non_zero_connection_id: true,
                    path_specific_packet_number_space: true,
                    path_validation_probe_bytes: Some(1200),
                    path_response_on_challenge_path: Some(true),
                    pmtu_bytes: Some(1400),
                    ecn_validation_succeeded: Some(true),
                    packets_sent: 100,
                    packets_received: 100,
                    bytes_sent: 100_000,
                    bytes_received: 100_000,
                },
            ],
            lifecycle_events: vec![
                PathLifecycleEvent {
                    sequence: 1,
                    occurred_at_unix_ms: 1_000,
                    path_id: 0,
                    maximum_path_id: None,
                    kind: LifecycleEventKind::Created,
                },
                PathLifecycleEvent {
                    sequence: 2,
                    occurred_at_unix_ms: 1_010,
                    path_id: 0,
                    maximum_path_id: None,
                    kind: LifecycleEventKind::Validated,
                },
                PathLifecycleEvent {
                    sequence: 3,
                    occurred_at_unix_ms: 1_020,
                    path_id: 1,
                    maximum_path_id: None,
                    kind: LifecycleEventKind::Created,
                },
                PathLifecycleEvent {
                    sequence: 4,
                    occurred_at_unix_ms: 1_030,
                    path_id: 1,
                    maximum_path_id: None,
                    kind: LifecycleEventKind::Validated,
                },
            ],
        }
    }

    fn trial() -> PairedTrialEvidence {
        PairedTrialEvidence {
            identity: ExperimentIdentity {
                topology_sha256: "e".repeat(64),
                resource_snapshot_sha256: "f".repeat(64),
                qdisc_config_sha256: "1".repeat(64),
                impairment_sha256: "2".repeat(64),
                workload_sha256: "3".repeat(64),
                seed: 17,
                started_at_unix_ms: 1_000,
                warmup_duration_us: 1_000_000,
                measurement_duration_us: 30_000_000,
            },
            client: endpoint("client"),
            server: endpoint("server"),
            ack_path_samples: vec![
                AckPathSample {
                    sample_id: 1,
                    endpoint_id: "client".to_owned(),
                    data_path_id: 0,
                    ack_return_path_id: 1,
                    acknowledged_packet_count: 64,
                    rtt_sample_count: 64,
                    measured_round_trip_us: 80_000,
                    observed_at_unix_ms: 2_000,
                    rtt_interpretation: RttInterpretation::CompositeDataPlusAckPath,
                },
                AckPathSample {
                    sample_id: 2,
                    endpoint_id: "client".to_owned(),
                    data_path_id: 1,
                    ack_return_path_id: 1,
                    acknowledged_packet_count: 32,
                    rtt_sample_count: 32,
                    measured_round_trip_us: 40_000,
                    observed_at_unix_ms: 2_100,
                    rtt_interpretation: RttInterpretation::SamePath,
                },
                AckPathSample {
                    sample_id: 3,
                    endpoint_id: "server".to_owned(),
                    data_path_id: 1,
                    ack_return_path_id: 0,
                    acknowledged_packet_count: 21,
                    rtt_sample_count: 21,
                    measured_round_trip_us: 75_000,
                    observed_at_unix_ms: 2_200,
                    rtt_interpretation: RttInterpretation::CompositeDataPlusAckPath,
                },
            ],
            stream_outcome: Some(StreamOutcome {
                stream_id: "stream-1".to_owned(),
                source_bytes: 1_000_000,
                network_received_unique_bytes: 1_000_000,
                in_order_application_released_bytes: 1_000_000,
                receiver_reorder_peak_bytes: 64_000,
                receiver_reorder_limit_bytes: 128_000,
                unreleased_bytes_at_reassembly_timeout: 0,
                completion_after_start_us: Some(5_000_000),
                deadline_after_start_us: Some(8_000_000),
            }),
        }
    }

    #[test]
    fn qualifies_two_endpoint_draft_21_trial_with_cross_path_ack() {
        let report = evaluate_multipath_quic_trial(&trial());
        assert_eq!(report.disposition, TrialDisposition::Pass);
        assert_eq!(report.protocol_disposition, TrialDisposition::Pass);
        assert_eq!(report.application_disposition, ApplicationDisposition::DeadlineMet);
        assert_eq!(report.validated_path_ids, vec![0, 1]);
        assert_eq!(report.cross_path_ack_sample_count, 1);
    }

    #[test]
    fn single_path_fallback_does_not_qualify_as_multipath() {
        let mut evidence = trial();
        evidence.client.paths.pop();
        evidence.server.paths.pop();
        evidence.client.lifecycle_events.truncate(2);
        evidence.server.lifecycle_events.truncate(2);
        let report = evaluate_multipath_quic_trial(&evidence);
        assert_eq!(report.disposition, TrialDisposition::Fail);
        assert_eq!(report.reason, "fewer_than_two_active_validated_paths");
    }

    #[test]
    fn endpoint_without_multipath_support_is_unsupported() {
        let mut evidence = trial();
        evidence.server.multipath_supported = false;
        let report = evaluate_multipath_quic_trial(&evidence);
        assert_eq!(report.disposition, TrialDisposition::Unsupported);
    }

    #[test]
    fn negotiated_extension_mismatch_is_a_failure_not_a_single_path_pass() {
        let mut evidence = trial();
        evidence.server.multipath_negotiated = false;
        let report = evaluate_multipath_quic_trial(&evidence);
        assert_eq!(report.disposition, TrialDisposition::Fail);
        assert_eq!(report.reason, "multipath_extension_not_negotiated");
    }

    #[test]
    fn wrong_draft_revision_is_unsupported() {
        let mut evidence = trial();
        evidence.client.implementation.protocol_revision = "draft-ietf-quic-multipath-20".to_owned();
        let report = evaluate_multipath_quic_trial(&evidence);
        assert_eq!(report.disposition, TrialDisposition::Unsupported);
    }

    #[test]
    fn path_ids_must_agree_at_both_endpoints() {
        let mut evidence = trial();
        evidence.server.paths[1].path_id = 2;
        evidence.server.paths[1].path_validation_probe_bytes = Some(1200);
        evidence.server.paths[1].path_response_on_challenge_path = Some(true);
        evidence.server.lifecycle_events[2].path_id = 2;
        evidence.server.lifecycle_events[3].path_id = 2;
        let report = evaluate_multipath_quic_trial(&evidence);
        assert_eq!(report.disposition, TrialDisposition::Fail);
        assert_eq!(report.reason, "client_and_server_active_path_sets_disagree");
    }

    #[test]
    fn path_id_reuse_is_rejected() {
        let mut evidence = trial();
        evidence.client.lifecycle_events.push(PathLifecycleEvent {
            sequence: 5,
            occurred_at_unix_ms: 1_040,
            path_id: 1,
            maximum_path_id: None,
            kind: LifecycleEventKind::Created,
        });
        let report = evaluate_multipath_quic_trial(&evidence);
        assert_eq!(report.disposition, TrialDisposition::Fail);
        assert_eq!(report.reason, "path_id_reused");
    }

    #[test]
    fn secondary_path_probe_must_be_at_least_1200_bytes() {
        let mut evidence = trial();
        evidence.client.paths[1].path_validation_probe_bytes = Some(1199);
        let report = evaluate_multipath_quic_trial(&evidence);
        assert_eq!(report.disposition, TrialDisposition::Fail);
        assert_eq!(report.reason, "secondary_path_validation_not_proven");
    }

    #[test]
    fn secondary_path_response_must_return_on_challenge_path() {
        let mut evidence = trial();
        evidence.server.paths[1].path_response_on_challenge_path = Some(false);
        let report = evaluate_multipath_quic_trial(&evidence);
        assert_eq!(report.disposition, TrialDisposition::Fail);
    }

    #[test]
    fn zero_length_connection_id_is_rejected() {
        let mut evidence = trial();
        evidence.server.paths[1].non_zero_connection_id = false;
        let report = evaluate_multipath_quic_trial(&evidence);
        assert_eq!(report.disposition, TrialDisposition::Fail);
        assert_eq!(report.reason, "active_path_uses_zero_length_connection_id");
    }

    #[test]
    fn missing_per_path_packet_number_space_is_rejected() {
        let mut evidence = trial();
        evidence.client.paths[1].path_specific_packet_number_space = false;
        let report = evaluate_multipath_quic_trial(&evidence);
        assert_eq!(report.disposition, TrialDisposition::Fail);
    }

    #[test]
    fn missing_ack_path_instrumentation_is_inconclusive() {
        let mut evidence = trial();
        evidence.ack_path_samples.clear();
        let report = evaluate_multipath_quic_trial(&evidence);
        assert_eq!(report.disposition, TrialDisposition::Inconclusive);
        assert_eq!(report.reason, "ack_path_observations_missing");
    }

    #[test]
    fn cross_path_ack_rtt_must_be_labeled_composite() {
        let mut evidence = trial();
        evidence.ack_path_samples[0].rtt_interpretation = RttInterpretation::SamePath;
        let report = evaluate_multipath_quic_trial(&evidence);
        assert_eq!(report.disposition, TrialDisposition::Fail);
        assert_eq!(report.reason, "cross_path_ack_sample_misrepresented_as_single_path_rtt");
    }

    #[test]
    fn unknown_ack_path_is_rejected() {
        let mut evidence = trial();
        evidence.ack_path_samples[0].ack_return_path_id = 99;
        let report = evaluate_multipath_quic_trial(&evidence);
        assert_eq!(report.disposition, TrialDisposition::Fail);
    }

    #[test]
    fn path_event_must_be_inside_the_frozen_measurement_window() {
        let mut evidence = trial();
        evidence.client.lifecycle_events[3].occurred_at_unix_ms = 99_999;
        let report = evaluate_multipath_quic_trial(&evidence);
        assert_eq!(report.disposition, TrialDisposition::Fail);
        assert_eq!(report.reason, "path_lifecycle_event_outside_measurement_window");
    }

    #[test]
    fn incomplete_path_pmtu_or_ecn_observation_is_inconclusive() {
        let mut evidence = trial();
        evidence.client.paths[1].pmtu_bytes = None;
        let report = evaluate_multipath_quic_trial(&evidence);
        assert_eq!(report.disposition, TrialDisposition::Inconclusive);
        assert_eq!(report.reason, "active_path_pmtu_missing");
    }

    #[test]
    fn stream_byte_accounting_cannot_exceed_network_unique_bytes() {
        let mut evidence = trial();
        evidence.stream_outcome.as_mut().unwrap().in_order_application_released_bytes += 1;
        let report = evaluate_multipath_quic_trial(&evidence);
        assert_eq!(report.disposition, TrialDisposition::Fail);
        assert_eq!(report.reason, "application_byte_accounting_invariant_violated");
    }

    #[test]
    fn receiver_reorder_buffer_limit_is_a_hard_gate() {
        let mut evidence = trial();
        evidence.stream_outcome.as_mut().unwrap().receiver_reorder_peak_bytes = 200_000;
        let report = evaluate_multipath_quic_trial(&evidence);
        assert_eq!(report.disposition, TrialDisposition::Fail);
        assert_eq!(report.reason, "receiver_reorder_buffer_limit_exceeded");
    }

    #[test]
    fn report_distinguishes_validated_paths_from_paths_used_for_data() {
        let mut evidence = trial();
        evidence.ack_path_samples.retain(|sample| sample.endpoint_id == "server");
        evidence.ack_path_samples[0].data_path_id = 0;
        evidence.ack_path_samples[0].ack_return_path_id = 0;
        evidence.ack_path_samples[0].rtt_interpretation = RttInterpretation::SamePath;
        let report = evaluate_multipath_quic_trial(&evidence);
        assert_eq!(report.disposition, TrialDisposition::Pass);
        assert_eq!(report.validated_path_ids, vec![0, 1]);
        assert_eq!(report.client_observed_data_path_ids, Vec::<u32>::new());
        assert_eq!(report.client_observed_ack_return_path_ids, Vec::<u32>::new());
        assert_eq!(report.server_observed_data_path_ids, vec![0]);
        assert_eq!(report.server_observed_ack_return_path_ids, vec![0]);
    }

    #[test]
    fn application_deadline_miss_is_not_hidden_by_protocol_pass() {
        let mut evidence = trial();
        evidence.stream_outcome.as_mut().unwrap().completion_after_start_us = Some(9_000_000);
        let report = evaluate_multipath_quic_trial(&evidence);
        assert_eq!(report.protocol_disposition, TrialDisposition::Pass);
        assert_eq!(report.application_disposition, ApplicationDisposition::DeadlineMissed);
        assert_eq!(report.disposition, TrialDisposition::Fail);
    }

    #[test]
    fn incomplete_application_stream_is_not_mislabeled_as_deadline_miss() {
        let mut evidence = trial();
        evidence.stream_outcome.as_mut().unwrap().in_order_application_released_bytes = 900_000;
        evidence.stream_outcome.as_mut().unwrap().completion_after_start_us = None;
        let report = evaluate_multipath_quic_trial(&evidence);
        assert_eq!(report.protocol_disposition, TrialDisposition::Pass);
        assert_eq!(report.application_disposition, ApplicationDisposition::Incomplete);
        assert_eq!(report.disposition, TrialDisposition::Fail);
        assert_eq!(
            report.reason,
            "protocol_qualified_but_application_stream_incomplete"
        );
    }

    #[test]
    fn deadline_beyond_measurement_window_is_inconclusive() {
        let mut evidence = trial();
        evidence.stream_outcome.as_mut().unwrap().deadline_after_start_us =
            Some(evidence.identity.measurement_duration_us + 1);
        let report = evaluate_multipath_quic_trial(&evidence);
        assert_eq!(report.disposition, TrialDisposition::Inconclusive);
        assert_eq!(report.application_disposition, ApplicationDisposition::NotMeasured);
        assert_eq!(report.reason, "application_deadline_outside_measurement_window");
    }

    #[test]
    fn missing_stream_outcome_is_inconclusive() {
        let mut evidence = trial();
        evidence.stream_outcome = None;
        let report = evaluate_multipath_quic_trial(&evidence);
        assert_eq!(report.disposition, TrialDisposition::Inconclusive);
        assert_eq!(report.application_disposition, ApplicationDisposition::NotMeasured);
    }

    #[test]
    fn different_congestion_controller_modes_are_not_paired_as_one_algorithm() {
        let mut evidence = trial();
        evidence.server.implementation.congestion_controller =
            CongestionControllerKind::UncoupledRenoPerPath;
        evidence.server.implementation.congestion_controller_id =
            "mp-quic-go-uncoupled-reno".to_owned();
        evidence.server.implementation.congestion_controller_artifact_sha256 = "9".repeat(64);
        let report = evaluate_multipath_quic_trial(&evidence);
        assert_eq!(report.disposition, TrialDisposition::Inconclusive);
        assert_eq!(report.reason, "endpoint_congestion_controller_pair_mismatch");
    }

    #[test]
    fn ack_samples_must_cover_both_endpoints() {
        let mut evidence = trial();
        evidence.ack_path_samples.retain(|sample| sample.endpoint_id == "client");
        let report = evaluate_multipath_quic_trial(&evidence);
        assert_eq!(report.disposition, TrialDisposition::Inconclusive);
        assert_eq!(report.reason, "ack_path_samples_missing_for_one_endpoint");
    }

    #[test]
    fn ack_sample_endpoint_must_be_known() {
        let mut evidence = trial();
        evidence.ack_path_samples[0].endpoint_id = "unknown-endpoint".to_owned();
        let report = evaluate_multipath_quic_trial(&evidence);
        assert_eq!(report.disposition, TrialDisposition::Fail);
        assert_eq!(report.reason, "ack_path_sample_endpoint_identity_unknown");
    }

    #[test]
    fn endpoint_ids_must_be_distinct() {
        let mut evidence = trial();
        evidence.server.endpoint_id = evidence.client.endpoint_id.clone();
        let report = evaluate_multipath_quic_trial(&evidence);
        assert_eq!(report.disposition, TrialDisposition::Inconclusive);
        assert_eq!(report.reason, "client_and_server_endpoint_ids_are_not_distinct");
    }

    #[test]
    fn endpoint_without_ack_samples_is_not_treated_as_observed() {
        let mut evidence = trial();
        evidence.ack_path_samples
            .retain(|sample| sample.endpoint_id == evidence.client.endpoint_id);
        let report = evaluate_multipath_quic_trial(&evidence);
        assert_eq!(report.disposition, TrialDisposition::Inconclusive);
        assert_eq!(report.reason, "ack_path_samples_missing_for_one_endpoint");
    }

    #[test]
    fn duplicate_path_validation_event_is_rejected() {
        let mut evidence = trial();
        evidence.client.lifecycle_events.push(PathLifecycleEvent {
            sequence: 5,
            occurred_at_unix_ms: 1_040,
            path_id: 1,
            maximum_path_id: None,
            kind: LifecycleEventKind::Validated,
        });
        let report = evaluate_multipath_quic_trial(&evidence);
        assert_eq!(report.disposition, TrialDisposition::Fail);
        assert_eq!(report.reason, "path_validation_event_out_of_lifecycle_order");
    }

    #[test]
    fn maximum_path_id_high_water_can_increase_after_path_churn() {
        let mut evidence = trial();
        evidence.client.maximum_path_id_limit_observed = 4;
        evidence.server.maximum_path_id_limit_observed = 4;
        evidence.client.lifecycle_events.push(PathLifecycleEvent {
            sequence: 5,
            occurred_at_unix_ms: 1_040,
            path_id: 0,
            maximum_path_id: Some(4),
            kind: LifecycleEventKind::MaximumPathIdRaised,
        });
        evidence.server.lifecycle_events.push(PathLifecycleEvent {
            sequence: 5,
            occurred_at_unix_ms: 1_040,
            path_id: 0,
            maximum_path_id: Some(4),
            kind: LifecycleEventKind::MaximumPathIdRaised,
        });
        let report = evaluate_multipath_quic_trial(&evidence);
        assert_eq!(report.disposition, TrialDisposition::Pass);
    }

    #[test]
    fn maximum_path_id_limit_cannot_decrease_or_repeat() {
        let mut evidence = trial();
        evidence.client.maximum_path_id_limit_observed = 2;
        evidence.client.lifecycle_events.push(PathLifecycleEvent {
            sequence: 5,
            occurred_at_unix_ms: 1_040,
            path_id: 0,
            maximum_path_id: Some(2),
            kind: LifecycleEventKind::MaximumPathIdRaised,
        });
        let report = evaluate_multipath_quic_trial(&evidence);
        assert_eq!(report.disposition, TrialDisposition::Fail);
        assert_eq!(report.reason, "maximum_path_id_limit_did_not_increase");
    }

    #[test]
    fn same_frozen_trial_is_deterministic() {
        let evidence = trial();
        assert_eq!(
            evaluate_multipath_quic_trial(&evidence),
            evaluate_multipath_quic_trial(&evidence)
        );
    }
}
