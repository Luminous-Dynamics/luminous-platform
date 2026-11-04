//! Shared-resource weighted max-min allocation over path/resource incidence.
//!
//! This is the next-step model beyond conservative disjoint-path selection.
//! Each candidate path consumes every resource in its dependency set. All paths
//! that traverse a resource share that resource's capacity ceiling. The allocator
//! computes a deterministic weighted max-min feasible rate vector by progressive
//! filling (binary search over a fixed-point water level), then applies a bounded
//! largest-ID-order integer remainder pass.
//!
//! It is a pure planning model. It does not authenticate resource inventories,
//! discover topology, schedule packets, or actuate a router/kernel. The caller
//! must supply already-qualified resource capacities and complete path incidence.

use std::collections::{BTreeMap, BTreeSet};

const MAX_PATHS: usize = 64;
const MAX_RESOURCES: usize = 128;
const MAX_RESOURCES_PER_PATH: usize = 32;
const MAX_ID_BYTES: usize = 128;
const MAX_WEIGHT: u32 = 1_000_000;
const LEVEL_SCALE: u128 = 1_000_000_000_000;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceCapacity {
    pub resource_id: String,
    pub capacity_bps: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PathRateDemand {
    pub path_id: String,
    /// Every hard capacity bottleneck consumed by this path.
    pub resource_ids: Vec<String>,
    /// Positive integer relative weight in the progressive-filling objective.
    pub weight: u32,
    /// Offered demand ceiling for this path, in bits per second.
    pub demand_bps: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PathRateAllocation {
    pub path_id: String,
    pub resource_ids: Vec<String>,
    pub weight: u32,
    pub demand_bps: u64,
    pub allocated_bps: u64,
    pub backlogged: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceUsage {
    pub resource_id: String,
    pub capacity_bps: u64,
    pub allocated_bps: u64,
    pub remaining_capacity_bps: u64,
    pub saturated: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResourceAllocationReport {
    pub path_allocations: Vec<PathRateAllocation>,
    pub resource_usage: Vec<ResourceUsage>,
    pub total_requested_bps: u128,
    pub total_allocated_bps: u128,
    pub unserved_requested_bps: u128,
    /// Weighted Jain index across backlogged paths. Undefined for fewer than two.
    pub backlogged_weighted_jain_index_ppm: Option<u32>,
    /// Resource incidence allocations are max-min fair, not necessarily sum-rate optimal.
    pub objective: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResourceAllocationError {
    EmptyPaths,
    EmptyResources,
    TooManyPaths,
    TooManyResources,
    EmptyPathId,
    EmptyResourceId,
    IdTooLong,
    IdContainsControlCharacter,
    DuplicatePathId,
    DuplicateResourceId,
    EmptyResourceSet,
    TooManyResourcesPerPath,
    DuplicateResourceInPath,
    UnknownResource,
    ZeroWeight,
    WeightTooLarge,
    ZeroDemand,
    ZeroResourceCapacity,
    ArithmeticOverflow,
    InternalInvariant,
}

fn valid_id(id: &str) -> Result<(), ResourceAllocationError> {
    if id.trim().is_empty() {
        return Err(ResourceAllocationError::EmptyResourceId);
    }
    if id.len() > MAX_ID_BYTES {
        return Err(ResourceAllocationError::IdTooLong);
    }
    if id.chars().any(char::is_control) {
        return Err(ResourceAllocationError::IdContainsControlCharacter);
    }
    Ok(())
}

fn validate_inputs(
    resources: &[ResourceCapacity],
    paths: &[PathRateDemand],
) -> Result<(), ResourceAllocationError> {
    if paths.is_empty() {
        return Err(ResourceAllocationError::EmptyPaths);
    }
    if resources.is_empty() {
        return Err(ResourceAllocationError::EmptyResources);
    }
    if paths.len() > MAX_PATHS {
        return Err(ResourceAllocationError::TooManyPaths);
    }
    if resources.len() > MAX_RESOURCES {
        return Err(ResourceAllocationError::TooManyResources);
    }

    let mut resource_ids = BTreeSet::new();
    for resource in resources {
        valid_id(&resource.resource_id)?;
        if !resource_ids.insert(resource.resource_id.as_str()) {
            return Err(ResourceAllocationError::DuplicateResourceId);
        }
        if resource.capacity_bps == 0 {
            return Err(ResourceAllocationError::ZeroResourceCapacity);
        }
    }

    let mut path_ids = BTreeSet::new();
    for path in paths {
        if path.path_id.trim().is_empty() {
            return Err(ResourceAllocationError::EmptyPathId);
        }
        if path.path_id.len() > MAX_ID_BYTES {
            return Err(ResourceAllocationError::IdTooLong);
        }
        if path.path_id.chars().any(char::is_control) {
            return Err(ResourceAllocationError::IdContainsControlCharacter);
        }
        if !path_ids.insert(path.path_id.as_str()) {
            return Err(ResourceAllocationError::DuplicatePathId);
        }
        if path.weight == 0 {
            return Err(ResourceAllocationError::ZeroWeight);
        }
        if path.weight > MAX_WEIGHT {
            return Err(ResourceAllocationError::WeightTooLarge);
        }
        if path.demand_bps == 0 {
            return Err(ResourceAllocationError::ZeroDemand);
        }
        if path.resource_ids.is_empty() {
            return Err(ResourceAllocationError::EmptyResourceSet);
        }
        if path.resource_ids.len() > MAX_RESOURCES_PER_PATH {
            return Err(ResourceAllocationError::TooManyResourcesPerPath);
        }

        let mut seen = BTreeSet::new();
        for resource_id in &path.resource_ids {
            valid_id(resource_id)?;
            if !seen.insert(resource_id.as_str()) {
                return Err(ResourceAllocationError::DuplicateResourceInPath);
            }
            if !resource_ids.contains(resource_id.as_str()) {
                return Err(ResourceAllocationError::UnknownResource);
            }
        }
    }

    Ok(())
}

fn resource_loads_scaled(
    allocations_scaled: &[u128],
    path_resource_indices: &[Vec<usize>],
    resource_count: usize,
) -> Result<Vec<u128>, ResourceAllocationError> {
    if allocations_scaled.len() != path_resource_indices.len() {
        return Err(ResourceAllocationError::InternalInvariant);
    }
    let mut loads = vec![0_u128; resource_count];
    for (allocation, indices) in allocations_scaled.iter().zip(path_resource_indices) {
        for index in indices {
            let load = loads
                .get_mut(*index)
                .ok_or(ResourceAllocationError::InternalInvariant)?;
            *load = load
                .checked_add(*allocation)
                .ok_or(ResourceAllocationError::ArithmeticOverflow)?;
        }
    }
    Ok(loads)
}

fn resource_loads(
    allocations: &[u64],
    path_resource_indices: &[Vec<usize>],
    resource_count: usize,
) -> Result<Vec<u128>, ResourceAllocationError> {
    if allocations.len() != path_resource_indices.len() {
        return Err(ResourceAllocationError::InternalInvariant);
    }
    let mut loads = vec![0_u128; resource_count];
    for (allocation, indices) in allocations.iter().zip(path_resource_indices) {
        for index in indices {
            let load = loads
                .get_mut(*index)
                .ok_or(ResourceAllocationError::InternalInvariant)?;
            *load = load
                .checked_add(u128::from(*allocation))
                .ok_or(ResourceAllocationError::ArithmeticOverflow)?;
        }
    }
    Ok(loads)
}

fn progressive_fill(
    paths: &[&PathRateDemand],
    path_resource_indices: &[Vec<usize>],
    resource_capacities: &[u64],
) -> Result<Vec<u64>, ResourceAllocationError> {
    let resource_capacities_scaled = resource_capacities
        .iter()
        .map(|capacity| {
            u128::from(*capacity)
                .checked_mul(LEVEL_SCALE)
                .ok_or(ResourceAllocationError::ArithmeticOverflow)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let demand_limits_scaled = paths
        .iter()
        .map(|path| {
            u128::from(path.demand_bps)
                .checked_mul(LEVEL_SCALE)
                .ok_or(ResourceAllocationError::ArithmeticOverflow)
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut allocations_scaled = vec![0_u128; paths.len()];
    let mut active = (0..paths.len()).collect::<Vec<_>>();
    let mut iterations = 0_usize;

    while !active.is_empty() {
        iterations += 1;
        if iterations > paths.len() + 1 {
            return Err(ResourceAllocationError::InternalInvariant);
        }

        let loads_scaled = resource_loads_scaled(
            &allocations_scaled,
            path_resource_indices,
            resource_capacities.len(),
        )?;

        // Freeze paths already at their demand, and paths that consume a
        // resource whose remaining capacity is below one fixed-point step for
        // all active consumers of that resource.
        let mut active_weight_by_resource = vec![0_u128; resource_capacities.len()];
        for path_index in &active {
            for resource_index in &path_resource_indices[*path_index] {
                active_weight_by_resource[*resource_index] = active_weight_by_resource
                    [*resource_index]
                    .checked_add(u128::from(paths[*path_index].weight))
                    .ok_or(ResourceAllocationError::ArithmeticOverflow)?;
            }
        }
        let mut blocked_resources = BTreeSet::new();
        for resource_index in 0..resource_capacities.len() {
            let remaining = resource_capacities_scaled[resource_index]
                .checked_sub(loads_scaled[resource_index])
                .ok_or(ResourceAllocationError::InternalInvariant)?;
            let active_weight = active_weight_by_resource[resource_index];
            if active_weight > 0 && remaining < active_weight {
                blocked_resources.insert(resource_index);
            }
        }

        let mut newly_frozen = BTreeSet::new();
        for path_index in &active {
            let remaining_demand = demand_limits_scaled[*path_index]
                .checked_sub(allocations_scaled[*path_index])
                .ok_or(ResourceAllocationError::InternalInvariant)?;
            if remaining_demand < u128::from(paths[*path_index].weight)
                || path_resource_indices[*path_index]
                    .iter()
                    .any(|resource_index| blocked_resources.contains(resource_index))
            {
                newly_frozen.insert(*path_index);
            }
        }
        if !newly_frozen.is_empty() {
            active.retain(|index| !newly_frozen.contains(index));
            continue;
        }

        // Increase each active path in proportion to its weight until the
        // next path demand or shared resource reaches its next event.
        let mut delta_level: Option<u128> = None;
        for path_index in &active {
            let remaining_demand = demand_limits_scaled[*path_index]
                .checked_sub(allocations_scaled[*path_index])
                .ok_or(ResourceAllocationError::InternalInvariant)?;
            let path_delta = remaining_demand / u128::from(paths[*path_index].weight);
            delta_level = Some(delta_level.map_or(path_delta, |current| current.min(path_delta)));
        }
        for resource_index in 0..resource_capacities.len() {
            let active_weight = active_weight_by_resource[resource_index];
            if active_weight == 0 {
                continue;
            }
            let remaining = resource_capacities_scaled[resource_index]
                .checked_sub(loads_scaled[resource_index])
                .ok_or(ResourceAllocationError::InternalInvariant)?;
            let resource_delta = remaining / active_weight;
            delta_level =
                Some(delta_level.map_or(resource_delta, |current| current.min(resource_delta)));
        }
        let delta = delta_level.ok_or(ResourceAllocationError::InternalInvariant)?;
        if delta == 0 {
            return Err(ResourceAllocationError::InternalInvariant);
        }

        for path_index in &active {
            let increment = u128::from(paths[*path_index].weight)
                .checked_mul(delta)
                .ok_or(ResourceAllocationError::ArithmeticOverflow)?;
            allocations_scaled[*path_index] = allocations_scaled[*path_index]
                .checked_add(increment)
                .ok_or(ResourceAllocationError::ArithmeticOverflow)?;
            if allocations_scaled[*path_index] > demand_limits_scaled[*path_index] {
                return Err(ResourceAllocationError::InternalInvariant);
            }
        }

        let updated_loads = resource_loads_scaled(
            &allocations_scaled,
            path_resource_indices,
            resource_capacities.len(),
        )?;
        let mut frozen_after_event = BTreeSet::new();
        for path_index in &active {
            let at_demand = demand_limits_scaled[*path_index]
                .saturating_sub(allocations_scaled[*path_index])
                < u128::from(paths[*path_index].weight);
            let uses_saturated_resource =
                path_resource_indices[*path_index]
                    .iter()
                    .any(|resource_index| {
                        let remaining = resource_capacities_scaled[*resource_index]
                            .saturating_sub(updated_loads[*resource_index]);
                        remaining < active_weight_by_resource[*resource_index]
                    });
            if at_demand || uses_saturated_resource {
                frozen_after_event.insert(*path_index);
            }
        }
        if frozen_after_event.is_empty() {
            return Err(ResourceAllocationError::InternalInvariant);
        }
        active.retain(|index| !frozen_after_event.contains(index));
    }

    allocations_scaled
        .iter()
        .zip(paths)
        .map(|(allocation, path)| {
            let rounded_down = allocation / LEVEL_SCALE;
            u64::try_from(rounded_down.min(u128::from(path.demand_bps)))
                .map_err(|_| ResourceAllocationError::ArithmeticOverflow)
        })
        .collect()
}

fn jain_index_ppm(normalized_rates: &[f64]) -> Option<u32> {
    if normalized_rates.len() < 2 {
        return None;
    }
    let sum = normalized_rates.iter().sum::<f64>();
    let sum_squares = normalized_rates.iter().map(|rate| rate * rate).sum::<f64>();
    if sum_squares == 0.0 {
        return Some(1_000_000);
    }
    let index = (sum * sum) / (normalized_rates.len() as f64 * sum_squares);
    Some((index.clamp(0.0, 1.0) * 1_000_000.0).round() as u32)
}

/// Allocate path rates under all shared resource ceilings.
///
/// Weighted progressive filling increases each backlogged path in proportion
/// to its weight until a resource saturates or a path reaches its demand. Every
/// backlogged path using a saturated resource freezes; paths not using that
/// resource continue to increase. The fixed-point search resolves the water
/// level deterministically to 1e-6 bits/s, then assigns at most one residual
/// bit/s per path in sorted path-ID order.
///
/// The result is weighted max-min fair over path rates. It is deliberately not
/// described as maximum-sum throughput: max-min fairness can leave a different
/// feasible allocation with a higher sum rate. That utilization trade-off is
/// measured explicitly by the report and benchmark profile.
pub fn allocate_weighted_max_min(
    resources: &[ResourceCapacity],
    paths: &[PathRateDemand],
) -> Result<ResourceAllocationReport, ResourceAllocationError> {
    validate_inputs(resources, paths)?;

    let mut ordered_resources = resources.iter().collect::<Vec<_>>();
    ordered_resources.sort_by(|left, right| left.resource_id.cmp(&right.resource_id));

    let mut ordered_paths = paths.iter().collect::<Vec<_>>();
    ordered_paths.sort_by(|left, right| left.path_id.cmp(&right.path_id));

    let resource_indices = ordered_resources
        .iter()
        .enumerate()
        .map(|(index, resource)| (resource.resource_id.as_str(), index))
        .collect::<BTreeMap<_, _>>();

    let path_resource_indices = ordered_paths
        .iter()
        .map(|path| {
            path.resource_ids
                .iter()
                .map(|resource_id| {
                    resource_indices
                        .get(resource_id.as_str())
                        .copied()
                        .ok_or(ResourceAllocationError::UnknownResource)
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .collect::<Result<Vec<_>, _>>()?;

    let resource_capacities = ordered_resources
        .iter()
        .map(|resource| resource.capacity_bps)
        .collect::<Vec<_>>();

    let mut rates = progressive_fill(&ordered_paths, &path_resource_indices, &resource_capacities)?;
    let mut loads = resource_loads(&rates, &path_resource_indices, ordered_resources.len())?;

    // Resolve sub-bit fixed-point residue deterministically. This gives no
    // path more than one extra bit/s and never violates a resource ceiling.
    for path_index in 0..ordered_paths.len() {
        let path = ordered_paths[path_index];
        if rates[path_index] >= path.demand_bps {
            continue;
        }
        let fits = path_resource_indices[path_index]
            .iter()
            .all(|resource_index| {
                loads[*resource_index] + 1 <= u128::from(resource_capacities[*resource_index])
            });
        if fits {
            rates[path_index] = rates[path_index]
                .checked_add(1)
                .ok_or(ResourceAllocationError::ArithmeticOverflow)?;
            for resource_index in &path_resource_indices[path_index] {
                loads[*resource_index] = loads[*resource_index]
                    .checked_add(1)
                    .ok_or(ResourceAllocationError::ArithmeticOverflow)?;
            }
        }
    }

    let path_allocations = ordered_paths
        .iter()
        .zip(&rates)
        .map(|(path, allocated_bps)| PathRateAllocation {
            path_id: path.path_id.clone(),
            resource_ids: {
                let mut ids = path.resource_ids.clone();
                ids.sort();
                ids
            },
            weight: path.weight,
            demand_bps: path.demand_bps,
            allocated_bps: *allocated_bps,
            backlogged: *allocated_bps < path.demand_bps,
        })
        .collect::<Vec<_>>();

    let resource_usage = ordered_resources
        .iter()
        .enumerate()
        .map(|(index, resource)| {
            let allocated = u64::try_from(loads[index])
                .map_err(|_| ResourceAllocationError::ArithmeticOverflow)?;
            if allocated > resource.capacity_bps {
                return Err(ResourceAllocationError::InternalInvariant);
            }
            Ok(ResourceUsage {
                resource_id: resource.resource_id.clone(),
                capacity_bps: resource.capacity_bps,
                allocated_bps: allocated,
                remaining_capacity_bps: resource.capacity_bps - allocated,
                saturated: allocated == resource.capacity_bps,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let total_requested_bps = ordered_paths.iter().try_fold(0_u128, |sum, path| {
        sum.checked_add(u128::from(path.demand_bps))
            .ok_or(ResourceAllocationError::ArithmeticOverflow)
    })?;
    let total_allocated_bps = rates.iter().try_fold(0_u128, |sum, rate| {
        sum.checked_add(u128::from(*rate))
            .ok_or(ResourceAllocationError::ArithmeticOverflow)
    })?;
    let unserved_requested_bps = total_requested_bps
        .checked_sub(total_allocated_bps)
        .ok_or(ResourceAllocationError::InternalInvariant)?;

    let normalized_backlogged_rates = path_allocations
        .iter()
        .filter(|allocation| allocation.backlogged)
        .map(|allocation| allocation.allocated_bps as f64 / f64::from(allocation.weight))
        .collect::<Vec<_>>();

    Ok(ResourceAllocationReport {
        path_allocations,
        resource_usage,
        total_requested_bps,
        total_allocated_bps,
        unserved_requested_bps,
        backlogged_weighted_jain_index_ppm: jain_index_ppm(&normalized_backlogged_rates),
        objective: "weighted_max_min_path_rate",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resource(id: &str, capacity_bps: u64) -> ResourceCapacity {
        ResourceCapacity {
            resource_id: id.to_owned(),
            capacity_bps,
        }
    }

    fn path(id: &str, resources: &[&str], weight: u32, demand_bps: u64) -> PathRateDemand {
        PathRateDemand {
            path_id: id.to_owned(),
            resource_ids: resources.iter().map(|value| (*value).to_owned()).collect(),
            weight,
            demand_bps,
        }
    }

    fn rate(report: &ResourceAllocationReport, id: &str) -> u64 {
        report
            .path_allocations
            .iter()
            .find(|allocation| allocation.path_id == id)
            .expect("path allocation exists")
            .allocated_bps
    }

    #[test]
    fn independent_paths_each_receive_their_own_resource_capacity() {
        let result = allocate_weighted_max_min(
            &[resource("a", 100), resource("b", 100)],
            &[
                path("path-a", &["a"], 1, 100),
                path("path-b", &["b"], 1, 100),
            ],
        )
        .expect("feasible independent resources");
        assert_eq!(rate(&result, "path-a"), 100);
        assert_eq!(rate(&result, "path-b"), 100);
        assert_eq!(result.total_allocated_bps, 200);
        assert_eq!(result.backlogged_weighted_jain_index_ppm, None);
    }

    #[test]
    fn saturated_independent_resource_does_not_freeze_unrelated_path() {
        let result = allocate_weighted_max_min(
            &[resource("small", 10), resource("large", 100)],
            &[
                path("path-small", &["small"], 1, 100),
                path("path-large", &["large"], 1, 100),
            ],
        )
        .expect("independent resources progress separately");
        assert_eq!(rate(&result, "path-small"), 10);
        assert_eq!(rate(&result, "path-large"), 100);
        assert_eq!(result.total_allocated_bps, 110);
    }

    #[test]
    fn shared_backhaul_caps_sum_across_distinct_access_links() {
        let result = allocate_weighted_max_min(
            &[
                resource("access-a", 100),
                resource("access-b", 100),
                resource("shared-backhaul", 100),
            ],
            &[
                path("path-a", &["access-a", "shared-backhaul"], 1, 100),
                path("path-b", &["access-b", "shared-backhaul"], 1, 100),
            ],
        )
        .expect("shared backhaul allocation");
        assert_eq!(rate(&result, "path-a"), 50);
        assert_eq!(rate(&result, "path-b"), 50);
        assert_eq!(result.total_allocated_bps, 100);
        assert_eq!(
            result
                .resource_usage
                .iter()
                .find(|usage| usage.resource_id == "shared-backhaul")
                .expect("shared resource")
                .allocated_bps,
            100
        );
    }

    #[test]
    fn weighted_max_min_respects_path_weights() {
        let result = allocate_weighted_max_min(
            &[resource("shared", 90)],
            &[
                path("light", &["shared"], 1, 100),
                path("heavy", &["shared"], 2, 100),
            ],
        )
        .expect("weighted allocation");
        assert_eq!(rate(&result, "light"), 30);
        assert_eq!(rate(&result, "heavy"), 60);
        assert_eq!(result.backlogged_weighted_jain_index_ppm, Some(1_000_000));
    }

    #[test]
    fn demand_limited_path_releases_unused_share_to_backlogged_peer() {
        let result = allocate_weighted_max_min(
            &[resource("shared", 100)],
            &[
                path("small", &["shared"], 1, 10),
                path("large", &["shared"], 1, 100),
            ],
        )
        .expect("demand-limited allocation");
        assert_eq!(rate(&result, "small"), 10);
        assert_eq!(rate(&result, "large"), 90);
        assert_eq!(result.backlogged_weighted_jain_index_ppm, None);
    }

    #[test]
    fn overlapping_resource_sets_allocate_path_rates_without_double_counting() {
        let result = allocate_weighted_max_min(
            &[resource("x", 10), resource("y", 10)],
            &[
                path("a", &["x"], 1, 100),
                path("b", &["x", "y"], 1, 100),
                path("c", &["y"], 1, 100),
            ],
        )
        .expect("overlapping resource incidence");
        assert_eq!(rate(&result, "a"), 5);
        assert_eq!(rate(&result, "b"), 5);
        assert_eq!(rate(&result, "c"), 5);
        assert_eq!(result.total_allocated_bps, 15);
        assert!(result.resource_usage.iter().all(|usage| usage.saturated));
    }

    #[test]
    fn chain_fixture_exposes_max_min_vs_max_sum_tradeoff_against_exact_lp() {
        // Exact LP for A uses X, B uses X+Y, C uses Y, with cap(X)=cap(Y)=10:
        // max A+B+C subject to A+B<=10 and B+C<=10. The optimum is 20
        // (A=10, B=0, C=10), while weighted max-min is A=B=C=5, total 15.
        let result = allocate_weighted_max_min(
            &[resource("x", 10), resource("y", 10)],
            &[
                path("a", &["x"], 1, 100),
                path("b", &["x", "y"], 1, 100),
                path("c", &["y"], 1, 100),
            ],
        )
        .expect("resource incidence allocation");
        let exact_lp_max_sum_bps = 20_u128;
        assert_eq!(result.total_allocated_bps, 15);
        let gap_ppm =
            (exact_lp_max_sum_bps - result.total_allocated_bps) * 1_000_000 / exact_lp_max_sum_bps;
        assert_eq!(gap_ppm, 250_000);
    }

    #[test]
    fn shared_secondary_bottleneck_not_hidden_by_distinct_access_links() {
        let result = allocate_weighted_max_min(
            &[
                resource("access-1", 100),
                resource("access-2", 100),
                resource("provider-egress", 40),
            ],
            &[
                path("one", &["access-1", "provider-egress"], 1, 100),
                path("two", &["access-2", "provider-egress"], 1, 100),
            ],
        )
        .expect("shared provider egress allocation");
        assert_eq!(rate(&result, "one"), 20);
        assert_eq!(rate(&result, "two"), 20);
    }

    #[test]
    fn unrequested_capacity_is_reported_as_remaining_not_fabricated_demand() {
        let result = allocate_weighted_max_min(
            &[resource("shared", 100)],
            &[path("only", &["shared"], 1, 20)],
        )
        .expect("demand below capacity");
        assert_eq!(rate(&result, "only"), 20);
        assert_eq!(result.resource_usage[0].remaining_capacity_bps, 80);
    }

    #[test]
    fn input_order_does_not_change_any_allocation_or_report() {
        let resources = vec![resource("b", 100), resource("a", 100), resource("c", 80)];
        let paths = vec![
            path("path-c", &["c", "a"], 2, 100),
            path("path-b", &["b"], 1, 100),
            path("path-a", &["a"], 1, 100),
        ];
        let first = allocate_weighted_max_min(&resources, &paths).expect("allocation");
        let second = allocate_weighted_max_min(
            &resources.into_iter().rev().collect::<Vec<_>>(),
            &paths.into_iter().rev().collect::<Vec<_>>(),
        )
        .expect("reordered allocation");
        assert_eq!(first, second);
    }

    #[test]
    fn duplicate_resource_id_is_rejected() {
        assert_eq!(
            allocate_weighted_max_min(
                &[resource("same", 100), resource("same", 100)],
                &[path("path", &["same"], 1, 100)],
            )
            .unwrap_err(),
            ResourceAllocationError::DuplicateResourceId
        );
    }

    #[test]
    fn duplicate_resource_within_one_path_is_rejected() {
        assert_eq!(
            allocate_weighted_max_min(&[resource("a", 100)], &[path("path", &["a", "a"], 1, 100)],)
                .unwrap_err(),
            ResourceAllocationError::DuplicateResourceInPath
        );
    }

    #[test]
    fn unknown_resource_is_rejected() {
        assert_eq!(
            allocate_weighted_max_min(
                &[resource("a", 100)],
                &[path("path", &["unknown"], 1, 100)],
            )
            .unwrap_err(),
            ResourceAllocationError::UnknownResource
        );
    }

    #[test]
    fn empty_and_oversized_resource_sets_are_rejected() {
        assert_eq!(
            allocate_weighted_max_min(&[resource("a", 100)], &[path("empty", &[], 1, 100)],)
                .unwrap_err(),
            ResourceAllocationError::EmptyResourceSet
        );
        let many = (0..=MAX_RESOURCES_PER_PATH)
            .map(|index| format!("r{index}"))
            .collect::<Vec<_>>();
        let resources = many.iter().map(|id| resource(id, 100)).collect::<Vec<_>>();
        assert_eq!(
            allocate_weighted_max_min(
                &resources,
                &[PathRateDemand {
                    path_id: "many".to_owned(),
                    resource_ids: many,
                    weight: 1,
                    demand_bps: 100,
                }],
            )
            .unwrap_err(),
            ResourceAllocationError::TooManyResourcesPerPath
        );
    }

    #[test]
    fn weight_and_demand_bounds_fail_closed() {
        assert_eq!(
            allocate_weighted_max_min(
                &[resource("a", 100)],
                &[path("zero-weight", &["a"], 0, 100)],
            )
            .unwrap_err(),
            ResourceAllocationError::ZeroWeight
        );
        assert_eq!(
            allocate_weighted_max_min(&[resource("a", 100)], &[path("zero-demand", &["a"], 1, 0)],)
                .unwrap_err(),
            ResourceAllocationError::ZeroDemand
        );
        assert_eq!(
            allocate_weighted_max_min(
                &[resource("a", 100)],
                &[path("oversized-weight", &["a"], MAX_WEIGHT + 1, 100)],
            )
            .unwrap_err(),
            ResourceAllocationError::WeightTooLarge
        );
    }

    #[test]
    fn resource_capacity_is_never_exceeded() {
        let result = allocate_weighted_max_min(
            &[
                resource("r1", 123_456_789),
                resource("r2", 87_654_321),
                resource("r3", 250_000_000),
            ],
            &[
                path("p1", &["r1", "r2"], 1, 500_000_000),
                path("p2", &["r2", "r3"], 7, 500_000_000),
                path("p3", &["r3"], 3, 500_000_000),
            ],
        )
        .expect("bounded resource allocation");
        assert!(
            result
                .resource_usage
                .iter()
                .all(|usage| usage.allocated_bps <= usage.capacity_bps)
        );
    }
}
