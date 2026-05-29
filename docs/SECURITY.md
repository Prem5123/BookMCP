# Security

BookMCP is designed as a local-first, read-only MCP surface over books that the user has already ingested through the CLI.

## Data Directory Boundary

The store lives in a platform-appropriate application data directory unless the user passes `--data-dir` or sets `BOOKMCP_HOME`. MCP tools operate only on known book, page, chapter, and chunk records in that managed store.

## Read-Only MCP Design

MCP tools must not read arbitrary paths, run shell commands, ingest new files, mutate stored content, or write outside the managed data directory. Ingestion remains a CLI operation.

## Input Validation

Book IDs, chunk IDs, chapter IDs, page numbers, resource URIs, query lengths, result counts, maximum character counts, and context windows must be validated before use.

IDs reject empty strings, slashes, null bytes, whitespace, `.` and `..`, and path traversal-like forms.

## Known Limitations

- OCR is not implemented yet.
- Layout-aware parsing is not implemented yet.
- PDF extraction quality depends on the PDF and extraction crate.
- Encrypted/password-protected PDFs are not bypassed.
- Users are responsible for ingesting only books and documents they have the right to use.

