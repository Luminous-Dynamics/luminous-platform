# Sovereign Home Systems: measurable benchmarks and release gates

**Status: proposed targets, not measured results.** The numerical values below are engineering goals to test, not claims about the current RFC or any hardware. We will publish measured values only with the exact source SHA, host/profile, input corpus, method, raw evidence and run identity.

## Paradigm shifts expressed as benchmarks

1. **From “it works” to zero safety-invariant violations.** A single unauthorized attempt or duplicate dispatch after an unresolved effect is a release blocker. Performance improvements never override authorization, lease or durable-journal requirements.
2. **From unit tests to an adversarial system model.** Differential trace replay, critical mutation testing, exhaustive input-state enumeration, bounded event-history replay and eventually TLA+/TLC model checking complement each other. The Rust suite enumerates 387,072 declared gate-input combinations and all 1,000,000 length-six histories over ten event types, including revocation, adapter drift, lease loss, crash/restart, provider acceptance, outcome observation and time advancement. These are finite, bounded models—not proof over arbitrary event histories. Model checking also depends on the specification's state space and assumptions.
3. **From cloud availability to measured local continuity.** Essential local functions must be probed under partitions, while actions needing current authority fail closed when freshness cannot be established.
4. **From “backup exists” to owner-proved recovery.** Measure clean-room recovery duration and loss of acknowledged configuration; check the restored state and its checksums, not merely boot.
5. **From privacy policy to leakage tests.** Seed recognizable secrets/household markers and require zero copies in public evidence.
6. **From vendor lock-in to migration evidence.** Export/import every promised data class on a clean system, with an explicit plan for re-provisioning secrets.
7. **From “supports updates” to rollback-tested lifecycle support.** Inject update failure at the specified boundaries and verify return to a previously verified state.

The machine-readable catalogue is [`targets-v1.json`](../benchmarks/home-systems/targets-v1.json), validated by [`home-systems-benchmark-targets.schema.json`](../schemas/home-systems-benchmark-targets.schema.json). Every item currently says `not_measured` or `blocked_runtime_not_ready`; the catalogue deliberately contains thresholds and methods, not fabricated score values.

## Proposed acceptance targets

| Area | Target | Gate? |
|---|---|---|
| Reference corpus | Evaluate all 11 current decision snapshots; zero unexpected typed decisions | Yes |
| Safety mutations | Zero surviving critical mutants across the named authorization/retry/reconciliation predicates, including rejection of empty request fingerprints | Yes |
| Exhaustive gate states | Enumerate 387,072 deterministic combinations of freshness, guards, bindings, journal identity/state, risk and attempt budget; zero invariant violations | Yes |
| Temporal event histories | Enumerate all 1,000,000 six-event histories over ten event types; zero invariant violations in the declared state model | Yes, bounded scope only |
| Cross-repo conformance | Zero decision divergences across shared vectors in the actual Hearth 004E runtime and Luminous adapter | Yes; blocked until runtime harness |
| HA subscription correlation | Zero accepted events with a mismatched subscription ID or stale session epoch | Yes; blocked until runtime harness |
| HA snapshot-to-stream continuity | Zero lost or out-of-order events across subscription, snapshot, buffered replay, and reconnect interleavings | Yes; blocked until native transport |
| HA allowlist projection | Zero non-allowlisted entities or unapproved fields in projected/cache state across compact added/changed/removed frames | Yes; blocked until native transport |
| Ambiguous recovery | Zero new dispatches while prior outcome is `Dispatching`, `ProviderAccepted` or `OutcomeUnknown`; inject faults at six defined boundaries | Yes |
| Local gate latency | p95 ≤ 10 ms and separately report p50/p99 on a Raspberry Pi 5 and low-power x86-64 host at 100 decisions/s | Measurement target, not current SLO |
| Durable journal | p99 ≤ 25 ms with durability sync enabled on a named SSD-backed profile | Measurement target; never relax durability to hit it |
| Offline continuity | At least 99.9% success for a declared essential-local probe set over a 24-hour partition | Product-profile target |
| Recovery | Restore within 30 minutes; no more than 5 minutes of acknowledged configuration loss | Product-profile target |
| Evidence privacy | Zero seeded secret/raw-household markers in all public/shareable evidence | Yes |
| Portability | 100% of advertised exportable data classes survive export/import comparison; secrets are re-provisioned | Product-profile target |
| Update recovery | 100% rollback success across every declared update-failure mode | Yes for shipped update path |

## How to benchmark correctly

- Record exact code SHA, corpus/schema digests, OS/kernel, CPU, memory, storage/filesystem, compiler/toolchain and build flags.
- Publish p50, p95 and p99 rather than a single average. Separate cold start, warm decision, journal sync, provider round-trip and observed postcondition.
- Run at least 30 timing batches for local gate latency and 10,000 journal commits for durability measurements. Preserve raw samples, including failures/timeouts.
- The initial Rust gate-state sweep is exhaustive over its declared Cartesian product, not over all event histories. Preserve every counterexample as a regression vector; follow with sequence generation and model checking.
- Use mutation testing to prove a check is exercised: intentionally disable each critical guard and require a specific regression test to fail. Include a mutant that permits an empty/whitespace request fingerprint and require the gate-level regression test to fail; schema validation is not a substitute for checking a privileged boundary. A test suite that remains green under a revoked-authority mutant is not acceptable.
- Run one workload on the pure Rust reference model and separately on the integrated edge runtime. Do not present reference-function speed as end-to-end household command latency.
- Prefer Home Assistant's native `subscribe_entities` stream, which supplies an initial compact snapshot and then incremental changes over one subscription. Pass only the exact validated entity allowlist, bind every event to the subscription ID, apply the initial snapshot before later deltas on a serialized receive loop, and reject malformed or disallowed data. For any future provider that exposes separate snapshot and event subscriptions, require subscribe-first plus buffered replay. Benchmark event loss/order and stale-session rejection separately; synthetic protocol-state results are not live-transport evidence. See the [official Home Assistant WebSocket API](https://developers.home-assistant.io/docs/api/websocket/). Project added/changed/removed compact frames into a minimal state cache containing only the allowlisted primary state, unit, and device class. Require transactional update application: malformed allowlisted data leaves the last valid cache untouched. Measure privacy leakage and continuity as separate release gates.
- Report availability separately for manual controls, local automation, remote control, cognition and cloud-dependent features. A single “uptime” metric hides degradation that matters to owners.
- Physical-stop response time, force/torque limits, electrical safety, thermal cutoffs and hazardous-state bounds require a hazard analysis and named hardware profile. They should not receive a universal numeric threshold copied from this generic RFC.

## Research alignment and boundary

NIST IR 8259 Rev. 1 broadens IoT manufacturer cybersecurity activities across the full product lifecycle, including maintenance, support and end-of-life ([NIST publication](https://csrc.nist.gov/pubs/ir/8259/r1/final)). The OWASP IoT Security Testing Guide provides a reusable methodology and catalog of device/security test cases ([OWASP ISTG](https://owasp.github.io/www-project-iot-security-testing-guide/)). TLC can explore every reachable state in a configured finite TLA+ model, but a completed model check applies only to that model's variables, bounds and assumptions—not automatically to implementation correctness ([TLA+ guidance](https://docs.tlapl.us/creating%3Astart)). Home Assistant's REST docs explain why changing an entity-state representation is not the same as communicating with a device, and why returned state changes may be caused by other events ([REST API](https://developers.home-assistant.io/docs/api/rest/)).

## Promotion ladder

- **H0 contract:** schema, corpus, typed decisions, negative controls.
- **H1 deterministic Rust:** exact-head tests, seeded generation and critical-mutant suite.
- **H2 edge runtime:** actual Hearth 004E path, durable journal and executor lease under replay/fault injection.
- **H3 provider simulation:** distinct Home Assistant observation/service-effect profiles and Matter test-profile coverage.
- **H4 bench/hardware:** independent postcondition observation, physical stop and recovery on one named device/profile.
- **H5 scope-specific review:** independent review and lifecycle evidence for each advertised capability. One profile's evidence does not qualify every device or action.

No gate is measured as passed by this document. CI queue state remains an evidence status, not a benchmark result.
