# Sandcastle Rust Workflow Verification

Host command:

```bash
npm run sandcastle:verify-rust-workflow
```

Sandbox setup hook:

```bash
npm install
```

Sandbox workflow commands:

```bash
rustup --version
cargo fmt
npm run typecheck
npm run test
```

The verifier creates a fresh Docker-backed Sandcastle sandbox from the rebuilt
image and runs the commands above in order. It does not perform a runtime Rust
install in the task shell; `rustup --version` only verifies the baked toolchain
is already present.
