use luminous_landlock_readonly_fixture::{
    apply_common_metadata_deny_filter, apply_read_only_directory,
};
use std::fs::{self, OpenOptions};
use std::io;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::os::unix::fs::PermissionsExt;
use std::process::{Command, ExitCode};
use std::time::{SystemTime, UNIX_EPOCH};

const FIXTURE_CONTENT: &str = "luminous-landlock-synthetic-canary-v1";

fn worker(
    allowed_root: &Path,
    denied_read_target: &Path,
    denied_write_target: &Path,
) -> io::Result<()> {
    // Install the restriction before opening the fixture or attempting effects.
    let landlock_abi = apply_read_only_directory(allowed_root)?;
    println!("LANDLOCK_ABI={landlock_abi}");
    let metadata_denylist_count = apply_common_metadata_deny_filter()?;
    println!("SECCOMP_METADATA_FILTER_COUNT={metadata_denylist_count}");

    let fixture = allowed_root.join("fixture.txt");
    let contents = fs::read_to_string(&fixture)?;
    if contents != FIXTURE_CONTENT {
        return Err(io::Error::new(
            ErrorKind::InvalidData,
            "fixture contents did not match the expected synthetic canary",
        ));
    }
    println!("ALLOW_READ_OK");

    match fs::read_to_string(denied_read_target) {
        Err(error) if error.kind() == ErrorKind::PermissionDenied => {
            println!("DENY_READ_OUTSIDE_OK");
        }
        Err(error) => return Err(error),
        Ok(_) => {
            return Err(io::Error::other(
                "Landlock unexpectedly allowed reading outside the allowed subtree",
            ));
        }
    }

    match OpenOptions::new().write(true).open(&fixture) {
        Err(error) if error.kind() == ErrorKind::PermissionDenied => {
            println!("DENY_WRITE_EXISTING_OK");
        }
        Err(error) => return Err(error),
        Ok(file) => {
            drop(file);
            return Err(io::Error::other(
                "Landlock unexpectedly allowed opening the read-only fixture for writing",
            ));
        }
    }

    match OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(denied_write_target)
    {
        Err(error) if error.kind() == ErrorKind::PermissionDenied => {
            println!("DENY_CREATE_OUTSIDE_OK");
        }
        Err(error) => return Err(error),
        Ok(file) => {
            drop(file);
            return Err(io::Error::other(
                "Landlock unexpectedly allowed creating a file outside the allowed subtree",
            ));
        }
    }

    match fs::set_permissions(&fixture, fs::Permissions::from_mode(0o600)) {
        Err(error) if error.kind() == ErrorKind::PermissionDenied => {
            println!("DENY_CHMOD_OK");
        }
        Err(error) => return Err(error),
        Ok(()) => {
            return Err(io::Error::other(
                "seccomp unexpectedly allowed changing fixture permissions",
            ));
        }
    }

    Ok(())
}

fn unique_temp_root() -> io::Result<PathBuf> {
    let base = std::env::temp_dir();
    let clock = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();

    for suffix in 0..100u32 {
        let candidate = base.join(format!(
            "luminous-landlock-fixture-{}-{clock}-{suffix}",
            std::process::id()
        ));
        match fs::create_dir(&candidate) {
            Ok(()) => return Ok(candidate),
            Err(error) if error.kind() == ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        ErrorKind::AlreadyExists,
        "could not allocate a unique disposable fixture directory",
    ))
}

fn run_parent() -> io::Result<()> {
    let root = unique_temp_root()?;
    let allowed = root.join("allowed-read-only");
    let blocked = root.join("blocked-write-target");
    let prepare = (|| {
        fs::create_dir(&allowed)?;
        fs::create_dir(&blocked)?;
        fs::write(allowed.join("fixture.txt"), FIXTURE_CONTENT)?;
        fs::set_permissions(
            allowed.join("fixture.txt"),
            fs::Permissions::from_mode(0o644),
        )?;
        fs::write(blocked.join("outside-canary.txt"), "forbidden-outside-canary")?;
        Ok::<(), io::Error>(())
    })();

    let result = prepare.and_then(|()| {
        let executable = std::env::current_exe()?;
        let output = Command::new(executable)
            .arg("--worker")
            .arg(&allowed)
            .arg(blocked.join("outside-canary.txt"))
            .arg(blocked.join("must-not-exist.txt"))
            .output()?;

        if !output.status.success() {
            eprintln!(
                "Landlock worker failed; stdout={:?}",
                String::from_utf8_lossy(&output.stdout)
            );
            eprintln!(
                "Landlock worker stderr={:?}",
                String::from_utf8_lossy(&output.stderr)
            );
            return Err(io::Error::other(format!(
                "Landlock worker exited with {}",
                output.status
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        for marker in [
            "ALLOW_READ_OK",
            "DENY_READ_OUTSIDE_OK",
            "DENY_WRITE_EXISTING_OK",
            "DENY_CREATE_OUTSIDE_OK",
            "DENY_CHMOD_OK",
        ] {
            if !stdout.lines().any(|line| line == marker) {
                return Err(io::Error::new(
                    ErrorKind::InvalidData,
                    format!("expected worker evidence marker {marker} was absent"),
                ));
            }
        }
        let landlock_abi = stdout
            .lines()
            .find_map(|line| line.strip_prefix("LANDLOCK_ABI="))
            .and_then(|value| value.parse::<i32>().ok())
            .filter(|abi| *abi >= 3)
            .ok_or_else(|| {
                io::Error::new(
                    ErrorKind::InvalidData,
                    "worker did not report a supported Landlock ABI version",
                )
            })?;
        let metadata_denylist_count = stdout
            .lines()
            .find_map(|line| line.strip_prefix("SECCOMP_METADATA_FILTER_COUNT="))
            .and_then(|value| value.parse::<usize>().ok())
            .filter(|count| *count >= 10)
            .ok_or_else(|| {
                io::Error::new(
                    ErrorKind::InvalidData,
                    "worker did not report the supported seccomp metadata denylist",
                )
            })?;

        let unchanged = fs::read_to_string(allowed.join("fixture.txt"))?;
        if unchanged != FIXTURE_CONTENT {
            return Err(io::Error::new(
                ErrorKind::InvalidData,
                "read-only fixture changed after worker execution",
            ));
        }
        let final_mode =
            fs::metadata(allowed.join("fixture.txt"))?.permissions().mode() & 0o7777;
        if final_mode != 0o644 {
            return Err(io::Error::new(
                ErrorKind::PermissionDenied,
                "independent parent oracle observed an unexpected mode mutation",
            ));
        }
        if blocked.join("outside-canary.txt").exists() {
            let outside = fs::read_to_string(blocked.join("outside-canary.txt"))?;
            if outside != "forbidden-outside-canary" {
                return Err(io::Error::new(
                    ErrorKind::InvalidData,
                    "independent parent oracle observed unexpected outside-canary mutation",
                ));
            }
        } else {
            return Err(io::Error::new(
                ErrorKind::NotFound,
                "independent parent oracle could not find the external read-canary",
            ));
        }
        if blocked.join("must-not-exist.txt").exists() {
            return Err(io::Error::new(
                ErrorKind::PermissionDenied,
                "independent parent oracle observed the forbidden output file",
            ));
        }

        println!(
            concat!(
                "{{\"schema\":\"luminous.landlock-seccomp-fixture-result.v1\",",
                "\"status\":\"pass\",",
                "\"landlock_abi\":{},",
                "\"seccomp_metadata_denylist_count\":{},",
                "\"allowed_read_by_child\":true,",
                "\"outside_read_denied_by_child\":true,",
                "\"existing_write_denied_by_child\":true,",
                "\"outside_create_denied_by_child\":true,",
                "\"chmod_denied_by_seccomp\":true,",
                "\"parent_oracle_allowed_canary_unchanged\":true,",
                "\"parent_oracle_outside_canary_unchanged\":true,",
                "\"parent_oracle_forbidden_output_absent\":true,",
                "\"parent_oracle_mode_unchanged\":true,",
                "\"full_metadata_mutation_coverage\":false,",
                "\"scope\":\"read_only_file_content_plus_tested_chmod_guard\"}}"
            ),
            landlock_abi,
            metadata_denylist_count
        );
        println!(
            "SCOPE read_only_file_content_plus_tested_chmod_guard; full metadata coverage and network are out of scope"
        );
        Ok(())
    });

    let cleanup = fs::remove_dir_all(&root);
    match (result, cleanup) {
        (Err(error), _) => Err(error),
        (Ok(()), Err(error)) => Err(error),
        (Ok(()), Ok(())) => Ok(()),
    }
}

fn main() -> ExitCode {
    let args: Vec<_> = std::env::args_os().collect();
    let result = if args.get(1).and_then(|value| value.to_str()) == Some("--worker") {
        if args.len() != 5 {
            Err(io::Error::new(
                ErrorKind::InvalidInput,
                "worker requires allowed-root, outside-read-canary, and outside-write-target paths",
            ))
        } else {
            worker(
                Path::new(&args[2]),
                Path::new(&args[3]),
                Path::new(&args[4]),
            )
        }
    } else {
        run_parent()
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("FAIL Landlock fixture: {error}");
            ExitCode::FAILURE
        }
    }
}
