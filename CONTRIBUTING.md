# Contributing to BookMCP

Useful contributions include reproducible PDF extraction reports, clearer setup instructions, agent integration examples, and focused fixes. Open an issue describing the user problem before starting a large feature. Small fixes can go straight to a pull request.

## Develop locally

Install Rust 1.96 or newer and a C/C++ build toolchain (SQLite is bundled). Clone the repository, then run:

```sh
cargo build --locked --workspace
cargo run --locked -p bookmcp-cli -- ingest tests/fixtures/tiny.pdf --book-id tiny-test --data-dir ./tmp/dev
cargo run --locked -p bookmcp-cli -- search test --data-dir ./tmp/dev
```

Read [AGENTS.md](AGENTS.md) for engineering rules and [the architecture](docs/ARCHITECTURE.md) for crate responsibilities. Keep books local, retain physical 1-based page citations, and preserve the read-only default MCP surface. Do not introduce OCR or semantic-search claims without an implemented local backend.

Before submitting, run all required checks:

```sh
cargo fmt --all
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace --all-features
```

Tests use generated data and the tiny original PDF in `tests/fixtures`. For a PDF bug report, include the BookMCP version, OS, exact command, expected result, and error output. Attach a minimal document you have permission to share, or describe how to generate one. Avoid posting private library databases, copyrighted books, credentials, or complete MCP logs containing book text.

Pull requests should explain the user-visible change and validation. Include regression tests for behavior fixes. Contributions are accepted under the project's dual MIT/Apache-2.0 license.

## Release process

1. Update all package versions together and the lockfile. Complete the quality gates and review documentation.
2. Use the **Release binaries** workflow's manual dispatch to test native packaging. It builds Linux x86-64/ARM64, macOS Intel/Apple Silicon, and Windows x86-64 archives as workflow artifacts.
3. After review, push a matching `vX.Y.Z` tag. The workflow reruns format, lint, and tests, checks the tag against the CLI package version, and creates a **draft** GitHub release containing binaries, docs, licenses, and SHA-256 checksums.
4. Download and smoke-test the artifacts, review the release notes, then publish the draft. macOS binaries are not notarized; Linux binaries require glibc 2.35 or newer. Source installation remains available on other supported Rust targets.

The release workflow must succeed on each native runner before claiming support for that release. A local macOS test run alone does not verify Windows or Linux behavior.
