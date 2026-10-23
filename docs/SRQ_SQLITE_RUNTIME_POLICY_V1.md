# SRQ shared SQLite runtime release policy v1

**Policy ID:** `SRQ-SQLITE-RUNTIME-POLICY-2026-10`  
**Status:** proposed runtime policy, enforced by the two SRQ Rust stores; CI execution on the current head is pending.

## Why this exists

The D1 observation store and policy-promotion store must not silently accept different database-engine release sets. SQLite's WAL-reset corruption bug affected WAL-mode databases when separate connections concurrently write/checkpoint; the documented fix is in 3.51.3, with backports to 3.44.6 and 3.50.7. Separately, SQLite 3.53.0 through 3.53.3 received follow-up fixes, and 3.54.0 had a heap-buffer-overflow on some 32-bit builds, fixed in 3.54.1. Upstream's release history is the source of these version decisions: [WAL-reset bug notes](https://www.sqlite.org/wal.html#walresetbug), [release history](https://sqlite.org/changes.html), and [3.54.1 release notice](https://www.sqlite.org/releaselog/3_54_1.html).

A minimum-version floor alone was insufficient: it would have allowed known-buggy SQLite 3.53.0–3.53.3 and 3.54.0. The shared policy therefore rejects known bad lines and fails closed on new, unreviewed minor/major versions.

## Current allow policy

| Release line | Decision | Reason |
|---|---|---|
| 3.44.6 and later patch releases within 3.44 | Allow | WAL-reset fix backport |
| 3.50.7 and later patch releases within 3.50 | Allow | WAL-reset fix backport |
| 3.51.3 and later patch releases within 3.51 | Allow | Upstream WAL-reset fix |
| 3.52.x | Reject | Withdrawn release line |
| 3.53.0–3.53.3 | Reject | Follow-up defects fixed in 3.53.4 |
| 3.53.4 and later patch releases within 3.53 | Allow | Follow-up fixes applied |
| 3.54.0 | Reject | Heap-buffer-overflow on some 32-bit builds |
| 3.54.1 and later patch releases within 3.54 | Allow | 3.54.0 issue fixed |
| Any new minor/major line, e.g. 3.55.x or 4.x | Reject until reviewed | Avoid implicitly trusting an unassessed new line |

This is a known-defect policy, not a guarantee that any allowed version is defect-free. Update the policy only after review of upstream release notes/security fixes and the supported platform matrix.

## One implementation, two enforcement points

The pure Rust crate `tools/srq-sqlite-runtime-policy` owns the parser and decision function. Both `tools/srq-d1-sqlite-store` and `tools/srq-policy-promotion-store` call the same function. The parser accepts only SQLite's canonical `major.minor.patch` string: suffixes, missing components, whitespace, leading signs, zero-padding and extra components fail closed.

The critical store crates do not enable rusqlite's bundled SQLite source. CI builds upstream SQLite 3.54.1 from a SHA3-256-pinned archive, checks the exact `sqlite_source_id()`, and exports the installation's `pkg-config` and shared-library paths before compiling the Rust stores. Both runtime tests compare the linked library version/source ID against those expected values. This guards against testing one engine while shipping another.

## Required test/evidence contract

The shared-policy unit test must cover:
- patched backports and accepted releases;
- withdrawn and known-buggy release lines;
- unreviewed future minor/major lines;
- malformed/noncanonical version strings.

The signed evidence-bundle manifest includes the shared policy test log as an `abstract_model_only` supplemental observation and the crate lockfile in `dependency_lock_sha256.srq_sqlite_runtime_policy`. The archive and produced-bundle validators must reject a missing/extra policy file, altered bytes, a false policy-log digest or a false lockfile digest. A matching digest proves the bytes match the manifest; it does not by itself prove the test assertions are sufficient.

## Qualification boundary

Passing this policy check supports only that the linked SQLite runtime matched the release policy and exact pinned CI source for the tested build. It does not establish crash consistency, production-store correctness, kernel-crash recovery, power-loss durability, cross-host locking or QEMU/physical qualification. Those require their own evidence and remain separate release gates.
