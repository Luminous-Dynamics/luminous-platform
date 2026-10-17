# Hearth Node pre-hardware VM tests

This directory contains the first executable NixOS/QEMU test scaffold for the
Hearth Node qualification programme.

## What this test proves

`hearth-foundation.nix` defines two test-double systemd units inside a
disposable NixOS VM:

- A core local-service sentinel that should stay active and be restartable.
- An optional inference sentinel that is deliberately made to fail.

The test asserts that the optional failure remains visible, is not a required
dependency of the core service, and does not prevent the core service from
remaining active or restarting.

**This is a test of the intended systemd dependency boundary, not a test of an
implemented Hearth Node, Home Assistant installation, real inference engine,
network isolation, hardware, or household safety.** Replace the sentinels with
the actual Hearth NixOS modules as they are implemented. Do not promote a
product profile based on this scaffold alone.

## Run the pinned test flake

The test harness has its own flake.nix and committed flake.lock, independent
of the repository root. Nixpkgs is pinned to commit
151fa4e8ddfdd8dd25d945ad94ed54a13de9f6e4.

Run the check from a Nix-enabled x86_64 Linux host:

```sh
nix flake check --print-build-logs ./tests/home-systems-vm
```

Or build just the VM test derivation:

```sh
nix build --print-build-logs ./tests/home-systems-vm#hearth-foundation
```

The lock file records the Nixpkgs source revision and NAR hash. Do not run
nix flake update casually: any pin change should be reviewed and the test
rerun against the updated input. The test permits QEMU software emulation when
KVM is unavailable; this can be considerably slower.

A compatibility runner remains available for development against a caller
supplied Nixpkgs checkout:

```sh
nix-build tests/home-systems-vm/run.nix -I nixpkgs=/path/to/nixpkgs-checkout
```

The run.nix path uses the caller's Nixpkgs and is not the pinned qualification
path. The NixOS test runner uses its standard Python test-driver DSL; no Python
service/runtime is added to the product. See the official
[NixOS test manual](https://nixos.org/manual/nixos/stable/#sec-call-nixos-test-outside-nixos).

## Next scenarios

Do not place all scenarios into this sentinel test. Add separate tests with
specific assertions and evidence for:

1. Boot and local service operation with WAN/DNS unavailable.
2. A hostile guest unable to reach a protected host/management endpoint.
3. CPU, memory, process, and disk exhaustion while the core service is protected.
4. Interrupted update, failed activation, rollback, and clean-VM restore.
5. Home Assistant integration using synthetic devices and no personal credentials.

Each suite must state what it cannot establish. In particular, a VM cannot
qualify actual power consumption, acoustics, sustained physical thermals, RF
behavior, electrical safety, or a physical emergency stop.
