# Language and Runtime Policy

**Status:** Proposed platform-wide default  
**Applies to:** Luminous Platform, Luminous Edge, Symthaea, Mycelix integration services, Nixward, Sovereign Ops, Xenia services, and the Public-Service Operations Profile  
**Decision:** Rust is the first-party production implementation language. Do not introduce handwritten JavaScript or TypeScript into production logic.

## 1. Normative rules

1. **Production services and domain logic are Rust-first.** APIs, adapters, policy enforcement, state machines, schema validation, workflow engines, background jobs, source ingestion, evidence verification, and privileged execution boundaries must be implemented in Rust unless an explicit, reviewed exception names the capability that Rust cannot reasonably provide.
2. **No handwritten JavaScript or TypeScript for first-party production application logic.** No Node.js backend, npm-based business logic, TypeScript SPA, JavaScript policy engine, browser-side authority checks, or JS-only connector.
3. **The civic profile's core workflows must work with JavaScript disabled.** Prefer Leptos server-side rendering that delivers meaningful HTML, conventional links, GET search/filter forms and POST form submissions. Do not require hydration or a JavaScript SPA for source provenance, case review, export, correction, or audit access.
4. **WebAssembly is optional, not the default.** Use Rust/WASM only for a bounded client-side capability that materially benefits from local computation or responsiveness. It must not own authority, durable state, or a security decision that is not independently rechecked on the server.
5. **Generated browser glue is distinct from authored application logic.** If a chosen Leptos/WASM build emits JavaScript glue, treat it as a build artifact: pin toolchains, produce reproducible artifacts where possible, inspect the emitted bundle, maintain an SBOM, and document why WASM is necessary. Do not add handwritten glue or JavaScript libraries by convenience.
6. **CSS, HTML templates, SVG, and static assets are allowed.** They are presentation formats, not a second domain-logic language. Avoid client-side scripting embedded in templates.
7. **No JS-driven deployment or authority.** A browser runtime must never determine tenant identity, authorise a mutation, validate a privileged plan as its final gate, sign a trusted receipt without the relevant key boundary, or bypass server-side validation.
8. **Test the no-JavaScript path.** Contract and UI tests must exercise ordinary HTTP requests, form submissions, validation failures, denied access, conflicts, timeout/retry behavior, and refresh/navigation with script execution disabled.
9. **Exceptions are narrow and explicit.** Any proposal to add handwritten JS/TS must identify the unavailable capability, alternatives considered, code surface, dependency/update policy, threat model, ownership, removal path, and tests. Approval is required before the exception is merged.

## 2. Recommended UI deployment profile

For the Public-Service Operations Profile:

- Serve server-rendered HTML from a Rust/Leptos server.
- Keep HTML links, search/filter controls, pagination and correction/review actions usable through regular HTTP requests.
- Use semantic HTML and CSS for presentation, and visible text explaining stale data, source coverage, uncertainty and incomplete retrieval.
- Keep the server authoritative for permissions, tenant scope, transition legality, idempotency, concurrency and audit records.
- Keep enhanced charts optional. The underlying values, source links and export must remain available as HTML tables and download responses.
- Do not introduce a Node.js build/runtime dependency for civic domain logic.

If a later visualisation requires WebAssembly, make it an isolated Rust-built enhancement. The page and its essential data must remain useful if the WASM or generated loader fails to load.

## 3. Why this is a default, not a purity test

Browsers natively expose DOM and many web APIs through JavaScript-facing interfaces. Rust/WASM toolchains such as wasm-bindgen can generate a small JavaScript loader/binding layer even when all authored application code is Rust. That generated glue should not be confused with a handwritten JavaScript application, but it is still an artifact that can affect delivered runtime behavior.

The target for this platform is therefore:
- **zero handwritten JS/TS in production application logic;**
- **zero JS requirement for civic core workflows;**
- **only reviewed, generated glue for an explicitly justified WASM enhancement;**
- **no opaque npm/Node domain or privileged runtime.**

Source references:
- Leptos CSR trade-offs and generated loader: https://book.leptos.dev/getting_started/index.html
- Leptos browser API integration: https://book.leptos.dev/web_sys.html
- wasm-bindgen's Rust/WASM-to-JavaScript interoperability: https://wasm-bindgen.github.io/wasm-bindgen/
