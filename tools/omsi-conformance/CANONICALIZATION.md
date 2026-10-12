# OMSI restricted canonical JSON and digest profile (prototype)

**Status:** implementation note for the 0.1 draft only. Not frozen. Not full RFC 8785/JCS conformance.

## Accepted input subset

The prototype rejects duplicate JSON object names at parse time; negative zero; all floating-point/exponent-form values; integers outside the exact IEEE-754 safe range (-(2^53-1) through +(2^53-1)); invalid Unicode scalar sequences; and object keys that cannot be encoded as UTF-16. Booleans and null remain distinct from integers. Arrays preserve their original order.

Object keys are ordered lexicographically by their UTF-16BE code-unit byte representation. The resulting object is encoded as compact JSON using UTF-8, without ASCII-escaping valid non-ASCII scalar values and without insignificant whitespace. The current Python implementation is the draft reference for these constraints; an independent cross-language vector suite is required before the profile can be frozen.

Because float values are excluded and allowed integers fit the exact interoperable safe range, the accepted-value subset aims to remove the main serialization ambiguities that complicate general JCS implementations. It does not claim that every RFC 8785 requirement or every conforming JCS input is supported. Implementations MUST NOT silently round values into this subset.

## Domain-separated digest

For a value V and registered ASCII domain label D, the prototype computes:

    SHA-256(UTF8("OMSI-DIGEST/0.1") || 0x00 || ASCII(D) || 0x00 || CanonicalJSON(V))

The output is lowercase hexadecimal prefixed with sha256:.

The domain is part of the digest input and MUST be a non-empty ASCII identifier without NUL. This prevents the same JSON bytes from being treated as the same digest subject across semantically different contexts.

The two registered labels used by this seed are:

- org.omsi.action-arguments/0.1 for an operation's action arguments.
- org.omsi.operation-manifest/0.1 for the complete immutable operation manifest.

The grant and receipt subject_manifest_digest values MUST use the operation-manifest label. They must not use the action-arguments label.

## Versioning and review requirements

Any change to accepted JSON types, key ordering, escaping, domain framing, registered labels or output format requires a new profile version and new golden vectors. Before a stable specification, require cross-language vectors for non-BMP key ordering, escaping, boundary integers, duplicate keys, malformed Unicode, array order and cross-domain separation, plus an independent implementation review. A digest binds bytes under this algorithm; it does not authenticate an issuer, prove authorization, or establish truth of observations.
