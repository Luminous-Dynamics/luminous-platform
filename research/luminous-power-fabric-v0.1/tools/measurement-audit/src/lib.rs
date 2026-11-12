use std::io::BufRead;

pub mod dc_protection;
pub mod isop_sharing;
pub mod isop_input_balance;
pub mod interleaved_ripple;
pub mod dc_rail_path;
pub mod power_trace;
pub mod transient_audit;
pub mod storage_buffer;
pub mod compute_power_benchmark;
pub mod pdn_frequency;
pub mod sampling_adequacy;
pub mod loss_budget;
pub mod energy_chain;
pub mod dab_sps;
pub mod dab_sps_physics;
pub mod dab_sps_resistive;
pub mod dab_mission_sweep;
pub mod cllc_fha;
pub mod cllc_t_network;
pub mod cllc_design;

const EXPECTED_HEADER: &str =
    "time_s,port_a_voltage_v,port_a_current_a,port_b_voltage_v,port_b_current_a,case_temp_c";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    AToB,
    BToA,
}

impl Direction {
    pub fn parse(value: &str) -> Result<Self, AuditError> {
        match value {
            "A_TO_B" => Ok(Self::AToB),
            "B_TO_A" => Ok(Self::BToA),
            other => Err(AuditError(format!(
                "unsupported direction {other:?}; expected A_TO_B or B_TO_A"
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AuditReport {
    pub direction: Direction,
    pub sample_count: usize,
    pub duration_s: f64,
    pub input_energy_j: f64,
    pub output_energy_j: f64,
    pub average_input_power_w: f64,
    pub average_output_power_w: f64,
    pub efficiency_percent: f64,
}

impl AuditReport {
    /// This report only confirms the CSV and integrated energy checks below.
    /// It is not a device qualification, safety assessment, or performance pass.
    pub const fn claim_ceiling(&self) -> &'static str {
        "PARSE_AND_ENERGY_CHECKS_ONLY_NOT_QUALIFICATION"
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditError(pub String);

impl std::fmt::Display for AuditError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for AuditError {}

#[derive(Debug, Clone, Copy)]
struct Sample {
    time_s: f64,
    port_a_power_w: f64,
    port_b_power_w: f64,
}

fn parse_number(raw: &str, line_number: usize, field_name: &str) -> Result<f64, AuditError> {
    let value = raw.parse::<f64>().map_err(|_| {
        AuditError(format!(
            "line {line_number}: {field_name} is not a plain numeric field"
        ))
    })?;

    if !value.is_finite() {
        return Err(AuditError(format!(
            "line {line_number}: {field_name} must be finite"
        )));
    }

    Ok(value)
}

/// Audit a six-column CSV capture using the v0.1 sign convention:
/// both port currents are positive into the converter.
///
/// A_TO_B: port A supplies net energy; port B receives net energy.
/// B_TO_A: port B supplies net energy; port A receives net energy.
///
/// The function does not read or authenticate a manifest, verify file digests,
/// assess calibration, remove outliers, infer missing samples, or qualify hardware.
pub fn audit_csv<R: BufRead>(
    mut reader: R,
    direction: Direction,
) -> Result<AuditReport, AuditError> {
    let mut header = String::new();
    let header_bytes = reader
        .read_line(&mut header)
        .map_err(|e| AuditError(format!("cannot read CSV header: {e}")))?;

    if header_bytes == 0 {
        return Err(AuditError("capture is empty".to_string()));
    }

    let header = header.trim_end_matches(|c| c == '\r' || c == '\n');
    if header != EXPECTED_HEADER {
        return Err(AuditError(format!(
            "unexpected CSV header; expected exactly {EXPECTED_HEADER:?}"
        )));
    }

    let mut previous: Option<Sample> = None;
    let mut input_energy_j = 0.0_f64;
    let mut output_energy_j = 0.0_f64;
    let mut sample_count = 0_usize;
    let mut first_time_s: Option<f64> = None;
    let mut last_time_s: Option<f64> = None;
    let mut line_number = 1_usize;

    let mut line = String::new();
    loop {
        line.clear();
        let bytes = reader
            .read_line(&mut line)
            .map_err(|e| AuditError(format!("line {}: read failure: {e}", line_number + 1)))?;
        if bytes == 0 {
            break;
        }

        line_number += 1;
        let row = line.trim_end_matches(|c| c == '\r' || c == '\n');
        if row.is_empty() {
            return Err(AuditError(format!("line {line_number}: blank rows are invalid")));
        }

        let fields: Vec<&str> = row.split(',').collect();
        if fields.len() != 6 {
            return Err(AuditError(format!(
                "line {line_number}: expected 6 comma-separated fields, found {}",
                fields.len()
            )));
        }

        let time_s = parse_number(fields[0], line_number, "time_s")?;
        let port_a_voltage_v = parse_number(fields[1], line_number, "port_a_voltage_v")?;
        let port_a_current_a = parse_number(fields[2], line_number, "port_a_current_a")?;
        let port_b_voltage_v = parse_number(fields[3], line_number, "port_b_voltage_v")?;
        let port_b_current_a = parse_number(fields[4], line_number, "port_b_current_a")?;
        // Temperature is checked even though it is not used in the energy integral.
        let _case_temp_c = parse_number(fields[5], line_number, "case_temp_c")?;

        if port_a_voltage_v <= 0.0 || port_b_voltage_v <= 0.0 {
            return Err(AuditError(format!(
                "line {line_number}: port voltages must be positive; polarity is represented by current sign"
            )));
        }

        let sample = Sample {
            time_s,
            port_a_power_w: port_a_voltage_v * port_a_current_a,
            port_b_power_w: port_b_voltage_v * port_b_current_a,
        };

        if first_time_s.is_none() {
            first_time_s = Some(time_s);
        }
        last_time_s = Some(time_s);

        if let Some(prev) = previous {
            let dt = sample.time_s - prev.time_s;
            if dt <= 0.0 {
                return Err(AuditError(format!(
                    "line {line_number}: timestamps must be strictly increasing"
                )));
            }

            let delta_a = (prev.port_a_power_w + sample.port_a_power_w) * 0.5 * dt;
            let delta_b = (prev.port_b_power_w + sample.port_b_power_w) * 0.5 * dt;

            match direction {
                Direction::AToB => {
                    input_energy_j += delta_a;
                    output_energy_j -= delta_b;
                }
                Direction::BToA => {
                    input_energy_j += delta_b;
                    output_energy_j -= delta_a;
                }
            }
        }

        previous = Some(sample);
        sample_count += 1;
    }

    if sample_count < 2 {
        return Err(AuditError(
            "at least two samples are required to integrate energy".to_string(),
        ));
    }

    let duration_s = last_time_s
        .zip(first_time_s)
        .map(|(last, first)| last - first)
        .ok_or_else(|| AuditError("capture duration is unavailable".to_string()))?;

    if !duration_s.is_finite() || duration_s <= 0.0 {
        return Err(AuditError("capture duration must be positive and finite".to_string()));
    }

    // The energy integral has a defined sign only when each port has a positive
    // net contribution in its declared role.
    if !input_energy_j.is_finite() || input_energy_j <= 0.0 {
        return Err(AuditError(
            "integrated input energy is non-positive or non-finite for the declared direction"
                .to_string(),
        ));
    }
    if !output_energy_j.is_finite() || output_energy_j <= 0.0 {
        return Err(AuditError(
            "integrated output energy is non-positive or non-finite for the declared direction"
                .to_string(),
        ));
    }

    let efficiency_percent = 100.0 * output_energy_j / input_energy_j;
    if !efficiency_percent.is_finite() || efficiency_percent <= 0.0 {
        return Err(AuditError(
            "efficiency ratio is non-positive or non-finite".to_string(),
        ));
    }
    if efficiency_percent > 100.0 {
        return Err(AuditError(format!(
            "integrated output energy exceeds input energy ({efficiency_percent:.6}%); check sign convention, channels, timing and measurement setup"
        )));
    }

    Ok(AuditReport {
        direction,
        sample_count,
        duration_s,
        input_energy_j,
        output_energy_j,
        average_input_power_w: input_energy_j / duration_s,
        average_output_power_w: output_energy_j / duration_s,
        efficiency_percent,
    })
}

#[cfg(test)]
mod tests {
    use super::{audit_csv, Direction};

    const HEADER: &str =
        "time_s,port_a_voltage_v,port_a_current_a,port_b_voltage_v,port_b_current_a,case_temp_c\n";

    #[test]
    fn audits_forward_energy_flow() {
        let capture = format!("{HEADER}0,48,10,48,-9.6,30\n1,48,10,48,-9.6,30\n");
        let report = audit_csv(capture.as_bytes(), Direction::AToB).unwrap();
        assert_eq!(report.sample_count, 2);
        assert!((report.duration_s - 1.0).abs() < 1e-9);
        assert!((report.average_input_power_w - 480.0).abs() < 1e-9);
        assert!((report.average_output_power_w - 460.8).abs() < 1e-9);
        assert!((report.input_energy_j - 480.0).abs() < 1e-9);
        assert!((report.output_energy_j - 460.8).abs() < 1e-9);
        assert!((report.efficiency_percent - 96.0).abs() < 1e-9);
        assert_eq!(
            report.claim_ceiling(),
            "PARSE_AND_ENERGY_CHECKS_ONLY_NOT_QUALIFICATION"
        );
    }

    #[test]
    fn audits_reverse_energy_flow() {
        let capture = format!("{HEADER}0,48,-9.6,48,10,30\n1,48,-9.6,48,10,30\n");
        let report = audit_csv(capture.as_bytes(), Direction::BToA).unwrap();
        assert!((report.efficiency_percent - 96.0).abs() < 1e-9);
    }

    #[test]
    fn rejects_wrong_header() {
        let capture = "time,va,ia,vb,ib,temp\n0,48,10,48,-9.6,30\n1,48,10,48,-9.6,30\n";
        assert!(audit_csv(capture.as_bytes(), Direction::AToB).is_err());
    }

    #[test]
    fn rejects_non_finite_measurement() {
        let capture = format!("{HEADER}0,48,NaN,48,-9.6,30\n1,48,10,48,-9.6,30\n");
        assert!(audit_csv(capture.as_bytes(), Direction::AToB).is_err());
    }

    #[test]
    fn rejects_non_monotonic_timestamps() {
        let capture = format!("{HEADER}1,48,10,48,-9.6,30\n1,48,10,48,-9.6,30\n");
        assert!(audit_csv(capture.as_bytes(), Direction::AToB).is_err());
    }

    #[test]
    fn rejects_wrong_energy_direction() {
        let capture = format!("{HEADER}0,48,-10,48,9.6,30\n1,48,-10,48,9.6,30\n");
        assert!(audit_csv(capture.as_bytes(), Direction::AToB).is_err());
    }

    #[test]
    fn rejects_apparent_efficiency_above_100_percent() {
        let capture = format!("{HEADER}0,48,10,48,-10.1,30\n1,48,10,48,-10.1,30\n");
        assert!(audit_csv(capture.as_bytes(), Direction::AToB).is_err());
    }

    #[test]
    fn rejects_single_sample() {
        let capture = format!("{HEADER}0,48,10,48,-9.6,30\n");
        assert!(audit_csv(capture.as_bytes(), Direction::AToB).is_err());
    }
}
