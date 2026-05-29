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
- `schema_migrations`

Ingestion writes use transactions for database records. Duplicate ingests replace existing rows for the same book ID cleanly.

## Search Design

Search is Tantivy BM25 keyword search. Chunks are indexed with book ID, chunk ID, text, page range, chapter title, and serialized citation metadata. Queries are parsed safely with a fallback sanitizer for punctuation-heavy input. `top_k` is capped to 50.

Semantic search is not exposed. Future local embeddings can use the existing `EmbeddingProvider` and `VectorIndex` traits.

## MCP Tools, Resources, And Prompts

The MCP server uses `rmcp` 1.7.0 with `ServiceExt`, `transport::stdio`, tool routers, and explicit resource/prompt handlers.

Tools expose book listing, metadata, TOC, search, page/chunk/context retrieval, definition finding, and example finding. Resources use only `book://` URIs over stored records. Prompts guide agents to search, fetch evidence, and answer with citations.

## Why CLI-Only Ingestion

Ingestion reads arbitrary user-selected files and mutates the local library. Keeping ingestion in the CLI prevents the MCP server from becoming a filesystem or mutation surface. MCP remains read-only by default.

## Future OCR And Vector Plan

OCR, layout-aware parsing, tables/figures, local embeddings, hybrid search, reranking, concept graphs, rule packs, and multi-book comparison are tracked in `docs/ROADMAP.md`.
