#![deny(unsafe_op_in_unsafe_fn)]

#[cfg(not(target_os = "linux"))]
compile_error!("luminous-fd-hygiene-probe is Linux-only");

use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::os::fd::{AsRawFd, RawFd};
use std::os::raw::{c_int, c_uint, c_void};
use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};
use std::process::{self, Command};
use std::time::{SystemTime, UNIX_EPOCH};

const CANARY: &str = "luminous-fd-inheritance-canary-v1";
const F_DUPFD: c_int = 0;
const F_GETFD: c_int = 1;
const FD_CLOEXEC: c_int = 1;
const EBADF_ERRNO: i32 = 9;
const FIRST_NON_STDIO: c_uint = 3;
const MIN_DUP_FD: c_int = 200;
const CLOSE_RANGE_UNSHARE: c_int = 1 << 1;
const CLOSE_RANGE_CLOEXEC: c_int = 1 << 2;

unsafe extern "C" {
    fn close_range(first: c_uint, last: c_uint, flags: c_int) -> c_int;
    fn fcntl(fd: c_int, command: c_int, ...) -> c_int;
    fn read(fd: c_int, buffer: *mut c_void, count: usize) -> isize;
    fn close(fd: c_int) -> c_int;
}

struct InheritableFd(RawFd);

impl InheritableFd {
    fn duplicate(source: RawFd) -> io::Result<Self> {
        // F_DUPFD deliberately creates a descriptor without FD_CLOEXEC. This is
        // the negative-control condition; the second child tests the exec guard.
        // SAFETY: fcntl receives a valid open descriptor and an integer minimum.
        let fd = unsafe { fcntl(source, F_DUPFD, MIN_DUP_FD) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }

        // SAFETY: fd was just returned by F_DUPFD and is owned by this value.
        let flags = unsafe { fcntl(fd, F_GETFD) };
        if flags < 0 || flags & FD_CLOEXEC != 0 {
            // SAFETY: fd is the newly-created descriptor owned here.
            unsafe {
                let _ = close(fd);
            }
            return Err(if flags < 0 {
                io::Error::last_os_error()
            } else {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "F_DUPFD unexpectedly set FD_CLOEXEC",
                )
            });
        }
        Ok(Self(fd))
    }

    fn raw(&self) -> RawFd {
        self.0
    }
}

impl Drop for InheritableFd {
    fn drop(&mut self) {
        // SAFETY: this guard owns the descriptor and closes it once.
        unsafe {
            let _ = close(self.0);
        }
    }
}

fn worker(mode: &str, fd: RawFd) -> io::Result<()> {
    // SAFETY: F_GETFD only inspects the supplied descriptor number.
    let flags = unsafe { fcntl(fd, F_GETFD) };

    match mode {
        "expect-inherited" => {
            if flags < 0 {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!(
                        "negative control expected inherited fd {fd}; fcntl failed: {}",
                        io::Error::last_os_error()
                    ),
                ));
            }

            let mut observed = vec![0_u8; CANARY.len()];
            // SAFETY: observed is writable for observed.len() bytes.
            let read_len = unsafe {
                read(
                    fd,
                    observed.as_mut_ptr().cast::<c_void>(),
                    observed.len(),
                )
            };
            if read_len < 0 {
                return Err(io::Error::last_os_error());
            }
            observed.truncate(read_len as usize);
            if observed != CANARY.as_bytes() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "inherited descriptor did not expose the exact synthetic canary",
                ));
            }
            println!("NEGATIVE_CONTROL_PASS inherited_fd_readable=true");
            Ok(())
        }
        "expect-absent" => {
            if flags == -1 && io::Error::last_os_error().raw_os_error() == Some(EBADF_ERRNO) {
                println!("CLOEXEC_CONTROL_PASS inherited_fd_absent=true");
                Ok(())
            } else {
                Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    format!(
                        "descriptor {fd} remained open after exec or absence was inconclusive (fcntl={flags}, errno={:?})",
                        io::Error::last_os_error().raw_os_error()
                    ),
                ))
            }
        }
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "unknown worker mode",
        )),
    }
}

fn run_worker(
    executable: &Path,
    mode: &str,
    fd: RawFd,
    expected_marker: &str,
) -> io::Result<()> {
    let output = Command::new(executable)
        .arg("--worker")
        .arg(mode)
        .arg(fd.to_string())
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
    if !stdout.lines().any(|line| line == expected_marker) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("worker {mode} omitted expected evidence marker {expected_marker}"),
        ));
    }
    Ok(())
}

fn run_parent() -> io::Result<()> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| io::Error::other(format!("system clock before Unix epoch: {error}")))?
        .as_nanos();
    let root: PathBuf = env::temp_dir().join(format!(
        "luminous-fd-hygiene-{}-{timestamp}",
        process::id()
    ));
    let path = root.join("canary");
    let mut directory = fs::DirBuilder::new();
    directory.mode(0o700);
    directory.create(&root)?;

    let result = (|| {
        let mut writer = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        writer.write_all(CANARY.as_bytes())?;
        writer.sync_all()?;
        drop(writer);

        let reader = File::open(&path)?;
        let descriptor = InheritableFd::duplicate(reader.as_raw_fd())?;
        let executable = env::current_exe()?;

        // Negative control: prove the intentionally inheritable descriptor really
        // crosses exec and can read the canary before applying the guard.
        run_worker(
            &executable,
            "expect-inherited",
            descriptor.raw(),
            "NEGATIVE_CONTROL_PASS inherited_fd_readable=true",
        )?;

        // CLOSE_RANGE_UNSHARE avoids mutating a descriptor table shared with
        // other tasks; CLOSE_RANGE_CLOEXEC preserves parent descriptors but
        // prevents descriptors >= 3 from surviving the next successful exec.
        // The probe fails closed if this kernel/API control is unavailable.
        // SAFETY: the range [3, UINT_MAX] is valid and flags are documented Linux
        // close_range flags. This test process is single-threaded by design.
        let mark_result = unsafe {
            close_range(
                FIRST_NON_STDIO,
                c_uint::MAX,
                CLOSE_RANGE_UNSHARE | CLOSE_RANGE_CLOEXEC,
            )
        };
        if mark_result != 0 {
            return Err(io::Error::last_os_error());
        }

        // CLOEXEC must mark, not immediately close, the descriptor in the parent.
        // SAFETY: descriptor is still owned and open in this process.
        let parent_flags = unsafe { fcntl(descriptor.raw(), F_GETFD) };
        if parent_flags < 0 || parent_flags & FD_CLOEXEC == 0 {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "parent descriptor was not retained with FD_CLOEXEC set",
            ));
        }

        // Positive control: after exec, the descriptor must not exist in the
        // child. A valid child exit and exact marker are both required.
        run_worker(
            &executable,
            "expect-absent",
            descriptor.raw(),
            "CLOEXEC_CONTROL_PASS inherited_fd_absent=true",
        )?;

        // Verify the close-on-exec marker did not close the parent's descriptor.
        // SAFETY: descriptor remains owned by this parent process.
        if unsafe { fcntl(descriptor.raw(), F_GETFD) } < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "parent descriptor unexpectedly disappeared",
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
        let worker_result = if args.len() == 4 {
            match args[3].parse::<RawFd>() {
                Ok(fd) => worker(&args[2], fd),
                Err(error) => Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("invalid descriptor argument: {error}"),
                )),
            }
        } else {
            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "worker requires a mode and descriptor number",
            ))
        };

        if let Err(error) = worker_result {
            eprintln!("FAIL fd-hygiene worker: {error}");
            process::exit(2);
        }
        return;
    }

    if let Err(error) = run_parent() {
        eprintln!("FAIL fd-hygiene probe: {error}");
        process::exit(1);
    }

    let checkout_sha = env::var("LUMINOUS_CHECKOUT_SHA")
        .ok()
        .filter(|value| {
            value.len() == 40
                && value.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        })
        .unwrap_or_else(|| "unbound_local".to_string());
    let kernel_release = fs::read_to_string("/proc/sys/kernel/osrelease")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| {
            !value.is_empty()
                && value
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || "._+-".contains(ch))
        })
        .unwrap_or_else(|| "unknown".to_string());

    println!(
        concat!(
            "{\"schema\":\"luminous.fd-inheritance-boundary-result.v1\",",
            "\"status\":\"pass\",",
            "\"checkout_sha\":\"{}\",",
            "\"kernel_release\":\"{}\",",
            "\"negative_control_inherited_fd_readable\":true,",
            "\"cloexec_control_descriptor_absent_after_exec\":true,",
            "\"parent_descriptor_survived_marking\":true,",
            "\"parent_descriptor_cloexec_flag_set\":true,",
            "\"close_range_unshare_enabled\":true,",
            "\"close_range_cloexec_enabled\":true,",
            "\"scope\":\"exec_boundary_descriptor_hygiene_probe_only\"}"
        ),
        checkout_sha,
        kernel_release
    );
}
