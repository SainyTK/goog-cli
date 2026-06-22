# Hand-Written API Types Over Generated Crates

Each API crate defines its own request/response structs with `serde`, making raw HTTP calls via `reqwest`. The auto-generated `google-apis-rs` family of crates (e.g., `google-drive3`, `google-docs1`) is explicitly not used.

## Considered Options

`google-apis-rs` generates typed Rust bindings from Google's API discovery documents. It was rejected for three reasons: (1) the generated code is notoriously verbose and difficult to use ergonomically; (2) generated crates lag behind API changes and are inconsistently maintained; (3) `goog` covers a curated subset of each API, not the full surface -- writing focused types for exactly what is needed is less total code than navigating the generated layer.

## Consequences

Each API crate owns its own type definitions. When Google adds a field we need, we add it to our struct. There is no code generation step in the build. API crates must be tested against real HTTP responses (via `wiremock` fixtures) to catch Google-side changes.
