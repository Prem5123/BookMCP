# BookMCP

BookMCP is a local Rust CLI and MCP server that ingests a PDF book into a structured library/index and exposes searchable, citable book knowledge to AI agents.

## Status

This repository is being implemented in phases from `BOOKMCP_IMPLEMENTATION_PROMPT.md`.

Current phase:

- Rust workspace bootstrap.
- Core domain types and validation.
- Documentation and CI foundations.

Planned v0 capabilities:

- Text-based PDF ingestion.
- Clear failure for scanned/image-only PDFs that require OCR.
- SQLite-backed local storage.
- Tantivy BM25 keyword search.
- Read-only stdio MCP server with tools, resources, and prompts.
- CLI workflows for ingesting, listing, searching, and fetching pages/chunks.

Not available yet:

- OCR for scanned PDFs.
- DRM/password bypass.
- Hosted AI or embedding API integrations.
- Semantic/vector search.
- HTTP MCP transport.

## Build

Install a stable Rust toolchain, then run:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Safety And Rights

Only ingest books or documents you have the right to use. BookMCP is designed to keep book content local and offline after dependencies are installed.

## Target CLI Shape

```sh
bookmcp ingest tests/fixtures/tiny.pdf --title "Tiny Test Book" --book-id tiny-test --data-dir ./tmp/bookmcp-test
bookmcp list --data-dir ./tmp/bookmcp-test
bookmcp search "test" --book-id tiny-test --data-dir ./tmp/bookmcp-test
bookmcp page tiny-test 1 --data-dir ./tmp/bookmcp-test
bookmcp serve --data-dir ./tmp/bookmcp-test --transport stdio
```

## Target MCP Configuration

```json
{
  "mcpServers": {
    "bookmcp": {
      "command": "bookmcp",
      "args": ["serve", "--transport", "stdio"]
    }
  }
}
```

## Target Agent Workflow

1. Search the book for a focused query.
2. Fetch the strongest matching chunk.
3. Fetch surrounding context when the chunk alone is insufficient.
4. Answer with citations that identify the book and page range.

## Troubleshooting

- If a PDF has no extractable text, it may be scanned and require OCR, which is planned but not part of the first text-PDF release.
- If MCP stdio responses look corrupted, ensure logs are written to stderr and protocol data is written only to stdout.

