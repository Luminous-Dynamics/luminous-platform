# Linux Landlock Read-Only Fixture

This is a small, real-kernel T1 integration probe—not a simulation of a policy decision.

## What it tests

The parent creates a disposable temporary directory containing:
- an allowed-read-only directory with one synthetic canary file;
- a separate blocked directory containing a second synthetic canary and a path where a forbidden new file must not appear.

It launches a child process. Before the child opens the canary, the child installs a Landlock filesystem ruleset with ABI >= 3. The ruleset handles file execution, writes, reads, directory reads, removals, creation, rename/link transitions, and truncation. The only path-beneath rule grants READ_FILE and READ_DIR inside the allowed subtree.

The child must then:
1. read the expected synthetic canary successfully;
2. fail to read the second canary outside the allowed subtree with PermissionDenied;
3. fail to open the existing allowed canary for writing with PermissionDenied;
4. fail to create a new file outside the allowed subtree with PermissionDenied.

After the child exits, the unsandboxed parent independently checks the allowed canary is unchanged, the outside canary is still present and unchanged, and the forbidden new file does not exist. The program emits one JSON result line with the observed Landlock ABI, the child's denied/allowed operation checks, and three separate parent-state-oracle fields. These distinguish a child's self-report from independently verified fixture state. The temporary tree is removed on normal return paths.

## Run

    cargo fmt --manifest-path tools/landlock-readonly-fixture/Cargo.toml -- --check
    cargo test --locked --manifest-path tools/landlock-readonly-fixture/Cargo.toml
    cargo run --locked --manifest-path tools/landlock-readonly-fixture/Cargo.toml
    cargo clippy --locked --manifest-path tools/landlock-readonly-fixture/Cargo.toml -- -D warnings

The CI workflow runs this integration binary on the hosted Linux runner. The test intentionally fails if Landlock is unavailable or the kernel ABI is below version 3; lack of support is not silently converted to a pass. The successful run prints distinct markers for allowed read, denied outside read, denied write to the existing fixture, and denied creation outside the subtree.

## Security boundary

The implementation uses direct Linux syscalls with a tiny FFI surface and no third-party dependencies. It calls PR_SET_NO_NEW_PRIVS, creates a ruleset, adds an O_PATH-anchored subtree rule, and calls landlock_restrict_self. ABI >= 3 is required because older ABI versions could not restrict truncation. See the official Linux Landlock userspace API: https://docs.kernel.org/userspace-api/landlock.html

This test proves only the tested kernel's file-content read/write/create controls for the stated fixture under the tested runner configuration. It is **not** a complete read-only filesystem lock: Landlock does not provide a general metadata-mutation right for operations such as chmod/chown, xattr and timestamp updates, so those need separate policy controls (for example, a carefully scoped seccomp layer) and tests. The machine-readable result explicitly marks metadata mutation syscalls as not covered. It does not:
- integrate the full syscall-policy core into a general process launcher;
- install seccomp filters or enforce syscall argument policy;
- restrict metadata mutation syscalls, network egress, inherited file descriptors, IPC, devices, credentials, or kernel attack surfaces;
- defeat a compromised kernel, host administrator, or an already-open sensitive file descriptor;
- establish production isolation, portability to other kernels, or superiority over commercial sandbox products.

Call the result landlock_file_content_access_fixture_observation; do not label it a complete read-only filesystem policy or a general endpoint-prevention result.
