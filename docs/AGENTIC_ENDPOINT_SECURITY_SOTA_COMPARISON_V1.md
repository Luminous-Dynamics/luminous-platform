# Agentic Endpoint Security: Industry Comparison and Falsifiable SOTA Plan v1

**Snapshot:** 2026-10-29  
**Status:** research benchmark contract; not a comparative test result.

## Executive assessment

Luminous currently has a useful assurance prototype: a Rust capability decision core, a Linux Landlock/seccomp micro-fixture, two descriptor-boundary probes, a frozen agentic endpoint contract, and a verifier for portable evidence bundles. It does not yet have a complete supervised launcher, comprehensive network/credential isolation, cross-platform enforcement, or a matched product-level benchmark.

That means the honest answer to “are we better than SOTA?” is **not yet demonstrated**. Integrated industry runtimes currently provide a broader product surface. Our potential differentiator is a stronger, independently checkable relationship between authorization, actual effect, and provenance, measured against the same attack corpus and benign workload suite.

## Industry reference points

These are descriptions of publicly documented architectures, not endorsements or independent assessments.

| System | Publicly documented controls | What Luminous must not claim without testing |
|---|---|---|
| OpenAI Codex | Sandbox boundaries and approval policy are separate controls; managed configuration, constrained execution, network policy, and agent-native telemetry are described in the May 8, 2026 engineering post. Codex controls also distinguish local command permissions from other tool surfaces. | We have not shown lower escape rate, lower overhead, better usability, or stronger cross-tool coverage than Codex. |
| Anthropic Claude Code | Filesystem and network isolation are explicitly paired; the sandboxing research describes OS primitives such as Linux bubblewrap and macOS Seatbelt, plus a network proxy for restricting outbound destinations. | A pathname-only test or a seccomp denylist is not equivalent to combined filesystem + network isolation. |
| NVIDIA OpenShell | NVIDIA's September 28, 2026 OpenShell 0.1.0 post describes a runtime combining sandboxed execution, controlled service access, credential management, auditability, and formal policy analysis. | A pure Rust policy model is not equivalent to a deployed agent runtime with network and credential controls. |

Primary references:
- OpenAI, [Running Codex safely at OpenAI](https://openai.com/index/running-codex-safely/) (May 8, 2026).
- OpenAI, [Agent approvals and security](https://learn.chatgpt.com/docs/agent-approvals-security) (live product documentation).
- Anthropic, [Making Claude Code safer with sandboxing](https://www.anthropic.com/research/claude-code-sandboxing).
- NVIDIA, [Add runtime controls to AI agents with NVIDIA OpenShell](https://developer.nvidia.com/blog/add-runtime-controls-to-ai-agents-with-nvidia-openshell/) (September 28, 2026).

## Paradigm shift: measure authorization-to-effect integrity

Most feature checklists say what a sandbox intends to block. The benchmark should instead ask four independent questions for every attempted action:

1. **Policy:** Was this exact operation authorized for this subject, resource, target and time window?
2. **Enforcement:** Which OS mechanism actually mediated it? A model's Deny value is not kernel enforcement.
3. **Effect:** Did the action really happen? Observe the filesystem, process tree, descriptor table, socket/packet boundary, credential sink, or durable state from outside the subject process.
4. **Provenance:** Can a separate verifier bind the source commit, workflow identity, environment, receipt and claim ceiling to the result?

A result qualifies only when these layers agree. A test that merely observes an expected exit code is insufficient. A self-declared event inside the untrusted process is not an independent oracle. A signed receipt authenticates provenance; it does not make a false sensor truthful.

## Benchmark matrix

Freeze all attack cases, oracle code, workload manifests and expected dispositions by digest. Execute every applicable case against every runtime/configuration and repeat across supported kernel/OS versions. Keep the corpus vendor-neutral.

| Domain | Positive control | Adversarial cases | Independent oracle |
|---|---|---|---|
| Filesystem | Read and write an explicitly granted canary | Outside-path read/write, symlink swap, path traversal, rename/hardlink, mount changes, chmod/chown/xattr/timestamp mutations | Parent-owned canary digest, mode/owner/mtime/xattr snapshot, mount table |
| Descriptors and process | Explicit IPC descriptor survives when allowlisted | Pre-opened writable fd, inherited directory fd, leaked socket/device/credential fd, fork/exec and multithreaded launcher race | Parent fd inventory and flags; child fd inventory; canary content and inode identity |
| Process authority | Bounded child process with declared environment | Unexpected exec, shell fallback, setuid/setcap, ptrace, namespace changes, unmodeled clone flags, writable-executable mappings | Parent process tree, /proc, namespace/capability state, audit trace |
| Network and data egress | Approved endpoint and protocol succeed | Raw TCP/UDP, DNS rebinding, IP literal, IPv6, redirect, alternate proxy bypass, Unix socket, metadata endpoints, unapproved MCP/service | External network namespace or gateway packet capture and server-side request log |
| Credentials | Least-privilege test credential reaches only its intended service | Environment/argv/file leak, inherited agent socket, log leak, credential reuse after revocation, confused-deputy calls | Canary credentials, sink monitoring, credential broker audit trail |
| Temporal/state | Fresh capability allows within its validity window | Expiry, revocation race, replay, stale policy, rollback, wall-clock reversal, crash/restart | Independent clock/test witness, durable ledger state, post-restart oracle |
| Agent workflow | Benign coding task completes | Prompt-injected tool request, malicious repository instruction, indirect tool response, attempted escape from sandbox | Harness-level command/action trace plus external effect observer |

The benchmark must include allowed operations as well as denied operations. A runtime that blocks everything can score well on a denial-only corpus while being unusable. Conversely, a successful task cannot excuse an unauthorized side effect.

## Quantitative gates to target

These are proposed qualification targets, not measured Luminous performance and not claims about competitors. Publish raw trial counts, seeds, failures, confidence intervals and environment digests.

| Metric | Initial target | Qualification rule |
|---|---|---|
| Unauthorized critical side effects | 0 observed | At least 2,995 independent trials per critical scenario class with no critical event gives a one-sided 95% binomial upper bound of about 0.1%, assuming independent Bernoulli trials. Report dependence and distribution limitations rather than treating repeated identical runs as fully independent. |
| Fail-closed correctness | 100% of defined malformed/stale/unknown cases denied | Mutation tests must demonstrate that every class of missing or contradictory evidence is caught; a green base corpus alone is not enough. |
| Oracle sensitivity | 100% on seeded observable violations | Every violation-injection control must be detected; deliberately corrupt or suppress the oracle and require qualification to fail. |
| False denial on supported benign operations | ≤0.1% target | Use a frozen representative workload; count explicit user-visible failures, not just API return codes. |
| Decision latency | p95 ≤1 ms for a normalized decision on the reference host | Measure 10k+ operations, a 1k-capability policy and multiple resource cardinalities; report p50/p95/p99, allocation counts and CPU. Target is for the pure decision core, not process launch. |
| Runtime overhead | ≤5% p95 regression versus the same unsandboxed workload for low-risk local tasks | Matched CPU, disk cache, kernel, hardware and command mix; include cold-start and tail latency. Security features cannot be silently disabled for the benchmark. |
| Revocation propagation | p95 ≤100 ms, zero post-effective-time critical operations | Test in-flight actions and broker/cache races under load; state where the revocation timestamp is linearized. |
| Descriptor hygiene | No inherited descriptors except an explicit allowlist | Test every supported descriptor class (file, directory, pipe, socket, device, memfd, pidfd, namespace handle); use a separate parent oracle. |
| Network confinement | 0 unapproved egress in the frozen corpus | Run packet capture outside the subject process; include DNS, IPv4/IPv6, redirects and direct-IP controls. |
| Credential exposure | 0 canary-secret disclosures to unauthorized sinks | Instrument environment, argv, files, logs, inherited descriptors and network destinations. |
| Evidence portability | Independent offline verifier returns a deterministic result from downloaded bytes | Exact source SHA, run ID/attempt, receipt schema, checksums, environment and claim ceiling must reconcile. Trusted attestation verification is a separate gate. |

For a zero-event trial count, the stated 0.1% upper-bound target is a useful planning rule, not proof of universal safety. Independence assumptions must be scrutinized; correlated seeds, repeated kernels and shared runner defects weaken the inference.

## Comparison protocol

Each comparison row needs: vendor/runtime version, commit or release digest, OS/kernel, hardware, policy configuration, network mode, user-approval mode, credential setup, exact corpus digest, oracle digest, raw runs, exclusions, and confidence interval.

Minimum baselines:

1. Unsandboxed baseline, used only to confirm that each attack control can produce its expected effect.
2. Linux-native baseline using the documented controls (for example, Landlock/seccomp or bubblewrap), configured to the intended policy.
3. Relevant available agent runtimes, including OpenAI Codex, Claude Code and NVIDIA OpenShell, with versioned settings appropriate to the exact test.
4. Luminous candidate, using the same corpus and independent effect oracle.

Use the same tasks and adversarial payloads. Do not compare Luminous's micro-test to another vendor's full product claims. Do not use a vendor's own logs as the sole oracle for that vendor; capture external effects uniformly. Report unsupported environments as “not tested,” not as passes. Keep all failures, timeouts and crashes in the denominator unless a predeclared infrastructure exclusion is independently evidenced.

## What would justify “better than SOTA”?

Only make that claim after all of the following are met:

- An independent team reproduces the public corpus and result digests.
- Every compared runtime receives the same policies, tasks, positive controls and adversarial trials.
- Luminous has no critical unauthorized effects within the agreed confidence bounds and materially improves at least one predeclared dimension (for example, lower false grants, verifiable provenance, revocation time or overhead) without a compensating regression elsewhere.
- Results include the full distribution and raw evidence; the claim is scoped to exact tested configurations, not the entire vendor product.
- The verifier rejects altered receipts, source mismatches, incomplete oracle evidence and inflated claim ceilings.

Until then, describe Luminous as a prototype adding testable authorization/effect correspondence and evidence provenance, not as a replacement for or improvement over Codex, Claude Code or OpenShell.

## Delivery sequence

**Now:** keep the Rust core fail-closed; retain the documented Landlock and descriptor experiments; validate the signing workflow itself with mutation tests; preserve exact-head artifacts.

**Next:** implement a supervised Linux launcher with an explicit allowlist of inherited descriptors, close-on-exec by default, default-deny network policy, isolated credentials, and independently observed process/filesystem/network effects. The capability core remains a decision engine, not the enforcement point.

**Then:** add the frozen cross-runtime scenario corpus and run every baseline under the same environment. Add revocation, network and credential cases only when their positive controls and independent oracles are in place.

**Release gate:** do not ship an endpoint-security guarantee until the launcher, policy model, kernel enforcement, observability and provenance layers pass as one integrated system.
