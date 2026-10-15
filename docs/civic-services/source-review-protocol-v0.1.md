# Source review and receipt protocol v0.1

**Status:** Proposed control contract; no source in the current Johannesburg fixture has been reviewed.  
**Scope:** Human source review, reproducible evidence metadata, receipt history, and publication eligibility.

## Why a receipt exists

A mutable `review_state: current` field is not adequate evidence by itself. Each non-`unreviewed` source state must point to the most recent matching `source_review_receipts` entry. The receipt binds a source ID and canonical URL to a content snapshot digest, capture/review times, reviewer identity and role, the decision, reuse-rights metadata, and explicit review scope.

This is a provenance record—not a digital signature, proof that a reviewer is qualified, or proof that a linked authority is legally correct. A human reviewer and the repository's review controls remain responsible for the underlying claim.

## Receipt requirements

A receipt records:
- The exact `source_id` and canonical HTTPS URL reviewed.
- The byte-level SHA-256 for the captured source snapshot, when bytes were available.
- Capture time, review time, and a bounded next-review deadline.
- A stable reviewer identifier and reviewer role. Avoid adding unnecessary personal information.
- The decision: `current`, `changed`, `stale`, `unavailable`, `disputed`, or `withdrawn`.
- Whether authority identity, effective status, content, applicability, and reuse rights were each reviewed.
- The reuse decision, attribution, license URL where known, and rights notes.

A `current` decision is invalid unless all review-scope fields are explicitly true and a snapshot SHA-256 is recorded. The active source record must match its latest receipt for URL, dates, reviewer, digest, and rights fields. A mismatch is an integrity failure, not a reason to trust either side.

The digest must be computed over the exact captured bytes, not normalized text. When legally permitted, retain the captured bytes in an access-controlled evidence store and make the reference resolvable to authorized reviewers. Do not redistribute copyrighted source material just to make it easy to hash. A digest in this prototype is not itself a copy of the evidence and cannot be independently recomputed without access to those bytes; storage and retrieval integration remains future work.

## Append-only history

`source_review_receipts` is an append-only ordered array:
1. Add a new receipt for a new review; do not edit an earlier receipt in place.
2. Point the source record to the newest receipt and make its review metadata agree with that receipt.
3. If a source changes, becomes unavailable, or is disputed, append the new decision and fail closed. Do not rewrite the old receipt to make the history look clean.
4. Pull-request CI replays every top-level jurisdiction-pack JSON changed in the PR's first-parent history. For every pack-changing commit, it compares both ledgers to that commit's first parent and requires each existing ledger to remain an exact prefix. Removing a pack fails closed. When a pack first appears in the PR history, it must start with an empty pair of ledgers, `publication_state: draft`, unreviewed sources with null receipt pointers, and draft claims with null receipt pointers. This prevents pre-populated review history from being introduced as if it predated the PR. Receipt removal, editing, or reordering in a later commit fails.

The per-commit check makes receipt rewriting within the PR history detectable, not cryptographically impossible. It cannot stop an administrator rewriting all repository history or a contributor inventing a receipt. Configure repository branch protection/CODEOWNERS and require independent human review before accepting a receipt that enables publication. The validator does not authenticate `reviewer_id` and cannot prove that the human actually performed the review.

## Claim-specific verification receipts

A source being current does not prove that a particular requirement summary is supported, correctly scoped, understandable, or safe to present as a user task. Every non-draft requirement therefore needs its own entry in `requirement_review_receipts`, and its active `review_receipt_id` must point to the latest receipt for that requirement.

A claim receipt binds:
- The requirement ID and a SHA-256 of the exact claim-bearing fields: title, claim type, summary, applicability predicates and prose, source references, user actions, and completion effect. The hash is SHA-256 over UTF-8 canonical JSON with object keys sorted and no insignificant whitespace; it excludes mutable review metadata.
- A separate `pack_context_sha256` over canonical JSON containing the top-level `pack_id`, `jurisdiction`, and `workflow` objects. The active receipt must match this digest, so changing jurisdiction identity or workflow scope invalidates the old claim receipt until a reviewer appends a new one.
- The review decision, reviewer ID/role, review time, and next-review deadline.
- The source-review receipt IDs actually checked.
- Five explicit checks: source support, jurisdiction, applicability, user-facing language, and completion effect.

A `verified` receipt requires all five checks to be true and the exact set of source receipt IDs currently attached to the requirement's declared source references. If the claim text, applicability, actions, cited sources, or completion effect changes, the claim digest changes and the receipt no longer verifies that content. The receipt also expires under the pack's review interval and freshness rules. A verified claim review must be timestamped at or after every cited source review. For `official_requirement` and `official_guidance` claims, the claim reviewer ID must differ from the reviewer ID on each cited source receipt. This is a separation-of-duties check on declared IDs, not authentication; repository approvals must still enforce reviewer identity and independence.

Claim-review receipts are part of the same append-only ledger policy. CI applies these checks to every changed top-level pack, not only the initial Johannesburg fixture, and protects both `source_review_receipts` and `requirement_review_receipts` against editing, removal, and reordering at every pack-changing commit in the PR history. This does not prove the reviewer did the work or authenticate the reviewer's identity. A repository approval rule and accountable human source/claim reviewers remain mandatory before a pack can be published.

## State and publication semantics

- `unreviewed` plus an empty receipt pointer/history is the valid initial state.
- Any other stored review state must point to the latest receipt for the source, and the state must equal the receipt decision.
- Time-based expiry remains a computed publication blocker even if the last recorded decision was `current`.
- Source content accuracy, legal applicability, and reuse rights are independent questions. `current` does not mean the source can be copied or that every requirement derived from it is correct.
- A user progress checkbox, AI summary, successful CI run, or source hash never proves government submission, licence approval, or legal compliance.

## Interoperability basis

The data separates evidence entities (captured source content and pack records), the review activity, and the responsible agent, consistent with the broad provenance model in [W3C PROV-O](https://www.w3.org/TR/prov-o/). This prototype implements a compact JSON contract rather than requiring an RDF stack.

## Current status

The checked-in Johannesburg fixture intentionally has an empty receipt list, null review pointers, unreviewed source records, draft requirements, and `publication_state: draft`. No source has been marked as reviewed during this change. The protocol is scaffolding for qualified human review, not a substitute for it.
