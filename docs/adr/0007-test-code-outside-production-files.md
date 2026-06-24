# Test Code Outside Production Files

Production implementation files contain only runtime code. Unit tests live in sibling `*_tests.rs` modules declared from the parent module, while command-level and black-box behavior tests live in `tests/`.

## Consequences

Test support code means mocks, fixtures, fakes, builders, and test-only helpers. When unit tests need implementation details, those details may be exposed with narrow `pub(super)` or `pub(crate)` visibility rather than adding test blocks to production files.
