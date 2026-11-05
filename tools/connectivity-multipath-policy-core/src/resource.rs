//! Deterministic weighted max-min allocation over shared capacity domains.
//!
//! This is a pure reference allocator. It does not configure transports, qdiscs,
//! routes, sockets or privileged network state.

use std::collections::{BTreeMap, BTreeSet};

const MAX_PATHS: usize = 256;
const MAX_DOMAINS: usize = 128;
const MAX_DOMAINS_PER_PATH: usize = 32;
const MAX_ID_BYTES: usize = 128;
const MAX_WEIGHT: u32 = 100;
const MAX_OBSERVATION_AGE_MS: u64 = 300_000;
const LEVEL_SCALE: u128 = 1_000_000;
const MAX_ROUNDING_STEPS: usize = MAX_PATHS * MAX_DOMAINS_PER_PATH * MAX_WEIGHT as usize;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourcePathDemand {
    pub path_id: String,
    pub weight: u32,
    pub demand_bps: u64,
    pub minimum_bps: u64,
    pub capacity_domain_ids: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapacityObservation {
    pub domain_id: String,
    pub capacity_bps: u64,
    pub observation_age_ms: u64,
    pub source_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceAllocationRequest {
    pub maximum_observation_age_ms: u64,
    pub paths: Vec<ResourcePathDemand>,
    pub capacities: Vec<CapacityObservation>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PathRateAllocation {
    pub path_id: String,
    pub weight: u32,
    pub demand_bps: u64,
    pub minimum_bps: u64,
    pub allocated_bps: u64,
    pub unmet_demand_bps: u64,
    pub capacity_domain_ids: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DomainRateAllocation {
    pub domain_id: String,
    pub capacity_bps: u64,
    pub allocated_bps: u64,
    pub remaining_bps: u64,
    pub utilization_ppm: u32,
    pub observation_age_ms: u64,
    pub source_sha256: String,
    pub binding: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceAllocationReport {
    pub maximum_observation_age_ms: u64,
    pub total_path_rate_bps: u128,
    pub rounding_adjustments_bps: u64,
    pub binding_domain_ids: Vec<String>,
    pub paths: Vec<PathRateAllocation>,
    pub domains: Vec<DomainRateAllocation>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResourceAllocationError {
    EmptyPaths,
    TooManyPaths,
    EmptyCapacityObservations,
    TooManyCapacityDomains,
    InvalidMaximumObservationAge,
    EmptyPathId,
    PathIdTooLong,
    DuplicatePathId,
    InvalidWeight,
    MinimumExceedsDemand,
    EmptyCapacityDomainSet,
    TooManyCapacityDomainsPerPath,
    BlankCapacityDomainId,
    CapacityDomainIdTooLong,
    DuplicateCapacityDomainId,
    UnknownCapacityDomain(String),
    BlankCapacityObservationId,
    CapacityObservationIdTooLong,
    DuplicateCapacityObservationId,
    ZeroCapacity,
    InvalidSourceDigest(String),
    StaleCapacityObservation(String),
    MinimumFloorsExceedCapacity(String),
    ArithmeticOverflow,
    RoundingWorkLimitExceeded,
    InternalInvariant,
}

#[derive(Clone, Debug)]
struct Path {
    id: String,
    weight: u32,
    demand: u64,
    floor: u64,
    domains: Vec<usize>,
}

#[derive(Clone, Debug)]
struct Domain {
    id: String,
    capacity: u64,
    age_ms: u64,
    source_sha256: String,
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn ceil_div(value: u128, divisor: u128) -> Result<u128, ResourceAllocationError> {
    if divisor == 0 {
        return Err(ResourceAllocationError::InternalInvariant);
    }
    value
        .checked_add(divisor - 1)
        .map(|adjusted| adjusted / divisor)
        .ok_or(ResourceAllocationError::ArithmeticOverflow)
}

fn normalize(
    request: &ResourceAllocationRequest,
) -> Result<(Vec<Path>, Vec<Domain>), ResourceAllocationError> {
    if request.paths.is_empty() {
        return Err(ResourceAllocationError::EmptyPaths);
    }
    if request.paths.len() > MAX_PATHS {
        return Err(ResourceAllocationError::TooManyPaths);
    }
    if request.capacities.is_empty() {
        return Err(ResourceAllocationError::EmptyCapacityObservations);
    }
    if request.capacities.len() > MAX_DOMAINS {
        return Err(ResourceAllocationError::TooManyCapacityDomains);
    }
    if request.maximum_observation_age_ms == 0
        || request.maximum_observation_age_ms > MAX_OBSERVATION_AGE_MS
    {
        return Err(ResourceAllocationError::InvalidMaximumObservationAge);
    }

    let mut observations = request.capacities.clone();
    observations.sort_by(|a, b| a.domain_id.cmp(&b.domain_id));
    let mut domain_indices = BTreeMap::new();
    let mut domains = Vec::with_capacity(observations.len());
    for item in observations {
        if item.domain_id.trim().is_empty() {
            return Err(ResourceAllocationError::BlankCapacityObservationId);
        }
        if item.domain_id.len() > MAX_ID_BYTES {
            return Err(ResourceAllocationError::CapacityObservationIdTooLong);
        }
        if item.capacity_bps == 0 {
            return Err(ResourceAllocationError::ZeroCapacity);
        }
        if item.observation_age_ms > request.maximum_observation_age_ms {
            return Err(ResourceAllocationError::StaleCapacityObservation(
                item.domain_id,
            ));
        }
        if !valid_sha256(&item.source_sha256) {
            return Err(ResourceAllocationError::InvalidSourceDigest(item.domain_id));
        }
        let index = domains.len();
        if domain_indices
            .insert(item.domain_id.clone(), index)
            .is_some()
        {
            return Err(ResourceAllocationError::DuplicateCapacityObservationId);
        }
        domains.push(Domain {
            id: item.domain_id,
            capacity: item.capacity_bps,
            age_ms: item.observation_age_ms,
            source_sha256: item.source_sha256,
        });
    }

    let mut input_paths = request.paths.clone();
    input_paths.sort_by(|a, b| a.path_id.cmp(&b.path_id));
    let mut seen_paths = BTreeSet::new();
    let mut paths = Vec::with_capacity(input_paths.len());

    for mut item in input_paths {
        if item.path_id.trim().is_empty() {
            return Err(ResourceAllocationError::EmptyPathId);
        }
        if item.path_id.len() > MAX_ID_BYTES {
            return Err(ResourceAllocationError::PathIdTooLong);
        }
        if !seen_paths.insert(item.path_id.clone()) {
            return Err(ResourceAllocationError::DuplicatePathId);
        }
        if item.weight == 0 || item.weight > MAX_WEIGHT {
            return Err(ResourceAllocationError::InvalidWeight);
        }
        if item.minimum_bps > item.demand_bps {
            return Err(ResourceAllocationError::MinimumExceedsDemand);
        }
        if item.capacity_domain_ids.is_empty() {
            return Err(ResourceAllocationError::EmptyCapacityDomainSet);
        }
        if item.capacity_domain_ids.len() > MAX_DOMAINS_PER_PATH {
            return Err(ResourceAllocationError::TooManyCapacityDomainsPerPath);
        }
        item.capacity_domain_ids.sort();

        let mut path_domains = Vec::with_capacity(item.capacity_domain_ids.len());
        let mut previous: Option<&str> = None;
        for domain_id in &item.capacity_domain_ids {
            if domain_id.trim().is_empty() {
                return Err(ResourceAllocationError::BlankCapacityDomainId);
            }
            if domain_id.len() > MAX_ID_BYTES {
                return Err(ResourceAllocationError::CapacityDomainIdTooLong);
            }
            if previous == Some(domain_id.as_str()) {
                return Err(ResourceAllocationError::DuplicateCapacityDomainId);
            }
            previous = Some(domain_id);
            let index = domain_indices
                .get(domain_id)
                .copied()
                .ok_or_else(|| ResourceAllocationError::UnknownCapacityDomain(domain_id.clone()))?;
            path_domains.push(index);
        }
        paths.push(Path {
            id: item.path_id,
            weight: item.weight,
            demand: item.demand_bps,
            floor: item.minimum_bps,
            domains: path_domains,
        });
    }
    Ok((paths, domains))
}

fn resource_usage(
    paths: &[Path],
    allocations: &[u64],
    domain_count: usize,
) -> Result<Vec<u128>, ResourceAllocationError> {
    let mut usage = vec![0_u128; domain_count];
    for (path, allocation) in paths.iter().zip(allocations) {
        for index in &path.domains {
            usage[*index] = usage[*index]
                .checked_add(u128::from(*allocation))
                .ok_or(ResourceAllocationError::ArithmeticOverflow)?;
        }
    }
    Ok(usage)
}

fn candidate(
    paths: &[Path],
    allocations: &[u64],
    remainders: &[u128],
    active: &[bool],
    level: u128,
) -> Result<(Vec<u64>, Vec<u128>), ResourceAllocationError> {
    let mut next_rates = allocations.to_vec();
    let mut next_remainders = remainders.to_vec();
    for (index, path) in paths.iter().enumerate() {
        if !active[index] {
            continue;
        }
        let scaled = u128::from(path.weight)
            .checked_mul(level)
            .and_then(|value| value.checked_add(remainders[index]))
            .ok_or(ResourceAllocationError::ArithmeticOverflow)?;
        let increment = scaled / LEVEL_SCALE;
        let gap = u128::from(path.demand.saturating_sub(allocations[index]));
        let applied = increment.min(gap);
        next_rates[index] = allocations[index]
            .checked_add(
                u64::try_from(applied).map_err(|_| ResourceAllocationError::ArithmeticOverflow)?,
            )
            .ok_or(ResourceAllocationError::ArithmeticOverflow)?;
        next_remainders[index] = if applied == gap {
            0
        } else {
            scaled % LEVEL_SCALE
        };
    }
    Ok((next_rates, next_remainders))
}

fn feasible(
    paths: &[Path],
    rates: &[u64],
    domains: &[Domain],
) -> Result<bool, ResourceAllocationError> {
    let usage = resource_usage(paths, rates, domains.len())?;
    Ok(usage
        .iter()
        .zip(domains)
        .all(|(used, domain)| *used <= u128::from(domain.capacity)))
}

fn less_normalized_service(left: usize, right: usize, paths: &[Path], allocations: &[u64]) -> bool {
    let left_extra = u128::from(allocations[left].saturating_sub(paths[left].floor));
    let right_extra = u128::from(allocations[right].saturating_sub(paths[right].floor));
    let left_scaled = left_extra * u128::from(paths[right].weight);
    let right_scaled = right_extra * u128::from(paths[left].weight);
    left_scaled < right_scaled || (left_scaled == right_scaled && paths[left].id < paths[right].id)
}

fn progressive_fill(
    paths: &[Path],
    domains: &[Domain],
) -> Result<(Vec<u64>, BTreeSet<usize>, u64), ResourceAllocationError> {
    let mut allocations = paths.iter().map(|path| path.floor).collect::<Vec<_>>();
    let mut remainders = vec![0_u128; paths.len()];
    let floor_usage = resource_usage(paths, &allocations, domains.len())?;
    for (index, used) in floor_usage.iter().enumerate() {
        if *used > u128::from(domains[index].capacity) {
            return Err(ResourceAllocationError::MinimumFloorsExceedCapacity(
                domains[index].id.clone(),
            ));
        }
    }

    let mut active = paths
        .iter()
        .map(|path| path.floor < path.demand)
        .collect::<Vec<_>>();
    let mut binding = BTreeSet::new();
    let mut frozen = 0_usize;

    while active.iter().any(|value| *value) {
        let max_level = paths
            .iter()
            .enumerate()
            .filter(|(index, _)| active[*index])
            .map(|(index, path)| {
                let gap = u128::from(path.demand.saturating_sub(allocations[index]));
                let remaining = gap
                    .checked_mul(LEVEL_SCALE)
                    .ok_or(ResourceAllocationError::ArithmeticOverflow)?
                    .saturating_sub(remainders[index]);
                ceil_div(remaining, u128::from(path.weight))
            })
            .collect::<Result<Vec<_>, ResourceAllocationError>>()?
            .into_iter()
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or(ResourceAllocationError::ArithmeticOverflow)?;

        let (all_demand_rates, _) =
            candidate(paths, &allocations, &remainders, &active, max_level)?;
        if feasible(paths, &all_demand_rates, domains)? {
            allocations = all_demand_rates;
            break;
        }

        let mut low = 0_u128;
        let mut high = max_level;
        while high - low > 1 {
            let middle = low + (high - low) / 2;
            let (rates, _) = candidate(paths, &allocations, &remainders, &active, middle)?;
            if feasible(paths, &rates, domains)? {
                low = middle;
            } else {
                high = middle;
            }
        }

        let (next_rates, next_remainders) =
            candidate(paths, &allocations, &remainders, &active, low)?;
        allocations = next_rates;
        remainders = next_remainders;

        // The immediately following micro-bps level is the first infeasible
        // level because binary search leaves high == low + 1.
        let (after_rates, _) = candidate(paths, &allocations, &remainders, &active, high - low)?;
        let after_usage = resource_usage(paths, &after_rates, domains.len())?;
        let mut newly_binding = BTreeSet::new();
        for (index, used) in after_usage.iter().enumerate() {
            if *used > u128::from(domains[index].capacity) {
                newly_binding.insert(index);
            }
        }
        if newly_binding.is_empty() {
            return Err(ResourceAllocationError::InternalInvariant);
        }

        let mut froze_any = false;
        for domain_index in &newly_binding {
            binding.insert(*domain_index);
        }
        for (index, path) in paths.iter().enumerate() {
            if !active[index] {
                continue;
            }
            let at_demand = allocations[index] >= path.demand;
            let touches_blocker = path
                .domains
                .iter()
                .any(|domain_index| newly_binding.contains(domain_index));
            if at_demand || touches_blocker {
                active[index] = false;
                frozen += 1;
                froze_any = true;
            }
        }
        if !froze_any || frozen > paths.len() {
            return Err(ResourceAllocationError::InternalInvariant);
        }
    }

    // Deterministic integer remainder assignment. Each extra bit is given to
    // the least-served normalized path whose entire dependency set has room.
    let mut rounding_steps = 0_u64;
    loop {
        let usage = resource_usage(paths, &allocations, domains.len())?;
        let mut best = None;
        for (index, path) in paths.iter().enumerate() {
            if allocations[index] >= path.demand
                || !path.domains.iter().any(|domain| binding.contains(domain))
            {
                continue;
            }
            if !path
                .domains
                .iter()
                .all(|domain| usage[*domain] < u128::from(domains[*domain].capacity))
            {
                continue;
            }
            if best.is_none_or(|previous| {
                less_normalized_service(index, previous, paths, &allocations)
            }) {
                best = Some(index);
            }
        }

        let Some(index) = best else {
            break;
        };
        allocations[index] = allocations[index]
            .checked_add(1)
            .ok_or(ResourceAllocationError::ArithmeticOverflow)?;
        rounding_steps = rounding_steps
            .checked_add(1)
            .ok_or(ResourceAllocationError::ArithmeticOverflow)?;
        if rounding_steps > MAX_ROUNDING_STEPS as u64 {
            return Err(ResourceAllocationError::RoundingWorkLimitExceeded);
        }
    }

    let final_usage = resource_usage(paths, &allocations, domains.len())?;
    for (index, used) in final_usage.iter().enumerate() {
        if *used > u128::from(domains[index].capacity) {
            return Err(ResourceAllocationError::InternalInvariant);
        }
        if *used == u128::from(domains[index].capacity) {
            binding.insert(index);
        }
    }
    Ok((allocations, binding, rounding_steps))
}

/// Allocate path rates subject to every declared capacity-domain ceiling.
/// All rates are integer bits/second; stale or missing capacity evidence fails closed.
pub fn allocate_resource_incidence_weighted_max_min(
    request: &ResourceAllocationRequest,
) -> Result<ResourceAllocationReport, ResourceAllocationError> {
    let (paths, domains) = normalize(request)?;
    let (allocations, binding, rounding_steps) = progressive_fill(&paths, &domains)?;
    let usage = resource_usage(&paths, &allocations, domains.len())?;

    let path_reports = paths
        .iter()
        .zip(&allocations)
        .map(|(path, rate)| PathRateAllocation {
            path_id: path.id.clone(),
            weight: path.weight,
            demand_bps: path.demand,
            minimum_bps: path.floor,
            allocated_bps: *rate,
            unmet_demand_bps: path.demand - *rate,
            capacity_domain_ids: path
                .domains
                .iter()
                .map(|index| domains[*index].id.clone())
                .collect(),
        })
        .collect::<Vec<_>>();

    let domain_reports = domains
        .iter()
        .zip(&usage)
        .enumerate()
        .map(|(index, (domain, used))| {
            let used =
                u64::try_from(*used).map_err(|_| ResourceAllocationError::ArithmeticOverflow)?;
            Ok(DomainRateAllocation {
                domain_id: domain.id.clone(),
                capacity_bps: domain.capacity,
                allocated_bps: used,
                remaining_bps: domain.capacity - used,
                utilization_ppm: u32::try_from(
                    (u128::from(used) * LEVEL_SCALE) / u128::from(domain.capacity),
                )
                .map_err(|_| ResourceAllocationError::ArithmeticOverflow)?,
                observation_age_ms: domain.age_ms,
                source_sha256: domain.source_sha256.clone(),
                binding: binding.contains(&index),
            })
        })
        .collect::<Result<Vec<_>, ResourceAllocationError>>()?;

    let total_path_rate_bps = allocations.iter().try_fold(0_u128, |sum, rate| {
        sum.checked_add(u128::from(*rate))
            .ok_or(ResourceAllocationError::ArithmeticOverflow)
    })?;
    let binding_domain_ids = binding
        .iter()
        .map(|index| domains[*index].id.clone())
        .collect::<Vec<_>>();

    Ok(ResourceAllocationReport {
        maximum_observation_age_ms: request.maximum_observation_age_ms,
        total_path_rate_bps,
        rounding_adjustments_bps: rounding_steps,
        binding_domain_ids,
        paths: path_reports,
        domains: domain_reports,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(byte: char) -> String {
        std::iter::repeat_n(byte, 64).collect()
    }

    fn domain(id: &str, capacity: u64) -> CapacityObservation {
        CapacityObservation {
            domain_id: id.to_owned(),
            capacity_bps: capacity,
            observation_age_ms: 100,
            source_sha256: digest('a'),
        }
    }

    fn path(
        id: &str,
        weight: u32,
        demand: u64,
        floor: u64,
        domain_ids: &[&str],
    ) -> ResourcePathDemand {
        ResourcePathDemand {
            path_id: id.to_owned(),
            weight,
            demand_bps: demand,
            minimum_bps: floor,
            capacity_domain_ids: domain_ids.iter().map(|value| (*value).to_owned()).collect(),
        }
    }

    fn request(
        paths: Vec<ResourcePathDemand>,
        capacities: Vec<CapacityObservation>,
    ) -> ResourceAllocationRequest {
        ResourceAllocationRequest {
            maximum_observation_age_ms: 1_000,
            paths,
            capacities,
        }
    }

    fn rate(report: &ResourceAllocationReport, id: &str) -> u64 {
        report
            .paths
            .iter()
            .find(|path| path.path_id == id)
            .expect("path report")
            .allocated_bps
    }

    fn remaining(report: &ResourceAllocationReport, id: &str) -> u64 {
        report
            .domains
            .iter()
            .find(|domain| domain.domain_id == id)
            .expect("domain report")
            .remaining_bps
    }

    #[test]
    fn independent_paths_reach_their_demand_ceilings() {
        let result = allocate_resource_incidence_weighted_max_min(&request(
            vec![path("a", 1, 40, 0, &["x"]), path("b", 1, 60, 0, &["y"])],
            vec![domain("x", 50), domain("y", 70)],
        ))
        .expect("independent domains");
        assert_eq!(rate(&result, "a"), 40);
        assert_eq!(rate(&result, "b"), 60);
        assert_eq!(result.total_path_rate_bps, 100);
    }

    #[test]
    fn two_access_paths_share_one_backhaul_ceiling() {
        let result = allocate_resource_incidence_weighted_max_min(&request(
            vec![
                path("a", 1, 100, 0, &["access-a", "backhaul-x"]),
                path("b", 1, 100, 0, &["access-b", "backhaul-x"]),
            ],
            vec![
                domain("access-a", 100),
                domain("access-b", 100),
                domain("backhaul-x", 70),
            ],
        ))
        .expect("shared backhaul");
        assert_eq!(rate(&result, "a"), 35);
        assert_eq!(rate(&result, "b"), 35);
        assert_eq!(remaining(&result, "backhaul-x"), 0);
    }

    #[test]
    fn domain_utilization_is_reported_in_parts_per_million() {
        let result = allocate_resource_incidence_weighted_max_min(&request(
            vec![path("a", 1, 40, 0, &["x"])],
            vec![domain("x", 100)],
        ))
        .expect("valid resource allocation");
        assert_eq!(result.domains[0].allocated_bps, 40);
        assert_eq!(result.domains[0].utilization_ppm, 400_000);
    }

    #[test]
    fn independent_path_keeps_filling_while_shared_pair_is_capped() {
        let result = allocate_resource_incidence_weighted_max_min(&request(
            vec![
                path("a", 1, 100, 0, &["access-a", "backhaul-x"]),
                path("b", 1, 100, 0, &["access-b", "backhaul-x"]),
                path("c", 1, 50, 0, &["access-c", "backhaul-y"]),
            ],
            vec![
                domain("access-a", 100),
                domain("access-b", 100),
                domain("access-c", 100),
                domain("backhaul-x", 70),
                domain("backhaul-y", 50),
            ],
        ))
        .expect("overlap plus independent path");
        assert_eq!(rate(&result, "a"), 35);
        assert_eq!(rate(&result, "b"), 35);
        assert_eq!(rate(&result, "c"), 50);
        assert_eq!(result.total_path_rate_bps, 120);
    }

    #[test]
    fn overlap_chain_freezes_only_paths_using_each_saturated_domain() {
        let result = allocate_resource_incidence_weighted_max_min(&request(
            vec![
                path("a", 1, 100, 0, &["x"]),
                path("b", 1, 100, 0, &["x", "y"]),
                path("c", 1, 100, 0, &["y"]),
            ],
            vec![domain("x", 60), domain("y", 100)],
        ))
        .expect("overlap chain");
        assert_eq!(rate(&result, "a"), 30);
        assert_eq!(rate(&result, "b"), 30);
        assert_eq!(rate(&result, "c"), 70);
        assert_eq!(remaining(&result, "x"), 0);
        assert_eq!(remaining(&result, "y"), 0);
    }

    #[test]
    fn demand_limited_path_releases_capacity_to_the_other_path() {
        let result = allocate_resource_incidence_weighted_max_min(&request(
            vec![path("a", 1, 20, 0, &["x"]), path("b", 1, 100, 0, &["x"])],
            vec![domain("x", 100)],
        ))
        .expect("demand limited");
        assert_eq!(rate(&result, "a"), 20);
        assert_eq!(rate(&result, "b"), 80);
    }

    #[test]
    fn weighted_one_two_four_shares_are_exact() {
        let result = allocate_resource_incidence_weighted_max_min(&request(
            vec![
                path("a", 1, 100, 0, &["x"]),
                path("b", 2, 100, 0, &["x"]),
                path("c", 4, 100, 0, &["x"]),
            ],
            vec![domain("x", 70)],
        ))
        .expect("weighted sharing");
        assert_eq!(rate(&result, "a"), 10);
        assert_eq!(rate(&result, "b"), 20);
        assert_eq!(rate(&result, "c"), 40);
    }

    #[test]
    fn feasible_service_floors_are_preserved() {
        let result = allocate_resource_incidence_weighted_max_min(&request(
            vec![path("a", 1, 100, 30, &["x"]), path("b", 1, 100, 20, &["x"])],
            vec![domain("x", 100)],
        ))
        .expect("floors fit");
        assert_eq!(rate(&result, "a"), 55);
        assert_eq!(rate(&result, "b"), 45);
    }

    #[test]
    fn infeasible_floors_fail_closed() {
        assert_eq!(
            allocate_resource_incidence_weighted_max_min(&request(
                vec![path("a", 1, 100, 60, &["x"]), path("b", 1, 100, 60, &["x"])],
                vec![domain("x", 100)],
            )),
            Err(ResourceAllocationError::MinimumFloorsExceedCapacity(
                "x".to_owned()
            ))
        );
    }

    #[test]
    fn stale_capacity_observation_is_rejected() {
        let mut stale = domain("x", 100);
        stale.observation_age_ms = 1_001;
        assert_eq!(
            allocate_resource_incidence_weighted_max_min(&request(
                vec![path("a", 1, 100, 0, &["x"])],
                vec![stale],
            )),
            Err(ResourceAllocationError::StaleCapacityObservation(
                "x".to_owned()
            ))
        );
    }

    #[test]
    fn malformed_source_digest_is_rejected() {
        let mut invalid = domain("x", 100);
        invalid.source_sha256 = "bad".to_owned();
        assert_eq!(
            allocate_resource_incidence_weighted_max_min(&request(
                vec![path("a", 1, 100, 0, &["x"])],
                vec![invalid],
            )),
            Err(ResourceAllocationError::InvalidSourceDigest("x".to_owned()))
        );
    }

    #[test]
    fn unknown_domain_observation_is_rejected() {
        assert_eq!(
            allocate_resource_incidence_weighted_max_min(&request(
                vec![path("a", 1, 100, 0, &["missing"])],
                vec![domain("x", 100)],
            )),
            Err(ResourceAllocationError::UnknownCapacityDomain(
                "missing".to_owned()
            ))
        );
    }

    #[test]
    fn duplicate_domain_in_a_path_is_rejected() {
        assert_eq!(
            allocate_resource_incidence_weighted_max_min(&request(
                vec![path("a", 1, 100, 0, &["x", "x"])],
                vec![domain("x", 100)],
            )),
            Err(ResourceAllocationError::DuplicateCapacityDomainId)
        );
    }

    #[test]
    fn duplicate_path_id_is_rejected() {
        assert_eq!(
            allocate_resource_incidence_weighted_max_min(&request(
                vec![path("a", 1, 100, 0, &["x"]), path("a", 1, 100, 0, &["x"])],
                vec![domain("x", 100)],
            )),
            Err(ResourceAllocationError::DuplicatePathId)
        );
    }

    #[test]
    fn input_order_does_not_change_the_report() {
        let paths = vec![
            path("c", 4, 100, 0, &["y", "x"]),
            path("a", 1, 100, 0, &["x"]),
            path("b", 2, 100, 0, &["x", "y"]),
        ];
        let capacities = vec![domain("y", 80), domain("x", 70)];
        let first = allocate_resource_incidence_weighted_max_min(&request(
            paths.clone(),
            capacities.clone(),
        ))
        .expect("first order");
        let second = allocate_resource_incidence_weighted_max_min(&request(
            paths.into_iter().rev().collect(),
            capacities.into_iter().rev().collect(),
        ))
        .expect("reverse order");
        assert_eq!(first, second);
    }

    #[test]
    fn one_bit_residual_is_allocated_deterministically() {
        let result = allocate_resource_incidence_weighted_max_min(&request(
            vec![path("a", 1, 100, 0, &["x"]), path("b", 1, 100, 0, &["x"])],
            vec![domain("x", 101)],
        ))
        .expect("integer remainder");
        assert_eq!(rate(&result, "a"), 51);
        assert_eq!(rate(&result, "b"), 50);
        assert_eq!(remaining(&result, "x"), 0);
        assert_eq!(result.rounding_adjustments_bps, 1);
    }

    fn weighted_service_score(rates: &[u64], paths: &[ResourcePathDemand]) -> Vec<(u64, u32)> {
        let mut score = paths
            .iter()
            .zip(rates)
            .map(|(path, rate)| (rate.saturating_sub(path.minimum_bps), path.weight))
            .collect::<Vec<_>>();
        score.sort_by(|left, right| {
            (u128::from(left.0) * u128::from(right.1))
                .cmp(&(u128::from(right.0) * u128::from(left.1)))
        });
        score
    }

    fn compare_weighted_scores(left: &[(u64, u32)], right: &[(u64, u32)]) -> std::cmp::Ordering {
        for (left_item, right_item) in left.iter().zip(right) {
            let ordering = (u128::from(left_item.0) * u128::from(right_item.1))
                .cmp(&(u128::from(right_item.0) * u128::from(left_item.1)));
            if ordering != std::cmp::Ordering::Equal {
                return ordering;
            }
        }
        left.len().cmp(&right.len())
    }

    #[test]
    fn weighted_progressive_fill_matches_exhaustive_shared_resource_oracle() {
        let paths = vec![path("a", 1, 5, 0, &["x"]), path("b", 2, 5, 0, &["x"])];
        let result = allocate_resource_incidence_weighted_max_min(&request(
            paths.clone(),
            vec![domain("x", 5)],
        ))
        .expect("weighted resource allocation");
        let actual = result
            .paths
            .iter()
            .map(|path| path.allocated_bps)
            .collect::<Vec<_>>();

        let mut best: Option<Vec<(u64, u32)>> = None;
        for a in 0..=5_u64 {
            for b in 0..=5_u64 {
                if a + b > 5 {
                    continue;
                }
                let score = weighted_service_score(&[a, b], &paths);
                if best.as_ref().is_none_or(|previous| {
                    compare_weighted_scores(&score, previous) == std::cmp::Ordering::Greater
                }) {
                    best = Some(score);
                }
            }
        }
        assert_eq!(
            weighted_service_score(&actual, &paths),
            best.expect("at least one feasible allocation")
        );
    }

    #[test]
    fn progressive_fill_matches_exhaustive_two_domain_overlap_oracle() {
        let paths = vec![
            path("a", 1, 4, 0, &["x"]),
            path("b", 1, 4, 0, &["x", "y"]),
            path("c", 1, 4, 0, &["y"]),
        ];
        let result = allocate_resource_incidence_weighted_max_min(&request(
            paths.clone(),
            vec![domain("x", 3), domain("y", 5)],
        ))
        .expect("overlapping resource allocation");
        let actual = result
            .paths
            .iter()
            .map(|path| path.allocated_bps)
            .collect::<Vec<_>>();

        let mut best: Option<Vec<(u64, u32)>> = None;
        for a in 0..=4_u64 {
            for b in 0..=4_u64 {
                for c in 0..=4_u64 {
                    if a + b > 3 || b + c > 5 {
                        continue;
                    }
                    let score = weighted_service_score(&[a, b, c], &paths);
                    if best.as_ref().is_none_or(|previous| {
                        compare_weighted_scores(&score, previous) == std::cmp::Ordering::Greater
                    }) {
                        best = Some(score);
                    }
                }
            }
        }
        assert_eq!(
            weighted_service_score(&actual, &paths),
            best.expect("at least one feasible allocation")
        );
    }

    #[test]
    fn distinct_evidence_sources_are_preserved_per_domain() {
        let x = domain("x", 100);
        let mut y = domain("y", 100);
        y.source_sha256 = digest('b');
        let result = allocate_resource_incidence_weighted_max_min(&request(
            vec![path("a", 1, 100, 0, &["x"]), path("b", 1, 100, 0, &["y"])],
            vec![y, x],
        ))
        .expect("per-domain provenance");
        assert_eq!(result.domains[0].source_sha256, digest('a'));
        assert_eq!(result.domains[1].source_sha256, digest('b'));
    }
}
