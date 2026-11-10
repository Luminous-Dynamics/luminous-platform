use crate::AuditError;

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
