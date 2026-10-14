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
4. Pull-request CI compares the base pack's receipt array with the head pack and requires the exact base array to remain an unchanged prefix. Receipt removal, editing, or reordering fails.

The history check provides tamper-evidence relative to the PR base, not a cryptographic guarantee against administrators rewriting repository history or a contributor inventing a new receipt. Configure repository branch protection/CODEOWNERS and require independent human review before accepting a receipt that enables publication. The validator does not authenticate `reviewer_id` and cannot prove that the human actually performed the review.

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
