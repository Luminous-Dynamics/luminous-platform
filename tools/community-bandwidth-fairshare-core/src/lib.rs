//! Pure reference model for allocating a measured byte budget on a shared link.
//!
//! This crate allocates bytes in a declared scheduling window. It does not
//! measure radio capacity, authenticate peers, enforce an AP queue, or verify
//! that a claimed essential class is authorized. Callers must supply those
//! trusted-boundary decisions separately.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

pub const MAX_PEERS: usize = 4_096;
pub const MIN_WEIGHT: u16 = 1;
pub const MAX_WEIGHT: u16 = 1_000;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AllocationPolicy {
    /// Capacity measured by the caller for one explicit scheduling window.
    pub total_capacity_bytes: u64,
    /// Capacity protected for essential requests before ordinary requests are
    /// considered. Unused reserve can be borrowed when no essential demand
    /// remains in the window.
    pub essential_reserve_bytes: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PeerDemand {
    /// Opaque, stable, non-PII identifier supplied by an authenticated adapter.
    pub peer_id: String,
    /// Relative weight in the range 1..=MAX_WEIGHT.
    pub weight: u16,
    pub essential_demand_bytes: u64,
    pub ordinary_demand_bytes: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PeerGrant {
    pub peer_id: String,
    pub essential_granted_bytes: u64,
    pub ordinary_granted_bytes: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Allocation {
    /// Grants are sorted lexicographically by opaque peer ID.
    pub grants: Vec<PeerGrant>,
    pub total_essential_granted_bytes: u64,
    pub total_ordinary_granted_bytes: u64,
    /// Capacity not allocated because demand was too small or no peer existed.
    pub unallocated_bytes: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AllocationError {
    ZeroCapacity,
    EssentialReserveExceedsCapacity,
    TooManyPeers,
    InvalidPeerId,
    DuplicatePeerId,
    InvalidWeight,
    ArithmeticOverflow,
}

pub fn allocate(
    policy: &AllocationPolicy,
    demands: &[PeerDemand],
) -> Result<Allocation, AllocationError> {
    if policy.total_capacity_bytes == 0 {
        return Err(AllocationError::ZeroCapacity);
    }
    if policy.essential_reserve_bytes > policy.total_capacity_bytes {
        return Err(AllocationError::EssentialReserveExceedsCapacity);
    }
    if demands.len() > MAX_PEERS {
        return Err(AllocationError::TooManyPeers);
    }

    let mut seen = BTreeSet::new();
    for demand in demands {
        if !valid_peer_id(&demand.peer_id) {
            return Err(AllocationError::InvalidPeerId);
        }
        if !seen.insert(demand.peer_id.clone()) {
            return Err(AllocationError::DuplicatePeerId);
        }
        if !(MIN_WEIGHT..=MAX_WEIGHT).contains(&demand.weight) {
            return Err(AllocationError::InvalidWeight);
        }
    }

    // Canonical ordering makes tie-breaking independent of input order.
    let mut ordered = demands.to_vec();
    ordered.sort_by(|left, right| left.peer_id.cmp(&right.peer_id));
    let weights: Vec<u16> = ordered.iter().map(|peer| peer.weight).collect();
    let essential_requests: Vec<u64> = ordered
        .iter()
        .map(|peer| peer.essential_demand_bytes)
        .collect();

    let reserve_grants = weighted_max_min(
        &weights,
        &essential_requests,
        policy.essential_reserve_bytes,
    );
    let essential_remaining: Vec<u64> = essential_requests
        .iter()
        .zip(&reserve_grants)
        .map(|(request, grant)| request - grant)
        .collect();
    let essential_still_unserved = essential_remaining.iter().any(|bytes| *bytes > 0);

    // If essential traffic remains after consuming the reserve, it may use the
    // ordinary portion of the budget before any ordinary request receives a
    // grant. If it does not remain, unused reserve is available to ordinary
    // traffic instead of being stranded.
    let extra_essential_capacity = if essential_still_unserved {
        policy.total_capacity_bytes - policy.essential_reserve_bytes
    } else {
        0
    };
    let extra_essential_grants =
        weighted_max_min(&weights, &essential_remaining, extra_essential_capacity);
    let essential_grants: Vec<u64> = reserve_grants
        .iter()
        .zip(&extra_essential_grants)
        .map(|(reserved, extra)| reserved + extra)
        .collect();
    let total_essential = checked_sum(essential_grants.iter().copied())?;

    let ordinary_capacity = policy.total_capacity_bytes - total_essential;
    let ordinary_requests: Vec<u64> = ordered
        .iter()
        .map(|peer| peer.ordinary_demand_bytes)
        .collect();
    let ordinary_grants = weighted_max_min(&weights, &ordinary_requests, ordinary_capacity);
    let total_ordinary = checked_sum(ordinary_grants.iter().copied())?;
    let total_granted = total_essential
        .checked_add(total_ordinary)
        .ok_or(AllocationError::ArithmeticOverflow)?;

    if total_granted > policy.total_capacity_bytes {
        return Err(AllocationError::ArithmeticOverflow);
    }

    let grants = ordered
        .into_iter()
        .zip(essential_grants)
        .zip(ordinary_grants)
        .map(|((peer, essential), ordinary)| PeerGrant {
            peer_id: peer.peer_id,
            essential_granted_bytes: essential,
            ordinary_granted_bytes: ordinary,
        })
        .collect();

    Ok(Allocation {
        grants,
        total_essential_granted_bytes: total_essential,
        total_ordinary_granted_bytes: total_ordinary,
        unallocated_bytes: policy.total_capacity_bytes - total_granted,
    })
}

fn valid_peer_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

fn checked_sum(values: impl Iterator<Item = u64>) -> Result<u64, AllocationError> {
    values.into_iter().try_fold(0_u64, |total, value| {
        total
            .checked_add(value)
            .ok_or(AllocationError::ArithmeticOverflow)
    })
}

/// Exact weighted max-min filling with integer byte units. Demand caps are
/// saturated first; the remaining budget is redistributed. When indivisible
/// byte rounding remains, largest fractional remainders win, then canonical
/// input order. We use u128 for cross-products to avoid u64 multiplication
/// overflow.
fn weighted_max_min(weights: &[u16], requests: &[u64], capacity: u64) -> Vec<u64> {
    let mut grants = vec![0_u64; requests.len()];
    let mut unmet = requests.to_vec();
    let mut active: Vec<usize> = (0..requests.len())
        .filter(|index| unmet[*index] > 0)
        .collect();
    let mut remaining_capacity = capacity;

    while remaining_capacity > 0 && !active.is_empty() {
        let weight_sum: u128 = active.iter().map(|index| u128::from(weights[*index])).sum();
        let capacity_wide = u128::from(remaining_capacity);

        let constrained: Vec<usize> = active
            .iter()
            .copied()
            .filter(|index| {
                u128::from(unmet[*index]) * weight_sum
                    <= capacity_wide * u128::from(weights[*index])
            })
            .collect();

        if !constrained.is_empty() {
            // Each constrained demand is at or below its weighted fair share;
            // the sum of all such demands cannot exceed the current budget.
            for index in constrained {
                let grant = unmet[index];
                grants[index] += grant;
                unmet[index] = 0;
                remaining_capacity -= grant;
            }
            active.retain(|index| unmet[*index] > 0);
            continue;
        }

        // No active request is small enough to saturate. Allocate the floor
        // of each proportional share, then distribute the fewer-than-N
        // leftover bytes by fractional remainder and stable index.
        let mut shares: Vec<(usize, u64, u128)> = Vec::with_capacity(active.len());
        let mut floor_total = 0_u64;
        for index in &active {
            let numerator = capacity_wide * u128::from(weights[*index]);
            let share = (numerator / weight_sum) as u64;
            let remainder = numerator % weight_sum;
            floor_total += share;
            shares.push((*index, share, remainder));
        }

        for (index, share, _) in &shares {
            grants[*index] += *share;
            unmet[*index] -= *share;
        }
        let leftover = remaining_capacity - floor_total;
        shares.sort_by(|left, right| right.2.cmp(&left.2).then_with(|| left.0.cmp(&right.0)));
        for (index, _, _) in shares.into_iter().take(leftover as usize) {
            grants[index] += 1;
            unmet[index] -= 1;
        }
        remaining_capacity = 0;
    }

    grants
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy(capacity: u64, reserve: u64) -> AllocationPolicy {
        AllocationPolicy {
            total_capacity_bytes: capacity,
            essential_reserve_bytes: reserve,
        }
    }

    fn peer(id: &str, weight: u16, essential: u64, ordinary: u64) -> PeerDemand {
        PeerDemand {
            peer_id: id.to_owned(),
            weight,
            essential_demand_bytes: essential,
            ordinary_demand_bytes: ordinary,
        }
    }

    #[test]
    fn zero_capacity_and_invalid_reserve_are_rejected() {
        assert_eq!(
            allocate(&policy(0, 0), &[]),
            Err(AllocationError::ZeroCapacity)
        );
        assert_eq!(
            allocate(&policy(10, 11), &[]),
            Err(AllocationError::EssentialReserveExceedsCapacity)
        );
    }

    #[test]
    fn duplicate_and_malformed_peer_ids_are_rejected() {
        assert_eq!(
            allocate(
                &policy(100, 0),
                &[peer("same", 1, 0, 1), peer("same", 1, 0, 1)]
            ),
            Err(AllocationError::DuplicatePeerId)
        );
        assert_eq!(
            allocate(&policy(100, 0), &[peer("peer with spaces", 1, 0, 1)]),
            Err(AllocationError::InvalidPeerId)
        );
    }

    #[test]
    fn invalid_weight_and_too_many_peers_are_rejected() {
        assert_eq!(
            allocate(&policy(100, 0), &[peer("peer-a", 0, 0, 1)]),
            Err(AllocationError::InvalidWeight)
        );
        let too_many: Vec<PeerDemand> = (0..=MAX_PEERS)
            .map(|index| peer(&format!("peer-{index:04}"), 1, 0, 1))
            .collect();
        assert_eq!(
            allocate(&policy(100, 0), &too_many),
            Err(AllocationError::TooManyPeers)
        );
    }

    #[test]
    fn essential_demand_is_served_before_ordinary_demand() {
        let result = allocate(&policy(100, 20), &[peer("peer-a", 1, 80, 100)]).unwrap();
        assert_eq!(result.grants[0].essential_granted_bytes, 80);
        assert_eq!(result.grants[0].ordinary_granted_bytes, 20);
        assert_eq!(result.unallocated_bytes, 0);
    }

    #[test]
    fn unmet_essential_demand_can_use_capacity_beyond_the_reserve() {
        let result = allocate(&policy(100, 20), &[peer("peer-a", 1, 200, 100)]).unwrap();
        assert_eq!(result.total_essential_granted_bytes, 100);
        assert_eq!(result.total_ordinary_granted_bytes, 0);
        assert_eq!(result.unallocated_bytes, 0);
    }

    #[test]
    fn unused_essential_reserve_is_borrowed_after_essential_demand_is_met() {
        let result = allocate(&policy(100, 30), &[peer("peer-a", 1, 10, 1_000)]).unwrap();
        assert_eq!(result.total_essential_granted_bytes, 10);
        assert_eq!(result.total_ordinary_granted_bytes, 90);
        assert_eq!(result.unallocated_bytes, 0);
    }

    #[test]
    fn equal_weights_split_capacity_evenly_with_stable_rounding() {
        let demands = vec![
            peer("peer-c", 1, 0, 100),
            peer("peer-a", 1, 0, 100),
            peer("peer-b", 1, 0, 100),
        ];
        let result = allocate(&policy(10, 0), &demands).unwrap();
        let amounts: Vec<u64> = result
            .grants
            .iter()
            .map(|grant| grant.ordinary_granted_bytes)
            .collect();
        assert_eq!(amounts, vec![4, 3, 3]);
        assert_eq!(result.unallocated_bytes, 0);
    }

    #[test]
    fn weighted_shares_match_their_integer_ratio() {
        let result = allocate(
            &policy(600, 0),
            &[
                peer("peer-a", 1, 0, 600),
                peer("peer-b", 2, 0, 600),
                peer("peer-c", 3, 0, 600),
            ],
        )
        .unwrap();
        let amounts: Vec<u64> = result
            .grants
            .iter()
            .map(|grant| grant.ordinary_granted_bytes)
            .collect();
        assert_eq!(amounts, vec![100, 200, 300]);
    }

    #[test]
    fn capped_demand_is_removed_and_its_share_is_redistributed() {
        let result = allocate(
            &policy(100, 0),
            &[
                peer("peer-a", 1, 0, 1),
                peer("peer-b", 1, 0, 100),
                peer("peer-c", 1, 0, 100),
            ],
        )
        .unwrap();
        let amounts: Vec<u64> = result
            .grants
            .iter()
            .map(|grant| grant.ordinary_granted_bytes)
            .collect();
        assert_eq!(amounts, vec![1, 50, 49]);
    }

    #[test]
    fn grants_are_invariant_to_input_order() {
        let inputs = [
            peer("peer-z", 3, 20, 90),
            peer("peer-a", 1, 5, 80),
            peer("peer-m", 2, 10, 70),
        ];
        let forward = allocate(&policy(100, 25), &inputs).unwrap();
        let reversed = allocate(
            &policy(100, 25),
            &inputs.into_iter().rev().collect::<Vec<_>>(),
        )
        .unwrap();
        assert_eq!(forward, reversed);
    }

    #[test]
    fn capacity_is_conserved_and_grants_never_exceed_demand() {
        let demands = vec![
            peer("peer-a", 1, 31, 53),
            peer("peer-b", 2, 41, 79),
            peer("peer-c", 5, 23, 91),
            peer("peer-d", 1, 7, 11),
        ];
        let result = allocate(&policy(177, 45), &demands).unwrap();
        let total = result.total_essential_granted_bytes + result.total_ordinary_granted_bytes;
        assert_eq!(total + result.unallocated_bytes, 177);
        for grant in &result.grants {
            let request = demands
                .iter()
                .find(|item| item.peer_id == grant.peer_id)
                .unwrap();
            assert!(grant.essential_granted_bytes <= request.essential_demand_bytes);
            assert!(grant.ordinary_granted_bytes <= request.ordinary_demand_bytes);
        }
    }

    #[test]
    fn low_total_demand_leaves_capacity_explicitly_unallocated() {
        let result = allocate(
            &policy(100, 20),
            &[peer("peer-a", 1, 3, 4), peer("peer-b", 1, 0, 5)],
        )
        .unwrap();
        assert_eq!(result.total_essential_granted_bytes, 3);
        assert_eq!(result.total_ordinary_granted_bytes, 9);
        assert_eq!(result.unallocated_bytes, 88);
    }

    #[test]
    fn empty_peer_set_has_no_grants_and_preserves_the_budget() {
        let result = allocate(&policy(99, 10), &[]).unwrap();
        assert!(result.grants.is_empty());
        assert_eq!(result.unallocated_bytes, 99);
    }

    #[test]
    fn maximum_u64_budget_does_not_overflow() {
        let result = allocate(&policy(u64::MAX, 0), &[peer("peer-a", 1, 0, u64::MAX)]).unwrap();
        assert_eq!(result.total_ordinary_granted_bytes, u64::MAX);
        assert_eq!(result.unallocated_bytes, 0);
    }
}
