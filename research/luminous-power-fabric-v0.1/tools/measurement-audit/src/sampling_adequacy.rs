use crate::AuditError;

const NS_PER_SECOND: f64 = 1_000_000_000.0;

#[derive(Debug, Clone, PartialEq)]
pub struct AntiAliasFilterContract {
    /// Highest frequency intended to remain in the passband.
    pub passband_edge_hz: f64,
    /// Frequency by which the required attenuation must already be achieved.
    pub stopband_start_hz: f64,
    pub stopband_attenuation_db: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SamplingAdequacyInput {
    pub trace_id: String,
    pub timestamps_ns: Vec<u64>,
    pub nominal_sample_interval_ns: u64,
    pub maximum_relative_interval_jitter: f64,
    /// Declared analog measurement-chain bandwidth, not just ADC clock rate.
    pub instrument_bandwidth_hz: f64,
    /// Highest signal frequency for which the requested engineering claim is made.
    pub claim_bandwidth_hz: f64,
    /// Caller-declared waveform-shape requirement. Nyquist's 2 samples/cycle
    /// is a mathematical minimum, not an adequate engineering waveform target.
    pub minimum_samples_per_cycle: f64,
    pub minimum_stopband_attenuation_db: f64,
    pub anti_alias_filter: Option<AntiAliasFilterContract>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SamplingAdequacyReport {
    pub trace_id: String,
    pub sample_count: usize,
    pub duration_s: f64,
    pub minimum_interval_ns: u64,
    pub maximum_interval_ns: u64,
    /// Maximum relative interval deviation from the declared nominal interval.
    pub maximum_relative_interval_jitter: f64,
    /// Conservative equivalent sample rate based on the largest observed gap.
    pub minimum_observed_sample_rate_hz: f64,
    pub conservative_nyquist_frequency_hz: f64,
    pub samples_per_claim_bandwidth_cycle: f64,
    pub claim_bandwidth_hz: f64,
    pub instrument_bandwidth_hz: f64,
    pub sample_density_adequate: bool,
    pub timestamp_jitter_adequate: bool,
    pub instrument_bandwidth_adequate: bool,
    pub anti_alias_filter_adequate: bool,
    pub sampling_adequate_for_claim: bool,
    pub limitations: Vec<String>,
}

impl SamplingAdequacyReport {
    pub const fn claim_ceiling(&self) -> &'static str {
        "SAMPLING_AND_DECLARED_BANDWIDTH_SCREEN_ONLY_NOT_SENSOR_VALIDATION_NOT_TRANSIENT_OR_HARDWARE_QUALIFICATION"
    }
}

/// Check whether a timestamped trace can support a claim at the declared
/// signal bandwidth.
///
/// The conservative effective rate uses the largest observed sample gap.
/// The caller must supply the analog instrument bandwidth and anti-alias
/// filter contract; sample timestamps alone cannot establish either. The
/// requested samples-per-cycle threshold is explicit because Nyquist's
/// two-sample bound is not sufficient for accurate transient shape fidelity.
pub fn audit_sampling_adequacy(
    input: SamplingAdequacyInput,
) -> Result<SamplingAdequacyReport, AuditError> {
    if input.trace_id.trim().is_empty()
        || input.timestamps_ns.len() < 2
        || input.nominal_sample_interval_ns == 0
    {
        return Err(AuditError(
            "trace identity, timestamp count or nominal sample interval is invalid".to_string(),
        ));
    }

    let scalar_values = [
        input.maximum_relative_interval_jitter,
        input.instrument_bandwidth_hz,
        input.claim_bandwidth_hz,
        input.minimum_samples_per_cycle,
        input.minimum_stopband_attenuation_db,
    ];
    if scalar_values.iter().any(|value| !value.is_finite())
        || input.maximum_relative_interval_jitter < 0.0
        || input.instrument_bandwidth_hz <= 0.0
        || input.claim_bandwidth_hz <= 0.0
        || input.minimum_samples_per_cycle < 2.0
        || input.minimum_stopband_attenuation_db <= 0.0
    {
        return Err(AuditError(
            "sampling adequacy limits and bandwidth values are outside their valid domain"
                .to_string(),
        ));
    }

    if let Some(filter) = &input.anti_alias_filter {
        let filter_values = [
            filter.passband_edge_hz,
            filter.stopband_start_hz,
            filter.stopband_attenuation_db,
        ];
        if filter_values.iter().any(|value| !value.is_finite())
            || filter.passband_edge_hz <= 0.0
            || filter.stopband_start_hz <= filter.passband_edge_hz
            || filter.stopband_attenuation_db <= 0.0
        {
            return Err(AuditError(
                "anti-alias filter contract is non-finite or outside its valid domain".to_string(),
            ));
        }
    }

    let mut minimum_interval_ns = u64::MAX;
    let mut maximum_interval_ns = 0_u64;
    let mut maximum_relative_interval_jitter = 0.0_f64;
    for (index, pair) in input.timestamps_ns.windows(2).enumerate() {
        if pair[1] <= pair[0] {
            return Err(AuditError(format!(
                "timestamp interval {index} is duplicate or decreasing"
            )));
        }
        let gap_ns = pair[1] - pair[0];
        minimum_interval_ns = minimum_interval_ns.min(gap_ns);
        maximum_interval_ns = maximum_interval_ns.max(gap_ns);
        let relative_jitter = gap_ns.abs_diff(input.nominal_sample_interval_ns) as f64
            / input.nominal_sample_interval_ns as f64;
        maximum_relative_interval_jitter =
            maximum_relative_interval_jitter.max(relative_jitter);
    }

    let duration_ns =
        input.timestamps_ns[input.timestamps_ns.len() - 1] - input.timestamps_ns[0];
    let duration_s = duration_ns as f64 / NS_PER_SECOND;
    let minimum_observed_sample_rate_hz = NS_PER_SECOND / maximum_interval_ns as f64;
    let conservative_nyquist_frequency_hz = minimum_observed_sample_rate_hz / 2.0;
    let samples_per_claim_bandwidth_cycle =
        minimum_observed_sample_rate_hz / input.claim_bandwidth_hz;

    let derived = [
        duration_s,
        minimum_observed_sample_rate_hz,
        conservative_nyquist_frequency_hz,
        samples_per_claim_bandwidth_cycle,
        maximum_relative_interval_jitter,
    ];
    if derived.iter().any(|value| !value.is_finite() || *value < 0.0)
        || duration_s <= 0.0
        || minimum_observed_sample_rate_hz <= 0.0
    {
        return Err(AuditError(
            "derived sampling adequacy values are non-finite or outside their valid domain"
                .to_string(),
        ));
    }

    let sample_density_adequate =
        samples_per_claim_bandwidth_cycle >= input.minimum_samples_per_cycle;
    let timestamp_jitter_adequate =
        maximum_relative_interval_jitter <= input.maximum_relative_interval_jitter;
    let instrument_bandwidth_adequate =
        input.instrument_bandwidth_hz >= input.claim_bandwidth_hz;

    let anti_alias_filter_adequate = input.anti_alias_filter.as_ref().is_some_and(|filter| {
        filter.passband_edge_hz >= input.claim_bandwidth_hz
            && filter.stopband_start_hz <= conservative_nyquist_frequency_hz
            && input.instrument_bandwidth_hz >= filter.stopband_start_hz
            && filter.stopband_attenuation_db >= input.minimum_stopband_attenuation_db
    });

    let sampling_adequate_for_claim = sample_density_adequate
        && timestamp_jitter_adequate
        && instrument_bandwidth_adequate
        && anti_alias_filter_adequate;

    let mut limitations = Vec::new();
    if !sample_density_adequate {
        limitations.push(
            "observed sample density is below the declared samples-per-cycle requirement"
                .to_string(),
        );
    }
    if !timestamp_jitter_adequate {
        limitations.push(
            "observed timestamp interval jitter exceeds the declared budget".to_string(),
        );
    }
    if !instrument_bandwidth_adequate {
        limitations.push(
            "declared instrument bandwidth is below the requested claim bandwidth".to_string(),
        );
    }
    if !anti_alias_filter_adequate {
        limitations.push(
            "anti-alias filter is missing or does not meet passband, stopband, Nyquist and attenuation constraints"
                .to_string(),
        );
    }
    limitations.push(
        "sampling adequacy is a metadata and timing screen; it does not validate sensor calibration, analog response, probe loading or the physical signal".to_string(),
    );

    Ok(SamplingAdequacyReport {
        trace_id: input.trace_id,
        sample_count: input.timestamps_ns.len(),
        duration_s,
        minimum_interval_ns,
        maximum_interval_ns,
        maximum_relative_interval_jitter,
        minimum_observed_sample_rate_hz,
        conservative_nyquist_frequency_hz,
        samples_per_claim_bandwidth_cycle,
        claim_bandwidth_hz: input.claim_bandwidth_hz,
        instrument_bandwidth_hz: input.instrument_bandwidth_hz,
        sample_density_adequate,
        timestamp_jitter_adequate,
        instrument_bandwidth_adequate,
        anti_alias_filter_adequate,
        sampling_adequate_for_claim,
        limitations,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        audit_sampling_adequacy, AntiAliasFilterContract, SamplingAdequacyInput,
    };

    fn valid_input() -> SamplingAdequacyInput {
        SamplingAdequacyInput {
            trace_id: "synthetic-fast-daq".to_string(),
            timestamps_ns: vec![0, 100, 200, 300, 400, 500],
            nominal_sample_interval_ns: 100,
            maximum_relative_interval_jitter: 0.01,
            instrument_bandwidth_hz: 2_000_000.0,
            claim_bandwidth_hz: 500_000.0,
            minimum_samples_per_cycle: 10.0,
            minimum_stopband_attenuation_db: 60.0,
            anti_alias_filter: Some(AntiAliasFilterContract {
                passband_edge_hz: 600_000.0,
                stopband_start_hz: 1_000_000.0,
                stopband_attenuation_db: 80.0,
            }),
        }
    }

    #[test]
    fn accepts_trace_with_declared_sample_and_analog_bandwidth_margin() {
        let report = audit_sampling_adequacy(valid_input()).unwrap();

        assert!(report.sampling_adequate_for_claim);
        assert!((report.minimum_observed_sample_rate_hz - 10_000_000.0).abs() < 1.0e-6);
        assert!((report.conservative_nyquist_frequency_hz - 5_000_000.0).abs() < 1.0e-6);
        assert!((report.samples_per_claim_bandwidth_cycle - 20.0).abs() < 1.0e-12);
    }

    #[test]
    fn rejects_50_hz_workload_trace_for_sub_microsecond_transient_claim() {
        let mut input = valid_input();
        input.trace_id = "50-hz-workload-trace".to_string();
        input.timestamps_ns = vec![0, 20_000_000, 40_000_000];
        input.nominal_sample_interval_ns = 20_000_000;
        input.instrument_bandwidth_hz = 25.0;
        input.claim_bandwidth_hz = 875_000.0;

        let report = audit_sampling_adequacy(input).unwrap();

        assert!(!report.sample_density_adequate);
        assert!(!report.instrument_bandwidth_adequate);
        assert!(!report.sampling_adequate_for_claim);
        assert!((report.conservative_nyquist_frequency_hz - 25.0).abs() < 1.0e-12);
    }

    #[test]
    fn rejects_insufficient_timestamp_jitter_budget() {
        let mut input = valid_input();
        input.timestamps_ns = vec![0, 100, 210, 310, 420, 520];

        let report = audit_sampling_adequacy(input).unwrap();

        assert!(!report.timestamp_jitter_adequate);
        assert!(!report.sampling_adequate_for_claim);
    }

    #[test]
    fn rejects_missing_anti_alias_filter_contract() {
        let mut input = valid_input();
        input.anti_alias_filter = None;

        let report = audit_sampling_adequacy(input).unwrap();

        assert!(!report.anti_alias_filter_adequate);
        assert!(!report.sampling_adequate_for_claim);
    }

    #[test]
    fn rejects_instrument_bandwidth_below_claim() {
        let mut input = valid_input();
        input.instrument_bandwidth_hz = 400_000.0;

        let report = audit_sampling_adequacy(input).unwrap();

        assert!(!report.instrument_bandwidth_adequate);
        assert!(!report.sampling_adequate_for_claim);
    }

    #[test]
    fn rejects_filter_stopband_above_conservative_nyquist() {
        let mut input = valid_input();
        input.anti_alias_filter.as_mut().unwrap().stopband_start_hz = 6_000_000.0;
        input.instrument_bandwidth_hz = 8_000_000.0;

        let report = audit_sampling_adequacy(input).unwrap();

        assert!(!report.anti_alias_filter_adequate);
        assert!(!report.sampling_adequate_for_claim);
    }

    #[test]
    fn rejects_duplicate_or_decreasing_timestamps() {
        let mut duplicate = valid_input();
        duplicate.timestamps_ns[2] = duplicate.timestamps_ns[1];
        assert!(audit_sampling_adequacy(duplicate).is_err());

        let mut decreasing = valid_input();
        decreasing.timestamps_ns.swap(2, 3);
        assert!(audit_sampling_adequacy(decreasing).is_err());
    }

    #[test]
    fn rejects_invalid_sample_rate_and_nonfinite_constraints() {
        let mut zero_interval = valid_input();
        zero_interval.nominal_sample_interval_ns = 0;
        assert!(audit_sampling_adequacy(zero_interval).is_err());

        let mut nonfinite = valid_input();
        nonfinite.claim_bandwidth_hz = f64::INFINITY;
        assert!(audit_sampling_adequacy(nonfinite).is_err());

        let mut too_few = valid_input();
        too_few.timestamps_ns = vec![0];
        assert!(audit_sampling_adequacy(too_few).is_err());
    }
}