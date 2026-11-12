# Dispatch policy trace contract — v0.1

**Parent:** [Issue #435 — causal dispatch decision traces and replayable outcomes](https://github.com/Luminous-Dynamics/luminous-platform/issues/435)  
**Stack:** dispatch scenario corpus [PR #433](https://github.com/Luminous-Dynamics/luminous-platform/pull/433)  
**Status:** typed, read-only validator and synthetic unit tests authored; Rust execution and CI pending.

## 1. The causal trace is the benchmark unit

A scenario manifest describes a test, but not what a controller knew, when it knew it, what it decided or whether the plant delivered the requested service. A final KPI can hide future-information leakage, late decisions, unknown command effects, short reserve excursions or a comparison arm receiving better forecasts.

The benchmark unit is therefore the **causal dispatch trace**:

`observation availability → decision → action disposition → plant/service outcome`

## 2. Causal timing contract

Each observation distinguishes event/acquisition time from availability time to the controller. Each decision binds its trigger, computation start/completion, deadline, policy-action digest, disposition and observations actually used.

The validator rejects observations that became available after computation started. A stale, missing, invalid, unknown or unverified observation may appear and may be handled by fallback, but cannot be represented as current. The v0.1 trace uses one monotonic clock epoch; a reboot, clock correction or epoch change must be qualified by the Mycelix stream/timebase owner, not inferred locally.

## 3. Outcome and non-compensatory service contract

Plant outcomes are consumed from Symtropy or measurement owners; this module does not simulate them. Half-open intervals must cover the entire mission horizon without gaps or overlap. Required critical energy must reconcile as delivered plus unserved. For every interval:
- shortfall above its local allowance fails the critical-service gate;
- any component/protection hard-limit violation fails the component gate;
- reserve lower bound below the interval obligation fails the reserve gate.

A later interval cannot erase a previous shortage or reserve breach. Primary-source, auxiliary and storage energy remain separate quantities; auxiliary energy cannot disappear outside the boundary.

## 4. Evidence ceiling

The effective evidence class is the lower of controller-decision evidence and plant-outcome evidence. A measured controller connected to a simulated plant therefore produces a modelled service result. Physical comparison eligibility requires both controller and plant evidence to be measured or stronger and current. Independent-claim eligibility also requires independently witnessed/accredited classes and assessment references; it still depends on Mycelix receipt verification and applicable lab/accreditation scope.

## 5. Pairwise comparison

Policy identity/revision is expected to differ. Exact matched conditions are:
- scenario ID, seed and scenario manifest;
- mission, environment, plant model and exogenous disturbance trace;
- initial state and terminal obligation;
- auxiliary boundary;
- information and compute budgets;
- clock epoch/timebase;
- mission horizon and decision cadence.

A mismatched pair receives no metric deltas. Reports keep service, component-limit, reserve, causal-integrity and evidence-current gates separate. No weighted composite winner is produced.

## 6. Ownership and claim ceiling

Platform owns typed trace validation and result composition. Symtropy owns plant outcomes/model validity. Mycelix owns observation/sample identity, calibration, timebase, uncertainty, signatures and invalidation (#6042, #6048, #6054). Symthaea may generate policy experiments and counterexamples but is not an evidence authority or hardware controller.

This is a read-only trace/accounting evaluator. It does not run dispatch, simulate physical plant behavior, authenticate signatures, qualify protection, issue commands or demonstrate physical controller safety or superiority.
