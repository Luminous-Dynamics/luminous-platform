# Vendored Agent Control Standard schemas

This directory vendors the machine-readable ACS v0.1.0 package from the ACS repository's v0.1.1 release at immutable commit `1af1f92c1e445cf5ffa662515f83275de6eb603e`.

- `docs/fixtures/acs-v0.1.0-schema-lock.json` lists the exact upstream Git blob IDs and expected byte sizes for the licensed bundle.
- The upstream `LICENSE` and `NOTICE` are included. Upstream v0.1.1 release notes state code and machine-readable schemas are Apache-2.0 licensed.
- GitHub reports the pinned commit as unsigned/unverified. This pin is useful for byte-level reproducibility but is **not** proof of publisher authenticity; review source provenance before accepting a pin update.
- Do not resolve schemas from the network at runtime. Resolve schema references from this local package and reject missing paths.
- Schema validity and a pinned snapshot do not establish ACS-Core conformance; hook coverage, authenticated envelopes, replay rejection, host failure behavior and policy enforcement require separate runtime evidence.
