# BookMCP Architecture

BookMCP is a Rust workspace with small crates and explicit boundaries:

- `bookmcp-core`: domain types, validated IDs, citations, shared models, and typed errors.
- `bookmcp-ingest`: PDF extraction, normalization, chapter detection, chunking, and ingest orchestration.
- `bookmcp-store`: SQLite storage and managed original-PDF library copies.
- `bookmcp-index`: Tantivy BM25 keyword indexing and search.
- `bookmcp-mcp`: read-only MCP service methods, tools, resources, prompts, and stdio serving.
- `bookmcp-cli`: user-facing command line application.

## Ingestion Pipeline

The CLI accepts one user-provided PDF path, canonicalizes it, verifies it is a file that looks like a PDF, computes SHA-256, extracts page text and PDF metadata/bookmarks, normalizes whitespace, detects chapters from outlines or conservative headings, chunks content with citations, stores records transactionally, copies the original PDF into `library/{book_id}.pdf`, and rebuilds the keyword index.

The current text extractor uses `pdf-extract` 0.10.0 through the `PdfExtractor` trait, plus a small `lopdf` inspection pass for title, author, and top-level bookmark data. `pdf-extract` provides page-by-page text extraction but does not solve OCR or layout reconstruction. If extraction returns no useful text, BookMCP fails with typed errors instead of pretending OCR worked.

## Storage Layout

The default data directory comes from the platform app-data location. It can be overridden by `--data-dir` or `BOOKMCP_HOME`.

Inside the data directory:

- `bookmcp.sqlite3`: SQLite database.
- `library/{book_id}.pdf`: managed copy of the source PDF.
- `index/`: Tantivy index directory.

SQLite schema:

- `books`
- `pages`
- `chapters`
- `chunks`
- `ingest_runs`
- `index_metadata`
- `lessons`
- `schema_migrations`

Ingestion writes use transactions for database records. Duplicate ingests replace existing rows for the same book ID cleanly.

## Search Design

Search is Tantivy BM25 keyword search. Chunks are indexed with book ID, chunk ID, text, page range, chapter title, and serialized citation metadata. Queries are parsed safely with a fallback sanitizer for punctuation-heavy input. `top_k` is capped to 50.

Semantic search is not exposed. Future local embeddings can use the existing `EmbeddingProvider` and `VectorIndex` traits.

## MCP Tools, Resources, And Prompts

The MCP server uses `rmcp` 1.7.0 with `ServiceExt`, `transport::stdio`, tool routers, and explicit resource/prompt handlers.

Tools expose book listing, metadata, TOC, search, page/chunk/context retrieval, definition finding, and example finding. Resources use `book://` URIs over stored records and `bookmcp://library` for the compact catalog. Prompts guide agents to search, fetch evidence, and answer with citations.

## Why CLI-Only Ingestion

Ingestion reads arbitrary user-selected files and mutates the local library. Keeping ingestion in the CLI prevents the MCP server from becoming a filesystem or mutation surface. MCP remains read-only by default.

## Future OCR And Vector Plan

OCR, layout-aware parsing, tables/figures, local embeddings, hybrid search, reranking, concept graphs, rule packs, and guided structured multi-book comparison are tracked in `docs/ROADMAP.md`.

## Agent context and lessons

`book_get_library_index` and `bookmcp://library` return a paginated metadata map, resource links, lesson counts, and retrieval instructions. They do not preload book passages. The CLI `context` command emits the same map for a project's agent instructions; it is a snapshot, so clients should refresh it through MCP. MCP clients control when tools and resources enter model context.

Schema version 2 adds `lessons`, independently preserved when a book is replaced. `lesson add` validates the referenced chunk, captures its citation and PDF hash, and persists a bounded user-authored title/body. MCP only reads lessons. On retrieval, a different or absent source hash marks the note stale. Notes are not included in the book's BM25 index, which keeps search results attributable to the book itself.

MCP request handlers use existing read-only store/index handles. The explicit `serve` CLI startup initializes the schema and restores a missing index from all stored chunks before accepting requests. Server instructions identify source content and lessons as untrusted data. Prompt handlers validate and include supplied arguments rather than returning generic instructions detached from the user's question.

## Consistency and recovery

The concrete PDF extractor parses the same byte snapshot used for its source hash. Before saving, the staged managed copy is checked against that hash. The store coordinates ordinary file-publication failures with the database transaction, preserving a prior original for rollback. The filesystem and SQLite are not a shared crash-atomic transaction; a sudden process/system failure between durable commits remains a recovery consideration.

An ingestion or scoped rebuild atomically replaces only that book's documents in Tantivy. A full rebuild stages the complete derived index before replacing the old directory, which also recovers damaged index metadata. CLI mutations hold a data-directory writer lock from snapshot through publication, rejecting competing writers with a retry message. If SQLite saves successfully and indexing fails, the CLI reports that partial outcome and provides `rebuild-index` as recovery. MCP creates fresh search readers so a running server observes subsequent committed ingests and lesson changes.

## Installation and distribution

The single `bookmcp` binary bundles SQLite and needs no external PDF runtime. `mcp-config codex` emits a TOML stanza, and `mcp-config claude` emits an `mcpServers` JSON object using the current executable and selected absolute data-directory paths. They print configuration for merging rather than overwriting client settings. Repository CI checks Rust formatting, linting, and tests; tag-triggered release automation packages native binaries with checksums.
