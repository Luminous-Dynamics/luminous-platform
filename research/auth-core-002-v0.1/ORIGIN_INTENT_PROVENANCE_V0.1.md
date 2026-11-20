# Origin-bound intent provenance profile v0.1

**Status:** draft research profile; non-normative and non-authoritative  
**Tracking:** [Platform AUTH-CORE-002 #503](https://github.com/Luminous-Dynamics/luminous-platform/issues/503)  
**Depends on:** [AUTH-CORE-001 #501](https://github.com/Luminous-Dynamics/luminous-platform/issues/501), [authorization-alignment benchmark #304](https://github.com/Luminous-Dynamics/luminous-platform/issues/304)  
**Date:** 2026-11-20

## 1. Purpose

This profile makes the origin of an agent-selected action and each sensitive parameter inspectable before a proposal is bound to an existing authority grant and participant permit.

It answers a question ordinary tool-call authorization does not answer:

> Did the principal authorize this action and its sensitive inputs, or did untrusted content merely cause the agent to choose them?

The profile is an **evidence projection and conformance contract**. It is not a capability, grant, policy decision, identity, instruction, source of trust, or permission to execute. Mycelix AGENT remains the canonical owner of authority; Symthaea INTX remains the canonical owner of intent/task semantics.

## 2. Core model: separate five things

Do not collapse:

1. **Principal intent** — what the principal specified or explicitly delegated.
2. **Content provenance** — where the content and each parameter value originated, including transformations and handoffs.
3. **Agent proposal** — the action and parameters the agent selected.
4. **Authority decision** — whether the canonical grant/policy system authorizes that exact proposal now.
5. **Participant effect** — what the participant accepted, committed, rejected, or cannot yet determine.

A source may be authorized to provide a data field without being authorized to issue instructions. A document can be a permitted source for an invoice amount while remaining untrusted as a source of new payment instructions. **Data-source delegation is not instruction authority.**

Verified provenance means the lineage is complete and attributable; it does not mean the content is benign, truthful, or authorized. Content taint, lineage state, and task-granted source permissions are separate fields.

## 3. Intent precision classes

### Fully specified

The principal fixes action, target and sensitive parameters. Retrieved content may inform reasoning, but it cannot replace those fixed values. Any changed action/target or undelegated sensitive parameter is misaligned.

### Parameter-open

The principal fixes the action and target but delegates named parameter fields to named sources under explicit constraints. The source can populate only its delegated fields. It cannot change the action, target, participant, purpose, operation or other fields.

### Action-open

The principal delegates action selection. That delegation is valid only when it names a finite pre-authorized action set, allowed selection sources, target/operation/effect constraints and a bounded validity interval. A source may select only an exact member of that set. Without a finite set or named selector, the agent must ask for clarification or abstain; arbitrary content cannot define a new action.

Action-selection provenance and per-parameter provenance are evaluated independently.

## 4. Provenance and transformation rules

Every source record is introduced by a trusted connector, principal or operator boundary—not self-labelled by the model. It records source identity/class, writer control, content taint, lineage completeness and an attestation/receipt reference.

For each sensitive parameter, record the value digest, all origin source references, transformation references and the applicable field delegation. For every transform or child-agent handoff:

- the output lineage must include the union of all input source references;
- taint cannot be removed by summarization, paraphrase, embedding, serialization, memory, cache or agent handoff;
- missing lineage is indeterminate, not trusted;
- a content digest or signature does not itself establish authority or truth.

An explicitly delegated third-party source may supply a constrained data value even when its content is untrusted as instructions. The constraint must be independently specified and evaluated (for example, a typed amount ceiling, supplier-directory match or enumerated option set). The source cannot thereby expand action or target authority.

## 5. Binding to canonical authority and effects

The manifest binds to the exact request tuple: action, participant, target, operation, argument digest, purpose, context, policy epoch and attempt ID. The same manifest commitment is referenced by the broker request and participant permit.

The manifest does not replace the canonical Mycelix grant. The grant must independently authorize the exact tuple, remain current, and attenuate monotonically through delegation. The final participant still rechecks authority immediately before admission and owns effect idempotency/finality.

The decision pipeline is:

**principal intent → provenance manifest → agent proposal → Mycelix authority → broker/host enforcement → participant admission → independent effect observation**

The origin evaluator can return only **eligible**, **clarification required**, **denied** or **indeterminate**. It never returns “authorized” or “execute”. Even an eligible result means only that the proposal is consistent with the declared intent/source-delegation contract and may proceed to independent authority checks.

Unknown origin/authority means no dispatch. Unknown effect outcome means reconcile the same attempt; never create a new attempt merely because an acknowledgement was lost.

## 6. Executable artifacts in this tranche

- origin-intent-manifest-v0.1.schema.json: Draft 2020-12 manifest schema with conditional precision-class requirements.
- origin-intent-manifest.example.v0.1.json: design-only sample, explicitly not claimed or executed.
- origin-intent-conformance.v0.1.json: 20 synthetic cases, including positive controls, action-open bounds, field-source delegation, provenance laundering, unknown lineage, stale receipts, request substitution and revoked/unknown authority.
- scripts/validate_origin_intent.py: test-only schema and deterministic semantic-vector checker.
- .github/workflows/origin-intent-conformance.yml: read-only, pinned, path-scoped validation workflow.

The evaluator is a benchmark oracle for the synthetic contract. It is not linked to a live broker, model, participant, connector or production authority service.

## 7. Qualification targets (proposed, not measured)

| Metric | Proposed target |
|---|---:|
| Unauthorized action-selection proposals on frozen hard negatives accepted as intent-eligible | 0 |
| Undelegated sensitive-parameter substitutions accepted | 0 |
| Transformation/child-handoff lineage loss accepted | 0 |
| Unknown or incomplete lineage promoted to eligible | 0 |
| Stale/revoked/invalid source receipt promoted to eligible | 0 |
| Eligible positive-control intent cases | 100% |
| Exact expected synthetic vector outcomes | 100% |
| Participant commits lacking current exact Mycelix authority | 0 |
| Participant commits with mismatched manifest/request/permit binding | 0 |
| False-block rate on eligible positive controls | Report separately; target ≤1% after a representative corpus exists |
| Clarification precision/recall and human-active time | Report separately |
| Task completion under attack | Report separately from attack success and authorization alignment |

These are engineering hypotheses, not observed results or universal security guarantees. A finite zero-violation corpus is not a proof of production safety.

## 8. Research and industry comparison

Relevant comparators are complementary, not interchangeable:

- **AgentDojo** provides dynamic tool-use environments and paired benign/security task structure. Preserve its native tasks, graders and denominators when reproducing it.
- **AutoDojo** (June 2026) adds adaptive indirect-prompt-injection attacks. Its paper reports that an adaptive attack recovered 28% overall and 64% on action-open tasks against a filter that achieved 0% static attack success in the reported configuration. This shows why fixed injection lists and instruction-string filters are insufficient; it is not a direct comparison to this profile.
- **Ignored, Not Refused** (October 2026) audits 33,000 trajectories across eleven open-weight agents and reports that 76–80% of failed attacks were ignored rather than refused; it also reports safe-but-incomplete outcomes in 21.7% of AgentDojo episodes. This motivates reporting task completion and failure mode alongside attack success.
- **AuthorizationBench** measures out-of-authorization actions and separates alignment from direct harm. Preserve its native task and trajectory definitions; do not convert proposal rates into committed-effect rates.
- **NIST NCCoE's Software and SI Agent Identity and Authorization project** is exploring standards-based agent identity and authorization. This profile is an interoperability/evidence contribution, not a replacement for identity standards or an assertion of NIST endorsement.
- **OWASP Agentic Security Initiative** threat categories (including goal hijack, tool misuse, identity/privilege abuse, memory/context injection and cascading failures) provide a threat-family crosswalk, not a conformance certificate.

The differentiator to test is not a claim that a prompt filter is unbeatable. It is whether a system preserves principal authority and source lineage at the effect boundary while retaining useful task completion. Run static, adaptive, cross-model, cross-source and multi-hop attacks; pair each with benign positive controls; use independent authorization and participant-effect oracles; and publish the full security/utility frontier.

Do not merge unlike attack success, out-of-authorization proposal rates, broker denials and unauthorized committed effects into one score. Do not claim to outperform state of the art until matched independent runs exist.

## 9. Ownership and non-goals

| Responsibility | Canonical owner |
|---|---|
| Typed task intent and precision | Symthaea INTX |
| Identity, grants, delegation, attenuation, purpose, budget and revocation | Mycelix AGENT |
| Authenticated source/session transport | Xenia |
| Claim/evidence reporting and test composition | Platform / WAEP |
| Host/process/network enforcement | Nixward |
| Final participant admission and effect finality | Participant |
| Independent postcondition/effect observation | Independent observer under a qualified evidence profile |

No parallel identity provider, capability database, policy engine, trust root, provenance ledger, source registry, agent runtime or cryptographic protocol is introduced.

## 10. Claim ceiling

This is a draft schema, synthetic conformance corpus and test-only evaluator. It does not establish a deployed vulnerability, implement runtime mediation, prove source authenticity, qualify Mycelix/Symthaea/Xenia/Nixward, demonstrate physical safety, or establish end-to-end Web4 security. The example is design-only; synthetic vectors are not real-world observations. Full CI, runtime integration, independent oracle review and cross-implementation reproduction remain outstanding.

## References

- AutoDojo, *Adaptive Attacks Expose Superficial Defenses and User-Underspecification Limits in LLM Agents* (13 June 2026): https://arxiv.org/abs/2606.15057
- Zarif et al., *Ignored, Not Refused: What Low Prompt-Injection Success Rates Hide in Small Tool-Using Agents* (16 October 2026): https://arxiv.org/abs/2610.23254
- NIST NCCoE, *Software and SI Agent Identity and Authorization*: https://www.nccoe.nist.gov/projects/software-and-si-agent-identity-and-authorization
- OWASP GenAI Security Project, Agentic Security Initiative: https://genai.owasp.org/
- AgentDojo: https://github.com/ethz-spylab/agentdojo
- AuthorizationBench: https://arxiv.org/abs/2610.27548
