# BookMCP

BookMCP is a local-first Rust CLI and read-only MCP server for turning legally usable, text-based PDF books into structured, searchable, citable local knowledge.

It ingests a PDF through the CLI, stores normalized book content in a managed SQLite library, builds a Tantivy BM25 keyword index, and exposes read-only MCP tools/resources/prompts for agents that need grounded answers with citations.

## Project Status

BookMCP is an initial v0 implementation. It supports text-based PDFs first and intentionally does not claim OCR, semantic search, hosted AI integration, or access-control bypasses.

Use BookMCP only with books or documents you have the right to process. Book content stays local; the project does not send extracted text to OpenAI, Anthropic, Gemini, or any hosted AI API.

## What Works In v0

- CLI ingestion for text-based PDFs.
- Clear typed failures for PDFs with no extractable text or scanned/image-only content.
- PDF title, author, and top-level bookmark extraction when available.
- Managed local storage for metadata, pages, chapters, chunks, citations, ingest runs, and original PDF copies.
- Local Tantivy BM25 keyword search across stored chunks.
- Page, chunk, and surrounding-context retrieval with citations.
- Read-only MCP stdio server.
- MCP tools, `book://` resources, and prompts for citable agent workflows.
- Offline operation after Rust dependencies are installed.

## Not Implemented

- OCR for scanned/image-only PDFs.
- Layout-aware parsing.
- Tables and figures extraction.
- Semantic/vector search.
- HTTP MCP transport.
- DRM, password, encryption, or access-control bypass.
- Hosted AI API integrations.

## Install From Source

Install Rust 1.96 or newer, then build the CLI:

```sh
cargo build -p bookmcp-cli
```

Run the CLI from the workspace:

```sh
cargo run -p bookmcp-cli -- --help
```

Or install the binary locally from this checkout:

```sh
cargo install --path crates/bookmcp-cli
bookmcp --help
```

By default, BookMCP uses a platform app-data directory. Override it with `--data-dir` or `BOOKMCP_HOME`.

## Quickstart

Use the tiny fixture to exercise the full CLI workflow:

```sh
cargo run -p bookmcp-cli -- ingest tests/fixtures/tiny.pdf \
  --title "Tiny Test Book" \
  --book-id tiny-test \
  --data-dir ./tmp/bookmcp-demo

cargo run -p bookmcp-cli -- list --data-dir ./tmp/bookmcp-demo
cargo run -p bookmcp-cli -- search "test" --book-id tiny-test --data-dir ./tmp/bookmcp-demo
cargo run -p bookmcp-cli -- page tiny-test 1 --data-dir ./tmp/bookmcp-demo
cargo run -p bookmcp-cli -- chunk tiny-test tiny-test-000001 --data-dir ./tmp/bookmcp-demo
cargo run -p bookmcp-cli -- rebuild-index --data-dir ./tmp/bookmcp-demo --book-id tiny-test
cargo run -p bookmcp-cli -- doctor --data-dir ./tmp/bookmcp-demo
```

Serve the MCP server over stdio:

```sh
cargo run -p bookmcp-cli -- serve --data-dir ./tmp/bookmcp-demo --transport stdio
```

## MCP Configuration

If `bookmcp` is installed on your `PATH`, use a stdio MCP configuration like:

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

To point an MCP client at a specific library, include `--data-dir`:

```json
{
  "mcpServers": {
    "bookmcp": {
      "command": "bookmcp",
      "args": ["serve", "--transport", "stdio", "--data-dir", "/path/to/bookmcp-data"]
    }
  }
}
```

During stdio serving, MCP protocol data is written only to stdout. Logs are configured for stderr.

## Agent Workflow

1. Call `book_search` with a focused question or term.
2. Call `book_get_chunk` for the strongest result.
3. Call `book_get_context` when surrounding chunks are needed.
4. Answer using retrieved evidence and include citations.

See [MCP tools](docs/MCP_TOOLS.md) for tool schemas, resources, and prompts.

## Development

Run the required quality gates before opening a PR:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

Useful project docs:

- [Architecture](docs/ARCHITECTURE.md)
- [MCP tools](docs/MCP_TOOLS.md)
- [Security](docs/SECURITY.md)
- [Roadmap](docs/ROADMAP.md)
- [Contributing](CONTRIBUTING.md)

## Troubleshooting

- `NoExtractableText` usually means the PDF is scanned/image-only or otherwise has no text layer. OCR is planned but not implemented.
- Encrypted or password-protected PDFs are not bypassed.
- If search returns no results, run `rebuild-index` for the data directory and retry.
- If MCP stdio clients fail, check that no shell wrapper writes logs to stdout.
- On very full disks, Rust builds can fail while compiling dependencies; remove generated `target/` artifacts and retry.

## License

BookMCP is licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
