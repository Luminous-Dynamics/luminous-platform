# OMSI 0.1 threat model

**Status:** draft. This is not an independent security assessment or certification.

## Assets

Customer tenant boundaries; target systems; identities and delegated authority; exact approved plans; credentials and secrets; service availability; integrity of lifecycle history; provenance and truthfulness of evidence; data portability; provider-to-provider trust relationships.

## Trust boundaries

1. Human/customer intent to a service desk or planner.
2. Advisory intelligence to policy and authorization.
3. Authenticated requester to OMSI API.
4. Provider control plane to tenant-specific executor.
5. Authority issuer to executor.
6. Executor observations to evidence verifier.
7. One provider's trust domain to another provider.
8. Portable exports to importing provider or auditor.

## Adversaries and failure modes

- Malicious or compromised provider administrator.
- Compromised RMM/PSA server attempting lateral movement across tenants.
- Stolen, expired or revoked principal key.
- Replay of an old approval or action.
- Time-of-check/time-of-use: target/configuration changes after approval.
- Misbound tenant or target causing a confused-deputy failure.
- Agent loss after command acceptance and before result receipt.
- Executor claims success without satisfying postconditions.
- Verifier shares the executor's compromised identity or control path.
- Extensions attempt to redefine core semantics.
- Secrets leak through arguments, events, logs or exports.
- Prompt injection or model output is treated as authority.
- Duplicate/out-of-order events or partial apply cause state confusion.

## Required mitigations for an Operations-conformant implementation

- Bind tenant, target, action, audience, manifest digest and authenticated principal at execution time.
- Recheck grant expiry, issuer trust and revocation immediately before mutation.
- Use nonce/replay protection and unique idempotency semantics.
- Bind approval to the exact canonical plan; any plan change requires fresh approval.
- Separate advisory planning, authorization policy, execution and verification.
- Reconcile target state before retries after an indeterminate result.
- Require postcondition evidence for verified; reject missing or temporally impossible observations.
- Where independent verification is claimed, use separate principals and independent control/failure paths validated by deployment policy.
- Strictly validate extensions and prevent unknown extension values from altering core semantics.
- Keep secrets out of manifests, events and receipts; use a separate secret-distribution mechanism.
- Record tamper-evident events and apply retention/access controls appropriate to customer risk.
- Include negative multi-tenant tests, failure injection and fuzzing.

## Residual risks and non-claims

A valid signature does not prove an action is safe. A schema-valid receipt does not prove the underlying observation is true. A different verifier name does not prove independence. A valid artifact digest does not establish publisher trust. The starter does not establish signature verification, identity binding, key storage, channel security, endpoint hardening, production multi-tenancy, revocation infrastructure or correctness of an endpoint agent.

AI confidence, Symthaea scores, Mycelix reputation, blockchain consensus and human-readable approval text are not authorization primitives.
