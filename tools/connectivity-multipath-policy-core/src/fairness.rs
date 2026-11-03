//! Deterministic tenant-first weighted max-min sharing for one shared bottleneck.
//!
//! This allocates a declared bottleneck capacity among caller-supplied tenant
//! principal IDs and then among each tenant's flows. Authentication and principal
//! binding are external to this module. It is a resource allocator,
//! not a packet scheduler, qdisc or tenant-authentication system.

use std::collections::{BTreeMap, BTreeSet};

const MAX_FLOWS: usize = 512;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FlowDemand {
    pub flow_id: String,
    pub tenant_id: String,
    /// Weight between tenants. All flows from one tenant must declare the same value.
    pub tenant_weight: u32,
    /// Weight among flows within this tenant.
    pub flow_weight: u32,
    pub demand_bps: u64,
    /// Minimum service floor for this flow in bits per second.
    pub minimum_bps: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FlowRateAllocation {
    pub flow_id: String,
    pub tenant_id: String,
    pub demand_bps: u64,
    pub minimum_bps: u64,
    pub allocated_bps: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TenantRateAllocation {
    pub tenant_id: String,
    pub weight: u32,
    pub demand_bps: u64,
    pub minimum_bps: u64,
    pub allocated_bps: u64,
    pub active_flow_count: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FairnessReport {
    pub capacity_bps: u64,
    pub allocated_bps: u64,
    pub unused_capacity_bps: u64,
    pub tenant_weighted_jain_index_ppm: Option<u32>,
    /// Worst weighted Jain index among tenants with at least two backlogged flows.
    pub worst_tenant_flow_weighted_jain_index_ppm: Option<u32>,
    pub per_tenant_flow_fairness: Vec<TenantFlowFairness>,
    pub tenant_allocations: Vec<TenantRateAllocation>,
    pub flow_allocations: Vec<FlowRateAllocation>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TenantFlowFairness {
    pub tenant_id: String,
    pub backlogged_flow_count: u32,
    pub weighted_jain_index_ppm: Option<u32>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FairnessError {
    ZeroCapacity,
    EmptyDemand,
    TooManyFlows,
    EmptyFlowId,
    EmptyTenantId,
    DuplicateFlowId,
    InvalidWeight,
    MinimumExceedsDemand,
    InconsistentTenantWeight,
    MinimumFloorsExceedCapacity,
    ArithmeticOverflow,
    InternalInvariant,
}

#[derive(Clone, Debug)]
struct ShareDemand {
    id: String,
    weight: u32,
    demand_bps: u64,
    minimum_bps: u64,
}

fn checked_sum(values: impl Iterator<Item = u64>) -> Result<u64, FairnessError> {
    values.try_fold(0_u64, |sum, value| {
        sum.checked_add(value).ok_or(FairnessError::ArithmeticOverflow)
    })
}

/// Integer weighted max-min water filling with deterministic largest-remainder
/// assignment. Floors are satisfied before residual capacity is shared.
fn weighted_max_min(
    capacity_bps: u64,
    demands: &[ShareDemand],
) -> Result<BTreeMap<String, u64>, FairnessError> {
    let minimum_total = checked_sum(demands.iter().map(|item| item.minimum_bps))?;
    if minimum_total > capacity_bps {
        return Err(FairnessError::MinimumFloorsExceedCapacity);
    }

    let mut allocations = BTreeMap::new();
    let mut active = Vec::new();
    for demand in demands {
        allocations.insert(demand.id.clone(), demand.minimum_bps);
        if demand.minimum_bps < demand.demand_bps {
            active.push(demand.clone());
        }
    }
    active.sort_by(|left, right| left.id.cmp(&right.id));
    let mut remaining = capacity_bps - minimum_total;

    while remaining > 0 && !active.is_empty() {
        let total_weight = active.iter().try_fold(0_u128, |sum, item| {
            sum.checked_add(u128::from(item.weight))
                .ok_or(FairnessError::ArithmeticOverflow)
        })?;
        if total_weight == 0 {
            return Err(FairnessError::InvalidWeight);
        }

        let mut saturated_ids = BTreeSet::new();
        let mut saturated_total = 0_u64;
        for item in &active {
            let current = *allocations
                .get(&item.id)
                .ok_or(FairnessError::InternalInvariant)?;
            let gap = item.demand_bps.saturating_sub(current);
            let proportional_numerator =
                u128::from(remaining) * u128::from(item.weight);
            let saturation_numerator = u128::from(gap) * total_weight;
            if proportional_numerator >= saturation_numerator {
                saturated_ids.insert(item.id.clone());
                saturated_total = saturated_total
                    .checked_add(gap)
                    .ok_or(FairnessError::ArithmeticOverflow)?;
            }
        }

        if !saturated_ids.is_empty() {
            if saturated_total > remaining {
                return Err(FairnessError::InternalInvariant);
            }
            for item in &active {
                if saturated_ids.contains(&item.id) {
                    let current = *allocations
                        .get(&item.id)
                        .ok_or(FairnessError::InternalInvariant)?;
                    allocations.insert(item.id.clone(), item.demand_bps.max(current));
                }
            }
            remaining -= saturated_total;
            active.retain(|item| !saturated_ids.contains(&item.id));
            continue;
        }

        let mut increments = Vec::with_capacity(active.len());
        let mut used = 0_u64;
        for item in &active {
            let numerator = u128::from(remaining) * u128::from(item.weight);
            let share = u64::try_from(numerator / total_weight)
                .map_err(|_| FairnessError::ArithmeticOverflow)?;
            let remainder = numerator % total_weight;
            used = used.checked_add(share).ok_or(FairnessError::ArithmeticOverflow)?;
            increments.push((item.id.clone(), share, remainder));
        }

        let mut leftover = remaining
            .checked_sub(used)
            .ok_or(FairnessError::InternalInvariant)?;
        increments.sort_by(|left, right| {
            right
                .2
                .cmp(&left.2)
                .then_with(|| left.0.cmp(&right.0))
        });

        for (id, mut increment, _) in increments {
            if leftover > 0 {
                increment = increment
                    .checked_add(1)
                    .ok_or(FairnessError::ArithmeticOverflow)?;
                leftover -= 1;
            }
            let current = *allocations.get(&id).ok_or(FairnessError::InternalInvariant)?;
            let next = current
                .checked_add(increment)
                .ok_or(FairnessError::ArithmeticOverflow)?;
            let demand = active
                .iter()
                .find(|item| item.id == id)
                .map(|item| item.demand_bps)
                .ok_or(FairnessError::InternalInvariant)?;
            if next > demand {
                return Err(FairnessError::InternalInvariant);
            }
            allocations.insert(id, next);
        }
        if leftover != 0 {
            return Err(FairnessError::InternalInvariant);
        }
        remaining = 0;
    }

    Ok(allocations)
}

fn jain_index_ppm(values: &[f64]) -> Option<u32> {
    if values.is_empty() {
        return None;
    }
    let sum = values.iter().sum::<f64>();
    let sum_squares = values.iter().map(|value| value * value).sum::<f64>();
    if sum_squares == 0.0 {
        return Some(1_000_000);
    }
    let index = (sum * sum) / (values.len() as f64 * sum_squares);
    Some((index.clamp(0.0, 1.0) * 1_000_000.0).round() as u32)
}

/// Allocate one shared bottleneck tenant-first, then flow-within-tenant.
///
/// A tenant opening more flows does not receive additional aggregate entitlement:
/// the first water-fill is over unique tenant principals, not transport flows.
pub fn allocate_tenant_weighted_max_min(
    capacity_bps: u64,
    flows: &[FlowDemand],
) -> Result<FairnessReport, FairnessError> {
    if capacity_bps == 0 {
        return Err(FairnessError::ZeroCapacity);
    }
    if flows.is_empty() {
        return Err(FairnessError::EmptyDemand);
    }
    if flows.len() > MAX_FLOWS {
        return Err(FairnessError::TooManyFlows);
    }

    let mut seen_flow_ids = BTreeSet::new();
    let mut tenants: BTreeMap<String, Vec<&FlowDemand>> = BTreeMap::new();
    let mut tenant_weights = BTreeMap::new();

    for flow in flows {
        if flow.flow_id.trim().is_empty() {
            return Err(FairnessError::EmptyFlowId);
        }
        if flow.tenant_id.trim().is_empty() {
            return Err(FairnessError::EmptyTenantId);
        }
        if !seen_flow_ids.insert(flow.flow_id.clone()) {
            return Err(FairnessError::DuplicateFlowId);
        }
        if flow.tenant_weight == 0 || flow.flow_weight == 0 {
            return Err(FairnessError::InvalidWeight);
        }
        if flow.minimum_bps > flow.demand_bps {
            return Err(FairnessError::MinimumExceedsDemand);
        }

        if let Some(previous) = tenant_weights.get(&flow.tenant_id) {
            if *previous != flow.tenant_weight {
                return Err(FairnessError::InconsistentTenantWeight);
            }
        } else {
            tenant_weights.insert(flow.tenant_id.clone(), flow.tenant_weight);
        }
        tenants
            .entry(flow.tenant_id.clone())
            .or_default()
            .push(flow);
    }

    let mut tenant_demands = Vec::with_capacity(tenants.len());
    for (tenant_id, tenant_flows) in &tenants {
        tenant_demands.push(ShareDemand {
            id: tenant_id.clone(),
            weight: *tenant_weights
                .get(tenant_id)
                .ok_or(FairnessError::InternalInvariant)?,
            demand_bps: checked_sum(tenant_flows.iter().map(|flow| flow.demand_bps))?,
            minimum_bps: checked_sum(tenant_flows.iter().map(|flow| flow.minimum_bps))?,
        });
    }

    let tenant_allocations_by_id = weighted_max_min(capacity_bps, &tenant_demands)?;
    let mut flow_allocations = Vec::with_capacity(flows.len());
    let mut tenant_allocations = Vec::with_capacity(tenants.len());

    for (tenant_id, tenant_flows) in &tenants {
        let tenant_budget = *tenant_allocations_by_id
            .get(tenant_id)
            .ok_or(FairnessError::InternalInvariant)?;
        let flow_demands = tenant_flows
            .iter()
            .map(|flow| ShareDemand {
                id: flow.flow_id.clone(),
                weight: flow.flow_weight,
                demand_bps: flow.demand_bps,
                minimum_bps: flow.minimum_bps,
            })
            .collect::<Vec<_>>();
        let flow_allocations_by_id = weighted_max_min(tenant_budget, &flow_demands)?;

        let allocated = checked_sum(
            tenant_flows
                .iter()
                .map(|flow| *flow_allocations_by_id.get(&flow.flow_id).unwrap_or(&0)),
        )?;
        let demand = checked_sum(tenant_flows.iter().map(|flow| flow.demand_bps))?;
        let minimum = checked_sum(tenant_flows.iter().map(|flow| flow.minimum_bps))?;

        tenant_allocations.push(TenantRateAllocation {
            tenant_id: tenant_id.clone(),
            weight: *tenant_weights
                .get(tenant_id)
                .ok_or(FairnessError::InternalInvariant)?,
            demand_bps: demand,
            minimum_bps: minimum,
            allocated_bps: allocated,
            active_flow_count: u32::try_from(tenant_flows.len())
                .map_err(|_| FairnessError::ArithmeticOverflow)?,
        });

        for flow in tenant_flows {
            flow_allocations.push(FlowRateAllocation {
                flow_id: flow.flow_id.clone(),
                tenant_id: flow.tenant_id.clone(),
                demand_bps: flow.demand_bps,
                minimum_bps: flow.minimum_bps,
                allocated_bps: *flow_allocations_by_id
                    .get(&flow.flow_id)
                    .ok_or(FairnessError::InternalInvariant)?,
            });
        }
    }

    flow_allocations.sort_by(|left, right| {
        left.tenant_id
            .cmp(&right.tenant_id)
            .then_with(|| left.flow_id.cmp(&right.flow_id))
    });
    tenant_allocations.sort_by(|left, right| left.tenant_id.cmp(&right.tenant_id));

    let allocated_bps = checked_sum(tenant_allocations.iter().map(|item| item.allocated_bps))?;
    if allocated_bps > capacity_bps {
        return Err(FairnessError::InternalInvariant);
    }

    // Fairness indices consider only backlogged principals. Demand-limited
    // tenants/flows should not be scored as unfair merely because they did not
    // consume capacity they did not request.
    let tenant_weighted_rates = tenant_allocations
        .iter()
        .filter(|item| item.allocated_bps < item.demand_bps)
        .map(|item| f64::from(item.allocated_bps) / f64::from(item.weight))
        .collect::<Vec<_>>();
    let flow_weights: BTreeMap<&str, u32> = flows
        .iter()
        .map(|flow| (flow.flow_id.as_str(), flow.flow_weight))
        .collect();
    let mut per_tenant_flow_fairness = Vec::with_capacity(tenants.len());
    for tenant in &tenant_allocations {
        let weighted_rates = flow_allocations
            .iter()
            .filter(|item| {
                item.tenant_id == tenant.tenant_id && item.allocated_bps < item.demand_bps
            })
            .map(|item| {
                let weight = *flow_weights.get(item.flow_id.as_str()).unwrap_or(&1);
                f64::from(item.allocated_bps) / f64::from(weight)
            })
            .collect::<Vec<_>>();
        per_tenant_flow_fairness.push(TenantFlowFairness {
            tenant_id: tenant.tenant_id.clone(),
            backlogged_flow_count: u32::try_from(weighted_rates.len())
                .map_err(|_| FairnessError::ArithmeticOverflow)?,
            weighted_jain_index_ppm: jain_index_ppm(&weighted_rates),
        });
    }
    let worst_tenant_flow_weighted_jain_index_ppm = per_tenant_flow_fairness
        .iter()
        .filter(|item| item.backlogged_flow_count >= 2)
        .filter_map(|item| item.weighted_jain_index_ppm)
        .min();

    Ok(FairnessReport {
        capacity_bps,
        allocated_bps,
        unused_capacity_bps: capacity_bps - allocated_bps,
        tenant_weighted_jain_index_ppm: jain_index_ppm(&tenant_weighted_rates),
        worst_tenant_flow_weighted_jain_index_ppm,
        per_tenant_flow_fairness,
        tenant_allocations,
        flow_allocations,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flow(
        flow_id: &str,
        tenant_id: &str,
        tenant_weight: u32,
        flow_weight: u32,
        demand_bps: u64,
        minimum_bps: u64,
    ) -> FlowDemand {
        FlowDemand {
            flow_id: flow_id.to_owned(),
            tenant_id: tenant_id.to_owned(),
            tenant_weight,
            flow_weight,
            demand_bps,
            minimum_bps,
        }
    }

    #[test]
    fn equal_weight_tenants_receive_equal_capacity() {
        let result = allocate_tenant_weighted_max_min(
            100,
            &[
                flow("a-1", "tenant-a", 1, 1, 100, 0),
                flow("b-1", "tenant-b", 1, 1, 100, 0),
            ],
        )
        .expect("feasible tenant demands");
        assert_eq!(result.allocated_bps, 100);
        assert_eq!(result.tenant_allocations[0].allocated_bps, 50);
        assert_eq!(result.tenant_allocations[1].allocated_bps, 50);
        assert_eq!(result.tenant_weighted_jain_index_ppm, Some(1_000_000));
    }

    #[test]
    fn opening_more_flows_does_not_increase_tenant_entitlement() {
        let one_flow = allocate_tenant_weighted_max_min(
            100,
            &[
                flow("a-1", "tenant-a", 1, 1, 100, 0),
                flow("b-1", "tenant-b", 1, 1, 100, 0),
            ],
        )
        .expect("feasible allocation");
        let many_flows = allocate_tenant_weighted_max_min(
            100,
            &[
                flow("a-1", "tenant-a", 1, 1, 100, 0),
                flow("a-2", "tenant-a", 1, 1, 100, 0),
                flow("a-3", "tenant-a", 1, 1, 100, 0),
                flow("a-4", "tenant-a", 1, 1, 100, 0),
                flow("a-5", "tenant-a", 1, 1, 100, 0),
                flow("a-6", "tenant-a", 1, 1, 100, 0),
                flow("b-1", "tenant-b", 1, 1, 100, 0),
            ],
        )
        .expect("feasible allocation");

        let a_one = one_flow
            .tenant_allocations
            .iter()
            .find(|tenant| tenant.tenant_id == "tenant-a")
            .expect("tenant a")
            .allocated_bps;
        let a_many = many_flows
            .tenant_allocations
            .iter()
            .find(|tenant| tenant.tenant_id == "tenant-a")
            .expect("tenant a")
            .allocated_bps;
        let b_many = many_flows
            .tenant_allocations
            .iter()
            .find(|tenant| tenant.tenant_id == "tenant-b")
            .expect("tenant b")
            .allocated_bps;
        assert_eq!(a_one, 50);
        assert_eq!(a_many, 50);
        assert_eq!(b_many, 50);
    }

    #[test]
    fn tenant_weights_produce_weighted_max_min_shares() {
        let result = allocate_tenant_weighted_max_min(
            70,
            &[
                flow("a", "tenant-a", 1, 1, 100, 0),
                flow("b", "tenant-b", 2, 1, 100, 0),
                flow("c", "tenant-c", 4, 1, 100, 0),
            ],
        )
        .expect("feasible weighted allocation");
        let shares = result
            .tenant_allocations
            .iter()
            .map(|tenant| (tenant.tenant_id.as_str(), tenant.allocated_bps))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(shares["tenant-a"], 10);
        assert_eq!(shares["tenant-b"], 20);
        assert_eq!(shares["tenant-c"], 40);
        assert_eq!(result.tenant_weighted_jain_index_ppm, Some(1_000_000));
    }

    #[test]
    fn flow_weights_are_applied_inside_each_tenant() {
        let result = allocate_tenant_weighted_max_min(
            100,
            &[
                flow("a-1", "tenant-a", 1, 1, 100, 0),
                flow("a-2", "tenant-a", 1, 3, 100, 0),
                flow("b-1", "tenant-b", 1, 1, 100, 0),
            ],
        )
        .expect("feasible allocation");
        let shares = result
            .flow_allocations
            .iter()
            .map(|flow| (flow.flow_id.as_str(), flow.allocated_bps))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(shares["a-1"], 13);
        assert_eq!(shares["a-2"], 37);
        assert_eq!(shares["b-1"], 50);
    }

    #[test]
    fn flow_fairness_is_measured_within_each_tenant_budget() {
        let result = allocate_tenant_weighted_max_min(
            100,
            &[
                flow("a-1", "tenant-a", 1, 1, 100, 0),
                flow("a-2", "tenant-a", 1, 1, 100, 0),
                flow("b-1", "tenant-b", 1, 1, 100, 0),
            ],
        )
        .expect("feasible allocation");
        assert_eq!(result.tenant_weighted_jain_index_ppm, Some(1_000_000));
        assert_eq!(
            result.worst_tenant_flow_weighted_jain_index_ppm,
            Some(1_000_000)
        );
        assert_eq!(result.per_tenant_flow_fairness.len(), 2);
        assert!(result
            .per_tenant_flow_fairness
            .iter()
            .all(|item| item.weighted_jain_index_ppm == Some(1_000_000)));
    }

    #[test]
    fn minimum_service_floors_are_honoured_when_feasible() {
        let result = allocate_tenant_weighted_max_min(
            100,
            &[
                flow("a", "tenant-a", 1, 1, 100, 30),
                flow("b", "tenant-b", 1, 1, 100, 20),
            ],
        )
        .expect("floors fit");
        assert_eq!(result.flow_allocations[0].minimum_bps, 30);
        assert_eq!(result.flow_allocations[0].allocated_bps, 55);
        assert_eq!(result.flow_allocations[1].allocated_bps, 45);
    }

    #[test]
    fn impossible_minimum_floors_fail_closed() {
        assert_eq!(
            allocate_tenant_weighted_max_min(
                100,
                &[
                    flow("a", "tenant-a", 1, 1, 100, 60),
                    flow("b", "tenant-b", 1, 1, 100, 60),
                ],
            )
            .unwrap_err(),
            FairnessError::MinimumFloorsExceedCapacity
        );
    }

    #[test]
    fn unused_capacity_is_not_fabricated_when_demand_is_low() {
        let result = allocate_tenant_weighted_max_min(
            100,
            &[
                flow("a", "tenant-a", 1, 1, 20, 0),
                flow("b", "tenant-b", 1, 1, 30, 0),
            ],
        )
        .expect("demand fits");
        assert_eq!(result.allocated_bps, 50);
        assert_eq!(result.unused_capacity_bps, 50);
    }

    #[test]
    fn identical_inputs_are_independent_of_flow_order() {
        let inputs = vec![
            flow("b-1", "tenant-b", 1, 1, 100, 0),
            flow("a-2", "tenant-a", 1, 1, 100, 0),
            flow("a-1", "tenant-a", 1, 1, 100, 0),
        ];
        let first = allocate_tenant_weighted_max_min(101, &inputs).expect("feasible");
        let second = allocate_tenant_weighted_max_min(
            101,
            &inputs.into_iter().rev().collect::<Vec<_>>(),
        )
        .expect("feasible");
        assert_eq!(first, second);
    }

    #[test]
    fn inconsistent_tenant_weights_are_rejected() {
        assert_eq!(
            allocate_tenant_weighted_max_min(
                100,
                &[
                    flow("a-1", "tenant-a", 1, 1, 100, 0),
                    flow("a-2", "tenant-a", 2, 1, 100, 0),
                ],
            )
            .unwrap_err(),
            FairnessError::InconsistentTenantWeight
        );
    }

    #[test]
    fn demand_limited_tenants_are_excluded_from_backlogged_fairness_index() {
        let result = allocate_tenant_weighted_max_min(
            100,
            &[
                flow("a", "tenant-a", 1, 1, 10, 0),
                flow("b", "tenant-b", 1, 1, 100, 0),
            ],
        )
        .expect("feasible allocation");
        assert_eq!(result.unused_capacity_bps, 0);
        assert_eq!(result.tenant_allocations[0].allocated_bps, 10);
        assert_eq!(result.tenant_allocations[1].allocated_bps, 90);
        assert_eq!(result.tenant_weighted_jain_index_ppm, Some(1_000_000));
    }

    #[test]
    fn duplicate_flow_ids_are_rejected() {
        assert_eq!(
            allocate_tenant_weighted_max_min(
                100,
                &[
                    flow("same", "tenant-a", 1, 1, 100, 0),
                    flow("same", "tenant-b", 1, 1, 100, 0),
                ],
            )
            .unwrap_err(),
            FairnessError::DuplicateFlowId
        );
    }

    #[test]
    fn weighted_flow_rounding_uses_stable_largest_remainder() {
        let forward = vec![
            flow("a-1", "tenant-a", 1, 1, 100, 0),
            flow("a-2", "tenant-a", 1, 3, 100, 0),
            flow("b-1", "tenant-b", 1, 1, 100, 0),
        ];
        let forward_report =
            allocate_tenant_weighted_max_min(100, &forward).expect("feasible allocation");
        let reverse_report = allocate_tenant_weighted_max_min(
            100,
            &forward.into_iter().rev().collect::<Vec<_>>(),
        )
        .expect("feasible allocation");
        assert_eq!(forward_report, reverse_report);
        let shares = forward_report
            .flow_allocations
            .iter()
            .map(|item| (item.flow_id.as_str(), item.allocated_bps))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(shares["a-1"], 13);
        assert_eq!(shares["a-2"], 37);
        assert_eq!(shares["b-1"], 50);
    }

    #[test]
    fn tenant_demand_sum_overflow_is_rejected() {
        assert_eq!(
            allocate_tenant_weighted_max_min(
                u64::MAX,
                &[
                    flow("a-1", "tenant-a", 1, 1, u64::MAX, 0),
                    flow("a-2", "tenant-a", 1, 1, u64::MAX, 0),
                ],
            )
            .unwrap_err(),
            FairnessError::ArithmeticOverflow
        );
    }

    #[test]
    fn flow_floor_above_demand_is_rejected() {
        assert_eq!(
            allocate_tenant_weighted_max_min(
                100,
                &[flow("a", "tenant-a", 1, 1, 10, 11)],
            )
            .unwrap_err(),
            FairnessError::MinimumExceedsDemand
        );
    }

    #[test]
    fn zero_weights_are_rejected() {
        assert_eq!(
            allocate_tenant_weighted_max_min(
                100,
                &[flow("a", "tenant-a", 0, 1, 100, 0)],
            )
            .unwrap_err(),
            FairnessError::InvalidWeight
        );
    }

    #[test]
    fn service_floor_may_equal_the_entire_capacity() {
        let result = allocate_tenant_weighted_max_min(
            100,
            &[flow("a", "tenant-a", 1, 1, 100, 100)],
        )
        .expect("floor equals capacity");
        assert_eq!(result.allocated_bps, 100);
        assert_eq!(result.unused_capacity_bps, 0);
    }
}
