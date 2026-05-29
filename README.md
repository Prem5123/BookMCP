# BookMCP

BookMCP is a local Rust CLI and read-only MCP server that turns a text-based PDF book into structured, searchable, citable knowledge for AI agents.

Input: a PDF book you have the right to use.

Output: a managed local library with SQLite records, a Tantivy BM25 keyword index, CLI commands, and MCP tools/resources/prompts.

## What Works In v0

- Ingest text-based PDFs through the CLI.
- Detect PDFs with no extractable text and fail clearly.
- Read PDF title/author metadata and top-level bookmarks when available.
- Store book metadata, pages, chapters, chunks, citations, ingest runs, and a managed copy of the original PDF.
- Search with local Tantivy BM25 keyword search.
- Fetch pages and chunks with citations.
- Serve a read-only MCP stdio server.
- Expose MCP tools, resources, and prompts for agent workflows.
- Work offline after dependencies are installed.

## Not Implemented Yet

- OCR for scanned/image-only PDFs.
- Layout-aware parsing.
- Tables and figures extraction.
- Semantic/vector search.
- HTTP MCP transport.
- DRM, password, or encryption bypass.
- Hosted AI API integrations.

## Build And Test

Install a stable Rust toolchain, then run:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

Build the CLI:

```sh
cargo build -p bookmcp-cli
```

## CLI Examples

```sh
cargo run -p bookmcp-cli -- ingest tests/fixtures/tiny.pdf --title "Tiny Test Book" --book-id tiny-test --data-dir ./tmp/bookmcp-test
cargo run -p bookmcp-cli -- list --data-dir ./tmp/bookmcp-test
cargo run -p bookmcp-cli -- search "test" --book-id tiny-test --data-dir ./tmp/bookmcp-test
cargo run -p bookmcp-cli -- page tiny-test 1 --data-dir ./tmp/bookmcp-test
cargo run -p bookmcp-cli -- chunk tiny-test tiny-test-000001 --data-dir ./tmp/bookmcp-test
cargo run -p bookmcp-cli -- rebuild-index --data-dir ./tmp/bookmcp-test --book-id tiny-test
cargo run -p bookmcp-cli -- serve --data-dir ./tmp/bookmcp-test --transport stdio
```

By default, BookMCP uses a platform app data directory. Override it with `--data-dir` or `BOOKMCP_HOME`.

## MCP Configuration

Example local stdio server config:

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

During stdio serving, MCP protocol data is written only to stdout. Logs are configured for stderr.

## Agent Workflow

1. Call `book_search` with a focused question or term.
2. Call `book_get_chunk` for the strongest result.
3. Call `book_get_context` when surrounding chunks are needed.
4. Answer using only retrieved evidence and include citations.

## Safety And Rights

Only ingest books or documents you have the right to use. BookMCP keeps content local and does not send book text to external services.

## Troubleshooting

- `NoExtractableText` usually means the PDF is scanned/image-only or otherwise has no text layer. OCR is planned but not implemented.
- Encrypted/password-protected PDFs are not bypassed.
- If search returns no results, run `rebuild-index` for the data directory and retry.
- If MCP stdio clients fail, check that no shell wrapper writes logs to stdout.
- On very full disks, Rust builds can fail while compiling dependencies; remove generated `target/` artifacts and retry.
