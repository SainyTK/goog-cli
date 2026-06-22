# Single-Crate Layout

Supersedes ADR-0004.

All Google API surfaces (`drive`, `docs`, `sheets`, etc.) are modules inside a single `goog` crate. There is no Cargo workspace.

## Context

ADR-0004 chose a Cargo workspace with one crate per API surface. Three factors have reversed that decision:

- The project is pre-1.0 and no sub-crate is published independently to crates.io, so there is no consumer that benefits from fine-grained versioning.
- Workspace overhead is felt now: every new API surface requires a new `Cargo.toml`, a new workspace member entry, and cross-crate dependency wiring.
- The compilation-parallelism benefit has not materialized. At this scale the API modules are small, and the linker step dominates -- parallel crate compilation does not meaningfully reduce wall-clock build time.

## Decision

Collapse the workspace into a single `goog` crate. Each Google API surface becomes a module (`goog::drive`, `goog::docs`, etc.). The `goog-auth` shared logic becomes `goog::auth`.

## Trigger for Revisiting

If a new Google API surface (e.g. `goog-sheets`) is added and the team needs to publish it independently or its dependency set diverges significantly from the rest, revisit whether to re-introduce a Cargo workspace at that point.

## Consequences

Adding a new Google API means adding a new module file and a `pub mod` declaration in `lib.rs`. No separate `Cargo.toml` is created. The single `Cargo.toml` grows, but remains manageable at the scale anticipated for this project.
