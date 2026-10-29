#![deny(unsafe_op_in_unsafe_fn)]

#[cfg(not(target_os = "linux"))]
compile_error!("luminous-landlock-preopened-fd-probe is Linux-only");

use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{self, ErrorKind, Read, Seek, SeekFrom, Write};
use std::os::fd::{AsRawFd, RawFd};
use std::os::raw::{c_int, c_long};
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{self, Command};
use std::time::{SystemTime, UNIX_EPOCH};

const BASELINE: &[u8] = b"SAFE-CANARY-0000000000000000\n";
const MUTATED: &[u8] = b"EVIL-CANARY-0000000000000000\n";

const SYS_LANDLOCK_CREATE_RULESET: c_long = 444;
const SYS_LANDLOCK_RESTRICT_SELF: c_long = 446;
const LANDLOCK_CREATE_RULESET_VERSION: c_int = 1;
const ACCESS_FS_WRITE_FILE: u64 = 1 << 1;
const ACCESS_FS_TRUNCATE: u64 = 1 << 14;

const PR_SET_NO_NEW_PRIVS: c_int = 38;
const F_DUPFD: c_int = 0;
const F_GETFD: c_int = 1;
const F_SETFD: c_int = 2;
const FD_CLOEXEC: c_int = 1;
const EBADF_ERRNO: i32 = 9;

unsafe extern "C" {
    fn syscall(number: c_long, ...) -> c_long;
    fn prctl(
        option: c_int,
        arg2: c_long,
        arg3: c_long,
        arg4: c_long,
        arg5: c_long,
    ) -> c_int;
    fn fcntl(fd: c_int, command: c_int, ...) -> c_int;
    fn pwrite(fd: c_int, buffer: *const u8, count: usize, offset: i64) -> isize;
    fn close(fd: c_int) -> c_int;
}

#[repr(C)]
struct RulesetAttr {
    handled_access_fs: u64,
}

struct OwnedFd(RawFd);

impl OwnedFd {
    fn new(fd: RawFd) -> io::Result<Self> {
        if fd < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(Self(fd))
        }
    }

    fn raw(&self) -> RawFd {
        self.0
    }
}

impl Drop for OwnedFd {
    fn drop(&mut self) {
        // SAFETY: this wrapper owns the descriptor and closes it once.
        unsafe {
            let _ = close(self.0);
        }
    }
}

fn duplicate_descriptor(source: RawFd, cloexec: bool) -> io::Result<OwnedFd> {
    // F_DUPFD creates a distinct descriptor without FD_CLOEXEC.
    // SAFETY: source is an open descriptor and the minimum is a valid integer.
    let fd = unsafe { fcntl(source, F_DUPFD, 512) };
    let owned = OwnedFd::new(fd)?;
    if cloexec {
        // SAFETY: the descriptor is owned by this value; F_SETFD sets descriptor flags.
        if unsafe { fcntl(owned.raw(), F_SETFD, FD_CLOEXEC) } != 0 {
            return Err(io::Error::last_os_error());
        }
    }

    // SAFETY: F_GETFD inspects the descriptor owned above.
    let flags = unsafe { fcntl(owned.raw(), F_GETFD) };
    if flags < 0 {
        return Err(io::Error::last_os_error());
    }
    if (flags & FD_CLOEXEC != 0) != cloexec {
        return Err(io::Error::new(
            ErrorKind::InvalidData,
            "descriptor close-on-exec flag does not match requested state",
        ));
    }
    Ok(owned)
}

fn apply_deny_write_landlock() -> io::Result<i32> {
    // Query the ABI first. An unsupported syscall or insufficient ABI is failure,
    // not an implicit pass with a weaker policy.
    // SAFETY: Landlock VERSION query takes a null attr, size zero, and VERSION flag.
    let abi = unsafe {
        syscall(
            SYS_LANDLOCK_CREATE_RULESET,
            std::ptr::null::<RulesetAttr>(),
            0usize,
            LANDLOCK_CREATE_RULESET_VERSION as c_long,
        )
    };
    if abi < 0 {
        return Err(io::Error::last_os_error());
    }
    if abi < 3 {
        return Err(io::Error::new(
            ErrorKind::Unsupported,
            format!("Landlock ABI {abi} is below required ABI 3"),
        ));
    }

    // Handle write/truncate without adding any path grants.
    let attr = RulesetAttr {
        handled_access_fs: ACCESS_FS_WRITE_FILE | ACCESS_FS_TRUNCATE,
    };
    // SAFETY: attr is a live repr(C) struct and size matches the struct.
    let ruleset_fd = unsafe {
        syscall(
            SYS_LANDLOCK_CREATE_RULESET,
            &attr as *const RulesetAttr,
            std::mem::size_of::<RulesetAttr>(),
            0 as c_long,
        )
    };
    let ruleset = OwnedFd::new(ruleset_fd as RawFd)?;

    // SAFETY: PR_SET_NO_NEW_PRIVS takes 1 and zero-valued trailing arguments.
    if unsafe { prctl(PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) } != 0 {
        return Err(io::Error::last_os_error());
    }

    // SAFETY: ruleset is a live fd produced by Landlock; flags are zero.
    let restricted = unsafe {
        syscall(
            SYS_LANDLOCK_RESTRICT_SELF,
            ruleset.raw() as c_long,
            0 as c_long,
        )
    };
    if restricted < 0 {
        return Err(io::Error::last_os_error());
    }
    drop(ruleset);
    Ok(abi as i32)
}

fn worker(mode: &str, fd: RawFd, path: &Path) -> io::Result<()> {
    // Landlock is applied only after this worker has crossed exec. In the
    // negative control, the write descriptor was opened by the parent before
    // this restriction; in the safe control, CLOEXEC should remove that fd.
    let abi = apply_deny_write_landlock()?;

    match mode {
        "expect-preopened-write" => {
            // SAFETY: F_GETFD checks whether the supplied descriptor survived exec.
            let descriptor_flags = unsafe { fcntl(fd, F_GETFD) };
            if descriptor_flags < 0 || descriptor_flags & FD_CLOEXEC != 0 {
                return Err(io::Error::new(
                    ErrorKind::NotFound,
                    format!(
                        "expected inherited writable descriptor {fd}; flags={descriptor_flags}, errno={:?}",
                        io::Error::last_os_error().raw_os_error()
                    ),
                ));
            }

            // Deliberately test the risky ordering: the O_RDWR descriptor exists
            // before Landlock is installed and remains available after restriction.
            // SAFETY: MUTATED is a live readable buffer and the offset is zero.
            let written = unsafe { pwrite(fd, MUTATED.as_ptr(), MUTATED.len(), 0) };
            if written != MUTATED.len() as isize {
                return Err(if written < 0 {
                    io::Error::last_os_error()
                } else {
                    io::Error::new(
                        ErrorKind::WriteZero,
                        format!("short pwrite: {written} of {} bytes", MUTATED.len()),
                    )
                });
            }
            println!("EXPECTED_EXPOSURE_PASS preopened_write_after_landlock=true");
            println!("LANDLOCK_ABI={abi}");
            Ok(())
        }
        "expect-closed-and-denied" => {
            // If close-on-exec works, the high descriptor is absent and a new
            // write-open by pathname must also be denied by the Landlock policy.
            // SAFETY: F_GETFD checks whether the descriptor survived exec.
            let descriptor_flags = unsafe { fcntl(fd, F_GETFD) };
            if descriptor_flags != -1
                || io::Error::last_os_error().raw_os_error() != Some(EBADF_ERRNO)
            {
                return Err(io::Error::new(
                    ErrorKind::PermissionDenied,
                    format!(
                        "descriptor {fd} remained open or absence was inconclusive (flags={descriptor_flags}, errno={:?})",
                        io::Error::last_os_error().raw_os_error()
                    ),
                ));
            }

            match OpenOptions::new().write(true).open(path) {
                Err(error) if error.kind() == ErrorKind::PermissionDenied => {}
                Err(error) => {
                    return Err(io::Error::new(
                        error.kind(),
                        format!("write-open failed for an unexpected reason: {error}"),
                    ));
                }
                Ok(_file) => {
                    return Err(io::Error::new(
                        ErrorKind::PermissionDenied,
                        "Landlock unexpectedly allowed a new write-open",
                    ));
                }
            }
            println!("SAFE_CONTROL_PASS inherited_fd_absent=true path_write_denied=true");
            println!("LANDLOCK_ABI={abi}");
            Ok(())
        }
        _ => Err(io::Error::new(
            ErrorKind::InvalidInput,
            "unknown worker mode",
        )),
    }
}

fn run_worker(executable: &Path, mode: &str, fd: RawFd, path: &Path) -> io::Result<()> {
    let output = Command::new(executable)
        .arg("--worker")
        .arg(mode)
        .arg(fd.to_string())
        .arg(path)
        .output()?;

    if !output.status.success() {
        return Err(io::Error::other(format!(
            "worker {mode} failed ({}); stdout={:?}; stderr={:?}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let expected = match mode {
        "expect-preopened-write" => "EXPECTED_EXPOSURE_PASS preopened_write_after_landlock=true",
        "expect-closed-and-denied" => {
            "SAFE_CONTROL_PASS inherited_fd_absent=true path_write_denied=true"
        }
        _ => {
            return Err(io::Error::new(
                ErrorKind::InvalidInput,
                "unknown worker mode",
            ));
        }
    };
    if !stdout.lines().any(|line| line == expected) {
        return Err(io::Error::new(
            ErrorKind::InvalidData,
            format!("worker {mode} omitted expected marker {expected}"),
        ));
    }
    let abi = stdout
        .lines()
        .find_map(|line| line.strip_prefix("LANDLOCK_ABI="))
        .and_then(|value| value.parse::<i32>().ok())
        .filter(|value| *value >= 3)
        .ok_or_else(|| io::Error::new(ErrorKind::InvalidData, "missing supported Landlock ABI"))?;
    Ok(())
}

fn read_exact_file(path: &Path) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    File::open(path)?.read_to_end(&mut bytes)?;
    Ok(bytes)
}

fn run_parent() -> io::Result<()> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| io::Error::other(format!("system clock before Unix epoch: {error}")))?
        .as_nanos();
    let root: PathBuf = env::temp_dir().join(format!(
        "luminous-landlock-preopened-fd-{}-{timestamp}",
        process::id()
    ));
    let mut directory = fs::DirBuilder::new();
    directory.mode(0o700);
    directory.create(&root)?;
    let path = root.join("victim.txt");

    let result = (|| {
        if BASELINE.len() != MUTATED.len() {
            return Err(io::Error::new(
                ErrorKind::InvalidData,
                "negative-control marker must preserve the baseline file length",
            ));
        }
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&path)?;
        file.write_all(BASELINE)?;
        file.sync_all()?;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;

        // Negative-control experiment: opening the writable object precedes
        // Landlock. The child should be able to mutate through that descriptor.
        let inherited = duplicate_descriptor(file.as_raw_fd(), false)?;
        run_worker(
            &env::current_exe()?,
            "expect-preopened-write",
            inherited.raw(),
            &path,
        )?;
        if read_exact_file(&path)? != MUTATED {
            return Err(io::Error::new(
                ErrorKind::InvalidData,
                "parent oracle did not observe the expected mutation through pre-opened fd",
            ));
        }
        drop(inherited);

        // Reset outside the restricted child. This is a separate safe control.
        file.set_len(0)?;
        file.seek(SeekFrom::Start(0))?;
        file.write_all(BASELINE)?;
        file.sync_all()?;

        // Mitigation control: the same kind of writable descriptor is marked
        // close-on-exec before launch; new pathname write-open must be denied.
        let protected_fd = duplicate_descriptor(file.as_raw_fd(), true)?;
        run_worker(
            &env::current_exe()?,
            "expect-closed-and-denied",
            protected_fd.raw(),
            &path,
        )?;

        // Parent independently checks the safe control's resulting file bytes.
        if read_exact_file(&path)? != BASELINE {
            return Err(io::Error::new(
                ErrorKind::InvalidData,
                "parent oracle found mutation after descriptor closure/path denial",
            ));
        }
        // The parent descriptor remains open; CLOEXEC only blocks inheritance.
        // SAFETY: protected_fd is still owned by the parent process.
        let parent_flags = unsafe { fcntl(protected_fd.raw(), F_GETFD) };
        if parent_flags < 0 || parent_flags & FD_CLOEXEC == 0 {
            return Err(io::Error::new(
                ErrorKind::InvalidData,
                "parent oracle did not observe FD_CLOEXEC on protected descriptor",
            ));
        }
        Ok(())
    })();

    let file_cleanup = fs::remove_file(&path);
    let directory_cleanup = fs::remove_dir(&root);
    match (result, file_cleanup, directory_cleanup) {
        (Err(error), _, _) => Err(error),
        (Ok(()), Err(error), _) => Err(error),
        (Ok(()), Ok(()), Err(error)) => Err(error),
        (Ok(()), Ok(()), Ok(())) => Ok(()),
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.get(1).map(String::as_str) == Some("--worker") {
        let worker_result = if args.len() == 5 {
            match args[3].parse::<RawFd>() {
                Ok(fd) => worker(&args[2], fd, Path::new(&args[4])),
                Err(error) => Err(io::Error::new(
                    ErrorKind::InvalidInput,
                    format!("invalid descriptor argument: {error}"),
                )),
            }
        } else {
            Err(io::Error::new(
                ErrorKind::InvalidInput,
                "worker requires a mode, descriptor number, and file path",
            ))
        };

        if let Err(error) = worker_result {
            eprintln!("FAIL Landlock pre-opened-fd worker: {error}");
            process::exit(2);
        }
        return;
    }

    if let Err(error) = run_parent() {
        eprintln!("FAIL Landlock pre-opened-fd probe: {error}");
        process::exit(1);
    }

    println!(
        concat!(
            "{\"schema\":\"luminous.landlock-preopened-fd-result.v1\",",
            "\"status\":\"pass\",",
            "\"preopened_write_succeeded_after_landlock\":true,",
            "\"parent_oracle_observed_preopened_write\":true,",
            "\"cloexec_descriptor_absent_after_exec\":true,",
            "\"new_path_write_open_denied_after_landlock\":true,",
            "\"parent_oracle_safe_control_unchanged\":true,",
            "\"full_descriptor_hygiene_coverage\":false,",
            "\"scope\":\"preopened_writable_fd_ordering_microtest\"}"
        )
    );
}
