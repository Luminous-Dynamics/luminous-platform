# SRQ quorum-witness model v1

**Status:** dependency-free protocol model. This is not a distributed witness service and does not provide cryptographic authenticity, durable monotonic storage, a trusted clock, TPM NV, or release qualification.

## Paradigm shift: use quorum intersection, not one rollback domain

A single local SQLite database is vulnerable to whole-store rollback unless its monotonic root lives outside that rollback domain. A single remote witness gives an attacker a single availability and integrity target. The next protocol step is a quorum of independent witnesses.

For Byzantine tolerance f, require n >= 3f+1 members and q = n-f distinct votes for a checkpoint. At the minimum n=3f+1, q equals 2f+1. For larger committees the model keeps q=n-f so the intersection bound remains at least f+1. With n=4, f=1, q=3, any two quorums of size 3 overlap in at least 2 witnesses. Under the explicit assumption that at most one witness is Byzantine and an honest witness never signs two different checkpoints for the same target/sequence, a conflicting pair of certificates cannot both form: at least one honest member lies in the intersection and refuses the second vote.

This is a safety property, not an availability guarantee. A quorum can be unavailable; the protocol must fail closed rather than promote locally when it cannot obtain the required certificate.

## Checkpoint binding

Every vote binds the exact target ID, sequence, previous checkpoint digest, checkpoint digest, previous and next policy generation, decision ID, immutable deployment identity, and issued/expiry times.

The checkpoint digest must itself be computed by a canonical encoding over every field. Each member must persist its per-target/per-sequence vote before returning a signature. This model compares structured checkpoints directly rather than implementing canonical encoding or signatures.

## What is tested

The Rust model checks:
- invalid membership settings and the n >= 3f+1 / q = n-f threshold rule, including a larger-committee regression check;
- insufficient quorums, duplicate and untrusted voter IDs, mismatched votes;
- sequence/predecessor/target/freshness failures;
- idempotent honest votes and per-height non-equivocation;
- inability to form two conflicting 3-of-4 certificates with one equivocator and three honest modeled members;
- quorum-intersection lower bound for every pair of 3-of-4 subsets;
- per-target separation and rejection of a conflicting certificate after local commit.

## Benchmark and production path

This is an executable protocol-model benchmark; it does not claim that a distributed implementation exists. The production follow-on requires:

1. A specified signature suite over canonical checkpoint bytes, with key rotation, revocation and threshold policy.
2. An append-only durable journal for each witness, persisting a vote before signing and rejecting conflicting target/sequence checkpoints after restart.
3. Fault injection that rolls back one witness disk, kills witnesses before/after journal fsync, isolates network partitions, drops/delays/replays messages, and attempts split views.
4. At most f Byzantine/compromised witness keys, with separate threat review for collusion, common-mode infrastructure and recovery-key compromise.
5. Separate reporting of safety violations, false rejects, quorum availability, decision latency p50/p95/p99, recovery latency, and evidence size.

Initial targets for a future service: zero conflicting certificates in 100,000 deterministic message/replay/partition mutations, exactly one decision across 64 concurrent callers, and zero promotion when fewer than q authenticated witness votes are available. These are proposed targets, not measured results.

## Explicit limitation

The Vote struct here is not a signature. The certificate verifier validates membership, uniqueness and binding only; production code must authenticate every vote before invoking equivalent validation. HonestWitness state is in memory and can be rolled back together with local state. These tests establish the model contract under its assumptions, not Byzantine fault tolerance of a live network.
