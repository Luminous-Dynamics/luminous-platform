# Vendor Target Scope Model v2

**Status:** proposed versioned contract; not yet wired into the v1 validators or qualified  
**Date:** 2026-10-11  
**Related:** [Issue #25 — separate deployment jurisdiction from management endpoint region](https://github.com/Luminous-Dynamics/luminous-platform/issues/25), [Issue #23 — vendor adapter roadmap](https://github.com/Luminous-Dynamics/luminous-platform/issues/23), [Vendor Adapter Execution Contract v1](VENDOR_ADAPTER_EXECUTION_CONTRACT_V1.md)

## Decision

Do not reuse the v1 `region` field as a proxy for deployment jurisdiction, management endpoint, hardware market, licensing region or support jurisdiction. These are distinct dimensions. Introduce a new, versioned scope contract and leave all v1 documents and their original evidence intact.

This is a contract proposal—not a claim that v2 validation, vendor adapters or device tests already exist. Until integration is implemented and qualified, all six adapters remain `A0_DISCOVERED / NOT_IMPLEMENTED`.

## Scope has two separate jobs

### 1. Target / adapter capability scope

This answers: *which specific device and management path has this operation actually been tested against?*

It always binds the physical/logical subject and implementation:

- vendor and product family;
- exact model, hardware revision and running software/firmware;
- exact adapter source revision;
- management endpoint identity where the endpoint can alter behavior, authority, data handling or available operations;
- only those vendor-specific dimensions that the operation declares relevant.

Do not treat a vendor name, protocol name, or a successful API response as an exact tested scope. The capability registry must declare which dimensions are relevant for each adapter/operation. Missing or unresolved required dimensions block promotion.

### 2. Deployment policy context

This answers: *which policy and legal/customer context governs this proposed action?*

It binds the deployment jurisdiction and the exact effective policy/overlay revision. It is **not** evidence that the adapter works in that jurisdiction, and adapter qualification is **not** evidence of legal compliance. A jurisdiction-sensitive policy change invalidates the approval/action binding even where the adapter and endpoint have not changed.

The receipt therefore needs two different bindings:
- a **tested capability-scope digest**, derived from exact device/software/adapter/endpoint and declared operation-relevant vendor dimensions;
- a **policy-context digest**, derived from the deployment jurisdiction, customer profile and exact effective policy/overlay revisions.

The action digest and signed authorization bind both digests. Neither can stand in for the other.

## Dimension state is explicit

Every applicable scope dimension uses an explicit state rather than relying on absent fields or wildcard strings:

- `assessed`: the value is present, exact, source-backed and bound into the relevant digest;
- `not_applicable`: the adapter contract states why this dimension cannot affect this operation, with a rationale and review evidence;
- `unresolved`: the meaning or value is not established.

An omitted dimension is a schema/contract error, not implicit `not_applicable`. `unknown`, `unspecified`, `any`, `all`, `global` and `*` must never satisfy an exact tested scope. A global service must explicitly name the canonical endpoint it uses; “global” is not an endpoint.

An unresolved dimension that the operation requires blocks A1+ promotion and blocks authorization/effect. The validator must not downgrade `unresolved` to `not_applicable` to make a fixture pass.

## Management endpoint identity

Model endpoint identity as a typed record, not a free-form region label. It should include:

- `kind`: `vendor_cloud_api`, `on_prem_controller`, `device_local` or `serial_console`;
- a stable `endpoint_profile_id` and pinned profile revision;
- the canonical base URI or a privacy-preserving digest of the canonical endpoint identity;
- source/evidence references establishing the endpoint profile and its use for the target;
- any separately relevant vendor endpoint-region identifier.

Canonicalization must be implemented once, specified and tested. At minimum it must define scheme, hostname normalization, effective port, base path and treatment of query/fragment components. API credentials must not be embedded in a URI. A private device hostname/IP, tenant ID or network location must not be written to public DHT records; keep sensitive originals in customer-controlled protected evidence and publish only reviewed, minimized commitments.

Before sending credentials or requests, the adapter must validate the endpoint against its declared allowlist. Do not follow an origin-changing redirect with credentials. Reject unexpected scheme/host/port/base-path changes; a redirect to another origin requires a new explicit endpoint resolution and authorization check. This also prevents endpoint substitution from becoming a server-side request forgery path.

### Meraki source-backed example

Cisco Meraki's current Dashboard API onboarding documentation lists these base URIs:

| Hosted Dashboard | Canonical API base URI |
|---|---|
| Most regions (default) | `https://api.meraki.com/api/v1` |
| Canada | `https://api.meraki.ca/api/v1` |
| China | `https://api.meraki.cn/api/v1` |
| India | `https://api.meraki.in/api/v1` |
| United States FedRAMP | `https://api.gov-meraki.com/api/v1` |

Sources: [Meraki API Getting Started](https://developer.cisco.com/meraki/api-v1/getting-started/) and [Meraki API Schema](https://developer.cisco.com/meraki/api-v1/schema/) (checked 2026-10-11).

These values illustrate why endpoint identity must not be inferred from where equipment is installed. The site jurisdiction and the API base URI must be recorded independently. This example does not authorize calls to any tenant, prove entitlement, or qualify the Meraki adapter.

## Vendor-specific scope extensions

Some operations need further dimensions such as hardware-market SKU, firmware channel, support-entitlement jurisdiction, cloud hosting region or tenant/organization scope. Do not invent a universal semantics for these fields.

Use versioned extension records with, at minimum, a stable namespaced extension ID, schema version/digest, an explicit assessment state, a value or protected evidence reference, and evidence references. A validator may accept an extension as assessed only when the corresponding schema/version is registered and validated. Unknown extension schemas fail closed for operations that depend on them. Tenant and support identifiers must be privacy-reviewed; they need not be placed in public Holochain records.

The adapter registry must declare per operation:
- required dimensions;
- optional dimensions that still affect authorization when present;
- explicitly not-applicable dimensions with rationale;
- extension schema/version dependencies.

The same declaration is consumed by the planner, authorizer and receipt verifier; do not duplicate independent hand-maintained lists across components.

## Proposed v2 record shape

This is illustrative and is **not yet the production schema**:

```json
{
  "schema_version": "luminous.vendor-target-scope.v2",
  "target": {
    "vendor": "Cisco Meraki",
    "product_family": "Meraki MX",
    "model": "EXACT-MODEL",
    "hardware_revision": "EXACT-REVISION",
    "software_version": "EXACT-VERSION",
    "identity_digest": "<sha256>",
    "management_endpoint": {
      "state": "assessed",
      "kind": "vendor_cloud_api",
      "endpoint_profile_id": "meraki-dashboard-api",
      "endpoint_profile_revision": "<40-char-source-revision>",
      "canonical_base_uri": "https://api.meraki.com/api/v1",
      "endpoint_identity_digest": "<sha256>",
      "evidence_refs": ["evidence:protected-endpoint-observation"]
    },
    "vendor_dimensions": []
  },
  "policy_context": {
    "deployment_jurisdiction": "ZA",
    "policy_overlay_revision": "<40-char-source-revision>",
    "effective_policy_digest": "<sha256>",
    "evidence_refs": ["evidence:customer-policy-approval"]
  },
  "legacy_scope_migration": {
    "status": "unresolved",
    "source_schema_version": "v1",
    "source_region_value": "preserved-exactly",
    "review_evidence_refs": []
  }
}
```

The example values are placeholders, not real qualification evidence. Production schemas should use strict `additionalProperties: false` for core objects, exact digest formats, conditional required fields, uniqueness constraints for extension IDs, and explicit supported endpoint kinds. The schema-version identifier must change for the real v2 contract; v1 schema IDs remain immutable.

## V1 migration rules

1. Preserve v1 receipts, registries, signatures, digests and timestamps byte-for-byte. They are historical evidence.
2. Keep v1 `target.region` and `tested_scope.region` as legacy values. Never auto-map them to deployment jurisdiction, API region, market region or support jurisdiction—even when the string looks familiar.
3. Add a migration record that retains the original value, records `unresolved` by default, and requires independent review evidence to resolve each new dimension. `global`, `US`, `EU`, `ZA`, or `us-east` by themselves do not establish which dimension was meant.
4. Do not reuse v1 authorization, target/action digests or tested-scope claims for a v2 operation. New canonical payloads require new digests and a new approval.
5. Historical v1 records may be validated for archival integrity, but an unresolved v1 scope must not authorize a new mutation or be promoted into tested v2 scope.
6. Maintain a deterministic migration report mapping each legacy record to either a reviewed v2 scope or `unresolved`; do not mutate the source record.

## Required negative tests before v2 can authorize an effect

- Site/deployment-jurisdiction drift changes the policy-context digest and invalidates the old approval.
- Effective policy/overlay revision drift invalidates the old approval even when jurisdiction code is unchanged.
- Management endpoint profile, canonical origin, port, base path or profile revision drift fails exact scope binding.
- Origin-changing redirect, unexpected scheme and credential forwarding across origins are rejected.
- Omitted required dimensions and `unresolved` states fail closed.
- `not_applicable` without declared rationale/evidence is rejected.
- Vendor extension ID/schema-version drift or unavailable extension schema is rejected where the operation depends on it.
- A vendor operation explicitly declaring a dimension `not_applicable` can pass that dimension's check only if the declaration is versioned and reviewable; it cannot acquire an inferred value.
- A legacy v1 `region` is preserved but cannot satisfy either v2 deployment-jurisdiction or endpoint requirements.
- Changing the endpoint or policy-context digest prevents reuse of a previously signed authorization.
- All current adapters remain A0 / NOT_IMPLEMENTED; scope-model tests alone do not promote adapter maturity.

## Implementation sequence

1. Freeze this semantic model and decide the canonical endpoint-identity serialization before writing production digests.
2. Add v2 registry/receipt schemas beside v1; do not mutate the meaning of v1.
3. Implement one shared scope parser/digest module and make registry validation, receipt validation and authorization consume the same normalized representation.
4. Add migration fixtures for unresolved legacy values and source-backed Meraki endpoint fixtures; add negative endpoint/policy drift tests.
5. Add the new validator and tests to the workflow path filters and commands, then run them locally with exact command/exit-code evidence.
6. Independently review canonicalization, extension resolution and migration behavior before making v2 the only route to new mutations.
7. Select an authorized non-production device only after the scope contract is stable; no device interaction is needed to finish the schema/migration tranche.

The v2 contract is not complete until the validator and authorizer both use the exact same canonical bytes/digests. A schema alone cannot enforce URL connection behavior or prove endpoint identity at runtime.
