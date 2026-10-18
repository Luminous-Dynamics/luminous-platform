//! Deterministic, privilege-free connectivity path selection.
//!
//! This crate evaluates caller-supplied link observations and declared user intent.
//! It does not inspect packets/content, probe networks, change routes, activate SIM/eSIM
//! profiles, perform radio transmission, or claim carrier/device authorization.
//! An adapter must re-check live state and authorization before any actuation.

use std::cmp::Ordering;
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum LinkKind {
    LocalService,
    Ethernet,
    Wifi,
    Mesh,
    Cellular,
    Satellite,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum TrustLevel {
    Untrusted,
    Authenticated,
    Trusted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum TrafficIntent {
    LocalOnly,
    RealtimeInteractive,
    Messaging,
    GeneralBrowsing,
    BackgroundSync,
    BulkTransfer,
    SensitiveData,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LinkCandidate {
    /// Stable, non-secret identifier. Never use SSIDs, phone numbers, IMSIs, or raw
    /// subscriber identifiers as this value.
    pub id: String,
    pub kind: LinkKind,
    pub available: bool,
    /// Must be set only after a separate policy/contract adapter checks authorization.
    pub authorized: bool,
    pub internet_available: bool,
    pub local_service_available: bool,
    pub metered: bool,
    pub trust: TrustLevel,
    pub observed_age_secs: u32,
    /// Cost observations use minor units per GiB and the stated ISO-like currency code.
    pub cost_currency: Option<String>,
    pub cost_per_gib_minor_units: Option<u64>,
    pub latency_p95_ms: Option<u32>,
    /// Reliability estimate in basis points: 10_000 == 100%.
    pub reliability_bps: Option<u16>,
    pub downstream_mbps: Option<u32>,
    /// Remaining allowance for metered links; None means unknown.
    pub remaining_data_bytes: Option<u64>,
    pub energy_mwh_per_gib: Option<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectionPolicy {
    pub intent: TrafficIntent,
    pub cost_currency: String,
    /// A hard ceiling. Missing/incomparable price is excluded when this is set.
    pub max_cost_per_gib_minor_units: Option<u64>,
    pub allow_metered: bool,
    pub max_latency_p95_ms: Option<u32>,
    pub minimum_reliability_bps: Option<u16>,
    pub minimum_downstream_mbps: Option<u32>,
    pub minimum_trust: TrustLevel,
    pub max_observation_age_secs: u32,
    /// Applies to metered links only; unknown allowance is excluded.
    pub required_transfer_bytes: Option<u64>,
}

impl SelectionPolicy {
    /// Construct a conservative general-purpose policy. Set explicit service
    /// constraints before deployment; this constructor is mainly for experiments.
    pub fn permissive(intent: TrafficIntent, cost_currency: impl Into<String>) -> Self {
        Self {
            intent,
            cost_currency: cost_currency.into(),
            max_cost_per_gib_minor_units: None,
            allow_metered: true,
            max_latency_p95_ms: None,
            minimum_reliability_bps: None,
            minimum_downstream_mbps: None,
            minimum_trust: TrustLevel::Untrusted,
            max_observation_age_secs: 60,
            required_transfer_bytes: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum ExclusionReason {
    LinkUnavailable,
    AdapterNotAuthorized,
    ObservationStale,
    InternetUnavailable,
    LocalServiceUnavailable,
    MeteredLinkNotAllowed,
    CostUnknownUnderHardLimit,
    CostCurrencyUnknownOrInvalid,
    CostCurrencyMismatch,
    CostAboveHardLimit,
    LatencyUnknownUnderHardLimit,
    LatencyAboveHardLimit,
    ReliabilityUnknownUnderHardLimit,
    ReliabilityBelowMinimum,
    ReliabilityObservationOutOfRange,
    ThroughputUnknownUnderMinimum,
    ThroughputBelowMinimum,
    RemainingAllowanceUnknown,
    RemainingAllowanceInsufficient,
    TrustBelowMinimum,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RankingCriterion {
    LocalServiceFirst,
    TrustDescending,
    ReliabilityDescending,
    LatencyAscending,
    CostAscending,
    EnergyAscending,
    ThroughputDescending,
    LinkIdAscending,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NoRouteReason {
    NoEligibleCandidate,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CandidateAssessment {
    pub link_id: String,
    pub eligible: bool,
    /// Empty when eligible. Multiple hard-constraint failures are all reported.
    pub exclusion_reasons: Vec<ExclusionReason>,
    /// One-based deterministic ranking; absent for excluded candidates.
    pub rank: Option<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Decision {
    pub intent: TrafficIntent,
    pub selected_link_id: Option<String>,
    pub alternatives: Vec<String>,
    pub ranking_criteria: Vec<RankingCriterion>,
    pub assessments: Vec<CandidateAssessment>,
    pub no_route_reason: Option<NoRouteReason>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecisionError {
    InvalidPolicyCurrency,
    ReliabilityThresholdOutOfRange,
    DuplicateLinkId,
}

fn valid_currency(code: &str) -> bool {
    code.len() == 3 && code.bytes().all(|byte| byte.is_ascii_uppercase())
}

fn cost_in_policy_currency(
    candidate: &LinkCandidate,
    policy: &SelectionPolicy,
) -> Option<u64> {
    if candidate.cost_currency.as_deref() == Some(policy.cost_currency.as_str()) {
        candidate.cost_per_gib_minor_units
    } else {
        None
    }
}

fn trust_meets_minimum(candidate: &LinkCandidate, policy: &SelectionPolicy) -> bool {
    let required = if policy.intent == TrafficIntent::SensitiveData {
        policy.minimum_trust.max(TrustLevel::Trusted)
    } else {
        policy.minimum_trust
    };
    candidate.trust >= required
}

fn exclusion_reasons(
    candidate: &LinkCandidate,
    policy: &SelectionPolicy,
) -> Vec<ExclusionReason> {
    let mut reasons = Vec::new();

    if !candidate.available {
        reasons.push(ExclusionReason::LinkUnavailable);
    }
    if !candidate.authorized {
        reasons.push(ExclusionReason::AdapterNotAuthorized);
    }
    if candidate.observed_age_secs > policy.max_observation_age_secs {
        reasons.push(ExclusionReason::ObservationStale);
    }

    if policy.intent == TrafficIntent::LocalOnly && !candidate.local_service_available {
        reasons.push(ExclusionReason::LocalServiceUnavailable);
    } else if policy.intent != TrafficIntent::LocalOnly && !candidate.internet_available {
        reasons.push(ExclusionReason::InternetUnavailable);
    }

    if candidate.metered && !policy.allow_metered {
        reasons.push(ExclusionReason::MeteredLinkNotAllowed);
    }

    if let Some(max_cost) = policy.max_cost_per_gib_minor_units {
        match (candidate.cost_currency.as_deref(), candidate.cost_per_gib_minor_units) {
            (None, _) | (_, None) => {
                reasons.push(ExclusionReason::CostUnknownUnderHardLimit);
            }
            (Some(currency), Some(_)) if !valid_currency(currency) => {
                reasons.push(ExclusionReason::CostCurrencyUnknownOrInvalid);
            }
            (Some(currency), Some(_)) if currency != policy.cost_currency => {
                reasons.push(ExclusionReason::CostCurrencyMismatch);
            }
            (Some(_), Some(cost)) if cost > max_cost => {
                reasons.push(ExclusionReason::CostAboveHardLimit);
            }
            _ => {}
        }
    }

    if let Some(max_latency) = policy.max_latency_p95_ms {
        match candidate.latency_p95_ms {
            None => reasons.push(ExclusionReason::LatencyUnknownUnderHardLimit),
            Some(latency) if latency > max_latency => {
                reasons.push(ExclusionReason::LatencyAboveHardLimit);
            }
            _ => {}
        }
    }

    if let Some(min_reliability) = policy.minimum_reliability_bps {
        match candidate.reliability_bps {
            None => reasons.push(ExclusionReason::ReliabilityUnknownUnderHardLimit),
            Some(value) if value > 10_000 => {
                reasons.push(ExclusionReason::ReliabilityObservationOutOfRange);
            }
            Some(value) if value < min_reliability => {
                reasons.push(ExclusionReason::ReliabilityBelowMinimum);
            }
            _ => {}
        }
    } else if matches!(candidate.reliability_bps, Some(value) if value > 10_000) {
        reasons.push(ExclusionReason::ReliabilityObservationOutOfRange);
    }

    if let Some(min_throughput) = policy.minimum_downstream_mbps {
        match candidate.downstream_mbps {
            None => reasons.push(ExclusionReason::ThroughputUnknownUnderMinimum),
            Some(value) if value < min_throughput => {
                reasons.push(ExclusionReason::ThroughputBelowMinimum);
            }
            _ => {}
        }
    }

    if candidate.metered && let Some(required) = policy.required_transfer_bytes {
        match candidate.remaining_data_bytes {
            None => reasons.push(ExclusionReason::RemainingAllowanceUnknown),
            Some(remaining) if remaining < required => {
                reasons.push(ExclusionReason::RemainingAllowanceInsufficient);
            }
            _ => {}
        }
    }

    if !trust_meets_minimum(candidate, policy) {
        reasons.push(ExclusionReason::TrustBelowMinimum);
    }

    reasons.sort();
    reasons.dedup();
    reasons
}

fn compare_option_ascending<T: Ord>(left: Option<T>, right: Option<T>) -> Ordering {
    match (left, right) {
        (Some(a), Some(b)) => a.cmp(&b),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

fn compare_option_descending<T: Ord>(left: Option<T>, right: Option<T>) -> Ordering {
    compare_option_ascending(right, left)
}

fn compare_cost(
    left: &LinkCandidate,
    right: &LinkCandidate,
    policy: &SelectionPolicy,
) -> Ordering {
    compare_option_ascending(
        cost_in_policy_currency(left, policy),
        cost_in_policy_currency(right, policy),
    )
}

fn compare_candidates(
    left: &LinkCandidate,
    right: &LinkCandidate,
    policy: &SelectionPolicy,
) -> Ordering {
    let comparison = match policy.intent {
        TrafficIntent::LocalOnly => {
            let left_local = left.kind == LinkKind::LocalService;
            let right_local = right.kind == LinkKind::LocalService;
            right_local
                .cmp(&left_local)
                .then_with(|| compare_option_ascending(left.latency_p95_ms, right.latency_p95_ms))
                .then_with(|| right.trust.cmp(&left.trust))
                .then_with(|| compare_option_descending(left.reliability_bps, right.reliability_bps))
        }
        TrafficIntent::RealtimeInteractive => {
            compare_option_ascending(left.latency_p95_ms, right.latency_p95_ms)
                .then_with(|| compare_option_descending(left.reliability_bps, right.reliability_bps))
                .then_with(|| compare_option_descending(left.downstream_mbps, right.downstream_mbps))
                .then_with(|| compare_cost(left, right, policy))
        }
        TrafficIntent::Messaging => {
            compare_option_descending(left.reliability_bps, right.reliability_bps)
                .then_with(|| compare_option_ascending(left.latency_p95_ms, right.latency_p95_ms))
                .then_with(|| compare_cost(left, right, policy))
                .then_with(|| right.trust.cmp(&left.trust))
        }
        TrafficIntent::GeneralBrowsing => {
            compare_cost(left, right, policy)
                .then_with(|| compare_option_ascending(left.latency_p95_ms, right.latency_p95_ms))
                .then_with(|| compare_option_descending(left.reliability_bps, right.reliability_bps))
                .then_with(|| compare_option_descending(left.downstream_mbps, right.downstream_mbps))
        }
        TrafficIntent::BackgroundSync => {
            compare_cost(left, right, policy)
                .then_with(|| compare_option_ascending(left.energy_mwh_per_gib, right.energy_mwh_per_gib))
                .then_with(|| compare_option_descending(left.reliability_bps, right.reliability_bps))
                .then_with(|| compare_option_descending(left.downstream_mbps, right.downstream_mbps))
        }
        TrafficIntent::BulkTransfer => {
            compare_cost(left, right, policy)
                .then_with(|| compare_option_descending(left.downstream_mbps, right.downstream_mbps))
                .then_with(|| compare_option_ascending(left.energy_mwh_per_gib, right.energy_mwh_per_gib))
                .then_with(|| compare_option_descending(left.reliability_bps, right.reliability_bps))
        }
        TrafficIntent::SensitiveData => {
            right.trust.cmp(&left.trust)
                .then_with(|| compare_option_descending(left.reliability_bps, right.reliability_bps))
                .then_with(|| compare_option_ascending(left.latency_p95_ms, right.latency_p95_ms))
                .then_with(|| compare_cost(left, right, policy))
        }
    };

    comparison.then_with(|| left.id.cmp(&right.id))
}

fn ranking_criteria(intent: TrafficIntent) -> Vec<RankingCriterion> {
    use RankingCriterion::*;
    match intent {
        TrafficIntent::LocalOnly => vec![
            LocalServiceFirst,
            LatencyAscending,
            TrustDescending,
            ReliabilityDescending,
            LinkIdAscending,
        ],
        TrafficIntent::RealtimeInteractive => vec![
            LatencyAscending,
            ReliabilityDescending,
            ThroughputDescending,
            CostAscending,
            LinkIdAscending,
        ],
        TrafficIntent::Messaging => vec![
            ReliabilityDescending,
            LatencyAscending,
            CostAscending,
            TrustDescending,
            LinkIdAscending,
        ],
        TrafficIntent::GeneralBrowsing => vec![
            CostAscending,
            LatencyAscending,
            ReliabilityDescending,
            ThroughputDescending,
            LinkIdAscending,
        ],
        TrafficIntent::BackgroundSync => vec![
            CostAscending,
            EnergyAscending,
            ReliabilityDescending,
            ThroughputDescending,
            LinkIdAscending,
        ],
        TrafficIntent::BulkTransfer => vec![
            CostAscending,
            ThroughputDescending,
            EnergyAscending,
            ReliabilityDescending,
            LinkIdAscending,
        ],
        TrafficIntent::SensitiveData => vec![
            TrustDescending,
            ReliabilityDescending,
            LatencyAscending,
            CostAscending,
            LinkIdAscending,
        ],
    }
}

/// Evaluate candidate links. Hard constraints precede intent-specific ranking.
/// Input order does not decide ties; the stable link identifier is the final tie-break.
///
/// The caller must re-check live state and authorization before acting on the decision.
pub fn choose_link(
    policy: &SelectionPolicy,
    candidates: &[LinkCandidate],
) -> Result<Decision, DecisionError> {
    if !valid_currency(&policy.cost_currency) {
        return Err(DecisionError::InvalidPolicyCurrency);
    }
    if matches!(policy.minimum_reliability_bps, Some(value) if value > 10_000) {
        return Err(DecisionError::ReliabilityThresholdOutOfRange);
    }

    let mut seen_ids = BTreeSet::new();
    for candidate in candidates {
        if !seen_ids.insert(candidate.id.as_str()) {
            return Err(DecisionError::DuplicateLinkId);
        }
    }

    let mut evaluated: Vec<(LinkCandidate, Vec<ExclusionReason>)> = candidates
        .iter()
        .cloned()
        .map(|candidate| {
            let reasons = exclusion_reasons(&candidate, policy);
            (candidate, reasons)
        })
        .collect();

    let mut eligible: Vec<LinkCandidate> = evaluated
        .iter()
        .filter(|(_, reasons)| reasons.is_empty())
        .map(|(candidate, _)| candidate.clone())
        .collect();
    eligible.sort_by(|left, right| compare_candidates(left, right, policy));

    let eligible_order: Vec<String> = eligible.iter().map(|candidate| candidate.id.clone()).collect();
    let selected_link_id = eligible_order.first().cloned();
    let alternatives = eligible_order.iter().skip(1).cloned().collect();

    let mut assessments: Vec<CandidateAssessment> = evaluated
        .drain(..)
        .map(|(candidate, reasons)| {
            let rank = eligible_order
                .iter()
                .position(|link_id| link_id == &candidate.id)
                .map(|zero_based| zero_based + 1);
            CandidateAssessment {
                link_id: candidate.id,
                eligible: reasons.is_empty(),
                exclusion_reasons: reasons,
                rank,
            }
        })
        .collect();
    assessments.sort_by(|left, right| left.link_id.cmp(&right.link_id));

    Ok(Decision {
        intent: policy.intent,
        no_route_reason: if selected_link_id.is_none() {
            Some(NoRouteReason::NoEligibleCandidate)
        } else {
            None
        },
        selected_link_id,
        alternatives,
        ranking_criteria: ranking_criteria(policy.intent),
        assessments,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(id: &str, kind: LinkKind) -> LinkCandidate {
        LinkCandidate {
            id: id.to_owned(),
            kind,
            available: true,
            authorized: true,
            internet_available: true,
            local_service_available: false,
            metered: true,
            trust: TrustLevel::Authenticated,
            observed_age_secs: 1,
            cost_currency: Some("ZAR".to_owned()),
            cost_per_gib_minor_units: Some(100),
            latency_p95_ms: Some(40),
            reliability_bps: Some(9_900),
            downstream_mbps: Some(50),
            remaining_data_bytes: Some(10_000_000_000),
            energy_mwh_per_gib: Some(100),
        }
    }

    fn policy(intent: TrafficIntent) -> SelectionPolicy {
        SelectionPolicy::permissive(intent, "ZAR")
    }

    #[test]
    fn background_sync_prefers_cheaper_eligible_link() {
        let policy = policy(TrafficIntent::BackgroundSync);
        let mut cheap = candidate("wifi-cheap", LinkKind::Wifi);
        cheap.cost_per_gib_minor_units = Some(10);
        let expensive = candidate("cellular-expensive", LinkKind::Cellular);
        let result = choose_link(&policy, &[expensive, cheap]).expect("valid policy");
        assert_eq!(result.selected_link_id.as_deref(), Some("wifi-cheap"));
        assert_eq!(result.alternatives, vec!["cellular-expensive"]);
    }

    #[test]
    fn hard_cost_cap_excludes_better_but_too_expensive_link() {
        let mut policy = policy(TrafficIntent::RealtimeInteractive);
        policy.max_cost_per_gib_minor_units = Some(200);
        let mut expensive = candidate("fast-expensive", LinkKind::Cellular);
        expensive.cost_per_gib_minor_units = Some(201);
        expensive.latency_p95_ms = Some(1);
        let affordable = candidate("slower-affordable", LinkKind::Wifi);
        let result = choose_link(&policy, &[expensive, affordable]).expect("valid policy");
        assert_eq!(result.selected_link_id.as_deref(), Some("slower-affordable"));
        let excluded = result.assessments.iter().find(|x| x.link_id == "fast-expensive").unwrap();
        assert!(excluded.exclusion_reasons.contains(&ExclusionReason::CostAboveHardLimit));
    }

    #[test]
    fn unknown_or_wrong_currency_price_cannot_pass_a_cost_ceiling() {
        let mut policy = policy(TrafficIntent::GeneralBrowsing);
        policy.max_cost_per_gib_minor_units = Some(500);
        let mut unknown = candidate("unknown-price", LinkKind::Wifi);
        unknown.cost_currency = None;
        unknown.cost_per_gib_minor_units = None;
        let mut foreign = candidate("foreign-price", LinkKind::Cellular);
        foreign.cost_currency = Some("USD".to_owned());
        let result = choose_link(&policy, &[unknown, foreign]).expect("valid policy");
        assert_eq!(result.selected_link_id, None);
        assert_eq!(result.no_route_reason, Some(NoRouteReason::NoEligibleCandidate));
    }

    #[test]
    fn stale_observations_are_never_eligible() {
        let mut policy = policy(TrafficIntent::GeneralBrowsing);
        policy.max_observation_age_secs = 30;
        let mut stale = candidate("stale", LinkKind::Wifi);
        stale.observed_age_secs = 31;
        let result = choose_link(&policy, &[stale]).expect("valid policy");
        assert_eq!(result.selected_link_id, None);
        assert!(result.assessments[0].exclusion_reasons.contains(&ExclusionReason::ObservationStale));
    }

    #[test]
    fn local_only_service_survives_without_global_internet() {
        let policy = policy(TrafficIntent::LocalOnly);
        let mut local = candidate("community-local", LinkKind::LocalService);
        local.internet_available = false;
        local.local_service_available = true;
        local.metered = false;
        local.trust = TrustLevel::Trusted;
        let result = choose_link(&policy, &[local]).expect("valid policy");
        assert_eq!(result.selected_link_id.as_deref(), Some("community-local"));
    }

    #[test]
    fn sensitive_intent_rejects_untrusted_path_even_when_fast_and_free() {
        let policy = policy(TrafficIntent::SensitiveData);
        let mut untrusted = candidate("public-hotspot", LinkKind::Wifi);
        untrusted.trust = TrustLevel::Untrusted;
        untrusted.cost_per_gib_minor_units = Some(0);
        untrusted.latency_p95_ms = Some(1);
        let trusted = {
            let mut link = candidate("trusted-wan", LinkKind::Ethernet);
            link.metered = false;
            link.trust = TrustLevel::Trusted;
            link
        };
        let result = choose_link(&policy, &[untrusted, trusted]).expect("valid policy");
        assert_eq!(result.selected_link_id.as_deref(), Some("trusted-wan"));
        let excluded = result.assessments.iter().find(|x| x.link_id == "public-hotspot").unwrap();
        assert!(excluded.exclusion_reasons.contains(&ExclusionReason::TrustBelowMinimum));
    }

    #[test]
    fn unauthorized_adapter_is_never_selected() {
        let policy = policy(TrafficIntent::GeneralBrowsing);
        let mut unauthorized = candidate("unauthorized-satellite", LinkKind::Satellite);
        unauthorized.authorized = false;
        unauthorized.cost_per_gib_minor_units = Some(0);
        let result = choose_link(&policy, &[unauthorized]).expect("valid policy");
        assert_eq!(result.selected_link_id, None);
        assert!(result.assessments[0].exclusion_reasons.contains(&ExclusionReason::AdapterNotAuthorized));
    }

    #[test]
    fn ranking_is_independent_of_candidate_input_order() {
        let policy = policy(TrafficIntent::Messaging);
        let a = candidate("link-b", LinkKind::Wifi);
        let mut b = candidate("link-a", LinkKind::Ethernet);
        b.reliability_bps = a.reliability_bps;
        let one = choose_link(&policy, &[a.clone(), b.clone()]).expect("valid policy");
        let two = choose_link(&policy, &[b, a]).expect("valid policy");
        assert_eq!(one, two);
    }

    #[test]
    fn no_eligible_path_is_explicit_and_has_reasons() {
        let mut policy = policy(TrafficIntent::RealtimeInteractive);
        policy.max_latency_p95_ms = Some(20);
        let mut link = candidate("high-latency", LinkKind::Satellite);
        link.latency_p95_ms = Some(300);
        let result = choose_link(&policy, &[link]).expect("valid policy");
        assert_eq!(result.selected_link_id, None);
        assert_eq!(result.no_route_reason, Some(NoRouteReason::NoEligibleCandidate));
        assert!(result.assessments[0].exclusion_reasons.contains(&ExclusionReason::LatencyAboveHardLimit));
    }

    #[test]
    fn metered_transfer_requires_known_remaining_allowance() {
        let mut policy = policy(TrafficIntent::BulkTransfer);
        policy.required_transfer_bytes = Some(5_000_000);
        let mut link = candidate("metered-unknown-remaining", LinkKind::Cellular);
        link.remaining_data_bytes = None;
        let result = choose_link(&policy, &[link]).expect("valid policy");
        assert_eq!(result.selected_link_id, None);
        assert!(result.assessments[0].exclusion_reasons.contains(&ExclusionReason::RemainingAllowanceUnknown));
    }

    #[test]
    fn invalid_policy_and_duplicate_identifiers_fail_explicitly() {
        let mut invalid_currency = policy(TrafficIntent::Messaging);
        invalid_currency.cost_currency = "zar".to_owned();
        assert_eq!(choose_link(&invalid_currency, &[]), Err(DecisionError::InvalidPolicyCurrency));

        let duplicate_policy = policy(TrafficIntent::Messaging);
        let links = [
            candidate("duplicate", LinkKind::Wifi),
            candidate("duplicate", LinkKind::Cellular),
        ];
        assert_eq!(choose_link(&duplicate_policy, &links), Err(DecisionError::DuplicateLinkId));
    }
    #[test]
    fn reliability_and_throughput_contracts_are_hard_constraints() {
        let mut policy = policy(TrafficIntent::RealtimeInteractive);
        policy.minimum_reliability_bps = Some(9_900);
        policy.minimum_downstream_mbps = Some(25);
        let mut weak = candidate("weak", LinkKind::Wifi);
        weak.reliability_bps = Some(9_899);
        weak.downstream_mbps = Some(100);
        let mut slow = candidate("slow", LinkKind::Cellular);
        slow.reliability_bps = Some(9_999);
        slow.downstream_mbps = Some(24);
        let result = choose_link(&policy, &[weak, slow]).expect("valid policy");
        assert_eq!(result.selected_link_id, None);
        assert!(result.assessments.iter().any(|item| {
            item.exclusion_reasons.contains(&ExclusionReason::ReliabilityBelowMinimum)
        }));
        assert!(result.assessments.iter().any(|item| {
            item.exclusion_reasons.contains(&ExclusionReason::ThroughputBelowMinimum)
        }));
    }

    #[test]
    fn reliability_values_outside_basis_point_range_are_rejected() {
        let policy = policy(TrafficIntent::Messaging);
        let mut invalid = candidate("bad-measurement", LinkKind::Wifi);
        invalid.reliability_bps = Some(10_001);
        let result = choose_link(&policy, &[invalid]).expect("valid policy");
        assert_eq!(result.selected_link_id, None);
        assert!(result.assessments[0].exclusion_reasons.contains(
            &ExclusionReason::ReliabilityObservationOutOfRange
        ));
        let mut invalid_policy = policy(TrafficIntent::Messaging);
        invalid_policy.minimum_reliability_bps = Some(10_001);
        assert_eq!(
            choose_link(&invalid_policy, &[]),
            Err(DecisionError::ReliabilityThresholdOutOfRange)
        );
    }

}
