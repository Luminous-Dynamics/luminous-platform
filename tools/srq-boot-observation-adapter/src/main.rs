use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use srq_boot_observation_adapter::{Observation, BLOCKED};

fn usage() {
    eprintln!("Usage: srq-boot-observation [--root PATH]");
    eprintln!("Default PATH is /; --root exists for isolated, untrusted fixtures.");
    eprintln!("Output is D0 local diagnostics, not signed or trusted attestation.");
}

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let mut root = PathBuf::from("/");
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--root" => {
                let Some(value) = args.next() else {
                    usage();
                    return ExitCode::from(2);
                };
                root = PathBuf::from(value);
            }
            "--help" | "-h" => {
                usage();
                return ExitCode::SUCCESS;
            }
            _ => {
                usage();
                return ExitCode::from(2);
            }
        }
    }

    let observation = Observation::collect(&root);
    println!("{}", observation.to_json());
    if observation.status == BLOCKED {
        ExitCode::from(3)
    } else {
        // This means only that the two locally observed NixOS path identities
        // agree. It is explicitly not an authorization or release PASS.
        ExitCode::SUCCESS
    }
}
