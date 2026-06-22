# Single-Crate Layout

Supersedes ADR-0004. All Google API surfaces (`drive`, `docs`, `sheets`, etc.) are modules inside a single `goog` crate. There is no Cargo workspace.

## Considered Options

The prior decision (ADR-0004) was a Cargo workspace with one crate per API surface. Three factors reversed it: the project is pre-1.0 and no sub-crate is published independently to crates.io; workspace overhead is felt now (every new API surface requires a new `Cargo.toml`, a new workspace member entry, and cross-crate dependency wiring); the compilation-parallelism benefit has not materialized -- at this scale the API modules are small and the linker step dominates.

Feature flags on a single library crate were also considered but rejected for the same reasons as ADR-0004 -- they add complexity without the compilation-parallelism benefit.

## Consequences

Each Google API surface becomes a module (`goog::drive`, `goog::docs`, etc.) and `goog-auth` shared logic becomes `goog::auth`. Adding a new surface means adding a module file and a `pub mod` declaration in `lib.rs` -- no separate `Cargo.toml` is created. If a new surface (e.g. `goog-sheets`) later needs independent publishing or its dependency set diverges significantly, the team should revisit whether to re-introduce a workspace at that point.
