# SRQ-1 real process-kill transaction model

**Status:** proposal / CI model; not production-store qualification. The model is intentionally supplemental and cannot satisfy a QEMU or physical release gate.

## Why this is a distinct experiment

The existing snapshot models injected an early return, then reopened state. This model launches a separate worker process, blocks at a named persistence boundary, and has the parent send Linux \`SIGKILL\` to the worker before opening the same state directory from a fresh process. That tests a genuine process termination, rather than returning from the writer's call stack.

The modeled state couples all of these fields in one immutable snapshot:

- active policy generation;
- policy digest;
- the challenge consumed for promotion;
- the decision ID authorizing that promotion.

The reader accepts only a complete generation-zero snapshot or a complete generation-one snapshot. Partial snapshots, duplicate fields, and mixed combinations fail closed. Recovery selects the active snapshot and removes abandoned temporary transaction files; an unrenamed temporary file never becomes current state.

## Fault points exercised

The parent kills a separate worker at each point:

1. \`after_temp_create\`: the transaction temp file exists but has no snapshot payload.
2. \`during_temp_write\`: a prefix of the next snapshot has been written and synced; the active record must remain generation zero.
3. \`after_temp_sync\`: the full next snapshot is written and synced to a temporary file but has not been renamed active; generation zero remains authoritative.
4. \`after_rename\`: the full next snapshot has atomically replaced the active file; restart must observe generation one and reject replay of the same challenge.
5. \`after_dir_sync\`: the replacement and parent directory sync have completed; restart must observe generation one and reject replay.

The harness also rejects incomplete or mixed snapshots and verifies that restart cleans leftover temporary records without treating them as current. It uses only the Rust standard library and the Linux \`kill\` utility; no external Rust dependencies are introduced.

## Baseline and what would establish progress

OSTree already documents old-or-new deployment outcomes under crash/power interruption, so an atomic pointer or file rename is not a novelty or superiority claim. SQLite documents transactional atomic commit and has dedicated crash-test machinery that varies points of interrupted writes and verifies post-recovery database consistency. SRQ should borrow this fault-injection mindset rather than invent another transaction protocol. See [OSTree atomic upgrades](https://github.com/ostreedev/ostree/blob/main/docs/atomic-upgrades.md) and [SQLite atomic commit](https://www.sqlite.org/atomiccommit.html).

A next production-shaped iteration must test the actual policy store adapter and deployment transaction. The fair comparison should use the same fault schedule and outcome oracle for the chosen baseline and Luminous path, report every checkpoint separately, and include p50/p95/p99 time-to-recover, false acceptance, false rejection, orphan cleanup, and data-loss outcomes. Do not combine a simple file model and a production database into one performance score.

## Qualification ceiling

Passing this experiment supports only this narrow statement: on the tested Linux host filesystem, the model recovered to the complete old snapshot for pre-rename kills and the complete new snapshot for post-rename kills, after actual worker-process termination.

It does **not** test a kernel panic, VM reset, storage-controller write reordering, sudden power loss, filesystem corruption, device cache durability, cross-host concurrency, the production updater, the production challenge ledger, TUF verification, cryptographic signer identity, trusted time, bootloader behavior, QEMU or physical hardware. A live-kernel process kill does not reproduce loss of the kernel page cache. The model is deliberately marked \`model\` / \`abstract_model_only\` and remains supplemental evidence, not a new passing SRQ release gate.
