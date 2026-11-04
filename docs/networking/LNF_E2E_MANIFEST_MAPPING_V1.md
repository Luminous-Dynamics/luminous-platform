# LNF End-to-End Run Manifest Mapping v1

**Status:** design/mapping proposal, not an executed experiment  
**Date:** 2026-11-02  
**Parent:** Luminous Network Fabric architecture, PR #207  
**Implementation tracker:** Luminous Platform issue #208  
**Purpose:** map already existing broker, federation-topology and measurement-receipt artifacts into one traceable end-to-end run without duplicating their schemas.

## 1. Decision

LNF should define one top-level run manifest that **references immutable subordinate artifacts by repository, commit, path, content digest and semantic role**.

It must not copy a mutable snapshot of every upstream profile and pretend they share the same semantics. A broker benchmark profile describes policy hypotheses; a Mycelix F0 manifest describes a heterogeneous federation topology and fault schedule; a measurement receipt records a specific evidence-bearing run. Those are different artifact types.

The LNF wrapper binds them into one experiment. It does not upgrade their evidence class or claim ceiling.

## 2. Source artifacts inspected

The following exact versions were inspected on 2026-11-02. They are on open draft branches/PRs, not merged release artifacts.

| Artifact | Exact source | Content digest / identity | What it provides |
|---|---|---|---|
| Provider-neutral broker implementation | `luminous-platform` PR #64, commit `85f1d9aa93457dab5b4c4aa372353799ee0984f8` | PR head SHA | Pure Rust policy core, hard exclusions, deterministic intent-based ranking and reasons |
| Broker benchmark profile | PR #64, `profiles/connectivity-broker-benchmark-profile-v1.json` | Git blob SHA `2693cc12985d8612d2f86c049862fec4ca800aaf` | 16 proposed benchmarks; status `draft_targets_not_results` |
| Broker benchmark schema | PR #64, `schemas/connectivity-broker-benchmark-profile-v1.schema.json` | Git blob SHA `a3887217b4505fe38e5b25aba8967ed2e79af415` | Strict Draft 2020-12 schema |
| Connectivity measurement receipt schema | PR #64, `schemas/connectivity-measurement-receipt-v1.schema.json` | Git blob SHA `1e81ae82d1d9082e4d19ff0ed69f6595a62f55b2` | Versioned trial/evidence receipt with explicit privacy and limitation fields |
| Mycelix F0 topology/fault fixture | Mycelix PR #3253, commit `006f7da146b4d1c781967c3ee8a6161b411b633c`, `docs/interoperability/fixtures/MYC_INT_007F_F0_FEDERATION.json` | Git blob SHA `537c6f299be47565c26d30f294c1518b5e8debb1` | Eight logical nodes, three zones, eleven demonstration phases and eighteen fault campaigns |
| LNF architecture | Luminous Platform PR #207 | Run against the exact PR head SHA | Cross-project planes, standards boundaries, evidence tiers and comparator strategy |

References:
- [Broker PR #64](https://github.com/Luminous-Dynamics/luminous-platform/pull/64)
- [Mycelix topology PR #3253](https://github.com/Luminous-Dynamics/mycelix/pull/3253)
- [LNF architecture PR #207](https://github.com/Luminous-Dynamics/luminous-platform/pull/207)
- [LNF end-to-end campaign issue #208](https://github.com/Luminous-Dynamics/luminous-platform/issues/208)

## 3. Semantic boundaries

### 3.1 Broker profile

The existing broker profile includes, among other things:
- `POLICY_HARD_CONSTRAINTS_001`
- `POLICY_EXPLANATION_001`
- `POLICY_REPLAY_001`
- `POLICY_DATA_MINIMIZATION_001`
- `COST_PER_USEFUL_TASK_001`
- `GATEWAY_FAILOVER_001`
- `LOCAL_CONTINUITY_001`
- `SERVICE_AVAILABILITY_001`
- `WIFI_ROAMING_BASELINE_001`
- `VOICE_VIDEO_RESILIENCE_001`
- `STORE_FORWARD_001`
- `ENERGY_PER_USEFUL_TASK_001)
- hardware, private-cellular, eSIM and direct-to-phone qualification gates.

The run manifest references the benchmark profile digest and records which benchmark IDs the campaign actually executes. It must not silently rename IDs or turn baseline-first / authorization-blocked items into failed or passed trials.

### 3.2 Mycelix topology/fault manifest

The F0 fixture defines federation semantics and provenance across eight logical nodes. Its nodes are not automatically the same as LNF's client, gateway, local service and DTN sender/receiver.

LNF reuses:
- stable semantic node IDs distinct from runtime container/host names;
- declared network zones;
- explicit link profiles and fault schedules;
- run generation and source/provenance labels;
- separation of event time, evidence observation time and physical/runtime identity;
- claim ceilings and no-promotion rules.

LNF defines its own topology roles for issue #208. It must not rewrite the Mycelix F0 eight-node fixture or claim that one profile is a semantic substitute for the other.

### 3.3 Measurement receipts

The existing connectivity receipt schema records individual benchmark trials and binds results to a benchmark profile, source, execution environment and artifacts. An LNF run may produce multiple receipts:
- policy-decision receipt;
- gateway failover receipt;
- local-service continuity receipt;
- queue/delivery receipt;
- independent verifier receipt;
- resource/energy measurement receipt.

The LNF run manifest should reference each receipt by exact bytes/digest and record its parent run/scenario. If the v1 receipt schema cannot represent a cross-plane result, create a separately versioned LNF wrapper rather than broadening the meaning of an existing v1 field.

## 4. Proposed LNF run-manifest fields

The first JSON schema should reject unknown top-level fields and validate all bounds. The following is a design inventory, not yet a published schema.

### Run identity
- `schema_version`, `profile_id`, `run_id`, `run_generation`;
- immutable LNF profile/manifest digest;
- creation timestamp as metadata only;
- deterministic seed corpus and fault-scheduler version;
- explicit evidence tier and claim ceiling.

### Source bindings
For each referenced artifact:
- repository and path;
- exact commit SHA;
- exact content/blob SHA-256 or Git blob identity, clearly distinguished;
- semantic role;
- source status (merged, draft PR, issue proposal, release);
- supported schema/interface version.

At minimum bind the broker profile/schema, measurement receipt schema, gateway/hardware profile, Mycelix topology/fault fixture, selected DTN implementation, selected BPv7/BPSec security profile, emulator and observer versions.

### Topology and runtime
- canonical semantic nodes: `client`, `gateway`, `local_service`, `dtn_sender`, `dtn_receiver`, `observer`;
- network zones and allowed reachability;
- WAN adapters with exact link impairment profiles;
- runtime locators (namespace/container/VM IDs) separately from semantic node IDs;
- exact kernel, OS image, Nix closure or container image digests;
- exact software source revisions and configuration digests;
- isolation class proving the lab has no accidental route to a production LAN.

### Workload
- task IDs and intent profiles;
- declared success condition and deadline;
- payload-size corpus and digest;
- data/cost/energy caps;
- whether work is live-only or explicitly durable;
- application idempotency and receipt contract;
- pseudonymous synthetic principals only.

### Fault schedule
- scenario ID and generation;
- target semantic link/node/service;
- trigger and start/stop semantics;
- deterministic seed, where applicable;
- impairment type and parameters;
- expected disposition and negative control;
- recovery trigger and cleanup postcondition.

### Evidence linkage
- input manifest and profile digests;
- raw event log and task trace digests;
- every measurement receipt digest;
- independent verifier version/result;
- missing/failed/aborted trials and denominator disposition;
- artifacts with source path, byte length, SHA-256 and retention role;
- explicit privacy declaration and limitations.

## 5. LNF E2E scenario IDs

Use stable scenario identifiers that map to issue #208. The run manifest selects a subset, but cannot silently change their semantics.

| Scenario | Minimum assertion | Prohibited inference |
|---|---|---|
| E2E-00 baseline | Local and remote tasks are measured on the named topology | A successful baseline proves resilience |
| E2E-01 primary WAN loss | New eligible flows restore on WAN-B within the measured distribution | Existing sessions survived |
| E2E-02 upstream blackhole | End-to-end health detects failure beyond physical link state | Link-up means Internet-up |
| E2E-03 DNS-only failure | DNS failure is distinguished from route and application failure | DNS failure means all IP paths are down |
| E2E-04 all WAN absent | Declared local tasks remain available with LAN/power intact | Global Internet is available |
| E2E-05 delayed durable message | Bounded message remains pending then progresses after route recovery | Queue acceptance means endpoint delivery |
| E2E-06 duplicate/reorder | Receiver idempotency prevents second side effect | Transport is exactly-once |
| E2E-07 expiry/stale authority | Expired or stale-authority traffic follows declared policy | Disconnected verifier knows globally current revocation |
| E2E-08 crash points | Durable state transition and receipts recover without silent loss | A daemon ACK proves application commit |
| E2E-09 quota/full disk | Acceptance stops explicitly at configured resource bounds | Queue grows without limit |
| E2E-10 tenant isolation | Cross-client access is rejected by negative controls | Network reachability proves application authorization |
| E2E-11 telemetry loss | Deterministic fallback keeps essential functions safe | Symthaea/Mycelix/cloud availability is required for basic forwarding |

## 6. Comparison arms

The campaign must declare, before execution:
1. matched baseline gateway policy (exact OpenWrt release/configuration and multi-WAN package);
2. LNF policy core with the same gateway/link hardware and workload;
3. local-only profile with all WANs absent;
4. selected BPv7 adapter/daemon at an exact revision;
5. ION-DTN comparator when the selected BPv7/BPSec profile is interoperable.

No link may be labelled cellular, satellite or fibre merely because an impairment profile resembles its latency or bandwidth. It remains a virtual link with a named impairment model until a physical link is measured.

## 7. Evidence classification and gates

- Static schema validation and Rust unit tests are contract evidence only.
- Namespace/VM or Containerlab runs are emulated-network evidence.
- Router/AP hardware tests are hardware-bench evidence.
- An authorized field trial is field-observed evidence.
- A receipt is not an attestation that the experiment occurred; independent verification must inspect raw evidence and exact subjects.
- `queued`, `skipped`, `not_run`, absent artifacts and missing denominators are not PASS.
- Do not convert synthetic failure counts into field outage probabilities.
- A campaign cannot claim 6G, satellite service, emergency-grade reliability or interplanetary flight qualification.

Initial proposed gates remain those in the LNF architecture and issue #208. The profile's 99% / 30-second failover target, 24-hour local-continuity target and 99% post-recovery DTN delivery target remain proposed acceptance thresholds, not measurements.

## 8. Initial implementation order

1. Pin the current broker profile, schema and receipt schema from PR #64 by exact commit and content digest.
2. Pin Mycelix F0 manifest PR #3253 and map its node/zone/fault identity semantics; do not reuse its eight nodes as LNF's topology.
3. Define the LNF client/gateway/local-service/DTN topology as a separate profile.
4. Add JSON Schema validation and an independent digest/denominator verifier; use Rust and existing validator code where its interface fits.
5. Add the virtual link adapter and deterministic fault scheduler.
6. Run E2E-00 through E2E-11 with raw logs, per-task traces and explicit missing/failed dispositions.
7. Add the DTN adapter only after exact BPv7/BPSec capability has been audited under `kitsune2-transport-dtn` issue #10.
8. Compare matched baseline and LNF arms. Publish Pareto trade-offs and all failures; do not aggregate away a broken hard constraint.

## 9. Non-goals

- No second connectivity-policy engine or measurement receipt v1.
- No clone of Mycelix's eight-node F0 fixture.
- No packet protocol, cryptographic format or BPSec replacement.
- No production router changes, external network load, live RF, SIM/eSIM provisioning or carrier credentials.
- No claim of hardware, field, commercial or interplanetary qualification from design fixtures or virtual links.

