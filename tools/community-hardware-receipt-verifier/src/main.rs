use community_hardware_receipt_verifier::verify_bytes;
use std::env;
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::process::ExitCode;

const MAX_SCHEMA_BYTES: u64 = 4 * 1024 * 1024;
const MAX_RECEIPT_BYTES: u64 = 16 * 1024 * 1024;

fn main() -> ExitCode {
    match run() {
        Ok(valid) => if valid { ExitCode::SUCCESS } else { ExitCode::FAILURE },
        Err(message) => {
            eprintln!("{message}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<bool, String> {
    let mut schema_path: Option<PathBuf> = None;
    let mut receipt_path: Option<PathBuf> = None;
    let mut artifact_root: Option<PathBuf> = None;
    let mut args = env::args_os().skip(1);

    while let Some(arg) = args.next() {
        if arg == "--help" || arg == "-h" {
            print_help();
            return Ok(true);
        }
        if arg == "--artifact-root" {
            artifact_root = Some(PathBuf::from(
                args.next().ok_or_else(|| "--artifact-root requires a directory".to_owned())?,
            ));
            continue;
        }
        if arg.to_string_lossy().starts_with('-') {
            return Err("Unknown option. Use --help for usage.".to_owned());
        }
        if schema_path.is_none() {
            schema_path = Some(PathBuf::from(arg));
        } else if receipt_path.is_none() {
            receipt_path = Some(PathBuf::from(arg));
        } else {
            return Err("Too many positional arguments. Use --help for usage.".to_owned());
        }
    }

    let schema_path = schema_path.ok_or_else(|| usage())?;
    let receipt_path = receipt_path.ok_or_else(|| usage())?;
    let schema_bytes = read_bounded(&schema_path, MAX_SCHEMA_BYTES, "schema")?;
    let receipt_bytes = read_bounded(&receipt_path, MAX_RECEIPT_BYTES, "receipt")?;
    let report = verify_bytes(&schema_bytes, &receipt_bytes, artifact_root.as_deref());

    let valid = report.valid;
    let serialized = serde_json::to_string_pretty(&report)
        .map_err(|_| "Could not serialize verification report.".to_owned())?;
    println!("{serialized}");
    Ok(valid)
}

fn read_bounded(path: &PathBuf, limit: u64, label: &str) -> Result<Vec<u8>, String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| format!("Cannot stat {label} file."))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(format!("{label} input must be a regular non-symlink file."));
    }
    if metadata.len() > limit {
        return Err(format!("{label} exceeds configured size limit."));
    }
    let file = fs::File::open(path).map_err(|_| format!("Cannot open {label} file."))?;
    let mut bytes = Vec::new();
    file.take(limit.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|_| format!("Cannot read {label} file."))?;
    if bytes.len() as u64 > limit {
        return Err(format!("{label} exceeds configured size limit."));
    }
    Ok(bytes)
}

fn usage() -> String {
    "Usage: community-hardware-receipt-verify <schema.json> <receipt.json> [--artifact-root DIR]".to_owned()
}

fn print_help() {
    println!("{}", usage());
    println!("Validates JSON Schema Draft 2020-12 and receipt semantics; never performs device actions.");
    println!("If --artifact-root is supplied, public artifacts must be regular files named by SHA-256.");
    println!("Exit codes: 0 valid receipt contract; 1 invalid receipt; 2 input or CLI error.");
    println!("A valid receipt is not a hardware qualification or deployment authorization.");
}
