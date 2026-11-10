use crate::AuditError;

/// Provenance class for an operating-point efficiency or standby-power value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnergyEvidenceKind {
    PhysicalMeasurement,
    VendorSpecification,
    AnalyticalModel,
    Assumption,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnergyStage {
    pub id: String,
    pub name: String,
    /// Human-readable boundary description; e.g. "facility 800 VDC bus -> 50 V bus".
    pub boundary: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnergyStageOperatingPoint {
    /// End-to-end stage efficiency at this exact load / voltage / cooling point.
    pub efficiency_fraction: f64,
    /// Stage input-side standby and auxiliary draw in watts at this operating point.
    /// The value is added to the power required at the stage input.
    pub standby_input_power_w: f64,
    pub efficiency_evidence: EnergyEvidenceKind,
    pub efficiency_reference: String,
    pub standby_evidence: EnergyEvidenceKind,
    pub standby_reference: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnergyLoadSegment {
    pub id: String,
    pub name: String,
    /// Hours represented by this load point in a 365-day accounting year.
    pub hours_per_year: f64,
    /// Power delivered to the final useful load, in watts.
    pub delivered_load_power_w: f64,
    /// Operating point for every stage, ordered facility/source -> final load.
    pub stage_operating_points: Vec<EnergyStageOperatingPoint>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnergyChainScenario {
    pub id: String,
    pub name: String,
    /// Three-letter ISO 4217 currency code for the tariff, e.g. ZAR or USD.
    pub currency_code: String,
    pub electricity_price_per_kwh: f64,
    pub electricity_price_reference: String,
    pub grid_carbon_kg_per_kwh: f64,
    pub grid_carbon_reference: String,
    pub stages: Vec<EnergyStage>,
    pub segments: Vec<EnergyLoadSegment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnergyEvidenceReference {
    pub kind: EnergyEvidenceKind,
    pub reference: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnergyStageEnergy {
    pub stage_id: String,
    pub stage_name: String,
    pub boundary: String,
    pub input_energy_kwh: f64,
    pub output_energy_kwh: f64,
    /// Includes conversion loss and the stage's input-side standby / auxiliary draw.
    pub total_energy_loss_kwh: f64,
    /// Original evidence references are retained in both segment and annual summaries.
    pub efficiency_evidence: Vec<EnergyEvidenceReference>,
    pub standby_evidence: Vec<EnergyEvidenceReference>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnergySegmentReport {
    pub segment_id: String,
    pub segment_name: String,
    pub hours_per_year: f64,
    pub delivered_load_power_w: f64,
    pub facility_input_power_w: f64,
    pub delivered_energy_kwh: f64,
    pub facility_input_energy_kwh: f64,
    pub chain_loss_energy_kwh: f64,
    pub end_to_end_efficiency_percent: Option<f64>,
    pub stages: Vec<EnergyStageEnergy>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnergyChainReport {
    pub scenario_id: String,
    pub scenario_name: String,
    pub total_hours_per_year: f64,
    pub delivered_energy_kwh: f64,
    pub facility_input_energy_kwh: f64,
    pub total_chain_loss_energy_kwh: f64,
    pub annual_end_to_end_efficiency_percent: f64,
    pub currency_code: String,
    pub electricity_price_per_kwh: f64,
    pub electricity_price_reference: String,
    pub grid_carbon_kg_per_kwh: f64,
    pub grid_carbon_reference: String,
    pub annual_electricity_cost: f64,
    pub annual_grid_emissions_kg: f64,
    pub stages: Vec<EnergyStageEnergy>,
    pub segments: Vec<EnergySegmentReport>,
}

impl EnergyChainReport {
    pub const fn claim_ceiling(&self) -> &'static str {
        "PARAMETERIZED_CONVERSION_CHAIN_ENERGY_ACCOUNTING_ONLY_NOT_SITE_MEASUREMENT_OR_EFFICIENCY_CLAIM"
    }
}

fn require_nonblank(value: &str, field: &str) -> Result<(), AuditError> {
    if value.trim().is_empty() {
        return Err(AuditError(format!("{field} must not be blank")));
    }
    Ok(())
}

fn validate_stage_point(
    point: &EnergyStageOperatingPoint,
    stage_name: &str,
    segment_name: &str,
) -> Result<(), AuditError> {
    if !point.efficiency_fraction.is_finite()
        || point.efficiency_fraction <= 0.0
        || point.efficiency_fraction >= 1.0
    {
        return Err(AuditError(format!(
            "segment {segment_name:?}, stage {stage_name:?}: efficiency must be finite and strictly between 0 and 1; idealized zero-loss stages are not accepted"
        )));
    }
    if !point.standby_input_power_w.is_finite() || point.standby_input_power_w < 0.0 {
        return Err(AuditError(format!(
            "segment {segment_name:?}, stage {stage_name:?}: standby input power must be finite and non-negative"
        )));
    }
    require_nonblank(&point.efficiency_reference, "efficiency_reference")?;
    require_nonblank(&point.standby_reference, "standby_reference")?;
    Ok(())
}

/// Evaluate a serial chain of conversion stages over a piecewise-constant annual load profile.
///
/// Stages are ordered from the facility/source to the final load. For each stage:
///
/// P_in = P_out / eta + P_standby,input
///
/// The input power required by a downstream stage becomes the output load of the
/// preceding stage. The model therefore accounts for downstream conversion losses
/// and standby consumption propagating upstream.
///
/// Each load segment has its own declared efficiency and standby values for every
/// stage. No interpolation between operating points is attempted. The supplied
/// efficiencies must already include the stated stage boundary; do not add
/// separately measured parasitics again if they are already within that boundary.
///
/// The load segments must cover exactly 8,760 hours so standby / idle intervals cannot
/// silently disappear from an annual cost calculation. A zero-load segment is allowed;
/// give it explicit stage standby inputs if the equipment remains energized. A fully
/// de-energized interval has zero input/output and an undefined segment efficiency.
///
/// This tool does not model voltage regulation, harmonics, power factor, cable
/// impedance, busbar I²R loss, dynamic load transients, storage dispatch, cooling
/// plant outside the declared stages, availability, degradation or uncertainty
/// propagation. Model those explicitly as additional stages or separate models;
/// do not silently fold them into an invented efficiency.
pub fn evaluate_energy_chain(
    scenario: &EnergyChainScenario,
) -> Result<EnergyChainReport, AuditError> {
    require_nonblank(&scenario.id, "scenario.id")?;
    require_nonblank(&scenario.name, "scenario.name")?;

    require_nonblank(&scenario.currency_code, "scenario.currency_code")?;
    if scenario.currency_code.len() != 3
        || !scenario.currency_code.bytes().all(|b| b.is_ascii_uppercase())
    {
        return Err(AuditError(
            "currency_code must be a three-letter uppercase ISO 4217 code".to_string(),
        ));
    }
    require_nonblank(
        &scenario.electricity_price_reference,
        "scenario.electricity_price_reference",
    )?;
    require_nonblank(
        &scenario.grid_carbon_reference,
        "scenario.grid_carbon_reference",
    )?;
    if !scenario.electricity_price_per_kwh.is_finite()
        || scenario.electricity_price_per_kwh < 0.0
    {
        return Err(AuditError(
            "electricity price must be finite and non-negative".to_string(),
        ));
    }
    if !scenario.grid_carbon_kg_per_kwh.is_finite()
        || scenario.grid_carbon_kg_per_kwh < 0.0
    {
        return Err(AuditError(
            "grid carbon intensity must be finite and non-negative".to_string(),
        ));
    }
    if scenario.stages.is_empty() {
        return Err(AuditError(
            "an energy-chain scenario requires at least one conversion stage".to_string(),
        ));
    }
    if scenario.segments.is_empty() {
        return Err(AuditError(
            "an energy-chain scenario requires at least one load segment".to_string(),
        ));
    }

    for (index, stage) in scenario.stages.iter().enumerate() {
        require_nonblank(&stage.id, &format!("stages[{index}].id"))?;
        require_nonblank(&stage.name, &format!("stages[{index}].name"))?;
        require_nonblank(&stage.boundary, &format!("stages[{index}].boundary"))?;
        if scenario.stages[..index].iter().any(|prior| prior.id == stage.id) {
            return Err(AuditError(format!(
                "duplicate energy stage id {:?}",
                stage.id
            )));
        }
    }

    let mut total_hours = 0.0_f64;
    for (index, segment) in scenario.segments.iter().enumerate() {
        require_nonblank(&segment.id, &format!("segments[{index}].id"))?;
        require_nonblank(&segment.name, &format!("segments[{index}].name"))?;
        if !segment.hours_per_year.is_finite() || segment.hours_per_year <= 0.0 {
            return Err(AuditError(format!(
                "segment {:?}: hours_per_year must be positive and finite",
                segment.name
            )));
        }
        if !segment.delivered_load_power_w.is_finite() || segment.delivered_load_power_w < 0.0 {
            return Err(AuditError(format!(
                "segment {:?}: delivered load power must be non-negative and finite",
                segment.name
            )));
        }
        if segment.stage_operating_points.len() != scenario.stages.len() {
            return Err(AuditError(format!(
                "segment {:?}: expected {} stage operating points, found {}",
                segment.name,
                scenario.stages.len(),
                segment.stage_operating_points.len()
            )));
        }
        if scenario.segments[..index].iter().any(|prior| prior.id == segment.id) {
            return Err(AuditError(format!(
                "duplicate load segment id {:?}",
                segment.id
            )));
        }

        total_hours += segment.hours_per_year;
        if !total_hours.is_finite() || total_hours > 8_760.0 + 1e-9 {
            return Err(AuditError(
                "load-profile hours must not exceed 8,760 hours per year".to_string(),
            ));
        }

        for (stage, point) in scenario.stages.iter().zip(&segment.stage_operating_points) {
            validate_stage_point(point, &stage.name, &segment.name)?;
        }
    }

    if (total_hours - 8_760.0).abs() > 1e-6 {
        return Err(AuditError(format!(
            "load profile must cover exactly 8,760 hours; found {total_hours:.6}. Include explicit idle / standby segments rather than omitting them."
        )));
    }

    let stage_count = scenario.stages.len();
    let mut total_delivered_energy_kwh = 0.0_f64;
    let mut total_input_energy_kwh = 0.0_f64;
    let mut stage_totals: Vec<EnergyStageEnergy> = scenario
        .stages
        .iter()
        .map(|stage| EnergyStageEnergy {
            stage_id: stage.id.clone(),
            stage_name: stage.name.clone(),
            boundary: stage.boundary.clone(),
            input_energy_kwh: 0.0,
            output_energy_kwh: 0.0,
            total_energy_loss_kwh: 0.0,
            efficiency_evidence: Vec::new(),
            standby_evidence: Vec::new(),
        })
        .collect();
    let mut segment_reports = Vec::with_capacity(scenario.segments.len());

    for segment in &scenario.segments {
        let mut downstream_power_w = segment.delivered_load_power_w;
        let mut per_stage_input_power_w = vec![0.0_f64; stage_count];
        let mut per_stage_output_power_w = vec![0.0_f64; stage_count];

        for index in (0..stage_count).rev() {
            let point = &segment.stage_operating_points[index];
            let output_power_w = downstream_power_w;
            let input_power_w =
                output_power_w / point.efficiency_fraction + point.standby_input_power_w;

            if !input_power_w.is_finite()
                || input_power_w < output_power_w
                || (output_power_w > 0.0 && input_power_w <= 0.0)
            {
                return Err(AuditError(format!(
                    "segment {:?}, stage {:?}: derived input power must be finite and not below output power",
                    segment.name, scenario.stages[index].name
                )));
            }

            per_stage_input_power_w[index] = input_power_w;
            per_stage_output_power_w[index] = output_power_w;
            downstream_power_w = input_power_w;
        }

        let hours = segment.hours_per_year;
        let delivered_energy_kwh = segment.delivered_load_power_w * hours / 1_000.0;
        let facility_input_energy_kwh = downstream_power_w * hours / 1_000.0;
        let chain_loss_energy_kwh = facility_input_energy_kwh - delivered_energy_kwh;
        let end_to_end_efficiency_percent = if facility_input_energy_kwh > 0.0 {
            Some(100.0 * delivered_energy_kwh / facility_input_energy_kwh)
        } else {
            None
        };

        if [
            delivered_energy_kwh,
            facility_input_energy_kwh,
            chain_loss_energy_kwh,
        ]
        .iter()
        .any(|value| !value.is_finite())
            || delivered_energy_kwh < 0.0
            || facility_input_energy_kwh < delivered_energy_kwh
            || chain_loss_energy_kwh < -1e-10
            || end_to_end_efficiency_percent
                .is_some_and(|value| !value.is_finite() || !(0.0..=100.0).contains(&value))
        {
            return Err(AuditError(format!(
                "segment {:?}: derived energy balance is invalid",
                segment.name
            )));
        }

        for index in 0..stage_count {
            let input_energy = per_stage_input_power_w[index] * hours / 1_000.0;
            let output_energy = per_stage_output_power_w[index] * hours / 1_000.0;
            let loss_energy = input_energy - output_energy;

            stage_totals[index].input_energy_kwh += input_energy;
            stage_totals[index].output_energy_kwh += output_energy;
            stage_totals[index].total_energy_loss_kwh += loss_energy;
            stage_totals[index].efficiency_evidence.push(EnergyEvidenceReference {
                kind: segment.stage_operating_points[index].efficiency_evidence,
                reference: segment.stage_operating_points[index].efficiency_reference.clone(),
            });
            stage_totals[index].standby_evidence.push(EnergyEvidenceReference {
                kind: segment.stage_operating_points[index].standby_evidence,
                reference: segment.stage_operating_points[index].standby_reference.clone(),
            });
        }

        segment_reports.push(EnergySegmentReport {
            segment_id: segment.id.clone(),
            segment_name: segment.name.clone(),
            hours_per_year: hours,
            delivered_load_power_w: segment.delivered_load_power_w,
            facility_input_power_w: downstream_power_w,
            delivered_energy_kwh,
            facility_input_energy_kwh,
            chain_loss_energy_kwh,
            end_to_end_efficiency_percent,
            stages: scenario
                .stages
                .iter()
                .enumerate()
                .map(|(index, stage)| EnergyStageEnergy {
                    stage_id: stage.id.clone(),
                    stage_name: stage.name.clone(),
                    boundary: stage.boundary.clone(),
                    input_energy_kwh: per_stage_input_power_w[index] * hours / 1_000.0,
                    output_energy_kwh: per_stage_output_power_w[index] * hours / 1_000.0,
                    total_energy_loss_kwh: (per_stage_input_power_w[index]
                        - per_stage_output_power_w[index])
                        * hours
                        / 1_000.0,
                    efficiency_evidence: vec![EnergyEvidenceReference {
                        kind: segment.stage_operating_points[index].efficiency_evidence,
                        reference: segment.stage_operating_points[index].efficiency_reference.clone(),
                    }],
                    standby_evidence: vec![EnergyEvidenceReference {
                        kind: segment.stage_operating_points[index].standby_evidence,
                        reference: segment.stage_operating_points[index].standby_reference.clone(),
                    }],
                })
                .collect(),
        });

        total_delivered_energy_kwh += delivered_energy_kwh;
        total_input_energy_kwh += facility_input_energy_kwh;
    }

    if !total_delivered_energy_kwh.is_finite()
        || !total_input_energy_kwh.is_finite()
        || total_input_energy_kwh <= 0.0
        || total_input_energy_kwh + 1e-10 < total_delivered_energy_kwh
    {
        return Err(AuditError(
            "annual energy aggregation is invalid or overflowed".to_string(),
        ));
    }

    let total_chain_loss_energy_kwh =
        total_input_energy_kwh - total_delivered_energy_kwh;
    let annual_efficiency =
        100.0 * total_delivered_energy_kwh / total_input_energy_kwh;
    let annual_electricity_cost =
        total_input_energy_kwh * scenario.electricity_price_per_kwh;
    let annual_grid_emissions_kg =
        total_input_energy_kwh * scenario.grid_carbon_kg_per_kwh;

    if [
        total_chain_loss_energy_kwh,
        annual_efficiency,
        annual_electricity_cost,
        annual_grid_emissions_kg,
    ]
    .iter()
    .any(|value| !value.is_finite())
    {
        return Err(AuditError(
            "annual cost, emissions or efficiency aggregation overflowed".to_string(),
        ));
    }

    Ok(EnergyChainReport {
        scenario_id: scenario.id.clone(),
        scenario_name: scenario.name.clone(),
        total_hours_per_year: total_hours,
        delivered_energy_kwh: total_delivered_energy_kwh,
        facility_input_energy_kwh: total_input_energy_kwh,
        total_chain_loss_energy_kwh,
        annual_end_to_end_efficiency_percent: annual_efficiency,
        currency_code: scenario.currency_code.clone(),
        electricity_price_per_kwh: scenario.electricity_price_per_kwh,
        electricity_price_reference: scenario.electricity_price_reference.clone(),
        grid_carbon_kg_per_kwh: scenario.grid_carbon_kg_per_kwh,
        grid_carbon_reference: scenario.grid_carbon_reference.clone(),
        annual_electricity_cost,
        annual_grid_emissions_kg,
        stages: stage_totals,
        segments: segment_reports,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        evaluate_energy_chain, EnergyChainScenario, EnergyEvidenceKind, EnergyLoadSegment,
        EnergyStage, EnergyStageOperatingPoint,
    };

    fn stage(id: &str, name: &str) -> EnergyStage {
        EnergyStage {
            id: id.to_string(),
            name: name.to_string(),
            boundary: format!("{name} electrical input -> electrical output"),
        }
    }

    fn point(efficiency: f64, standby_w: f64, reference: &str) -> EnergyStageOperatingPoint {
        EnergyStageOperatingPoint {
            efficiency_fraction: efficiency,
            standby_input_power_w: standby_w,
            efficiency_evidence: EnergyEvidenceKind::AnalyticalModel,
            efficiency_reference: reference.to_string(),
            standby_evidence: EnergyEvidenceKind::Assumption,
            standby_reference: format!("assumption:{reference}:standby"),
        }
    }

    fn scenario() -> EnergyChainScenario {
        EnergyChainScenario {
            id: "synthetic-chain-1".to_string(),
            name: "Synthetic two-stage DC conversion chain".to_string(),
            currency_code: "ZAR".to_string(),
            electricity_price_per_kwh: 0.20,
            electricity_price_reference: "synthetic:tariff".to_string(),
            grid_carbon_kg_per_kwh: 0.70,
            grid_carbon_reference: "synthetic:grid-carbon".to_string(),
            stages: vec![
                stage("dcx", "800V-to-50V DCX"),
                stage("bus", "50V-to-load converter"),
            ],
            segments: vec![EnergyLoadSegment {
                id: "full-load".to_string(),
                name: "Full load".to_string(),
                hours_per_year: 8_760.0,
                delivered_load_power_w: 1_000.0,
                stage_operating_points: vec![
                    point(0.98, 5.0, "synthetic:dcx:98%"),
                    point(0.97, 2.0, "synthetic:bus:97%"),
                ],
            }],
        }
    }

    #[test]
    fn computes_serial_chain_energy_and_annual_cost() {
        let report = evaluate_energy_chain(&scenario()).unwrap();

        let stage2_input = 1_000.0 / 0.97 + 2.0;
        let facility_input = stage2_input / 0.98 + 5.0;
        let expected_input_kwh = facility_input * 8_760.0 / 1_000.0;
        let expected_output_kwh = 1_000.0 * 8_760.0 / 1_000.0;

        assert!((report.segments[0].facility_input_power_w - facility_input).abs() < 1e-9);
        assert!((report.facility_input_energy_kwh - expected_input_kwh).abs() < 1e-9);
        assert!((report.delivered_energy_kwh - expected_output_kwh).abs() < 1e-9);
        assert!((report.total_chain_loss_energy_kwh - (expected_input_kwh - expected_output_kwh)).abs() < 1e-9);
        assert!((report.annual_electricity_cost - expected_input_kwh * 0.20).abs() < 1e-9);
        assert!((report.annual_grid_emissions_kg - expected_input_kwh * 0.70).abs() < 1e-9);
        assert!(report.annual_end_to_end_efficiency_percent < 97.0);
    }

    #[test]
    fn uses_load_dependent_efficiency_without_interpolation() {
        let mut s = scenario();
        s.segments = vec![
            EnergyLoadSegment {
                id: "light".to_string(),
                name: "Light load".to_string(),
                hours_per_year: 4_000.0,
                delivered_load_power_w: 200.0,
                stage_operating_points: vec![
                    point(0.95, 5.0, "model:dcx:light"),
                    point(0.90, 2.0, "model:bus:light"),
                ],
            },
            EnergyLoadSegment {
                id: "heavy".to_string(),
                name: "Heavy load".to_string(),
                hours_per_year: 4_760.0,
                delivered_load_power_w: 1_000.0,
                stage_operating_points: vec![
                    point(0.98, 5.0, "model:dcx:heavy"),
                    point(0.97, 2.0, "model:bus:heavy"),
                ],
            },
        ];

        let report = evaluate_energy_chain(&s).unwrap();
        assert_eq!(report.segments.len(), 2);
        assert!(report.segments[0].end_to_end_efficiency_percent.unwrap()
            < report.segments[1].end_to_end_efficiency_percent.unwrap());
        assert!((report.total_hours_per_year - 8_760.0).abs() < 1e-9);
    }

    #[test]
    fn rejects_missing_stage_operating_point() {
        let mut s = scenario();
        s.segments[0].stage_operating_points.pop();
        assert!(evaluate_energy_chain(&s).is_err());
    }

    #[test]
    fn rejects_idealized_or_invalid_efficiency() {
        for invalid in [0.0, -0.1, 1.0, 1.01, f64::NAN] {
            let mut s = scenario();
            s.segments[0].stage_operating_points[0].efficiency_fraction = invalid;
            assert!(evaluate_energy_chain(&s).is_err());
        }
    }

    #[test]
    fn rejects_missing_efficiency_provenance() {
        let mut s = scenario();
        s.segments[0].stage_operating_points[0].efficiency_reference.clear();
        assert!(evaluate_energy_chain(&s).is_err());
    }

    #[test]
    fn rejects_missing_standby_provenance() {
        let mut s = scenario();
        s.segments[0].stage_operating_points[0].standby_reference.clear();
        assert!(evaluate_energy_chain(&s).is_err());
    }

    #[test]
    fn accepts_explicit_zero_load_standby_interval() {
        let mut s = scenario();
        s.segments[0].hours_per_year = 8_000.0;
        s.segments[0].delivered_load_power_w = 1_000.0;
        s.segments.push(EnergyLoadSegment {
            id: "idle".to_string(),
            name: "Energized idle".to_string(),
            hours_per_year: 760.0,
            delivered_load_power_w: 0.0,
            stage_operating_points: vec![
                point(0.95, 4.0, "synthetic:dcx:idle"),
                point(0.90, 1.0, "synthetic:bus:idle"),
            ],
        });

        let report = evaluate_energy_chain(&s).unwrap();
        let idle = report.segments.iter().find(|segment| segment.segment_id == "idle").unwrap();
        assert_eq!(idle.delivered_energy_kwh, 0.0);
        assert!(idle.facility_input_energy_kwh > 0.0);
        assert_eq!(idle.end_to_end_efficiency_percent, Some(0.0));
    }

    #[test]
    fn rejects_incomplete_annual_hours() {
        let mut s = scenario();
        s.segments[0].hours_per_year = 8_000.0;
        assert!(evaluate_energy_chain(&s).is_err());
    }

    #[test]
    fn rejects_annual_hours_above_calendar_limit() {
        let mut s = scenario();
        s.segments[0].hours_per_year = 8_761.0;
        assert!(evaluate_energy_chain(&s).is_err());
    }

    #[test]
    fn rejects_duplicate_stage_or_segment_ids() {
        let mut s = scenario();
        s.stages[1].id = s.stages[0].id.clone();
        assert!(evaluate_energy_chain(&s).is_err());

        let mut s = scenario();
        s.segments.push(s.segments[0].clone());
        assert!(evaluate_energy_chain(&s).is_err());
    }

    #[test]
    fn rejects_missing_tariff_and_carbon_provenance() {
        let mut s = scenario();
        s.electricity_price_reference.clear();
        assert!(evaluate_energy_chain(&s).is_err());

        let mut s = scenario();
        s.grid_carbon_reference.clear();
        assert!(evaluate_energy_chain(&s).is_err());
    }

    #[test]
    fn rejects_invalid_currency_code() {
        let mut s = scenario();
        s.currency_code = "usd".to_string();
        assert!(evaluate_energy_chain(&s).is_err());

        let mut s = scenario();
        s.currency_code = "US".to_string();
        assert!(evaluate_energy_chain(&s).is_err());
    }

    #[test]
    fn rejects_negative_standby_and_cost_inputs() {
        let mut s = scenario();
        s.segments[0].stage_operating_points[0].standby_input_power_w = -1.0;
        assert!(evaluate_energy_chain(&s).is_err());

        let mut s = scenario();
        s.electricity_price_per_kwh = -0.01;
        assert!(evaluate_energy_chain(&s).is_err());
    }

    #[test]
    fn stage_energy_balances_close() {
        let report = evaluate_energy_chain(&scenario()).unwrap();
        let stage_losses: f64 = report
            .stages
            .iter()
            .map(|stage| stage.total_energy_loss_kwh)
            .sum();

        assert!((stage_losses - report.total_chain_loss_energy_kwh).abs() < 1e-8);
    }
}
