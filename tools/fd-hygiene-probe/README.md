# Inherited file-descriptor boundary probe

This is a narrow Linux integration probe for one sandbox-launch invariant: **a child must not inherit ambient file descriptors that were not explicitly granted to it**.

## Why this exists

Path restrictions are not a complete resource boundary. Linux Landlock documents that access rights associated with file descriptors are affected by when and how those descriptors were acquired; pre-existing descriptors can retain access, including across process boundaries. See the [Landlock userspace API documentation](https://docs.kernel.org/userspace-api/landlock.html) and its [file-descriptor rights section](https://kernel.org/doc/html/v7.0-rc5/userspace-api/landlock.html).

The Linux `close_range(2)` interface supports `CLOSE_RANGE_CLOEXEC` (Linux 5.11+) to mark descriptors for closure at a successful `execve`, while retaining them in the parent. `CLOSE_RANGE_UNSHARE` prevents the operation from mutating a file-descriptor table shared with other tasks. See the [close_range(2) manual](https://man7.org/linux/man-pages/man2/close_range.2.html).

## What the executable does

1. Creates a disposable synthetic file containing a fixed non-secret canary.
2. Duplicates a read-only descriptor to a high descriptor number without `FD_CLOEXEC`.
3. **Negative control:** executes itself and requires the child to read the exact canary through that inherited descriptor. If this doesn't work, the probe cannot claim it established the inheritance hazard.
4. Calls `close_range(3, UINT_MAX, CLOSE_RANGE_UNSHARE | CLOSE_RANGE_CLOEXEC)`.
5. **Positive control:** executes itself again and requires the descriptor to be absent in the child with `EBADF`, while the descriptor remains open and marked `FD_CLOEXEC` in the parent.
6. Emits one JSON evidence record. CI checks its schema and all Boolean outcomes.

The probe fails closed if `close_range`/the requested flags are unavailable. It does not fall back to a weaker behavior and call it success.

## Scope and limitations

- Demonstrates descriptor inheritance and the exec-boundary effect of the named `close_range` flags on the specific Linux CI runner.
- Uses only synthetic local data; no network, host configuration mutation, privilege escalation, or arbitrary commands are involved.
- It does **not** implement a production launcher, audit every descriptor type, prove race freedom for a multithreaded supervisor, prove Landlock/seccomp correctness, or establish general endpoint security.
- A real launcher must define its descriptor contract explicitly: preserve only required channels, mark/close everything else before executing untrusted code, and test all intentional IPC, credential, socket, device, namespace and control descriptors.
- This probe does not prove that an inherited descriptor is the only possible escape route. It tests one known resource-transfer channel.

## Run

```sh
cargo fmt --manifest-path tools/fd-hygiene-probe/Cargo.toml -- --check
cargo test --locked --manifest-path tools/fd-hygiene-probe/Cargo.toml
cargo run --quiet --locked --manifest-path tools/fd-hygiene-probe/Cargo.toml
```

The final command prints a single `luminous.fd-inheritance-boundary-result.v1` JSON record on success. Treat a missing record, an unsupported kernel interface, failed positive/negative control, or malformed evidence as a failed probe—not as zero observed leakage.
