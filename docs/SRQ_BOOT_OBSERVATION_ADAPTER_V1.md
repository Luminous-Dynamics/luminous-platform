# SRQ boot-observation adapter v1

**Status:** integration contract and implementation plan. The adapter is not implemented or qualified; local Sovereign Boot JSON is not trusted attestation evidence by itself.

## Objective

Bind the logical promotion and durable decision to the exact immutable deployment that actually booted. Do not use a logical increment such as `generation=42` as a substitute for the platform deployment identity.

Sovereign Boot's public `nix/modules/sovereign-boot.nix` already deliberately distinguishes:
- `/run/booted-system`: resolved identity of the system that booted;
- `/run/current-system`: current userspace activation, which can change during a live `nixos-rebuild switch/test` without rebooting.

The module also writes an `lkg-decision-v1` JSON record containing boot ID, booted-generation path, current userspace generation, health state and a decision/reason. That local record is useful diagnostic evidence but does not contain the full SRQ target/decision/challenge/policy/artifact binding and is not independently signed. The module references `spore-recovery-linux` and `spore-boot-state`; issue [Sovereign Boot #3](https://github.com/Luminous-Dynamics/sovereign-boot/issues/3) correctly tracks that these state/recovery tools are not yet exported as a qualified standalone package.

## Protocol: challenge, observe, authenticate, reconcile

1. **Durable promotion:** the promotion transaction commits decision ID, target ID, one-time promotion challenge consumption, logical generation, immutable deployment identity, policy and artifact digests.
2. **Fresh observation challenge:** an independent verifier issues a new challenge *after* the committed decision is readable. It binds target ID, decision ID, expected logical generation, expected immutable deployment ID, expected effective-policy digest, challenge ID, issuance/expiry data, and audience/verifier identity. This must be distinct from the promotion challenge.
3. **Observe:** an adapter samples the current boot ID and resolves `/run/booted-system` and `/run/current-system` independently. It must not relabel `current-system` as the booted system. It collects the expected artifact/policy measurement from a defined source; a pathname alone is not a measured digest.
4. **Authenticate:** the adapter returns the verifier's nonce and decision binding in a response authenticated by an explicitly configured trust root. A userspace file written by root is diagnostic but not a hardware attestation. A production attestation tier must specify key custody and validation—e.g. a TPM-backed quote and PCR policy if hardware root-of-trust claims are required.
5. **Reconcile:** a verifier checks signature/attester identity, nonce freshness and one-time use, target binding, logical generation, immutable deployment identity, effective-policy digest and artifact measurement. Any missing, stale, replayed or contradictory fact returns Pending/Blocked, never PASS.
6. **Preserve:** the signed evidence subject includes the exact response bytes, challenge bytes, verifier policy digest, source/build identity and measured file hashes. It must remain bound to an exact workflow/source commit and the result's execution tier.

## Wire-record shape

The future authenticated response should contain at least these fields (the exact encoding and cryptographic envelope are a separate implementation artifact):

| Field | Meaning |
|---|---|
| `schema` | Fixed protocol version, e.g. `srq-boot-observation-v1` |
| `target_id` | Stable target identity established by the verifier trust policy |
| `decision_id` | Exact durable promotion decision being reconciled |
| `challenge_id` | Fresh post-commit observation challenge |
| `boot_id` | Current Linux boot instance identifier, such as `/proc/sys/kernel/random/boot_id`; informative only until authenticated |
| `logical_generation` | SRQ promotion counter, not a NixOS/OSTree generation number |
| `expected_deployment_id` | Immutable deployment selected by the authorized decision |
| `booted_deployment_id` | Nix store path or platform deployment identity resolved from the source representing what actually booted |
| `current_userspace_deployment_id` | Separate current userspace identity for live-switch/mixed-state detection |
| `effective_policy_digest` | Digest of the policy actually applied in the observed deployment, using a specified canonical encoding |
| `artifact_digest` | Digest of the artifact identity the promotion authorized, tied to the observed deployment by an explicit mapping |
| `measurement` | Typed measurement format, measurement bytes/digest, key/attester identity and the verifier's measurement policy |
| `source` | Agent build SHA, OS/runtime version, measurement tool version and exact evidence-source description |

For NixOS, `expected_deployment_id`, `booted_deployment_id`, and `current_userspace_deployment_id` are canonical `/nix/store/<hash>-<name>` paths, not SRQ integer counters. For OSTree, specify the deployment checksum plus whatever identity is needed to uniquely distinguish the active deployment. Do not compare raw Nix store paths directly to artifact SHA-256 values; preserve both identifiers and define their relationship.

## Trust tiers

- **Tier D0 — local diagnostic:** reads the NixOS runtime paths and local LKG JSON. Useful for debugging and non-security health dashboards. It cannot set a release gate to PASS.
- **Tier D1 — authenticated software response:** a challenge-bound response signed with a provisioned target key and verified by an independent verifier. This proves possession/control of that key under the policy; it does not by itself prove a hardware root of trust or protect against a compromised target OS.
- **Tier D2 — hardware-backed runtime measurement:** the response includes a fresh, verified hardware-rooted measurement (for example, a TPM quote under an approved PCR/secure-boot policy) and ties the measured state to the booted deployment. Requires a documented provisioning, revocation, freshness and recovery policy.
- **Tier D3 — physical recovery qualification:** matched physical reset/power-interruption and recovery evidence. D2 signatures and QEMU results do not substitute for this tier.

All record parsing uses fail-closed schema/field validation. No absent field is silently filled from `/run/current-system`, a cached receipt, an earlier boot ID, or an expected digest copied from the artifact itself.

## Test matrix

### Model/contract (host CI)

- NixOS old `booted-system` with new `current-system` must be blocked even if the logical counter matches.
- Durable decision deployment ID, challenge expected deployment ID and actual booted deployment ID each independently mismatch; each must fail closed.
- Challenge reuse, wrong target/decision, expired or missing challenge, missing measurement, missing boot ID and wrong key/attester all fail closed.
- Mutate each field across intent, durable record, challenge and observation; no contradictory record can reconcile as consistent.
- A well-shaped local JSON file, a valid SHA-256 digest or an LKG disposition of `promoted` is not proof of signature/attestation.

### Disposable QEMU/NixOS

- Collect real `/run/booted-system`, `/run/current-system`, `/proc/sys/kernel/random/boot_id` and the boot-state/LKG outputs in a VM with pinned package overrides.
- Exercise reboot into the authorized deployment, live userspace switch without reboot, rollback to previous generation, missing/malformed receipt, service failure, and challenge replay/expiry.
- Report false accepts/rejects, boot-to-observation latency p50/p95/p99, recovery outcome, raw receipts, source SHA, VM definition and hashes. QEMU remains a separate execution tier.

### Physical

- Only after a trust-root design and QEMU lane are stable, repeat relevant reset/power-interruption cases on declared hardware/storage with actual attested runtime measurements.
- Never infer physical Secure Boot, TPM, storage durability, GPU handoff or power-loss behavior from host CI or a software-only receipt.

## Current blockers and dependencies

- [Sovereign Boot #3](https://github.com/Luminous-Dynamics/sovereign-boot/issues/3): standalone lifecycle/state/recovery binaries and package boundary.
- [Sovereign Boot #9](https://github.com/Luminous-Dynamics/sovereign-boot/issues/9): disposable QEMU DRM/boot-to-display-manager qualification.
- [SRQ #91](https://github.com/Luminous-Dynamics/luminous-platform/issues/91): atomic challenge consumption with promotion.
- [SRQ #93](https://github.com/Luminous-Dynamics/luminous-platform/issues/93): authenticated workflow/artifact provenance.
- [SRQ #98](https://github.com/Luminous-Dynamics/luminous-platform/issues/98): durable promotion versus independently observed booted generation.

## Qualification boundary

This document defines the next interface; it does not assert that an adapter, signature, TPM quote, QEMU run or physical experiment exists. The current model validates record structure only. SRQ remains `NOT_QUALIFIED` until the executable path and each scenario's declared evidence tier are satisfied.


## D0 reference collector now implemented (2026-10-21)

A dependency-free Rust command is available at `tools/srq-boot-observation-adapter/` and can be built/tested independently of the unfinished monorepo binaries:

```bash
cargo test --manifest-path tools/srq-boot-observation-adapter/Cargo.toml --locked
cargo run --manifest-path tools/srq-boot-observation-adapter/Cargo.toml --locked -- --help
sudo cargo run --manifest-path tools/srq-boot-observation-adapter/Cargo.toml --locked
```

The default command samples the Linux boot ID and reads the symlink targets of `/run/booted-system` and `/run/current-system`. It validates the boot-ID shape and canonical Nix store path shape, returns a stable reason-code list, and blocks when the two deployment IDs diverge. `--root` allows tests to use an isolated fixture tree without reading or modifying the host runtime.

The output schema labels itself `D0-local-diagnostic`, always sets `authenticity_status=NOT_AUTHENTICATED`, leaves promotion binding and measurement as not provided, and calls the capture timestamp untrusted. Even an internally consistent observation is named `OBSERVED_CONSISTENT_DIAGNOSTIC`, never PASS or QUALIFIED. It deliberately does not infer an artifact digest from a path and does not authenticate the host.

The SRQ workflow now runs its standalone tests and binds the test log into the signed evidence-bundle inventory as a supplemental `model / abstract_model_only` observation. That means the test result may be authenticated as an artifact if/when the main-branch attestation path is exercised; it does **not** turn the D0 output into trusted target evidence. The model/tool must still be integrated with a challenge-bound authenticated response (D1), or TPM/hardware-backed measurements (D2), before security-gate use.

This collector is intentionally a narrow bridge, not a substitute for the missing `spore-recovery-linux` / `spore-boot-state` package or Sovereign Boot's disposable QEMU tests. It reads symlink targets, not contents of the deployed artifact, so Nix store path identity remains separate from the authorized artifact and effective-policy digests.
