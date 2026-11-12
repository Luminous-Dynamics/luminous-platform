use crate::dab_sps::{screen_dab_sps, DabSpsOperatingPoint};
use crate::dab_sps_physics::{evaluate_dab_sps_physics, DabSpsPhysicsInput};
use crate::dab_sps_resistive::{evaluate_dab_sps_resistive, DabSpsResistiveInput};
use crate::AuditError;

/// One frozen operating point in a mission profile. Weight is expressed in hours
/// (or another common duration unit); absolute scaling cancels in coverage and
/// energy ratios. A zero-power standby point belongs in a separate standby test.
#[derive(Debug, Clone, PartialEq)]
pub struct DabMissionPoint {
    pub point_id: String,
    pub source_voltage_v: f64,
    pub destination_voltage_referred_v: f64,
    pub requested_signed_power_w: f64,
    pub ambient_temperature_c: f64,
    pub hard_switching_loss_w: f64,
    pub mission_weight_hours: f64,
}

/// A deterministic candidate configuration for a first-order DAB design sweep.
/// Hard-switching loss and thermal resistance are explicit per-point / candidate
/// inputs, not derived automatically from a device model.
#[derive(Debug, Clone, PartialEq)]
pub struct DabDesignCandidate {
    pub candidate_id: String,
    pub series_inductance_referred_h: f64,
    pub switching_frequency_hz: f64,
    pub equivalent_series_resistance_ohm: f64,
    pub maximum_phase_shift_rad: f64,
    pub maximum_peak_current_a: Option<f64>,
    pub require_analytic_zvs_polarity: bool,
    pub residual_zvs_loss_fraction: f64,
    pub junction_to_ambient_thermal_resistance_k_per_w: f64,
    pub junction_temperature_limit_c: f64,
    pub minimum_power_delivery_ratio: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DabPointFailure {
    PowerDemandAboveIdealTransferLimit,
    PhaseShiftLimitExceeded,
    AnalyticZvsPolarityRequiredButNotMet,
    PeakCurrentLimitExceeded,
    ThermalLimitExceeded,
    PowerDeliveryBelowMinimum,
    PowerDirectionMismatch,
    ModelEvaluationInvalid,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DabMissionPointResult {
    pub point_id: String,
    pub mission_weight_hours: f64,
    pub requested_signed_power_w: f64,
    pub signed_phase_shift_rad: Option<f64>,
    pub signed_delivered_power_w: Option<f64>,
    pub power_delivery_ratio: Option<f64>,
    pub inductor_rms_current_a: Option<f64>,
    pub inductor_peak_current_a: Option<f64>,
    pub fha_overestimation_percent: Option<f64>,
    pub analytic_zvs_polarity_met: Option<bool>,
    pub series_resistive_loss_w: Option<f64>,
    pub switching_loss_w: Option<f64>,
    pub estimated_junction_temperature_c: Option<f64>,
    pub thermal_margin_c: Option<f64>,
    pub modelled_efficiency_percent: Option<f64>,
    pub failures: Vec<DabPointFailure>,
}

impl DabMissionPointResult {
    pub fn is_feasible(&self) -> bool {
        self.failures.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DabDesignSummary {
    pub candidate_id: String,
    pub total_mission_weight_hours: f64,
    pub modelled_mission_efficiency_percent: Option<f64>,
    pub mission_energy_delivery_ratio: Option<f64>,
    pub weighted_feasible_coverage_percent: f64,
    pub weighted_analytic_zvs_polarity_coverage_percent: f64,
    pub maximum_peak_current_a: Option<f64>,
    pub minimum_thermal_margin_c: Option<f64>,
    pub modelled_point_count: usize,
    pub total_point_count: usize,
    pub full_mission_feasible: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DabDesignEvaluation {
    pub candidate_id: String,
    pub point_results: Vec<DabMissionPointResult>,
    pub summary: DabDesignSummary,
}

impl DabDesignEvaluation {
    pub const fn claim_ceiling(&self) -> &'static str {
        "DETERMINISTIC_DAB_MISSION_PROFILE_SCREEN_NOT_FULL_DEVICE_MODEL_OR_QUALIFICATION"
    }
}

fn validate_candidate(candidate: &DabDesignCandidate) -> Result<(), AuditError> {
    if candidate.candidate_id.trim().is_empty() {
        return Err(AuditError("DAB candidate_id is required".to_string()));
    }
    let values = [
        candidate.series_inductance_referred_h,
        candidate.switching_frequency_hz,
        candidate.equivalent_series_resistance_ohm,
        candidate.maximum_phase_shift_rad,
        candidate.residual_zvs_loss_fraction,
        candidate.junction_to_ambient_thermal_resistance_k_per_w,
        candidate.junction_temperature_limit_c,
        candidate.minimum_power_delivery_ratio,
    ];
    if values.iter().any(|value| !value.is_finite()) {
        return Err(AuditError("DAB candidate parameters must be finite".to_string()));
    }
    if candidate.series_inductance_referred_h <= 0.0
        || candidate.switching_frequency_hz <= 0.0
        || candidate.equivalent_series_resistance_ohm < 0.0
        || candidate.maximum_phase_shift_rad <= 0.0
        || candidate.maximum_phase_shift_rad > std::f64::consts::PI / 2.0
        || !(0.0..=1.0).contains(&candidate.residual_zvs_loss_fraction)
        || candidate.junction_to_ambient_thermal_resistance_k_per_w < 0.0
        || candidate.minimum_power_delivery_ratio < 0.0
        || candidate.minimum_power_delivery_ratio > 1.5
    {
        return Err(AuditError("DAB candidate parameters are outside their valid ranges".to_string()));
    }
    if let Some(limit) = candidate.maximum_peak_current_a {
        if !limit.is_finite() || limit <= 0.0 {
            return Err(AuditError("DAB maximum_peak_current_a must be positive and finite when set".to_string()));
        }
    }
    Ok(())
}

fn validate_mission(points: &[DabMissionPoint]) -> Result<(), AuditError> {
    if points.is_empty() {
        return Err(AuditError("DAB mission profile must contain at least one point".to_string()));
    }
    let mut ids = std::collections::HashSet::new();
    for point in points {
        if point.point_id.trim().is_empty() || !ids.insert(point.point_id.as_str()) {
            return Err(AuditError("DAB mission point IDs must be non-empty and unique".to_string()));
        }
        let values = [
            point.source_voltage_v,
            point.destination_voltage_referred_v,
            point.requested_signed_power_w,
            point.ambient_temperature_c,
            point.hard_switching_loss_w,
            point.mission_weight_hours,
        ];
        if values.iter().any(|value| !value.is_finite()) {
            return Err(AuditError(format!("mission point {} has non-finite fields", point.point_id)));
        }
        if point.source_voltage_v <= 0.0
            || point.destination_voltage_referred_v <= 0.0
            || point.requested_signed_power_w == 0.0
            || point.hard_switching_loss_w < 0.0
            || point.mission_weight_hours <= 0.0
        {
            return Err(AuditError(format!("mission point {} has invalid values", point.point_id)));
        }
    }
    Ok(())
}

/// Evaluate one DAB candidate against every frozen mission point.
///
/// Phase is selected from the ideal SPS power equation, then current/power is
/// recomputed with the resistive periodic RL model. Every point remains in the
/// result, including points that fail power, phase, current, ZVS-polarity,
/// thermal or delivery requirements. No infeasible point is silently dropped.
pub fn evaluate_dab_design_candidate(
    candidate: &DabDesignCandidate,
    mission: &[DabMissionPoint],
) -> Result<DabDesignEvaluation, AuditError> {
    validate_candidate(candidate)?;
    validate_mission(mission)?;

    let mut point_results = Vec::with_capacity(mission.len());
    let mut total_weight = 0.0_f64;
    let mut feasible_weight = 0.0_f64;
    let mut zvs_weight = 0.0_f64;
    let mut output_energy_wh = 0.0_f64;
    let mut input_energy_wh = 0.0_f64;
    let mut requested_energy_wh = 0.0_f64;
    let mut max_peak_current = 0.0_f64;
    let mut min_thermal_margin = f64::INFINITY;
    let mut modelled_point_count = 0_usize;

    for point in mission {
        total_weight += point.mission_weight_hours;
        requested_energy_wh +=
            point.mission_weight_hours * point.requested_signed_power_w.abs();

        let mut result = DabMissionPointResult {
            point_id: point.point_id.clone(),
            mission_weight_hours: point.mission_weight_hours,
            requested_signed_power_w: point.requested_signed_power_w,
            signed_phase_shift_rad: None,
            signed_delivered_power_w: None,
            power_delivery_ratio: None,
            inductor_rms_current_a: None,
            inductor_peak_current_a: None,
            fha_overestimation_percent: None,
            analytic_zvs_polarity_met: None,
            series_resistive_loss_w: None,
            switching_loss_w: None,
            estimated_junction_temperature_c: None,
            thermal_margin_c: None,
            modelled_efficiency_percent: None,
            failures: Vec::new(),
        };

        let phase = match screen_dab_sps(DabSpsOperatingPoint {
            port_a_voltage_v: point.source_voltage_v,
            port_b_voltage_referred_to_a_v: point.destination_voltage_referred_v,
            leakage_inductance_referred_to_a_h: candidate.series_inductance_referred_h,
            switching_frequency_hz: candidate.switching_frequency_hz,
            requested_signed_power_a_to_b_w: point.requested_signed_power_w,
        }) {
            Ok(value) => value,
            Err(_) => {
                result.failures.push(DabPointFailure::PowerDemandAboveIdealTransferLimit);
                point_results.push(result);
                continue;
            }
        };

        result.signed_phase_shift_rad = Some(phase.signed_phase_shift_rad);
        if phase.signed_phase_shift_rad.abs() > candidate.maximum_phase_shift_rad {
            result.failures.push(DabPointFailure::PhaseShiftLimitExceeded);
        }

        let physics = match evaluate_dab_sps_physics(DabSpsPhysicsInput {
            source_voltage_v: point.source_voltage_v,
            destination_voltage_referred_v: point.destination_voltage_referred_v,
            series_inductance_referred_h: candidate.series_inductance_referred_h,
            switching_frequency_hz: candidate.switching_frequency_hz,
            signed_phase_shift_rad: phase.signed_phase_shift_rad,
        }) {
            Ok(value) => value,
            Err(_) => {
                result.failures.push(DabPointFailure::ModelEvaluationInvalid);
                point_results.push(result);
                continue;
            }
        };

        let resistive = match evaluate_dab_sps_resistive(DabSpsResistiveInput {
            source_voltage_v: point.source_voltage_v,
            destination_voltage_referred_v: point.destination_voltage_referred_v,
            series_inductance_referred_h: candidate.series_inductance_referred_h,
            switching_frequency_hz: candidate.switching_frequency_hz,
            signed_phase_shift_rad: phase.signed_phase_shift_rad,
            equivalent_series_resistance_ohm: candidate.equivalent_series_resistance_ohm,
        }) {
            Ok(value) => value,
            Err(_) => {
                result.failures.push(DabPointFailure::ModelEvaluationInvalid);
                point_results.push(result);
                continue;
            }
        };

        let zvs_met = physics.meets_analytic_zvs_polarity_screen;
        let switching_loss = if zvs_met {
            point.hard_switching_loss_w * candidate.residual_zvs_loss_fraction
        } else {
            point.hard_switching_loss_w
        };
        let total_loss = resistive.series_resistive_loss_w + switching_loss;
        let junction_temperature = point.ambient_temperature_c
            + total_loss * candidate.junction_to_ambient_thermal_resistance_k_per_w;
        let thermal_margin = candidate.junction_temperature_limit_c - junction_temperature;
        // For A_TO_B, port A supplies and port B receives. For B_TO_A,
        // port B supplies and port A receives. Keep the delivered power sign
        // tied to the requested direction while measuring the receiving port.
        let requested_direction = point.requested_signed_power_w.signum();
        let (delivered_magnitude_w, converter_input_port_power_w) =
            if requested_direction > 0.0 {
                (
                    resistive.destination_port_power_w.abs(),
                    resistive.source_port_power_w.abs(),
                )
            } else {
                (
                    resistive.source_port_power_w.abs(),
                    resistive.destination_port_power_w.abs(),
                )
            };
        let signed_delivered_power = requested_direction * delivered_magnitude_w;
        let delivery_ratio =
            delivered_magnitude_w / point.requested_signed_power_w.abs();
        let modelled_input_power =
            converter_input_port_power_w + switching_loss;
        let modelled_efficiency = if modelled_input_power > 0.0 {
            100.0 * signed_delivered_power.abs() / modelled_input_power
        } else {
            f64::NAN
        };

        let expected_sign = point.requested_signed_power_w.signum();
        if signed_delivered_power.signum() != expected_sign {
            result.failures.push(DabPointFailure::PowerDirectionMismatch);
        }
        if delivery_ratio < candidate.minimum_power_delivery_ratio {
            result.failures.push(DabPointFailure::PowerDeliveryBelowMinimum);
        }
        if candidate.require_analytic_zvs_polarity && !zvs_met {
            result.failures.push(DabPointFailure::AnalyticZvsPolarityRequiredButNotMet);
        }
        if let Some(limit) = candidate.maximum_peak_current_a {
            if resistive.inductor_peak_current_a > limit {
                result.failures.push(DabPointFailure::PeakCurrentLimitExceeded);
            }
        }
        if junction_temperature > candidate.junction_temperature_limit_c {
            result.failures.push(DabPointFailure::ThermalLimitExceeded);
        }

        if [
            signed_delivered_power,
            delivery_ratio,
            resistive.inductor_rms_current_a,
            resistive.inductor_peak_current_a,
            physics.fha_overestimation_percent,
            resistive.series_resistive_loss_w,
            switching_loss,
            junction_temperature,
            thermal_margin,
            modelled_efficiency,
        ]
        .iter()
        .any(|value| !value.is_finite())
        {
            result.failures.push(DabPointFailure::ModelEvaluationInvalid);
        } else {
            result.signed_delivered_power_w = Some(signed_delivered_power);
            result.power_delivery_ratio = Some(delivery_ratio);
            result.inductor_rms_current_a = Some(resistive.inductor_rms_current_a);
            result.inductor_peak_current_a = Some(resistive.inductor_peak_current_a);
            result.fha_overestimation_percent = Some(physics.fha_overestimation_percent);
            result.analytic_zvs_polarity_met = Some(zvs_met);
            result.series_resistive_loss_w = Some(resistive.series_resistive_loss_w);
            result.switching_loss_w = Some(switching_loss);
            result.estimated_junction_temperature_c = Some(junction_temperature);
            result.thermal_margin_c = Some(thermal_margin);
            result.modelled_efficiency_percent = Some(modelled_efficiency);

            modelled_point_count += 1;
            output_energy_wh +=
                point.mission_weight_hours * signed_delivered_power.abs();
            input_energy_wh +=
                point.mission_weight_hours * modelled_input_power;
            max_peak_current = max_peak_current.max(resistive.inductor_peak_current_a);
            min_thermal_margin = min_thermal_margin.min(thermal_margin);
            if zvs_met {
                zvs_weight += point.mission_weight_hours;
            }
        }

        if result.failures.is_empty() {
            feasible_weight += point.mission_weight_hours;
        }
        point_results.push(result);
    }

    let modelled_efficiency = if modelled_point_count == mission.len() && input_energy_wh > 0.0 {
        Some(100.0 * output_energy_wh / input_energy_wh)
    } else {
        None
    };
    let delivery_ratio = if modelled_point_count == mission.len() && requested_energy_wh > 0.0 {
        Some(output_energy_wh / requested_energy_wh)
    } else {
        None
    };
    let zvs_coverage = 100.0 * zvs_weight / total_weight;
    let feasible_coverage = 100.0 * feasible_weight / total_weight;
    let full_mission_feasible = modelled_point_count == mission.len()
        && point_results.iter().all(DabMissionPointResult::is_feasible);

    Ok(DabDesignEvaluation {
        candidate_id: candidate.candidate_id.clone(),
        point_results,
        summary: DabDesignSummary {
            candidate_id: candidate.candidate_id.clone(),
            total_mission_weight_hours: total_weight,
            modelled_mission_efficiency_percent: modelled_efficiency,
            mission_energy_delivery_ratio: delivery_ratio,
            weighted_feasible_coverage_percent: feasible_coverage,
            weighted_analytic_zvs_polarity_coverage_percent: zvs_coverage,
            maximum_peak_current_a: if modelled_point_count > 0 {
                Some(max_peak_current)
            } else {
                None
            },
            minimum_thermal_margin_c: if modelled_point_count > 0 {
                Some(min_thermal_margin)
            } else {
                None
            },
            modelled_point_count,
            total_point_count: mission.len(),
            full_mission_feasible,
        },
    })
}

fn dominates(a: &DabDesignSummary, b: &DabDesignSummary) -> bool {
    let (Some(a_eff), Some(b_eff)) = (
        a.modelled_mission_efficiency_percent,
        b.modelled_mission_efficiency_percent,
    ) else {
        return false;
    };
    let (Some(a_delivery), Some(b_delivery)) = (
        a.mission_energy_delivery_ratio,
        b.mission_energy_delivery_ratio,
    ) else {
        return false;
    };
    let (Some(a_current), Some(b_current)) =
        (a.maximum_peak_current_a, b.maximum_peak_current_a)
    else {
        return false;
    };
    let (Some(a_margin), Some(b_margin)) =
        (a.minimum_thermal_margin_c, b.minimum_thermal_margin_c)
    else {
        return false;
    };

    let no_worse = a_eff >= b_eff
        && a_delivery >= b_delivery
        && a.weighted_analytic_zvs_polarity_coverage_percent
            >= b.weighted_analytic_zvs_polarity_coverage_percent
        && a_current <= b_current
        && a_margin >= b_margin;
    let strictly_better = a_eff > b_eff
        || a_delivery > b_delivery
        || a.weighted_analytic_zvs_polarity_coverage_percent
            > b.weighted_analytic_zvs_polarity_coverage_percent
        || a_current < b_current
        || a_margin > b_margin;
    no_worse && strictly_better
}

/// Return candidate IDs on a transparent Pareto frontier.
///
/// Only candidates that are fully feasible at every frozen mission point are
/// considered. Objectives are: maximize mission efficiency, delivered-energy
/// ratio, analytic ZVS-polarity coverage and thermal margin; minimize peak
/// current. Cost, power density, reactive power, EMI and serviceability are not
/// included until corresponding validated models and evidence exist.
pub fn pareto_frontier_ids(evaluations: &[DabDesignEvaluation]) -> Vec<String> {
    let feasible: Vec<&DabDesignSummary> = evaluations
        .iter()
        .filter(|evaluation| evaluation.summary.full_mission_feasible)
        .map(|evaluation| &evaluation.summary)
        .collect();

    feasible
        .iter()
        .filter(|candidate| {
            !feasible
                .iter()
                .any(|other| other.candidate_id != candidate.candidate_id && dominates(other, candidate))
        })
        .map(|candidate| candidate.candidate_id.clone())
        .collect()
}

/// Evaluate a deterministic set of predeclared candidates against the same mission profile.
pub fn evaluate_dab_design_set(
    candidates: &[DabDesignCandidate],
    mission: &[DabMissionPoint],
) -> Result<(Vec<DabDesignEvaluation>, Vec<String>), AuditError> {
    if candidates.is_empty() {
        return Err(AuditError("DAB design set must contain at least one candidate".to_string()));
    }
    let mut ids = std::collections::HashSet::new();
    let mut evaluations = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        if !ids.insert(candidate.candidate_id.as_str()) {
            return Err(AuditError("DAB candidate IDs must be unique".to_string()));
        }
        evaluations.push(evaluate_dab_design_candidate(candidate, mission)?);
    }
    let frontier = pareto_frontier_ids(&evaluations);
    Ok((evaluations, frontier))
}

#[cfg(test)]
mod tests {
    use super::{
        evaluate_dab_design_candidate, evaluate_dab_design_set, DabDesignCandidate,
        DabMissionPoint, DabPointFailure,
    };

    fn mission() -> Vec<DabMissionPoint> {
        vec![
            DabMissionPoint {
                point_id: "forward-500w".to_string(),
                source_voltage_v: 48.0,
                destination_voltage_referred_v: 48.0,
                requested_signed_power_w: 500.0,
                ambient_temperature_c: 25.0,
                hard_switching_loss_w: 0.5,
                mission_weight_hours: 4.0,
            },
            DabMissionPoint {
                point_id: "reverse-250w".to_string(),
                source_voltage_v: 48.0,
                destination_voltage_referred_v: 48.0,
                requested_signed_power_w: -250.0,
                ambient_temperature_c: 30.0,
                hard_switching_loss_w: 0.4,
                mission_weight_hours: 2.0,
            },
        ]
    }

    fn candidate(id: &str, resistance_ohm: f64, max_phase_degrees: f64) -> DabDesignCandidate {
        DabDesignCandidate {
            candidate_id: id.to_string(),
            series_inductance_referred_h: 3.84e-6,
            switching_frequency_hz: 100_000.0,
            equivalent_series_resistance_ohm: resistance_ohm,
            maximum_phase_shift_rad: max_phase_degrees.to_radians(),
            maximum_peak_current_a: Some(100.0),
            require_analytic_zvs_polarity: true,
            residual_zvs_loss_fraction: 0.05,
            junction_to_ambient_thermal_resistance_k_per_w: 2.0,
            junction_temperature_limit_c: 125.0,
            minimum_power_delivery_ratio: 0.98,
        }
    }

    #[test]
    fn evaluates_all_forward_and_reverse_mission_points() {
        let result = evaluate_dab_design_candidate(&candidate("base", 0.005, 90.0), &mission()).unwrap();
        assert_eq!(result.point_results.len(), 2);
        assert!(result.summary.full_mission_feasible);
        assert_eq!(result.summary.modelled_point_count, 2);
        assert!(result.summary.modelled_mission_efficiency_percent.unwrap() < 100.0);
        assert!(result.point_results[1].modelled_efficiency_percent.unwrap() < 100.0);
        assert!(result.point_results[1].signed_delivered_power_w.unwrap() < 0.0);
        assert!(result.summary.weighted_analytic_zvs_polarity_coverage_percent > 99.9);
        assert!(result.point_results.iter().all(|point| point.failures.is_empty()));
    }

    #[test]
    fn retains_phase_shift_failures_instead_of_dropping_points() {
        let result = evaluate_dab_design_candidate(&candidate("limited", 0.005, 20.0), &mission()).unwrap();
        assert_eq!(result.point_results.len(), 2);
        assert!(!result.summary.full_mission_feasible);
        assert!(result.point_results[0]
            .failures
            .contains(&DabPointFailure::PhaseShiftLimitExceeded));
        assert!(result.point_results[0].signed_delivered_power_w.is_some());
    }

    #[test]
    fn retains_ideal_power_infeasible_points() {
        let mut c = candidate("low-capacity", 0.005, 90.0);
        c.series_inductance_referred_h = 10e-6;
        let result = evaluate_dab_design_candidate(&c, &mission()).unwrap();
        assert_eq!(result.point_results.len(), 2);
        assert!(result.point_results[0]
            .failures
            .contains(&DabPointFailure::PowerDemandAboveIdealTransferLimit));
        assert_eq!(result.summary.modelled_point_count, 1);
        assert!(!result.summary.full_mission_feasible);
    }

    #[test]
    fn emits_a_multiobjective_pareto_frontier() {
        let candidates = vec![
            candidate("higher-loss", 0.02, 90.0),
            candidate("lower-loss", 0.005, 90.0),
        ];
        let (evaluations, frontier) = evaluate_dab_design_set(&candidates, &mission()).unwrap();
        assert_eq!(evaluations.len(), 2);
        assert!(evaluations.iter().all(|result| result.summary.full_mission_feasible));
        assert_eq!(frontier, vec!["lower-loss".to_string()]);
    }

    #[test]
    fn rejects_duplicate_candidate_ids_and_invalid_mission_weights() {
        let c = candidate("duplicate", 0.005, 90.0);
        assert!(evaluate_dab_design_set(&[c.clone(), c], &mission()).is_err());

        let mut bad = mission();
        bad[0].mission_weight_hours = 0.0;
        assert!(evaluate_dab_design_candidate(&candidate("bad", 0.005, 90.0), &bad).is_err());
    }
}
