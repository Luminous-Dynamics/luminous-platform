# Linux Landlock Read-Only Fixture

This is a small, real-kernel T1 integration probe—not a simulation of a policy decision.

## What it tests

The parent creates a disposable temporary directory containing:
- an allowed-read-only directory with one synthetic canary file;
- a separate blocked directory containing a second synthetic canary and a path where a forbidden new file must not appear.

It launches a child process. Before opening the canary, the child installs a Landlock filesystem ruleset with ABI >= 3, then installs an architecture-checked seccomp filter. Landlock grants only file and directory reads inside the allowed subtree and handles common file-content, create/remove, rename/link, and truncation rights. The seccomp layer denies a fixed architecture-specific list of common chmod/chown, xattr-mutation, and timestamp-mutation syscall numbers. It is a **denylist**, not a default-deny syscall sandbox.

The child must then:
1. Read the permitted synthetic canary.
2. Receive PermissionDenied when reading a second canary outside the permitted subtree.
3. Receive PermissionDenied when opening the permitted canary for writing.
4. Receive PermissionDenied when creating a new file outside the permitted subtree.
5. Receive PermissionDenied when attempting to change the permitted canary's mode through chmod.
6. Receive PermissionDenied when attempting to set a user extended attribute.
7. Receive PermissionDenied when attempting to change the file timestamps.

The parent first verifies that xattrs and timestamp APIs work on the fixture filesystem. After the child exits, the unsandboxed parent independently verifies that the allowed canary's bytes, mode, and modification time remain unchanged; the tested xattr is absent; the outside canary remains present and unchanged; and the forbidden new file does not exist. A machine-readable JSON result includes the observed Landlock ABI, the seccomp denylist size, child checks, and separate parent-state-oracle checks. The CI workflow validates this versioned result schema and all required fields; it does not accept process exit status alone as evidence. The temporary fixture is removed on normal return paths.

## Run

    cargo fmt --manifest-path tools/landlock-readonly-fixture/Cargo.toml -- --check
    cargo test --locked --manifest-path tools/landlock-readonly-fixture/Cargo.toml
    cargo run --locked --manifest-path tools/landlock-readonly-fixture/Cargo.toml
    cargo clippy --locked --manifest-path tools/landlock-readonly-fixture/Cargo.toml -- -D warnings

The CI workflow runs this integration binary on the hosted Linux runner. The test intentionally fails if Landlock is unavailable or the kernel ABI is below version 3; lack of support is not silently converted to a pass. The successful run prints distinct markers for allowed read, denied outside read, denied write to the existing fixture, and denied creation outside the subtree.

## Security boundary

The implementation uses direct Linux syscalls with a tiny FFI surface and no third-party dependencies. It calls PR_SET_NO_NEW_PRIVS, creates a ruleset, adds an O_PATH-anchored subtree rule, and calls landlock_restrict_self. ABI >= 3 is required because older ABI versions could not restrict truncation. See the official Linux Landlock userspace API: https://docs.kernel.org/userspace-api/landlock.html

This test demonstrates only the named kernel's Landlock file-content controls plus the tested chmod, setxattr, and utimensat denials for this fixture under this runner configuration. It is **not** a complete read-only filesystem sandbox. The seccomp filter denies a fixed list of syscall numbers on the compiled architecture; it does not cover every metadata-changing route, filesystem-specific ioctl, device/IPC path, or unlisted syscall. The JSON record explicitly sets `full_metadata_mutation_coverage` to `false`.

It also does not:
- integrate the full syscall-policy core into a general process launcher;
- enforce a default-deny syscall policy or validate arbitrary syscall arguments;
- establish network egress restrictions, inherited-file-descriptor safety, IPC/device restrictions, or kernel integrity;
- defeat a compromised kernel, host administrator, or access through an already-open sensitive file descriptor;
- establish production isolation, cross-kernel portability, or superiority over commercial sandbox products.

Call the result `read_only_file_content_plus_tested_metadata_guards`; do not describe it as complete metadata protection or general endpoint prevention.