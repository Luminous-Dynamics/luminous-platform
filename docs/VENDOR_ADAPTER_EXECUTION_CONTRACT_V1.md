# Vendor Adapter Execution Contract v1

**Reviewed:** 2026-10-10  
**State:** contract and synthetic fixture only; no vendor operation implementation is claimed  
**Scope:** safe orchestration of management API/CLI operations on customer-authorized network/security devices

## 1. Canonical machine-readable contracts

- Capability registry: [vendor-adapter-capability-registry-v1.json](../security/vendor-adapter-capability-registry-v1.json)
- Capability schema: [vendor-adapter-capability-registry-v1.schema.json](../schemas/vendor-adapter-capability-registry-v1.schema.json)
- Mutation receipt schema: [vendor-operation-effect-receipt-v1.schema.json](../schemas/vendor-operation-effect-receipt-v1.schema.json)
- Synthetic PREPARED receipt fixture: [vendor-operation-effect-receipt-prepared-v1.json](../tests/fixtures/vendor-operation-effect-receipt-prepared-v1.json)
- Registry validator: [validate_vendor_adapter_registry.py](../tools/validate_vendor_adapter_registry.py)
- Receipt validator: [validate_vendor_operation_receipt.py](../tools/validate_vendor_operation_receipt.py)
- Implementation acceptance criteria: [Issue #23](https://github.com/Luminous-Dynamics/luminous-platform/issues/23)

The schemas define allowed shapes; the Python validators add cross-document and cross-field invariants. Passing either validator proves only that the described record conforms to the declared contract—not that a vendor device behaves correctly or a physical effect occurred.

## 2. Trust and authority boundaries

The adapter is an untrusted actuator. It translates a typed request into a vendor API/CLI operation and returns observations. It does not choose policy, create its own authorization, decide that an uncertain action succeeded, or certify its own result.

- **Policy authority:** owns versioned policy and may approve a precisely scoped change.
- **Planner:** resolves a target using exact inventory, produces action and desired-state digests, obtains a fresh pre-state, and renders the proposed diff and blast radius.
- **Authorizer:** signs approval bound to the exact target, action digest, pre-state digest, policy revision, profile digest, expiry and allowed window. For privileged changes, requester and approver are distinct principals.
- **Executor:** performs only the authorized typed operation. No ambient shell/root access, fallback command path, or undeclared capability is implied by a manifest.
- **Independent verifier:** separately observes the device after execution and compares the observed state with the intended state. The executor cannot serve as the only verifier.
- **Evidence/trust fabric:** retains raw evidence under the customer's access/retention controls and may publish minimized signed references or commitments to Mycelix/Holochain. Shared records never contain credentials, raw configurations, or customer personal data.
- **Customer authority:** retains final environment-specific release/authorization responsibility. The software does not manufacture an approval.

## 3. Operation classes and retry rule

| Operation | Class | Retry after uncertain outcome |
|---|---|---|
| Inventory, health, version, entitlement and lifecycle reads | Read-only | Usually safe to repeat if request semantics are read-only; preserve rate limits and audit context |
| Configuration export | Read-only with sensitive-data handling | May be repeated, but outputs must remain in approved protected storage; redact before sharing |
| Configuration plan/diff | Planning | Recompute from a freshly observed state; an earlier plan is stale after any relevant state change |
| Configuration apply | State-changing | Never blindly replay after timeout/uncertain acknowledgement |
| OEM firmware update or recovery | Disruptive lifecycle | Never blindly replay; reconcile actual device/boot state through read-only means first |
| Alternative OS flashing | Reflash / highest risk | Never blindly replay; if outcome is uncertain, stop and use a documented read-only diagnosis/recovery procedure |

An idempotency key is a correlation and deduplication aid, not proof that a vendor API/CLI operation is idempotent. A timeout after sending a request does not prove failure, and an HTTP/API success response does not prove that the device reached the desired state.

## 4. Durable effect state machine

Each state-changing operation writes a durable event before and after its possible side effects. The receipt is append-only evidence; do not rewrite a failure to look like success. A retry after terminal uncertainty must be a new operation with a new authorization after reconciliation.

| State | Meaning | Required behavior |
|---|---|---|
| `PREPARED` | Target, action, pre-state and profile have been frozen; no effect is claimed | No mutation allowed |
| `AUTHORIZATION_VERIFIED` | Signature and all authorization bindings verified | Authorization must be current, unexpired and bound to exact current intent |
| `APPLY_STARTED` | Immediately before the side effect, the executor re-read and confirmed the authorized pre-state | If pre-state differs, abort and re-plan; journal that mutation may now have occurred |
| `APPLIED_UNVERIFIED` | Request returned or a possible effect was observed, but desired state is not independently verified | Do not call this a pass; collect fresh read-only observations |
| `VERIFIED` | Independent verifier observed the exact intended result and linked the raw evidence | Terminal success for this bounded operation only |
| `FAILED` | Definite no-effect/rejection, or an observed partial effect with explicit remediation evidence | Preserve failure output and artifacts; a direct transition from `APPLIED_UNVERIFIED` must use `EFFECT_PARTIAL` with evidence. A no-effect/rejected conclusion after uncertainty requires read-only reconciliation evidence; remediation is a new authorized attempt |
| `INDETERMINATE` | Effect may have occurred, but the outcome cannot yet be established | No re-apply or automatic retry. Reconcile read-only; append only a permitted reconciliation state |
| `ABORTED` | Operation stopped before any effect | Record reason; any new attempt uses a fresh plan/authorization |

Allowed high-level transitions are:

- `PREPARED → AUTHORIZATION_VERIFIED → APPLY_STARTED → APPLIED_UNVERIFIED → VERIFIED`
- `PREPARED → FAILED/ABORTED`
- `AUTHORIZATION_VERIFIED → FAILED/ABORTED`
- `APPLY_STARTED → FAILED/INDETERMINATE`
- `APPLIED_UNVERIFIED → FAILED/INDETERMINATE`
- `INDETERMINATE → APPLIED_UNVERIFIED/VERIFIED/FAILED` after read-only reconciliation

There is intentionally **no transition from `INDETERMINATE` back to `APPLY_STARTED`** in the same receipt. A fresh operation can only be considered after the existing one is reconciled, the current pre-state is captured, and new authorization is obtained.

## 5. Authorization binding contract

A privileged authorization is invalid unless it binds to all of these values:

1. Exact target identity digest, including manufacturer/model/hardware revision/software/region context.
2. Exact action digest and resulting desired-state digest.
3. Exact observed pre-state digest and timestamp.
4. Policy revision and deployment-profile digest.
5. Requester and distinct approver identity references.
6. Expiry and approved maintenance window.
7. A verifiable authorization signature/receipt.

Immediately before mutation, re-read enough device state to establish that the pre-state remains identical to the authorized digest. Any drift, expired approval, device identity mismatch, profile change, policy revision change or action-diff change invalidates approval. The agent must stop rather than attempt to update the signed approval in place.

The mutation receipt records an opaque target identity digest rather than requiring a raw serial number in shared records. Protect the mapping to the physical asset as sensitive customer inventory. Even a digest can leak information if it is predictable or correlatable; use an appropriately keyed commitment or random asset token where needed.

The receipt also binds the **adapter implementation source commit** and exact product family. Effect-state semantics are fail-closed too: a receipt cannot move directly from `APPLIED_UNVERIFIED` to `FAILED` while claiming `NO_EFFECT` or `EFFECT_REJECTED`; it must record `EFFECT_PARTIAL` and evidence. A no-effect/rejected outcome after an uncertain or possibly applied request must pass through reconciliation evidence. Journal events and authorization timestamps must not predate receipt creation or contradict each other. Before `AUTHORIZATION_VERIFIED` or any possible device effect, the receipt validator cross-checks the capability registry: the named operation must be `tested`, its adapter must meet the risk-specific maturity gate, the physical target's vendor/product family/model/hardware revision/software version/region must match the tested scope, and the receipt's adapter revision must equal that operation's tested source revision. A family-level `A0_DISCOVERED` entry may support a `PREPARED` planning receipt only; it cannot authorize an operation. This cross-document gate is still a contract implementation, not evidence that a real device was tested.

## 6. Recovery evidence gates

Before an operation begins, the receipt must reference a recovery plan and evidence sufficient for the risk of the operation. For firmware/reflash, that normally means:

- exact known-good image or manufacturer-approved recovery artifact, where lawful and applicable;
- verified image source, immutable version, hash/signature and exact hardware compatibility;
- tested physical recovery procedure and required tools/console/power stability;
- configuration/license/certificate backup plan and explicit record of anything that cannot be backed up;
- rollback limits and support/warranty/RMA implications;
- test evidence demonstrating recovery for the exact model and revision.

A file called a “recovery plan” is not proof of a successful recovery drill. The release gate needs raw observations tied to the exact subject. For a first alternative-OS conversion, use lab/non-production equipment and a known recovery path; do not experiment on the only live firewall or site gateway.

## 7. Vendor-specific semantics are not interchangeable

Official docs consulted on 10 October 2026 underscore why the common core must not erase per-vendor behavior:

- **Palo Alto PAN-OS:** the REST API covers a subset of firewall/Panorama functions; some configuration commit workflows require XML API or another documented management interface. Limit concurrent API work as directed by the exact release documentation and verify the running config after commit. See [PAN-OS REST API guide](https://docs.paloaltonetworks.com/ngfw/api/get-started-with-the-pan-os-rest-api) and [request/response and commit notes](https://docs.paloaltonetworks.com/ngfw/api/get-started-with-the-pan-os-rest-api/pan-os-rest-api-request-response-structure).
- **Cisco Meraki:** the documented Dashboard API uses a shared organization-level call budget (current docs list 10 requests/second per organization); HTTP 429 responses and Retry-After must be handled with bounded backoff, and organization-scoped credentials inherit their owner's privileges. See [rate limits](https://developer.cisco.com/meraki/api-v1/rate-limit/) and [API usage](https://documentation.meraki.com/Platform_Management/Dashboard_Administration/Operate_and_Maintain/How-Tos/How_to_Use_the_Cisco_Meraki_Dashboard_API).
- **Holochain:** integrity validation should be deterministic and pure; missing DHT dependencies can result in unresolved validation and retry. This makes it suitable for validating shared evidence structure, not for observing whether a physical switch actually applied a policy. See [Holochain data validation](https://developer.holochain.org/concepts/7_validation/).

These are discovery notes, not implementation claims. Verify API and limit details against the exact product/release on each support engagement.

## 8. Privacy and globally portable governance

The technical contract is vendor-neutral and globally reusable; legal and customer requirements attach as separate, versioned overlays. Keep data residency, privacy, export/import, crypto validation, equipment approval, procurement, warranty/support location and evidence-retention requirements jurisdiction-specific.

A shared Holochain DHT must not become the default store for full inventory, raw configurations or customer evidence. Share signed minimal claims/commitments only after privacy review. If the trust fabric is offline or partitioned, the local agent can continue enforcing existing approved policy, but it must not invent a fresh global authorization.

## 9. Acceptance gate before a vendor adapter can mutate a device

No mutation-capable adapter can advance beyond discovery/read-only until all of these pass for an exact physical device/software/region scope:

- schema and adversarial tests for action/target/pre-state/policy/profile binding, exact adapter source revision, product family and tested model/revision/software/region scope;
- malformed receipts must fail with structured schema errors before any cross-registry lookup; invalid input must not crash the validator or reach adapter binding logic;
- cross-document receipt-to-registry validation that rejects authorization/effects unless the operation is explicitly tested and its adapter maturity meets the risk-specific gate;
- stale pre-state, expired authorization, forged actor, wrong tenant/region/target and revoked credential denial tests;
- terminal failure/effect consistency, direct partial-effect reporting, reconciliation evidence, receipt chronology and authorization/journal timestamp binding tests;
- timeout-before-apply, timeout-after-apply, rate-limit and partial-commit cases;
- no-replay behavior after `INDETERMINATE` and read-only reconciliation tests;
- independently observed post-state and preserved raw artifacts;
- recovery/rollback drill matching the operation's risk class;
- support/entitlement/licensing and customer authorization review.

The registry in its current state deliberately labels each vendor family `A0_DISCOVERED` and `NOT_IMPLEMENTED`. This is a plan and validation contract, not evidence that vendor integration is done.