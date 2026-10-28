# DSE-003 — Repair-Aware Field Device Contract

**Status:** Design proposal; not implemented on hardware.  
**Related:** Luminous-Dynamics/luminous-platform#128 and #131.

## Thesis

Ruggedness, repairability and verified recovery are often evaluated separately. The product-level opportunity is a field device that carries an auditable record of the protection envelope it has actually been tested to meet after each repair.

Do not claim a device retains an ingress/drop/environmental rating merely because a replacement part fits. If a repair disturbs a seal, hinge, display mounting or chassis joint, the service record must mark which claims remain supported and which require requalification.

## Service-evidence receipt

A service receipt should include:

- Device model/SKU, serial identifier, hardware revision and configuration digest.
- Event ID, service time, operator/tool identity at an appropriate privacy level, and service-procedure revision.
- Failed module, part number/lot, removed component and replacement component IDs where available.
- Gaskets/seals/adhesive parts replaced and the surfaces/fasteners disturbed.
- Procedure steps performed and any deviations.
- Tests actually run: boot/core function, display/touch, battery safety, ports, sensors, thermal check, offline recovery, and ingress testing where applicable.
- Test fixture/profile/calibration identifiers, outputs and pass/fail results.
- Environmental claims still supported, claims suspended pending retest, and the conditions not tested.
- Receipt digest, signer identity and signature format; append-only receipt chaining or an independently witnessed store where appropriate.

A receipt is evidence of a documented service/test event, not proof that an untested property is safe. Signatures authenticate who attested to a receipt and whether its bytes changed; they do not guarantee the truth of the attestation. Independent audit and calibrated testing remain necessary.

## Example state machine

`InService -> FaultReported -> RepairInProgress -> FunctionVerified -> EnvironmentRequalified -> InService`

If the repair does not disturb environmental seals, the workflow may skip an ingress re-test only when the test plan explicitly allows that repair class and seal integrity is verified. If a seal or structural member is disturbed and no required re-test runs, transition to `FunctionVerified` but retain `EnvironmentRequalificationRequired`; do not silently show full qualification status.

Failures transition to `Quarantined` or `RestrictedOperation`, with exact restrictions recorded. A failed battery-safety check must not be papered over by a generic “repair complete” flag.

## Prototype experiment

1. Capture factory baseline device ID/configuration and declared functional/environmental test profile.
2. Run baseline function suite and the agreed ingress profile.
3. Perform one repair class using published instructions; record module IDs, part/lot and seal changes.
4. Verify boot and core functionality locally, without cloud access.
5. Generate a signed service receipt and validate its signature/chain independently.
6. Run required post-repair environmental tests and compare against baseline.
7. Verify the UI reports qualification only for tests actually passed and exposes suspended claims.
8. Repeat on multiple devices and technicians; report repair-time distribution, test failures, receipt-validation failures and post-repair ingress outcomes.

## Benchmark metrics

- Time from fault report to restored service, split into diagnosis, repair, requalification and logistics.
- Repair success without motherboard replacement.
- Post-repair functional and ingress pass rates.
- Receipt completeness, offline verification rate and false “fully qualified” status rate.
- Time to detect an invalid/stale receipt and recovery from a missing/offline evidence store.
- Cost per verified operating hour, including test/repair labor and downtime.
- Offline recovery success after storage replacement or damaged system configuration.

## Industry comparison

Current products already cover parts of this concept. Panasonic TOUGHBOOK offers modular rugged configurations and model-specific environmental claims; Framework offers extensive repair documentation and replaceable modules; Fairphone offers phone-part repairability and a documented support horizon. This design does not claim these vendors lack service records or internal quality processes. The benchmark asks whether a public, offline-verifiable, repair-triggered qualification record reduces uncertainty and speeds field triage without making unsupported durability claims.

- Panasonic TOUGHBOOK 40: https://eu.connect.panasonic.com/gb/en/toughbook/toughbook-40
- Framework Laptop 13 Pro: https://frame.work/laptop13pro
- Framework repair/install guides: https://guides.frame.work/c/Framework_Laptop_13_Pro
- Fairphone Gen 6: https://www.fairphone.com/the-fairphone-gen-6

## Go/no-go

Advance only if the receipt accurately distinguishes tested from untested properties, remains verifiable offline, and reduces time to correct operating/qualification state against an undocumented baseline. A receipt that always says “pass,” is not tied to hardware revision, or fails to mark disturbed seals is a failed design.

## Evidence boundary

This is a design specification. There is no working service-receipt implementation, physical prototype, independent ingress result or measured time/cost improvement yet.
