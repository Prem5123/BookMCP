# Integration coverage

This is a virtual Cargo workspace, so executable integration tests belong to their owning crates:

- `crates/bookmcp-cli/tests/e2e.rs`: fixture ingestion, duplicate protection, listing, search, page/chunk citations, scoped rebuild, and doctor corruption diagnostics.
- `crates/bookmcp-cli/tests/stdio.rs`: launches the actual `bookmcp serve` executable with piped stdin/stdout, performs MCP initialization, discovers tools/resources, retrieves cited evidence, validates errors and prompt arguments, reads newly saved lessons, and sees another ingested book without restarting. It also verifies empty-library startup and generated client configuration with spaces in paths.
- Store/index tests: transaction rollback, source-copy hash validation, failed file/DB publication, lesson freshness, read-only access, legacy migration, and multi-book index consistency.
- Ingest tests: real generated encrypted/malformed/blank PDFs, source snapshot consistency, bookmark cycles and repeated titles, Unicode chunks, and chapter boundaries.

Run the complete suite from the repository root:

```sh
cargo test --locked --workspace --all-features
```

Tests create temporary libraries and use the original `tests/fixtures/tiny.pdf` or PDFs generated within tests. They do not require model credentials, external PDF tools, or a running agent. Stdio responses have deadlines and child processes are cleaned up on failure. The transport test verifies MCP interoperability at the protocol boundary; a cloud-model answer is not part of the deterministic test suite.
