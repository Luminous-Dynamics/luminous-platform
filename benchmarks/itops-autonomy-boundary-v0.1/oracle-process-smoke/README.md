# ITOPS oracle process smoke harness

This crate is a deliberately narrow implementation slice for
[Luminous Platform issue #213](https://github.com/Luminous-Dynamics/luminous-platform/issues/213).
It tests whether the participant-effect ledger and service observer can be
exercised through separate child-process protocols rather than only through
in-process models.

## Run

From this directory:

\`\`\`sh
cargo test
cargo run --release
\`\`\`

The default command starts two child processes, sends them deterministic
newline-delimited JSON requests, and emits one JSON smoke receipt. The
participant child handles a dispatched operation whose response is lost,
blocks a second attempt while the first outcome is unknown, accepts explicit
reconciliation, and then permits a new attempt. The observer child maps stale
and future-dated health samples to \`unknown\`.

The integration test runs the executable twice and requires identical
canonical event digests. Process IDs are checked for distinctness but are
excluded from the deterministic event digest.

## What this demonstrates

- The participant ledger and observer communicate over independent
  stdin/stdout channels in distinct child processes.
- An unresolved outcome locks new dispatches; the rejected attempt is not
  recorded as a dispatched attempt.
- A new dispatch is accepted only after explicit reconciliation.
- Stale and future-dated service observations fail closed to \`unknown\`.
- The event stream is reproducible for the same fixed scenario and logical
  clock.

## Explicit limits

This is **not** the complete ITOPS runner and does not execute a model agent.
The runner drives a deterministic protocol script. The children are separate
processes but run under the same OS user, executable, parent supervisor, and
host trust boundary. There is no separate account, container, VM, credential
authority, signed oracle attestation, network isolation proof, or protection
against a hostile same-user process. Process IDs and domain labels are
diagnostic metadata, not cryptographic proof of independence.

The smoke receipt is not an instance of \`decision-result.schema.json\` and is
not a benchmark episode result. It does not qualify the 23-case corpus, execute
the full workflow DAGs, measure an agent, or establish a production safety
rate. The next steps are an untrusted agent adapter, per-episode environment
reset, externally administered oracle services, signed receipts, schema-bound
result emission through the qualification core, and adversarial integration
tests.

No Rust build or test result is claimed until the exact branch head has been
compiled and its tests executed.
