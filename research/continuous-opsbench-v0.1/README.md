# Continuous OpsBench v0.1

Research-only deterministic evaluator for issue #188. This is the continuous-operation companion to episodic OpsBench, not a replacement for it.

## What is evaluated

The evaluator consumes separate planes:
- evaluator-owned fault truth and active/revert intervals;
- agent-produced incident reports and proposed/executed intervention references;
- participant-owned attempt/effect receipts;
- independent service observations bound to a frozen offered-demand workload profile;
- a no-action twin using the same scenario schedule and workload identity.

It derives per-fault correlation, exact service/artifact localization, repair before evaluator cleanup, sustained workload-conditioned recovery, false repair claims, intervention safety, duplicate committed effects, unauthorized committed effects, and demand-weighted degraded request-time.

Every injected fault remains in the denominator, including faults that disappear before an agent invocation. No-action observations are compared only when bound to the same scenario and workload profile. Metrics remain separate; there is no compensatory composite score.

## Run

```sh
cargo test --locked
```

No external crates or live infrastructure are used.

## Claim ceiling

This is a typed offline evaluator over supplied records. It does not generate Kubernetes faults, run an agent, authenticate publishers, verify cryptographic signatures, prove the independence of declared observer sources, or establish production representativeness. A committed participant receipt is trusted as participant-owned input. A production adapter must bind exact episode/run/arm and evidence digests, use a source-authenticated participant ledger and observer, and integrate through Platform Saga and the existing effect broker. Nixward remains the sole host transaction and recovery authority.

The synthetic fixture is not a benchmark result and cannot support SOTA or production-safety claims.


## Temporal topology and causal-model freshness v0.1

The additive `src/temporal.rs` evaluator separates evaluator-owned topology truth from agent-visible topology observations. It binds causal hypotheses to exact topology epochs/digests and reports stale-model action attempts, stale committed effects, stale or missing observations, unknown truth, sequence gaps, and unknown edges. Event time, ingest time, processing time, and valid time remain separate. See `TEMPORAL_TOPOLOGY_V0_1.md`, `schemas/temporal-topology-bundle.schema.json`, and the transition fixture.

This is a structural synthetic evaluator. It does not authenticate topology sources or enforce action preconditions at the participant boundary. A production adapter must bind the epoch into a conditional participant operation to address time-of-check/time-of-use races.


## Closed-loop operational stability v0.1

The additive \`src/stability.rs\` module provides a deterministic discrete-time synthetic plant and evaluator for issue #237. It includes delayed actuation, delayed/noisy observations with explicit source ticks, critical/bulk workload priority, retry amplification, API/database capacity mismatch, action churn/reversals, request-time SLO violation integral, queue/latency maxima, post-fault settling, and a reserved no-intervention recovery tail. Settling time is the first post-fault tick after which every remaining sample stays healthy for the rest of the run; a temporary healthy streak followed by a regression does not count as settlement.

Controller actions are supplied as a trace; this module does not implement an agent, scheduler, autoscaler or policy gate. The evaluator reports critical and bulk failures separately and preserves the plant trace separately from delayed observations.

**Claim ceiling:** this is a small synthetic plant for deterministic scenario construction and metric validation. It does not establish production representativeness, formal/Lyapunov stability, or convergence for arbitrary systems. Stability thresholds must be calibrated and preregistered against a pinned plant/workload before candidate-agent evaluation. The initial synthetic tests are conformance tests, not agent benchmark results.


## Competing controller actions

The stability reference applies an evaluator-owned precedence rule to simultaneous mutations of the same resource in the same effect tick: Operator > Runbook > Agent > Autoscaler. Equal-precedence proposals are resolved by stable action ID order. Every proposal gets an explicit `ActionOutcome` (Applied or Overridden with the winning action ID); conflict and override counts are reported separately from action churn. The resolver is input-order independent, so shuffling an otherwise identical action log cannot change the simulated plant. This is a benchmark arbitration fixture, not a recommended universal production policy.


The workload model reports critical and bulk failure rates, latency envelopes, and request-time SLO violation integrals independently. Aggregate SLO compliance and persistent recovery require **both** workload classes to pass their declared failure-rate and latency limits; bulk or critical degradation cannot be hidden by a healthy aggregate. The aggregate request-time violation integral is the sum of the two class-specific integrals.


## Observation-driven closed-loop controller runner

The additive `run_stability_closed_loop` function calls a `StabilityController` once per actionable tick. Its only input is the current delayed/noisy `PlantObservation`; evaluator fault truth and actual plant state are not passed through the policy interface. The runner records the observation source tick, measurement values, proposed action IDs, issue time and applied/overridden outcomes. It stops invoking the controller early enough that every proposed actuation lands before the reserved no-intervention tail.

The default controller runner is incremental **O(T + A log A)**, where T is the number of plant ticks and A is the number of proposed actions. It advances the plant once per tick and feeds only the observation delivered at that tick to each policy. A separate **O(T²)** prefix-replay runner remains as an independent orchestration reference. Before returning, the streaming path replays the complete action trace through the offline scorer and requires exact equality of plant samples, delayed/noisy observations and action outcomes. A divergence is a hard error, not a score.

Discrete-time semantics are causal: a controller receives its observation at tick t, proposes an action, and the earliest possible effect is the next plant transition. Thus zero additional actuation delay means an effect at t+1; it never changes the sample that generated the decision. The no-intervention tail gate uses the same due-tick function, so no delayed action can enter the tail.

Each ControllerDecision preserves the complete observation and every proposed ControlAction, in addition to its decision tick, source tick and action IDs. validate_closed_loop_evidence checks that the observation exactly matches the independent scorer's observation at that decision tick, all actions are bound to that controller/tick/source observation, the decision action list is complete, and concatenating those proposals exactly reproduces the scored action trace. It replays the scorer and rejects tampered observations, substituted operations, orphan actions and missing action rows.

The controller trait is an in-process policy seam, not a live LLM integration. Model/provider, prompt, tool, memory, policy and action budgets still need to be supplied by the external runner manifest and hashed into each arm.


The controller harness also accepts multiple policies in one run. Each declares a stable unique controller ID and controller class; policies are sorted by ID before each tick and receive the same permitted observation. Their actions are then arbitrated by the evaluator-owned precedence rule. The resulting action and decision traces are invariant to the order in which the caller supplies the controller slice. This supports paired competition between an agent policy, deterministic runbook, and autoscaler baseline without giving any policy access to evaluator truth.


## Deterministic autoscaling baselines

The reference plant now includes three deterministic policies:

- `BangBangCapacityController`: one threshold switches between normal and elevated capacity.
- `HysteresisCapacityController`: separate scale-up and scale-down thresholds create a deadband.
- `PersistentHysteresisCapacityController`: queue-aware debounce; overload must persist for N delivered observations before scale-up, while both failure and queue signals must remain healthy for M observations before scale-down. An intermediate observation breaks the streak.

All policies consume the same `PlantObservation` stream and share the same action budget, causal actuation delay, and deterministic arbitration. The paired noise fixture makes bang-bang repeatedly reverse its setpoint, while two-threshold hysteresis scales once and holds. The persistence-filtered controller additionally refuses to act on alternating single-sample spikes. Tests assert that all these differences are evaluated against the same plant truth and SLO contract; these are synthetic policy comparisons, not production autoscaling results.


## Deterministic watch-to-probe episode v0.1

The additive `src/watch_probe.rs` module provides a small scripted reference episode that composes:
- a bounded watch bound to tenant, target, purpose, source and expiry;
- an authority snapshot rechecked at event delivery and probe admission;
- source-sequenced events with duplicate, event-ID collision, gap, future-delivery, watch-start and identity handling;
- a policy that proposes only an existing catalog entry ID;
- exact-scope, read-only probe admission;
- a collector-owned observation (which may come from a different source than the watch event) and a separate impact observation;
- observation/impact receipts bound to exact probe, operation, tenant, target, purpose and time window, with observation freshness constrained to after the triggering wake; explicit unknown for a collector timeout with continuing work or a missing impact record;
- evaluator-only pre/post decision loss, scored after evidence acquisition.

The module includes eighteen deterministic unit tests for a valid positive episode, revocation during dormancy, duplicate delivery, sequence gaps, write-capable probe rejection, unknown impact, independence-domain collision, timeout with continuing work, quiet expiry/no-match behavior, and wrong-tenant events.

**Claim ceiling:** this is a one-probe, in-process scripted conformance reference. It does not consume the production CloudEvents/inbox/outbox path, verify cryptographic source authenticity, enforce live Xenia revocation, execute a real collector, establish process isolation between policy and evaluator, provide multi-probe stopping optimization, or establish production event-delivery guarantees. It does not replace Platform #309; it is the first small code seam for it. The evaluator's hidden truth is supplied to the harness and is not passed as a policy method argument, but a future process-separated runner is required to enforce a true information-flow boundary.

Run the complete crate locally:

```sh
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
```

No local or CI result is claimed by this change. Exact-head qualification is required before the code is treated as a qualified implementation slice.

## Paired attention experiment v0.1

The additive `src/paired_attention.rs` module reuses the existing deterministic stability plant and compares two policies on the **same immutable plant configuration and noise vectors**:

- **Fixed polling:** invokes the policy at a configured cadence, including quiet polls.
- **Event-triggered:** runs a lightweight observation monitor each actionable tick but invokes the policy only on a rising edge of the watched condition.
- **No-action counterfactual:** runs the same plant with no controller actions.

Both controller arms receive only the delayed/noisy `PlantObservation` and the eligible read-only probe identifier set produced by the shared `eligible_probe_ids_for_watch` filter. An out-of-set policy proposal is denied. Watch wakeups and probe selections have separate budgets. A selected read-only probe ID is only a proposal: it does not create a plant control action or authorize mutation. Both attention arms run the same no-action plant, so plant outcomes must match; remediation belongs in a later separately-authorized experiment. The evaluator compares wake records against actual plant samples only after each arm to report false wakes, missed fault episodes and event-to-wake latency.

The module includes eight paired-run tests covering invocation reduction, detection latency, no-action comparison, watch activation during a persistent fault, watch expiry during a persistent fault, authority expiry, out-of-set probe rejection, and a zero-probe-budget negative control.

**Metric boundary:** `monitor_checks` are deterministic monitor callbacks, not LLM calls. `cognitive_invocations` count scheduled policy invocations. `probe_selection_calls` and `admitted_probe_selections` count selection proposals, not actual remote collector executions. The module does not yet consume independent source/impact receipts for every paired wake; that is the next #320 integration increment. A synthetic plant pass is not a live SRE benchmark result.

**Claim ceiling:** the experiment is an offline paired controller comparison on a deterministic synthetic service model. It does not prove production event-delivery guarantees, model superiority, live Xenia authority enforcement, actual probe resource externality, independent observer authenticity or performance on SREGym/SRE-Marathon.

## Durable/ephemeral event schedule evaluator v0.1

The additive `src/event_schedule.rs` module reuses #306's watch/event contract semantics and provides an offline, deterministic evaluator over a frozen source schedule. It is not a second broker, subscription service, durable inbox, timer or production scheduler.

Strategies:
- **Fixed polling:** detects a condition only if its declared current-state validity window overlaps a poll tick.
- **Event-triggered:** processes the delivered source event record even after an ephemeral condition has cleared, provided the source record is still retained and its response deadline has not passed.
- **Coalesced event-triggered:** reduces policy wake groups only when epoch and coalescing key match within the configured window. Every event ID and incident ID remains in the wake receipt; the wake budget counts actual coalesced groups, not raw event count.

The timeline includes separate event time, valid interval, delivery time, record-retention deadline and response deadline. Source cursor semantics include:
- same event ID + epoch + sequence + payload digest: duplicate suppression;
- same event ID rebound to another sequence/epoch/digest: identity collision;
- sequence jump: explicit delivery gap;
- lower new sequence: out-of-order record;
- source epoch changes only after an explicit restart record with a declared proof digest;
- source cursor watermark: detects a final missing event even when no later event sequence arrives;
- unavailable source watermark: explicit source-unavailable result, never quiet success.

A watermark whose `next_sequence` equals the consumer's next expected sequence confirms continuity through that point. A greater watermark proves a gap. The deterministic harness verifies digest shape and binding fields only; canonical event owner code must authenticate the source and verify the watermark/restart proof before treating them as evidence.

Authority is rechecked at each processed record. Watch scope, event source identity, event sequence, event deadlines, event retention and wake/resource budgets are independent checks. Watch events can request a cognitive wake; they never authorize plant mutation or a consequential action.

The module includes 25 deterministic unit tests across multi-incident events, event coalescing, persistent/ephemeral state, fixed-poll misses, event deadlines and latency, duplicate replay after checkpoint, event-ID collisions, sequence gaps/out-of-order events, authenticated-restart shape, source epoch substitution, final cursor watermark gaps, source unavailability, authority revocation, indeterminate conditions, no-match quiet controls, positive controls and zero consequential mutation.

**Claim ceiling:** this is an offline schedule/conformance evaluator. It does not subscribe to the production CloudEvents path, authenticate event producers, cryptographically validate owner-held restart/watermark evidence, establish real inbox durability, prove delivery SLAs, integrate live Xenia revocation, execute diagnostic collectors or prove production real-time behavior. Source and event records are synthetic inputs; no SREGym/SRE-Marathon result is claimed.

## Independent watch-to-probe receipt auditor v0.1

The additive `src/episode_audit.rs` module independently checks the structural join across the existing event-schedule and watch/probe contracts. It requires an explicit decision for every event in a wake batch (probe requested, stop without probe, ask, abstain or escalate), then checks the probe request against the exact event, watch, authority epoch and read-only catalog entry.

For a requested probe, it checks:
- request -> wake-decision -> exact event schedule record and wake batch;
- source observation -> request, event, operation, tenant, target, purpose, collector source and control domain;
- impact receipt -> the same request/event/operation/scope and a measurement window after admission;
- observer control-domain separation from the collector;
- freshness, deadline and authority currentness;
- decision receipt -> exact observation and impact receipt IDs, recomputed loss reduction and terminal disposition.

Missing/unknown observation, impact or decision evidence produces an explicit indeterminate result; it is never converted to zero. A measured zero resource impact is a distinct, present observation. A write-capable or out-of-scope probe, substituted receipt, orphan request, mismatched event/wake, duplicate receipt identity or consequential mutation attempt is a hard audit failure.

The module includes deterministic positive, no-probe, quiet, missingness, staleness, cross-request substitution, independence-conflict, authority-revocation, write-capability and arithmetic-mutation controls.

**Claim ceiling:** this is a structural offline cross-record auditor over supplied records. It does not authenticate source publishers, verify signatures or raw-byte digests, prove process/organizational independence, execute the production CloudEvents path or real collectors, measure physical probe interference, integrate live Xenia admission, or establish SREGym/SRE-Marathon superiority. A passing audit qualifies only the supplied record graph and frozen synthetic contract.


## Source-qualified scheduled attention bridge v0.1

The additive \`src/scheduled_attention.rs\` runner composes the existing event-schedule evaluator with the existing deterministic no-action stability plant. It evaluates the schedule from one frozen timeline and reconstructs the exact same source events for the policy input; callers cannot supply one timeline to the scheduler and substitute another to the policy bridge.

Each policy wake includes source ID, source epoch, event ID, incident ID, sequence, payload digest, event/valid/delivery/processing/deadline clocks, and the watch/authority identity. A coalesced batch is one policy invocation, but every contained event requires exactly one explicit disposition. A selected read-only probe is checked against the eligible catalog IDs and recorded only as a proposal.

The runner deliberately stops before Xenia admission, remote collector execution, impact measurement or consequential mutation. It does not manufacture probe request receipts, source observations, impact receipts or zero-cost measurements. These must be produced by their respective real boundaries before #336 can qualify the complete receipt chain.

Tests cover exact source-identity preservation, one invocation with per-event decisions for a coalesced batch, catalog-bound read-only proposals, rejection of an ineligible proposal, rejection of missing policy dispositions, sequence-gap fail-closed behavior, quiet/no-event behavior and equality with an independent no-action plant replay.

**Claim ceiling:** this is still a deterministic in-process bridge over synthetic event schedules and the existing synthetic plant. It does not authenticate event publishers, prove policy process isolation, integrate live Xenia, execute collectors, measure probe externality, or establish real-world SRE repair performance. Probe proposals are not admitted operations and this runner makes no SREGym/SRE-Marathon claim.


## Scheduled-attention → episode-audit bridge v0.1

The source-qualified scheduled-attention runner now has a structural adapter into the independent episode auditor. It maps each per-event policy disposition to a WakeDecisionReceipt without reconstructing event identity from tick or synthetic sequence values. The adapter rejects missing/duplicate/orphan decisions and source-epoch substitution.

A selected probe is represented as ProbeProposedNotAdmitted, not ProbeRequested. The auditor checks the proposal against the read-only, scope-matched catalog, requires that no admission request be represented for the same event, and marks the episode Indeterminate with ProbeProposalNotAdmitted. This is intentional: a policy proposal is not an Xenia admission, collector execution, source observation or impact receipt. Non-probe dispositions can be audited as explicit no-probe decisions. Local decision IDs are deterministic join keys only, not cryptographic attestations.

Tests cover exact identity mapping, proposal-versus-request distinction, explicit indeterminate status for an unadmitted proposal, missing per-event disposition rejection and source-epoch substitution rejection.

**Claim ceiling:** the adapter closes the schedule-to-policy-decision record seam only. It does not close admission, collection, observer independence, impact or sustained recovery. No complete episode qualification is possible from this adapter alone.


### Proposal catalog snapshot (follow-up hardening)

A proposal now preserves the exact catalog contract visible at selection time: operation digest, tenant/target/purpose, collector source ID, collector control domain, read-only bit, maximum duration, proposal tick and the observation source tick. The source-qualified runner resolves the already-eligible probe ID against exactly one catalog entry before emitting this proposal. The auditor adapter consumes exactly one proposal for every SelectReadOnlyProbe decision and none for other dispositions; it checks the proposal's observation tick against the actual wake observation. The auditor then compares every frozen field against the catalog supplied at audit time. A later catalog mutation, ambiguous entry, substituted operation digest, changed scope/control domain, changed duration or forged proposal time cannot be silently treated as the same proposed operation.

This still remains a policy proposal, not an admission request or collector receipt. A valid catalog snapshot does not grant execution authority.


### Source identity carried through the audit receipt

The adapter's wake-decision receipt now retains source ID and payload digest in addition to source epoch, event ID, incident ID, sequence, watch and authority epoch. The admission/request receipt carries the same source ID and payload digest, and the auditor requires exact equality to the wake-decision receipt plus the watch's source ID. This prevents a downstream admission record from matching only by event ID/tick while changing the source or payload identity. The structural auditor validates the digest's SHA-256 encoding but does not authenticate its publisher or re-read the original event bytes; that still belongs to the source-owned signed evidence path.


## Paired five-arm context-compilation ablation v0.1

The additive `src/context_ablation.rs` module implements the orthogonal context-compiler factor described in issue #318. It builds five presentations from the same frozen source-evidence universe:

1. raw source records;
2. normalized canonical records;
3. normalized records plus topology relations and exact change/diff evidence;
4. topology plus event-time/observation-time ordering and explicit freshness states;
5. temporal context plus transparent, deterministic evidence/object ranking and alternative candidates.

All arms preserve the same evidence-ID universe. Unknown, absent and present remain distinct. Ranking is based only on visible evidence, target/dependency relationships, freshness and source diversity; source diversity is not treated as proof of independent failure domains. No evaluator-owned root-cause truth is passed through the `ContextReasoner` interface. The evaluator scores the returned diagnosis afterward using a separate ground-truth object.

The runner uses one reasoner adapter, preregistered arm order and identical per-arm context/token/tool/time budgets. It retains budget-exceeded arms in the denominator rather than dropping them, and records context bytes, estimated input tokens, compiler time, reasoner time, token usage, tool calls, cost, root correctness, support precision/recall and unsupported evidence claims. The byte/4 token estimate is preflight only; the reasoner adapter's actual usage is recorded separately. Arm order is supplied by the experiment harness so scenario blocks can be randomized/counterbalanced externally.

**Claim ceiling:** this is an offline experiment harness with scripted test reasoners. It does not call a live model, provide real tool adapters, establish SREGym results, or prove context-arm superiority. Confirmatory results require pinned model/provider snapshot, prompts, effort, tool permissions, fault/seed blocks, native grader, and paired multi-attempt runs. Native SREGym outcomes and Luminous-specific authority/effect metrics remain separate.


## Paired five-arm context-compilation ablation v0.1

The additive `src/context_ablation.rs` module implements the orthogonal context-compiler factor described in issue #318. It builds five presentations from the same frozen source-evidence universe: (1) raw source records; (2) normalized canonical records; (3) topology relations plus exact change/diff evidence; (4) temporal ordering and freshness/completeness; and (5) ranked causal evidence with alternative candidate objects and explicit support IDs.

Every arm preserves the same evidence-ID universe; Unknown, Absent and Present remain distinct. Ranking uses only visible evidence, target/dependency relationships, freshness and source diversity—different source IDs are not described as independent failure domains. The reasoner interface receives no evaluator-owned root cause or required-evidence list. The evaluator scores only after the answer returns, and separates root correctness, root visibility, root grounding, evidence precision/recall and unsupported citations.

Every packet/result is bound to the same paired block, attempt, scenario revision, model snapshot, prompt, permission profile, tool catalogue, scorer, reasoning effort and sampling seed. The caller supplies a preregistered arm order so it can randomize/counterbalance order across blocks. A fresh reasoner conversation/session is required for each arm. All arms share the same byte/token/output/tool/wall-time budgets; an over-budget arm remains in the denominator with BudgetExceeded instead of disappearing. The byte/4 token figure is a preflight estimate only; the model adapter's measured token usage is recorded separately.

**Claim ceiling:** this is an offline paired-experiment harness with scripted test reasoners. It does not call a live model, implement the actual Kubernetes/Radar/Xenia tool gateways, establish SREGym results, or prove context-arm superiority. Confirmatory work still requires a pinned model/provider snapshot, prompt/effort/permissions/tool schemas, native SREGym revision and grader, randomized paired fault/seed blocks, multiple attempts, and separate native versus Luminous assurance scoring. Wall-time budget is measured and scored after the adapter returns; hard cancellation must be implemented by the external model adapter.


### Paired context-ablation analysis and denominator discipline

`analyze_paired_context_results` groups records by fault family, scenario, paired block and attempt. It requires exactly one row for every one of the five arms and an identical scenario/model/prompt/permission/tool/scorer/effort/seed manifest within each block. Duplicate arms, incomplete blocks and changed manifests fail rather than being silently dropped.

The analyzer reports, per fault family and arm: paired-block count, scored count, budget-exceeded count, correct-root and grounded-root counts, correctness/grounding over **all** paired blocks, conditional correctness/grounding among scored blocks, context bytes, estimated tokens, measured input tokens and reasoner time. Contrasts against raw-source context report all-block deltas alongside conditional deltas only where both arms scored, plus pair coverage and token/time differences. This preserves the distinction between better quality, better coverage and simply dropping hard/over-budget examples.

These are analysis primitives, not observed results. The fixture reasoner is scripted; competitive results require the pinned live model and native benchmark protocol described in #318.


### Model-visible context isolation and fresh sessions

The reasoner adapter now receives only the rendered operational evidence text plus an equal per-arm budget. It does not receive the full `ContextPacket` or the evaluator-side scenario ID, paired-block ID, attempt number, arm enum/name, model/prompt/permission/tool/scorer digests, or sampling seed. The rendered context exposes the operational target and evidence; the arm-specific evidence presentation is necessarily visible, but the treatment label and pairing metadata are not serialized into the prompt.

Before each arm, the adapter must create a fresh provider session and return its session ID. The runner rejects empty, reused or mismatched session IDs, and the result row retains the session ID for audit. The paired-results analyzer independently rejects duplicate session IDs across result rows. This is a structural contract, not proof that a remote provider truly erased all hidden conversational state; a live adapter must be tested with sentinel leakage controls and independent conversation isolation verification.

The exact-head test suite also covers the `bool::then_some` eager-evaluation traps found during initial CI: future-dated timestamps must not subtract before their validity check, and empty scored-pair means must produce `None` rather than divide by zero.


### Prompt isolation, per-arm sessions and scenario-clustered inference

The context reasoner receives only the rendered operational evidence text plus an equal budget. The benchmark treatment label (arm), scenario ID, paired-block ID, attempt number, model/prompt/permission/tool/scorer digests and seed are kept evaluator-side and are not serialized into the prompt or supplied as a ContextPacket to the reasoner. A fresh provider session is created for each arm; its ID is retained in the result and reused/empty/mismatched session IDs are rejected. A real adapter still needs provider-level sentinel leakage tests: unique local IDs alone cannot prove that a remote service discarded hidden conversation state.

The paired-results analyzer rejects duplicate arm rows, incomplete five-arm blocks, mismatched per-block manifests, duplicate reasoner sessions, duplicate scenario/attempt identities, repeated sampling seeds within one scenario, and model/prompt/permission/tool/scorer/scenario-revision drift across attempts in the same scenario cluster. Sampling seeds may vary across attempts; the other execution conditions remain fixed.

Repeated attempts are nested within the fault scenario. For each candidate-vs-raw contrast, attempts are first averaged within each distinct scenario. The analyzer reports scenario-level wins/losses/ties and the equal-scenario-weighted macro delta. A deterministic SplitMix64 bootstrap resamples the scenario clusters (not attempts), using 10,000 replicates and the caller's preregistered seed. A percentile 95% interval is omitted below 20 distinct scenario clusters. This is intentionally conservative; 17-fault Lite cannot produce the interval, even if each fault is attempted three times. All-block denominators still count budget-exceeded rows as unsuccessful, while scored-only and both-scored contrasts remain separately labeled.

This is a descriptive inferential aid, not a substitute for a preregistered power analysis, a hierarchical model for multiple fault families, multiplicity control across arms/families, or provider-native uncertainty. Confirmatory claims should report the number of independent scenarios, attempts per scenario, per-cluster outcomes, interval method, seed and all exclusions.


### Executable randomization and five-slot counterbalancing

The arm-order seed and counterbalance slot are no longer passive manifest fields. `counterbalanced_arm_order(seed, slot)` derives a deterministic seeded base permutation and rotates it by the slot. Slots 0 through 4 therefore place each of the five context arms at each invocation position exactly once over a complete cycle. The runner recomputes the expected order from the profile and rejects a caller-supplied order that does not match. The input validator binds the slot to the one-based attempt number (`(attempt_id - 1) mod 5`), while the result analyzer verifies each arm's recorded position against that same recomputed order. It rejects duplicate/missing positions and an arm-position swap even when the position set remains 0–4.

The seed and slot remain evaluator-side and are not rendered into the model prompt. The base permutation is shared within a scenario so the cyclic rotation controls position effects without changing the underlying shuffled sequence. A reproducibility test proves the same seed gives the same order, a changed seed changes the base order for the fixed fixture seed, and all five arms visit every position once per cycle.


### Effective reasoner manifest attestation

A requested experiment manifest is not enough: a provider alias can resolve to a changed model snapshot, a tool adapter can drift, or a request can silently use another effort/seed. Each adapter response therefore returns an effective invocation manifest containing the served model snapshot digest, exact system/developer prompt-template digest, permission-profile digest, tool-catalog digest, reasoning effort and sampling seed. The runner requires byte-for-byte digest/value equality with the preregistered block manifest before scoring the answer. Mismatch aborts the block as `ReasonerManifestMismatch`; it is not counted as a model failure or silently pooled under the requested model label.

This is an adapter-reported manifest, not a cryptographic provider attestation. The live provider adapter must source the values from the actual request/response metadata and immutable prompt/tool configuration, not blindly echo the requested profile. Unknown/unpinned provider model revisions must remain unqualified for model-comparative claims. A further provider-side audit and independent capture path is still needed to prove the adapter's report is truthful.


### Model-specific token preflight and unknown-tokenizer behavior

The byte/4 input-token figure remains a rough diagnostic estimate and is not a hard rejection criterion. Before each arm, the adapter may return an exact token count from the pinned model snapshot's tokenizer. If available, that exact count is used for preflight and an over-budget arm is retained as `BudgetExceeded` without opening a model session. If exact preflight is unavailable, the runner still calls the adapter with the hard token budget; returned actual input-token usage is checked and an overrun is retained as `BudgetExceeded`, never scored. Every row stores both the rough byte/4 estimate and the optional model-specific preflight count so their errors can be quantified against actual usage. The adapter is responsible for enforcing the supplied budget before submission when exact local token counting is unavailable; the post-call result alone cannot prevent provider-side spend.
