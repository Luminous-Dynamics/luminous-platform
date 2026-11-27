# Source-Anchored Delegation: MasDrift and AIP Comparison v0.1

**Status:** Proposed architecture and benchmark plan. This is not an implemented protocol, standards claim, or measured result.

**Tracking:** Platform #543, authorization/evidence profile PR #542, runner #540.

## 1. Research update

Two directly relevant public works materially sharpen the programme.

### MasDrift

Xu et al., *MasDrift: Benchmarking Authorization Preservation Across Multi-Agent Architectures* (arXiv:2608.07556, 2 August 2026), introduces 600 benign productivity tasks across eight domains, with reserved actions, sensitive audiences and explicit critical constraints. It evaluates single-agent, centralized supervisor-tree and decentralized peer architectures across nine coordination conditions and six model configurations.

Its metrics include:
- required-step completion and step coverage;
- unauthorized action attempts and executed actions;
- over-disclosure of sensitive content;
- constraint loss and first handoff where it occurs;
- required-call block rate and token overhead for defenses.

The latest v3 paper revision reports 600 tasks across eight domains and substantially different outcomes by coordination architecture. Across its generic conditions, centralized hierarchies achieved 93.9–98.6% completion while unauthorized execution ranged from 2.7–19.8%; peer networks completed 85.7–87.0% with 0.6–0.8% unauthorized execution. In one macro comparison, central supervisors had 97.1% completion and 11.7% unauthorized execution versus 86.1% and 0.7% for peers. Increasing supervisor depth from one to three improved completion by 4.6 percentage points but increased unauthorized execution from 2.7% to 19.8%. The authors' source-reanchoring treatment reduced unauthorized execution across all six model configurations at a pooled 1.6-point completion cost; Chain propagation blocked up to 54.5% of required calls and reduced completion by as much as 36.3 points. The paper's heterogeneous strong-lead/weak-executor case reached 12.5% unauthorized execution versus 0.6% for homogeneous strong-agent teams.

These are author-reported benchmark outcomes, not Luminous measurements. They demonstrate why architecture depth, authority preservation, required-call blocking and utility must be measured together. Use v3 as the pinned comparison basis; older preprint revisions report different values.

Paper (v3, 15 Oct 2026): https://arxiv.org/abs/2608.07556  
Research implementation and MIT-licensed code: https://github.com/ZhuoningXu/MasDrift

### AIP — Agent Identity Protocol

Prakash's *Agent Identity Protocol (AIP): Verifiable Delegation for AI Agent Systems*, draft draft-prakash-aip-01, published 19 August 2026, is an individual informational Internet-Draft that currently states an expiry of 20 February 2027. It proposes Invocation-Bound Capability Tokens (IBCTs), with compact Ed25519/JWT mode for single-hop calls and Biscuit/Datalog chained mode for multi-hop delegation.

Its draft specifies:
- root identity and initial authority in block 0;
- signed delegation blocks with attenuation of tools, budget, domains and expiry;
- bounded delegation depth and non-empty delegation context;
- verification of chain integrity, root binding, depth, attenuation, context, policy and revocation;
- protocol bindings for MCP, A2A and generic HTTP;
- completion records with different verification trust levels.

The paper's earlier March 2026 version reports 600 adversarial attempts and performance figures, but those are author-reported research results, not an independent certification or proof of general deployment safety.

Important limitations explicitly stated by draft -01:
- the token is a bearer credential unless separately bound to proof of possession; the draft says verification does not authenticate the presenter;
- token-specific revocation via CRLs is deferred to v2, while key removal and short lifetimes are the current approach;
- completion can be self-reported, counter-signed or third-party attested, so a completion block does not automatically prove a protected-resource commit.

Draft: https://www.ietf.org/archive/id/draft-prakash-aip-01.html  
Earlier paper: https://arxiv.org/abs/2603.24775

AIP is a useful interoperability and protocol comparator. It should be treated as work in progress, not as an adopted IETF standard.

## 1.1 Existing Luminous prototype discovered

Platform already contains a pure synthetic implementation of both relevant policy arms:

- `benchmarks/itops-autonomy-boundary-v0.1/qualification-core/src/delegation.rs` implements strict edge-by-edge monotonic attenuation.
- `benchmarks/itops-autonomy-boundary-v0.1/qualification-core/src/root_reanchor.rs` implements a separate root-reanchored authorization predicate.

The root-reanchor predicate requires:
- stable root task and plan identity across the chain;
- exact parent task ID, parent task digest and parent grant linkage at every hop;
- issuer-to-parent-subject identity continuity;
- every parent to precommit the exact child task digest and child plan digest;
- the root grant to precommit the complete descendant task/plan manifest;
- fresh, active authority observations for every ancestor, including intermediates;
- exact requested effect parameters, target, operations, data scopes, budgets and approvals within both root and acting-leaf envelopes.

It deliberately does not intersect the final leaf's local effect scope through an intermediate agent's narrower operational role. It does not, however, treat a task manifest as unlimited authority: the descendant must be precommitted at root and parent, the parent must be delegable, all ancestors must remain active, and the actual effect must fit both root and leaf.

Its source tests include the key differential: strict attenuation rejects a precommitted leaf operation because the intermediate role is narrower, while root-reanchor accepts the leaf under the root+leaf envelopes. Negative tests reject descendants absent from the root manifest, task/plan cross-pairings, root/leaf scope expansion, revoked intermediates, and missing/stale/unknown ancestor observations.

This is a promising existing design to reuse and qualify, not a production authorization service. Its observations are supplied to a pure predicate; it does not cryptographically authenticate the authority observations, issue/sign capabilities, authenticate the request presenter, or enforce revocation at a live participant commit. The qualification-core README says its 52 test sources have not been compiled or run. There is also no public Cargo manifest or CI workflow in that subdirectory; issue #545 tracks this executable-closure blocker. The root-reanchor predicate currently validates request_digest syntax but does not recompute it from the pending effect fields; issue #546 tracks binding the permit to a verified canonical act from the act-bound profile.

The existing [Act-bound crypto profile v0.2](https://github.com/Luminous-Dynamics/luminous-platform/blob/main/research/agent-effect-gate-v0.1/ACT_BOUND_CRYPTO_PROFILE_V0.2.md) already separates artifact provenance, SPIFFE workload identity, attestation, act-specific authorization handles, participant atomic finality and independent service truth. It explicitly states that no cryptographic verifier is wired into the reference sink yet. Issue #544 should integrate root-reanchor with this boundary model rather than write a parallel predicate.

## 2. The design conclusion

The correct architecture is neither "put all authority in the source policy and ignore delegation" nor "trust every delegated policy chain to preserve the source boundary."

Use three independently checkable objects:

1. **Source authorization policy (P0)** — the immutable, owner-authorized boundary derived from the original request, resource/task scope and any explicit approvals. It lives outside the agent coordination graph.
2. **Invocation-bound delegation proof** — cryptographically identifies the issuer, delegate, workload, parent/child lineage, branch assignment, capability scope, depth, budget, expiry, revocation generation and exact invocation context. It proves how the call arrived and which branch has been assigned authority. It cannot rewrite P0.
3. **Participant-enforced effect receipt** — proves whether the exact operation actually committed at the protected resource, and binds the commit to actor, tenant, resource, arguments, policy generation, credential generation and transaction/state revision.

A call is allowed only when the source policy, branch assignment, live delegation proof, identity/proof-of-possession, consent state, call-time policy decision and participant enforcement all agree.

The source policy is authoritative. Delegation proofs supply identity, provenance and an attenuated branch assignment. Neither a textual handoff nor a signed token alone authorizes an effect at the resource.

## 3. Why the hybrid is better than naïve chain propagation

MasDrift's chain-only treatment exposes the risk of cumulative under-authorization: every parent may narrow its child policy because it cannot predict all the legitimate operations required downstream. The resulting chain remains secure by denying work, but utility collapses.

The proposed hybrid uses a **source-anchored capability issuer**:

1. Each branch receives a task packet and an explicit branch assignment.
2. The child requests a capability for the exact operations/resources/arguments needed by its assigned branch.
3. The independent issuer evaluates the request against P0, the parent's delegation right, the branch contract, budget, expiry, user-consent state and current revocation epoch.
4. If the request is already authorized by P0 and the parent is allowed to delegate that branch, the issuer mints a child capability with the exact needed scope and an append-only lineage link.
5. If the request exceeds P0, is ambiguous, requires approval, exceeds budget, or cannot be attributed to the current branch, the issuer returns Require-Confirmation or Deny.
6. Every actual call is re-evaluated against P0 and the current capability at the enforcement boundary. A stale or absent decision never becomes authorization by inference.
7. The protected resource validates the invocation identity and the current revocation/fencing epoch at the commit boundary. A response-loss/unknown commit is reconciled before a new mutation.
8. The independent observer emits an effect receipt; a signed completion record may reference that receipt but cannot replace it.

This avoids treating a lossy handoff as an authority source and avoids blindly intersecting the final call with an accidentally over-narrowed prompt-derived policy. It does **not** allow a child to regain authority that the original principal withheld: only the source issuer may re-issue a properly scoped child grant, and only within P0, the delegator's authorized branch, the consent/approval state and the current revocation epoch.

The policy compiler itself is security-critical. It must not silently promote ambiguous natural-language intent into permission. Ambiguous scope, audience, irreversibility, financial amount or external recipient defaults to Require-Confirmation, with the exact pending call digest shown to the approving principal.

## 4. Proposed invocation-bound authority envelope

The reference wire contract should bind at least:

- root_authority_digest and source-policy version;
- original principal, tenant, task and branch/assignment digests;
- issuer, caller, delegate and executing workload identities;
- parent capability digest, child capability digest and signed delegation lineage;
- tool ID, operation, canonical arguments digest, resource/destination digest and audience;
- action stage: current-request, post-approval or never;
- explicit consent/approval receipt digest where required;
- capability expiry, delegation depth, budget, one-use nonce and revocation/fencing epoch;
- runtime/PEP generation and proof-of-possession binding (e.g. mTLS or signed request bound to workload identity);
- policy decision trace digest and exact PDP verdict;
- participant transaction ID/state revision and independent effect receipt reference.

AIP is a useful source for signed identities, append-only attenuation, bounded depth, purpose/context, policy and protocol bindings. Luminous adds the independent source-policy anchor, current commit-time revocation/fencing check, and effect-level evidence. For high-impact operations, use proof of possession rather than a bearer token alone. Keep key revocation, token/capability revocation, parent grant revocation and participant commit barriers as distinct states.

## 5. Reference-monitor verdicts

The monitor returns exactly one of:

- ALLOW: the exact call is within P0, the branch/capability, identity, audience, budgets, consent and current policy state;
- REQUIRE_CONFIRMATION: the source policy permits the action only after a fresh human decision or the scope is ambiguous;
- DENY: the action exceeds the source policy, branch assignment, capability, resource, tenant, budget, revocation state or runtime boundary;
- UNKNOWN_RECONCILIATION_REQUIRED: a previous consequential effect may have committed but the evidence is incomplete or conflicting.

An unknown state never falls through to allow. Retries must preserve the exact idempotency key and request digest; a new key cannot be used to bypass the unresolved operation.

## 6. Benchmark integration

Use MasDrift as the direct authorization-preservation benchmark, AgentDyn for dynamic prompt-injection utility/security, and the ATR causal graph/effect profile as the new evidence layer.

Preserve MasDrift's native conditions:
- single agent;
- peer widths N2/N4/N8 and typed peer condition;
- centralized supervisor depths C1/C2/C3 and typed C2 condition;
- model/persona combinations and original 600-task/8-domain dataset;
- defenses none, authorizer (Source), chain, and both;
- required-step completion, unauthorized attempts/executions, over-disclosure, constraint loss / first handoff, required-call block rate and token overhead.

Add one matched fifth defense arm: source_anchor_capability_receipts (proposed hybrid). Do not change MasDrift's original labels or treat its LLM judge as a protected-resource oracle.

For every arm, capture:
- protected-state and authorization oracles independently;
- attempted vs executed reserved calls;
- sensitive audience violations;
- first handoff losing the constraint;
- required calls physically blocked;
- completion, step coverage, token/tool-call cost and policy-evaluation latency;
- delegation depth, handoff count, capability issue/refresh/revoke counts;
- unauthorized committed effects and independently verified effects;
- p50/p95/p99 revocation-to-downstream-denial and unknown-state reconciliation latency.

The ATR graph fixtures now explicitly include three outcome classes:
1. authorized hierarchical work succeeds;
2. tainted memory reaches a downstream reserved action that commits without source-policy/PEP authorization;
3. a child grant is over-restrictive and safely blocks required work, so task utility fails while authorization remains intact.

This avoids conflating security by blocking everything with high-quality authorization.

## 7. Proposed benchmark targets

These remain hypotheses until the matched matrix is run:
- zero unauthorized committed effects;
- zero over-disclosure outside task audiences;
- zero post-revocation commits after the defined participant barrier;
- zero false verified-effect receipts;
- 100% rejection of delegation scope widening, invalid identity and stale/revoked capability negatives;
- source-policy decisions consistent with adjudicated reserved-action labels;
- required-call block rate no worse than the source-anchored baseline, with a target within 1 percentage point on matched tasks;
- task completion at least equal to the source-anchored baseline within its confidence interval;
- p95 source-policy decision and proof verification latency within the declared SLO;
- independent effect receipt coverage of 100% for high-impact mutations.

Report security hard gates and utility separately. No weighted aggregate may compensate for an unauthorized commit; no low violation rate may hide a high required-call block rate.

## 8. Known limitations and open tests

- Source-policy compilation can itself be wrong; test ambiguous intents, missing context, conflicting constraints and policy mutations against human-adjudicated labels.
- Delegation tokens do not prove presenter identity without proof of possession.
- Key revocation is not the same as capability revocation or revocation of already active sessions/connections.
- A policy decision at request time is not enough if revocation or policy generation changes before commit.
- A locally durable receipt can be rolled back with a privileged database restore; bind high-assurance state to independent remote high-water marks.
- A cryptographic chain proves what was signed, not that an operator/KMS/build/backup domain is independent.
- A tool's completion message or agent-signed completion block is not independent proof of resource commit.
- Uninstrumented tools and alternate resource paths remain explicit residual assumptions.

## 9. References

- MasDrift paper: https://arxiv.org/abs/2608.07556
- MasDrift implementation (MIT): https://github.com/ZhuoningXu/MasDrift
- AIP draft -01 (individual informational Internet-Draft): https://www.ietf.org/archive/id/draft-prakash-aip-01.html
- AIP earlier paper/evaluation: https://arxiv.org/abs/2603.24775
- OWASP Top 10 for Agentic Applications 2026: https://genai.owasp.org/top-10-for-agentic-applications-2026/
- Google Agent Anomaly Detection benchmark study (12 Nov 2026): https://developers.googleblog.com/en/how-agent-anomaly-detection-scores-on-benchmarks-we-didnt-write/

External benchmark claims and draft protocol descriptions are attributed to their authors. This document proposes an integration and evaluation plan; it does not claim Luminous has implemented or passed the hybrid.
