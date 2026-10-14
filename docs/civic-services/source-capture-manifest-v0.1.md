# Reproducible source capture manifest v0.1

**Status:** Proposed tool contract. It does not make legal claims and does not create a source-review receipt.  
**Related:** [Source review protocol](source-review-protocol-v0.1.md), [JSON Schema](../../schemas/source-capture-manifest-v0.1.0.schema.json).

## Purpose

Review needs a repeatable way to answer a narrow question: are the exact bytes retrieved today the same bytes that were captured previously, and did the observed redirect target or media type change? This utility hashes a file already captured by an operator. It performs no network request, does not parse the source as law, and never changes `review_state`.

This reflects the provenance model's separation between an entity (captured bytes/manifest), the activity (capture/hash/compare), and an agent responsible for the capture or review. See [W3C PROV-O](https://www.w3.org/TR/prov-o/) and [NIST Secure Software Development Framework](https://csrc.nist.gov/projects/ssdf) for provenance and traceability context.

## Manifest fields

The manifest records source ID, canonical URL, observed final URL, operator-supplied timezone-aware capture time, content type when observed, exact snapshot byte length and SHA-256, comparison result, and change signals. The comparison states mean:
- `baseline`: no prior manifest was provided;
- `unchanged`: digest, final URL, and content type match the prior manifest;
- `changed`: at least one of those three observations differs.

`changed` describes byte or capture metadata differences only. It does not say whether a law changed. `unchanged` does not prove that the source is authoritative, effective, correctly interpreted, or applicable. The final URL is recorded separately because a redirect-target change is material evidence for a human to inspect.

The tool requires a strictly later timestamp for a new capture relative to its baseline. It rejects malformed IDs, non-HTTPS URLs, embedded URL credentials, empty snapshots, malformed prior manifests, mismatched source identity, and timezone-naive timestamps.

## Example

```sh
python tools/civic-pack-validator/capture_source_snapshot.py \\
  --source-id joburg.business-licences \\
  --canonical-url https://joburg.org.za/work_/InvestInJoburg/Pages/Business-Licences.aspx \\
  --snapshot /secure/capture/business-licences.html \\
  --captured-at 2026-10-14T18:00:00Z \\
  --final-url https://joburg.org.za/work_/InvestInJoburg/Pages/Business-Licences.aspx \\
  --content-type text/html \\
  --output /secure/evidence/business-licences.manifest.json
```

To compare a future capture, pass the earlier manifest with `--previous-manifest`. The script emits a manifest and retains neither the input bytes nor an invented review receipt. Treat snapshot bytes as potentially copyrighted or sensitive: retain them only where legally permitted, under access control, and outside the public repository unless redistribution rights are established.

## Non-goals and release boundary

- No network, browser, DNS, TLS, signature or government identity checks are performed.
- The script cannot prove the timestamp/header values supplied by the operator.
- SHA-256 binds the digest to the bytes only if the bytes remain available for verification.
- A change signal must not automatically set a source to current, stale, disputed or withdrawn; a human reviewer must inspect the evidence and record a new source-review receipt.
- Publication remains blocked until both source evidence and each claim that depends on it have approved review receipts.
