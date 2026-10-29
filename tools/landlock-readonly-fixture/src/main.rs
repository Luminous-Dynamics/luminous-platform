use luminous_landlock_readonly_fixture::apply_read_only_directory;
use std::fs::{self, OpenOptions};
use std::io;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::{SystemTime, UNIX_EPOCH};

const FIXTURE_CONTENT: &str = "luminous-landlock-synthetic-canary-v1";

fn worker(allowed_root: &Path, denied_write_target: &Path) -> io::Result<()> {
    // Install the restriction before opening the fixture or attempting effects.
    apply_read_only_directory(allowed_root)?;

    let fixture = allowed_root.join("fixture.txt");
    let contents = fs::read_to_string(&fixture)?;
    if contents != FIXTURE_CONTENT {
        return Err(io::Error::new(
            ErrorKind::InvalidData,
            "fixture contents did not match the expected synthetic canary",
        ));
    }
    println!("ALLOW_READ_OK");

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
        Ok::<(), io::Error>(())
    })();

    let result = prepare.and_then(|()| {
        let executable = std::env::current_exe()?;
        let output = Command::new(executable)
            .arg("--worker")
            .arg(&allowed)
            .arg(blocked.join("must-not-exist.txt"))
            .output()?;

        if !output.status.success() {
            eprintln!("Landlock worker failed; stdout={:?}", String::from_utf8_lossy(&output.stdout));
            eprintln!("Landlock worker stderr={:?}", String::from_utf8_lossy(&output.stderr));
            return Err(io::Error::new(
                ErrorKind::Other,
                format!("Landlock worker exited with {}", output.status),
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        for marker in ["ALLOW_READ_OK", "DENY_WRITE_EXISTING_OK", "DENY_CREATE_OUTSIDE_OK"] {
            if !stdout.lines().any(|line| line == marker) {
                return Err(io::Error::new(
                    ErrorKind::InvalidData,
                    format!("expected worker evidence marker {marker} was absent"),
                ));
            }
        }

        let unchanged = fs::read_to_string(allowed.join("fixture.txt"))?;
        if unchanged != FIXTURE_CONTENT {
            return Err(io::Error::new(
                ErrorKind::InvalidData,
                "read-only fixture changed after worker execution",
            ));
        }
        if blocked.join("must-not-exist.txt").exists() {
            return Err(io::Error::new(
                ErrorKind::PermissionDenied,
                "independent parent oracle observed the forbidden output file",
            ));
        }

        println!("PASS Landlock ABI >= 3 read-only fixture enforcement");
        println!("ORACLE read_allowed=true existing_write_denied=true outside_create_denied=true");
        println!("SCOPE disposable_local_filesystem_fixture_only; no network or syscall-wide claim");
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
        if args.len() != 4 {
            Err(io::Error::new(ErrorKind::InvalidInput, "worker requires allowed-root and denied-target paths"))
        } else {
            worker(Path::new(&args[2]), Path::new(&args[3]))
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
