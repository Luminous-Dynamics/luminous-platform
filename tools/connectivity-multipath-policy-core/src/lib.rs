//! Deterministic, privilege-free multipath session scheduling model.
//!
//! The model consumes broker-admitted path observations. It does not discover
//! addresses, create subflows, modify routes, inspect packet contents, or send
//! packets. Its finish-time and reorder figures are planning estimates, not
//! transport measurements.
pub mod fairness;
pub mod trace;

use std::collections::BTreeSet;

const PARTS_PER_MILLION: u128 = 1_000_000;
const BYTES_PER_MIB: u128 = 1_048_576;
const MAX_CAPACITY_DOMAINS_PER_PATH: usize = 32;
const MAX_CAPACITY_DOMAIN_ID_BYTES: usize = 128;
const MAX_CAPACITY_EVIDENCE_SOURCE_ID_BYTES: usize = 128;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrafficIntent {
    Interactive,
    Messaging,
    BulkTransfer,
    CostSensitive,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Strategy {
    BestRtt,
    LowestCost,
    AggregateCapacity,
    IntentAware,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecisionMode {
    NoRoute,
    SinglePath,
    Multipath,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PathObservation {
    pub id: String,
    /// Must be supplied by the existing broker; false paths are never scheduled.
    pub broker_eligible: bool,
    pub authorization_valid: bool,
    pub telemetry_fresh: bool,
    /// Local policy trust rank, not a cryptographic score.
    pub trust_level: u8,
    pub rtt_us: u64,
    pub jitter_us: u64,
    pub loss_ppm: u32,
    pub capacity_bps: u64,
    pub mtu_bytes: u32,
    pub data_remaining_bytes: u64,
    /// ISO-4217-style uppercase currency code for the integer cost rate.
    pub cost_currency: String,
    /// Cost and energy rate per MiB, in caller-declared integer units.
    pub cost_micros_per_mib: u64,
    pub energy_uwh_per_mib: u64,
    /// Operator-declared common-cause failure group.
    pub shared_risk_group: String,
    /// Complete set of hard throughput bottlenecks consumed by this path.
    pub capacity_domain_ids: Vec<String>,
    /// Provenance and validity of the capacity dependency set.
    pub capacity_domain_evidence: CapacityDomainEvidence,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapacityDomainEvidence {
    /// Stable identifier of the inventory/controller supplying the dependency set.
    pub source_id: String,
    /// Digest of the exact source inventory artifact.
    pub inventory_sha256: String,
    /// Frozen end-to-end topology snapshot this dependency evidence belongs to.
    pub topology_sha256: String,
    /// False means the inventory cannot attest that every bottleneck is enumerated.
    pub complete: bool,
    /// Operator-declared evidence confidence in parts per million (0..=1,000,000).
    pub confidence_ppm: u32,
    /// Unix epoch milliseconds at which this observation was made.
    pub observed_at_unix_ms: u64,
    /// Unix epoch milliseconds after which this observation must not be used.
    pub valid_until_unix_ms: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScheduleRequest {
    pub task_id: String,
    /// SHA-256 of the frozen topology/run snapshot expected by this decision.
    pub topology_sha256: String,
    /// Caller-supplied decision time; all capacity dependency evidence is
    /// checked against this exact timestamp for deterministic replay.
    pub now_unix_ms: u64,
    /// Minimum operator-declared capacity-dependency evidence confidence in ppm.
    /// This is an evidence-grade field, not a statistical probability.
    pub minimum_capacity_domain_confidence_ppm: u32,
    pub intent: TrafficIntent,
    pub bytes: u64,
    pub deadline_us: Option<u64>,
    pub max_cost_micros: u64,
    /// Currency expected for every eligible path's cost rate.
    pub cost_currency: String,
    pub max_energy_uwh: u64,
    pub minimum_trust: u8,
    pub minimum_mtu_bytes: u32,
    pub allow_multipath: bool,
    pub maximum_paths: u8,
    pub ordered_delivery: bool,
    pub reorder_buffer_limit_bytes: u64,
    /// Multipath must improve modelled completion time by at least this ratio.
    /// 10_000 = 1%; 1_000_000 = 100%.
    pub minimum_gain_ppm: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PathAllocation {
    pub path_id: String,
    pub bytes: u64,
    pub effective_capacity_bps: u64,
    pub estimated_finish_us: u64,
    pub estimated_cost_micros: u64,
    pub estimated_energy_uwh: u64,
    pub shared_risk_group: String,
    pub capacity_domain_ids: Vec<String>,
    pub capacity_domain_evidence: CapacityDomainEvidence,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PathExclusion {
    pub path_id: String,
    pub reason: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScheduleDecision {
    pub task_id: String,
    pub strategy: Strategy,
    pub mode: DecisionMode,
    pub allocations: Vec<PathAllocation>,
    pub predicted_completion_us: Option<u64>,
    pub estimated_cost_micros: u64,
    pub estimated_energy_uwh: u64,
    /// Conservative proxy: bytes on allocations predicted to finish after the
    /// earliest allocation. This is not a packet-level reassembly simulation.
    pub reorder_exposure_bytes: u64,
    pub distinct_shared_risk_groups: usize,
    pub distinct_capacity_domains: usize,
    pub exclusions: Vec<PathExclusion>,
    pub explanation: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScheduleError {
    EmptyTaskId,
    ZeroPayload,
    ZeroMaximumPaths,
    InvalidLossBound,
    InvalidGainThreshold,
    InvalidCurrencyCode,
    InvalidTopologyDigest,
    InvalidCurrentTime,
    InvalidCapacityDomainConfidenceFloor,
    InvalidPathId,
    DuplicatePathId,
    TooManyPaths,
    TooManyTraceSegments,
    ZeroTraceSegmentSize,
    ZeroReassemblyTimeout,
    ArithmeticOverflow,
    InternalInvariant,
}

#[derive(Clone, Debug)]
struct Candidate {
    allocations: Vec<PathAllocation>,
    finish_us: u64,
    cost_micros: u64,
    energy_uwh: u64,
    reorder_exposure_bytes: u64,
    distinct_shared_risk_groups: usize,
    distinct_capacity_domains: usize,
}

fn ceil_div(numerator: u128, denominator: u128) -> u128 {
    numerator.div_ceil(denominator)
}

fn as_u64(value: u128) -> u64 {
    value.min(u64::MAX as u128) as u64
}

fn estimated_cost(bytes: u64, rate_per_mib: u64) -> Option<u64> {
    u64::try_from(ceil_div(
        u128::from(bytes) * u128::from(rate_per_mib),
        BYTES_PER_MIB,
    ))
    .ok()
}

fn effective_capacity(path: &PathObservation) -> u64 {
    let surviving = PARTS_PER_MILLION - u128::from(path.loss_ppm);
    as_u64((u128::from(path.capacity_bps) * surviving / PARTS_PER_MILLION).max(1))
}

fn transfer_time_us(bytes: u64, capacity_bps: u64) -> u64 {
    if capacity_bps == 0 {
        return u64::MAX;
    }
    as_u64(ceil_div(
        u128::from(bytes) * 8 * 1_000_000,
        u128::from(capacity_bps),
    ))
}

fn finish_time_us(bytes: u64, path: &PathObservation) -> u64 {
    path.rtt_us
        .saturating_add(path.jitter_us)
        .saturating_add(transfer_time_us(bytes, effective_capacity(path)))
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn capacity_domain_ids_error(ids: &[String]) -> Option<&'static str> {
    if ids.is_empty() {
        return Some("capacity_domains_unknown");
    }
    if ids.len() > MAX_CAPACITY_DOMAINS_PER_PATH {
        return Some("capacity_domain_set_too_large");
    }
    if ids.iter().any(|id| id.trim().is_empty()) {
        return Some("capacity_domain_id_blank");
    }
    if ids.iter().any(|id| id.len() > MAX_CAPACITY_DOMAIN_ID_BYTES) {
        return Some("capacity_domain_id_too_long");
    }
    if ids.iter().any(|id| id.chars().any(char::is_control)) {
        return Some("capacity_domain_id_contains_control_character");
    }
    let unique = ids.iter().map(String::as_str).collect::<BTreeSet<_>>();
    if unique.len() != ids.len() {
        return Some("capacity_domain_id_duplicate");
    }
    None
}

fn capacity_domain_evidence_error(
    request: &ScheduleRequest,
    evidence: &CapacityDomainEvidence,
) -> Option<&'static str> {
    if evidence.source_id.trim().is_empty()
        || evidence.source_id.len() > MAX_CAPACITY_EVIDENCE_SOURCE_ID_BYTES
    {
        return Some("capacity_domain_evidence_source_unknown");
    }
    if !is_sha256(&evidence.inventory_sha256) {
        return Some("capacity_domain_inventory_digest_invalid");
    }
    if !is_sha256(&evidence.topology_sha256)
        || evidence.topology_sha256 != request.topology_sha256
    {
        return Some("capacity_domain_topology_digest_mismatch");
    }
    if !evidence.complete {
        return Some("capacity_domain_evidence_incomplete");
    }
    if evidence.confidence_ppm > 1_000_000 {
        return Some("capacity_domain_evidence_confidence_invalid");
    }
    if evidence.confidence_ppm < request.minimum_capacity_domain_confidence_ppm {
        return Some("capacity_domain_evidence_confidence_below_floor");
    }
    if evidence.observed_at_unix_ms == 0
        || evidence.observed_at_unix_ms > request.now_unix_ms
    {
        return Some("capacity_domain_evidence_observation_time_invalid");
    }
    if evidence.valid_until_unix_ms <= evidence.observed_at_unix_ms {
        return Some("capacity_domain_evidence_validity_window_invalid");
    }
    if evidence.valid_until_unix_ms <= request.now_unix_ms {
        return Some("capacity_domain_evidence_stale_or_expired");
    }
    None
}

fn capacity_domains_disjoint(left: &[String], right: &[String]) -> bool {
    left.iter()
        .all(|left_id| right.iter().all(|right_id| left_id != right_id))
}

fn canonical_capacity_domain_ids(ids: &[String]) -> Vec<String> {
    let mut canonical = ids.to_vec();
    canonical.sort();
    canonical
}

fn exclusion_reason(request: &ScheduleRequest, path: &PathObservation) -> Option<&'static str> {
    if !path.broker_eligible {
        return Some("broker_ineligible");
    }
    if !path.authorization_valid {
        return Some("authorization_invalid_or_unknown");
    }
    if !path.telemetry_fresh {
        return Some("telemetry_stale_or_unknown");
    }
    if path.trust_level < request.minimum_trust {
        return Some("trust_below_required_floor");
    }
    if path.mtu_bytes < request.minimum_mtu_bytes {
        return Some("mtu_below_required_floor");
    }
    if path.capacity_bps == 0 {
        return Some("capacity_unavailable");
    }
    if path.loss_ppm >= 1_000_000 {
        return Some("loss_rate_invalid_or_total");
    }
    if path.data_remaining_bytes == 0 {
        return Some("data_allowance_exhausted");
    }
    if path.cost_currency != request.cost_currency {
        return Some("cost_currency_mismatch");
    }
    if path.shared_risk_group.trim().is_empty() {
        return Some("shared_risk_group_unknown");
    }
    if let Some(reason) = capacity_domain_ids_error(&path.capacity_domain_ids) {
        return Some(reason);
    }
    if let Some(reason) = capacity_domain_evidence_error(request, &path.capacity_domain_evidence) {
        return Some(reason);
    }
    None
}

fn validate_request(request: &ScheduleRequest) -> Result<(), ScheduleError> {
    if request.task_id.trim().is_empty() {
        return Err(ScheduleError::EmptyTaskId);
    }
    if request.bytes == 0 {
        return Err(ScheduleError::ZeroPayload);
    }
    if request.maximum_paths == 0 {
        return Err(ScheduleError::ZeroMaximumPaths);
    }
    if !is_sha256(&request.topology_sha256) {
        return Err(ScheduleError::InvalidTopologyDigest);
    }
    if request.now_unix_ms == 0 {
        return Err(ScheduleError::InvalidCurrentTime);
    }
    if request.minimum_capacity_domain_confidence_ppm == 0
        || request.minimum_capacity_domain_confidence_ppm > 1_000_000
    {
        return Err(ScheduleError::InvalidCapacityDomainConfidenceFloor);
    }
    if request.minimum_gain_ppm > 1_000_000 {
        return Err(ScheduleError::InvalidGainThreshold);
    }
    if request.cost_currency.len() != 3
        || !request
            .cost_currency
            .as_bytes()
            .iter()
            .all(|byte| byte.is_ascii_uppercase())
    {
        return Err(ScheduleError::InvalidCurrencyCode);
    }
    Ok(())
}

fn validate_path_set(paths: &[PathObservation]) -> Result<(), ScheduleError> {
    if paths.len() > 64 {
        return Err(ScheduleError::TooManyPaths);
    }
    let mut seen = BTreeSet::new();
    for path in paths {
        if path.id.trim().is_empty() {
            return Err(ScheduleError::InvalidPathId);
        }
        if !seen.insert(path.id.as_str()) {
            return Err(ScheduleError::DuplicatePathId);
        }
        if path.loss_ppm > 1_000_000 {
            return Err(ScheduleError::InvalidLossBound);
        }
    }
    Ok(())
}

fn single_candidate(request: &ScheduleRequest, path: &PathObservation) -> Option<Candidate> {
    if path.data_remaining_bytes < request.bytes {
        return None;
    }
    let cost = estimated_cost(request.bytes, path.cost_micros_per_mib)?;
    let energy = estimated_cost(request.bytes, path.energy_uwh_per_mib)?;
    if cost > request.max_cost_micros || energy > request.max_energy_uwh {
        return None;
    }
    let finish = finish_time_us(request.bytes, path);
    if request
        .deadline_us
        .is_some_and(|deadline| finish > deadline)
    {
        return None;
    }

    let mut groups = BTreeSet::new();
    groups.insert(path.shared_risk_group.as_str());
    let capacity_domains = path
        .capacity_domain_ids
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    Some(Candidate {
        allocations: vec![PathAllocation {
            path_id: path.id.clone(),
            bytes: request.bytes,
            effective_capacity_bps: effective_capacity(path),
            estimated_finish_us: finish,
            estimated_cost_micros: cost,
            estimated_energy_uwh: energy,
            shared_risk_group: path.shared_risk_group.clone(),
            capacity_domain_ids: canonical_capacity_domain_ids(&path.capacity_domain_ids),
            capacity_domain_evidence: path.capacity_domain_evidence.clone(),
        }],
        finish_us: finish,
        cost_micros: cost,
        energy_uwh: energy,
        reorder_exposure_bytes: 0,
        distinct_shared_risk_groups: groups.len(),
        distinct_capacity_domains: capacity_domains.len(),
    })
}

fn candidate_within_budgets(request: &ScheduleRequest, candidate: &Candidate) -> bool {
    candidate.cost_micros <= request.max_cost_micros
        && candidate.energy_uwh <= request.max_energy_uwh
        && request
            .deadline_us
            .is_none_or(|deadline| candidate.finish_us <= deadline)
        && (!request.ordered_delivery
            || candidate.reorder_exposure_bytes <= request.reorder_buffer_limit_bytes)
}

fn aggregate_candidate(
    request: &ScheduleRequest,
    eligible: &[&PathObservation],
) -> Option<Candidate> {
    if !request.allow_multipath || request.maximum_paths < 2 || eligible.len() < 2 {
        return None;
    }

    // Choose the highest-capacity disjoint pair exactly (O(n^2)); the
    // default maximum_paths=2 is the common case and must not lose a valid
    // independent pair merely because a higher-capacity middle path overlaps
    // with both members. For larger path counts, extend that best pair using
    // deterministic greedy admission and report the resulting dependency set.
    let mut best_pair: Option<(u128, &PathObservation, &PathObservation)> = None;
    for left_index in 0..eligible.len() {
        for right_index in left_index + 1..eligible.len() {
            let left = eligible[left_index];
            let right = eligible[right_index];
            if !capacity_domains_disjoint(&left.capacity_domain_ids, &right.capacity_domain_ids) {
                continue;
            }
            let combined_capacity =
                u128::from(effective_capacity(left)) + u128::from(effective_capacity(right));
            let replace =
                best_pair
                    .as_ref()
                    .is_none_or(|(best_capacity, best_left, best_right)| {
                        combined_capacity > *best_capacity
                            || (combined_capacity == *best_capacity
                                && (left.id.as_str(), right.id.as_str())
                                    < (best_left.id.as_str(), best_right.id.as_str()))
                    });
            if replace {
                best_pair = Some((combined_capacity, left, right));
            }
        }
    }
    let Some((_, first, second)) = best_pair else {
        return None;
    };
    let mut selected = vec![first, second];
    let mut seen_capacity_domains = first
        .capacity_domain_ids
        .iter()
        .chain(second.capacity_domain_ids.iter())
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if selected.len() < usize::from(request.maximum_paths) {
        let mut ordered: Vec<&PathObservation> = eligible
            .iter()
            .copied()
            .filter(|path| path.id != first.id && path.id != second.id)
            .collect();
        ordered.sort_by(|left, right| {
            effective_capacity(right)
                .cmp(&effective_capacity(left))
                .then_with(|| left.id.cmp(&right.id))
        });
        for path in ordered {
            let path_domains = path
                .capacity_domain_ids
                .iter()
                .map(String::as_str)
                .collect::<BTreeSet<_>>();
            if path_domains.is_disjoint(&seen_capacity_domains) {
                seen_capacity_domains.extend(path_domains);
                selected.push(path);
            }
            if selected.len() >= usize::from(request.maximum_paths) {
                break;
            }
        }
    }

    let mut remaining = request.bytes;
    let total_capacity: u128 = selected
        .iter()
        .map(|path| u128::from(effective_capacity(path)))
        .sum();
    if total_capacity == 0 {
        return None;
    }

    let mut allocations: Vec<PathAllocation> = selected
        .iter()
        .map(|path| {
            let proportional = as_u64(
                u128::from(request.bytes) * u128::from(effective_capacity(path)) / total_capacity,
            );
            let bytes = proportional.min(path.data_remaining_bytes).min(remaining);
            remaining -= bytes;
            let capacity = effective_capacity(path);
            Some(PathAllocation {
                path_id: path.id.clone(),
                bytes,
                effective_capacity_bps: capacity,
                estimated_finish_us: finish_time_us(bytes, path),
                estimated_cost_micros: estimated_cost(bytes, path.cost_micros_per_mib)?,
                estimated_energy_uwh: estimated_cost(bytes, path.energy_uwh_per_mib)?,
                shared_risk_group: path.shared_risk_group.clone(),
                capacity_domain_ids: canonical_capacity_domain_ids(&path.capacity_domain_ids),
            })
        })
        .collect::<Option<Vec<_>>>()?;

    // Redistribute rounding remainder and quota-limited shares deterministically.
    for allocation in &mut allocations {
        if remaining == 0 {
            break;
        }
        let path = selected
            .iter()
            .find(|path| path.id.as_str() == allocation.path_id.as_str())?;
        let spare = path.data_remaining_bytes.saturating_sub(allocation.bytes);
        let add = spare.min(remaining);
        allocation.bytes = allocation.bytes.saturating_add(add);
        remaining -= add;
        allocation.estimated_finish_us = finish_time_us(allocation.bytes, path);
        allocation.estimated_cost_micros =
            estimated_cost(allocation.bytes, path.cost_micros_per_mib)?;
        allocation.estimated_energy_uwh =
            estimated_cost(allocation.bytes, path.energy_uwh_per_mib)?;
    }
    if remaining != 0 {
        return None;
    }

    allocations.retain(|allocation| allocation.bytes > 0);
    allocations.sort_by(|left, right| left.path_id.cmp(&right.path_id));

    let finish = allocations
        .iter()
        .map(|allocation| allocation.estimated_finish_us)
        .max()?;
    let earliest = allocations
        .iter()
        .map(|allocation| allocation.estimated_finish_us)
        .min()?;
    let reorder_exposure = if request.ordered_delivery {
        allocations
            .iter()
            .filter(|allocation| allocation.estimated_finish_us > earliest)
            .map(|allocation| allocation.bytes)
            .sum()
    } else {
        0
    };
    let cost = allocations.iter().try_fold(0_u64, |sum, allocation| {
        sum.checked_add(allocation.estimated_cost_micros)
    })?;
    let energy = allocations.iter().try_fold(0_u64, |sum, allocation| {
        sum.checked_add(allocation.estimated_energy_uwh)
    })?;
    let groups: BTreeSet<&str> = allocations
        .iter()
        .map(|allocation| allocation.shared_risk_group.as_str())
        .collect();
    let capacity_domains: BTreeSet<&str> = allocations
        .iter()
        .flat_map(|allocation| allocation.capacity_domain_ids.iter().map(String::as_str))
        .collect();

    Some(Candidate {
        allocations,
        finish_us: finish,
        cost_micros: cost,
        energy_uwh: energy,
        reorder_exposure_bytes: reorder_exposure,
        distinct_shared_risk_groups: groups.len(),
        distinct_capacity_domains: capacity_domains.len(),
    })
}

fn best_single_by_finish(candidates: &[Candidate]) -> Option<Candidate> {
    candidates.iter().cloned().min_by(|left, right| {
        left.finish_us
            .cmp(&right.finish_us)
            .then_with(|| left.cost_micros.cmp(&right.cost_micros))
            .then_with(|| {
                left.allocations[0]
                    .path_id
                    .cmp(&right.allocations[0].path_id)
            })
    })
}

fn best_single_by_rtt(candidates: &[Candidate], paths: &[&PathObservation]) -> Option<Candidate> {
    candidates
        .iter()
        .min_by(|left, right| {
            let left_id = &left.allocations[0].path_id;
            let right_id = &right.allocations[0].path_id;
            let left_rtt = paths
                .iter()
                .find(|path| path.id.as_str() == left_id.as_str())
                .map_or(u64::MAX, |path| path.rtt_us);
            let right_rtt = paths
                .iter()
                .find(|path| path.id.as_str() == right_id.as_str())
                .map_or(u64::MAX, |path| path.rtt_us);
            left_rtt
                .cmp(&right_rtt)
                .then_with(|| left.cost_micros.cmp(&right.cost_micros))
                .then_with(|| left_id.cmp(right_id))
        })
        .cloned()
}

fn best_single_by_cost(candidates: &[Candidate]) -> Option<Candidate> {
    candidates.iter().cloned().min_by(|left, right| {
        left.cost_micros
            .cmp(&right.cost_micros)
            .then_with(|| left.finish_us.cmp(&right.finish_us))
            .then_with(|| {
                left.allocations[0]
                    .path_id
                    .cmp(&right.allocations[0].path_id)
            })
    })
}

fn enough_improvement(single: &Candidate, multi: &Candidate, minimum_gain_ppm: u32) -> bool {
    if multi.finish_us >= single.finish_us || single.finish_us == 0 {
        return false;
    }
    let gain = u128::from(single.finish_us - multi.finish_us) * PARTS_PER_MILLION
        / u128::from(single.finish_us);
    gain >= u128::from(minimum_gain_ppm)
}

fn to_decision(
    request: &ScheduleRequest,
    strategy: Strategy,
    candidate: Option<Candidate>,
    exclusions: Vec<PathExclusion>,
    explanation: Vec<String>,
) -> ScheduleDecision {
    match candidate {
        Some(candidate) => ScheduleDecision {
            task_id: request.task_id.clone(),
            strategy,
            mode: if candidate.allocations.len() > 1 {
                DecisionMode::Multipath
            } else {
                DecisionMode::SinglePath
            },
            allocations: candidate.allocations,
            predicted_completion_us: Some(candidate.finish_us),
            estimated_cost_micros: candidate.cost_micros,
            estimated_energy_uwh: candidate.energy_uwh,
            reorder_exposure_bytes: candidate.reorder_exposure_bytes,
            distinct_shared_risk_groups: candidate.distinct_shared_risk_groups,
            distinct_capacity_domains: candidate.distinct_capacity_domains,
            exclusions,
            explanation,
        },
        None => ScheduleDecision {
            task_id: request.task_id.clone(),
            strategy,
            mode: DecisionMode::NoRoute,
            allocations: Vec::new(),
            predicted_completion_us: None,
            estimated_cost_micros: 0,
            estimated_energy_uwh: 0,
            reorder_exposure_bytes: 0,
            distinct_shared_risk_groups: 0,
            distinct_capacity_domains: 0,
            exclusions,
            explanation: vec!["no candidate schedule satisfies hard constraints".to_owned()],
        },
    }
}

/// Evaluate one deterministic scheduling strategy over caller-supplied path
/// observations. Inputs are metadata only; no transport or OS operation occurs.
pub fn evaluate(
    request: &ScheduleRequest,
    paths: &[PathObservation],
    strategy: Strategy,
) -> Result<ScheduleDecision, ScheduleError> {
    validate_request(request)?;
    validate_path_set(paths)?;

    let mut eligible = Vec::new();
    let mut exclusions = Vec::new();
    for path in paths {
        if let Some(reason) = exclusion_reason(request, path) {
            exclusions.push(PathExclusion {
                path_id: path.id.clone(),
                reason,
            });
        } else {
            eligible.push(path);
        }
    }
    exclusions.sort_by(|left, right| left.path_id.cmp(&right.path_id));
    eligible.sort_by(|left, right| left.id.cmp(&right.id));

    let singles: Vec<Candidate> = eligible
        .iter()
        .filter_map(|path| single_candidate(request, path))
        .filter(|candidate| candidate_within_budgets(request, candidate))
        .collect();
    let best_single = best_single_by_finish(&singles);
    let best_rtt_single = best_single_by_rtt(&singles, &eligible);
    let cheapest_single = best_single_by_cost(&singles);
    let multi = aggregate_candidate(request, &eligible)
        .filter(|candidate| candidate_within_budgets(request, candidate));

    let (chosen, explanation) = match strategy {
        Strategy::BestRtt => (
            best_rtt_single,
            vec![
                "selected eligible single path with the lowest estimated completion time"
                    .to_owned(),
            ],
        ),
        Strategy::LowestCost => (
            cheapest_single,
            vec!["selected lowest-cost eligible single path within hard limits".to_owned()],
        ),
        Strategy::AggregateCapacity => {
            if let Some(candidate) = multi {
                (
                    Some(candidate),
                    vec![
                        "split bytes by effective capacity within per-path and session limits"
                            .to_owned(),
                    ],
                )
            } else {
                (
                    best_single,
                    vec![
                        "multipath unavailable or inadmissible; selected the best eligible single path"
                            .to_owned(),
                    ],
                )
            }
        }
        Strategy::IntentAware => {
            let use_multi =
                best_single
                    .as_ref()
                    .zip(multi.as_ref())
                    .is_some_and(|(single, parallel)| {
                        enough_improvement(single, parallel, request.minimum_gain_ppm)
                    });
            if use_multi {
                (
                    multi,
                    vec![
                        "multipath cleared the gain threshold and all hard constraints".to_owned(),
                    ],
                )
            } else {
                let candidate = match request.intent {
                    TrafficIntent::Interactive => best_single,
                    TrafficIntent::BulkTransfer => best_single.or(multi),
                    TrafficIntent::Messaging | TrafficIntent::CostSensitive => {
                        cheapest_single.or(multi)
                    }
                };
                (
                    candidate,
                    vec![
                        "multipath did not clear the gain threshold; task intent favored a single path"
                            .to_owned(),
                    ],
                )
            }
        }
    };

    Ok(to_decision(
        request,
        strategy,
        chosen,
        exclusions,
        explanation,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path(id: &str, rtt_us: u64, capacity_bps: u64, cost: u64, risk: &str) -> PathObservation {
        PathObservation {
            id: id.to_owned(),
            broker_eligible: true,
            authorization_valid: true,
            telemetry_fresh: true,
            trust_level: 2,
            rtt_us,
            jitter_us: 100,
            loss_ppm: 0,
            capacity_bps,
            mtu_bytes: 1500,
            data_remaining_bytes: 16 * 1_048_576,
            cost_currency: "ZAR".to_owned(),
            cost_micros_per_mib: cost,
            energy_uwh_per_mib: 10,
            shared_risk_group: risk.to_owned(),
            capacity_domain_ids: vec![format!("capacity-{id}")],
            capacity_domain_evidence: CapacityDomainEvidence {
                source_id: "operator-inventory-1".to_owned(),
                inventory_sha256: "a".repeat(64),
                topology_sha256: "f".repeat(64),
                complete: true,
                confidence_ppm: 1_000_000,
                observed_at_unix_ms: 900_000,
                valid_until_unix_ms: 1_100_000,
            },
        }
    }

    fn request() -> ScheduleRequest {
        ScheduleRequest {
            task_id: "task-1".to_owned(),
            topology_sha256: "f".repeat(64),
            now_unix_ms: 1_000_000,
            minimum_capacity_domain_confidence_ppm: 950_000,
            intent: TrafficIntent::BulkTransfer,
            bytes: 8 * 1_048_576,
            deadline_us: Some(5_000_000),
            max_cost_micros: 1_000_000,
            cost_currency: "ZAR".to_owned(),
            max_energy_uwh: 1_000_000,
            minimum_trust: 1,
            minimum_mtu_bytes: 1200,
            allow_multipath: true,
            maximum_paths: 2,
            ordered_delivery: false,
            reorder_buffer_limit_bytes: 0,
            minimum_gain_ppm: 10_000,
        }
    }

    #[test]
    fn aggregate_strategy_uses_two_eligible_paths() {
        let result = evaluate(
            &request(),
            &[
                path("a", 10_000, 20_000_000, 100, "wan-a"),
                path("b", 15_000, 80_000_000, 100, "wan-b"),
            ],
            Strategy::AggregateCapacity,
        )
        .expect("valid inputs");
        assert_eq!(result.mode, DecisionMode::Multipath);
        assert_eq!(result.allocations.len(), 2);
        assert_eq!(
            result
                .allocations
                .iter()
                .map(|item| item.bytes)
                .sum::<u64>(),
            request().bytes
        );
    }

    #[test]
    fn aggregate_never_counts_two_paths_in_one_capacity_domain() {
        let mut first = path("a", 10_000, 40_000_000, 10, "failure-a");
        let mut second = path("b", 20_000, 60_000_000, 10, "failure-b");
        first.capacity_domain_ids = vec!["access-a".to_owned(), "shared-backhaul".to_owned()];
        second.capacity_domain_ids = vec!["access-b".to_owned(), "shared-backhaul".to_owned()];
        let result = evaluate(&request(), &[first, second], Strategy::AggregateCapacity)
            .expect("valid observations");
        assert_eq!(result.mode, DecisionMode::SinglePath);
        assert_eq!(result.allocations.len(), 1);
        assert_eq!(result.distinct_shared_risk_groups, 1);
        assert_eq!(result.distinct_capacity_domains, 2);
    }

    #[test]
    fn chooses_the_best_independent_pair_when_highest_path_overlaps_both() {
        let mut path_a = path("a", 10_000, 60_000_000, 10, "risk-a");
        let mut path_b = path("b", 10_000, 100_000_000, 10, "risk-b");
        let mut path_c = path("c", 10_000, 50_000_000, 10, "risk-c");
        path_a.capacity_domain_ids = vec!["access-a".to_owned(), "shared-x".to_owned()];
        path_b.capacity_domain_ids = vec!["shared-x".to_owned(), "shared-y".to_owned()];
        path_c.capacity_domain_ids = vec!["access-c".to_owned(), "shared-y".to_owned()];
        let result = evaluate(
            &request(),
            &[path_b, path_c, path_a],
            Strategy::AggregateCapacity,
        )
        .expect("valid multi-domain observations");
        assert_eq!(result.mode, DecisionMode::Multipath);
        assert_eq!(
            result
                .allocations
                .iter()
                .map(|allocation| allocation.path_id.as_str())
                .collect::<Vec<_>>(),
            vec!["a", "c"]
        );
        assert_eq!(result.distinct_capacity_domains, 4);
    }

    #[test]
    fn blank_and_duplicate_capacity_domain_ids_exclude_only_the_invalid_path() {
        let mut blank = path("blank", 1_000, 100_000_000, 1, "risk-blank");
        blank.capacity_domain_ids = vec!["access".to_owned(), " ".to_owned()];
        let mut duplicate = path("duplicate", 2_000, 90_000_000, 1, "risk-duplicate");
        duplicate.capacity_domain_ids = vec!["access".to_owned(), "access".to_owned()];
        let allowed_a = path("allowed-a", 20_000, 20_000_000, 10, "risk-a");
        let allowed_b = path("allowed-b", 25_000, 25_000_000, 10, "risk-b");
        let result = evaluate(
            &request(),
            &[blank, duplicate, allowed_a, allowed_b],
            Strategy::AggregateCapacity,
        )
        .expect("invalid individual dependencies are excluded");
        assert_eq!(result.mode, DecisionMode::Multipath);
        assert_eq!(result.exclusions.len(), 2);
        assert_eq!(result.exclusions[0].reason, "capacity_domain_id_blank");
        assert_eq!(result.exclusions[1].reason, "capacity_domain_id_duplicate");
    }

    #[test]
    fn incomplete_capacity_domain_inventory_is_excluded() {
        let mut path = path("incomplete", 10_000, 20_000_000, 10, "risk-a");
        path.capacity_domain_evidence.complete = false;
        let result = evaluate(&request(), &[path], Strategy::BestRtt).expect("valid path");
        assert_eq!(result.mode, DecisionMode::NoRoute);
        assert_eq!(
            result.exclusions[0].reason,
            "capacity_domain_evidence_incomplete"
        );
    }

    #[test]
    fn expired_capacity_domain_inventory_is_excluded() {
        let mut path = path("expired", 10_000, 20_000_000, 10, "risk-a");
        path.capacity_domain_evidence.valid_until_unix_ms = request().now_unix_ms;
        let result = evaluate(&request(), &[path], Strategy::BestRtt).expect("valid path");
        assert_eq!(result.mode, DecisionMode::NoRoute);
        assert_eq!(
            result.exclusions[0].reason,
            "capacity_domain_evidence_stale_or_expired"
        );
    }

    #[test]
    fn capacity_domain_inventory_from_another_topology_is_excluded() {
        let mut path = path("wrong-topology", 10_000, 20_000_000, 10, "risk-a");
        path.capacity_domain_evidence.topology_sha256 = "b".repeat(64);
        let result = evaluate(&request(), &[path], Strategy::BestRtt).expect("valid path");
        assert_eq!(result.mode, DecisionMode::NoRoute);
        assert_eq!(
            result.exclusions[0].reason,
            "capacity_domain_topology_digest_mismatch"
        );
    }

    #[test]
    fn malformed_inventory_digest_is_excluded() {
        let mut path = path("bad-inventory", 10_000, 20_000_000, 10, "risk-a");
        path.capacity_domain_evidence.inventory_sha256 = "not-a-digest".to_owned();
        let result = evaluate(&request(), &[path], Strategy::BestRtt).expect("valid path");
        assert_eq!(result.mode, DecisionMode::NoRoute);
        assert_eq!(
            result.exclusions[0].reason,
            "capacity_domain_inventory_digest_invalid"
        );
    }

    #[test]
    fn low_confidence_capacity_domain_inventory_is_excluded() {
        let mut path = path("low-confidence", 10_000, 20_000_000, 10, "risk-a");
        path.capacity_domain_evidence.confidence_ppm = 949_999;
        let result = evaluate(&request(), &[path], Strategy::BestRtt).expect("valid path");
        assert_eq!(result.mode, DecisionMode::NoRoute);
        assert_eq!(
            result.exclusions[0].reason,
            "capacity_domain_evidence_confidence_below_floor"
        );
    }

    #[test]
    fn confidence_above_one_million_is_invalid() {
        let mut path = path("invalid-confidence", 10_000, 20_000_000, 10, "risk-a");
        path.capacity_domain_evidence.confidence_ppm = 1_000_001;
        let result = evaluate(&request(), &[path], Strategy::BestRtt).expect("invalid evidence is excluded");
        assert_eq!(result.mode, DecisionMode::NoRoute);
        assert_eq!(
            result.exclusions[0].reason,
            "capacity_domain_evidence_confidence_invalid"
        );
    }

    #[test]
    fn future_capacity_domain_observation_is_excluded() {
        let mut path = path("future", 10_000, 20_000_000, 10, "risk-a");
        path.capacity_domain_evidence.observed_at_unix_ms = request().now_unix_ms + 1;
        let result = evaluate(&request(), &[path], Strategy::BestRtt).expect("valid path");
        assert_eq!(result.mode, DecisionMode::NoRoute);
        assert_eq!(
            result.exclusions[0].reason,
            "capacity_domain_evidence_observation_time_invalid"
        );
    }

    #[test]
    fn capacity_domain_id_length_is_bounded() {
        let mut path = path("too-long", 10_000, 20_000_000, 10, "risk-a");
        path.capacity_domain_ids = vec!["x".repeat(MAX_CAPACITY_DOMAIN_ID_BYTES + 1)];
        let result = evaluate(&request(), &[path], Strategy::BestRtt).expect("valid path");
        assert_eq!(result.mode, DecisionMode::NoRoute);
        assert_eq!(result.exclusions[0].reason, "capacity_domain_id_too_long");
    }

    #[test]
    fn capacity_domain_ids_are_canonicalized_in_the_report() {
        let mut first = path("a", 10_000, 20_000_000, 10, "risk-a");
        first.capacity_domain_ids = vec!["z".to_owned(), "a".to_owned()];
        let result = evaluate(&request(), &[first], Strategy::BestRtt).expect("valid path");
        assert_eq!(
            result.allocations[0].capacity_domain_ids,
            vec!["a".to_owned(), "z".to_owned()]
        );
    }

    #[test]
    fn unknown_capacity_domain_is_excluded_before_aggregation() {
        let mut unknown = path("unknown", 1_000, 100_000_000, 1, "failure-a");
        unknown.capacity_domain_ids.clear();
        let allowed = path("allowed", 20_000, 10_000_000, 10, "failure-b");
        let result = evaluate(&request(), &[unknown, allowed], Strategy::AggregateCapacity)
            .expect("valid observations");
        assert_eq!(result.mode, DecisionMode::SinglePath);
        assert_eq!(result.exclusions[0].reason, "capacity_domains_unknown");
    }

    #[test]
    fn broker_ineligible_path_is_never_scheduled() {
        let mut ineligible = path("blocked", 1, 1_000_000_000, 0, "blocked");
        ineligible.broker_eligible = false;
        let result = evaluate(
            &request(),
            &[
                ineligible,
                path("allowed", 20_000, 10_000_000, 50, "allowed"),
            ],
            Strategy::AggregateCapacity,
        )
        .expect("valid inputs");
        assert!(
            result
                .allocations
                .iter()
                .all(|item| item.path_id != "blocked")
        );
        assert_eq!(result.exclusions[0].reason, "broker_ineligible");
    }

    #[test]
    fn cost_and_energy_caps_are_hard_limits() {
        let mut req = request();
        req.max_cost_micros = 1;
        let result = evaluate(
            &req,
            &[path("expensive", 1, 100_000_000, 10_000, "a")],
            Strategy::LowestCost,
        )
        .expect("valid inputs");
        assert_eq!(result.mode, DecisionMode::NoRoute);
    }

    #[test]
    fn ordered_delivery_rejects_reorder_exposure_above_buffer() {
        let mut req = request();
        req.ordered_delivery = true;
        req.reorder_buffer_limit_bytes = 1;
        let result = evaluate(
            &req,
            &[
                path("fast", 1_000, 100_000_000, 10, "a"),
                path("slow", 200_000, 20_000_000, 10, "b"),
            ],
            Strategy::AggregateCapacity,
        )
        .expect("valid inputs");
        assert_eq!(result.mode, DecisionMode::SinglePath);
    }

    #[test]
    fn stale_and_untrusted_paths_are_excluded() {
        let mut stale = path("stale", 1, 50_000_000, 1, "a");
        stale.telemetry_fresh = false;
        let mut untrusted = path("untrusted", 1, 50_000_000, 1, "b");
        untrusted.trust_level = 0;
        let result =
            evaluate(&request(), &[stale, untrusted], Strategy::BestRtt).expect("valid inputs");
        assert_eq!(result.mode, DecisionMode::NoRoute);
        assert_eq!(result.exclusions.len(), 2);
    }

    #[test]
    fn same_inputs_are_deterministic_under_candidate_permutation() {
        let paths = vec![
            path("b", 50_000, 40_000_000, 20, "b"),
            path("a", 10_000, 20_000_000, 15, "a"),
        ];
        let first = evaluate(&request(), &paths, Strategy::IntentAware).expect("valid inputs");
        let second = evaluate(
            &request(),
            &paths.into_iter().rev().collect::<Vec<_>>(),
            Strategy::IntentAware,
        )
        .expect("valid inputs");
        assert_eq!(first, second);
    }

    #[test]
    fn duplicate_ids_are_rejected() {
        let paths = vec![
            path("same", 10_000, 20_000_000, 10, "a"),
            path("same", 20_000, 30_000_000, 10, "b"),
        ];
        assert_eq!(
            evaluate(&request(), &paths, Strategy::BestRtt).unwrap_err(),
            ScheduleError::DuplicatePathId
        );
    }

    #[test]
    fn best_rtt_baseline_selects_minimum_rtt_not_fastest_completion() {
        let mut req = request();
        req.deadline_us = None;
        req.allow_multipath = false;
        let paths = vec![
            path("low-rtt-slow-capacity", 1_000, 1_000_000, 10, "a"),
            path("high-rtt-high-capacity", 100_000, 100_000_000, 10, "b"),
        ];
        let result = evaluate(&req, &paths, Strategy::BestRtt).expect("valid inputs");
        assert_eq!(result.mode, DecisionMode::SinglePath);
        assert_eq!(result.allocations[0].path_id, "low-rtt-slow-capacity");

        let fastest = evaluate(&req, &paths, Strategy::IntentAware).expect("valid inputs");
        assert_eq!(fastest.allocations[0].path_id, "high-rtt-high-capacity");
    }

    #[test]
    fn mixed_currency_paths_are_excluded_before_ranking() {
        let mut usd = path("usd", 1_000, 100_000_000, 1, "usd");
        usd.cost_currency = "USD".to_owned();
        let zar = path("zar", 10_000, 10_000_000, 10, "zar");
        let result = evaluate(&request(), &[usd, zar], Strategy::LowestCost).expect("valid inputs");
        assert_eq!(result.mode, DecisionMode::SinglePath);
        assert_eq!(result.allocations[0].path_id, "zar");
        assert_eq!(result.exclusions[0].reason, "cost_currency_mismatch");
    }

    #[test]
    fn malformed_request_currency_is_rejected() {
        let mut req = request();
        req.cost_currency = "zar".to_owned();
        assert_eq!(
            evaluate(
                &req,
                &[path("a", 1_000, 10_000_000, 1, "a")],
                Strategy::BestRtt
            )
            .unwrap_err(),
            ScheduleError::InvalidCurrencyCode
        );
    }

    #[test]
    fn aggregate_cost_overflow_fails_closed() {
        let mut req = request();
        req.max_cost_micros = u64::MAX;
        let paths = vec![
            path("a", 10_000, 40_000_000, u64::MAX, "a"),
            path("b", 10_000, 40_000_000, u64::MAX, "b"),
        ];
        let result = evaluate(&req, &paths, Strategy::AggregateCapacity)
            .expect("valid inputs produce a no-route disposition");
        assert_eq!(result.mode, DecisionMode::NoRoute);
        assert!(result.allocations.is_empty());
    }

    #[test]
    fn no_multipath_when_peer_or_profile_disables_it() {
        let mut req = request();
        req.allow_multipath = false;
        let result = evaluate(
            &req,
            &[
                path("a", 10_000, 10_000_000, 10, "a"),
                path("b", 10_000, 100_000_000, 10, "b"),
            ],
            Strategy::AggregateCapacity,
        )
        .expect("valid inputs");
        assert_eq!(result.mode, DecisionMode::SinglePath);
    }
}
