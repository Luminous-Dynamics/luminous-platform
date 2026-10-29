use luminous_landlock_readonly_fixture::{
    apply_common_metadata_deny_filter, apply_read_only_directory,
};
use std::ffi::CString;
use std::fs::{self, OpenOptions};
use std::io;
use std::io::ErrorKind;
use std::os::raw::{c_char, c_int, c_void};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::{SystemTime, UNIX_EPOCH};

const FIXTURE_CONTENT: &str = "luminous-landlock-synthetic-canary-v1";

#[repr(C)]
struct Timespec {
    tv_sec: i64,
    tv_nsec: i64,
}

const AT_FDCWD: c_int = -100;
const ENODATA_ERRNO: i32 = 61;

unsafe extern "C" {
    fn setxattr(
        path: *const c_char,
        name: *const c_char,
        value: *const c_void,
        size: usize,
        flags: c_int,
    ) -> c_int;
    fn getxattr(
        path: *const c_char,
        name: *const c_char,
        value: *mut c_void,
        size: usize,
    ) -> isize;
    fn removexattr(path: *const c_char, name: *const c_char) -> c_int;
    fn utimensat(
        dirfd: c_int,
        pathname: *const c_char,
        times: *const Timespec,
        flags: c_int,
    ) -> c_int;
}

fn c_path(path: &Path) -> io::Result<CString> {
    CString::new(path.as_os_str().as_bytes())
        .map_err(|_| io::Error::new(ErrorKind::InvalidInput, "path contains NUL"))
}

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

    let fixture_c = c_path(&fixture)?;
    let xattr_name = CString::new("user.luminous_probe").expect("literal has no NUL");
    let xattr_value = b"x";
    // SAFETY: C strings and the value slice remain valid for the duration of the call.
    let set_result = unsafe {
        setxattr(
            fixture_c.as_ptr(),
            xattr_name.as_ptr(),
            xattr_value.as_ptr().cast(),
            xattr_value.len(),
            0,
        )
    };
    if set_result == -1 && io::Error::last_os_error().kind() == ErrorKind::PermissionDenied {
        println!("DENY_SETXATTR_OK");
    } else if set_result == 0 {
        return Err(io::Error::other(
            "seccomp unexpectedly allowed setting a user xattr",
        ));
    } else {
        return Err(io::Error::other(format!(
            "setxattr failed for an unexpected reason: {}",
            io::Error::last_os_error()
        )));
    }

    let requested_times = [
        Timespec {
            tv_sec: 1,
            tv_nsec: 0,
        },
        Timespec {
            tv_sec: 1,
            tv_nsec: 0,
        },
    ];
    // SAFETY: fixture_c and the two-entry times array remain valid for this call.
    let time_result = unsafe {
        utimensat(
            AT_FDCWD,
            fixture_c.as_ptr(),
            requested_times.as_ptr(),
            0,
        )
    };
    if time_result == -1 && io::Error::last_os_error().kind() == ErrorKind::PermissionDenied {
        println!("DENY_UTIMENSAT_OK");
    } else if time_result == 0 {
        return Err(io::Error::other(
            "seccomp unexpectedly allowed changing fixture timestamps",
        ));
    } else {
        return Err(io::Error::other(format!(
            "utimensat failed for an unexpected reason: {}",
            io::Error::last_os_error()
        )));
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
        // Positive controls ensure that xattr and timestamp APIs work on this
        // filesystem before the restricted child starts.
        let fixture_path = allowed.join("fixture.txt");
        let fixture_c = c_path(&fixture_path)?;
        let xattr_name = CString::new("user.luminous_probe").expect("literal has no NUL");
        let control_value = b"control";
        // SAFETY: all pointers refer to live NUL-terminated strings or byte slices.
        if unsafe {
            setxattr(
                fixture_c.as_ptr(),
                xattr_name.as_ptr(),
                control_value.as_ptr().cast(),
                control_value.len(),
                0,
            )
        } != 0 {
            return Err(io::Error::last_os_error());
        }
        let mut verify_value = [0_u8; 16];
        // SAFETY: output buffer is writable for its full declared size.
        let read_len = unsafe {
            getxattr(
                fixture_c.as_ptr(),
                xattr_name.as_ptr(),
                verify_value.as_mut_ptr().cast(),
                verify_value.len(),
            )
        };
        if read_len != control_value.len() as isize
            || &verify_value[..control_value.len()] != control_value
        {
            return Err(io::Error::new(
                ErrorKind::InvalidData,
                "xattr positive control did not round-trip",
            ));
        }
        // SAFETY: both C strings are valid and live.
        if unsafe { removexattr(fixture_c.as_ptr(), xattr_name.as_ptr()) } != 0 {
            return Err(io::Error::last_os_error());
        }
        let positive_control_times = [
            Timespec {
                tv_sec: 1,
                tv_nsec: 0,
            },
            Timespec {
                tv_sec: 1,
                tv_nsec: 0,
            },
        ];
        // SAFETY: path and the two-entry timestamps array are valid for this call.
        if unsafe {
            utimensat(
                AT_FDCWD,
                fixture_c.as_ptr(),
                positive_control_times.as_ptr(),
                0,
            )
        } != 0
        {
            return Err(io::Error::last_os_error());
        }
        let control_mtime = fs::metadata(&fixture_path)?.modified()?;
        if control_mtime
            .duration_since(UNIX_EPOCH)
            .map_err(|_| {
                io::Error::new(
                    ErrorKind::InvalidData,
                    "positive-control mtime predates epoch",
                )
            })?
            .as_secs()
            != 1
        {
            return Err(io::Error::new(
                ErrorKind::InvalidData,
                "timestamp positive control did not update mtime to the requested value",
            ));
        }
        fs::write(blocked.join("outside-canary.txt"), "forbidden-outside-canary")?;
        Ok::<(), io::Error>(())
    })();

    let result = prepare.and_then(|()| {
        // Capture ground truth before the restricted child starts, not after it.
        let fixture_path = allowed.join("fixture.txt");
        let before_metadata = fs::metadata(&fixture_path)?;
        let original_mode = before_metadata.permissions().mode() & 0o7777;
        let original_mtime = before_metadata.modified()?;

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
            "DENY_SETXATTR_OK",
            "DENY_UTIMENSAT_OK",
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

        let unchanged = fs::read_to_string(&fixture_path)?;
        if unchanged != FIXTURE_CONTENT {
            return Err(io::Error::new(
                ErrorKind::InvalidData,
                "read-only fixture changed after worker execution",
            ));
        }
        let final_mode =
            fs::metadata(allowed.join("fixture.txt"))?.permissions().mode() & 0o7777;
        if final_mode != 0o644 || final_mode != original_mode {
            return Err(io::Error::new(
                ErrorKind::PermissionDenied,
                "independent parent oracle observed an unexpected mode mutation",
            ));
        }
        if fs::metadata(&fixture_path)?.modified()? != original_mtime {
            return Err(io::Error::new(
                ErrorKind::InvalidData,
                "independent parent oracle observed an unexpected timestamp mutation",
            ));
        }
        let fixture_c = c_path(&fixture_path)?;
        let xattr_name = CString::new("user.luminous_probe").expect("literal has no NUL");
        let mut xattr_buffer = [0_u8; 16];
        // SAFETY: output buffer is writable for its full declared size.
        let xattr_result = unsafe {
            getxattr(
                fixture_c.as_ptr(),
                xattr_name.as_ptr(),
                xattr_buffer.as_mut_ptr().cast(),
                xattr_buffer.len(),
            )
        };
        if xattr_result != -1
            || io::Error::last_os_error().raw_os_error() != Some(ENODATA_ERRNO)
        {
            return Err(io::Error::new(
                ErrorKind::InvalidData,
                "independent parent oracle found a user xattr or could not verify its absence",
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
                "{{\"schema\":\"luminous.landlock-seccomp-fixture-result.v2\",",
                "\"status\":\"pass\",",
                "\"landlock_abi\":{},",
                "\"seccomp_metadata_denylist_count\":{},",
                "\"allowed_read_by_child\":true,",
                "\"outside_read_denied_by_child\":true,",
                "\"existing_write_denied_by_child\":true,",
                "\"outside_create_denied_by_child\":true,",
                "\"chmod_denied_by_seccomp\":true,",
                "\"setxattr_denied_by_seccomp\":true,",
                "\"utimensat_denied_by_seccomp\":true,",
                "\"parent_oracle_allowed_canary_unchanged\":true,",
                "\"parent_oracle_outside_canary_unchanged\":true,",
                "\"parent_oracle_forbidden_output_absent\":true,",
                "\"parent_oracle_mode_unchanged\":true,",
                "\"parent_oracle_mtime_unchanged\":true,",
                "\"parent_oracle_xattr_absent\":true,",
                "\"full_metadata_mutation_coverage\":false,",
                "\"scope\":\"read_only_file_content_plus_tested_metadata_guards\"}}"
            ),
            landlock_abi,
            metadata_denylist_count
        );
        println!(
            "SCOPE read_only_file_content_plus_tested_metadata_guards; \
             full metadata coverage and network are out of scope"
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
