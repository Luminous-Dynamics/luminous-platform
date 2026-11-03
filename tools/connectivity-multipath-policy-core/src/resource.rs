//! Bounded resource-incidence allocation over measured shared bottlenecks.
//!
//! This module is a deterministic planning model. It does not discover paths,
//! measure links, authenticate operators or actuate a router/transport. Every
//! resource limit is derived from a topology-bound evidence record.

use std::collections::{BTreeMap, BTreeSet};

const RATE_SCALE: u128 = 1_000_000;
const MAX_PATHS: usize = 64;
const MAX_RESOURCES: usize = 64;
const MAX_DOMAINS_PER_PATH: usize = 32;
const MAX_ID_BYTES: usize = 128;
const MAX_SOURCE_ID_BYTES: usize = 256;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapacityResourceEvidence {
    pub source_id: String,
    pub inventory_sha256: String,
    pub topology_sha256: String,
    pub complete: bool,
    /// Evidence grade on a declared 0..=1,000,000 scale, not a probability.
    pub confidence_ppm: u32,
    pub measurement_window_start_unix_ms: u64,
    pub measurement_window_end_unix_ms: u64,
    pub observed_at_unix_ms: u64,
    pub valid_until_unix_ms: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapacityResource {
    pub resource_id: String,
    /// Configured or declared hard resource rate.
    pub declared_capacity_bps: u64,
    /// Measured background load; excludes committed reservations below.
    pub measured_background_load_bps: u64,
    /// Capacity committed to other traffic but not included in measured load.
    pub committed_reservations_bps: u64,
    /// Operator safety headroom deliberately withheld from this allocator.
    pub safety_margin_bps: u64,
    pub evidence: CapacityResourceEvidence,
}

impl CapacityResource {
    /// Compute the capacity available to this allocation snapshot.
    pub fn residual_capacity_bps(&self) -> Result<u64, ResourceAllocationError> {
        let consumed = self
            .measured_background_load_bps
            .checked_add(self.committed_reservations_bps)
            .and_then(|value| value.checked_add(self.safety_margin_bps))
            .ok_or(ResourceAllocationError::ArithmeticOverflow)?;
        self.declared_capacity_bps
            .checked_sub(consumed)
            .ok_or(ResourceAllocationError::ResourceAccountingExceedsCapacity)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourcePath {
    pub path_id: String,
    /// Path-specific ceiling, independent of shared resource ceilings.
    pub path_ceiling_bps: u64,
    /// Relative entitlement for weighted progressive filling.
    pub weight: u32,
    /// Every known hard capacity bottleneck consumed by this path.
    pub capacity_domain_ids: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceAllocationRequest {
    pub topology_sha256: String,
    /// Caller-supplied timestamp used for deterministic evidence validation.
    pub now_unix_ms: u64,
    pub minimum_evidence_confidence_ppm: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResourceAllocationError {
    InvalidTopologyDigest,
    InvalidDecisionTime,
    InvalidConfidenceFloor,
    TooManyPaths,
    TooManyResources,
    InvalidPathId,
    DuplicatePathId,
    InvalidResourceId,
    DuplicateResourceId,
    InvalidEvidenceEnvelope,
    InvalidResourceCapacity,
    ResourceAccountingExceedsCapacity,
    InvalidPathCapacity,
    InvalidPathWeight,
    TooManyDomainsPerPath,
    EmptyCapacityDomainSet,
    BlankCapacityDomainId,
    DuplicateCapacityDomainId,
    ArithmeticOverflow,
    InternalInvariant,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourcePathExclusion {
    pub path_id: String,
    pub reason: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PathRateAllocation {
    pub path_id: String,
    pub path_ceiling_bps: u64,
    pub weight: u32,
    pub allocated_rate_bps: u64,
    pub saturated: bool,
    pub capacity_domain_ids: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceDomainUsage {
    pub resource_id: String,
    pub residual_capacity_bps: u64,
    pub allocated_rate_bps: u64,
    pub unused_capacity_bps: u64,
    pub saturated: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceAllocationReport {
    pub topology_sha256: String,
    pub decision_time_unix_ms: u64,
    pub total_allocated_rate_bps: u64,
    pub path_allocations: Vec<PathRateAllocation>,
    pub resource_usage: Vec<ResourceDomainUsage>,
    pub saturated_path_ids: Vec<String>,
    pub saturated_resource_ids: Vec<String>,
    pub exclusions: Vec<ResourcePathExclusion>,
    /// This is a weighted max-min result, not a global maximum-goodput claim.
    pub objective: &'static str,
}

#[derive(Clone, Debug)]
struct EligiblePath {
    path: ResourcePath,
    domain_indexes: Vec<usize>,
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn valid_identifier(value: &str) -> bool {
    !value.trim().is_empty()
        && value.len() <= MAX_ID_BYTES
        && !value.chars().any(char::is_control)
}

fn validate_request(request: &ResourceAllocationRequest) -> Result<(), ResourceAllocationError> {
    if !is_sha256(&request.topology_sha256) {
        return Err(ResourceAllocationError::InvalidTopologyDigest);
    }
    if request.now_unix_ms == 0 {
        return Err(ResourceAllocationError::InvalidDecisionTime);
    }
    if request.minimum_evidence_confidence_ppm == 0
        || request.minimum_evidence_confidence_ppm > 1_000_000
    {
        return Err(ResourceAllocationError::InvalidConfidenceFloor);
    }
    Ok(())
}

fn evidence_error(
    request: &ResourceAllocationRequest,
    evidence: &CapacityResourceEvidence,
) -> Option<&'static str> {
    if !valid_identifier(&evidence.source_id) || evidence.source_id.len() > MAX_SOURCE_ID_BYTES {
        return Some("resource_evidence_source_invalid");
    }
    if !is_sha256(&evidence.inventory_sha256) {
        return Some("resource_inventory_digest_invalid");
    }
    if !is_sha256(&evidence.topology_sha256)
        || evidence.topology_sha256 != request.topology_sha256
    {
        return Some("resource_topology_digest_mismatch");
    }
    if !evidence.complete {
        return Some("resource_inventory_incomplete");
    }
    if evidence.confidence_ppm < request.minimum_evidence_confidence_ppm
        || evidence.confidence_ppm > 1_000_000
    {
        return Some("resource_evidence_confidence_out_of_range_or_below_floor");
    }
    if evidence.measurement_window_start_unix_ms == 0
        || evidence.measurement_window_start_unix_ms >= evidence.measurement_window_end_unix_ms
        || evidence.measurement_window_end_unix_ms > evidence.observed_at_unix_ms
        || evidence.observed_at_unix_ms > request.now_unix_ms
    {
        return Some("resource_measurement_window_invalid");
    }
    if evidence.valid_until_unix_ms <= evidence.observed_at_unix_ms
        || evidence.valid_until_unix_ms <= request.now_unix_ms
    {
        return Some("resource_evidence_stale_or_expired");
    }
    None
}

fn canonical_ids(ids: &[String]) -> Vec<String> {
    let mut result = ids.to_vec();
    result.sort();
    result
}

/// Allocate path rates with deterministic weighted max-min progressive filling.
///
/// The algorithm increases all active paths proportionally to their weight
/// until a path ceiling or a shared resource residual is reached. Paths using a
/// saturated resource freeze; paths not using that resource may continue.
pub fn allocate_resource_incidence(
    request: &ResourceAllocationRequest,
    paths: &[ResourcePath],
    resources: &[CapacityResource],
) -> Result<ResourceAllocationReport, ResourceAllocationError> {
    validate_request(request)?;
    if paths.len() > MAX_PATHS {
        return Err(ResourceAllocationError::TooManyPaths);
    }
    if resources.len() > MAX_RESOURCES {
        return Err(ResourceAllocationError::TooManyResources);
    }

    let mut resource_index = BTreeMap::new();
    let mut resource_residuals = Vec::with_capacity(resources.len());
    let mut resource_ids = BTreeSet::new();
    for (index, resource) in resources.iter().enumerate() {
        if !valid_identifier(&resource.resource_id) {
            return Err(ResourceAllocationError::InvalidResourceId);
        }
        if !resource_ids.insert(resource.resource_id.as_str()) {
            return Err(ResourceAllocationError::DuplicateResourceId);
        }
        if resource.declared_capacity_bps == 0 {
            return Err(ResourceAllocationError::InvalidResourceCapacity);
        }
        if evidence_error(request, &resource.evidence).is_some() {
            // Invalid resource evidence is not a global failure: paths using
            // this resource are excluded below, while independent resources
            // remain eligible.
            resource_residuals.push(None);
        } else {
            resource_residuals.push(Some(resource.residual_capacity_bps()?));
        }
        resource_index.insert(resource.resource_id.as_str(), index);
    }

    let mut seen_path_ids = BTreeSet::new();
    let mut exclusions = Vec::new();
    let mut eligible = Vec::new();
    let mut ordered_paths = paths.iter().collect::<Vec<_>>();
    ordered_paths.sort_by(|left, right| left.path_id.cmp(&right.path_id));

    for path in ordered_paths {
        if !valid_identifier(&path.path_id) {
            return Err(ResourceAllocationError::InvalidPathId);
        }
        if !seen_path_ids.insert(path.path_id.as_str()) {
            return Err(ResourceAllocationError::DuplicatePathId);
        }
        if path.path_ceiling_bps == 0 {
            exclusions.push(ResourcePathExclusion {
                path_id: path.path_id.clone(),
                reason: "path_capacity_unavailable",
            });
            continue;
        }
        if path.weight == 0 {
            exclusions.push(ResourcePathExclusion {
                path_id: path.path_id.clone(),
                reason: "path_weight_invalid",
            });
            continue;
        }
        if path.capacity_domain_ids.is_empty() {
            exclusions.push(ResourcePathExclusion {
                path_id: path.path_id.clone(),
                reason: "capacity_domains_unknown",
            });
            continue;
        }
        if path.capacity_domain_ids.len() > MAX_DOMAINS_PER_PATH {
            exclusions.push(ResourcePathExclusion {
                path_id: path.path_id.clone(),
                reason: "capacity_domain_set_too_large",
            });
            continue;
        }
        if path
            .capacity_domain_ids
            .iter()
            .any(|domain| !valid_identifier(domain))
        {
            exclusions.push(ResourcePathExclusion {
                path_id: path.path_id.clone(),
                reason: "capacity_domain_id_invalid",
            });
            continue;
        }
        let unique_domains = path
            .capacity_domain_ids
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        if unique_domains.len() != path.capacity_domain_ids.len() {
            exclusions.push(ResourcePathExclusion {
                path_id: path.path_id.clone(),
                reason: "capacity_domain_id_duplicate",
            });
            continue;
        }

        let mut domain_indexes = Vec::with_capacity(path.capacity_domain_ids.len());
        let mut exclusion_reason = None;
        for domain_id in &path.capacity_domain_ids {
            let Some(index) = resource_index.get(domain_id.as_str()).copied() else {
                exclusion_reason = Some("capacity_resource_unknown");
                break;
            };
            let Some(residual) = resource_residuals[index] else {
                exclusion_reason = Some("capacity_resource_evidence_invalid");
                break;
            };
            if residual == 0 {
                exclusion_reason = Some("capacity_resource_residual_zero");
                break;
            }
            domain_indexes.push(index);
        }
        if let Some(reason) = exclusion_reason {
            exclusions.push(ResourcePathExclusion {
                path_id: path.path_id.clone(),
                reason,
            });
            continue;
        }
        domain_indexes.sort_unstable();
        eligible.push(EligiblePath {
            path: path.clone(),
            domain_indexes,
        });
    }

    let mut rates_scaled = vec![0_u128; eligible.len()];
    let path_ceiling_scaled = eligible
        .iter()
        .map(|item| u128::from(item.path.path_ceiling_bps) * RATE_SCALE)
        .collect::<Vec<_>>();
    let mut remaining_scaled = resource_residuals
        .iter()
        .map(|value| value.map(|bps| u128::from(bps) * RATE_SCALE))
        .collect::<Vec<_>>();
    let mut active = vec![true; eligible.len()];

    for _iteration in 0..=eligible.len() {
        if !active.iter().any(|is_active| *is_active) {
            break;
        }

        let mut delta_level: Option<u128> = None;
        for (index, item) in eligible.iter().enumerate() {
            if !active[index] {
                continue;
            }
            let headroom = path_ceiling_scaled[index]
                .checked_sub(rates_scaled[index])
                .ok_or(ResourceAllocationError::InternalInvariant)?;
            let candidate = headroom / u128::from(item.path.weight);
            delta_level = Some(delta_level.map_or(candidate, |current| current.min(candidate)));
        }

        let mut active_domain_weights = vec![0_u128; resources.len()];
        for (index, item) in eligible.iter().enumerate() {
            if !active[index] {
                continue;
            }
            for domain_index in &item.domain_indexes {
                active_domain_weights[*domain_index] = active_domain_weights[*domain_index]
                    .checked_add(u128::from(item.path.weight))
                    .ok_or(ResourceAllocationError::ArithmeticOverflow)?;
            }
        }
        for (domain_index, active_weight) in active_domain_weights.iter().enumerate() {
            if *active_weight == 0 {
                continue;
            }
            let remaining = remaining_scaled[domain_index]
                .ok_or(ResourceAllocationError::InternalInvariant)?;
            let candidate = remaining / *active_weight;
            delta_level = Some(delta_level.map_or(candidate, |current| current.min(candidate)));
        }

        let delta = delta_level.ok_or(ResourceAllocationError::InternalInvariant)?;
        if delta == 0 {
            let mut freeze = vec![false; eligible.len()];
            for (index, item) in eligible.iter().enumerate() {
                if !active[index] {
                    continue;
                }
                let path_headroom = path_ceiling_scaled[index]
                    .checked_sub(rates_scaled[index])
                    .ok_or(ResourceAllocationError::InternalInvariant)?;
                if path_headroom < u128::from(item.path.weight) {
                    freeze[index] = true;
                }
            }
            for (domain_index, active_weight) in active_domain_weights.iter().enumerate() {
                if *active_weight == 0 {
                    continue;
                }
                let remaining = remaining_scaled[domain_index]
                    .ok_or(ResourceAllocationError::InternalInvariant)?;
                if remaining < *active_weight {
                    for (index, item) in eligible.iter().enumerate() {
                        if active[index] && item.domain_indexes.contains(&domain_index) {
                            freeze[index] = true;
                        }
                    }
                }
            }
            if !freeze.iter().any(|should_freeze| *should_freeze) {
                return Err(ResourceAllocationError::InternalInvariant);
            }
            for (index, should_freeze) in freeze.iter().enumerate() {
                if *should_freeze {
                    active[index] = false;
                }
            }
            continue;
        }

        for (index, item) in eligible.iter().enumerate() {
            if active[index] {
                let increment = delta
                    .checked_mul(u128::from(item.path.weight))
                    .ok_or(ResourceAllocationError::ArithmeticOverflow)?;
                rates_scaled[index] = rates_scaled[index]
                    .checked_add(increment)
                    .ok_or(ResourceAllocationError::ArithmeticOverflow)?;
            }
        }
        for (domain_index, active_weight) in active_domain_weights.iter().enumerate() {
            if *active_weight == 0 {
                continue;
            }
            let used = delta
                .checked_mul(*active_weight)
                .ok_or(ResourceAllocationError::ArithmeticOverflow)?;
            let remaining = remaining_scaled[domain_index]
                .ok_or(ResourceAllocationError::InternalInvariant)?;
            remaining_scaled[domain_index] = Some(
                remaining
                    .checked_sub(used)
                    .ok_or(ResourceAllocationError::InternalInvariant)?,
            );
        }

        let mut freeze = vec![false; eligible.len()];
        for (index, item) in eligible.iter().enumerate() {
            if active[index]
                && rates_scaled[index] >= path_ceiling_scaled[index]
            {
                freeze[index] = true;
            }
        }
        for (domain_index, active_weight) in active_domain_weights.iter().enumerate() {
            if *active_weight == 0 {
                continue;
            }
            let remaining = remaining_scaled[domain_index]
                .ok_or(ResourceAllocationError::InternalInvariant)?;
            if remaining < *active_weight {
                for (index, item) in eligible.iter().enumerate() {
                    if active[index] && item.domain_indexes.contains(&domain_index) {
                        freeze[index] = true;
                    }
                }
            }
        }
        if !freeze.iter().any(|should_freeze| *should_freeze) {
            return Err(ResourceAllocationError::InternalInvariant);
        }
        for (index, should_freeze) in freeze.iter().enumerate() {
            if *should_freeze {
                active[index] = false;
            }
        }
    }

    if active.iter().any(|is_active| *is_active) {
        return Err(ResourceAllocationError::InternalInvariant);
    }

    let mut integer_rates = rates_scaled
        .iter()
        .map(|value| u64::try_from(value / RATE_SCALE).map_err(|_| ResourceAllocationError::ArithmeticOverflow))
        .collect::<Result<Vec<_>, _>>()?;
    let mut fractional_order = (0..eligible.len()).collect::<Vec<_>>();
    fractional_order.sort_by(|left, right| {
        (rates_scaled[*right] % RATE_SCALE)
            .cmp(&(rates_scaled[*left] % RATE_SCALE))
            .then_with(|| eligible[*left].path.path_id.cmp(&eligible[*right].path.path_id))
    });

    // Largest-remainder bps rounding is admitted only when every resource on
    // the path has one full bps of residual headroom. This cannot overcommit.
    for index in fractional_order {
        if integer_rates[index] >= eligible[index].path.path_ceiling_bps {
            continue;
        }
        let fits_all_resources = eligible[index].domain_indexes.iter().all(|domain_index| {
            let Some(capacity) = resource_residuals[*domain_index] else {
                return false;
            };
            let used = eligible
                .iter()
                .enumerate()
                .filter(|(_, path)| path.domain_indexes.contains(domain_index))
                .try_fold(0_u64, |sum, (path_index, _)| {
                    sum.checked_add(integer_rates[path_index])
                });
            used.is_some_and(|used| used < capacity)
        });
        if fits_all_resources {
            integer_rates[index] = integer_rates[index]
                .checked_add(1)
                .ok_or(ResourceAllocationError::ArithmeticOverflow)?;
        }
    }

    let mut domain_used = vec![0_u64; resources.len()];
    for (index, item) in eligible.iter().enumerate() {
        for domain_index in &item.domain_indexes {
            domain_used[*domain_index] = domain_used[*domain_index]
                .checked_add(integer_rates[index])
                .ok_or(ResourceAllocationError::ArithmeticOverflow)?;
        }
    }

    let mut path_allocations = Vec::with_capacity(eligible.len());
    let mut saturated_path_ids = Vec::new();
    let mut total_allocated_rate_bps = 0_u64;
    for (index, item) in eligible.iter().enumerate() {
        let allocated = integer_rates[index];
        if allocated > item.path.path_ceiling_bps {
            return Err(ResourceAllocationError::InternalInvariant);
        }
        total_allocated_rate_bps = total_allocated_rate_bps
            .checked_add(allocated)
            .ok_or(ResourceAllocationError::ArithmeticOverflow)?;
        let saturated = allocated == item.path.path_ceiling_bps;
        if saturated {
            saturated_path_ids.push(item.path.path_id.clone());
        }
        path_allocations.push(PathRateAllocation {
            path_id: item.path.path_id.clone(),
            path_ceiling_bps: item.path.path_ceiling_bps,
            weight: item.path.weight,
            allocated_rate_bps: allocated,
            saturated,
            capacity_domain_ids: canonical_ids(&item.path.capacity_domain_ids),
        });
    }

    let mut resource_usage = Vec::with_capacity(resources.len());
    let mut saturated_resource_ids = Vec::new();
    for (index, resource) in resources.iter().enumerate() {
        let Some(residual) = resource_residuals[index] else {
            continue;
        };
        let allocated = domain_used[index];
        if allocated > residual {
            return Err(ResourceAllocationError::InternalInvariant);
        }
        let unused = residual - allocated;
        let saturated = unused == 0;
        if saturated {
            saturated_resource_ids.push(resource.resource_id.clone());
        }
        resource_usage.push(ResourceDomainUsage {
            resource_id: resource.resource_id.clone(),
            residual_capacity_bps: residual,
            allocated_rate_bps: allocated,
            unused_capacity_bps: unused,
            saturated,
        });
    }
    path_allocations.sort_by(|left, right| left.path_id.cmp(&right.path_id));
    resource_usage.sort_by(|left, right| left.resource_id.cmp(&right.resource_id));
    saturated_path_ids.sort();
    saturated_resource_ids.sort();
    exclusions.sort_by(|left, right| left.path_id.cmp(&right.path_id));

    Ok(ResourceAllocationReport {
        topology_sha256: request.topology_sha256.clone(),
        decision_time_unix_ms: request.now_unix_ms,
        total_allocated_rate_bps,
        path_allocations,
        resource_usage,
        saturated_path_ids,
        saturated_resource_ids,
        exclusions,
        objective: "weighted_max_min_progressive_filling_not_global_sum_rate_optimum",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evidence() -> CapacityResourceEvidence {
        CapacityResourceEvidence {
            source_id: "operator-inventory".to_owned(),
            inventory_sha256: "a".repeat(64),
            topology_sha256: "b".repeat(64),
            complete: true,
            confidence_ppm: 1_000_000,
            measurement_window_start_unix_ms: 800,
            measurement_window_end_unix_ms: 900,
            observed_at_unix_ms: 950,
            valid_until_unix_ms: 1_100,
        }
    }

    fn request() -> ResourceAllocationRequest {
        ResourceAllocationRequest {
            topology_sha256: "b".repeat(64),
            now_unix_ms: 1_000,
            minimum_evidence_confidence_ppm: 950_000,
        }
    }

    fn resource(id: &str, capacity: u64, load: u64, reserved: u64, margin: u64) -> CapacityResource {
        CapacityResource {
            resource_id: id.to_owned(),
            declared_capacity_bps: capacity,
            measured_background_load_bps: load,
            committed_reservations_bps: reserved,
            safety_margin_bps: margin,
            evidence: evidence(),
        }
    }

    fn path(id: &str, ceiling: u64, weight: u32, domains: &[&str]) -> ResourcePath {
        ResourcePath {
            path_id: id.to_owned(),
            path_ceiling_bps: ceiling,
            weight,
            capacity_domain_ids: domains.iter().map(|domain| (*domain).to_owned()).collect(),
        }
    }

    #[test]
    fn disjoint_paths_receive_their_independent_resource_capacity() {
        let report = allocate_resource_incidence(
            &request(),
            &[
                path("a", 100, 1, &["access-a"]),
                path("b", 80, 1, &["access-b"]),
            ],
            &[resource("access-a", 100, 0, 0, 0), resource("access-b", 80, 0, 0, 0)],
        )
        .expect("valid independent resources");
        assert_eq!(report.total_allocated_rate_bps, 180);
        assert_eq!(report.path_allocations[0].allocated_rate_bps, 100);
        assert_eq!(report.path_allocations[1].allocated_rate_bps, 80);
    }

    #[test]
    fn shared_backhaul_capacity_is_allocated_once() {
        let report = allocate_resource_incidence(
            &request(),
            &[
                path("a", 100, 1, &["access-a", "backhaul"]),
                path("b", 100, 1, &["access-b", "backhaul"]),
            ],
            &[
                resource("access-a", 100, 0, 0, 0),
                resource("access-b", 100, 0, 0, 0),
                resource("backhaul", 100, 0, 0, 0),
            ],
        )
        .expect("valid shared backhaul");
        assert_eq!(report.total_allocated_rate_bps, 100);
        assert_eq!(report.path_allocations[0].allocated_rate_bps, 50);
        assert_eq!(report.path_allocations[1].allocated_rate_bps, 50);
        let backhaul = report
            .resource_usage
            .iter()
            .find(|item| item.resource_id == "backhaul")
            .expect("backhaul usage");
        assert_eq!(backhaul.allocated_rate_bps, 100);
        assert_eq!(backhaul.unused_capacity_bps, 0);
    }

    #[test]
    fn a_saturated_shared_resource_freezes_its_paths_but_not_independent_paths() {
        let report = allocate_resource_incidence(
            &request(),
            &[
                path("a", 100, 1, &["shared-x"]),
                path("b", 100, 1, &["shared-x"]),
                path("c", 80, 1, &["independent-y"]),
            ],
            &[
                resource("shared-x", 100, 0, 0, 0),
                resource("independent-y", 80, 0, 0, 0),
            ],
        )
        .expect("valid resources");
        assert_eq!(report.total_allocated_rate_bps, 180);
        assert_eq!(report.path_allocations[0].allocated_rate_bps, 50);
        assert_eq!(report.path_allocations[1].allocated_rate_bps, 50);
        assert_eq!(report.path_allocations[2].allocated_rate_bps, 80);
    }

    #[test]
    fn weights_control_shares_on_a_shared_bottleneck() {
        let report = allocate_resource_incidence(
            &request(),
            &[
                path("premium", 100, 2, &["shared"]),
                path("standard", 100, 1, &["shared"]),
            ],
            &[resource("shared", 90, 0, 0, 0)],
        )
        .expect("valid weighted resources");
        assert_eq!(report.total_allocated_rate_bps, 90);
        assert_eq!(report.path_allocations[0].allocated_rate_bps, 60);
        assert_eq!(report.path_allocations[1].allocated_rate_bps, 30);
    }

    #[test]
    fn path_ceiling_saturation_allows_other_paths_to_continue() {
        let report = allocate_resource_incidence(
            &request(),
            &[
                path("small", 10, 1, &["shared"]),
                path("large", 100, 1, &["shared"]),
            ],
            &[resource("shared", 100, 0, 0, 0)],
        )
        .expect("valid path ceilings");
        assert_eq!(report.total_allocated_rate_bps, 100);
        assert_eq!(report.path_allocations[0].allocated_rate_bps, 10);
        assert_eq!(report.path_allocations[1].allocated_rate_bps, 90);
        assert!(report.path_allocations[0].saturated);
        assert!(report.saturated_resource_ids.contains(&"shared".to_owned()));
    }

    #[test]
    fn safety_margin_reservations_and_background_load_reduce_residual_capacity() {
        let report = allocate_resource_incidence(
            &request(),
            &[path("a", 100, 1, &["shared"])],
            &[resource("shared", 100, 20, 10, 5)],
        )
        .expect("valid reservation accounting");
        assert_eq!(report.total_allocated_rate_bps, 65);
        assert_eq!(report.resource_usage[0].residual_capacity_bps, 65);
    }

    #[test]
    fn zero_residual_resource_excludes_only_paths_that_use_it() {
        let report = allocate_resource_incidence(
            &request(),
            &[
                path("blocked", 100, 1, &["zero"]),
                path("free", 40, 1, &["free"]),
            ],
            &[resource("zero", 100, 100, 0, 0), resource("free", 40, 0, 0, 0)],
        )
        .expect("valid resources with a zero residual");
        assert_eq!(report.total_allocated_rate_bps, 40);
        assert_eq!(report.exclusions[0].reason, "capacity_resource_residual_zero");
        assert_eq!(report.path_allocations.len(), 1);
        assert_eq!(report.path_allocations[0].path_id, "free");
    }

    #[test]
    fn invalid_or_stale_resource_evidence_excludes_dependent_paths() {
        let mut stale = resource("stale", 100, 0, 0, 0);
        stale.evidence.valid_until_unix_ms = request().now_unix_ms;
        let report = allocate_resource_incidence(
            &request(),
            &[
                path("stale-path", 100, 1, &["stale"]),
                path("valid-path", 50, 1, &["valid"]),
            ],
            &[stale, resource("valid", 50, 0, 0, 0)],
        )
        .expect("one invalid resource need not block independent paths");
        assert_eq!(report.total_allocated_rate_bps, 50);
        assert_eq!(
            report.exclusions[0].reason,
            "capacity_resource_evidence_invalid"
        );
    }

    #[test]
    fn topology_digest_mismatch_excludes_resource_consumers() {
        let mut mismatched = resource("wrong-topology", 100, 0, 0, 0);
        mismatched.evidence.topology_sha256 = "c".repeat(64);
        let report = allocate_resource_incidence(
            &request(),
            &[path("blocked", 100, 1, &["wrong-topology"])],
            &[mismatched],
        )
        .expect("invalid topology evidence excludes dependent path");
        assert_eq!(report.total_allocated_rate_bps, 0);
        assert_eq!(
            report.exclusions[0].reason,
            "capacity_resource_evidence_invalid"
        );
    }

    #[test]
    fn unknown_resource_reference_excludes_path() {
        let report = allocate_resource_incidence(
            &request(),
            &[path("unknown", 100, 1, &["missing"])],
            &[resource("known", 100, 0, 0, 0)],
        )
        .expect("unknown dependencies exclude path");
        assert_eq!(report.total_allocated_rate_bps, 0);
        assert_eq!(report.exclusions[0].reason, "capacity_resource_unknown");
    }

    #[test]
    fn duplicate_domain_ids_exclude_path_without_duplicating_capacity() {
        let report = allocate_resource_incidence(
            &request(),
            &[path("duplicate", 100, 1, &["shared", "shared"])],
            &[resource("shared", 100, 0, 0, 0)],
        )
        .expect("duplicate path domain is excluded");
        assert_eq!(report.total_allocated_rate_bps, 0);
        assert_eq!(report.exclusions[0].reason, "capacity_domain_id_duplicate");
    }

    #[test]
    fn input_order_permutation_produces_identical_rates_and_reports() {
        let paths = vec![
            path("a", 100, 2, &["shared", "access-a"]),
            path("b", 100, 1, &["shared", "access-b"]),
            path("c", 20, 1, &["independent"]),
        ];
        let resources = vec![
            resource("shared", 90, 0, 0, 0),
            resource("access-a", 100, 0, 0, 0),
            resource("access-b", 100, 0, 0, 0),
            resource("independent", 20, 0, 0, 0),
        ];
        let first = allocate_resource_incidence(&request(), &paths, &resources)
            .expect("valid first order");
        let second = allocate_resource_incidence(
            &request(),
            &paths.into_iter().rev().collect::<Vec<_>>(),
            &resources.into_iter().rev().collect::<Vec<_>>(),
        )
        .expect("valid reversed order");
        assert_eq!(first, second);
    }

    #[test]
    fn duplicate_resource_ids_fail_closed() {
        assert_eq!(
            allocate_resource_incidence(
                &request(),
                &[path("a", 100, 1, &["shared"])],
                &[
                    resource("shared", 100, 0, 0, 0),
                    resource("shared", 100, 0, 0, 0),
                ],
            )
            .unwrap_err(),
            ResourceAllocationError::DuplicateResourceId
        );
    }

    #[test]
    fn resource_load_and_reservations_cannot_exceed_declared_capacity() {
        let report = allocate_resource_incidence(
            &request(),
            &[path("a", 100, 1, &["overcommitted-inventory"])],
            &[resource("overcommitted-inventory", 100, 70, 20, 20)],
        );
        assert_eq!(
            report.unwrap_err(),
            ResourceAllocationError::ResourceAccountingExceedsCapacity
        );
    }

    #[test]
    fn malformed_measurement_window_is_not_allocated() {
        let mut bad = resource("bad-window", 100, 0, 0, 0);
        bad.evidence.measurement_window_end_unix_ms = 1_200;
        let report = allocate_resource_incidence(
            &request(),
            &[path("path", 100, 1, &["bad-window"])],
            &[bad],
        )
        .expect("invalid measurement window excludes dependent path");
        assert_eq!(report.total_allocated_rate_bps, 0);
        assert_eq!(
            report.exclusions[0].reason,
            "capacity_resource_evidence_invalid"
        );
    }

    #[test]
    fn zero_capacity_domain_ids_are_not_synthesized_or_inferred() {
        let report = allocate_resource_incidence(
            &request(),
            &[path("no-domains", 100, 1, &[])],
            &[resource("resource", 100, 0, 0, 0)],
        )
        .expect("missing dependency inventory excludes path");
        assert_eq!(report.total_allocated_rate_bps, 0);
        assert_eq!(report.exclusions[0].reason, "capacity_domains_unknown");
    }
}
