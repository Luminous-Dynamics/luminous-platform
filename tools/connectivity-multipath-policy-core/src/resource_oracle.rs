//! Exact, bounded weighted max-min reference allocator for shared resource incidence.
//!
//! Each flow is assigned to one fixed route represented by the set of resource
//! domains it consumes. Every resource has one hard capacity ceiling. Tenant
//! weights are normalized across a tenant's currently active flows so creating
//! more flows does not increase the tenant's aggregate entitlement.
//!
//! This is a small-topology reference oracle, not a production multi-commodity
//! solver, route selector or packet scheduler. It uses checked rational
//! arithmetic and fails closed on bounds or overflow.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

const MAX_RESOURCES: usize = 64;
const MAX_FLOWS: usize = 128;
const MAX_RESOURCES_PER_FLOW: usize = 32;
const MAX_ID_BYTES: usize = 128;
const MAX_TENANT_WEIGHT: u32 = 1_000_000;
const MAX_FLOW_WEIGHT: u32 = 1_000_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExactRate {
    pub numerator_bps: u128,
    pub denominator: u128,
}

impl ExactRate {
    fn from_bps(value: u64) -> Self {
        Self {
            numerator_bps: u128::from(value),
            denominator: 1,
        }
    }

    fn new(numerator_bps: u128, denominator: u128) -> Result<Self, ResourceOracleError> {
        let value = Fraction::new(numerator_bps, denominator)?;
        Ok(Self {
            numerator_bps: value.numerator,
            denominator: value.denominator,
        })
    }

    pub fn floor_bps(self) -> Result<u64, ResourceOracleError> {
        u64::try_from(self.numerator_bps / self.denominator)
            .map_err(|_| ResourceOracleError::ArithmeticOverflow)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Fraction {
    numerator: u128,
    denominator: u128,
}

impl Fraction {
    const ZERO: Self = Self {
        numerator: 0,
        denominator: 1,
    };

    fn new(numerator: u128, denominator: u128) -> Result<Self, ResourceOracleError> {
        if denominator == 0 {
            return Err(ResourceOracleError::DivisionByZero);
        }
        if numerator == 0 {
            return Ok(Self::ZERO);
        }
        let divisor = gcd(numerator, denominator);
        Ok(Self {
            numerator: numerator / divisor,
            denominator: denominator / divisor,
        })
    }

    fn from_u64(value: u64) -> Self {
        Self {
            numerator: u128::from(value),
            denominator: 1,
        }
    }

    fn is_zero(self) -> bool {
        self.numerator == 0
    }

    fn checked_add(self, other: Self) -> Result<Self, ResourceOracleError> {
        let common = gcd(self.denominator, other.denominator);
        let left_scale = other.denominator / common;
        let right_scale = self.denominator / common;
        let left = self
            .numerator
            .checked_mul(left_scale)
            .ok_or(ResourceOracleError::ArithmeticOverflow)?;
        let right = other
            .numerator
            .checked_mul(right_scale)
            .ok_or(ResourceOracleError::ArithmeticOverflow)?;
        let numerator = left
            .checked_add(right)
            .ok_or(ResourceOracleError::ArithmeticOverflow)?;
        let denominator = self
            .denominator
            .checked_mul(left_scale)
            .ok_or(ResourceOracleError::ArithmeticOverflow)?;
        Self::new(numerator, denominator)
    }

    fn checked_sub(self, other: Self) -> Result<Self, ResourceOracleError> {
        if self.checked_cmp(other)? == Ordering::Less {
            return Err(ResourceOracleError::InternalInvariant);
        }
        let common = gcd(self.denominator, other.denominator);
        let left_scale = other.denominator / common;
        let right_scale = self.denominator / common;
        let left = self
            .numerator
            .checked_mul(left_scale)
            .ok_or(ResourceOracleError::ArithmeticOverflow)?;
        let right = other
            .numerator
            .checked_mul(right_scale)
            .ok_or(ResourceOracleError::ArithmeticOverflow)?;
        let numerator = left
            .checked_sub(right)
            .ok_or(ResourceOracleError::InternalInvariant)?;
        let denominator = self
            .denominator
            .checked_mul(left_scale)
            .ok_or(ResourceOracleError::ArithmeticOverflow)?;
        Self::new(numerator, denominator)
    }

    fn checked_mul(self, other: Self) -> Result<Self, ResourceOracleError> {
        let left_cancel = gcd(self.numerator, other.denominator);
        let right_cancel = gcd(other.numerator, self.denominator);
        let numerator = (self.numerator / left_cancel)
            .checked_mul(other.numerator / right_cancel)
            .ok_or(ResourceOracleError::ArithmeticOverflow)?;
        let denominator = (self.denominator / right_cancel)
            .checked_mul(other.denominator / left_cancel)
            .ok_or(ResourceOracleError::ArithmeticOverflow)?;
        Self::new(numerator, denominator)
    }

    fn checked_div(self, other: Self) -> Result<Self, ResourceOracleError> {
        if other.is_zero() {
            return Err(ResourceOracleError::DivisionByZero);
        }
        let reciprocal = Self::new(other.denominator, other.numerator)?;
        self.checked_mul(reciprocal)
    }

    fn checked_cmp(self, other: Self) -> Result<Ordering, ResourceOracleError> {
        let common_numerator = gcd(self.numerator, other.numerator);
        let common_denominator = gcd(self.denominator, other.denominator);
        let left_numerator = self.numerator / common_numerator.max(1);
        let right_numerator = other.numerator / common_numerator.max(1);
        let left_denominator = self.denominator / common_denominator;
        let right_denominator = other.denominator / common_denominator;
        let left = left_numerator
            .checked_mul(right_denominator)
            .ok_or(ResourceOracleError::ArithmeticOverflow)?;
        let right = right_numerator
            .checked_mul(left_denominator)
            .ok_or(ResourceOracleError::ArithmeticOverflow)?;
        Ok(left.cmp(&right))
    }

    fn to_exact_rate(self) -> ExactRate {
        ExactRate {
            numerator_bps: self.numerator,
            denominator: self.denominator,
        }
    }
}

fn gcd(mut left: u128, mut right: u128) -> u128 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left.max(1)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceCapacity {
    pub resource_id: String,
    pub capacity_bps: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceFlowDemand {
    pub flow_id: String,
    pub tenant_id: String,
    /// Stable tenant entitlement weight, shared by every flow of this tenant.
    pub tenant_weight: u32,
    /// Relative weight within this tenant's currently active flows.
    pub flow_weight: u32,
    pub demand_bps: u64,
    pub minimum_bps: u64,
    /// Fixed-route resource incidence: every listed resource constrains this flow.
    pub resource_ids: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FlowAllocation {
    pub flow_id: String,
    pub tenant_id: String,
    pub allocated: ExactRate,
    pub demand_bps: u64,
    pub minimum_bps: u64,
    pub resource_ids: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceUsage {
    pub resource_id: String,
    pub capacity_bps: u64,
    pub allocated: ExactRate,
    pub saturated: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceAllocationReport {
    pub flow_allocations: Vec<FlowAllocation>,
    pub resource_usage: Vec<ResourceUsage>,
    pub active_bottleneck_resources: Vec<String>,
    /// Number of progressive-filling rounds. Each round freezes at least one flow.
    pub progressive_filling_rounds: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResourceOracleError {
    EmptyResourceSet,
    TooManyResources,
    InvalidResourceId,
    DuplicateResourceId,
    ZeroCapacityNotAllowed,
    EmptyFlowSet,
    TooManyFlows,
    EmptyFlowId,
    DuplicateFlowId,
    EmptyTenantId,
    InvalidTenantWeight,
    InconsistentTenantWeight,
    InvalidFlowWeight,
    MinimumExceedsDemand,
    EmptyFlowResourceSet,
    TooManyResourcesPerFlow,
    InvalidFlowResourceId,
    DuplicateFlowResourceId,
    UnknownResource,
    MinimumFloorsExceedCapacity,
    DivisionByZero,
    ArithmeticOverflow,
    InternalInvariant,
}

#[derive(Clone, Debug)]
struct ValidatedFlow {
    demand: ResourceFlowDemand,
    resource_indices: Vec<usize>,
}

fn validate_id(value: &str) -> bool {
    !value.trim().is_empty()
        && value.len() <= MAX_ID_BYTES
        && !value.chars().any(char::is_control)
}

fn validate_inputs(
    resources: &[ResourceCapacity],
    flows: &[ResourceFlowDemand],
) -> Result<(Vec<ValidatedFlow>, Vec<ResourceCapacity>), ResourceOracleError> {
    if resources.is_empty() {
        return Err(ResourceOracleError::EmptyResourceSet);
    }
    if resources.len() > MAX_RESOURCES {
        return Err(ResourceOracleError::TooManyResources);
    }
    if flows.is_empty() {
        return Err(ResourceOracleError::EmptyFlowSet);
    }
    if flows.len() > MAX_FLOWS {
        return Err(ResourceOracleError::TooManyFlows);
    }

    let mut sorted_resources = resources.to_vec();
    sorted_resources.sort_by(|left, right| left.resource_id.cmp(&right.resource_id));
    let mut resource_index = BTreeMap::new();
    for (index, resource) in sorted_resources.iter().enumerate() {
        if !validate_id(&resource.resource_id) {
            return Err(ResourceOracleError::InvalidResourceId);
        }
        if resource.capacity_bps == 0 {
            return Err(ResourceOracleError::ZeroCapacityNotAllowed);
        }
        if resource_index
            .insert(resource.resource_id.as_str(), index)
            .is_some()
        {
            return Err(ResourceOracleError::DuplicateResourceId);
        }
    }

    let mut sorted_flows = flows.to_vec();
    sorted_flows.sort_by(|left, right| {
        left.tenant_id
            .cmp(&right.tenant_id)
            .then_with(|| left.flow_id.cmp(&right.flow_id))
    });
    let mut seen_flow_ids = BTreeSet::new();
    let mut tenant_weights = BTreeMap::new();
    let mut validated = Vec::with_capacity(sorted_flows.len());

    for flow in sorted_flows {
        if !validate_id(&flow.flow_id) {
            return Err(ResourceOracleError::EmptyFlowId);
        }
        if !seen_flow_ids.insert(flow.flow_id.clone()) {
            return Err(ResourceOracleError::DuplicateFlowId);
        }
        if !validate_id(&flow.tenant_id) {
            return Err(ResourceOracleError::EmptyTenantId);
        }
        if flow.tenant_weight == 0 || flow.tenant_weight > MAX_TENANT_WEIGHT {
            return Err(ResourceOracleError::InvalidTenantWeight);
        }
        if flow.flow_weight == 0 || flow.flow_weight > MAX_FLOW_WEIGHT {
            return Err(ResourceOracleError::InvalidFlowWeight);
        }
        if flow.minimum_bps > flow.demand_bps {
            return Err(ResourceOracleError::MinimumExceedsDemand);
        }
        if let Some(previous) = tenant_weights.get(&flow.tenant_id) {
            if *previous != flow.tenant_weight {
                return Err(ResourceOracleError::InconsistentTenantWeight);
            }
        } else {
            tenant_weights.insert(flow.tenant_id.clone(), flow.tenant_weight);
        }
        if flow.resource_ids.is_empty() {
            return Err(ResourceOracleError::EmptyFlowResourceSet);
        }
        if flow.resource_ids.len() > MAX_RESOURCES_PER_FLOW {
            return Err(ResourceOracleError::TooManyResourcesPerFlow);
        }

        let mut ids = flow.resource_ids.clone();
        ids.sort();
        if ids.iter().any(|id| !validate_id(id)) {
            return Err(ResourceOracleError::InvalidFlowResourceId);
        }
        if ids.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(ResourceOracleError::DuplicateFlowResourceId);
        }
        let mut indices = Vec::with_capacity(ids.len());
        for id in &ids {
            let index = resource_index
                .get(id.as_str())
                .copied()
                .ok_or(ResourceOracleError::UnknownResource)?;
            indices.push(index);
        }
        let mut canonical_flow = flow;
        canonical_flow.resource_ids = ids;
        validated.push(ValidatedFlow {
            demand: canonical_flow,
            resource_indices: indices,
        });
    }
    Ok((validated, sorted_resources))
}

/// Allocate fixed-route flows by tenant-normalized weighted progressive filling.
///
/// At each round, every still-active tenant receives its tenant weight. That
/// entitlement is divided among the tenant's currently active flows according
/// to their flow weights. The next round ends at the first flow demand or shared
/// resource ceiling. Flows using a saturated resource freeze; remaining flows
/// are re-normalized within their tenant and the residual capacity is shared.
///
/// This yields an exact rational weighted max-min allocation for the supplied
/// fixed routes and declared resource incidence matrix.
pub fn allocate_exact_weighted_max_min(
    resources: &[ResourceCapacity],
    flows: &[ResourceFlowDemand],
) -> Result<ResourceAllocationReport, ResourceOracleError> {
    let (validated, resources) = validate_inputs(resources, flows)?;
    let mut allocations = validated
        .iter()
        .map(|flow| Fraction::from_u64(flow.demand.minimum_bps))
        .collect::<Vec<_>>();
    let capacities = resources
        .iter()
        .map(|resource| Fraction::from_u64(resource.capacity_bps))
        .collect::<Vec<_>>();
    let mut usage = vec![Fraction::ZERO; resources.len()];

    for (flow_index, flow) in validated.iter().enumerate() {
        for resource_index in &flow.resource_indices {
            usage[*resource_index] = usage[*resource_index]
                .checked_add(allocations[flow_index])?;
        }
    }
    for (resource_index, used) in usage.iter().enumerate() {
        if used.checked_cmp(capacities[resource_index])? == Ordering::Greater {
            return Err(ResourceOracleError::MinimumFloorsExceedCapacity);
        }
    }

    let mut active = validated
        .iter()
        .enumerate()
        .filter_map(|(index, flow)| {
            (flow.demand.minimum_bps < flow.demand.demand_bps).then_some(index)
        })
        .collect::<BTreeSet<_>>();
    let mut rounds = 0_u32;

    while !active.is_empty() {
        rounds = rounds
            .checked_add(1)
            .ok_or(ResourceOracleError::ArithmeticOverflow)?;
        if rounds > u32::try_from(MAX_FLOWS).unwrap_or(u32::MAX) {
            return Err(ResourceOracleError::InternalInvariant);
        }

        let mut active_flow_weight_sums: BTreeMap<&str, u128> = BTreeMap::new();
        for flow_index in &active {
            let flow = &validated[*flow_index].demand;
            let total = active_flow_weight_sums
                .entry(flow.tenant_id.as_str())
                .or_insert(0);
            *total = total
                .checked_add(u128::from(flow.flow_weight))
                .ok_or(ResourceOracleError::ArithmeticOverflow)?;
        }

        let mut effective_weights = BTreeMap::new();
        for flow_index in &active {
            let flow = &validated[*flow_index].demand;
            let total_flow_weight = *active_flow_weight_sums
                .get(flow.tenant_id.as_str())
                .ok_or(ResourceOracleError::InternalInvariant)?;
            let numerator = u128::from(flow.tenant_weight)
                .checked_mul(u128::from(flow.flow_weight))
                .ok_or(ResourceOracleError::ArithmeticOverflow)?;
            let weight = Fraction::new(numerator, total_flow_weight)?;
            effective_weights.insert(*flow_index, weight);
        }

        let mut delta: Option<Fraction> = None;
        for (resource_index, capacity) in capacities.iter().enumerate() {
            let mut slope = Fraction::ZERO;
            for flow_index in &active {
                if validated[*flow_index]
                    .resource_indices
                    .contains(&resource_index)
                {
                    slope = slope.checked_add(effective_weights[flow_index])?;
                }
            }
            if slope.is_zero() {
                continue;
            }
            let residual = capacity.checked_sub(usage[resource_index])?;
            let candidate = residual.checked_div(slope)?;
            delta = Some(match delta {
                Some(current) if current.checked_cmp(candidate)? != Ordering::Greater => current,
                _ => candidate,
            });
        }

        for flow_index in &active {
            let flow = &validated[*flow_index].demand;
            let demand_remaining =
                Fraction::from_u64(flow.demand_bps).checked_sub(allocations[*flow_index])?;
            let candidate = demand_remaining.checked_div(effective_weights[flow_index])?;
            delta = Some(match delta {
                Some(current) if current.checked_cmp(candidate)? != Ordering::Greater => current,
                _ => candidate,
            });
        }

        let delta = delta.ok_or(ResourceOracleError::InternalInvariant)?;
        if delta.is_zero() {
            let saturated_resources = usage
                .iter()
                .enumerate()
                .filter_map(|(index, used)| {
                    (used.checked_cmp(capacities[index]).ok() == Some(Ordering::Equal))
                        .then_some(index)
                })
                .collect::<BTreeSet<_>>();
            let before = active.len();
            active.retain(|flow_index| {
                !validated[*flow_index]
                    .resource_indices
                    .iter()
                    .any(|resource_index| saturated_resources.contains(resource_index))
            });
            if active.len() == before {
                return Err(ResourceOracleError::InternalInvariant);
            }
            continue;
        }

        let mut increments = BTreeMap::new();
        for flow_index in &active {
            let increment = effective_weights[flow_index].checked_mul(delta)?;
            allocations[*flow_index] = allocations[*flow_index].checked_add(increment)?;
            increments.insert(*flow_index, increment);
        }
        for (flow_index, increment) in increments {
            for resource_index in &validated[flow_index].resource_indices {
                usage[*resource_index] = usage[*resource_index].checked_add(increment)?;
                if usage[*resource_index].checked_cmp(capacities[*resource_index])?
                    == Ordering::Greater
                {
                    return Err(ResourceOracleError::InternalInvariant);
                }
            }
        }

        let saturated_resources = usage
            .iter()
            .enumerate()
            .filter_map(|(index, used)| {
                (used.checked_cmp(capacities[index]).ok() == Some(Ordering::Equal))
                    .then_some(index)
            })
            .collect::<BTreeSet<_>>();
        let before = active.len();
        active.retain(|flow_index| {
            let demand_reached = allocations[*flow_index]
                .checked_cmp(Fraction::from_u64(validated[*flow_index].demand.demand_bps))
                .ok()
                == Some(Ordering::Equal);
            let resource_saturated = validated[*flow_index]
                .resource_indices
                .iter()
                .any(|resource_index| saturated_resources.contains(resource_index));
            !demand_reached && !resource_saturated
        });
        if active.len() == before {
            return Err(ResourceOracleError::InternalInvariant);
        }
    }

    let mut flow_allocations = validated
        .iter()
        .enumerate()
        .map(|(index, flow)| FlowAllocation {
            flow_id: flow.demand.flow_id.clone(),
            tenant_id: flow.demand.tenant_id.clone(),
            allocated: allocations[index].to_exact_rate(),
            demand_bps: flow.demand.demand_bps,
            minimum_bps: flow.demand.minimum_bps,
            resource_ids: flow.demand.resource_ids.clone(),
        })
        .collect::<Vec<_>>();
    flow_allocations.sort_by(|left, right| {
        left.tenant_id
            .cmp(&right.tenant_id)
            .then_with(|| left.flow_id.cmp(&right.flow_id))
    });

    let mut resource_usage = Vec::with_capacity(resources.len());
    let mut active_bottleneck_resources = Vec::new();
    for (index, resource) in resources.iter().enumerate() {
        let saturated = usage[index].checked_cmp(capacities[index])? == Ordering::Equal;
        if saturated {
            active_bottleneck_resources.push(resource.resource_id.clone());
        }
        resource_usage.push(ResourceUsage {
            resource_id: resource.resource_id.clone(),
            capacity_bps: resource.capacity_bps,
            allocated: usage[index].to_exact_rate(),
            saturated,
        });
    }

    Ok(ResourceAllocationReport {
        flow_allocations,
        resource_usage,
        active_bottleneck_resources,
        progressive_filling_rounds: rounds,
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

    fn flow(
        id: &str,
        tenant: &str,
        tenant_weight: u32,
        flow_weight: u32,
        demand_bps: u64,
        minimum_bps: u64,
        resources: &[&str],
    ) -> ResourceFlowDemand {
        ResourceFlowDemand {
            flow_id: id.to_owned(),
            tenant_id: tenant.to_owned(),
            tenant_weight,
            flow_weight,
            demand_bps,
            minimum_bps,
            resource_ids: resources.iter().map(|id| (*id).to_owned()).collect(),
        }
    }

    fn rate(numerator_bps: u128, denominator: u128) -> ExactRate {
        ExactRate::new(numerator_bps, denominator).expect("valid exact rate")
    }

    #[test]
    fn two_access_paths_share_one_backhaul_ceiling() {
        let result = allocate_exact_weighted_max_min(
            &[resource("access-a", 60), resource("access-b", 80), resource("backhaul", 100)],
            &[
                flow("flow-a", "tenant-a", 1, 1, 1_000, 0, &["access-a", "backhaul"]),
                flow("flow-b", "tenant-b", 1, 1, 1_000, 0, &["access-b", "backhaul"]),
            ],
        )
        .expect("feasible shared backhaul");
        assert_eq!(result.flow_allocations[0].allocated, rate(50, 1));
        assert_eq!(result.flow_allocations[1].allocated, rate(50, 1));
        assert_eq!(result.resource_usage[2].allocated, rate(100, 1));
        assert!(result.resource_usage[2].saturated);
    }

    #[test]
    fn opening_more_flows_does_not_increase_tenant_entitlement() {
        let result = allocate_exact_weighted_max_min(
            &[resource("backhaul", 90)],
            &[
                flow("a-1", "tenant-a", 1, 1, 100, 0, &["backhaul"]),
                flow("a-2", "tenant-a", 1, 1, 100, 0, &["backhaul"]),
                flow("a-3", "tenant-a", 1, 1, 100, 0, &["backhaul"]),
                flow("b-1", "tenant-b", 1, 1, 100, 0, &["backhaul"]),
            ],
        )
        .expect("feasible tenant fairness");
        let tenant_a = result
            .flow_allocations
            .iter()
            .filter(|item| item.tenant_id == "tenant-a")
            .map(|item| item.allocated.floor_bps().expect("bounded rate"))
            .sum::<u64>();
        let tenant_b = result
            .flow_allocations
            .iter()
            .find(|item| item.tenant_id == "tenant-b")
            .expect("tenant b flow")
            .allocated
            .floor_bps()
            .expect("bounded rate");
        assert_eq!(tenant_a, 45);
        assert_eq!(tenant_b, 45);
    }

    #[test]
    fn within_tenant_flow_weights_split_the_tenant_share() {
        let result = allocate_exact_weighted_max_min(
            &[resource("backhaul", 120)],
            &[
                flow("a-heavy", "tenant-a", 1, 2, 100, 0, &["backhaul"]),
                flow("a-light", "tenant-a", 1, 1, 100, 0, &["backhaul"]),
                flow("b-only", "tenant-b", 1, 1, 100, 0, &["backhaul"]),
            ],
        )
        .expect("feasible hierarchical weights");
        assert_eq!(
            result
                .flow_allocations
                .iter()
                .find(|item| item.flow_id == "a-heavy")
                .expect("heavy flow")
                .allocated,
            rate(40, 1)
        );
        assert_eq!(
            result
                .flow_allocations
                .iter()
                .find(|item| item.flow_id == "a-light")
                .expect("light flow")
                .allocated,
            rate(20, 1)
        );
        assert_eq!(
            result
                .flow_allocations
                .iter()
                .find(|item| item.flow_id == "b-only")
                .expect("tenant b flow")
                .allocated,
            rate(60, 1)
        );
    }

    #[test]
    fn disjoint_resources_are_not_constrained_by_one_another() {
        let result = allocate_exact_weighted_max_min(
            &[resource("r-a", 60), resource("r-b", 80)],
            &[
                flow("a", "tenant-a", 1, 1, 100, 0, &["r-a"]),
                flow("b", "tenant-b", 1, 1, 100, 0, &["r-b"]),
            ],
        )
        .expect("independent resources");
        assert_eq!(result.flow_allocations[0].allocated, rate(60, 1));
        assert_eq!(result.flow_allocations[1].allocated, rate(80, 1));
    }

    #[test]
    fn demand_limited_flow_releases_entitlement_to_other_tenants() {
        let result = allocate_exact_weighted_max_min(
            &[resource("r", 100)],
            &[
                flow("small", "tenant-a", 1, 1, 10, 0, &["r"]),
                flow("large", "tenant-b", 1, 1, 100, 0, &["r"]),
            ],
        )
        .expect("remaining demand absorbs capacity");
        assert_eq!(result.flow_allocations[0].allocated, rate(10, 1));
        assert_eq!(result.flow_allocations[1].allocated, rate(90, 1));
    }

    #[test]
    fn one_flow_freezes_at_its_private_bottleneck() {
        let result = allocate_exact_weighted_max_min(
            &[resource("private-a", 20), resource("shared", 100)],
            &[
                flow("a", "tenant-a", 1, 1, 100, 0, &["private-a", "shared"]),
                flow("b", "tenant-b", 1, 1, 100, 0, &["shared"]),
            ],
        )
        .expect("private bottleneck");
        assert_eq!(result.flow_allocations[0].allocated, rate(20, 1));
        assert_eq!(result.flow_allocations[1].allocated, rate(80, 1));
        assert_eq!(result.resource_usage[0].allocated, rate(20, 1));
        assert_eq!(result.resource_usage[1].allocated, rate(100, 1));
    }

    #[test]
    fn feasible_minimum_floors_are_satisfied_before_progressive_filling() {
        let result = allocate_exact_weighted_max_min(
            &[resource("r", 100)],
            &[
                flow("a", "tenant-a", 1, 1, 100, 20, &["r"]),
                flow("b", "tenant-b", 1, 1, 100, 10, &["r"]),
            ],
        )
        .expect("feasible floors");
        assert_eq!(result.flow_allocations[0].allocated, rate(55, 1));
        assert_eq!(result.flow_allocations[1].allocated, rate(45, 1));
    }

    #[test]
    fn infeasible_minimum_floors_are_rejected() {
        assert_eq!(
            allocate_exact_weighted_max_min(
                &[resource("r", 100)],
                &[
                    flow("a", "tenant-a", 1, 1, 100, 60, &["r"]),
                    flow("b", "tenant-b", 1, 1, 100, 50, &["r"]),
                ],
            )
            .unwrap_err(),
            ResourceOracleError::MinimumFloorsExceedCapacity
        );
    }

    #[test]
    fn resource_ids_must_be_known_and_unique_within_each_flow() {
        assert_eq!(
            allocate_exact_weighted_max_min(
                &[resource("r", 100)],
                &[flow("a", "tenant-a", 1, 1, 100, 0, &["missing"])],
            )
            .unwrap_err(),
            ResourceOracleError::UnknownResource
        );
        assert_eq!(
            allocate_exact_weighted_max_min(
                &[resource("r", 100)],
                &[flow("a", "tenant-a", 1, 1, 100, 0, &["r", "r"])],
            )
            .unwrap_err(),
            ResourceOracleError::DuplicateFlowResourceId
        );
    }

    #[test]
    fn permuting_resources_and_flows_is_deterministic() {
        let resources = vec![resource("a", 60), resource("b", 80), resource("shared", 100)];
        let flows = vec![
            flow("a", "tenant-a", 1, 1, 100, 0, &["a", "shared"]),
            flow("b", "tenant-b", 1, 1, 100, 0, &["b", "shared"]),
        ];
        let first = allocate_exact_weighted_max_min(&resources, &flows).expect("valid");
        let second = allocate_exact_weighted_max_min(
            &resources.into_iter().rev().collect::<Vec<_>>(),
            &flows.into_iter().rev().collect::<Vec<_>>(),
        )
        .expect("same inputs in reverse order");
        assert_eq!(first, second);
    }

    #[test]
    fn exact_rational_rates_preserve_non_integer_bits_per_second() {
        let result = allocate_exact_weighted_max_min(
            &[resource("r", 50)],
            &[
                flow("heavy", "tenant-a", 2, 1, 100, 0, &["r"]),
                flow("light", "tenant-b", 1, 1, 100, 0, &["r"]),
            ],
        )
        .expect("weighted exact allocation");
        assert_eq!(
            result.flow_allocations[0].allocated,
            rate(100, 3)
        );
        assert_eq!(
            result.flow_allocations[1].allocated,
            rate(50, 3)
        );
        assert_eq!(
            result.resource_usage[0].allocated,
            rate(50, 1)
        );
    }

    #[test]
    fn capacity_resource_zero_is_rejected_explicitly() {
        assert_eq!(
            allocate_exact_weighted_max_min(
                &[resource("r", 0)],
                &[flow("a", "tenant-a", 1, 1, 100, 0, &["r"])],
            )
            .unwrap_err(),
            ResourceOracleError::ZeroCapacityNotAllowed
        );
    }

    #[test]
    fn tenant_weight_must_be_consistent_across_flows() {
        assert_eq!(
            allocate_exact_weighted_max_min(
                &[resource("r", 100)],
                &[
                    flow("a", "tenant-a", 1, 1, 100, 0, &["r"]),
                    flow("b", "tenant-a", 2, 1, 100, 0, &["r"]),
                ],
            )
            .unwrap_err(),
            ResourceOracleError::InconsistentTenantWeight
        );
    }
}
