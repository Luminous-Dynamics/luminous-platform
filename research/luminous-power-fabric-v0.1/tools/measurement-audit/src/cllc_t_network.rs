use crate::AuditError;

/// Frequency-window inputs for the selected low-frequency boost mode in
/// Huang et al.'s T-network CLLC design method (their Eq. 10).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClLcLowFrequencyBoostWindowInput {
    pub primary_series_resonant_frequency_hz: f64,
    pub secondary_series_resonant_frequency_hz: f64,
    /// k = Lm/Lr1 = Lm/(n²*Lr2), under the paper's symmetric ratio assumption.
    pub magnetizing_to_resonant_inductance_ratio: f64,
    /// The controller's declared operating-frequency limits.
    pub controller_minimum_frequency_hz: f64,
    pub controller_maximum_frequency_hz: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClLcLowFrequencyBoostWindowResult {
    pub topology_lower_bound_hz: f64,
    pub topology_upper_bound_hz: f64,
    pub effective_lower_bound_hz: f64,
    pub effective_upper_bound_hz: f64,
    /// True only when the intersection is a non-empty open interval.
    pub feasible: bool,
}

impl ClLcLowFrequencyBoostWindowResult {
    pub const fn claim_ceiling(&self) -> &'static str {
        "PAPER_SPECIFIC_CLLC_LOW_FREQUENCY_BOOST_WINDOW_ONLY_NOT_ZVS_OR_FULL_DESIGN_QUALIFICATION"
    }
}

/// Screen the paper's selected low-frequency boost-mode switching window:
///
/// max(fr1/sqrt(1+k), fr2/sqrt(1+2k)) < fs < min(fr1, fr2)
///
/// The result is intersected with the caller's controller frequency window.
/// This is one paper-specific topology-state constraint. It does not evaluate
/// complex bridge modulation impedance, Y-parameter state signs, constant-power
/// ZVS boundaries, unity-gain ZVS, maximum gain, device Coss/dead time, currents,
/// losses, or reverse-mode feasibility.
pub fn screen_cllc_low_frequency_boost_window(
    input: ClLcLowFrequencyBoostWindowInput,
) -> Result<ClLcLowFrequencyBoostWindowResult, AuditError> {
    let values = [
        input.primary_series_resonant_frequency_hz,
        input.secondary_series_resonant_frequency_hz,
        input.magnetizing_to_resonant_inductance_ratio,
        input.controller_minimum_frequency_hz,
        input.controller_maximum_frequency_hz,
    ];
    if values.iter().any(|value| !value.is_finite() || *value <= 0.0) {
        return Err(AuditError(
            "CLLC boost-window frequencies and inductance ratio must be positive and finite"
                .to_string(),
        ));
    }
    if input.controller_minimum_frequency_hz >= input.controller_maximum_frequency_hz {
        return Err(AuditError(
            "controller minimum frequency must be below its maximum frequency".to_string(),
        ));
    }

    let fr1 = input.primary_series_resonant_frequency_hz;
    let fr2 = input.secondary_series_resonant_frequency_hz;
    let k = input.magnetizing_to_resonant_inductance_ratio;

    let lower_primary = fr1 / (1.0 + k).sqrt();
    let lower_secondary = fr2 / (1.0 + 2.0 * k).sqrt();
    let topology_lower_bound_hz = lower_primary.max(lower_secondary);
    let topology_upper_bound_hz = fr1.min(fr2);

    let effective_lower_bound_hz =
        topology_lower_bound_hz.max(input.controller_minimum_frequency_hz);
    let effective_upper_bound_hz =
        topology_upper_bound_hz.min(input.controller_maximum_frequency_hz);

    if [
        lower_primary,
        lower_secondary,
        topology_lower_bound_hz,
        topology_upper_bound_hz,
        effective_lower_bound_hz,
        effective_upper_bound_hz,
    ]
    .iter()
    .any(|value| !value.is_finite())
    {
        return Err(AuditError(
            "CLLC boost-window calculation produced a non-finite bound".to_string(),
        ));
    }

    Ok(ClLcLowFrequencyBoostWindowResult {
        topology_lower_bound_hz,
        topology_upper_bound_hz,
        effective_lower_bound_hz,
        effective_upper_bound_hz,
        feasible: effective_lower_bound_hz < effective_upper_bound_hz,
    })
}

/// Minimal complex number type for the small-signal FHA impedance network.
/// Re = real part, Im = imaginary part, with the documented exp(+j omega t)
/// phasor convention for this generic evaluator.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ComplexValue {
    pub re: f64,
    pub im: f64,
}

impl ComplexValue {
    pub const fn new(re: f64, im: f64) -> Self {
        Self { re, im }
    }

    pub fn magnitude(self) -> f64 {
        self.re.hypot(self.im)
    }

    pub fn phase_rad(self) -> f64 {
        self.im.atan2(self.re)
    }

    fn add(self, other: Self) -> Self {
        Self::new(self.re + other.re, self.im + other.im)
    }

    fn sub(self, other: Self) -> Self {
        Self::new(self.re - other.re, self.im - other.im)
    }

    fn mul(self, other: Self) -> Self {
        Self::new(
            self.re * other.re - self.im * other.im,
            self.re * other.im + self.im * other.re,
        )
    }

    fn div(self, other: Self) -> Result<Self, AuditError> {
        let denominator = other.re * other.re + other.im * other.im;
        if !denominator.is_finite() || denominator <= 0.0 {
            return Err(AuditError(
                "complex division has a zero or non-finite denominator".to_string(),
            ));
        }
        let value = Self::new(
            (self.re * other.re + self.im * other.im) / denominator,
            (self.im * other.re - self.re * other.im) / denominator,
        );
        if !value.re.is_finite() || !value.im.is_finite() {
            return Err(AuditError(
                "complex division produced a non-finite result".to_string(),
            ));
        }
        Ok(value)
    }

    fn reciprocal(self) -> Result<Self, AuditError> {
        Self::new(1.0, 0.0).div(self)
    }

    fn is_finite(self) -> bool {
        self.re.is_finite() && self.im.is_finite()
    }
}

/// Generic T-network input using already-signed branch reactances.
///
/// Branch impedances are Z1=jX1, Z2=jX2 and Z3=jX3. X1, X2 and X3 are
/// supplied explicitly so the caller must apply the exact circuit orientation
/// and transformer-referral convention from the topology being analysed.
/// The secondary active bridge/load is represented as Zac = Req + jXeq.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClLcTNetworkInput {
    pub x1_ohm: f64,
    pub x2_ohm: f64,
    pub x3_ohm: f64,
    pub secondary_equivalent_resistance_ohm: f64,
    pub secondary_equivalent_reactance_ohm: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClLcTNetworkResult {
    pub determinant_reactance_squared_ohm2: f64,
    /// Y-parameter values in siemens for the paper's two-port current convention.
    pub y1_s: ComplexValue,
    pub y2_s: ComplexValue,
    pub y3_s: ComplexValue,
    pub secondary_equivalent_admittance_s: ComplexValue,
    /// U2/U1 from the network relation I2 = -Y2*U1 + Y3*U2 = Yeq*U2.
    pub secondary_voltage_to_primary_voltage: ComplexValue,
    pub primary_input_admittance_s: ComplexValue,
    pub primary_input_impedance_ohm: ComplexValue,
}

impl ClLcTNetworkResult {
    pub const fn claim_ceiling(&self) -> &'static str {
        "GENERIC_FHA_T_NETWORK_PHASOR_EVALUATION_ONLY_NOT_CLCC_STATE_OR_ZVS_QUALIFICATION"
    }
}

/// Derive the secondary equivalent AC impedance for the specific VF-TPS
/// modulation representation in Huang et al. (IEICE Electronics Express 2026),
/// Eq. (1):
///
/// Zac,eq = [8*Rload*cos²(theta2)*cos²(phi2)/(n²*pi²)] * exp(j*phi2)
///
/// theta2 is the secondary-bridge phase-shift angle, phi2 is the phase
/// difference between the secondary AC-port voltage and current fundamentals,
/// and n is the transformer's primary-to-secondary turns ratio as defined by
/// that paper. This formula is modulation- and convention-specific; do not
/// reuse it for another bridge modulation without re-deriving the equivalent.
pub fn secondary_vftps_equivalent_impedance(
    secondary_dc_load_ohm: f64,
    turns_ratio_primary_to_secondary: f64,
    secondary_bridge_phase_shift_rad: f64,
    secondary_ac_voltage_current_phase_rad: f64,
) -> Result<ComplexValue, AuditError> {
    let values = [
        secondary_dc_load_ohm,
        turns_ratio_primary_to_secondary,
        secondary_bridge_phase_shift_rad,
        secondary_ac_voltage_current_phase_rad,
    ];
    if values.iter().any(|value| !value.is_finite()) {
        return Err(AuditError(
            "VF-TPS equivalent-impedance inputs must be finite".to_string(),
        ));
    }
    if secondary_dc_load_ohm <= 0.0 || turns_ratio_primary_to_secondary <= 0.0 {
        return Err(AuditError(
            "VF-TPS load resistance and turns ratio must be positive".to_string(),
        ));
    }

    let cos_theta = secondary_bridge_phase_shift_rad.cos();
    let cos_phi = secondary_ac_voltage_current_phase_rad.cos();
    let magnitude = 8.0
        * secondary_dc_load_ohm
        * cos_theta.powi(2)
        * cos_phi.powi(2)
        / (turns_ratio_primary_to_secondary.powi(2) * std::f64::consts::PI.powi(2));

    if !magnitude.is_finite() || magnitude <= 0.0 {
        return Err(AuditError(
            "VF-TPS equivalent impedance magnitude must be positive and finite".to_string(),
        ));
    }

    let result = ComplexValue::new(
        magnitude * secondary_ac_voltage_current_phase_rad.cos(),
        magnitude * secondary_ac_voltage_current_phase_rad.sin(),
    );
    if !result.is_finite() {
        return Err(AuditError(
            "VF-TPS equivalent impedance is not finite".to_string(),
        ));
    }
    Ok(result)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClLcVfTpsTNetworkInput {
    pub x1_ohm: f64,
    pub x2_ohm: f64,
    pub x3_ohm: f64,
    pub secondary_dc_load_ohm: f64,
    pub turns_ratio_primary_to_secondary: f64,
    pub secondary_bridge_phase_shift_rad: f64,
    pub secondary_ac_voltage_current_phase_rad: f64,
}

/// Convenience composition of the paper-specific VF-TPS secondary impedance
/// calculation with the generic T-network phasor evaluator.
pub fn evaluate_cllc_vftps_t_network(
    input: ClLcVfTpsTNetworkInput,
) -> Result<ClLcTNetworkResult, AuditError> {
    let zac = secondary_vftps_equivalent_impedance(
        input.secondary_dc_load_ohm,
        input.turns_ratio_primary_to_secondary,
        input.secondary_bridge_phase_shift_rad,
        input.secondary_ac_voltage_current_phase_rad,
    )?;
    evaluate_cllc_t_network(ClLcTNetworkInput {
        x1_ohm: input.x1_ohm,
        x2_ohm: input.x2_ohm,
        x3_ohm: input.x3_ohm,
        secondary_equivalent_resistance_ohm: zac.re,
        secondary_equivalent_reactance_ohm: zac.im,
    })
}

/// Evaluate the generic T-type impedance network used in the 2026 CLLC design paper.
///
/// With Z1=jX1, Z2=jX2 and Z3=jX3:
/// D = X1*X2 + X1*X3 + X2*X3
/// Y1 = -j*(X2+X3)/D
/// Y2 =  j*X3/D
/// Y3 = -j*(X1+X3)/D
///
/// The paper's port relation is:
/// I1 = Y1*U1 + Y2*U2
/// I2 = -Y2*U1 + Y3*U2 = Yeq*U2
///
/// Therefore:
/// U2/U1 = Y2/(Y3-Yeq)
/// Yin,1 = Y1 + Y2^2/(Y3-Yeq)
/// Zin,1 = 1/Yin,1
///
/// This helper only evaluates a declared small-signal FHA phasor network.
/// It does not choose modulation angles, infer X1/X2/X3 from a physical converter,
/// classify a target topology state, calculate current stress, model harmonics,
/// include semiconductor Coss/dead time, prove ZVS/ZCS, calculate losses or
/// authorize hardware operation. Sign and referral conventions must be reviewed
/// before using the result for a specific CLLC schematic.
pub fn evaluate_cllc_t_network(
    input: ClLcTNetworkInput,
) -> Result<ClLcTNetworkResult, AuditError> {
    let values = [
        input.x1_ohm,
        input.x2_ohm,
        input.x3_ohm,
        input.secondary_equivalent_resistance_ohm,
        input.secondary_equivalent_reactance_ohm,
    ];
    if values.iter().any(|value| !value.is_finite()) {
        return Err(AuditError(
            "T-network reactances and secondary equivalent impedance must be finite".to_string(),
        ));
    }
    if input.secondary_equivalent_resistance_ohm <= 0.0 {
        return Err(AuditError(
            "secondary equivalent resistance must be positive".to_string(),
        ));
    }

    let x1 = input.x1_ohm;
    let x2 = input.x2_ohm;
    let x3 = input.x3_ohm;
    let determinant = x1 * x2 + x1 * x3 + x2 * x3;

    if !determinant.is_finite() || determinant == 0.0 {
        return Err(AuditError(
            "T-network determinant is zero or non-finite; network is singular".to_string(),
        ));
    }

    let y1 = ComplexValue::new(0.0, -(x2 + x3) / determinant);
    let y2 = ComplexValue::new(0.0, x3 / determinant);
    let y3 = ComplexValue::new(0.0, -(x1 + x3) / determinant);
    let zac = ComplexValue::new(
        input.secondary_equivalent_resistance_ohm,
        input.secondary_equivalent_reactance_ohm,
    );
    let y_eq = zac.reciprocal()?;

    let secondary_voltage_ratio = y2.div(y3.sub(y_eq))?;
    let primary_input_admittance = y1.add(y2.mul(y2).div(y3.sub(y_eq))?);
    if primary_input_admittance.magnitude() == 0.0 {
        return Err(AuditError(
            "primary input admittance is zero; input impedance is unbounded".to_string(),
        ));
    }
    let primary_input_impedance = primary_input_admittance.reciprocal()?;

    let outputs = [
        y1,
        y2,
        y3,
        y_eq,
        secondary_voltage_ratio,
        primary_input_admittance,
        primary_input_impedance,
    ];
    if outputs.iter().any(|value| !value.is_finite()) {
        return Err(AuditError(
            "T-network evaluation produced a non-finite phasor".to_string(),
        ));
    }

    Ok(ClLcTNetworkResult {
        determinant_reactance_squared_ohm2: determinant,
        y1_s: y1,
        y2_s: y2,
        y3_s: y3,
        secondary_equivalent_admittance_s: y_eq,
        secondary_voltage_to_primary_voltage: secondary_voltage_ratio,
        primary_input_admittance_s: primary_input_admittance,
        primary_input_impedance_ohm: primary_input_impedance,
    })
}

#[cfg(test)]
mod tests {
    use super::{evaluate_cllc_t_network, ClLcTNetworkInput};

    fn example() -> ClLcTNetworkInput {
        ClLcTNetworkInput {
            x1_ohm: -2.0,
            x2_ohm: -3.0,
            x3_ohm: 10.0,
            secondary_equivalent_resistance_ohm: 10.0,
            secondary_equivalent_reactance_ohm: -1.0,
        }
    }

    #[test]
    fn computes_paper_low_frequency_boost_window() {
        let result = super::screen_cllc_low_frequency_boost_window(
            super::ClLcLowFrequencyBoostWindowInput {
                primary_series_resonant_frequency_hz: 115_000.0,
                secondary_series_resonant_frequency_hz: 100_000.0,
                magnetizing_to_resonant_inductance_ratio: 3.0,
                controller_minimum_frequency_hz: 50_000.0,
                controller_maximum_frequency_hz: 125_000.0,
            },
        )
        .unwrap();

        assert!((result.topology_lower_bound_hz - 57_500.0).abs() < 1e-9);
        assert!((result.topology_upper_bound_hz - 100_000.0).abs() < 1e-9);
        assert!((result.effective_lower_bound_hz - 57_500.0).abs() < 1e-9);
        assert!(result.feasible);
    }

    #[test]
    fn intersects_topology_window_with_controller_limits() {
        let result = super::screen_cllc_low_frequency_boost_window(
            super::ClLcLowFrequencyBoostWindowInput {
                primary_series_resonant_frequency_hz: 115_000.0,
                secondary_series_resonant_frequency_hz: 100_000.0,
                magnetizing_to_resonant_inductance_ratio: 3.0,
                controller_minimum_frequency_hz: 70_000.0,
                controller_maximum_frequency_hz: 90_000.0,
            },
        )
        .unwrap();

        assert!((result.effective_lower_bound_hz - 70_000.0).abs() < 1e-9);
        assert!((result.effective_upper_bound_hz - 90_000.0).abs() < 1e-9);
        assert!(result.feasible);
    }

    #[test]
    fn rejects_empty_frequency_window() {
        let result = super::screen_cllc_low_frequency_boost_window(
            super::ClLcLowFrequencyBoostWindowInput {
                primary_series_resonant_frequency_hz: 115_000.0,
                secondary_series_resonant_frequency_hz: 100_000.0,
                magnetizing_to_resonant_inductance_ratio: 3.0,
                controller_minimum_frequency_hz: 105_000.0,
                controller_maximum_frequency_hz: 120_000.0,
            },
        )
        .unwrap();

        assert!(!result.feasible);
        assert!(result.effective_lower_bound_hz >= result.effective_upper_bound_hz);
    }

    #[test]
    fn rejects_invalid_boost_window_inputs() {
        let mut input = super::ClLcLowFrequencyBoostWindowInput {
            primary_series_resonant_frequency_hz: 115_000.0,
            secondary_series_resonant_frequency_hz: 100_000.0,
            magnetizing_to_resonant_inductance_ratio: 3.0,
            controller_minimum_frequency_hz: 70_000.0,
            controller_maximum_frequency_hz: 90_000.0,
        };
        input.magnetizing_to_resonant_inductance_ratio = 0.0;
        assert!(super::screen_cllc_low_frequency_boost_window(input).is_err());

        input.magnetizing_to_resonant_inductance_ratio = 3.0;
        input.controller_minimum_frequency_hz = 100_000.0;
        assert!(super::screen_cllc_low_frequency_boost_window(input).is_err());
    }

    #[test]
    fn computes_secondary_vftps_equivalent_impedance() {
        let z = super::secondary_vftps_equivalent_impedance(
            10.0,
            1.8,
            0.2,
            0.1,
        )
        .unwrap();
        assert!((z.re - 2.3671789275).abs() < 1e-9);
        assert!((z.im - 0.2375101215).abs() < 1e-9);
    }

    #[test]
    fn zero_phase_difference_produces_real_secondary_impedance() {
        let z = super::secondary_vftps_equivalent_impedance(10.0, 1.8, 0.0, 0.0).unwrap();
        assert!(z.re > 0.0);
        assert!(z.im.abs() < 1e-12);
    }

    #[test]
    fn rejects_zero_modulation_equivalent_impedance() {
        assert!(super::secondary_vftps_equivalent_impedance(
            10.0,
            1.8,
            std::f64::consts::PI / 2.0,
            0.1,
        )
        .is_err());
    }

    #[test]
    fn composes_vftps_impedance_and_t_network_evaluation() {
        let input = super::ClLcVfTpsTNetworkInput {
            x1_ohm: -2.0,
            x2_ohm: -3.0,
            x3_ohm: 10.0,
            secondary_dc_load_ohm: 10.0,
            turns_ratio_primary_to_secondary: 1.8,
            secondary_bridge_phase_shift_rad: 0.2,
            secondary_ac_voltage_current_phase_rad: 0.1,
        };
        let result = super::evaluate_cllc_vftps_t_network(input).unwrap();
        assert!(result.primary_input_impedance_ohm.re.is_finite());
        assert!(result.primary_input_impedance_ohm.im.is_finite());
    }

    #[test]
    fn computes_t_network_y_parameters() {
        let result = evaluate_cllc_t_network(example()).unwrap();
        assert!((result.determinant_reactance_squared_ohm2 + 44.0).abs() < 1e-12);
        assert!((result.y1_s.im - 7.0 / 44.0).abs() < 1e-12);
        assert!((result.y2_s.im + 10.0 / 44.0).abs() < 1e-12);
        assert!((result.y3_s.im - 8.0 / 44.0).abs() < 1e-12);
    }

    #[test]
    fn computes_secondary_voltage_and_primary_input_phasors() {
        let result = evaluate_cllc_t_network(example()).unwrap();
        assert!((result.secondary_voltage_to_primary_voltage.re + 0.9927234927).abs() < 1e-9);
        assert!((result.secondary_voltage_to_primary_voltage.im - 0.5717255717).abs() < 1e-9);
        assert!((result.primary_input_admittance_s.re - 0.1299376299).abs() < 1e-9);
        assert!((result.primary_input_admittance_s.im - 0.3847098847).abs() < 1e-9);
        assert!((result.primary_input_impedance_ohm.re - 0.7880477257).abs() < 1e-9);
        assert!((result.primary_input_impedance_ohm.im + 2.3331943938).abs() < 1e-9);
    }

    #[test]
    fn handles_positive_and_negative_secondary_reactance() {
        let capacitive = evaluate_cllc_t_network(example()).unwrap();
        let mut inductive_input = example();
        inductive_input.secondary_equivalent_reactance_ohm = 1.0;
        let inductive = evaluate_cllc_t_network(inductive_input).unwrap();

        assert_ne!(
            capacitive.secondary_equivalent_admittance_s,
            inductive.secondary_equivalent_admittance_s
        );
        assert_ne!(
            capacitive.primary_input_impedance_ohm,
            inductive.primary_input_impedance_ohm
        );
    }

    #[test]
    fn rejects_singular_t_network_determinant() {
        let mut input = example();
        input.x1_ohm = 1.0;
        input.x2_ohm = 1.0;
        input.x3_ohm = -0.5;
        assert!(evaluate_cllc_t_network(input).is_err());
    }

    #[test]
    fn rejects_non_positive_secondary_resistance() {
        let mut input = example();
        input.secondary_equivalent_resistance_ohm = 0.0;
        assert!(evaluate_cllc_t_network(input).is_err());
    }

    #[test]
    fn rejects_non_finite_inputs() {
        let mut input = example();
        input.x2_ohm = f64::NAN;
        assert!(evaluate_cllc_t_network(input).is_err());
    }
}
