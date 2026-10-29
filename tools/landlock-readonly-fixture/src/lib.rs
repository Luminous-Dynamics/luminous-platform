#![deny(unsafe_op_in_unsafe_fn)]

//! Minimal Linux Landlock adapter for an isolated read-only fixture.
//!
//! SECURITY BOUNDARY:
//! - This crate applies actual kernel-enforced Landlock restrictions to the current
//!   process and its future children; it is not a general syscall-policy adapter.
//! - It only demonstrates a read-only filesystem hierarchy on Linux kernels with
//!   Landlock ABI >= 3. It does not install seccomp, restrict network access, or
//!   establish host-kernel integrity.
//! - The caller must enter this function before opening any sensitive resource or
//!   launching untrusted code. Open descriptors acquired before restriction may
//!   retain access that Landlock does not revoke.
//! - This fixture is for a disposable test process. Do not treat its simple ruleset
//!   as a production sandbox profile.

#[cfg(not(target_os = "linux"))]
compile_error!("luminous-landlock-readonly-fixture is Linux-only");

#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
compile_error!("Landlock syscall numbers are configured only for x86_64 and aarch64");

use std::ffi::CString;
use std::io;
use std::mem::size_of;
use std::os::raw::{c_char, c_int, c_long, c_ulong};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

const SYS_LANDLOCK_CREATE_RULESET: c_long = 444;
const SYS_LANDLOCK_ADD_RULE: c_long = 445;
const SYS_LANDLOCK_RESTRICT_SELF: c_long = 446;
const LANDLOCK_CREATE_RULESET_VERSION: c_int = 1;
const LANDLOCK_RULE_PATH_BENEATH: c_int = 1;

const PR_SET_NO_NEW_PRIVS: c_int = 38;
const O_CLOEXEC: c_int = 0o2000000;
const O_PATH: c_int = 0o10000000;

// Landlock filesystem rights available in ABI v1-v3.
// ABI v2 adds REFER; ABI v3 adds TRUNCATE.
const ACCESS_FS_EXECUTE: u64 = 1 << 0;
const ACCESS_FS_WRITE_FILE: u64 = 1 << 1;
const ACCESS_FS_READ_FILE: u64 = 1 << 2;
const ACCESS_FS_READ_DIR: u64 = 1 << 3;
const ACCESS_FS_REMOVE_DIR: u64 = 1 << 4;
const ACCESS_FS_REMOVE_FILE: u64 = 1 << 5;
const ACCESS_FS_MAKE_CHAR: u64 = 1 << 6;
const ACCESS_FS_MAKE_DIR: u64 = 1 << 7;
const ACCESS_FS_MAKE_REG: u64 = 1 << 8;
const ACCESS_FS_MAKE_SOCK: u64 = 1 << 9;
const ACCESS_FS_MAKE_FIFO: u64 = 1 << 10;
const ACCESS_FS_MAKE_BLOCK: u64 = 1 << 11;
const ACCESS_FS_MAKE_SYM: u64 = 1 << 12;
const ACCESS_FS_REFER: u64 = 1 << 13;
const ACCESS_FS_TRUNCATE: u64 = 1 << 14;

const READ_ONLY_ACCESS: u64 = ACCESS_FS_READ_FILE | ACCESS_FS_READ_DIR;
const HANDLED_ACCESS_ABI3: u64 = ACCESS_FS_EXECUTE
    | ACCESS_FS_WRITE_FILE
    | ACCESS_FS_READ_FILE
    | ACCESS_FS_READ_DIR
    | ACCESS_FS_REMOVE_DIR
    | ACCESS_FS_REMOVE_FILE
    | ACCESS_FS_MAKE_CHAR
    | ACCESS_FS_MAKE_DIR
    | ACCESS_FS_MAKE_REG
    | ACCESS_FS_MAKE_SOCK
    | ACCESS_FS_MAKE_FIFO
    | ACCESS_FS_MAKE_BLOCK
    | ACCESS_FS_MAKE_SYM
    | ACCESS_FS_REFER
    | ACCESS_FS_TRUNCATE;

#[repr(C)]
struct LandlockRulesetAttr {
    handled_access_fs: u64,
}

#[repr(C)]
struct LandlockPathBeneathAttr {
    allowed_access: u64,
    parent_fd: c_int,
}

unsafe extern "C" {
    fn syscall(number: c_long, ...) -> c_long;
    fn open(path: *const c_char, flags: c_int, ...) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn prctl(
        option: c_int,
        arg2: c_ulong,
        arg3: c_ulong,
        arg4: c_ulong,
        arg5: c_ulong,
    ) -> c_int;
}

struct OwnedFd(c_int);

impl OwnedFd {
    fn new(fd: c_int) -> io::Result<Self> {
        if fd < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(Self(fd))
        }
    }

    fn raw(&self) -> c_int {
        self.0
    }
}

impl Drop for OwnedFd {
    fn drop(&mut self) {
        // SAFETY: this wrapper owns the descriptor and closes it exactly once.
        // Drop cannot report close errors; the security-critical operations are
        // checked before returning from the adapter.
        unsafe {
            let _ = close(self.0);
        }
    }
}

fn open_path(path: &Path) -> io::Result<OwnedFd> {
    let path = CString::new(path.as_os_str().as_bytes())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "path contains NUL"))?;
    // SAFETY: path is NUL-terminated and the supplied flags do not require a mode arg.
    OwnedFd::new(unsafe { open(path.as_ptr(), O_PATH | O_CLOEXEC) })
}

/// Applies a kernel-enforced, read-only Landlock rule for one directory subtree.
///
/// Requires Landlock ABI >= 3 so file truncation, rename/link transitions, file
/// writes, removals, creation, and execution can all be included in the handled
/// access mask. Any unsupported or failed step returns an error and does not
/// claim that the sandbox has been installed.
///
/// Important: call this before opening any sensitive file descriptors or running
/// any untrusted task. Landlock restricts the current thread and its future
/// children; it does not revoke access through descriptors opened beforehand.
pub fn apply_read_only_directory(allowed_root: &Path) -> io::Result<i32> {
    let abi = {
        // SAFETY: LANDLOCK_CREATE_RULESET_VERSION requires a null attr, size zero,
        // and flag 1 as documented by the Linux Landlock userspace API.
        unsafe {
            syscall(
                SYS_LANDLOCK_CREATE_RULESET,
                std::ptr::null::<LandlockRulesetAttr>(),
                0usize,
                LANDLOCK_CREATE_RULESET_VERSION as c_long,
            )
        }
    };
    if abi < 0 {
        return Err(io::Error::last_os_error());
    }
    if abi < 3 {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            format!("Landlock ABI {abi} is below required ABI 3"),
        ));
    }

    let ruleset_attr = LandlockRulesetAttr {
        handled_access_fs: HANDLED_ACCESS_ABI3,
    };
    let ruleset_fd = {
        // SAFETY: attr points to a valid repr(C) ABI3 struct for the duration of
        // the syscall; the size exactly matches the struct and flags are zero.
        let fd = unsafe {
            syscall(
                SYS_LANDLOCK_CREATE_RULESET,
                &ruleset_attr as *const LandlockRulesetAttr,
                size_of::<LandlockRulesetAttr>(),
                0 as c_long,
            )
        };
        OwnedFd::new(fd as c_int)?
    };

    let canonical_root = std::fs::canonicalize(allowed_root)?;
    let root_fd = open_path(&canonical_root)?;
    let rule = LandlockPathBeneathAttr {
        allowed_access: READ_ONLY_ACCESS,
        parent_fd: root_fd.raw(),
    };
    let add_result = {
        // SAFETY: rule points to a valid repr(C) struct; parent_fd is an open O_PATH
        // descriptor for the canonical directory, and flags are zero.
        unsafe {
            syscall(
                SYS_LANDLOCK_ADD_RULE,
                ruleset_fd.raw() as c_long,
                LANDLOCK_RULE_PATH_BENEATH as c_long,
                &rule as *const LandlockPathBeneathAttr,
                0 as c_long,
            )
        }
    };
    if add_result < 0 {
        return Err(io::Error::last_os_error());
    }
    drop(root_fd);

    let no_new_privs = {
        // SAFETY: PR_SET_NO_NEW_PRIVS takes integer 1 and three zero arguments.
        unsafe { prctl(PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) }
    };
    if no_new_privs != 0 {
        return Err(io::Error::last_os_error());
    }

    let restrict_result = {
        // SAFETY: ruleset_fd is a live descriptor created above and flags are zero.
        unsafe { syscall(SYS_LANDLOCK_RESTRICT_SELF, ruleset_fd.raw() as c_long, 0 as c_long) }
    };
    if restrict_result < 0 {
        return Err(io::Error::last_os_error());
    }
    drop(ruleset_fd);
    Ok(abi as i32)
}
