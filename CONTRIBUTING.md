# Contributing

Thanks for helping improve BookMCP. This project is intended to stay local-first, secure by default, and honest about what is implemented.

## Development Setup

Install Rust 1.96 or newer, then run:

```sh
cargo build --workspace
```

Before opening a pull request, run:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Engineering Expectations

- Preserve local-only book content handling.
- Keep MCP read-only by default.
- Do not add OCR, semantic search, or hosted AI features as placeholders.
- Use typed errors in library crates.
- Keep external page numbers 1-based.
- Do not use `unsafe` in project crates.
- Add focused tests for behavior changes.
- Document future work in `docs/ROADMAP.md`, not inline `TODO` or `FIXME` comments.

## Security And Rights

Only test with PDFs that can be legally used for that purpose. Do not add fixtures or examples containing copyrighted book text unless they are clearly licensed for redistribution.

Report security concerns privately when possible. See [SECURITY.md](SECURITY.md).
