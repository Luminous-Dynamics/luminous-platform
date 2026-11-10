use std::env;
use std::fs::File;
use std::io::BufReader;
use std::process::ExitCode;

use power_fabric_measurement_audit::{audit_csv, Direction};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: power-fabric-audit <capture.csv> <A_TO_B|B_TO_A>");
        return ExitCode::from(2);
    }

    let direction = match Direction::parse(&args[2]) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("INVALID: {error}");
            return ExitCode::from(2);
        }
    };

    let file = match File::open(&args[1]) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("INVALID: cannot open capture: {error}");
            return ExitCode::from(2);
        }
    };

    let report = match audit_csv(BufReader::new(file), direction) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("INVALID: {error}");
            return ExitCode::from(1);
        }
    };

    println!("status={}", report.claim_ceiling());
    println!("direction={:?}", report.direction);
    println!("sample_count={}", report.sample_count);
    println!("duration_s={:.9}", report.duration_s);
    println!("input_energy_j={:.9}", report.input_energy_j);
    println!("output_energy_j={:.9}", report.output_energy_j);
    println!("average_input_power_w={:.9}", report.average_input_power_w);
    println!("average_output_power_w={:.9}", report.average_output_power_w);
    println!("efficiency_percent={:.6}", report.efficiency_percent);
    println!("qualification=NOT_ESTABLISHED");
    println!("calibration=NOT_VERIFIED");
    println!("raw_data_digest=NOT_VERIFIED");

    ExitCode::SUCCESS
}
