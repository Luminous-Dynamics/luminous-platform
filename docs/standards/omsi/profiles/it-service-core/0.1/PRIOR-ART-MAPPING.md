# IT Service Core 0.1 — prior-art and mapping note

**Status:** incubation note, not a stable profile.

## Why not invent another ticket API?

TM Forum TMF724 already standardizes incident records, create/retrieve/update operations and lifecycle notifications. OMSI should not create a competing full case-management API unless independent examples establish a concrete gap.

- Official TMF724 directory: https://www.tmforum.org/open-digital-architecture/open-apis/incident-management-api-TMF724/v4.0
- Public Swagger repository: https://github.com/tmforum-apis/TMF724_IncidentManagement

The public Swagger mirror's filename and embedded version label are not perfectly aligned with the official directory naming, reinforcing the need to pin exact artifacts and checksums. Integrators must consult the official current directory and license before adopting any artifact.

## Possible OMSI value-add

An OMSI adapter may carry shared cross-provider envelope fields around an existing service-case model:

- Stable source namespace and external record identifier.
- Authenticated tenant binding and tenant-scoped affected-resource references.
- Correlation and causal references.
- Explicit source and target status vocabularies.
- Loss-aware category, priority and impact mapping.
- Event/source timestamps and synchronization version.
- Portable evidence references and custody metadata.

This is a mapping envelope, not a replacement case schema.

## Mapping hazards

- **Tenant:** a payload field is not authorization; derive the permitted tenant from authenticated identity and policy.
- **Priority/impact:** source-specific values may be incomparable; retain original value and mapping provenance rather than flattening silently.
- **Lifecycle:** states such as cleared, resolved, closed and completed do not necessarily mean the same thing. Preserve source state and make translation loss explicit.
- **Identifiers:** retain provider namespace plus external identifier; do not assume vendor IDs are globally unique.
- **Timestamps:** distinguish occurrence, ingestion, update and resolution times.
- **Evidence:** a URL alone does not guarantee durable, authorized or portable access.

## Gate before stabilizing the profile

Collect representative exports/events from at least two independently implemented PSA/ITSM systems, map both directions, enumerate lossy mappings and test round-trip behavior. Until then, keep ManagedResource and ServiceCase schemas incubation-only.
