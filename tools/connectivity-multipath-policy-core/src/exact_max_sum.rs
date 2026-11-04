//! Bounded exhaustive max-sum reference for small resource-incidence graphs.
//!
//! This implementation is intentionally independent from progressive filling.
//! It enumerates the complete bounded integer rate space and is a benchmark
//! oracle only; it is not part of the production scheduling path.

use crate::resource_allocation::{PathRateDemand, ResourceCapacity};
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_ORACLE_PATHS: usize = 8;
pub const MAX_ORACLE_RESOURCES: usize = 16;
pub const MAX_ORACLE_ID_BYTES: usize = 128;
pub const MAX_ORACLE_RATE_BPS: u64 = 1_000;
pub const MAX_ORACLE_STATES: u64 = 2_000_000;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PathRateFloor {
    pub path_id: String,
    pub minimum_bps: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExactPathRateAllocation {
    pub path_id: String,
    pub resource_ids: Vec<String>,
    pub demand_bps: u64,
    pub minimum_bps: u64,
    pub allocated_bps: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExactResourceUsage {
    pub resource_id: String,
    pub capacity_bps: u64,
    pub allocated_bps: u64,
    pub remaining_capacity_bps: u64,
    pub saturated: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExactMaxSumReport {
    pub path_allocations: Vec<ExactPathRateAllocation>,
    pub resource_usage: Vec<ExactResourceUsage>,
    pub total_requested_bps: u128,
    pub total_allocated_bps: u128,
    pub unserved_requested_bps: u128,
    pub zero_rate_path_count: usize,
    /// Unweighted Jain index over all path allocations, including zero-rate paths.
    /// Undefined when fewer than two paths exist or every allocation is zero.
    pub unweighted_jain_index_ppm: Option<u32>,
    /// Complete rate-vector assignments visited, including resource-infeasible assignments.
    pub assignments_examined: u64,
    /// Assignments satisfying every resource capacity constraint.
    pub feasible_allocations_examined: u64,
    pub search_space_size: u64,
    /// Equal-sum witnesses prefer lexicographically larger rates in sorted path-ID order.
    pub tie_break: &'static str,
    pub objective: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExactMaxSumError {
    EmptyPaths,
    EmptyResources,
    TooManyPaths,
    TooManyResources,
    TooManyResourcesPerPath,
    InvalidPathId,
    InvalidResourceId,
    DuplicatePathId,
    DuplicateResourceId,
    DuplicateResourceInPath,
    UnknownResource,
    ZeroDemand,
    ZeroWeight,
    WeightTooLarge,
    DemandBoundExceeded,
    ResourceCapacityBoundExceeded,
    InvalidFloorPathId,
    DuplicateFloorPathId,
    UnknownFloorPath,
    MinimumExceedsDemand,
    SearchSpaceTooLarge,
    InfeasibleMinimums,
    ArithmeticOverflow,
    InternalInvariant,
}

#[derive(Clone, Debug)]
struct SearchState {
    assignments_examined: u64,
    feasible_allocations_examined: u64,
    best_total_bps: Option<u128>,
    best_rates: Option<Vec<u64>>,
    best_resource_loads: Option<Vec<u64>>,
}

fn valid_id(id: &str) -> bool {
    !id.trim().is_empty() && id.len() <= MAX_ORACLE_ID_BYTES && !id.chars().any(char::is_control)
}

fn checked_total(values: impl Iterator<Item = u64>) -> Result<u128, ExactMaxSumError> {
    values.try_fold(0_u128, |sum, value| {
        sum.checked_add(u128::from(value))
            .ok_or(ExactMaxSumError::ArithmeticOverflow)
    })
}

fn calculate_resource_loads(
    rates: &[u64],
    path_resource_indices: &[Vec<usize>],
    resource_count: usize,
) -> Result<Vec<u64>, ExactMaxSumError> {
    if rates.len() != path_resource_indices.len() {
        return Err(ExactMaxSumError::InternalInvariant);
    }
    let mut loads = vec![0_u64; resource_count];
    for (rate, resource_indices) in rates.iter().zip(path_resource_indices) {
        for resource_index in resource_indices {
            let load = loads
                .get_mut(*resource_index)
                .ok_or(ExactMaxSumError::InternalInvariant)?;
            *load = load
                .checked_add(*rate)
                .ok_or(ExactMaxSumError::ArithmeticOverflow)?;
        }
    }
    Ok(loads)
}

fn resource_feasible(loads: &[u64], capacities: &[u64]) -> bool {
    loads.len() == capacities.len()
        && loads
            .iter()
            .zip(capacities)
            .all(|(load, capacity)| load <= capacity)
}

fn should_replace_witness(
    total_bps: u128,
    rates: &[u64],
    current_total_bps: Option<u128>,
    current_rates: Option<&[u64]>,
) -> bool {
    match (current_total_bps, current_rates) {
        (None, None) => true,
        (Some(current_total), Some(current_rates)) => {
            total_bps > current_total || (total_bps == current_total && rates > current_rates)
        }
        _ => false,
    }
}

fn enumerate_rate_vectors(
    path_index: usize,
    ordered_paths: &[&PathRateDemand],
    floors: &[u64],
    path_resource_indices: &[Vec<usize>],
    resource_capacities: &[u64],
    rates: &mut [u64],
    state: &mut SearchState,
) -> Result<(), ExactMaxSumError> {
    if path_index == ordered_paths.len() {
        state.assignments_examined = state
            .assignments_examined
            .checked_add(1)
            .ok_or(ExactMaxSumError::ArithmeticOverflow)?;
        let loads =
            calculate_resource_loads(rates, path_resource_indices, resource_capacities.len())?;
        if !resource_feasible(&loads, resource_capacities) {
            return Ok(());
        }

        state.feasible_allocations_examined = state
            .feasible_allocations_examined
            .checked_add(1)
            .ok_or(ExactMaxSumError::ArithmeticOverflow)?;
        let total_bps = rates.iter().try_fold(0_u128, |sum, rate| {
            sum.checked_add(u128::from(*rate))
                .ok_or(ExactMaxSumError::ArithmeticOverflow)
        })?;
        if should_replace_witness(
            total_bps,
            rates,
            state.best_total_bps,
            state.best_rates.as_deref(),
        ) {
            state.best_total_bps = Some(total_bps);
            state.best_rates = Some(rates.to_vec());
            state.best_resource_loads = Some(loads);
        }
        return Ok(());
    }

    let minimum = floors[path_index];
    let demand = ordered_paths[path_index].demand_bps;
    // Descending traversal plus an explicit comparison makes equal-optimum
    // selection deterministic even when caller input order is permuted.
    let mut rate = demand;
    loop {
        rates[path_index] = rate;
        enumerate_rate_vectors(
            path_index + 1,
            ordered_paths,
            floors,
            path_resource_indices,
            resource_capacities,
            rates,
            state,
        )?;
        if rate == minimum {
            break;
        }
        rate -= 1;
    }
    Ok(())
}

fn jain_index_ppm(rates: &[u64]) -> Result<Option<u32>, ExactMaxSumError> {
    if rates.len() < 2 {
        return Ok(None);
    }
    let sum = rates.iter().try_fold(0_u128, |total, rate| {
        total
            .checked_add(u128::from(*rate))
            .ok_or(ExactMaxSumError::ArithmeticOverflow)
    })?;
    let sum_squares = rates.iter().try_fold(0_u128, |total, rate| {
        let square = u128::from(*rate)
            .checked_mul(u128::from(*rate))
            .ok_or(ExactMaxSumError::ArithmeticOverflow)?;
        total
            .checked_add(square)
            .ok_or(ExactMaxSumError::ArithmeticOverflow)
    })?;
    if sum_squares == 0 {
        return Ok(None);
    }
    let numerator = sum
        .checked_mul(sum)
        .and_then(|value| value.checked_mul(1_000_000))
        .ok_or(ExactMaxSumError::ArithmeticOverflow)?;
    let denominator = u128::try_from(rates.len())
        .map_err(|_| ExactMaxSumError::ArithmeticOverflow)?
        .checked_mul(sum_squares)
        .ok_or(ExactMaxSumError::ArithmeticOverflow)?;
    let rounded = numerator
        .checked_add(denominator / 2)
        .ok_or(ExactMaxSumError::ArithmeticOverflow)?
        / denominator;
    u32::try_from(rounded.min(1_000_000))
        .map(Some)
        .map_err(|_| ExactMaxSumError::ArithmeticOverflow)
}

fn validate_and_index(
    resources: &[ResourceCapacity],
    paths: &[PathRateDemand],
    minimums: &[PathRateFloor],
) -> Result<
    (
        Vec<&PathRateDemand>,
        Vec<&ResourceCapacity>,
        Vec<u64>,
        Vec<Vec<usize>>,
        u64,
    ),
    ExactMaxSumError,
> {
    if paths.is_empty() {
        return Err(ExactMaxSumError::EmptyPaths);
    }
    if resources.is_empty() {
        return Err(ExactMaxSumError::EmptyResources);
    }
    if paths.len() > MAX_ORACLE_PATHS {
        return Err(ExactMaxSumError::TooManyPaths);
    }
    if resources.len() > MAX_ORACLE_RESOURCES {
        return Err(ExactMaxSumError::TooManyResources);
    }

    let mut ordered_resources = resources.iter().collect::<Vec<_>>();
    ordered_resources.sort_by(|left, right| left.resource_id.cmp(&right.resource_id));
    let mut resource_indices = BTreeMap::new();
    for (index, resource) in ordered_resources.iter().enumerate() {
        if !valid_id(&resource.resource_id) {
            return Err(ExactMaxSumError::InvalidResourceId);
        }
        if resource_indices
            .insert(resource.resource_id.as_str(), index)
            .is_some()
        {
            return Err(ExactMaxSumError::DuplicateResourceId);
        }
        if resource.capacity_bps > MAX_ORACLE_RATE_BPS {
            return Err(ExactMaxSumError::ResourceCapacityBoundExceeded);
        }
    }

    let mut ordered_paths = paths.iter().collect::<Vec<_>>();
    ordered_paths.sort_by(|left, right| left.path_id.cmp(&right.path_id));
    let mut seen_path_ids = BTreeSet::new();
    for path in &ordered_paths {
        if !valid_id(&path.path_id) {
            return Err(ExactMaxSumError::InvalidPathId);
        }
        if !seen_path_ids.insert(path.path_id.as_str()) {
            return Err(ExactMaxSumError::DuplicatePathId);
        }
        if path.demand_bps == 0 {
            return Err(ExactMaxSumError::ZeroDemand);
        }
        if path.demand_bps > MAX_ORACLE_RATE_BPS {
            return Err(ExactMaxSumError::DemandBoundExceeded);
        }
        if path.weight == 0 {
            return Err(ExactMaxSumError::ZeroWeight);
        }
        if path.weight > 1_000_000 {
            return Err(ExactMaxSumError::WeightTooLarge);
        }
        if path.resource_ids.is_empty() {
            return Err(ExactMaxSumError::EmptyResourceSet);
        }
        if path.resource_ids.len() > MAX_ORACLE_RESOURCES {
            return Err(ExactMaxSumError::TooManyResourcesPerPath);
        }
        let mut seen = BTreeSet::new();
        for resource_id in &path.resource_ids {
            if !valid_id(resource_id) {
                return Err(ExactMaxSumError::InvalidResourceId);
            }
            if !seen.insert(resource_id.as_str()) {
                return Err(ExactMaxSumError::DuplicateResourceInPath);
            }
            if !resource_indices.contains_key(resource_id.as_str()) {
                return Err(ExactMaxSumError::UnknownResource);
            }
        }
    }

    let mut minimum_by_path = BTreeMap::new();
    for minimum in minimums {
        if !valid_id(&minimum.path_id) {
            return Err(ExactMaxSumError::InvalidFloorPathId);
        }
        if minimum_by_path
            .insert(minimum.path_id.as_str(), minimum.minimum_bps)
            .is_some()
        {
            return Err(ExactMaxSumError::DuplicateFloorPathId);
        }
        if !seen_path_ids.contains(minimum.path_id.as_str()) {
            return Err(ExactMaxSumError::UnknownFloorPath);
        }
    }

    let mut floors = Vec::with_capacity(ordered_paths.len());
    let mut search_space_size = 1_u64;
    for path in &ordered_paths {
        let minimum = *minimum_by_path.get(path.path_id.as_str()).unwrap_or(&0);
        if minimum > path.demand_bps {
            return Err(ExactMaxSumError::MinimumExceedsDemand);
        }
        floors.push(minimum);
        let choices = path
            .demand_bps
            .checked_sub(minimum)
            .and_then(|remaining| remaining.checked_add(1))
            .ok_or(ExactMaxSumError::ArithmeticOverflow)?;
        search_space_size = search_space_size
            .checked_mul(choices)
            .ok_or(ExactMaxSumError::SearchSpaceTooLarge)?;
        if search_space_size > MAX_ORACLE_STATES {
            return Err(ExactMaxSumError::SearchSpaceTooLarge);
        }
    }

    let path_resource_indices = ordered_paths
        .iter()
        .map(|path| {
            let mut indices = path
                .resource_ids
                .iter()
                .map(|resource_id| {
                    resource_indices
                        .get(resource_id.as_str())
                        .copied()
                        .ok_or(ExactMaxSumError::UnknownResource)
                })
                .collect::<Result<Vec<_>, _>>()?;
            indices.sort_unstable();
            Ok(indices)
        })
        .collect::<Result<Vec<_>, ExactMaxSumError>>()?;
    Ok((
        ordered_paths,
        ordered_resources,
        floors,
        path_resource_indices,
        search_space_size,
    ))
}

/// Find the exact maximum-sum integer allocation for one bounded resource graph.
///
/// The oracle enumerates every path-rate vector from the supplied per-path
/// floors through each path's demand. Resource capacity zero is permitted and
/// correctly forces every path using that resource to rate zero. Weights are
/// validated for input consistency but do not affect the sum-rate objective.
pub fn maximize_total_rate(
    resources: &[ResourceCapacity],
    paths: &[PathRateDemand],
) -> Result<ExactMaxSumReport, ExactMaxSumError> {
    maximize_total_rate_with_floors(resources, paths, &[])
}

/// Find the exact maximum-sum integer allocation subject to per-path floors.
///
/// Omitted floors mean zero. A floor vector is keyed by path ID, so input
/// permutation cannot change which path receives a minimum. Equal-sum optima
/// prefer lexicographically larger rate vectors in ascending path-ID order.
pub fn maximize_total_rate_with_floors(
    resources: &[ResourceCapacity],
    paths: &[PathRateDemand],
    minimums: &[PathRateFloor],
) -> Result<ExactMaxSumReport, ExactMaxSumError> {
    let (ordered_paths, ordered_resources, floors, path_resource_indices, search_space_size) =
        validate_and_index(resources, paths, minimums)?;
    let resource_capacities = ordered_resources
        .iter()
        .map(|resource| resource.capacity_bps)
        .collect::<Vec<_>>();
    let mut state = SearchState {
        assignments_examined: 0,
        feasible_allocations_examined: 0,
        best_total_bps: None,
        best_rates: None,
        best_resource_loads: None,
    };
    let mut rates = vec![0_u64; ordered_paths.len()];
    enumerate_rate_vectors(
        0,
        &ordered_paths,
        &floors,
        &path_resource_indices,
        &resource_capacities,
        &mut rates,
        &mut state,
    )?;
    let best_rates = state
        .best_rates
        .ok_or(ExactMaxSumError::InfeasibleMinimums)?;
    let best_loads = state
        .best_resource_loads
        .ok_or(ExactMaxSumError::InternalInvariant)?;
    let total_requested_bps = checked_total(ordered_paths.iter().map(|path| path.demand_bps))?;
    let total_allocated_bps = state
        .best_total_bps
        .ok_or(ExactMaxSumError::InternalInvariant)?;
    let unserved_requested_bps = total_requested_bps
        .checked_sub(total_allocated_bps)
        .ok_or(ExactMaxSumError::InternalInvariant)?;
    let zero_rate_path_count = best_rates.iter().filter(|rate| **rate == 0).count();

    let path_allocations = ordered_paths
        .iter()
        .zip(&floors)
        .zip(&best_rates)
        .map(|((path, minimum_bps), allocated_bps)| {
            let mut resource_ids = path.resource_ids.clone();
            resource_ids.sort();
            ExactPathRateAllocation {
                path_id: path.path_id.clone(),
                resource_ids,
                demand_bps: path.demand_bps,
                minimum_bps: *minimum_bps,
                allocated_bps: *allocated_bps,
            }
        })
        .collect::<Vec<_>>();

    let resource_usage = ordered_resources
        .iter()
        .zip(&best_loads)
        .map(|(resource, allocated_bps)| {
            if *allocated_bps > resource.capacity_bps {
                return Err(ExactMaxSumError::InternalInvariant);
            }
            Ok(ExactResourceUsage {
                resource_id: resource.resource_id.clone(),
                capacity_bps: resource.capacity_bps,
                allocated_bps: *allocated_bps,
                remaining_capacity_bps: resource.capacity_bps - *allocated_bps,
                saturated: *allocated_bps == resource.capacity_bps,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(ExactMaxSumReport {
        path_allocations,
        resource_usage,
        total_requested_bps,
        total_allocated_bps,
        unserved_requested_bps,
        zero_rate_path_count,
        unweighted_jain_index_ppm: jain_index_ppm(&best_rates)?,
        assignments_examined: state.assignments_examined,
        feasible_allocations_examined: state.feasible_allocations_examined,
        search_space_size,
        tie_break: "lexicographically_larger_rate_vector_by_sorted_path_id",
        objective: "maximum_sum_requested_integer_path_rate",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resource_allocation::allocate_weighted_max_min;

    fn resource(id: &str, capacity: u64) -> ResourceCapacity {
        ResourceCapacity {
            resource_id: id.to_owned(),
            capacity_bps: capacity,
        }
    }

    fn path(id: &str, resources: &[&str], demand: u64) -> PathRateDemand {
        PathRateDemand {
            path_id: id.to_owned(),
            resource_ids: resources.iter().map(|id| (*id).to_owned()).collect(),
            weight: 1,
            demand_bps: demand,
        }
    }

    fn rate(report: &ExactMaxSumReport, id: &str) -> u64 {
        report
            .path_allocations
            .iter()
            .find(|path| path.path_id == id)
            .expect("path allocation")
            .allocated_bps
    }

    #[test]
    fn chain_fixture_exactly_measures_weighted_max_min_utilization_gap() {
        let resources = [resource("x", 10), resource("y", 10)];
        let paths = [
            path("a", &["x"], 100),
            path("b", &["x", "y"], 100),
            path("c", &["y"], 100),
        ];
        let exact = maximize_total_rate(&resources, &paths).expect("bounded exact result");
        let fair = allocate_weighted_max_min(&resources, &paths).expect("weighted max-min result");
        assert_eq!(fair.total_allocated_bps, 15);
        assert_eq!(exact.total_allocated_bps, 20);
        assert_eq!(
            (rate(&exact, "a"), rate(&exact, "b"), rate(&exact, "c")),
            (10, 0, 10)
        );
        let gap_ppm = (exact.total_allocated_bps - fair.total_allocated_bps) * 1_000_000
            / exact.total_allocated_bps;
        assert_eq!(gap_ppm, 250_000);
        assert_eq!(exact.zero_rate_path_count, 1);
        assert_eq!(exact.unweighted_jain_index_ppm, Some(666_667));
        assert_eq!(exact.assignments_examined, 1_030_301);
    }

    #[test]
    fn two_paths_on_one_resource_report_starvation_and_fairness_separately() {
        let report = maximize_total_rate(
            &[resource("shared", 10)],
            &[path("a", &["shared"], 10), path("b", &["shared"], 10)],
        )
        .expect("bounded exact result");
        assert_eq!(report.total_allocated_bps, 10);
        assert_eq!((rate(&report, "a"), rate(&report, "b")), (10, 0));
        assert_eq!(report.zero_rate_path_count, 1);
        assert_eq!(report.unweighted_jain_index_ppm, Some(500_000));
    }

    #[test]
    fn independent_resources_reach_the_sum_of_their_capacities() {
        let report = maximize_total_rate(
            &[resource("x", 10), resource("y", 12)],
            &[path("a", &["x"], 10), path("b", &["y"], 10)],
        )
        .expect("independent resources");
        assert_eq!(report.total_allocated_bps, 20);
        assert_eq!(report.unserved_requested_bps, 0);
    }

    #[test]
    fn demand_limited_path_releases_unused_capacity_to_peer() {
        let report = maximize_total_rate(
            &[resource("shared", 10)],
            &[
                path("small", &["shared"], 2),
                path("large", &["shared"], 10),
            ],
        )
        .expect("demand limited max-sum");
        assert_eq!(rate(&report, "small"), 2);
        assert_eq!(rate(&report, "large"), 8);
        assert_eq!(report.total_allocated_bps, 10);
    }

    #[test]
    fn minimum_floors_constrain_max_sum_without_violating_shared_resources() {
        let report = maximize_total_rate_with_floors(
            &[resource("x", 10), resource("y", 10)],
            &[
                path("a", &["x"], 10),
                path("b", &["x", "y"], 10),
                path("c", &["y"], 10),
            ],
            &[
                PathRateFloor {
                    path_id: "a".to_owned(),
                    minimum_bps: 2,
                },
                PathRateFloor {
                    path_id: "b".to_owned(),
                    minimum_bps: 2,
                },
                PathRateFloor {
                    path_id: "c".to_owned(),
                    minimum_bps: 2,
                },
            ],
        )
        .expect("feasible floors");
        assert_eq!(report.total_allocated_bps, 18);
        assert_eq!(rate(&report, "b"), 2);
        assert!(
            report
                .path_allocations
                .iter()
                .all(|path| path.allocated_bps >= path.minimum_bps)
        );
    }

    #[test]
    fn infeasible_minimum_floors_are_reported_not_relaxed() {
        let error = maximize_total_rate_with_floors(
            &[resource("x", 10)],
            &[path("a", &["x"], 10), path("b", &["x"], 10)],
            &[
                PathRateFloor {
                    path_id: "a".to_owned(),
                    minimum_bps: 6,
                },
                PathRateFloor {
                    path_id: "b".to_owned(),
                    minimum_bps: 5,
                },
            ],
        )
        .unwrap_err();
        assert_eq!(error, ExactMaxSumError::InfeasibleMinimums);
    }

    #[test]
    fn zero_residual_resource_is_a_valid_zero_rate_boundary() {
        let report = maximize_total_rate(
            &[resource("drained-backhaul", 0), resource("access", 10)],
            &[
                path("drained", &["drained-backhaul"], 10),
                path("other", &["access"], 10),
            ],
        )
        .expect("zero residual resource is valid evidence");
        assert_eq!(rate(&report, "drained"), 0);
        assert_eq!(rate(&report, "other"), 10);
    }

    #[test]
    fn input_permutations_produce_the_same_exact_witness() {
        let resources = [resource("y", 10), resource("x", 10)];
        let paths = [
            path("c", &["y"], 100),
            path("a", &["x"], 100),
            path("b", &["x", "y"], 100),
        ];
        let first = maximize_total_rate(&resources, &paths).expect("first input order");
        let second = maximize_total_rate(
            &resources.into_iter().rev().collect::<Vec<_>>(),
            &paths.into_iter().rev().collect::<Vec<_>>(),
        )
        .expect("permuted input order");
        assert_eq!(first, second);
    }

    #[test]
    fn enumeration_budget_exceeded_is_explicitly_unsupported() {
        let resources = [resource("shared", 1_000)];
        let paths = (0..8)
            .map(|index| path(&format!("path-{index}"), &["shared"], 500))
            .collect::<Vec<_>>();
        assert_eq!(
            maximize_total_rate(&resources, &paths).unwrap_err(),
            ExactMaxSumError::SearchSpaceTooLarge
        );
    }

    #[test]
    fn demand_and_resource_bounds_are_enforced() {
        assert_eq!(
            maximize_total_rate(&[resource("x", 100)], &[path("too-big", &["x"], 1_001)],)
                .unwrap_err(),
            ExactMaxSumError::DemandBoundExceeded
        );
        assert_eq!(
            maximize_total_rate(&[resource("x", 1_001)], &[path("a", &["x"], 100)],).unwrap_err(),
            ExactMaxSumError::ResourceCapacityBoundExceeded
        );
    }

    #[test]
    fn duplicate_and_unknown_resource_incidence_are_rejected() {
        let mut duplicate = path("a", &["x"], 10);
        duplicate.resource_ids.push("x".to_owned());
        assert_eq!(
            maximize_total_rate(&[resource("x", 10)], &[duplicate]).unwrap_err(),
            ExactMaxSumError::DuplicateResourceInPath
        );
        assert_eq!(
            maximize_total_rate(&[resource("x", 10)], &[path("a", &["missing"], 10)]).unwrap_err(),
            ExactMaxSumError::UnknownResource
        );
    }

    #[test]
    fn empty_path_resource_incidence_is_rejected() {
        assert_eq!(
            maximize_total_rate(&[resource("x", 10)], &[path("empty", &[], 10)],).unwrap_err(),
            ExactMaxSumError::EmptyResourceSet
        );
    }

    #[test]
    fn duplicate_and_unknown_path_floors_are_rejected() {
        let resources = [resource("x", 10)];
        let paths = [path("a", &["x"], 10)];
        let duplicate = [
            PathRateFloor {
                path_id: "a".to_owned(),
                minimum_bps: 1,
            },
            PathRateFloor {
                path_id: "a".to_owned(),
                minimum_bps: 2,
            },
        ];
        assert_eq!(
            maximize_total_rate_with_floors(&resources, &paths, &duplicate).unwrap_err(),
            ExactMaxSumError::DuplicateFloorPathId
        );
        assert_eq!(
            maximize_total_rate_with_floors(
                &resources,
                &paths,
                &[PathRateFloor {
                    path_id: "missing".to_owned(),
                    minimum_bps: 1
                }]
            )
            .unwrap_err(),
            ExactMaxSumError::UnknownFloorPath
        );
    }

    #[test]
    fn floor_must_not_exceed_path_demand() {
        assert_eq!(
            maximize_total_rate_with_floors(
                &[resource("x", 10)],
                &[path("a", &["x"], 10)],
                &[PathRateFloor {
                    path_id: "a".to_owned(),
                    minimum_bps: 11
                }],
            )
            .unwrap_err(),
            ExactMaxSumError::MinimumExceedsDemand
        );
    }

    #[test]
    fn empty_resource_and_path_sets_are_invalid() {
        assert_eq!(
            maximize_total_rate(&[], &[path("a", &["x"], 10)]).unwrap_err(),
            ExactMaxSumError::EmptyResources
        );
        assert_eq!(
            maximize_total_rate(&[resource("x", 10)], &[]).unwrap_err(),
            ExactMaxSumError::EmptyPaths
        );
    }
}
