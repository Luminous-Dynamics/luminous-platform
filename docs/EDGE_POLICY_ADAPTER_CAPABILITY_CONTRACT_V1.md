# Luminous Edge device-adapter capability contract v1

**Status:** capability-qualification logic and tests; not vendor certification or physical-device qualification.

## Paradigm shift: declare guarantees, not product names

A device adapter must declare what the device and its management protocol actually guarantee. “The API returned success” does not establish effective-policy readback, service reachability, configuration isolation, or recovery. Luminous should qualify these properties independently and refuse automatic rollout when a required guarantee is missing.

The Rust module is `tools/edge-policy-observation-reconciler/src/adapter_capability.rs`. It classifies:
- **Transaction semantics:** native NETCONF confirmed commit, single-request gNMI Set transaction, or unclassified behavior.
- **Rollback semantics:** native confirmed-commit timeout, explicit idempotent rollback to a recoverable baseline, manual-only recovery, or no rollback.
- **Concurrency control:** target-side generation fencing or an exclusive configuration lock.
- **Observation:** exact effective-policy readback, independent data-plane probe, and an independent positive-control path.
- **Deployment requirements:** minimum rollback timeout, whether explicit rollback is allowed, and whether target-side fencing is mandatory.

The result is a structured assessment with blockers and claim limits, not a command to change a device.

## Protocol distinctions supported by external standards

### NETCONF confirmed commit

[RFC 6241, Section 8.4](https://www.rfc-editor.org/rfc/rfc6241.html#section-8.4) defines confirmed commit: absent a confirming commit before the timeout, a device reverts the configuration. The protocol includes a cancel-commit capability and describes session-loss/reboot rollback behavior; the RFC also warns that shared configurations can cause other changes to be altered or removed, making locking relevant.

The adapter qualification therefore refuses automatic rollout when the declared timeout is shorter than the deployment recovery window, the necessary cancel/lock behavior is absent, or the adapter does not declare the expected revert-on-session-loss semantics. Those profile fields must be populated from real target capability negotiation and adapter qualification.

### gNMI Set transaction

The [OpenConfig gNMI specification](https://github.com/openconfig/reference/blob/master/rpc/gnmi/gnmi-specification.md) treats the changes within one Set request as a transaction. This does **not** establish a transaction across multiple devices, nor does it supply NETCONF confirmed-commit timeout semantics by itself.

For gNMI, the v1 gate requires declared single-request atomicity and a separate explicit rollback whose operation is idempotent and whose approved baseline is recoverable. A stricter deployment can disallow this mode or require native device-side generation fencing.

### Differential preflight

[Batfish differential questions](https://batfish.readthedocs.io/en/latest/notebooks/differentialQuestions.html) compare reachability and filter behavior between snapshots. That belongs in pre-change impact prediction. This adapter gate answers a different question: *does this specific target adapter claim the minimum transactional, rollback, readback and probe capabilities required for the selected rollout policy?*

The intended architecture composes the two rather than presenting either one as sufficient.

## Initial qualification rules

| Declared adapter mode | Eligible only when |
|---|---|
| NETCONF confirmed commit | Timeout meets configured recovery window; cancel-commit and candidate locking are supported; revert-on-session-loss is declared; rollback mode matches native confirmed commit; required readback/probes/concurrency controls exist. |
| gNMI single Set | One-request atomicity is declared; explicit rollback is permitted by deployment policy; rollback is idempotent; the exact baseline is recoverable; required readback/probes/concurrency controls exist. |
| Manual-only rollback | Never eligible for automatic rollout. |
| Unclassified transaction behavior | Never eligible for automatic rollout. |
| No rollback | Never eligible for automatic rollout. |

Every result states that the profile is an adapter assertion, not authentication or hardware evidence. Device-specific contract tests are still required; test every supported firmware/capability combination rather than extrapolating across a product family.

## Tests and benchmarks

Unit tests exercise qualified NETCONF and gNMI profiles, undersized confirmed-commit timeout, missing locking/session-loss behavior, absent rollback, unclassified behavior, policy that requires target-side fencing, missing effective-policy readback, missing positive controls, invalid profile identifiers, and deployment policy disallowing explicit rollback.

Proposed qualification targets—not achieved results:
- Zero automatic-eligibility decisions for malformed, missing, contradictory, or insufficient required capabilities.
- One conformance fixture per supported adapter, protocol mode, vendor model, firmware and capability-negotiation response.
- Inject configuration timeout, controller disconnect, device reboot, stale readback, concurrent writer, failed probe and failed rollback.
- Measure rollback completion p50/p95/p99 and actual service interruption separately from RPC latency.
- Run at least 30 randomized paired trials per qualified hardware/firmware/topology tuple and report every failure and inconclusive case.
- Never count self-reported capability fields alone as proof; produce retained test evidence tied to the adapter version and target firmware.

## Non-goals and limitations

This is a preflight contract, not a NETCONF or gNMI client and not a vendor capability-discovery implementation. It cannot verify the truth of capability assertions, create a target-side lease, or guarantee the physical device honors advertised semantics. The production adapter must implement negotiation, locking/fencing, effective-policy readback and independent probing; test each operation against the actual device; and update its profile when firmware or configuration changes. The orchestration layer must bind that profile and its qualification receipt to the target identity and adapter version.
