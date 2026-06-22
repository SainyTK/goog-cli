# Cargo Workspace with One Crate Per Google API

> **Superseded by ADR-0006** (`0006-single-crate-layout.md`)

The project is a Cargo workspace. Each Google API surface (`goog-auth`, `goog-drive`, `goog-docs`, `goog-sheets`, etc.) is its own library crate. A thin `goog` binary crate composes them.

## Considered Options

A single-crate layout (`src/drive.rs`, `src/docs.rs`, etc.) was considered and rejected. As the number of APIs grows, a single crate becomes a monolith with a sprawling `Cargo.toml`, no parallel compilation between API modules, and no clean boundary enforcement. The workspace approach lets each API crate compile in parallel, own its dependencies, and be reasoned about independently.

Feature flags on a single library crate were also considered but rejected -- they add complexity without the compilation parallelism benefit.

## Consequences

Adding a new Google API means creating a new crate and adding one dependency line to the binary crate. No existing crate is modified. The `goog-auth` crate is a shared dependency of all API crates and must maintain a stable internal interface.
