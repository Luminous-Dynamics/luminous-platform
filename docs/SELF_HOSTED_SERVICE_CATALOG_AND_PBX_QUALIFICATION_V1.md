# Self-Hosted Service Catalog and PBX Qualification v1

**Status:** Proposed contract and M0 example. No service in this document is declared production-qualified.

## Decision

Keep the **deployment profile** and the **self-hosted service profile** as separate contracts.

- The deployment profile describes the host/fleet trust boundary: who may authorize mutations, which coordinator dependencies are allowed, data-export defaults, and what evidence a platform claim needs.
- A self-hosted service profile describes one workload: source pinning, runtime choice, network endpoints, secret references, persistent state, backups, health signals, lifecycle semantics, and service-specific qualification checks.
- A deployment adapter composes these contracts but must not upgrade a claim's maturity merely because the host profile is valid.

The initial service contract is defined in `schemas/self-hosted-service-profile-v1.schema.json`. The proposed Asterisk profile is `profiles/services/asterisk-pbx-nixos-v1.yaml`; it is deliberately M0.

## Runtime backends

Use a small number of explicit backends, not a single universal packaging assumption.

| Backend | First use | Required boundary |
|---|---|---|
| `nixos-native` | Best fit when a suitable NixOS module/package exists; Asterisk is the first candidate | Pin the host's nixpkgs flake input; record the exact revision and built derivation; evaluate and test the rendered configuration. |
| `oci-container` | Applications primarily distributed as OCI images | Require an immutable image digest, explicit user/network/volume/capability policy, and lifecycle management through a defined runtime adapter. Podman Quadlet is a candidate because it generates systemd-managed units from container/network/volume files. Verify cgroup v2 and the selected rootful/rootless unit model on the target. |
| `virtual-machine` | Appliance distributions and applications with a defined guest operating-system install path | Pin/identify the guest image and installation inputs; isolate networking; version snapshots; test restore and guest upgrade. The host must not treat a VM snapshot as proof of application-data consistency. |

Do not begin with Kubernetes. Native systemd/NixOS services, Podman Quadlet, and dedicated VMs cover different real deployment constraints with less operational surface area.

## First workload: Asterisk, then FreePBX

Nixpkgs includes a NixOS Asterisk service module at `nixos/modules/services/networking/asterisk.nix`, with options for enabling the service and defining configuration files. That makes native Asterisk a useful first integration target; it does **not** prove that a proposed dialplan, bind address, trunk, or firewall is correct.

Start with a private lab network and synthetic endpoints. Do not open public SIP/RTP as a side effect of installation. The profile currently marks UDP/5060 and a bounded RTP range as private-network examples, binds them to an illustrative RFC1918 lab address, restricts declared sources to an example private subnet, disables TCP/5061 and HTTP management, prohibits public exposure by default, and defaults call recording off. The addresses/CIDRs are examples to replace with the exact deployment plan; the validator rejects wildcard binds and broad source ranges, but cannot prove the rendered configuration or host firewall actually enforce them. The exact listeners, routing, and effective firewall must be observed before test claims are made.

For a full-featured web-managed PBX, use a dedicated VM as the first compatibility boundary. FreePBX 17's official installation path specifies Debian 12 and a minimum of 2 GB RAM / 20 GB disk. This is a reason to begin with a Debian guest appliance rather than make full FreePBX containerization a prerequisite for the first platform milestone.

### PBX-specific safety rules

1. **No public exposure by implication.** A service manifest is not an authorization to open a firewall or router. Public endpoints require a separate operator decision, an assessment reference, exact ports/protocols, and target-bound activation evidence.
2. **Network security and call authorization are separate gates.** Asterisk's official security guidance warns that poor configuration can permit unauthorized calls and create substantial charges. Test both network access and dialplan/context permissions.
3. **No secrets in profiles or evidence.** The profile contains secret *references* only. Never include SIP passwords, trunk tokens, private keys, or plaintext rendered configuration in Git, command-line arguments, telemetry, or public receipts.
4. **Constrain RTP explicitly.** Choose the media range for the deployment, configure Asterisk to match it, and apply matching firewall policy. Test bidirectional media and one-way/no-audio failures instead of considering SIP registration sufficient.
5. **Do not enable recording by default.** Call recording needs an explicit feature decision, notice/consent design, retention controls, restricted access, and a jurisdiction-specific review. Emergency calling, regulatory duties, and carrier terms remain out of scope until assessed for the actual deployment.
6. **Preserve local service availability.** Mycelix, Symthaea, a UI, telemetry exporters, and remote orchestration are not runtime dependencies for call handling, local diagnosis, or the out-of-band recovery procedure.
7. **Treat rollback honestly.** A NixOS configuration rollback does not automatically reverse call-detail records, voicemail, schema migrations, or recordings. Backup/restore must be application-aware and exercised in an isolated target.

## Workload profile contract

The versioned schema requires:

- Stable service identity and upstream provenance.
- Explicit runtime kind, architecture, update strategy, and a source-pin mode compatible with that runtime: NixOS-native requires the host flake-lock mode, OCI requires an immutable image digest whose reference embeds the same digest, and VM appliances require a concrete guest-image digest. An unresolved pin is allowed only at M0/M1. NixOS claims above declaration-only maturity require a recorded build/derivation or closure digest.
- Authority and control-plane independence assertions.
- Explicit dependencies, whether each is required to start, data shared with it, and a declared unknown-dependency policy. A platform control plane, coordination service, or telemetry exporter cannot be required for workload startup.
- Per-endpoint transport, exactly one port or port range, exposure, enablement, explicit bind address, allowed source CIDRs, and purpose. Supported transports for these endpoints are TCP, UDP, and SCTP; ICMP is not a socket endpoint and is intentionally outside this schema. Each endpoint is explicitly either a workload `listener` or an `ingress-route` served by a declared reverse-proxy dependency. Ingress routes pin that dependency and a DNS hostname set; unknown/non-proxy references and duplicate normalized hostnames on the same proxy/port are rejected. Enabled listeners cannot use wildcard/multicast bind addresses; private-network endpoints require a bind address in RFC1918 IPv4 or IPv6 ULA space and source CIDRs wholly inside the same private address family. CIDRs with host bits set are rejected instead of silently normalized. IP addresses are canonicalized before collision checks so alternate IPv6 spellings cannot bypass overlap detection. Public exposure approval must apply to an endpoint that is currently declared public; stale, unscoped approval is rejected. Duplicate IDs, repeated health-check/dependency IDs, and overlapping enabled port claims on the same bind/protocol are rejected. A profile cannot prove the host firewall or proxy actually implements these declarations—the runtime verifier must compare observed sockets, vhost routes, and effective firewall policy.
- Data classification, persistent paths, backup policy, restore-test requirement, optional sensitive-capture policy, telemetry export, and retention owner.
- Secret backend and reference names, with `values_in_profile: false`.
- Installation, upgrade, rollback, uninstall, and data-migration declarations.
- Health checks across process, readiness, functional, security, backup, and recovery behaviors.
- Maturity, required checks, qualified/unqualified capabilities, evidence references, and limitations.

The schema rejects unknown fields. The Python validator reuses the existing strict deployment-profile YAML parser (duplicate keys, aliases, anchors, and explicit tags are rejected) and canonical profile digest implementation. It additionally rejects unsafe or internally inconsistent declarations, including incompatible runtime/source-pin modes, unapproved or unscoped public exposure approval, wildcard binds, invalid or contradictory source CIDRs, overlapping listener claims, ingress routes that reference unknown/non-proxy dependencies, malformed or colliding DNS hostnames, missing socket ports, unsupported transports, inverted port ranges, missing periodic backup/restore expectations for stateful services, OCI references whose embedded digest disagrees with the declared digest, and required dependencies on the platform coordination/control plane. Invalid YAML that parses to a non-JSON value (for example, a non-finite float) must yield a bounded invalid report rather than crash while computing the profile digest. The CLI uses the same report keys even when parsing fails; profile digest is null when no canonical profile exists, while schema digest is included if the schema loaded successfully.

The declaration validator can emit `VALID_DECLARATION`, `INVALID`, or `QUALIFICATION_BLOCKED`. It always sets `qualification_evaluated: false`. It is not allowed to emit a runtime-qualified verdict; that requires a separate adapter- and environment-specific evidence evaluator. The schema remains workload-neutral: communication recording is an optional `sensitive_capture` policy, not a mandatory field imposed on file, identity, or developer services.

Run locally:

```bash
python -m pip install --requirement requirements-profile-validator.txt
python -m unittest discover -s tests -p 'test_*.py' -v
python tools/validate_self_hosted_service_profile.py profiles/services/asterisk-pbx-nixos-v1.yaml
```

## Second workload: Nextcloud (stateful collaboration)

`profiles/services/nextcloud-nixos-v1.yaml` exercises a different failure surface: user files are not enough to restore a service when configuration and database state are inconsistent. The profile declares a database, a data filesystem, a reverse-proxy/TLS path, and an optional backup agent as separate dependencies, with the database/filesystem required for startup but the platform control plane not required. Its public exposure remains prohibited and its status remains M0.

This is grounded in the NixOS Nextcloud module (which exposes a native `services.nextcloud` service) and Nextcloud's administration guidance: a complete backup must account for configuration, relevant custom apps/themes, data, and the database, while restoration needs the matching configuration/data/database set. A filesystem-only snapshot must not be treated as a valid restoration test. The service contract now distinguishes a process listener from a reverse-proxy ingress route; the Nextcloud HTTPS entry is modeled as an ingress route referencing the declared `tls-reverse-proxy` dependency and an example DNS name, rather than claiming that the PHP application itself binds port 443. These names and paths are illustrative; exact paths, database engine, version, trusted domains, TLS termination, and migration behavior must be resolved against the pinned module before activation.

## Asterisk qualification sequence

| Gate | Evidence required | Does not prove |
|---|---|---|
| 0 — Declaration | Strict parse, schema validation, cross-field checks, stable profile/schema digests | Buildability or any runtime behavior |
| 1 — Exact source | Locked nixpkgs revision, package/derivation identity, evaluated NixOS config, relevant package/module references | Correct listeners, firewall, calls, or restore |
| 2 — Isolated lab | Effective bind/port capture; service start/restart; private synthetic endpoint registration; permitted and denied dialplan cases; bidirectional audio; controlled SIPp call scenarios | Carrier interoperability, production network quality, or compliance |
| 3 — Fault and recovery | Restart, network loss, coordinator unavailable, disk/resource pressure, backup/restore in a separate VM, and post-restore checks | High availability unless failover was specifically tested |
| 4 — Controlled deployment | Target-bound authorization, frozen source/config plan, explicit operator approval, observed post-state, redacted evidence packet | Independent audit or legal compliance |
| 5 — Independent assessment | Named assessor, stated scope, evidence review, exceptions, and re-test policy | Capabilities outside the assessed scope |

No level advances on a queued workflow, a source file's presence, or an authored test. The evidence evaluator must bind verdicts to the exact service-profile digest, schema digest, source revision, build/derivation identity, target identity, test-tool versions, scenario set, observed results, and limitations.

### Suggested synthetic test set

Use SIPp with reviewed, version-pinned scenarios for local UAC/UAS call setup and termination; require bidirectional media observation for media cases. Include negative cases for an unknown endpoint, wrong credentials, prohibited outbound prefix/context, and management access from an untrusted network. Simulate service restart and disconnection; verify calls fail safely and the operator can recover locally. Restore data into an isolated instance and verify voicemail/configuration/permissions rather than only checking that an archive extracted.

Run all tests only against an isolated test environment or an explicitly authorized target. Never aim high-rate SIP traffic at public carrier infrastructure.

## Catalog expansion order

1. **Foundation:** DNS, ACME/TLS automation, reverse proxy, WireGuard, backup/restore, and monitoring. Each needs its own exposure and recovery profile.
2. **Stateful collaboration:** a file/collaboration service plus database/storage dependencies. Validate application-consistent restore rather than container restart only.
3. **Federated communications:** a homeserver and TURN/network behavior with federation explicitly opt-in.
4. **PBX:** native Asterisk first; dedicated FreePBX VM as a separate appliance profile.
5. **Developer and identity services:** source hosting, password management, and identity, each with a separate key/data recovery contract.

Mycelix can later exchange minimized, consent-scoped provenance/attestation for operational events. It must not receive call contents, credentials, raw call-detail records, or private customer topology by default. A Holochain receipt can establish what was attested and by whom; it does not prove a PBX is secure, an emergency call completed, or an incident is resolved.

## Research references

- Nixpkgs Asterisk NixOS module: https://github.com/NixOS/nixpkgs/blob/master/nixos/modules/services/networking/asterisk.nix
- Nixpkgs Asterisk derivation: https://github.com/NixOS/nixpkgs/blob/master/pkgs/servers/asterisk/default.nix
- Asterisk network security: https://docs.asterisk.org/Deployment/Important-Security-Considerations/Network-Security/
- Asterisk secure-calling tutorial: https://docs.asterisk.org/Deployment/Secure-Calling/Secure-Calling-Tutorial/
- FreePBX 17 installation prerequisites: https://www.freepbx.org/get-started/
- Podman Quadlet/systemd integration: https://docs.podman.io/en/stable/markdown/podman-systemd.unit.5.html
- SIPp project and scenario/performance test documentation: https://github.com/SIPp/sipp and https://sipp.readthedocs.io/en/latest/
- NixOS Nextcloud module and manual: https://github.com/NixOS/nixpkgs/blob/master/nixos/modules/services/web-apps/nextcloud.nix and https://nixos.org/manual/nixos/stable/
- Nextcloud backup and restore guidance: https://docs.nextcloud.com/server/stable/admin_manual/maintenance/backup.html and https://docs.nextcloud.com/server/stable/admin_manual/maintenance/restore.html

**Evidence status at authoring:** current upstream documents and Nixpkgs source were inspected. The new profile/validator files have been committed to a proposal branch, but test execution, Nix evaluation/build, SIP call-flow behavior, physical deployment, restore, and independent review have not been run in this session.
