# BookMCP Agent Instructions

BookMCP is a production Rust workspace for turning legally usable PDF books into structured, searchable, citable local knowledge exposed through a CLI and read-only MCP server.

## Engineering Rules

- Inspect the current repository before changing files.
- Preserve user work. Never revert unrelated changes.
- Prefer explicit errors, typed models, small modules, focused tests, and clear documentation.
- Use `thiserror` for library errors and reserve `anyhow` for binary/application boundaries.
- Use `serde` for JSON models and `schemars` for MCP-facing schemas.
- Keep external page numbers 1-based.
- Add `#![forbid(unsafe_code)]` in project crates.
- Do not use `unsafe` in BookMCP code.
- Do not use `unwrap()` or `expect()` in library or server code. Tests may use them when they clarify failures.
- Do not add placeholder implementations that return empty success.
- Do not add `TODO` or `FIXME` comments in production code; record future work in `docs/ROADMAP.md`.
- Run `cargo fmt`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and `cargo test --workspace --all-features` before claiming completion.

## Product Boundaries

- Support text-based PDFs first.
- Detect scanned or image-only PDFs and fail with a clear typed error such as `OcrRequired` or `NoExtractableText`.
- Do not pretend OCR exists unless it is implemented cleanly.
- Do not bypass PDF encryption, passwords, DRM, or access controls.
- Ingestion happens through the CLI by default.
- MCP is read-only by default and must not expose arbitrary filesystem reads, shell commands, or mutation tools.
- Book content must stay local. Do not send it to hosted AI APIs.
- Keyword/BM25 search is the first search mode. Do not expose semantic search until a real local vector implementation exists behind an optional feature.

## Security Rules

- Ingestion may read only the PDF path the user supplied.
- MCP tools must validate all book IDs, chunk IDs, resource URIs, limits, and context windows.
- Reject path traversal-like IDs, empty IDs, slashes, null bytes, and whitespace.
- Cap query length, `top_k`, `max_chars`, and surrounding context sizes.
- Log tool calls with `tracing`, but do not log full book text by default.

