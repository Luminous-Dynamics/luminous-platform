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
        assert!((result.normalized_voltage_gain - 0.9000698).abs() < 1e-6);
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
