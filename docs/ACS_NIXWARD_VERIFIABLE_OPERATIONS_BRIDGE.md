# ACS ↔ Nixward Verifiable Operations Bridge

**Status:** Proposed adapter and qualification contract; not implementation evidence  
**Revision date:** 2026-10-13  
**Normative scope:** First isolated-VM operations slice  
**ACS baseline:** Repository release tag `v0.1.1`, immutable commit `1af1f92c1e445cf5ffa662515f83275de6eb603e`; wire/schema namespace remains ACS specification `v0.1.0`  
**Nixward baseline inspected:** `main` at `ae60256f0be73138a539ecda048e81056dd14a30`; re-pin at implementation time before claiming conformance

## 1. Decision

Implement an **adapter between an agent-control wire protocol and Nixward's existing canonical System Transaction protocol**. Do not create another transaction engine, authority signer, or host mutation path.

- **ACS** is the Observed Agent ↔ Guardian interception and decision contract.
- **Nixward System Transaction** remains the canonical target/change/authorization/application/verification lifecycle for NixOS mutations.
- **Luminous Platform** owns the product integration, operator UX, correlation/telemetry, capability policy, and packaging.
- **Symthaea** may reason, diagnose, predict, and propose. None of those outputs constitutes permission.
- **Mycelix** may later coordinate organizational identity, consent, provenance, witness, and evidence exchange. It is not the sole online authority for local host safety.

Existing authoritative Nixward references:
- [System Transaction Architecture](https://github.com/Luminous-Dynamics/nixward/blob/ae60256f0be73138a539ecda048e81056dd14a30/docs/SYSTEM_TRANSACTION_ARCHITECTURE.md)
- [System Transaction Wire v1](https://github.com/Luminous-Dynamics/nixward/blob/ae60256f0be73138a539ecda048e81056dd14a30/docs/SYSTEM_TRANSACTION_WIRE_V1.md)
- [Wire v1 fixture](https://github.com/Luminous-Dynamics/nixward/blob/ae60256f0be73138a539ecda048e81056dd14a30/docs/fixtures/system-transaction-v1.json)

ACS references:
- [Pinned ACS v0.1.1 repository release](https://github.com/GenAI-Security-Project/agent-control-standard/tree/1af1f92c1e445cf5ffa662515f83275de6eb603e)
- [ACS v0.1.0 conformance definition at the pinned release](https://github.com/GenAI-Security-Project/agent-control-standard/blob/1af1f92c1e445cf5ffa662515f83275de6eb603e/docs/spec/conformance.md)
- [ACS instrument specification at the pinned release](https://github.com/GenAI-Security-Project/agent-control-standard/blob/1af1f92c1e445cf5ffa662515f83275de6eb603e/docs/spec/instrument/specification.md)

The ACS v0.1.1 repository release explicitly says the normative schema namespace and specification package remain v0.1.0. Pin both the release commit and schema namespace; never refer merely to a moving `main` or `integration` branch.

## 2. Security invariant

**An ACS `allow` means only that the observed agent may continue through the applicable host hook. It is not Nixward mutation authority.**

A consequential mutation requires all of the following, checked by the authoritative Nixward path:
1. An explicitly supported typed operation and exact target.
2. A fresh target-side pre-state observation.
3. A versioned, validated ChangePlan and recovery binding.
4. Owner/operator authorization over the exact plan under Nixward's existing authority protocol.
5. Fresh authorization validity, including signer trust, revocation, audience, expiry, nonce/replay, target, and exact plan subject.
6. The executor applying only the already-authorized exact operation.
7. Independent post-state observation and verification before success/promotion is claimed.

Do not add a new ACS-specific signing key that competes with the existing detached Ed25519 authority protocol. The ACS envelope authentication key authenticates the bridge message; it does not replace owner authorization.

## 3. Authority boundaries

| Component | May do | Must not do |
|---|---|---|
| Observed Agent / Symthaea | Propose typed intent, request bounded observations, explain hypotheses, predict side effects | Hold the host-mutation signing key or directly call arbitrary privileged shell |
| ACS host hook | Intercept supported events; send authenticated requests; honor ALLOW/DENY/MODIFY/ASK/DEFER; record request/result | Treat an ACS response as a Nixward ChangeAuthorization |
| ACS Guardian adapter | Validate envelope, replay/freshness, policy inputs, and operation allowlist; map tool requests to Nixward plans; request explicit approval where needed | Invent missing identity, infer approval from natural language, mint owner authority, or self-certify an effect |
| Nixward | Observe authoritative target state, create/validate plan, verify existing authorization, snapshot, apply exact authorized operation, verify post-state and recovery | Reinterpret a signed plan, silently widen scope, accept stale target state |
| Independent verifier | Check schemas, identities, policy references, digests, order of lifecycle events, and observed result | Trust only the executor's success exit code or its own statement of correctness |
| Mycelix (later/optional) | Exchange signed evidence and coordinate multi-party identity/consent where policy requires it | Become a hidden global dependency for local recovery or make a network outage expand local permissions |

The evaluator must be logically and capability-separated from the executor. Code/process separation alone is insufficient if both share unrestricted credentials or mutable policy state.

## 4. One request, end to end

1. **Session establishment:** the host adapter negotiates the pinned ACS capabilities and verifies the required signature/channel contract. Bind the session to a host-authenticated client identity; do not trust identity fields supplied by the model as proof.
2. **Interception:** before a mutation-capable tool runs, the host emits `steps/toolCallRequest`. All relevant hook payload fields and provenance must be populated by trusted host-side code, not synthesized by the LLM.
3. **Input validation:** the Guardian checks the pinned JSON Schema, request signature, timestamp window, unique request id, session binding, allowed tool name, argument schema, caller, tenant, target, and size/resource bounds.
4. **Typed translation:** the bridge translates only a declared tool/operation pair into a typed Nixward request. No generic `bash`, arbitrary shell string, evaluator expression, arbitrary path, or unbounded command is a mutation capability.
5. **Observe and plan:** Nixward obtains target-side observations and creates the canonical ChangePlan. Hashes identify evidence; hashes alone do not establish that evidence is trustworthy.
6. **Validate and preview:** Nixward validates the plan and produces a stable digest plus human-readable diff, blast radius, pre-state identity, and bounded recovery strategy.
7. **Approval and authority:** if approval is required, display the exact target, action/diff, pre-state, risk, expiry, and recovery path. Approval must pass through the existing Nixward authority mechanism and be bound to the exact plan. An ACS `ASK` interaction alone is not the cryptographic authority artifact.
8. **Immediate pre-apply check:** Nixward rechecks target identity and relevant pre-state immediately before mutation. A mismatch invalidates the operation; approval is not transferable to the changed state.
9. **Apply:** use only the exact operation covered by the plan/authorization. The executor cannot swap arguments, paths, target, environment, or desired state after approval.
10. **Verify:** independently observe the target and compare it with the intended post-state. A zero exit code, ACS `toolCallResult`, or executor-generated receipt is not sufficient by itself.
11. **Record:** correlate the ACS request/session to the Nixward transaction identity and store minimal evidence references/digests. Keep sensitive snapshots and secret-bearing content in authorized local/customer-controlled storage.
12. **Reconcile:** after a timeout, crash, lost response, or network partition, read actual target state and reconcile before retry. Never infer that no effect occurred simply because the caller did not receive an acknowledgement.

## 5. ACS compatibility profile

This adapter targets the pinned ACS v0.1.0 wire/schema contract distributed by repository release v0.1.1. It must implement and test all ACS-Core requirements relevant to the selected host integration before claiming ACS-Core conformance.

At minimum, the conformance review must cover:
- capability handshake and negotiated timeout/failure posture;
- JSON-RPC request/response envelopes and published schemas;
- the required hook set for the actual host, including `subagentStart` when the framework can spawn subagents;
- all required dispositions and required disposition-specific fields;
- SessionContext/chain semantics and published chain head for content-bearing steps;
- `request_id`/timestamp replay rejection;
- signed request and response envelopes using the ACS baseline or a separately qualified accepted cryptographic profile;
- liveness via the defined system method;
- wrapped MCP hooks if any session exposes MCP tools;
- exact host behavior for a Guardian timeout, malformed response, signature failure, handshake failure, and unsupported disposition.

### 5.1 Failure posture

ACS v0.1.0 defines a default `on_decision_failure=proceed` posture, i.e. a silent/transport failure can be fail-open unless the deployment configures otherwise. **This product profile MUST explicitly configure and verify `deny` for every tool or hook that can initiate a consequential mutation.** The configuration and behavior must be demonstrated in tests; a config string in a file is not proof that the selected host honors it.

For the MVP, also deny/defer if:
- handshake is absent, mismatched, or unverified;
- request/response signature cannot be validated;
- replay/freshness checks fail;
- the Guardian or Nixward authority service is unreachable;
- schema parsing is ambiguous or required fields are missing;
- policy evaluation errors, times out, or returns an unsupported disposition;
- a human approval is required but unavailable;
- the exact operation is not in the allowlist;
- current authorization is not verified.

If the host framework cannot reliably block the operation on hook failure, it is not qualified to expose mutation-capable tools through this profile.

### 5.2 Reference implementation is not the security oracle

The ACS reference README on its moving integration branch explicitly records missing HMAC signatures, replay protection, some required hooks/dispositions, liveness, and wrapped MCP support. This is not evidence against the standard; it means the reference demo must not be mistaken for a conformant, secured deployment. Verify the exact code revision, not only the README or a successful demo.

Do not expose an unauthenticated Guardian socket to other local processes or a network. Bind and firewall the service narrowly, validate origin where the transport uses origins, authenticate each message according to the pinned profile, and verify request/response binding. Loopback binding alone does not authenticate a local caller.

## 6. Nixward contract reuse and the open lifecycle gap

Nixward's System Transaction Wire v1 already defines the stable serialized boundary and lifecycle identity for consequential machine changes. The bridge MUST reference the canonical ChangePlan digest, target-machine digest, transaction identity, authority evidence, pre-state/snapshot, application evidence, and verification evidence that the existing implementation provides. It must not mint a parallel transaction id or make a second state machine authoritative.

**Blocker to resolve before enabling retryable mutations:** the published System Transaction Wire v1 lifecycle enumerates planned, validated, authorized, snapshotted, applied, verified, promoted, plus failed/recovered outcomes. The published lifecycle does not name an explicit *indeterminate external effect* state. A process crash or timeout after a mutation begins cannot truthfully be mapped to either “not applied” or “applied and verified” without fresh target observation.

Required resolution:
1. Inspect the exact current Nixward storage/executor implementation to determine whether an existing durable journal already represents an indeterminate attempt and reconciliation.
2. If that semantic exists, document and test its exact mapping to Wire v1 without changing v1's meaning.
3. If not, propose a Nixward-owned, versioned, backward-compatible schema/state migration. Preserve historical evidence and never silently map old failures into successful effects.
4. Until the gap is resolved, block operations whose timeout/crash behavior could have an ambiguous effect; do not add a competing state store in Luminous Platform to conceal the gap.

This is a hard blocker for non-idempotent or ambiguous effects, not an excuse to mark unexecuted tests as passed.

## 7. Scope and non-scope

### In scope
- Disposable NixOS VM only.
- One allowlisted, low-risk operation against a dedicated fixture service or similarly isolated target.
- Read-only observation and dry-run/plan mode first.
- Exact action/pre-state authorization, fail-closed host hook behavior, post-state verification, and timeout/restart/recovery tests.
- One real ACS host integration only after its exact hook and failure semantics have been inspected.

### Out of scope
- Arbitrary shell or unrestricted root access to Symthaea or another general agent.
- Production host changes, bootloader/firmware operations, storage destruction, routing/firewall/authentication mutations, and fleet-wide rollout.
- Claiming that Mycelix replication or a signed receipt proves the underlying assertion true.
- Treating Phi, confidence, model explanations, or a self-evaluation score as an authorization primitive.
- General ACS conformance claims before the entire required surface for this host has been qualified.
- Any consciousness, general-intelligence, or broad industry-superiority claim based on this slice.

The exact first operation is conditional on a source audit showing that the candidate operation is routed through the canonical Nixward transaction path. If it is not, choose another already-supported typed operation or implement the missing path in Nixward; do not create a shortcut.

## 8. Required evidence packet

Each qualification run must bind:
- test/fixture corpus id, version, digest, and all expected cases;
- exact Symthaea, platform, Nixward, host-adapter, and policy revisions;
- ACS release commit, schema namespace, claimed profile, and host framework/client versions;
- VM image, NixOS generation/configuration identity, hardware/virtualization and environment manifest;
- test command/workflow and dependency identities;
- request, transaction, authorization, snapshot, application, and verification identities/digests;
- exact per-case terminal result and reason code;
- logs/metrics redacted of secrets and private customer content;
- explicit limitations, skipped cases, unresolved blockers, and reviewer/evaluator identity;
- final exact-head qualification outcome.

A queued, pending, cancelled, skipped, stale, missing, or unexecuted result is not a pass. A successful build does not prove runtime safety; a simulated run does not qualify a real host. Reports must preserve the distinction.

## 9. Acceptance gates

### Gate A — Contract and source mapping
- [ ] Nixward Wire v1 source, fixture, ChangePlan, authority verifier, and executor code are traced at exact immutable commits.
- [ ] ACS pinned schemas and normative sections are fetched by exact revision and validated locally in the implementation workflow.
- [ ] Every supported ACS host event, disposition, timeout, and failure posture is mapped to an existing contract or tracked as an explicit unsupported capability.
- [ ] No duplicate transaction identity, authority signer, or executor path is added.
- [ ] The indeterminate-effect lifecycle gap is closed or the affected operation class is blocked.

### Gate B — Request integrity and authority
- [ ] Valid signed requests are accepted; missing/invalid signatures fail closed.
- [ ] Duplicate request id, stale/future timestamp, wrong session, response-id mismatch, and malformed schemas fail closed.
- [ ] Exact plan/target/pre-state/policy/expiry/nonce changes invalidate authority.
- [ ] `ALLOW` alone cannot authorize a Nixward mutation.
- [ ] ASK approval is not confused with a valid Nixward ChangeAuthorization.
- [ ] Symthaea has no raw signing key, broad shell, or direct privileged executor capability.

### Gate C — Effect correctness and recovery
- [ ] Unauthorized, unsupported, stale, or altered operations yield zero effects.
- [ ] A valid operation affects only the exact target and capability described in the authorization.
- [ ] The verifier independently observes the intended post-state.
- [ ] Crash before apply, crash during apply, timeout after apply, lost acknowledgement, duplicate retry, partial effect, and verifier unavailability each have a deterministic, evidence-backed outcome.
- [ ] No ambiguous operation is automatically replayed before reconciliation.
- [ ] Missing or contradictory evidence yields UNKNOWN/BLOCKED, not PASS.

### Gate D — Safety and operations
- [ ] Hook timeout and Guardian unavailability are shown to deny consequential tool execution on the actual selected host, not only in a unit-test mock.
- [ ] Normal host boot and recovery still work with the optional UI, Symthaea process, Mycelix peer/network, or Guardian service unavailable.
- [ ] No secrets appear in logs, traces, DHT records, receipt payloads, or crash dumps used by the test.
- [ ] Exact-head tests and the evidence packet are independently inspected before the pilot decision.

## 10. Source-grounded caveats

- ACS-Core requires message signatures and replay protection; the standard's default decision-failure posture is fail-open. The product profile deliberately overrides that posture for mutation-capable hooks and must test that the host enforces it.
- ACS conformance profile names are handshake declarations, not independently verified facts. The implementation must qualify its own claims.
- Nixward's architecture states that cognitive outputs are advisory and that consequential changes use its existing authority protocol. Reuse that boundary.
- An evidence digest identifies an evidence record; it does not alone prove that the record is trustworthy or that its claim is true.
- This document only defines a proposed mapping and gates. No adapter, signature verification, replay-defense path, VM operation, recovery matrix, or ACS conformance has been implemented or tested by this document.

## 11. Research / reference links

- [ACS repository release v0.1.1 and commit](https://github.com/GenAI-Security-Project/agent-control-standard/tree/1af1f92c1e445cf5ffa662515f83275de6eb603e)
- [ACS-Core conformance at the pinned commit](https://github.com/GenAI-Security-Project/agent-control-standard/blob/1af1f92c1e445cf5ffa662515f83275de6eb603e/docs/spec/conformance.md)
- [ACS instrument specification at the pinned commit](https://github.com/GenAI-Security-Project/agent-control-standard/blob/1af1f92c1e445cf5ffa662515f83275de6eb603e/docs/spec/instrument/specification.md)
- [ACS issue documenting reference Guardian signature gap](https://github.com/GenAI-Security-Project/agent-control-standard/issues/70) — use as a reference-implementation finding, not as a claim about every ACS deployment.
- [NIST SP 800-204D: supply-chain security in DevSecOps pipelines](https://csrc.nist.gov/pubs/sp/800/204/d/final)
- [Nixward System Transaction Architecture](https://github.com/Luminous-Dynamics/nixward/blob/ae60256f0be73138a539ecda048e81056dd14a30/docs/SYSTEM_TRANSACTION_ARCHITECTURE.md)
