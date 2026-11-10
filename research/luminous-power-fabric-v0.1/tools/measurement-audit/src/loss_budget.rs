use crate::AuditError;

/// Loss categories are accounting buckets, not an assertion about a topology.
/// The caller supplies loss values from datasheets, analytical models,
/// switching simulation or physical measurements.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LossCategory {
    SemiconductorConduction,
    SemiconductorSwitching,
    GateDrive,
    MagneticCopper,
    MagneticCore,
    Capacitor,
    Interconnect,
    Snubber,
    CoolingAndAuxiliary,
    ControlAndSensing,
    Other,
}

/// Preserve the origin of every value. No evidence kind upgrades another.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LossEvidence {
    DatasheetAtDeclaredOperatingPoint,
    AnalyticalModel,
    SwitchingSimulation,
    PhysicalMeasurement,
    Assumption,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LossTerm {
    pub name: String,
    pub category: LossCategory,
    pub watts: f64,
    /// One-standard-deviation uncertainty in watts, if justified by the source.
    /// Correlation between terms is not assumed to be known.
    pub standard_uncertainty_w: f64,
    pub evidence: LossEvidence,
    pub reference: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LossBudget {
    pub output_power_w: f64,
    /// Fractional target, e.g. 0.98 for 98%. Must be strictly between zero and one.
    pub target_efficiency: f64,
    pub terms: Vec<LossTerm>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LossContribution {
    pub name: String,
    pub category: LossCategory,
    pub watts: f64,
    pub share_of_total_percent: f64,
    pub standard_uncertainty_w: f64,
    pub evidence: LossEvidence,
    pub reference: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LossReport {
    pub output_power_w: f64,
    pub total_loss_w: f64,
    pub input_power_w: f64,
    pub efficiency_percent: f64,
    pub max_loss_for_target_w: f64,
    /// Positive means the target has loss headroom; negative means the budget
    /// exceeds the target allowance.
    pub target_loss_headroom_w: f64,
    /// Root-sum-square uncertainty; interpretable only if component uncertainties
    /// are sufficiently independent.
    pub rss_standard_uncertainty_w: f64,
    /// Linear sum is a conservative bound when covariance is unknown.
    pub linear_uncertainty_bound_w: f64,
    pub contributions: Vec<LossContribution>,
}

impl LossReport {
    pub const fn claim_ceiling(&self) -> &'static str {
        "PARAMETERIZED_LOSS_ACCOUNTING_ONLY_NOT_TOPOLOGY_MODEL_OR_QUALIFICATION"
    }
}

/// Evaluate a caller-supplied loss budget.
///
/// Efficiency is computed at a common power boundary:
/// eta = P_out / (P_out + sum(P_loss_i)).
///
/// This helper does not derive DAB/CLLC waveforms, RMS currents, switching energies,
/// magnetic loss coefficients, thermal resistance, ZVS/ZCS regions or component
/// operating points. Those inputs must come from a separately validated model or
/// measurement, with their source and uncertainty retained in each LossTerm.
pub fn evaluate_loss_budget(budget: &LossBudget) -> Result<LossReport, AuditError> {
    if !budget.output_power_w.is_finite() || budget.output_power_w <= 0.0 {
        return Err(AuditError(
            "output_power_w must be positive and finite".to_string(),
        ));
    }
    if !budget.target_efficiency.is_finite()
        || budget.target_efficiency <= 0.0
        || budget.target_efficiency >= 1.0
    {
        return Err(AuditError(
            "target_efficiency must be a finite fraction strictly between 0 and 1".to_string(),
        ));
    }
    if budget.terms.is_empty() {
        return Err(AuditError(
            "at least one explicit loss term is required; empty budgets must not imply 100% efficiency"
                .to_string(),
        ));
    }

    let mut total_loss_w = 0.0_f64;
    let mut uncertainty_square_sum = 0.0_f64;
    let mut linear_uncertainty_bound_w = 0.0_f64;

    for (index, term) in budget.terms.iter().enumerate() {
        if term.name.trim().is_empty() {
            return Err(AuditError(format!("loss term {index}: name is required")));
        }
        if term.reference.trim().is_empty() {
            return Err(AuditError(format!(
                "loss term {index}: provenance reference is required"
            )));
        }
        if !term.watts.is_finite() || term.watts < 0.0 {
            return Err(AuditError(format!(
                "loss term {index}: watts must be finite and non-negative"
            )));
        }
        if !term.standard_uncertainty_w.is_finite() || term.standard_uncertainty_w < 0.0 {
            return Err(AuditError(format!(
                "loss term {index}: standard_uncertainty_w must be finite and non-negative"
            )));
        }
        total_loss_w += term.watts;
        uncertainty_square_sum += term.standard_uncertainty_w * term.standard_uncertainty_w;
        linear_uncertainty_bound_w += term.standard_uncertainty_w;
    }

    if !total_loss_w.is_finite()
        || !uncertainty_square_sum.is_finite()
        || !linear_uncertainty_bound_w.is_finite()
    {
        return Err(AuditError(
            "loss or uncertainty aggregation overflowed".to_string(),
        ));
    }

    let input_power_w = budget.output_power_w + total_loss_w;
    if !input_power_w.is_finite() || input_power_w <= 0.0 {
        return Err(AuditError(
            "derived input_power_w must be positive and finite".to_string(),
        ));
    }

    let efficiency_percent = 100.0 * budget.output_power_w / input_power_w;
    if !efficiency_percent.is_finite() || efficiency_percent <= 0.0 || efficiency_percent > 100.0 {
        return Err(AuditError(
            "derived efficiency is outside the physical accounting interval".to_string(),
        ));
    }

    let max_loss_for_target_w =
        budget.output_power_w * (1.0 / budget.target_efficiency - 1.0);
    let target_loss_headroom_w = max_loss_for_target_w - total_loss_w;

    let contributions = budget
        .terms
        .iter()
        .map(|term| LossContribution {
            name: term.name.clone(),
            category: term.category,
            watts: term.watts,
            share_of_total_percent: if total_loss_w > 0.0 {
                100.0 * term.watts / total_loss_w
            } else {
                0.0
            },
            standard_uncertainty_w: term.standard_uncertainty_w,
            evidence: term.evidence,
            reference: term.reference.clone(),
        })
        .collect();

    Ok(LossReport {
        output_power_w: budget.output_power_w,
        total_loss_w,
        input_power_w,
        efficiency_percent,
        max_loss_for_target_w,
        target_loss_headroom_w,
        rss_standard_uncertainty_w: uncertainty_square_sum.sqrt(),
        linear_uncertainty_bound_w,
        contributions,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        evaluate_loss_budget, LossBudget, LossCategory, LossEvidence, LossTerm,
    };

    fn term(name: &str, watts: f64, uncertainty: f64) -> LossTerm {
        LossTerm {
            name: name.to_string(),
            category: LossCategory::Other,
            watts,
            standard_uncertainty_w: uncertainty,
            evidence: LossEvidence::AnalyticalModel,
            reference: "fixture://synthetic-loss-input".to_string(),
        }
    }

    #[test]
    fn computes_energy_balance_and_efficiency() {
        let budget = LossBudget {
            output_power_w: 500.0,
            target_efficiency: 0.98,
            terms: vec![term("aggregate loss", 12.5, 0.5)],
        };
        let report = evaluate_loss_budget(&budget).unwrap();

        assert!((report.input_power_w - 512.5).abs() < 1e-9);
        assert!((report.efficiency_percent - (500.0 / 512.5 * 100.0)).abs() < 1e-9);
        assert!((report.total_loss_w - 12.5).abs() < 1e-9);
        assert!(report.target_loss_headroom_w < 0.0);
    }

    #[test]
    fn calculates_loss_allowance_for_97_and_98_percent_targets() {
        let make_budget = |target| LossBudget {
            output_power_w: 500.0,
            target_efficiency: target,
            terms: vec![term("fixture", 1.0, 0.0)],
        };
        let at_97 = evaluate_loss_budget(&make_budget(0.97)).unwrap();
        let at_98 = evaluate_loss_budget(&make_budget(0.98)).unwrap();

        assert!((at_97.max_loss_for_target_w - 500.0 * (1.0 / 0.97 - 1.0)).abs() < 1e-9);
        assert!((at_98.max_loss_for_target_w - 500.0 * (1.0 / 0.98 - 1.0)).abs() < 1e-9);
        assert!(at_98.max_loss_for_target_w < at_97.max_loss_for_target_w);
    }

    #[test]
    fn keeps_evidence_labels_and_term_provenance() {
        let mut physical = term("measured copper", 2.0, 0.1);
        physical.evidence = LossEvidence::PhysicalMeasurement;
        physical.category = LossCategory::MagneticCopper;
        physical.reference = "run://physical-run-42".to_string();

        let report = evaluate_loss_budget(&LossBudget {
            output_power_w: 100.0,
            target_efficiency: 0.95,
            terms: vec![physical],
        })
        .unwrap();

        assert_eq!(report.contributions[0].evidence, LossEvidence::PhysicalMeasurement);
        assert_eq!(report.contributions[0].reference, "run://physical-run-42");
        assert_eq!(report.contributions[0].category, LossCategory::MagneticCopper);
    }

    #[test]
    fn reports_rss_and_conservative_linear_uncertainty() {
        let report = evaluate_loss_budget(&LossBudget {
            output_power_w: 100.0,
            target_efficiency: 0.95,
            terms: vec![term("a", 3.0, 3.0), term("b", 4.0, 4.0)],
        })
        .unwrap();

        assert!((report.rss_standard_uncertainty_w - 5.0).abs() < 1e-9);
        assert!((report.linear_uncertainty_bound_w - 7.0).abs() < 1e-9);
    }

    #[test]
    fn rejects_empty_loss_budget() {
        let result = evaluate_loss_budget(&LossBudget {
            output_power_w: 500.0,
            target_efficiency: 0.98,
            terms: vec![],
        });
        assert!(result.is_err());
    }

    #[test]
    fn rejects_negative_loss() {
        let result = evaluate_loss_budget(&LossBudget {
            output_power_w: 500.0,
            target_efficiency: 0.98,
            terms: vec![term("negative power", -0.1, 0.0)],
        });
        assert!(result.is_err());
    }

    #[test]
    fn rejects_non_finite_loss() {
        let result = evaluate_loss_budget(&LossBudget {
            output_power_w: 500.0,
            target_efficiency: 0.98,
            terms: vec![term("nan", f64::NAN, 0.0)],
        });
        assert!(result.is_err());
    }

    #[test]
    fn rejects_missing_provenance() {
        let mut bad = term("untracked loss", 2.0, 0.1);
        bad.reference.clear();
        let result = evaluate_loss_budget(&LossBudget {
            output_power_w: 500.0,
            target_efficiency: 0.98,
            terms: vec![bad],
        });
        assert!(result.is_err());
    }

    #[test]
    fn rejects_invalid_efficiency_target() {
        for target in [0.0, 1.0, 1.01, f64::NAN] {
            let result = evaluate_loss_budget(&LossBudget {
                output_power_w: 500.0,
                target_efficiency: target,
                terms: vec![term("loss", 1.0, 0.0)],
            });
            assert!(result.is_err());
        }
    }
}
