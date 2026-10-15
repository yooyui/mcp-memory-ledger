# Rust Toolchain Policy

状态：`implemented / M0.5 local source contract`

## Current Decision

The repository pins Rust `1.95.0` in `rust-toolchain.toml` and declares
`rust-version = "1.95"` in `Cargo.toml`.

This is the current verified build and support floor for the source-only
technical MVP. It is intentionally conservative: the project does not claim
support for older Rust releases that are not exercised by the repository gate.
The pin also keeps local formatting, Clippy, tests, and GitHub Actions on the
same compiler and component versions.

## Update Rule

A toolchain update must be an explicit repository change. It must update both
files together and pass:

```zsh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
./scripts/test-tier.sh full
./scripts/status-sync-check.sh
```

The GitHub Actions matrix runs these gates on Linux and macOS. Windows remains
outside this claim until the M2 runtime-parity gate produces real Windows
evidence.

## Non-Claims

- This pin is not a packaged runtime or installer.
- It does not prove compatibility with Rust releases older than `1.95`.
- It does not establish Windows runtime parity.
- It does not change the current source-only Local Alpha boundary.
