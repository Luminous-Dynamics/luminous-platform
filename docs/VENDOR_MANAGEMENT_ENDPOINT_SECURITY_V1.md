# Vendor Management Endpoint and Credential Boundary v1

**State:** implementation contract / review checklist; not an implementation or device qualification claim.  
**Related:** [Issue #25 — separate deployment jurisdiction from endpoint region](https://github.com/Luminous-Dynamics/luminous-platform/issues/25), [Issue #23 — vendor adapter roadmap](https://github.com/Luminous-Dynamics/luminous-platform/issues/23), [scope model v2 review thread](https://github.com/Luminous-Dynamics/luminous-platform/pull/26), [first lab slice](VENDOR_ADAPTER_FIRST_LAB_SLICE_V1.md).

## 1. Decision

A management endpoint is an authority boundary, not just a hostname setting. The selected endpoint profile must be frozen before credentials are retrieved and before observations can enter a qualification evidence packet.

Do not overload one `region` field. Keep these dimensions separate:

- `deployment_jurisdiction`: site/customer policy context for privacy, retention, procurement, export/import and other applicable rules.
- `management_endpoint`: the actual API/control-plane origin or a locally managed device endpoint profile.
- `hardware_or_commercial_region`: exact regional SKU, support, licensing or firmware channel where it has an evidenced effect.
- `support_jurisdiction`: only where the OEM/customer agreement makes it a distinct decision input.

A dimension must be explicitly `assessed`, `not_applicable` with a reason, or `unresolved`. Omission, wildcard values, `global`, or a vendor-family default do not satisfy an exact-scope requirement. Historical v1 records remain historical: do not infer what their old `region` meant and do not reuse v1 authorization or tested-scope receipts as v2 evidence.

## 2. Endpoint profile fields

Before adapter execution, freeze a versioned profile containing:

- stable profile ID, profile revision and digest;
- vendor, product family, exact adapter ID and adapter source revision;
- endpoint kind (`vendor_cloud_api`, `local_device_api`, `netconf_ssh`, `netconf_tls`, `gnmi_tls`, or another explicitly documented method);
- canonical origin: scheme, DNS name or literal address, effective port, and documented API base path;
- endpoint-selection source and its revision/date (for example, vendor documentation or customer-provisioned local endpoint);
- allowed redirect policy (default: no redirects); if explicitly supported, every hop must be revalidated against an allowlist and credentials must never be forwarded to a different origin;
- TLS/SSH trust profile, server identity checks, client authentication mode and minimum protocol policy;
- credential reference only (never credential bytes), expected API roles/scopes, tenant/org/account binding, rotation/expiry metadata and read/write permission boundary;
- network egress boundary and proxy policy; ambient proxy configuration must not silently redefine the destination;
- applicable deployment, commercial, support and data-residency dimensions, each with an explicit evidence-backed state;
- rate limit, bounded timeout, pagination and retry rules;
- raw-output classification, redaction profile, retention location and allowed shared-evidence fields.

Use secret-manager references or a scoped local credential broker. Never put credentials in command-line arguments, source fixtures, adapter logs, exception strings, URLs, query parameters, public receipts, or a shared DHT entry. Avoid broad organization/account admin tokens when a narrower read-only role exists.

## 3. Connection preflight — fail closed

Every new connection or credential-bound request must satisfy these gates:

1. **Authority:** the asset owner/customer authorized this precise adapter, endpoint profile and operation. A hostname or API key by itself is not approval.
2. **Scope:** adapter ID/revision, target identity, endpoint-profile revision, tenant/org/account and applicable jurisdiction dimensions match the frozen work case exactly.
3. **Destination:** scheme, hostname/address, effective port and API base path match the approved profile. Disable redirects by default. If an adapter must redirect, require a documented vendor contract and revalidate scheme, origin, destination allowlist and TLS identity at every hop; strip authorization/cookies on any origin change and stop rather than forwarding credentials.
4. **Peer identity:** validate TLS certificate chain and hostname, or the protocol's documented secure transport identity. Do not disable certificate verification to “fix” a connection. SSH host keys must be checked against a provisioned trust record. Do not silently downgrade to plaintext or a weaker method on failure.
5. **Credential binding:** confirm least privilege and expected tenant/org/account scope. A credential that authenticates successfully but resolves to another organization is a scope failure, not a recoverable warning.
6. **Time and freshness:** the clock must be usable for expiry/freshness assertions. If clock trust is unavailable, mark time-sensitive claims unresolved and do not perform privileged work.
7. **Network containment:** outbound access is constrained to the declared endpoint and required dependencies. No arbitrary URL, device-provided redirect, discovered link, or response field may trigger a new connection without an allowlist decision.
8. **Read-only enforcement:** during A0/A1 discovery work, the credential role, adapter operation allowlist and API method/path allowlist must all forbid mutation. A client-side UI toggle is not an enforcement boundary.
9. **Evidence:** retain a redacted connection decision record: profile digest, target/scope digest, auth mechanism class (not secret), peer identity result, decision time, adapter revision and bounded outcome code.

Any failed gate returns a stable denial code and no device observation is promoted to verified state. Do not include raw authorization headers, certificate private data, complete device responses or exception strings in a denial record.

## 4. Protocol-specific baseline

Use only the transport and authentication modes the exact product documents and the lab has validated.

- **gNMI:** follow the pinned gNMI specification and device-specific documentation. The OpenConfig specification requires encrypted transport, prohibits fallback to unencrypted sessions, and describes certificate-based peer authentication. Do not assume every vendor/model implements gNMI or the same YANG paths.
- **NETCONF:** follow RFC 6241 plus the selected transport binding. Authentication, integrity, confidentiality and replay protection depend on the transport; validate the authenticated peer and its authorized identity.
- **RESTCONF:** follow RFC 8040. Use HTTPS/TLS and validate the server certificate and identity; do not bypass certificate failure or retry over plaintext.
- **Vendor cloud APIs:** use the vendor-documented API origin for the selected account/region and current API revision. Meraki, for example, documents distinct base URIs for certain cloud environments. The origin must be selected explicitly, not guessed from a deployment country or silently defaulted from a broad label.
- **SSH/CLI:** validate host keys, constrain commands to named adapter operations, disable shell interpolation and do not pass secrets as argv. Capture command identity and bounded result status; never infer an effect from exit code alone.
- **SNMP:** allow SNMPv3 only with the exact authenticated/privacy profile supported by the target and a scoped principal. Do not silently fall back to SNMPv1/v2c or emit community strings into logs.

Normative references:

- [OpenConfig gNMI specification](https://github.com/openconfig/reference/blob/master/rpc/gnmi/gnmi-specification.md)
- [RFC 6241 — Network Configuration Protocol (NETCONF)](https://www.rfc-editor.org/rfc/rfc6241)
- [RFC 8040 — RESTCONF Protocol](https://www.rfc-editor.org/rfc/rfc8040)
- [Cisco Meraki API v1 — Getting Started](https://developer.cisco.com/meraki/api-v1/getting-started/)
- [NIST SP 800-218 — Secure Software Development Framework](https://csrc.nist.gov/pubs/sp/800/218/final)
- [SLSA provenance specification](https://slsa.dev/spec/v1.2/provenance) and [artifact verification guidance](https://slsa.dev/spec/v1.2/verifying-artifacts)

These references define protocol/process requirements, not support for a specific model, implementation correctness, or production qualification.

## 5. Adversarial qualification corpus

Before any adapter operation can be promoted beyond discovery, tests must demonstrate at least:

- approved origin succeeds with correct peer identity;
- hostname, scheme, effective port, path, endpoint profile revision or account/tenant binding changed;
- redirect to same host but unapproved path; redirect to a different origin; redirect loop; downgrade attempt;
- invalid, expired, mismatched-host or untrusted TLS certificate; changed SSH host key;
- missing credential, revoked credential, over-privileged credential and credential mapped to the wrong tenant/org;
- environment proxy mutation or an unapproved proxy path;
- arbitrary URL or device-supplied link injection;
- clock unavailable/skewed and expired endpoint profile;
- 401/403, 404, rate limit, transient 5xx, timeout before dispatch and timeout after request dispatch;
- API version drift, pagination truncation, duplicate pages and malformed content type/encoding;
- mutation HTTP method/path submitted to a read-only adapter;
- sensitive values in body, headers, URL, error response and stack trace do not appear in emitted logs/evidence;
- secret rotation and profile revocation prevent reuse of an old authorization/profile revision.

Expected decisions must be authored independently from the adapter under test. Each test asserts the returned decision code, whether any connection/request was permitted, whether credentials were sent, and whether the evidence record stays redacted. Network tests should use a controlled fake endpoint/proxy and synthetic credentials, never customer production credentials.

## 6. Evidence and maturity interaction

A passing endpoint-preflight test does not promote a vendor operation by itself. Store the exact endpoint profile digest and scope decisions beside the operation result, so the independent verifier can check that observation and authorization used the same endpoint and account scope.

A1 requires deterministic synthetic contract tests. A2 additionally requires an authorized lab target, exact endpoint/peer binding and a reproduced evidence packet. Change and firmware maturity require their separate authorization, effect-journal and recovery gates. Do not award one vendor-family maturity grade to models, regions, endpoints or operations not represented by evidence.

## 7. Stop and recovery rules

Stop and preserve a redacted record when the destination, tenant/account identity, peer identity, endpoint profile, adapter revision or relevant jurisdiction changes unexpectedly; when credentials appear in output; when a read-only method attempts mutation; or when connection status is too uncertain to establish the observation's scope.

Do not retry by weakening TLS, accepting a new host key without out-of-band verification, removing a tenant check, following an unapproved redirect, or broadening an endpoint allowlist. A changed endpoint is a new profile revision and may require new lab evidence and authorization.

## 8. External approval record — deliberately separate from profile

A profile describes a destination and its connection constraints; it cannot authorize itself. A separate approval record is versioned in [vendor-management-endpoint-approval-v1.schema.json](../schemas/vendor-management-endpoint-approval-v1.schema.json), with a [semantic binding checker](../tools/validate_vendor_management_endpoint_approval.py) and synthetic pending fixture.

The decision record binds the canonical profile digest, profile revision, adapter ID/source revision, exact target model/hardware/software identity, principal and tenant/org scope, policy revision, requestor, distinct approver, an explicit allowlist of read-only operations, and a bounded validity interval. The schema makes pending records non-authorizing and prohibits write/lifecycle/reflash operations from this read-only approval family. Rejected/revoked decisions require signed-decision metadata; revocation references the prior decision.

The validator checks record structure and cross-field/profile binding only. It does **not** cryptographically verify signature bytes, authenticate the signing key against a trusted identity registry, confirm the approver's delegated permissions, bind the target identity digest to a fresh inventory observation, verify the operation against a tested capability-registry entry, perform revocation lookup, or authorize a connection. Runtime enforcement must verify the signed payload digest and signature using a versioned trusted key/scheme policy; confirm the key was trusted and authorized for this decision at signing time; bind the target identity digest to a fresh independently observed inventory record; require the exact operation to be `tested` for that adapter revision/model/revision/software/endpoint-profile scope; and verify current state, validity interval and revocation before credential use. All those checks are mandatory; a structurally valid signed-looking record is still not authority.

The frozen profile digest uses the explicitly versioned `luminous-json-canonical-v1` representation: sorted ASCII schema keys, compact JSON, UTF-8, no floating-point values, no arbitrary properties, and only exact interoperable integer values in the range -9,007,199,254,740,991 through 9,007,199,254,740,991, followed by SHA-256. Larger integer-like identifiers or quantities must be represented as strings. The approval record carries the exact digest scheme in `profile_digest_scheme`; an unknown/missing identifier is invalid. This is a Luminous contract version, not a blanket claim of complete RFC 8785 implementation. Changing canonicalization requires a version bump and new binding evidence. The verifier and profile-digest implementation reject duplicate JSON member names, floating-point/non-finite numbers, integers outside the interoperable safe range, and the `-0` integer token before canonicalization; the last restriction is a deliberate ambiguity defense informed by the verified RFC 8785 erratum, not a claim that this custom format implements JCS. The signature verifier reads approval/profile/schema/trust-store/registry JSON through a bounded regular-file reader (4 MiB maximum per JSON file); the detached signature file is capped at 64 bytes. Final-component symlinks and non-regular files are rejected, and file metadata/size is checked for stability during reads. Oversized or unstable input fails closed rather than being loaded without a resource bound. Strict JSON parsing and both canonicalization paths also enforce a maximum structural nesting depth of 128 containers; malformed or deeper input is converted to a controlled validation failure.

Contract commands:

```sh
python tools/validate_vendor_management_endpoint_profile.py
python -m unittest discover -s tests -p "test_vendor_management_endpoint_profile.py" -v
python tools/validate_vendor_management_endpoint_approval.py
python -m unittest discover -s tests -p "test_vendor_management_endpoint_approval.py" -v
```

Both validators are intended to be network-free. Schema failures are rendered as stable validator-rule codes and JSON Pointers; diagnostics do not include rejected input values, credential-shaped strings, or full exception text. Regression tests deliberately inject secret-like invalid values and require them to remain absent from diagnostics. This is defense in depth, not a reason to place credentials in a profile. Passing these synthetic contracts is not device qualification, signer authentication, customer authorization, or production readiness.

## 9. Current implementation slice

The repository now contains these initial contract artifacts:

- [Profile schema](../schemas/vendor-management-endpoint-profile-v1.schema.json)
- [Semantic validator](../tools/validate_vendor_management_endpoint_profile.py)
- [Synthetic fixture](../tests/fixtures/vendor-management-endpoint-profile-v1.json), using the reserved `.invalid` domain and non-secret references
- [Adversarial tests](../tests/test_vendor_management_endpoint_profile.py)
- Workflow entry in [hardware-portfolio.yml](../.github/workflows/hardware-portfolio.yml)

The validator performs contract checks only and makes no network requests. A source write or schema parse is not a validator/test pass. These artifacts are authored but have not been shown executing successfully in this session; the profile remains `draft`, and no device or credential was contacted. The local commands are:

```sh
python tools/validate_vendor_management_endpoint_profile.py
python -m unittest discover -s tests -p "test_vendor_management_endpoint_profile.py" -v
```

The profile itself is never an approval. Its only lifecycle states are `draft` and `retired`; a field such as `approved_for_lab` or a list of arbitrary evidence strings cannot authorize a connection or operation. Lab approval must be a separate signed authority record that binds the profile digest, exact target, expected principal and tenant/org scope, allowed read-only operations, policy revision, approver identity and expiry. The verifier must authenticate the approver and verify those bindings; merely finding a reference string is not enough. The runtime caller must pin both the trust-store SHA-256 and its monotonically managed revision through an independently protected channel; a digest alone does not prevent an operator or pipeline from intentionally selecting an older trust store that predates revocation. The CLI therefore requires both `--trust-store-sha256` and `--trust-store-revision`. The revision floor/expected value must not be sourced from the same untrusted bundle as the trust-store bytes.

Evidence consumers must compute a profile digest from the complete frozen profile using a single specified canonicalization contract (recommended: RFC 8785 JSON Canonicalization Scheme followed by SHA-256) and place that digest in the external signed work/evidence record. Do not store a self-referential digest inside the profile. The independent verifier must recompute the digest from the exact profile bytes/parsed value and refuse a mismatch.

## 10. Definition of done for the endpoint gate

- A versioned endpoint profile schema and strict semantic validator exist.
- The profile separates deployment jurisdiction from endpoint origin and optional vendor-specific dimensions.
- Adversarial tests cover destination substitution, transport downgrade, identity mismatch, credential scope and privacy leaks.
- The independent evidence verifier binds results to the profile digest and adapter revision.
- Source links and last-reviewed dates are recorded; vendor-specific claims are tested against exact product documentation.
- No operation is marked tested until the relevant checks and evidence are available. A queued workflow, source inspection, standards link or synthetic-only pass is never described as lab qualification.