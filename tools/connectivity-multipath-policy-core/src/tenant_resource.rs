//! Tenant-first weighted progressive filling over shared resource incidence.
//!
//! Each tenant contributes one aggregate entitlement weight. Active flow weights
//! divide that tenant entitlement; active path split weights divide a flow's
//! share. When a path/resource/demand saturates, the affected path variable is
//! frozen and the remaining weights are renormalized. This prevents a tenant
//! from multiplying its entitlement by opening additional flows or paths.
//
//! This is a bounded deterministic planning model, not a global multi-commodity
//! LP solver and not a router/transport actuator.

use super::resource::{
    CapacityResource, ResourceAllocationError, ResourceAllocationRequest, ResourceDomainUsage,
    ResourcePathExclusion, canonical_ids, validate_request, validate_resource_inventory,
    valid_identifier,
};
use std::collections::{BTreeMap, BTreeSet};

const MAX_TENANTS: usize = 64;
const MAX_FLOWS: usize = 256;
const MAX_PATHS: usize = 64;
const MAX_PATHS_PER_FLOW: usize = 8;
const MAX_TENANT_WEIGHT: u32 = 1_000_000;
const MAX_FLOW_WEIGHT: u32 = 1_000;
const MAX_PATH_SPLIT_WEIGHT: u32 = 1_000;
const WEIGHT_SCALE: u128 = 1_000_000_000_000;
const RATE_DENOMINATOR: u128 = 1_000_000_000_000_000_000;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TenantResourcePrincipal {
    pub tenant_id: String,
    pub tenant_weight: u32,
    /// Aggregate minimum service floor across all flows belonging to this tenant.
    pub minimum_service_bps: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TenantResourceFlow {
    pub flow_id: String,
    pub tenant_id: String,
    pub flow_weight: u32,
    pub demand_bps: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TenantResourcePath {
    pub path_id: String,
    pub flow_id: String,
    pub path_ceiling_bps: u64,
    /// Relative split weight among the active paths of this flow.
    pub split_weight: u32,
    /// Complete set of known capacity bottlenecks consumed by this path.
    pub capacity_domain_ids: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TenantPathRateAllocation {
    pub tenant_id: String,
    pub flow_id: String,
    pub path_id: String,
    pub path_ceiling_bps: u64,
    pub split_weight: u32,
    pub allocated_rate_bps: u64,
    pub capacity_domain_ids: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TenantFlowRateAllocation {
    pub tenant_id: String,
    pub flow_id: String,
    pub flow_weight: u32,
    pub demand_bps: u64,
    pub allocated_rate_bps: u64,
    pub active_path_count: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TenantRateAllocation {
    pub tenant_id: String,
    pub tenant_weight: u32,
    pub minimum_service_bps: u64,
    pub demand_bps: u64,
    pub allocated_rate_bps: u64,
    pub active_flow_count: u32,
    pub minimum_service_met: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TenantResourceAllocationReport {
    pub topology_sha256: String,
    pub resource_snapshot_sha256: String,
    pub decision_time_unix_ms: u64,
    pub total_allocated_rate_bps: u64,
    pub tenant_allocations: Vec<TenantRateAllocation>,
    pub flow_allocations: Vec<TenantFlowRateAllocation>,
    pub path_allocations: Vec<TenantPathRateAllocation>,
    pub resource_usage: Vec<ResourceDomainUsage>,
    pub saturated_tenant_ids: Vec<String>,
    pub saturated_flow_ids: Vec<String>,
    pub saturated_path_ids: Vec<String>,
    pub saturated_resource_ids: Vec<String>,
    pub exclusions: Vec<ResourcePathExclusion>,
    pub objective: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TenantResourceAllocationError {
    Resource(ResourceAllocationError),
    TooManyTenants,
    TooManyFlows,
    TooManyPaths,
    InvalidTenantId,
    DuplicateTenantId,
    InvalidTenantWeight,
    InvalidFlowId,
    DuplicateFlowId,
    FlowTenantUnknown,
    InvalidFlowWeight,
    ZeroFlowDemand,
    TenantDemandOverflow,
    MinimumExceedsTenantDemand,
    InvalidPathId,
    DuplicatePathId,
    PathFlowUnknown,
    InvalidPathSplitWeight,
    TooManyPathsPerFlow,
    MinimumServiceInfeasible,
    ArithmeticOverflow,
    InternalInvariant,
}

#[derive(Clone, Debug)]
struct TenantEntry {
    principal: TenantResourcePrincipal,
    flow_indexes: Vec<usize>,
}

#[derive(Clone, Debug)]
struct FlowEntry {
    flow: TenantResourceFlow,
    tenant_index: usize,
    path_indexes: Vec<usize>,
}

#[derive(Clone, Debug)]
struct EligiblePath {
    path: TenantResourcePath,
    flow_index: usize,
    domain_indexes: Vec<usize>,
}

#[derive(Clone, Copy, Debug)]
struct FractionalWeight {
    path_index: usize,
    remainder: u128,
    denominator: u128,
}

fn rate_scaled(rate_bps: u64) -> Result<u128, TenantResourceAllocationError> {
    u128::from(rate_bps)
        .checked_mul(RATE_DENOMINATOR)
        .ok_or(TenantResourceAllocationError::ArithmeticOverflow)
}

fn checked_add(left: u64, right: u64) -> Result<u64, TenantResourceAllocationError> {
    left.checked_add(right)
        .ok_or(TenantResourceAllocationError::ArithmeticOverflow)
}

fn flow_totals_scaled(
    flows: &[FlowEntry],
    paths: &[EligiblePath],
    rates_scaled: &[u128],
) -> Result<Vec<u128>, TenantResourceAllocationError> {
    let mut totals = vec![0_u128; flows.len()];
    for (path_index, path) in paths.iter().enumerate() {
        totals[path.flow_index] = totals[path.flow_index]
            .checked_add(rates_scaled[path_index])
            .ok_or(TenantResourceAllocationError::ArithmeticOverflow)?;
    }
    Ok(totals)
}

fn tenant_totals_scaled(
    tenants: &[TenantEntry],
    flows: &[FlowEntry],
    flow_totals: &[u128],
) -> Result<Vec<u128>, TenantResourceAllocationError> {
    let mut totals = vec![0_u128; tenants.len()];
    for (flow_index, flow) in flows.iter().enumerate() {
        totals[flow.tenant_index] = totals[flow.tenant_index]
            .checked_add(flow_totals[flow_index])
            .ok_or(TenantResourceAllocationError::ArithmeticOverflow)?;
    }
    Ok(totals)
}

/// Compute normalized path weights. For every active tenant the sum of its
/// active path weights is exactly tenant_weight * WEIGHT_SCALE; flow and path
/// multiplicity therefore cannot increase aggregate tenant entitlement.
fn compute_effective_path_weights(
    tenants: &[TenantEntry],
    flows: &[FlowEntry],
    paths: &[EligiblePath],
    active_paths: &[bool],
    included_tenants: &[bool],
) -> Result<Vec<u128>, TenantResourceAllocationError> {
    let mut active_path_weight_per_flow = vec![0_u128; flows.len()];
    for (path_index, path) in paths.iter().enumerate() {
        if !active_paths[path_index] || !included_tenants[flows[path.flow_index].tenant_index] {
            continue;
        }
        active_path_weight_per_flow[path.flow_index] = active_path_weight_per_flow
            [path.flow_index]
            .checked_add(u128::from(path.path.split_weight))
            .ok_or(TenantResourceAllocationError::ArithmeticOverflow)?;
    }

    let mut active_flow_weight_per_tenant = vec![0_u128; tenants.len()];
    for (flow_index, flow) in flows.iter().enumerate() {
        if active_path_weight_per_flow[flow_index] == 0 {
            continue;
        }
        active_flow_weight_per_tenant[flow.tenant_index] = active_flow_weight_per_tenant
            [flow.tenant_index]
            .checked_add(u128::from(flow.flow.flow_weight))
            .ok_or(TenantResourceAllocationError::ArithmeticOverflow)?;
    }

    let mut weights = vec![0_u128; paths.len()];
    let mut fractional = vec![Vec::<FractionalWeight>::new(); tenants.len()];
    let mut used_weight = vec![0_u128; tenants.len()];

    for (path_index, path) in paths.iter().enumerate() {
        if !active_paths[path_index] {
            continue;
        }
        let flow = &flows[path.flow_index];
        let tenant_index = flow.tenant_index;
        if !included_tenants[tenant_index] {
            continue;
        }
        let flow_weight_sum = active_flow_weight_per_tenant[tenant_index];
        let path_weight_sum = active_path_weight_per_flow[path.flow_index];
        if flow_weight_sum == 0 || path_weight_sum == 0 {
            return Err(TenantResourceAllocationError::InternalInvariant);
        }
        let numerator = u128::from(tenants[tenant_index].principal.tenant_weight)
            .checked_mul(WEIGHT_SCALE)
            .and_then(|value| value.checked_mul(u128::from(flow.flow.flow_weight)))
            .and_then(|value| value.checked_mul(u128::from(path.path.split_weight)))
            .ok_or(TenantResourceAllocationError::ArithmeticOverflow)?;
        let denominator = flow_weight_sum
            .checked_mul(path_weight_sum)
            .ok_or(TenantResourceAllocationError::ArithmeticOverflow)?;
        let weight = numerator / denominator;
        if weight == 0 {
            return Err(TenantResourceAllocationError::InternalInvariant);
        }
        weights[path_index] = weight;
        used_weight[tenant_index] = used_weight[tenant_index]
            .checked_add(weight)
            .ok_or(TenantResourceAllocationError::ArithmeticOverflow)?;
        fractional[tenant_index].push(FractionalWeight {
            path_index,
            remainder: numerator % denominator,
            denominator,
        });
    }

    for tenant_index in 0..tenants.len() {
        if !included_tenants[tenant_index] {
            continue;
        }
        if fractional[tenant_index].is_empty() {
            continue;
        }
        let target = u128::from(tenants[tenant_index].principal.tenant_weight)
            .checked_mul(WEIGHT_SCALE)
            .ok_or(TenantResourceAllocationError::ArithmeticOverflow)?;
        let leftover = target
            .checked_sub(used_weight[tenant_index])
            .ok_or(TenantResourceAllocationError::InternalInvariant)?;
        let leftover_count = usize::try_from(leftover)
            .map_err(|_| TenantResourceAllocationError::ArithmeticOverflow)?;
        if leftover_count > fractional[tenant_index].len() {
            return Err(TenantResourceAllocationError::InternalInvariant);
        }
        fractional[tenant_index].sort_by(|left, right| {
            let left_fraction = left.remainder * right.denominator;
            let right_fraction = right.remainder * left.denominator;
            right_fraction
                .cmp(&left_fraction)
                .then_with(|| {
                    paths[left.path_index]
                        .path
                        .path_id
                        .cmp(&paths[right.path_index].path.path_id)
                })
        });
        for item in fractional[tenant_index].iter().take(leftover_count) {
            weights[item.path_index] = weights[item.path_index]
                .checked_add(1)
                .ok_or(TenantResourceAllocationError::ArithmeticOverflow)?;
        }
    }
    Ok(weights)
}

fn sum_weights_by_flow(
    flows: &[FlowEntry],
    paths: &[EligiblePath],
    weights: &[u128],
) -> Result<Vec<u128>, TenantResourceAllocationError> {
    let mut result = vec![0_u128; flows.len()];
    for (path_index, path) in paths.iter().enumerate() {
        result[path.flow_index] = result[path.flow_index]
            .checked_add(weights[path_index])
            .ok_or(TenantResourceAllocationError::ArithmeticOverflow)?;
    }
    Ok(result)
}

fn sum_weights_by_tenant(
    tenants: &[TenantEntry],
    flows: &[FlowEntry],
    flow_weights: &[u128],
) -> Result<Vec<u128>, TenantResourceAllocationError> {
    let mut result = vec![0_u128; tenants.len()];
    for (flow_index, flow) in flows.iter().enumerate() {
        result[flow.tenant_index] = result[flow.tenant_index]
            .checked_add(flow_weights[flow_index])
            .ok_or(TenantResourceAllocationError::ArithmeticOverflow)?;
    }
    Ok(result)
}

fn sum_weights_by_resource(
    resource_count: usize,
    paths: &[EligiblePath],
    weights: &[u128],
) -> Result<Vec<u128>, TenantResourceAllocationError> {
    let mut result = vec![0_u128; resource_count];
    for (path_index, path) in paths.iter().enumerate() {
        if weights[path_index] == 0 {
            continue;
        }
        for domain_index in &path.domain_indexes {
            result[*domain_index] = result[*domain_index]
                .checked_add(weights[path_index])
                .ok_or(TenantResourceAllocationError::ArithmeticOverflow)?;
        }
    }
    Ok(result)
}

fn any_pending_tenant_floor(
    tenants: &[TenantEntry],
    tenant_totals: &[u128],
) -> Result<bool, TenantResourceAllocationError> {
    for (index, tenant) in tenants.iter().enumerate() {
        let minimum = rate_scaled(tenant.principal.minimum_service_bps)?;
        if tenant_totals[index] < minimum {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Hierarchical weighted max-min over tenant principals, flows and path
/// alternatives. Floors are completed first; the normal fair-share phase then
/// uses the same tenant-normalized weights.
pub fn allocate_tenant_resource_incidence(
    request: &ResourceAllocationRequest,
    tenants: &[TenantResourcePrincipal],
    flows: &[TenantResourceFlow],
    paths: &[TenantResourcePath],
    resources: &[CapacityResource],
) -> Result<TenantResourceAllocationReport, TenantResourceAllocationError> {
    validate_request(request).map_err(TenantResourceAllocationError::Resource)?;
    if tenants.is_empty() || tenants.len() > MAX_TENANTS {
        return Err(TenantResourceAllocationError::TooManyTenants);
    }
    if flows.is_empty() || flows.len() > MAX_FLOWS {
        return Err(TenantResourceAllocationError::TooManyFlows);
    }
    if paths.len() > MAX_PATHS {
        return Err(TenantResourceAllocationError::TooManyPaths);
    }

    let (resource_index, resource_residuals) =
        validate_resource_inventory(request, resources)
            .map_err(TenantResourceAllocationError::Resource)?;

    let mut ordered_tenants = tenants.iter().collect::<Vec<_>>();
    ordered_tenants.sort_by(|left, right| left.tenant_id.cmp(&right.tenant_id));
    let mut tenant_index = BTreeMap::new();
    let mut tenant_entries = Vec::with_capacity(tenants.len());
    for principal in ordered_tenants {
        if !valid_identifier(&principal.tenant_id) {
            return Err(TenantResourceAllocationError::InvalidTenantId);
        }
        if principal.tenant_weight == 0 || principal.tenant_weight > MAX_TENANT_WEIGHT {
            return Err(TenantResourceAllocationError::InvalidTenantWeight);
        }
        if tenant_index
            .insert(principal.tenant_id.clone(), tenant_entries.len())
            .is_some()
        {
            return Err(TenantResourceAllocationError::DuplicateTenantId);
        }
        tenant_entries.push(TenantEntry {
            principal: principal.clone(),
            flow_indexes: Vec::new(),
        });
    }

    let mut ordered_flows = flows.iter().collect::<Vec<_>>();
    ordered_flows.sort_by(|left, right| {
        left.tenant_id
            .cmp(&right.tenant_id)
            .then_with(|| left.flow_id.cmp(&right.flow_id))
    });
    let mut flow_index = BTreeMap::new();
    let mut flow_entries = Vec::with_capacity(flows.len());
    let mut tenant_demands = vec![0_u64; tenant_entries.len()];
    for flow in ordered_flows {
        if !valid_identifier(&flow.flow_id) {
            return Err(TenantResourceAllocationError::InvalidFlowId);
        }
        if flow.demand_bps == 0 {
            return Err(TenantResourceAllocationError::ZeroFlowDemand);
        }
        if flow.flow_weight == 0 || flow.flow_weight > MAX_FLOW_WEIGHT {
            return Err(TenantResourceAllocationError::InvalidFlowWeight);
        }
        let Some(owner_index) = tenant_index.get(&flow.tenant_id).copied() else {
            return Err(TenantResourceAllocationError::FlowTenantUnknown);
        };
        if flow_index
            .insert(flow.flow_id.clone(), flow_entries.len())
            .is_some()
        {
            return Err(TenantResourceAllocationError::DuplicateFlowId);
        }
        tenant_demands[owner_index] = checked_add(tenant_demands[owner_index], flow.demand_bps)?;
        tenant_entries[owner_index]
            .flow_indexes
            .push(flow_entries.len());
        flow_entries.push(FlowEntry {
            flow: flow.clone(),
            tenant_index: owner_index,
            path_indexes: Vec::new(),
        });
    }

    for (index, tenant) in tenant_entries.iter().enumerate() {
        if tenant.principal.minimum_service_bps > tenant_demands[index] {
            return Err(TenantResourceAllocationError::MinimumExceedsTenantDemand);
        }
    }

    let mut ordered_paths = paths.iter().collect::<Vec<_>>();
    ordered_paths.sort_by(|left, right| left.path_id.cmp(&right.path_id));
    let mut seen_path_ids = BTreeSet::new();
    let mut paths_per_flow = vec![0_usize; flow_entries.len()];
    let mut exclusions = Vec::new();
    let mut eligible_paths = Vec::new();

    for path in ordered_paths {
        if !valid_identifier(&path.path_id) {
            return Err(TenantResourceAllocationError::InvalidPathId);
        }
        if !seen_path_ids.insert(path.path_id.as_str()) {
            return Err(TenantResourceAllocationError::DuplicatePathId);
        }
        let Some(flow_index) = flow_index.get(&path.flow_id).copied() else {
            return Err(TenantResourceAllocationError::PathFlowUnknown);
        };
        if path.split_weight == 0 || path.split_weight > MAX_PATH_SPLIT_WEIGHT {
            return Err(TenantResourceAllocationError::InvalidPathSplitWeight);
        }
        paths_per_flow[flow_index] += 1;
        if paths_per_flow[flow_index] > MAX_PATHS_PER_FLOW {
            return Err(TenantResourceAllocationError::TooManyPathsPerFlow);
        }
        if path.path_ceiling_bps == 0 {
            exclusions.push(ResourcePathExclusion {
                path_id: path.path_id.clone(),
                reason: "path_capacity_unavailable",
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
        if path.capacity_domain_ids.len() > super::resource::MAX_DOMAINS_PER_PATH {
            exclusions.push(ResourcePathExclusion {
                path_id: path.path_id.clone(),
                reason: "capacity_domain_set_too_large",
            });
            continue;
        }
        if path
            .capacity_domain_ids
            .iter()
            .any(|domain_id| !valid_identifier(domain_id))
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
            let Some(index) = resource_index.get(domain_id).copied() else {
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
        let path_index = eligible_paths.len();
        eligible_paths.push(EligiblePath {
            path: path.clone(),
            flow_index,
            domain_indexes,
        });
        flow_entries[flow_index].path_indexes.push(path_index);
    }

    let mut rates_scaled = vec![0_u128; eligible_paths.len()];
    let mut active_paths = vec![true; eligible_paths.len()];
    let mut remaining_scaled = resource_residuals
        .iter()
        .map(|residual| {
            residual.map(|value| rate_scaled(value)).transpose()
        })
        .collect::<Result<Vec<_>, _>>()?;

    let maximum_iterations = eligible_paths
        .len()
        .saturating_mul(4)
        .saturating_add(flow_entries.len().saturating_mul(2))
        .saturating_add(tenant_entries.len().saturating_mul(2))
        .saturating_add(16);

    for _iteration in 0..maximum_iterations {
        let current_flow_rates =
            flow_totals_scaled(&flow_entries, &eligible_paths, &rates_scaled)?;
        let current_tenant_rates =
            tenant_totals_scaled(&tenant_entries, &flow_entries, &current_flow_rates)?;
        let floor_phase = any_pending_tenant_floor(&tenant_entries, &current_tenant_rates)?;

        // Path ceilings and per-flow demands are permanent upper bounds.
        for (path_index, path) in eligible_paths.iter().enumerate() {
            if !active_paths[path_index] {
                continue;
            }
            let path_limit = rate_scaled(path.path.path_ceiling_bps)?;
            let flow_limit = rate_scaled(flow_entries[path.flow_index].flow.demand_bps)?;
            if rates_scaled[path_index] >= path_limit
                || current_flow_rates[path.flow_index] >= flow_limit
            {
                active_paths[path_index] = false;
            }
        }

        let included_tenants = tenant_entries
            .iter()
            .enumerate()
            .map(|(index, tenant)| {
                !floor_phase
                    || current_tenant_rates[index]
                        < rate_scaled(tenant.principal.minimum_service_bps).unwrap_or(u128::MAX)
            })
            .collect::<Vec<_>>();
        let effective_weights = compute_effective_path_weights(
            &tenant_entries,
            &flow_entries,
            &eligible_paths,
            &active_paths,
            &included_tenants,
        )?;
        if !effective_weights.iter().any(|weight| *weight > 0) {
            if floor_phase {
                return Err(TenantResourceAllocationError::MinimumServiceInfeasible);
            }
            break;
        }

        let active_flow_weights =
            sum_weights_by_flow(&flow_entries, &eligible_paths, &effective_weights)?;
        let active_tenant_weights =
            sum_weights_by_tenant(&tenant_entries, &flow_entries, &active_flow_weights)?;
        let active_resource_weights =
            sum_weights_by_resource(resources.len(), &eligible_paths, &effective_weights)?;

        let mut delta_level: Option<u128> = None;
        for (path_index, path) in eligible_paths.iter().enumerate() {
            let weight = effective_weights[path_index];
            if weight == 0 {
                continue;
            }
            let headroom = rate_scaled(path.path.path_ceiling_bps)?
                .checked_sub(rates_scaled[path_index])
                .ok_or(TenantResourceAllocationError::InternalInvariant)?;
            let candidate = headroom / weight;
            delta_level = Some(delta_level.map_or(candidate, |current| current.min(candidate)));
        }
        for (flow_index, flow) in flow_entries.iter().enumerate() {
            let weight = active_flow_weights[flow_index];
            if weight == 0 {
                continue;
            }
            let headroom = rate_scaled(flow.flow.demand_bps)?
                .checked_sub(current_flow_rates[flow_index])
                .ok_or(TenantResourceAllocationError::InternalInvariant)?;
            let candidate = headroom / weight;
            delta_level = Some(delta_level.map_or(candidate, |current| current.min(candidate)));
        }
        if floor_phase {
            for (tenant_index, tenant) in tenant_entries.iter().enumerate() {
                let weight = active_tenant_weights[tenant_index];
                if weight == 0 {
                    continue;
                }
                let minimum = rate_scaled(tenant.principal.minimum_service_bps)?;
                if current_tenant_rates[tenant_index] >= minimum {
                    continue;
                }
                let headroom = minimum
                    .checked_sub(current_tenant_rates[tenant_index])
                    .ok_or(TenantResourceAllocationError::InternalInvariant)?;
                let candidate = headroom.div_ceil(weight);
                delta_level = Some(delta_level.map_or(candidate, |current| current.min(candidate)));
            }
        }
        for (resource_index, weight) in active_resource_weights.iter().enumerate() {
            if *weight == 0 {
                continue;
            }
            let remaining =
                remaining_scaled[resource_index].ok_or(TenantResourceAllocationError::InternalInvariant)?;
            let candidate = remaining / *weight;
            delta_level = Some(delta_level.map_or(candidate, |current| current.min(candidate)));
        }

        let delta = delta_level.ok_or(TenantResourceAllocationError::InternalInvariant)?;
        if delta == 0 {
            let mut freeze = vec![false; eligible_paths.len()];
            for (path_index, path) in eligible_paths.iter().enumerate() {
                let weight = effective_weights[path_index];
                if !active_paths[path_index] || weight == 0 {
                    continue;
                }
                let headroom = rate_scaled(path.path.path_ceiling_bps)?
                    .checked_sub(rates_scaled[path_index])
                    .ok_or(TenantResourceAllocationError::InternalInvariant)?;
                if headroom < weight {
                    freeze[path_index] = true;
                }
            }
            for (flow_index, flow) in flow_entries.iter().enumerate() {
                let weight = active_flow_weights[flow_index];
                if weight == 0 {
                    continue;
                }
                let headroom = rate_scaled(flow.flow.demand_bps)?
                    .checked_sub(current_flow_rates[flow_index])
                    .ok_or(TenantResourceAllocationError::InternalInvariant)?;
                if headroom < weight {
                    for path_index in &flow.path_indexes {
                        if active_paths[*path_index] && effective_weights[*path_index] > 0 {
                            freeze[*path_index] = true;
                        }
                    }
                }
            }
            for (resource_index, weight) in active_resource_weights.iter().enumerate() {
                if *weight == 0 {
                    continue;
                }
                let remaining =
                    remaining_scaled[resource_index].ok_or(TenantResourceAllocationError::InternalInvariant)?;
                if remaining < *weight {
                    for (path_index, path) in eligible_paths.iter().enumerate() {
                        if active_paths[path_index]
                            && effective_weights[path_index] > 0
                            && path.domain_indexes.contains(&resource_index)
                        {
                            freeze[path_index] = true;
                        }
                    }
                }
            }
            if !freeze.iter().any(|value| *value) {
                return Err(TenantResourceAllocationError::InternalInvariant);
            }
            for (path_index, should_freeze) in freeze.iter().enumerate() {
                if *should_freeze {
                    active_paths[path_index] = false;
                }
            }
            if floor_phase {
                let totals = tenant_totals_scaled(
                    &tenant_entries,
                    &flow_entries,
                    &flow_totals_scaled(&flow_entries, &eligible_paths, &rates_scaled)?,
                )?;
                for (tenant_index, tenant) in tenant_entries.iter().enumerate() {
                    let minimum = rate_scaled(tenant.principal.minimum_service_bps)?;
                    if totals[tenant_index] < minimum
                        && !eligible_paths.iter().enumerate().any(|(path_index, path)| {
                            active_paths[path_index]
                                && flows_tenant_index(&flow_entries[path.flow_index]) == tenant_index
                        })
                    {
                        return Err(TenantResourceAllocationError::MinimumServiceInfeasible);
                    }
                }
            }
            continue;
        }

        for (path_index, weight) in effective_weights.iter().enumerate() {
            if *weight == 0 {
                continue;
            }
            let increment = delta
                .checked_mul(*weight)
                .ok_or(TenantResourceAllocationError::ArithmeticOverflow)?;
            rates_scaled[path_index] = rates_scaled[path_index]
                .checked_add(increment)
                .ok_or(TenantResourceAllocationError::ArithmeticOverflow)?;
        }
        for (resource_index, weight) in active_resource_weights.iter().enumerate() {
            if *weight == 0 {
                continue;
            }
            let used = delta
                .checked_mul(*weight)
                .ok_or(TenantResourceAllocationError::ArithmeticOverflow)?;
            let remaining =
                remaining_scaled[resource_index].ok_or(TenantResourceAllocationError::InternalInvariant)?;
            remaining_scaled[resource_index] = Some(
                remaining
                    .checked_sub(used)
                    .ok_or(TenantResourceAllocationError::InternalInvariant)?,
            );
        }

        let updated_flow_rates =
            flow_totals_scaled(&flow_entries, &eligible_paths, &rates_scaled)?;
        let mut freeze = vec![false; eligible_paths.len()];
        for (path_index, path) in eligible_paths.iter().enumerate() {
            if active_paths[path_index]
                && rates_scaled[path_index] >= rate_scaled(path.path.path_ceiling_bps)?
            {
                freeze[path_index] = true;
            }
        }
        for (flow_index, flow) in flow_entries.iter().enumerate() {
            if updated_flow_rates[flow_index] >= rate_scaled(flow.flow.demand_bps)? {
                for path_index in &flow.path_indexes {
                    if active_paths[*path_index] && effective_weights[*path_index] > 0 {
                        freeze[*path_index] = true;
                    }
                }
            }
        }
        for (resource_index, weight) in active_resource_weights.iter().enumerate() {
            if *weight == 0 {
                continue;
            }
            let remaining =
                remaining_scaled[resource_index].ok_or(TenantResourceAllocationError::InternalInvariant)?;
            if remaining < *weight {
                for (path_index, path) in eligible_paths.iter().enumerate() {
                    if active_paths[path_index]
                        && effective_weights[path_index] > 0
                        && path.domain_indexes.contains(&resource_index)
                    {
                        freeze[path_index] = true;
                    }
                }
            }
        }
        let any_frozen = freeze.iter().any(|value| *value);
        for (path_index, should_freeze) in freeze.iter().enumerate() {
            if *should_freeze {
                active_paths[path_index] = false;
            }
        }
        let updated_tenant_rates =
            tenant_totals_scaled(&tenant_entries, &flow_entries, &updated_flow_rates)?;
        let floors_complete = tenant_entries.iter().enumerate().all(|(index, tenant)| {
            updated_tenant_rates[index]
                >= rate_scaled(tenant.principal.minimum_service_bps).unwrap_or(u128::MAX)
        });
        if !any_frozen && !(floor_phase && floors_complete) {
            return Err(TenantResourceAllocationError::InternalInvariant);
        }
        if floor_phase && !floors_complete {
            for (tenant_index, tenant) in tenant_entries.iter().enumerate() {
                let minimum = rate_scaled(tenant.principal.minimum_service_bps)?;
                if updated_tenant_rates[tenant_index] < minimum
                    && !eligible_paths.iter().enumerate().any(|(path_index, path)| {
                        active_paths[path_index]
                            && flows_tenant_index(&flow_entries[path.flow_index]) == tenant_index
                    })
                {
                    return Err(TenantResourceAllocationError::MinimumServiceInfeasible);
                }
            }
        }
    }

    let final_flow_rates =
        flow_totals_scaled(&flow_entries, &eligible_paths, &rates_scaled)?;
    let final_tenant_rates =
        tenant_totals_scaled(&tenant_entries, &flow_entries, &final_flow_rates)?;

    let mut integer_path_rates = rates_scaled
        .iter()
        .map(|value| {
            u64::try_from(value / RATE_DENOMINATOR)
                .map_err(|_| TenantResourceAllocationError::ArithmeticOverflow)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut integer_flow_rates = vec![0_u64; flow_entries.len()];
    let mut integer_resource_usage = vec![0_u64; resources.len()];
    for (path_index, path) in eligible_paths.iter().enumerate() {
        integer_flow_rates[path.flow_index] = checked_add(
            integer_flow_rates[path.flow_index],
            integer_path_rates[path_index],
        )?;
        for domain_index in &path.domain_indexes {
            integer_resource_usage[*domain_index] =
                checked_add(integer_resource_usage[*domain_index], integer_path_rates[path_index])?;
        }
    }

    // Deterministic largest-remainder rounding never exceeds a path, flow or
    // shared-resource ceiling.
    let mut fractional_order = (0..eligible_paths.len()).collect::<Vec<_>>();
    fractional_order.sort_by(|left, right| {
        (rates_scaled[*right] % RATE_DENOMINATOR)
            .cmp(&(rates_scaled[*left] % RATE_DENOMINATOR))
            .then_with(|| {
                eligible_paths[*left]
                    .path
                    .path_id
                    .cmp(&eligible_paths[*right].path.path_id)
            })
    });
    for path_index in fractional_order {
        if rates_scaled[path_index] % RATE_DENOMINATOR == 0 {
            continue;
        }
        let path = &eligible_paths[path_index];
        let flow = &flow_entries[path.flow_index];
        if integer_path_rates[path_index] >= path.path.path_ceiling_bps
            || integer_flow_rates[path.flow_index] >= flow.flow.demand_bps
        {
            continue;
        }
        let fits_resources = path.domain_indexes.iter().all(|domain_index| {
            let Some(residual) = resource_residuals[*domain_index] else {
                return false;
            };
            integer_resource_usage[*domain_index] < residual
        });
        if !fits_resources {
            continue;
        }
        integer_path_rates[path_index] = checked_add(integer_path_rates[path_index], 1)?;
        integer_flow_rates[path.flow_index] = checked_add(integer_flow_rates[path.flow_index], 1)?;
        for domain_index in &path.domain_indexes {
            integer_resource_usage[*domain_index] =
                checked_add(integer_resource_usage[*domain_index], 1)?;
        }
    }

    let mut integer_tenant_rates = vec![0_u64; tenant_entries.len()];
    for (flow_index, flow) in flow_entries.iter().enumerate() {
        integer_tenant_rates[flow.tenant_index] = checked_add(
            integer_tenant_rates[flow.tenant_index],
            integer_flow_rates[flow_index],
        )?;
    }
    for (tenant_index, tenant) in tenant_entries.iter().enumerate() {
        if integer_tenant_rates[tenant_index] < tenant.principal.minimum_service_bps {
            return Err(TenantResourceAllocationError::MinimumServiceInfeasible);
        }
    }

    let mut total_allocated_rate_bps = 0_u64;
    let mut path_allocations = Vec::with_capacity(eligible_paths.len());
    let mut saturated_path_ids = Vec::new();
    for (path_index, path) in eligible_paths.iter().enumerate() {
        let flow = &flow_entries[path.flow_index];
        let tenant = &tenant_entries[flow.tenant_index];
        let allocated_rate_bps = integer_path_rates[path_index];
        if allocated_rate_bps > path.path.path_ceiling_bps {
            return Err(TenantResourceAllocationError::InternalInvariant);
        }
        total_allocated_rate_bps = checked_add(total_allocated_rate_bps, allocated_rate_bps)?;
        if allocated_rate_bps == path.path.path_ceiling_bps {
            saturated_path_ids.push(path.path.path_id.clone());
        }
        path_allocations.push(TenantPathRateAllocation {
            tenant_id: tenant.principal.tenant_id.clone(),
            flow_id: flow.flow.flow_id.clone(),
            path_id: path.path.path_id.clone(),
            path_ceiling_bps: path.path.path_ceiling_bps,
            split_weight: path.path.split_weight,
            allocated_rate_bps,
            capacity_domain_ids: canonical_ids(&path.path.capacity_domain_ids),
        });
    }

    let mut flow_allocations = Vec::with_capacity(flow_entries.len());
    let mut saturated_flow_ids = Vec::new();
    for (flow_index, flow) in flow_entries.iter().enumerate() {
        let allocated_rate_bps = integer_flow_rates[flow_index];
        if allocated_rate_bps == flow.flow.demand_bps {
            saturated_flow_ids.push(flow.flow.flow_id.clone());
        }
        let active_path_count = u32::try_from(
            flow.path_indexes
                .iter()
                .filter(|path_index| integer_path_rates[**path_index] > 0)
                .count(),
        )
        .map_err(|_| TenantResourceAllocationError::ArithmeticOverflow)?;
        flow_allocations.push(TenantFlowRateAllocation {
            tenant_id: flow.flow.tenant_id.clone(),
            flow_id: flow.flow.flow_id.clone(),
            flow_weight: flow.flow.flow_weight,
            demand_bps: flow.flow.demand_bps,
            allocated_rate_bps,
            active_path_count,
        });
    }

    let mut tenant_allocations = Vec::with_capacity(tenant_entries.len());
    let mut saturated_tenant_ids = Vec::new();
    for (tenant_index, tenant) in tenant_entries.iter().enumerate() {
        let demand_bps = tenant_demands[tenant_index];
        let allocated_rate_bps = integer_tenant_rates[tenant_index];
        let active_flow_count = u32::try_from(
            tenant
                .flow_indexes
                .iter()
                .filter(|flow_index| integer_flow_rates[**flow_index] > 0)
                .count(),
        )
        .map_err(|_| TenantResourceAllocationError::ArithmeticOverflow)?;
        let minimum_service_met = allocated_rate_bps >= tenant.principal.minimum_service_bps;
        if minimum_service_met {
            saturated_tenant_ids.push(tenant.principal.tenant_id.clone());
        }
        tenant_allocations.push(TenantRateAllocation {
            tenant_id: tenant.principal.tenant_id.clone(),
            tenant_weight: tenant.principal.tenant_weight,
            minimum_service_bps: tenant.principal.minimum_service_bps,
            demand_bps,
            allocated_rate_bps,
            active_flow_count,
            minimum_service_met,
        });
    }

    let mut resource_usage = Vec::new();
    let mut saturated_resource_ids = Vec::new();
    for (index, resource) in resources.iter().enumerate() {
        let Some(residual_capacity_bps) = resource_residuals[index] else {
            continue;
        };
        let allocated_rate_bps = integer_resource_usage[index];
        if allocated_rate_bps > residual_capacity_bps {
            return Err(TenantResourceAllocationError::InternalInvariant);
        }
        let unused_capacity_bps = residual_capacity_bps - allocated_rate_bps;
        let saturated = unused_capacity_bps == 0;
        if saturated {
            saturated_resource_ids.push(resource.resource_id.clone());
        }
        resource_usage.push(ResourceDomainUsage {
            resource_id: resource.resource_id.clone(),
            source_id: resource.evidence.source_id.clone(),
            inventory_sha256: resource.evidence.inventory_sha256.clone(),
            topology_sha256: resource.evidence.topology_sha256.clone(),
            inventory_complete: resource.evidence.complete,
            confidence_ppm: resource.evidence.confidence_ppm,
            measurement_window_start_unix_ms: resource.evidence.measurement_window_start_unix_ms,
            measurement_window_end_unix_ms: resource.evidence.measurement_window_end_unix_ms,
            observed_at_unix_ms: resource.evidence.observed_at_unix_ms,
            valid_until_unix_ms: resource.evidence.valid_until_unix_ms,
            declared_capacity_bps: resource.declared_capacity_bps,
            measured_background_load_bps: resource.measured_background_load_bps,
            committed_reservations_bps: resource.committed_reservations_bps,
            safety_margin_bps: resource.safety_margin_bps,
            residual_capacity_bps,
            allocated_rate_bps,
            unused_capacity_bps,
            saturated,
        });
    }

    path_allocations.sort_by(|left, right| left.path_id.cmp(&right.path_id));
    flow_allocations.sort_by(|left, right| {
        left.tenant_id
            .cmp(&right.tenant_id)
            .then_with(|| left.flow_id.cmp(&right.flow_id))
    });
    tenant_allocations.sort_by(|left, right| left.tenant_id.cmp(&right.tenant_id));
    resource_usage.sort_by(|left, right| left.resource_id.cmp(&right.resource_id));
    exclusions.sort_by(|left, right| left.path_id.cmp(&right.path_id));
    saturated_tenant_ids.sort();
    saturated_flow_ids.sort();
    saturated_path_ids.sort();
    saturated_resource_ids.sort();

    Ok(TenantResourceAllocationReport {
        topology_sha256: request.topology_sha256.clone(),
        resource_snapshot_sha256: request.resource_snapshot_sha256.clone(),
        decision_time_unix_ms: request.now_unix_ms,
        total_allocated_rate_bps,
        tenant_allocations,
        flow_allocations,
        path_allocations,
        resource_usage,
        saturated_tenant_ids,
        saturated_flow_ids,
        saturated_path_ids,
        saturated_resource_ids,
        exclusions,
        objective: "tenant_weighted_max_min_then_flow_weighted_max_min_over_shared_resource_incidence",
    })
}

fn flows_tenant_index(flow: &FlowEntry) -> usize {
    flow.tenant_index
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evidence() -> super::super::resource::CapacityResourceEvidence {
        super::super::resource::CapacityResourceEvidence {
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
            resource_snapshot_sha256: "c".repeat(64),
            now_unix_ms: 1_000,
            minimum_evidence_confidence_ppm: 950_000,
        }
    }

    fn resource(id: &str, capacity: u64) -> CapacityResource {
        CapacityResource {
            resource_id: id.to_owned(),
            declared_capacity_bps: capacity,
            measured_background_load_bps: 0,
            committed_reservations_bps: 0,
            safety_margin_bps: 0,
            evidence: evidence(),
        }
    }

    fn tenant(id: &str, weight: u32, floor: u64) -> TenantResourcePrincipal {
        TenantResourcePrincipal {
            tenant_id: id.to_owned(),
            tenant_weight: weight,
            minimum_service_bps: floor,
        }
    }

    fn flow(id: &str, tenant_id: &str, weight: u32, demand: u64) -> TenantResourceFlow {
        TenantResourceFlow {
            flow_id: id.to_owned(),
            tenant_id: tenant_id.to_owned(),
            flow_weight: weight,
            demand_bps: demand,
        }
    }

    fn path(
        id: &str,
        flow_id: &str,
        ceiling: u64,
        split_weight: u32,
        domains: &[&str],
    ) -> TenantResourcePath {
        TenantResourcePath {
            path_id: id.to_owned(),
            flow_id: flow_id.to_owned(),
            path_ceiling_bps: ceiling,
            split_weight,
            capacity_domain_ids: domains.iter().map(|domain| (*domain).to_owned()).collect(),
        }
    }

    #[test]
    fn tenant_flow_multiplication_does_not_multiply_aggregate_entitlement() {
        let tenants = vec![tenant("a", 1, 0), tenant("b", 1, 0)];
        let mut flows = (0..10)
            .map(|index| flow(&format!("a-{index:02}"), "a", 1, 100))
            .collect::<Vec<_>>();
        flows.push(flow("b-0", "b", 1, 100));
        let mut paths = flows
            .iter()
            .map(|item| path(&format!("p-{}", item.flow_id), &item.flow_id, 100, 1, &["shared"]))
            .collect::<Vec<_>>();
        paths.sort_by(|left, right| left.path_id.cmp(&right.path_id));
        let report = allocate_tenant_resource_incidence(
            &request(),
            &tenants,
            &flows,
            &paths,
            &[resource("shared", 110)],
        )
        .expect("feasible allocation");
        let a = report.tenant_allocations.iter().find(|x| x.tenant_id == "a").unwrap();
        let b = report.tenant_allocations.iter().find(|x| x.tenant_id == "b").unwrap();
        assert_eq!(a.allocated_rate_bps, 55);
        assert_eq!(b.allocated_rate_bps, 55);
    }

    #[test]
    fn unequal_tenant_weights_set_aggregate_entitlement_before_flow_weights() {
        let report = allocate_tenant_resource_incidence(
            &request(),
            &[tenant("a", 2, 0), tenant("b", 1, 0)],
            &[flow("fa", "a", 1, 100), flow("fb", "b", 1, 100)],
            &[
                path("pa", "fa", 100, 1, &["shared"]),
                path("pb", "fb", 100, 1, &["shared"]),
            ],
            &[resource("shared", 90)],
        )
        .expect("feasible allocation");
        let a = report.tenant_allocations.iter().find(|x| x.tenant_id == "a").unwrap();
        let b = report.tenant_allocations.iter().find(|x| x.tenant_id == "b").unwrap();
        assert_eq!(a.allocated_rate_bps, 60);
        assert_eq!(b.allocated_rate_bps, 30);
    }

    #[test]
    fn flow_weights_share_within_a_tenant_after_tenant_entitlement() {
        let report = allocate_tenant_resource_incidence(
            &request(),
            &[tenant("a", 1, 0), tenant("b", 1, 0)],
            &[
                flow("a-small", "a", 1, 10),
                flow("a-large", "a", 3, 100),
                flow("b", "b", 1, 100),
            ],
            &[
                path("p-a-small", "a-small", 100, 1, &["shared"]),
                path("p-a-large", "a-large", 100, 1, &["shared"]),
                path("p-b", "b", 100, 1, &["shared"]),
            ],
            &[resource("shared", 90)],
        )
        .expect("feasible allocation");
        let a_small = report.flow_allocations.iter().find(|x| x.flow_id == "a-small").unwrap();
        let a_large = report.flow_allocations.iter().find(|x| x.flow_id == "a-large").unwrap();
        let b = report.flow_allocations.iter().find(|x| x.flow_id == "b").unwrap();
        assert_eq!(a_small.allocated_rate_bps, 10);
        assert_eq!(a_large.allocated_rate_bps, 35);
        assert_eq!(b.allocated_rate_bps, 45);
    }

    #[test]
    fn multiple_paths_for_one_flow_do_not_multiply_its_tenant_weight() {
        let report = allocate_tenant_resource_incidence(
            &request(),
            &[tenant("a", 1, 0), tenant("b", 1, 0)],
            &[flow("fa", "a", 1, 100), flow("fb", "b", 1, 100)],
            &[
                path("pa1", "fa", 100, 1, &["shared"]),
                path("pa2", "fa", 100, 1, &["shared"]),
                path("pb", "fb", 100, 1, &["shared"]),
            ],
            &[resource("shared", 100)],
        )
        .expect("feasible allocation");
        let a = report.tenant_allocations.iter().find(|x| x.tenant_id == "a").unwrap();
        let b = report.tenant_allocations.iter().find(|x| x.tenant_id == "b").unwrap();
        assert_eq!(a.allocated_rate_bps, 50);
        assert_eq!(b.allocated_rate_bps, 50);
        let a_paths = report.path_allocations.iter().filter(|x| x.flow_id == "fa").collect::<Vec<_>>();
        assert_eq!(a_paths.len(), 2);
        assert_eq!(a_paths.iter().map(|x| x.allocated_rate_bps).sum::<u64>(), 50);
    }

    #[test]
    fn tenant_minimum_service_floors_are_honoured_when_feasible() {
        let report = allocate_tenant_resource_incidence(
            &request(),
            &[tenant("a", 1, 20), tenant("b", 1, 20)],
            &[flow("fa", "a", 1, 100), flow("fb", "b", 1, 100)],
            &[
                path("pa", "fa", 100, 1, &["shared"]),
                path("pb", "fb", 100, 1, &["shared"]),
            ],
            &[resource("shared", 100)],
        )
        .expect("both floors fit");
        assert!(report.tenant_allocations.iter().all(|x| x.minimum_service_met));
        assert_eq!(report.tenant_allocations[0].allocated_rate_bps, 50);
        assert_eq!(report.tenant_allocations[1].allocated_rate_bps, 50);
    }

    #[test]
    fn infeasible_tenant_minimum_floors_fail_closed() {
        let error = allocate_tenant_resource_incidence(
            &request(),
            &[tenant("a", 1, 30), tenant("b", 1, 30)],
            &[flow("fa", "a", 1, 100), flow("fb", "b", 1, 100)],
            &[
                path("pa", "fa", 100, 1, &["shared"]),
                path("pb", "fb", 100, 1, &["shared"]),
            ],
            &[resource("shared", 50)],
        )
        .unwrap_err();
        assert_eq!(error, TenantResourceAllocationError::MinimumServiceInfeasible);
    }

    #[test]
    fn exact_small_overlap_oracle_agrees_with_tenant_progressive_fill() {
        // A uses X; B uses X+Y; C uses Y. X and Y each have capacity 10.
        // The exact integral leximin witness is [5, 5, 5], not the
        // maximum-sum but starving allocation [10, 0, 10].
        let report = allocate_tenant_resource_incidence(
            &request(),
            &[
                tenant("a", 1, 0),
                tenant("b", 1, 0),
                tenant("c", 1, 0),
            ],
            &[
                flow("fa", "a", 1, 10),
                flow("fb", "b", 1, 10),
                flow("fc", "c", 1, 10),
            ],
            &[
                path("pa", "fa", 10, 1, &["x"]),
                path("pb", "fb", 10, 1, &["x", "y"]),
                path("pc", "fc", 10, 1, &["y"]),
            ],
            &[resource("x", 10), resource("y", 10)],
        )
        .expect("feasible overlap allocation");
        let mut actual = report
            .flow_allocations
            .iter()
            .map(|flow| flow.allocated_rate_bps)
            .collect::<Vec<_>>();
        actual.sort();
        let mut oracle_best = Vec::new();
        let mut best: Option<Vec<u64>> = None;
        for a in 0..=10_u64 {
            for b in 0..=10_u64 {
                for c in 0..=10_u64 {
                    if a + b > 10 || b + c > 10 {
                        continue;
                    }
                    let mut vector = vec![a, b, c];
                    vector.sort();
                    if best.as_ref().is_none_or(|current| vector > *current) {
                        best = Some(vector);
                    }
                }
            }
        }
        oracle_best = best.expect("at least zero allocation is feasible");
        assert_eq!(actual, oracle_best);
        assert_eq!(oracle_best, vec![5, 5, 5]);
    }

    #[test]
    fn path_and_resource_order_permutations_are_deterministic() {
        let tenants = vec![tenant("a", 1, 0), tenant("b", 1, 0)];
        let flows = vec![flow("fa", "a", 1, 100), flow("fb", "b", 1, 100)];
        let paths = vec![
            path("pa", "fa", 100, 1, &["x", "shared"]),
            path("pb", "fb", 100, 1, &["y", "shared"]),
        ];
        let resources = vec![resource("x", 100), resource("y", 100), resource("shared", 100)];
        let forward = allocate_tenant_resource_incidence(
            &request(),
            &tenants,
            &flows,
            &paths,
            &resources,
        )
        .expect("forward order");
        let reverse = allocate_tenant_resource_incidence(
            &request(),
            &tenants.into_iter().rev().collect::<Vec<_>>(),
            &flows.into_iter().rev().collect::<Vec<_>>(),
            &paths.into_iter().rev().collect::<Vec<_>>(),
            &resources.into_iter().rev().collect::<Vec<_>>(),
        )
        .expect("reverse order");
        assert_eq!(forward, reverse);
    }

    #[test]
    fn invalid_resource_evidence_excludes_dependent_path() {
        let mut stale = resource("stale", 100);
        stale.evidence.valid_until_unix_ms = 950;
        let report = allocate_tenant_resource_incidence(
            &request(),
            &[tenant("a", 1, 0)],
            &[flow("fa", "a", 1, 100)],
            &[path("pa", "fa", 100, 1, &["stale"])],
            &[stale],
        )
        .expect("stale evidence is a path exclusion");
        assert_eq!(report.path_allocations.len(), 0);
        assert_eq!(report.exclusions[0].reason, "capacity_resource_evidence_invalid");
    }

    #[test]
    fn invalid_snapshot_cannot_satisfy_a_tenant_floor() {
        let mut stale = resource("stale", 100);
        stale.evidence.valid_until_unix_ms = 950;
        let error = allocate_tenant_resource_incidence(
            &request(),
            &[tenant("a", 1, 10)],
            &[flow("fa", "a", 1, 100)],
            &[path("pa", "fa", 100, 1, &["stale"])],
            &[stale],
        )
        .unwrap_err();
        assert_eq!(error, TenantResourceAllocationError::MinimumServiceInfeasible);
    }
}
