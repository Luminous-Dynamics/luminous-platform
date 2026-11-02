# Long-running monitoring and event-aware waiting benchmark v0.1

## Purpose

This track measures how agents sustain attention to changing external state while controlling latency and resource consumption. It is distinct from incident diagnosis/repair: the main behavior is deciding when to observe, wait, re-observe, and act.

The track is motivated by SentinelBench (Microsoft Research, June 2026), which introduces 100 time-evolving tasks across 10 synthetic web environments and measures completion, reaction time and resource use. Its published comparison shows meaningful differences between event-aware waiting and fixed sleep: for GPT-5.4, the wait_for baseline completed 69/100 tasks on a 40-minute-equivalent setting at median $0.48/task, while sleep completed 56/100 at $4.65/task. These are SentinelBench's own results and cost basis; do not transfer them to OpsBench or assume they generalize to real SRE systems.

References:
- SentinelBench paper: https://arxiv.org/abs/2606.05342
- Microsoft Research: https://www.microsoft.com/en-us/research/publication/sentinelbench-a-benchmark-for-long-running-monitoring-agents/
- Baseline results: https://sentinelbench.com/
- SRE-Marathon continuous-operation context: https://arxiv.org/abs/2609.33023

## Agent/evaluator information boundary

The task contract and agent-visible observation/tool interfaces are separate from hidden evaluator truth.

**Agent-visible:**
- task goal and condition/predicate identity;
- allowed observer/tool capabilities;
- task window and declared wait ceiling;
- state snapshots returned by the allowed observer;
- agent wait decisions, wake causes and authorized actions.

**Evaluator-only:**
- exact event onset and expiry;
- whether a task is a no-op or an event will occur;
- hidden expected action truth and event schedule;
- oracle labels and the exact missed/false-wakeup classification.

The evaluator-only truth schema is stored separately. Never mount it into the agent's filesystem, prompt, retrieval corpus, tool results, logs or runtime environment. Source authentication and artifact access controls are still required from the sealed/runtime layers.

## Task families

- **Passive wait-for-signal:** the agent should wait until a new external signal appears, then detect it.
- **Active monitoring:** the agent can sample or subscribe to an observable source and react when a condition changes.
- **Relative baseline:** the agent must capture a baseline state and detect a later change relative to it.
- **No-op:** the event never occurs; the correct behavior is to refrain from action and avoid unnecessary polling.
- **Urgent ephemeral:** an event appears for a bounded interval and requires an authorized action before expiry.
- **Persistent event:** an event eventually occurs and remains available until the run ends.

A single task can include multiple non-overlapping event windows for one condition. Overlapping indistinguishable windows are rejected unless the task uses a richer predicate identity.

## Metrics

Do not combine these into one score.

| Metric | Definition |
|---|---|
| Detection recall | Events with at least one valid Present observation during the evaluator's active event window / expected events |
| Missed-event count | Expected events with no valid detection inside the active window |
| False-positive observations | Present snapshots when no evaluator event for that condition is active |
| False-signal wakeups | Wait decisions attributed to a signal without a corresponding present condition during an active event |
| Idle timeout wakeups | Timeout returns when no evaluator event is active at the wake time |
| Detection latency | First valid observation time minus evaluator event onset |
| Reaction latency | Correct committed action completion minus the first valid detection time |
| Event-to-action latency | Correct committed action completion minus evaluator event onset |
| Correct in-window action | Authorized participant-committed action, bound to the triggering observation and expected action digest, completed before event expiry |
| Premature/late actions | Actions attempted before event onset or completed after expiry, respectively |
| No-op violations | Any action attempt on an evaluator no-op task |
| Polling calls | Bounded polling + fixed sleep + exponential backoff calls |
| Event-driven wait calls | Explicit event-driven wait calls |
| Resource use | Tokens, tool/observation/wait/action calls, API cost, aggregate CPU time and calibrated energy where available |
| Evidence completeness | Complete observer and resource streams, no gaps/drops, source binding and bounded clock uncertainty |

The evaluator records detection and action timestamps separately. Event-to-action latency must not be confused with reaction latency.

## Protocol and integrity controls

- Freeze agent/model/scaffold/tool versions, task cohort, event-generator, resource ceilings and observer identity.
- Do not reveal event onset/expiry or no-op status to the agent.
- Use independent evaluator truth and observer receipts; do not infer an event from the agent's wake explanation.
- Every observation binds to the exact task predicate, source ID, control domain, snapshot digest, sequence and timestamp.
- Wait decisions bind to their resulting observation and actual wait duration. Clock uncertainty is bounded.
- Action receipts bind to a prior observation; a successful result requires an authorized, participant-committed effect.
- Unknown condition state, incomplete observation, missing resource telemetry, gaps/dropped events or partial coverage are Indeterminate.
- No-op tasks are hard controls: any attempted action is a safety failure.
- Resource counters must reconcile to actual observation, wait and action records.
- Physical energy is reported only when device identity, calibration profile, source and measurement interval are supplied. Otherwise energy is not measured.

## Known-answer and adversarial tests

The deterministic test suite includes 40 known-answer controls across no-op, passive wait, urgent ephemeral and persistent-event task families, plus 60 mutation vectors for:
- evaluator truth leaking into the agent task contract;
- event onset/expiry and condition-predicate substitutions;
- observer identity/domain and state snapshot changes;
- sequence gaps/duplicates and stale/future observations;
- wait strategy, duration, wake cause and resulting-observation mismatch;
- actions before event, after expiry, without a valid observation, without authorization, without participant commit, or with a substituted action digest;
- no-op action, false wakeups, resource count mismatch, energy calibration problems;
- partial/unknown/tampered coverage and missing telemetry.

These tests validate the encoded structural arithmetic. They do not prove that a real monitoring source delivered complete state or that the event generator represents production timing.

## Comparison plan

Run at least four paired strategies when supported:
1. event-driven wait/notification;
2. bounded periodic polling;
3. exponential backoff;
4. fixed sleep.

Use identical hidden event timelines, environment snapshots, task contracts, execution budgets and model/tool identities. Randomize strategy order under a frozen schedule. Include no-op, persistent and ephemeral conditions, plus variable event-time distributions and noisy/duplicate/contradictory signals.

Report the latency/resource Pareto frontier and uncertainty by independent scenario cluster. A lower cost per event is useful only alongside missed-event, false-wakeup and safety outcomes. Do not claim superiority over SentinelBench without reproducing its exact tasks and scoring separately.

## Claim ceiling

The evaluator measures event-window arithmetic and declared resource/observer evidence. It does not itself run the agent, authenticate publisher signatures, protect hidden truth from a malicious runtime, prove production distribution fidelity, or measure physical energy unless a calibrated external source is supplied. A green synthetic test suite is not production readiness.
