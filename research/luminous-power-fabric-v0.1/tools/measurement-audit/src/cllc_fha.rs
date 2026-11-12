use crate::AuditError;

/// Direction-specific equivalent quality factor for a symmetric CLLC tank.
///
/// Charging and discharging generally have different equivalent AC load
/// resistances, so the caller must supply the Q calculated for the declared
/// direction and operating point. This enum prevents silently reusing one Q
/// while claiming that both directions were evaluated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClLcDirection {
    Charging,
    Discharging,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClLcFhaInput {
    /// k_L = L_m / L_rp, with the symmetric tank referred to the active side.
    pub magnetizing_to_resonant_inductance_ratio: f64,
    /// Direction-specific equivalent quality factor from the declared load.
    pub quality_factor: f64,
    /// omega_s / omega_r1, strictly positive.
    pub normalized_frequency: f64,
    pub direction: ClLcDirection,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClLcFhaResult {
    pub direction: ClLcDirection,
    pub magnetizing_to_resonant_inductance_ratio: f64,
    pub quality_factor: f64,
    pub normalized_frequency: f64,
    pub normalized_voltage_gain: f64,
}

impl ClLcFhaResult {
    pub const fn claim_ceiling(&self) -> &'static str {
        "SYMMETRIC_CLLC_FHA_GAIN_SCREEN_ONLY_NOT_IMPEDANCE_ZVS_LOSS_OR_QUALIFICATION"
    }
}

/// Evaluate the ideal symmetric CLLC first-harmonic-approximation (FHA) gain.
///
/// Based on the symmetric tank FHA expression in Tao et al., Electronics 2026,
/// 15(21), 4918, Eqs. (4) and (5). The functional expression is the same in both
/// directions, but charging and discharging use different equivalent load
/// quality factors.
///
/// M = 1 / sqrt(A^2 + B^2)
///
/// A = 1 + 1/k_L - 1/(k_L * omega_n^2)
///
/// B = (Q/k_L) * ((2*k_L+1)*omega_n - (2*k_L+2)/omega_n + 1/omega_n^3)
///
/// At omega_n = 1, this ideal symmetric FHA expression gives M = 1 independent
/// of Q and k_L.
///
/// This function does NOT determine the physical turns ratio, tank components,
/// input impedance phase, ZVS/ZCS, magnetizing energy vs Coss/dead time, gain
/// monotonicity over a whole operating band, circulating current, RMS/peak
/// current, device/magnetic losses, thermal limits, transient response, or
/// controller timing. Those are separate constraints and models.
pub fn evaluate_cllc_fha_gain(input: ClLcFhaInput) -> Result<ClLcFhaResult, AuditError> {
    let k_l = input.magnetizing_to_resonant_inductance_ratio;
    let q = input.quality_factor;
    let omega_n = input.normalized_frequency;

    if !k_l.is_finite() || k_l <= 0.0 {
        return Err(AuditError(
            "CLLC magnetizing-to-resonant inductance ratio k_L must be positive and finite"
                .to_string(),
        ));
    }
    if !q.is_finite() || q <= 0.0 {
        return Err(AuditError(
            "CLLC direction-specific quality factor Q must be positive and finite".to_string(),
        ));
    }
    if !omega_n.is_finite() || omega_n <= 0.0 {
        return Err(AuditError(
            "CLLC normalized frequency must be positive and finite".to_string(),
        ));
    }

    let omega_n_sq = omega_n * omega_n;
    let omega_n_cubed = omega_n_sq * omega_n;
    if !omega_n_sq.is_finite() || !omega_n_cubed.is_finite() || omega_n_cubed == 0.0 {
        return Err(AuditError(
            "CLLC normalized frequency powers overflowed or underflowed".to_string(),
        ));
    }

    let a = 1.0 + 1.0 / k_l - 1.0 / (k_l * omega_n_sq);
    let resonant_bracket = (2.0 * k_l + 1.0) * omega_n
        - (2.0 * k_l + 2.0) / omega_n
        + 1.0 / omega_n_cubed;
    let b = (q / k_l) * resonant_bracket;
    let denominator_sq = a * a + b * b;

    if !a.is_finite()
        || !resonant_bracket.is_finite()
        || !b.is_finite()
        || !denominator_sq.is_finite()
        || denominator_sq <= 0.0
    {
        return Err(AuditError(
            "CLLC FHA gain calculation is outside the finite numerical domain".to_string(),
        ));
    }

    let normalized_voltage_gain = 1.0 / denominator_sq.sqrt();
    if !normalized_voltage_gain.is_finite() || normalized_voltage_gain <= 0.0 {
        return Err(AuditError(
            "CLLC FHA gain is not positive and finite".to_string(),
        ));
    }

    Ok(ClLcFhaResult {
        direction: input.direction,
        magnetizing_to_resonant_inductance_ratio: k_l,
        quality_factor: q,
        normalized_frequency: omega_n,
        normalized_voltage_gain,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClLcSampledTrend {
    Increasing,
    Decreasing,
    NonDecreasing,
    NonIncreasing,
    Flat,
    NonMonotonic,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClLcGainPoint {
    pub normalized_frequency: f64,
    pub normalized_voltage_gain: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClLcFhaCurveResult {
    pub direction: ClLcDirection,
    pub magnetizing_to_resonant_inductance_ratio: f64,
    pub quality_factor: f64,
    pub points: Vec<ClLcGainPoint>,
    /// Classification applies only to the supplied discrete frequency samples.
    /// It is not a proof of continuous-band monotonicity.
    pub sampled_trend: ClLcSampledTrend,
}

impl ClLcFhaCurveResult {
    pub const fn claim_ceiling(&self) -> &'static str {
        "DISCRETE_CLLC_FHA_GAIN_SCAN_ONLY_NOT_CONTINUOUS_MONOTONICITY_OR_ZVS_PROOF"
    }
}

/// Evaluate the FHA gain over a strictly increasing list of normalized frequencies.
///
/// This is useful for *screening* gain branches and identifying sampled turning
/// points. A finite grid cannot prove that the continuous gain curve is monotonic,
/// and gain monotonicity alone does not prove inductive input impedance or ZVS.
pub fn scan_cllc_fha_gain_curve(
    magnetizing_to_resonant_inductance_ratio: f64,
    quality_factor: f64,
    direction: ClLcDirection,
    normalized_frequencies: &[f64],
) -> Result<ClLcFhaCurveResult, AuditError> {
    if normalized_frequencies.len() < 2 {
        return Err(AuditError(
            "CLLC gain scan requires at least two normalized-frequency samples".to_string(),
        ));
    }

    let mut points = Vec::with_capacity(normalized_frequencies.len());
    for (index, &frequency) in normalized_frequencies.iter().enumerate() {
        if index > 0 && frequency <= normalized_frequencies[index - 1] {
            return Err(AuditError(
                "CLLC gain scan frequencies must be strictly increasing".to_string(),
            ));
        }
        let evaluated = evaluate_cllc_fha_gain(ClLcFhaInput {
            magnetizing_to_resonant_inductance_ratio,
            quality_factor,
            normalized_frequency: frequency,
            direction,
        })?;
        points.push(ClLcGainPoint {
            normalized_frequency: frequency,
            normalized_voltage_gain: evaluated.normalized_voltage_gain,
        });
    }

    let mut non_decreasing = true;
    let mut non_increasing = true;
    for pair in points.windows(2) {
        if pair[1].normalized_voltage_gain < pair[0].normalized_voltage_gain {
            non_decreasing = false;
        }
        if pair[1].normalized_voltage_gain > pair[0].normalized_voltage_gain {
            non_increasing = false;
        }
    }

    let sampled_trend = match (non_decreasing, non_increasing) {
        (true, true) => ClLcSampledTrend::Flat,
        (true, false) => {
            if points.windows(2).all(|p| p[1].normalized_voltage_gain > p[0].normalized_voltage_gain) {
                ClLcSampledTrend::Increasing
            } else {
                ClLcSampledTrend::NonDecreasing
            }
        }
        (false, true) => {
            if points.windows(2).all(|p| p[1].normalized_voltage_gain < p[0].normalized_voltage_gain) {
                ClLcSampledTrend::Decreasing
            } else {
                ClLcSampledTrend::NonIncreasing
            }
        }
        (false, false) => ClLcSampledTrend::NonMonotonic,
    };

    Ok(ClLcFhaCurveResult {
        direction,
        magnetizing_to_resonant_inductance_ratio,
        quality_factor,
        points,
        sampled_trend,
    })
}

#[cfg(test)]
mod tests {
    use super::{evaluate_cllc_fha_gain, ClLcDirection, ClLcFhaInput};

    fn input(q: f64, normalized_frequency: f64) -> ClLcFhaInput {
        ClLcFhaInput {
            magnetizing_to_resonant_inductance_ratio: 3.51,
            quality_factor: q,
            normalized_frequency,
            direction: ClLcDirection::Charging,
        }
    }

    #[test]
    fn normalized_gain_is_unity_at_resonance_for_any_positive_q_and_k() {
        for k_l in [0.5, 1.0, 3.51, 8.0] {
            for q in [0.01, 0.3, 1.0, 4.0] {
                let result = evaluate_cllc_fha_gain(ClLcFhaInput {
                    magnetizing_to_resonant_inductance_ratio: k_l,
                    quality_factor: q,
                    normalized_frequency: 1.0,
                    direction: ClLcDirection::Charging,
                })
                .unwrap();
                assert!((result.normalized_voltage_gain - 1.0).abs() < 1e-12);
            }
        }
    }

    #[test]
    fn evaluates_below_resonance_gain_reference_point() {
        let result = evaluate_cllc_fha_gain(input(0.3, 0.8)).unwrap();
        assert!((result.normalized_voltage_gain - 1.1419409).abs() < 1e-6);
    }

    #[test]
    fn evaluates_above_resonance_gain_reference_point() {
        let result = evaluate_cllc_fha_gain(input(0.3, 1.2)).unwrap();
        assert!((result.normalized_voltage_gain - 0.9000652).abs() < 1e-6);
    }

    #[test]
    fn preserves_direction_specific_quality_factor() {
        let charge = evaluate_cllc_fha_gain(ClLcFhaInput {
            direction: ClLcDirection::Charging,
            ..input(0.3, 0.8)
        })
        .unwrap();
        let discharge = evaluate_cllc_fha_gain(ClLcFhaInput {
            direction: ClLcDirection::Discharging,
            ..input(0.6, 0.8)
        })
        .unwrap();

        assert_eq!(charge.direction, ClLcDirection::Charging);
        assert_eq!(discharge.direction, ClLcDirection::Discharging);
        assert_ne!(charge.quality_factor, discharge.quality_factor);
        assert_ne!(charge.normalized_voltage_gain, discharge.normalized_voltage_gain);
    }

    #[test]
    fn scans_a_sampled_gain_branch_and_classifies_trend() {
        let result = super::scan_cllc_fha_gain_curve(
            3.51,
            0.3,
            ClLcDirection::Charging,
            &[1.0, 1.1, 1.2, 1.4, 1.6],
        )
        .unwrap();
        assert_eq!(result.points.len(), 5);
        assert_eq!(result.sampled_trend, super::ClLcSampledTrend::Decreasing);
        assert_eq!(
            result.claim_ceiling(),
            "DISCRETE_CLLC_FHA_GAIN_SCAN_ONLY_NOT_CONTINUOUS_MONOTONICITY_OR_ZVS_PROOF"
        );
    }

    #[test]
    fn rejects_unsorted_or_duplicate_gain_scan_frequencies() {
        assert!(super::scan_cllc_fha_gain_curve(
            3.51,
            0.3,
            ClLcDirection::Charging,
            &[0.8, 1.0, 0.9],
        )
        .is_err());
        assert!(super::scan_cllc_fha_gain_curve(
            3.51,
            0.3,
            ClLcDirection::Charging,
            &[0.8, 0.8],
        )
        .is_err());
    }

    #[test]
    fn rejects_short_gain_scan() {
        assert!(super::scan_cllc_fha_gain_curve(
            3.51,
            0.3,
            ClLcDirection::Charging,
            &[1.0],
        )
        .is_err());
    }

    #[test]
    fn rejects_invalid_parameters() {
        let mut p = input(0.3, 0.8);
        p.magnetizing_to_resonant_inductance_ratio = 0.0;
        assert!(evaluate_cllc_fha_gain(p).is_err());

        let mut p = input(0.0, 0.8);
        assert!(evaluate_cllc_fha_gain(p).is_err());

        let mut p = input(0.3, 0.0);
        assert!(evaluate_cllc_fha_gain(p).is_err());

        let mut p = input(0.3, f64::NAN);
        assert!(evaluate_cllc_fha_gain(p).is_err());
    }

    #[test]
    fn rejects_numerically_extreme_frequency() {
        let p = input(0.3, f64::MAX);
        assert!(evaluate_cllc_fha_gain(p).is_err());
    }
}
