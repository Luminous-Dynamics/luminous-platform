# Landlock pre-opened writable descriptor experiment

This probe tests one specific launch-ordering failure mode: a writable file descriptor is opened before the child installs its Landlock restrictions, then inherited into that child. The experiment uses a disposable file with synthetic contents and an independent parent-process oracle.

## Why this is a separate test

Landlock checks filesystem access in the context of its documented rights model. Some rights, including truncate and device-ioctl restrictions, are associated with descriptors when opened; a file descriptor acquired before restriction can retain authority that a path-only review of the post-restriction process would miss. The kernel documentation specifically warns that an existing descriptor can retain different rights from a descriptor opened after restrictions and that such descriptors can be passed between processes.

References:
- [Linux Landlock userspace API — file-descriptor rights](https://docs.kernel.org/userspace-api/landlock.html)
- [Linux close_range(2) manual](https://man7.org/linux/man-pages/man2/close_range.2.html)

## Experiment

The parent creates a private temporary directory (mode 0700) and a file containing a non-secret baseline. It opens the file read/write and duplicates a descriptor without close-on-exec.

1. **Expected-exposure negative control:** the child inherits the writable descriptor and then applies a Landlock ruleset that handles WRITE_FILE and TRUNCATE with no path grants. It writes a same-length marker through the already-open descriptor. The parent must independently observe the exact changed bytes.
2. The parent restores the baseline and verifies the restoration.
3. **Mitigation positive control:** an otherwise equivalent writable descriptor is marked FD_CLOEXEC. The child applies the same Landlock ruleset; it must observe that the descriptor is absent (EBADF), and opening the file by pathname for writing must be denied.
4. The parent verifies that the file remains byte-for-byte unchanged and that the descriptor remains open with FD_CLOEXEC in the parent.

The experiment requires Landlock ABI >= 3 because it handles the truncation right. Unsupported or inconclusive results fail the probe; they are never reported as successful containment.

## Claim boundary

- The expected exposure demonstrates why a launcher must control descriptor inheritance and ordering, rather than assuming a later pathname policy revokes an already-granted handle.
- The safe control demonstrates this particular descriptor/exec case, not a complete descriptor inventory.
- This is not a general sandbox, kernel exploit, privilege escalation, broad bypass claim, or production launcher.
- The test does not cover every descriptor type, filesystem-specific operation, path race, network interface, credentials, IPC channel, or multithreaded launcher race.
- The result schema explicitly says full_descriptor_hygiene_coverage=false. No vendor comparison or product-readiness claim follows from this micro-test.

## Run

\`\`\`sh
cargo fmt --manifest-path tools/landlock-preopened-fd-probe/Cargo.toml -- --check
cargo test --locked --manifest-path tools/landlock-preopened-fd-probe/Cargo.toml
cargo run --quiet --locked --manifest-path tools/landlock-preopened-fd-probe/Cargo.toml
\`\`\`

Successful execution emits one luminous.landlock-preopened-fd-result.v1 JSON evidence record.
