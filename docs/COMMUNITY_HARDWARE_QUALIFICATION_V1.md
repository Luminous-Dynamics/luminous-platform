# Community Hardware Qualification v1

**Status:** Proposal and synthetic fixture only. No physical device has been tested by this document. This is not a certification, safety approval, vendor endorsement, or deployment authorization.

## Goal

Turn community DIY technology into an enterprise-grade *evidence process*: contributors can run portable, reusable diagnostics; others can reproduce a specific result; maintainers can understand the evidence and limitations; and a customer can choose an exact hardware/software profile with a truthful support boundary.

The paradigm shift is from "this board boots on my desk" to "this exact hardware revision, firmware, software profile and test suite has this reproducible evidence, under these stated conditions, with these remaining blockers."

This is a companion to (not a replacement for):
- [Hardware portfolio and H0-H5 maturity model, PR #22](https://github.com/Luminous-Dynamics/luminous-platform/pull/22).
- [Open-hardware design package plan, PR #22 branch](https://github.com/Luminous-Dynamics/luminous-platform/blob/docs/government-network-hardware-catalog-v1/docs/OPEN_HARDWARE_DESIGN_PACKAGE_PLAN_V1.md).
- [Firmware evidence benchmark, PR #55](https://github.com/Luminous-Dynamics/luminous-platform/pull/55).
- [Open home-systems architecture, PR #50](https://github.com/Luminous-Dynamics/luminous-platform/pull/50), including its pre-hardware qualification and existing evidence classes.

It deliberately does not invent a second hardware maturity scale. The existing portfolio H0-H5 contract remains the product lifecycle: H0 candidate; H1 source-reviewed; H2 buildable; H3 platform-integrated; H4 operationally qualified for a named profile; H5 deployment-authorized for a named customer/environment. A test receipt is an evidence input, not permission to promote itself or the portfolio entry.

## Reuse mature diagnostics formats

Prefer existing upstream diagnostic output formats over a Luminous-only test ecosystem. Evaluate [Open Compute Project Test & Validation](https://github.com/opencomputeproject/ocp-diag-core) and its [Rust API](https://github.com/opencomputeproject/ocp-diag-core-rust) for portable test-step/measurement/diagnosis output when applicable. OCP describes portable diagnostics that can span bring-up, integration, reliability, third-party labs, manufacturing, datacenter operations and RMA. OCPDiag is a framework, not a universal hardware qualification certificate.

A Luminous receipt should wrap or reference the exact raw diagnostic artifact by digest and bind it to exact subject, firmware, test-suite, runner, environment, calibration, limitations and review references. Do not fork or copy the OCP output schema merely to add Luminous provenance. Before adopting an upstream SDK, pin the exact version/revision, inspect its license and maintenance state, and test its schema/API on representative fixtures. A format label never implies conformance unless its pinned validator actually runs.

For openness, record artifact-level source and license completeness. [OSHWA certification requirements](https://certification.oshwa.org/requirements.html) address open-source hardware compliance; an openness label alone is not evidence of performance, reliability, electrical safety, secure boot, firmware support or procurement suitability. Schematics, editable source, BOM, firmware, FPGA bitstreams and third-party blobs need separately stated coverage.

## Community evidence model

Keep three axes independent:

1. **Product maturity:** existing portfolio H0-H5. Only the portfolio qualification owner can update it with exact, reviewed evidence and named-profile scope.
2. **Evidence basis:** reuse the home-systems evidence classes: `designed`, `static_checked`, `unit_contract_tested`, `vm_simulated`, `bench_measured`, `field_observed`, `independently_reviewed_qualified`.
3. **Review state:** a submission may be unreviewed, independently reproduced, maintainer-reviewed, lab-reviewed, disputed or superseded. A contributor's self-declared review state is not authoritative; review records must bind the exact receipt digest and reviewer identity outside the submitted record.

A physical result must never be inferred from a VM/simulator result. A bench run is not a field observation; a field observation is not automatically a qualification; multiple signatures do not prove truth; popular vote or reputation does not replace technical review. A signature can bind an issuer and content but cannot prove the equipment identity, calibration, test integrity or conclusion.

## Receipt contract

The proposed `luminous.hardware-test-receipt.v1` schema is intentionally a run/evidence record, not the hardware catalog.

Each submitted receipt identifies:
- **Subject:** existing portfolio item ID when applicable; manufacturer/model and exact hardware revision; privacy-preserving unit reference; firmware/bootloader/BMC/CPLD/FPGA/radio versions and hashes or explicit unknowns.
- **Test identity:** suite and version, immutable suite source revision, suite digest, runner revision/digest, exact parameter/profile commitments, and diagnostic output format/version.
- **Environment:** VM vs physical mode, OS/kernel/toolchain, flake lock or equivalent deployment identity, network isolation, test conditions, and instrument identity/calibration evidence where a measurement depends on it.
- **Results:** each named test's expected outcome, status, measured values/units/uncertainty/sample count as applicable, and bound evidence-artifact digests.
- **Limits:** explicit unsupported claims, failed/skipped tests, missing instrumentation, sample size, known firmware blobs, supply/BOM issues and environmental boundaries.
- **Privacy and publication:** pseudonymous contributor and unit references; redaction and visibility for each artifact. Never publish raw serials, access tokens, exact customer topology, private diagnostic payloads or sensitive physical location. Sensitive artifacts may remain in owner-controlled storage with a public digest and controlled retrieval reference.

Structural JSON validation is necessary but insufficient: the receiver must check digest contents, revision bindings, artifact access, signature/issuer trust when used, test-policy requirements, stale evidence and reviewer records. Exact-ID uniqueness and cross-field promotion rules require semantic validation beyond JSON Schema. Unknowns stay unknown.

## Test campaign matrix

For each candidate and named deployment profile, write the test plan before looking at the results. Select applicable tests by device risks and intended use; do not pretend one generic suite makes every device enterprise-ready.

- **Identity and source:** exact SKU/board revision/lot where available; immutable source/artifact revisions; per-artifact licenses; complete/partial BOM, approved substitutions, firmware and known closed components.
- **Basic function:** ports, sensors, memory, storage, radio, accelerator and peripheral checks supported by the exact device. Unsupported probes must be reported, not silently skipped.
- **Interoperability:** supported NixOS/kernel/driver versions, boot mode, network/switch/optic peer matrix, protocols and tested workload profiles.
- **Performance:** frozen workload, baseline, warm/cold treatment, sustained results, p50/p95/p99 where applicable, throughput, tail behaviour and error counts.
- **Power and environment:** idle/load energy at a declared measurement boundary, temperatures, thermal throttling, acoustics where relevant, power-cycle behaviour and applicable electrical/environmental safety evidence. Software-only or VM tests cannot qualify these.
- **Security and lifecycle:** update authenticity and freshness, rollback/recovery, key provisioning, default credentials, debug interfaces, management-plane isolation, revocation and end-of-life export.
- **Reliability and repair:** fault injection, soak/power-cycle protocol, recovery time, backup/restore, replacement instructions, repair time, parts availability and known failure modes.
- **Negative controls:** wrong firmware/board, stale source or metadata, corrupted artifacts, expired/untrusted signing key, unsupported revision, missing calibration, wrong measurement unit, changed test parameters, missing evidence, and a deliberately forged "pass" receipt.

Use calibrated instruments for measured claims. Specify sample sizes and acceptance tolerances per profile and risk class. For an initial higher-assurance physical pilot, aim for at least 3 distinct units and replication by at least 2 independent tester groups when units and logistics permit. If that is infeasible, publish the exact smaller sample and restrict the claim; never imply population-level reliability from one board.

## Community participation without community-washing

- Make test plans, runner source, fixture data and report schemas public where licenses and privacy permit. Pin immutable versions and provide a Nix/dev-shell path for repeatability.
- Accept contributions from hobbyists, refurbishers, repair shops, universities, makerspaces, vendors and independent labs, but give every contributor the same evidence requirements.
- Separate reporter, runner maintainer, reviewer and deployment-authorizer roles for higher-risk claims. Require conflict-of-interest disclosure when a reviewer sells the device or wrote the tested firmware.
- Preserve failures and indeterminate runs. Do not rank only by passes, hide denominators, delete adverse reports, or average a critical security failure away.
- Re-run a small random and risk-based sample of community-submitted campaigns. Use blind negative-control artifacts to test whether reviewers and automated filters catch forged, stale or out-of-scope receipts.
- Resolve contradictions by exact model/revision/firmware/environment and recency; do not let vote count or an aggregate reputation score settle a technical disagreement.
- Publish signed review/dispute/supersession records against exact receipt digests. Revoke or supersede evidence when the device revision, firmware, test runner, policy or vulnerability state changes.
- Release contributors' private raw logs only with explicit consent and a redaction/privacy review.

## Existing-project integration contract

| Component | Responsibility | Explicit non-responsibility |
|---|---|---|
| **Luminous Platform** | Hardware portfolio linkages, test receipt schema, qualification workflow, named profile and limitations | Does not declare physical truth from a JSON file or create another actuator |
| **Nixward** | Exact host/deployment observations and configuration changes through its canonical System Transaction; read-only diagnosis by default; recovery evidence where supported | Does not turn community results into authorization or blindly flash arbitrary devices |
| **Sovereign Boot** | Boot/recovery-state evidence on explicitly supported host profiles; optional boot component, fail-open boundary and recovery testing | Not a universal hardware tester; boot imagery or successful boot is not Secure Boot qualification |
| **Sovereign Ops** | Operator workflow for selecting profiles, requesting approval, tracking tests and reviewing receipts; execution delegates to Nixward | No parallel privileged executor. Public contract docs must not expose private implementation details or customer data |
| **Symthaea** | Advisory anomaly triage, selecting follow-up diagnostics and summarizing observed evidence with uncertainty | Cannot mint authority, authenticate its own evidence, change H0-H5 maturity, or certify its recommendation |
| **Mycelix** | Optional cross-organization witness/attestation, dispute and supersession references over privacy-safe evidence digests | Not a universal trust authority; no raw serials, secrets, private customer topology or high-volume telemetry in a shared DHT |
| **Luminous Edge** | Publish/pin tested flake modules and supported profiles; qualify the composed facade and source boundaries | No claim that a meta-flake makes each underlying device or imported component qualified |

For Symthaea-based triage, track precision, recall, false negatives, false positives, calibration and abstention on a frozen labelled corpus. Suggestions remain advisory and must link to observations. For Mycelix, distinguish "issuer authenticated", "content unchanged", "independently reproduced", and "qualified for profile" as separate claims.

## Initial product portfolio

Prefer qualifying existing hardware and modular builds before designing a custom PCB. Keep "open hardware", "open firmware on commercial hardware" and "repairable DIY system assembled from commodity parts" as separate labels.

1. **DIY edge/network kit:** OpenWrt One is a research candidate for a lab/branch gateway; exact revision, source/licensing, BOM, supply, firmware lifecycle, measured VLAN/firewall throughput and regional suitability remain to be verified. No model-wide or production claim yet.
2. **Repairable home/SMB node:** a clearly specified commodity x86/SFF configuration running NixOS, with component-level openness disclosed, exact replacement parts, backup/restore and power/thermal measurements. Do not describe commodity hardware as open-hardware merely because the OS is open.
3. **Storage and recovery kit:** supported disk/controller/enclosure/UPS combinations, tested backup/restore, SMART/error capture, outage behaviour and explicit data-integrity limits.
4. **Open compute/FPGA lab:** exact source-pinned boards for research, hardware bring-up or accelerator experimentation; simulation, FPGA implementation and production-silicon qualification remain distinct.
5. **Root-of-trust and identity:** assess maintained TPM/HSM/security-token options and their exact certificate, firmware and supplier scope. Open RTL or an open specification is not the same as a purchasable, provisioned and validated security module.
6. **Sensors and community networking:** open sensor/mesh/router designs with electrical, RF/regulatory, environmental and maintenance boundaries appropriate to the intended deployment.

Candidates are not commitments. Start with one safe profile and one exact hardware revision; broaden the catalog only as its sourcing, software support and evidence justify it.

## Proposed benchmark gates

- 100% of supported-hardware claims bind to exact model/revision, compatible firmware/software versions, suite/runner digest, profile scope and limitations.
- 100% of required test cases have an explicit pass/fail/skipped/indeterminate result and preserved evidence; skipped/unsupported cannot count as pass.
- 0 false acceptance of deliberately forged, wrong-subject, wrong-revision, stale, corrupted or unsupported test records in the negative corpus.
- 0 unauthorized changes and 0 firmware updates dispatched without exact target, artifact identity, freshness checks, explicit authority and a qualified recovery path.
- 100% of critical security/update negative controls reject invalid signature/role, rollback, wrong hardware identity and manifest mismatch for a profile claiming those protections.
- Reproduce each initial higher-assurance claim with at least two independent tester groups and, where feasible, three physical units; report deviations rather than suppress them.
- Re-run qualification when hardware revision, firmware, runner, kernel/driver, profile, policy or material source changes; stale evidence cannot support a current claim without a defined compatibility policy.
- Publish a complete claim/evidence/limitations report. Do not reduce product quality to one public green badge.

Performance and reliability targets must be profile-specific. Freeze realistic workloads and thermal/power measurement boundaries first; report throughput, p95/p99 latency, errors, idle/load watts, temperatures, availability, recovery time, repair time, sample size and uncertainty. The pilot should measure enough to build a baseline before any cross-product superiority claim.

## Delivery sequence

1. Review this envelope and cross-project ownership boundaries; align it with the H0-H5 catalog and home-system evidence classes.
2. Add a Rust-first semantic validator and a separate oracle; validate schema meta-schema, ID uniqueness, time ordering, profile/subject binding, required evidence by claim class and forbidden maturity promotion.
3. Add a synthetic corpus with positive, negative and indeterminate records; mutation-test the validator and oracle.
4. Implement one no-actuation diagnostic pilot on a pinned DIY edge device or spare lab machine. Capture immutable identity and telemetry, but don't flash firmware or change a host as part of the first campaign.
5. Add optional OCPDiag output ingestion behind a pinned, validated adapter. Keep toolchain and upstream maturity review separate.
6. Integrate Nixward observations and optional boot/recovery evidence; later add Sovereign Ops review workflow, Symthaea advisory triage and Mycelix attestations only through the boundaries above.
7. Promote to H3/H4 only with exact-head implementation and physical evidence. H5 requires separate named-environment/customer authorization.

The accompanying JSON file is a **synthetic example** designed to demonstrate an incomplete VM-only campaign. It is not real hardware evidence and must not be ingested as a true run.
