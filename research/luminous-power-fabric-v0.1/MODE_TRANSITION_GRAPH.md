# Power Fabric operating-mode transition graph — v0.1

**Tracking:** [Issue #454 — transition graph and independent protection oracles](https://github.com/Luminous-Dynamics/luminous-platform/issues/454)  
**Functional coverage:** [PR #446 — controller HIL / commissioning / periodic matrix](https://github.com/Luminous-Dynamics/luminous-platform/pull/446)  
**HIL boundary passport:** [PR #453](https://github.com/Luminous-Dynamics/luminous-platform/pull/453)

**Status:** synthetic offline transition-coverage corpus and typed validator only. No controller, HIL, breaker, inverter or protection device has been operated or tested.

## 1. Why a state graph is necessary

A mode label such as "islanded" does not describe the topology, PCC breaker state, grid synchronism, grid-forming source, protection status, communications freshness, critical service or reserve. A controller can report the right label while the physical plant is in a different state.

The benchmark therefore treats a transition as a guarded, authority-owned state change with independent postconditions. Command acknowledgement is never treated as proof of physical effect.

## 2. Standards and research landscape

IEC TS 62898-3-5:2026 is the published IEC test-method anchor for HIL, commissioning and periodic testing of microgrid monitoring/control/energy-management systems. Its published contents separately list operation-mode-transition HIL tests, device switching, islanding detection and black start.

IEEE 2030.8-2018 remains a published testing reference; P2030.8 is an active revision project. IEEE 2030.7-2017 remains the published controller-function reference; P2030.7 is an active revision project. These are not blanket product certifications.

**Important date correction:** a C-HIL paper titled *Controller hardware-in-the-loop testing of a hybrid control architecture for multi-feeder microgrids with heterogeneous generation* is publicly indexed with a publication date of **April 2027** and describes 38 modes on a three-feeder microgrid. April 2027 is after the current research snapshot date (13 November 2026). Treat it as a forthcoming/prepublication research pointer, not as an already-published 2026 SOTA result or a verified performance baseline. Its 38-mode design motivates a future scale target; this corpus does not claim 38-mode coverage.

Earlier published comparators include the NLR/IEEE controller-HIL evaluation of a commercial microgrid management system with multiple grid-forming inverters and DNP3 (2024 conference work, published in proceedings in 2025), and NLR's Borrego Springs PHIL work on grid-forming battery inverter transitions/load steps (2025). Those are legitimate HIL/PHIL references, but not direct apples-to-apples baselines for this synthetic offline validator.

Sources:
- IEC TS 62898-3-5:2026: https://webstore.iec.ch/en/publication/84175
- IEEE P2030.8: https://standards.ieee.org/ieee/2030.8/12543/
- NLR / IEEE multi-GFM C-HIL paper: https://doi.org/10.1109/ECCE55643.2024.10861183
- NLR Borrego Springs PHIL results: https://doi.org/10.2172/3014992
- Forthcoming 38-mode C-HIL article listing: https://doi.org/10.1016/j.epsr.2026.113933

## 3. Architecture: the transition contract

Each state binds topology and controller configuration digests plus the declared PCC breaker, grid, synchronism, grid-forming, protection, communications, service and reserve status.

Each transition binds:
- source state set and destination state;
- whether the transition is permitted;
- requesting actor separately from authorization owner;
- protection-interlock requirement;
- typed guard references and independently observed postconditions;
- bounded failure disposition;
- explicit recovery-transition reference, where applicable;
- reason for every forbidden transition.

A transition test binds positive, negative or recovery case to the transition ID, satisfied/violated guards, observed postconditions and evidence class.

## 4. Authority separation

| Role | May do | Must not do |
|---|---|---|
| Supervisory MEMS | Propose schedules and request a transition | Independently operate switchgear, bypass protection or certify physical effect |
| Local controller | Execute approved local sequence under current observations/interlocks | Ignore independent protection or infer physical state from an acknowledgement |
| Independent protection | Trip and own independent permissives/interlocks | Be overridden by the optimization layer |
| Operator | Request/acknowledge permitted operations | Override protection or stale/unknown-state holds |
| Maintenance authority | Authorize inspection/maintenance workflow | Declare a trip cleared without independent protection evidence |

Symthaea may propose scenarios, forecast and advise. It receives no operational or protection authority from this benchmark.

## 5. Initial synthetic coverage

The fixture includes eight states, 20 guard definitions and 17 transition records: 14 permitted and three explicitly forbidden. There are 76 synthetic test cases:
- positive coverage for every permitted transition;
- 60 single-guard negative mutation cases (one per transition/guard pair), with every other guard satisfied;
- recovery-path tests for the two communications-loss transitions.

The transition families cover planned islanding, unexpected grid loss, reconnection and synchronism, black start/failed start, protection trip/inspection hold, communications loss, reconciliation, direct supervisory switchgear requests, reconnection without sync-check and protection-lockout bypass attempts.

The typed Rust validator checks unique IDs, state/digest enums and selected state invariants, transition references, authority ownership, an independent-protection guard on every protection-interlocked transition, forbidden reasons, guard references, recovery-edge continuity, evidence class, complete positive/negative/recovery test coverage, isolated failure of every declared guard (all other guards satisfied), and postcondition evidence presence. Recovery test guards and postconditions are evaluated against the recovery edge itself, not the initiating failed edge.

This is a **coverage-structure validator**. It does not evaluate a real controller's guard logic, prove the transitions safe, evaluate grid dynamics or authorize actions.

## 6. Competitive benchmark programme

Targets below are research goals, not achieved results or standards acceptance limits:

- 100% positive coverage for all permitted transition IDs.
- 100% negative coverage for all permitted and forbidden transition IDs, with explicit violated guard IDs.
- 100% recovery coverage for transitions that declare a recovery route.
- Zero accepted forbidden transitions in a future independently executed deterministic corpus.
- Zero transition completion based only on command acknowledgement; require independently observed postconditions.
- 100% exact topology/controller/firmware/protection configuration binding.
- Scale to 38 or more topology modes only after defining a multi-feeder topology model and a full transition relation; do not pad the corpus with redundant labels.
- Report transition false accepts/rejects, detection latency, decision latency, transition duration, time-to-stable, interruption of critical service, reserve trajectory, recovery duration and requalification separately.
- Stratify results by controller hardware, firmware, topology, DER mix, communications profile and HIL/PHIL/field boundary.

## 7. Validation and claim ceiling

Files:
- `schemas/mode-transition-graph-v0.1.schema.json`
- `schemas/mode-transition-graph.synthetic.json`
- `tools/measurement-audit/src/mode_transition_corpus.rs`

Claim ceiling: **SYNTHETIC_TRANSITION_COVERAGE_ONLY_NOT_HIL_SAFETY_OR_CONFORMITY**.

The JSON fixtures can be structurally/semantically checked offline; the authored Rust tests require a Rust toolchain to compile/run. This validator does not operate hardware, model power flow, test protection, prove a safety case, demonstrate field performance or establish IEC/IEEE conformity. All energized testing remains with qualified owners/laboratories under approved safety procedures.
