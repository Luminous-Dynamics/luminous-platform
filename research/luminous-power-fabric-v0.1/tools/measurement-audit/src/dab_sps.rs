use crate::AuditError;

/// Ideal single-phase-shift (SPS) DAB operating point.
///
/// The secondary DC voltage is already referred to the primary side and the
/// leakage/series inductance is also referred to the primary side. This avoids
/// ambiguity about transformer turns-ratio convention in the screening API.
///
/// Positive requested power means Port A -> Port B. Negative means Port B ->
/// Port A. The returned phase-shift sign follows that convention.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DabSpsOperatingPoint {
    pub port_a_voltage_v: f64,
    pub port_b_voltage_referred_to_a_v: f64,
    pub leakage_inductance_referred_to_a_h: f64,
    pub switching_frequency_hz: f64,
    pub requested_signed_power_a_to_b_w: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DabSpsScreeningResult {
    pub max_ideal_power_w: f64,
    pub signed_phase_shift_rad: f64,
    pub signed_phase_shift_degrees: f64,
    pub ideal_signed_transferred_power_w: f64,
    pub ideal_transfer_utilization_percent: f64,
}

impl DabSpsScreeningResult {
    pub const fn claim_ceiling(&self) -> &'static str {
        "IDEAL_DAB_SPS_POWER_TRANSFER_SCREEN_ONLY_NOT_LOSS_OR_ZVS_QUALIFICATION"
    }
}

/// Invert the ideal SPS DAB power equation for a requested signed transfer.
///
/// Using the referred secondary voltage and primary-referred leakage inductance:
///
/// P = Va * Vb_ref * phi * (pi - |phi|) / (2 * pi^2 * fs * L)
///
/// |P|max = Va * Vb_ref / (8 * fs * L), at |phi| = pi/2.
///
/// This is an ideal lossless steady-state transfer relation. It does not derive
/// device RMS/peak current, circulating current, ZVS/ZCS conditions, dead-time,
/// magnetizing current, semiconductor switching/conduction loss, magnetic loss,
/// thermal limits, EPS/TPS operation, startup or transient response.
pub fn screen_dab_sps(point: DabSpsOperatingPoint) -> Result<DabSpsScreeningResult, AuditError> {
    let positive_inputs = [
        point.port_a_voltage_v,
        point.port_b_voltage_referred_to_a_v,
        point.leakage_inductance_referred_to_a_h,
        point.switching_frequency_hz,
    ];
    if positive_inputs
        .iter()
        .any(|value| !value.is_finite() || *value <= 0.0)
    {
        return Err(AuditError(
            "DAB voltage, referred inductance and switching frequency must be positive and finite"
                .to_string(),
        ));
    }
    if !point.requested_signed_power_a_to_b_w.is_finite() {
        return Err(AuditError(
            "requested signed power must be finite".to_string(),
        ));
    }

    let max_ideal_power_w = point.port_a_voltage_v
        * point.port_b_voltage_referred_to_a_v
        / (8.0 * point.switching_frequency_hz * point.leakage_inductance_referred_to_a_h);

    if !max_ideal_power_w.is_finite() || max_ideal_power_w <= 0.0 {
        return Err(AuditError(
            "derived ideal maximum power is not positive and finite".to_string(),
        ));
    }

    let requested_magnitude_w = point.requested_signed_power_a_to_b_w.abs();
    if requested_magnitude_w > max_ideal_power_w {
        return Err(AuditError(format!(
            "requested power {:.6} W exceeds ideal SPS maximum {:.6} W; this operating point is infeasible under this ideal model",
            requested_magnitude_w, max_ideal_power_w
        )));
    }

    let utilization = requested_magnitude_w / max_ideal_power_w;
    let phase_magnitude_rad =
        (std::f64::consts::PI / 2.0) * (1.0 - (1.0 - utilization).sqrt());
    let sign = point.requested_signed_power_a_to_b_w.signum();
    let signed_phase_shift_rad = sign * phase_magnitude_rad;

    let ideal_signed_transferred_power_w = sign
        * max_ideal_power_w
        * (4.0 / std::f64::consts::PI.powi(2))
        * phase_magnitude_rad
        * (std::f64::consts::PI - phase_magnitude_rad);

    let result = DabSpsScreeningResult {
        max_ideal_power_w,
        signed_phase_shift_rad,
        signed_phase_shift_degrees: signed_phase_shift_rad.to_degrees(),
        ideal_signed_transferred_power_w,
        ideal_transfer_utilization_percent: 100.0 * utilization,
    };

    if !result.signed_phase_shift_rad.is_finite()
        || !result.signed_phase_shift_degrees.is_finite()
        || !result.ideal_signed_transferred_power_w.is_finite()
    {
        return Err(AuditError(
            "DAB SPS screening result is not finite".to_string(),
        ));
    }

    Ok(result)
}

/// Calculate the ideal primary-referred leakage inductance that would transfer
/// the requested power at a selected SPS phase-shift magnitude.
///
/// This is a design-space equation only. The resulting inductance must still be
/// evaluated for current stress, ZVS, voltage-ratio range, tolerance, losses and
/// thermal limits in the non-ideal model.
pub fn leakage_inductance_for_power_and_phase(
    port_a_voltage_v: f64,
    port_b_voltage_referred_to_a_v: f64,
    switching_frequency_hz: f64,
    requested_power_w: f64,
    target_phase_shift_rad: f64,
) -> Result<f64, AuditError> {
    let positive_inputs = [
        port_a_voltage_v,
        port_b_voltage_referred_to_a_v,
        switching_frequency_hz,
        requested_power_w,
        target_phase_shift_rad,
    ];
    if positive_inputs
        .iter()
        .any(|value| !value.is_finite() || *value <= 0.0)
    {
        return Err(AuditError(
            "DAB design inputs and target phase shift must be positive and finite".to_string(),
        ));
    }
    if target_phase_shift_rad > std::f64::consts::PI / 2.0 {
        return Err(AuditError(
            "ideal SPS target phase magnitude must not exceed pi/2".to_string(),
        ));
    }

    let inductance_h = port_a_voltage_v
        * port_b_voltage_referred_to_a_v
        * target_phase_shift_rad
        * (std::f64::consts::PI - target_phase_shift_rad)
        / (2.0
            * std::f64::consts::PI.powi(2)
            * switching_frequency_hz
            * requested_power_w);

    if !inductance_h.is_finite() || inductance_h <= 0.0 {
        return Err(AuditError(
            "derived DAB inductance is not positive and finite".to_string(),
        ));
    }

    Ok(inductance_h)
}

#[cfg(test)]
mod tests {
    use super::{screen_dab_sps, DabSpsOperatingPoint};

    fn point(requested_power_w: f64) -> DabSpsOperatingPoint {
        DabSpsOperatingPoint {
            port_a_voltage_v: 48.0,
            port_b_voltage_referred_to_a_v: 48.0,
            leakage_inductance_referred_to_a_h: 3.84e-6,
            switching_frequency_hz: 100_000.0,
            requested_signed_power_a_to_b_w: requested_power_w,
        }
    }

    #[test]
    fn derives_maximum_ideal_power() {
        let result = screen_dab_sps(point(562.5)).unwrap();
        assert!((result.max_ideal_power_w - 750.0).abs() < 1e-9);
    }

    #[test]
    fn inverts_power_relation_at_three_quarters_of_maximum() {
        let result = screen_dab_sps(point(562.5)).unwrap();
        assert!((result.signed_phase_shift_rad - std::f64::consts::PI / 4.0).abs() < 1e-9);
        assert!((result.ideal_signed_transferred_power_w - 562.5).abs() < 1e-9);
        assert!((result.ideal_transfer_utilization_percent - 75.0).abs() < 1e-9);
    }

    #[test]
    fn maximum_power_occurs_at_ninety_degree_phase_shift() {
        let result = screen_dab_sps(point(750.0)).unwrap();
        assert!((result.signed_phase_shift_rad - std::f64::consts::PI / 2.0).abs() < 1e-9);
        assert!((result.ideal_signed_transferred_power_w - 750.0).abs() < 1e-9);
    }

    #[test]
    fn negative_power_reverses_phase_shift_sign() {
        let result = screen_dab_sps(point(-562.5)).unwrap();
        assert!((result.signed_phase_shift_rad + std::f64::consts::PI / 4.0).abs() < 1e-9);
        assert!((result.ideal_signed_transferred_power_w + 562.5).abs() < 1e-9);
    }

    #[test]
    fn calculates_inductance_for_target_power_and_phase() {
        let inductance = super::leakage_inductance_for_power_and_phase(
            48.0,
            48.0,
            100_000.0,
            500.0,
            std::f64::consts::PI / 4.0,
        )
        .unwrap();
        assert!((inductance - 4.32e-6).abs() < 1e-12);

        let mut p = point(500.0);
        p.leakage_inductance_referred_to_a_h = inductance;
        let result = screen_dab_sps(p).unwrap();
        assert!((result.signed_phase_shift_rad - std::f64::consts::PI / 4.0).abs() < 1e-9);
    }

    #[test]
    fn rejects_target_phase_above_sps_maximum() {
        let result = super::leakage_inductance_for_power_and_phase(
            48.0,
            48.0,
            100_000.0,
            500.0,
            std::f64::consts::PI,
        );
        assert!(result.is_err());
    }

    #[test]
    fn rejects_power_above_ideal_sps_limit() {
        assert!(screen_dab_sps(point(750.01)).is_err());
    }

    #[test]
    fn rejects_infeasible_500w_point_for_ten_microhenry_example() {
        let mut p = point(500.0);
        p.leakage_inductance_referred_to_a_h = 10e-6;
        assert!(screen_dab_sps(p).is_err());
    }

    #[test]
    fn rejects_invalid_electrical_inputs() {
        let mut p = point(100.0);
        p.port_a_voltage_v = 0.0;
        assert!(screen_dab_sps(p).is_err());

        let mut p = point(100.0);
        p.switching_frequency_hz = f64::INFINITY;
        assert!(screen_dab_sps(p).is_err());

        let mut p = point(100.0);
        p.requested_signed_power_a_to_b_w = f64::NAN;
        assert!(screen_dab_sps(p).is_err());
    }
}
