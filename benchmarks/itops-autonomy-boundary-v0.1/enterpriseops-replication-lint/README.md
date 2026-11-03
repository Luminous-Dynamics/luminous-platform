# EnterpriseOps-Gym parity readiness auditor

This small Rust crate validates the versioned parity manifest and makes
unresolved reproduction inputs machine-readable.

From this directory:

    cargo test
    cargo run --release

The crate uses a pinned Draft 2020-12 validator with format assertions.
It reports the exact ServiceNow repository source commit and published
reference row, then lists every unresolved dataset, environment, scorer,
system-under-test and reference-arm pin.

The example is intentionally `planning_unpinned`. It must not become
`ready_to_execute` until:
- the Hugging Face dataset revision and exact task/domain manifests are pinned;
- every container image and database snapshot has a content digest;
- the native scorer path, source commit and known-answer fixture are pinned;
- the candidate agent profile (model, prompt, tools, orchestrator, memory,
  inference settings) is immutable and hashed;
- the published reference model/fallback configuration is reproduced and
  its profile artifact and digest are published;
- run command, analysis code and raw output receipt digests are available.

The reference reproduction requirement is intentional: a Luminous run merely
scored on the benchmark can be reported as a new measurement, but it must not
be presented as a like-for-like reproduction of the published leader unless
the corresponding reference arm and execution conditions are also pinned.

No cargo build/test, schema validator run, container launch, model inference,
or benchmark has been executed in the authoring environment.
