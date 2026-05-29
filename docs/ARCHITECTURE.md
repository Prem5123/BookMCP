# BookMCP Architecture

BookMCP is organized as a Rust workspace with narrow crate responsibilities:

- `bookmcp-core`: domain types, validation, citations, shared models, and typed errors.
- `bookmcp-ingest`: PDF extraction, normalization, chapter detection, chunking, and ingest orchestration.
- `bookmcp-store`: SQLite-backed local storage.
- `bookmcp-index`: Tantivy BM25 indexing and search.
- `bookmcp-mcp`: read-only MCP server implementation.
- `bookmcp-cli`: user-facing binary.

## Ingestion Pipeline

The CLI accepts one user-provided PDF path, canonicalizes it, verifies it is a file that looks like a PDF, computes its SHA-256, extracts page text, normalizes text, detects metadata and conservative chapters, chunks content with citations, stores it transactionally, and rebuilds the search index.

Scanned or image-only PDFs must fail clearly with typed errors. OCR is future work and must not be faked.

## Storage Layout

The planned store uses a platform-appropriate app data directory, with overrides through `--data-dir` and `BOOKMCP_HOME`. SQLite tables will include `books`, `pages`, `chapters`, `chunks`, and `ingest_runs`. The original PDF will be copied into a managed library directory by default.

## Search Design

The first search implementation is Tantivy-backed keyword/BM25. Search results include chunk IDs, page ranges, chapter titles when available, snippets, scores, and citations. Semantic search is reserved for a future optional feature with a real local embedding provider and vector index.

## MCP Design

The MCP server is read-only by default. The first transport is stdio. MCP tools, resources, and prompts expose indexed book content without arbitrary filesystem reads or mutation tools. Tool outputs that return book content include citations and are capped to prevent huge context dumps.

The current RMCP target is `rmcp` 1.7.0, based on the official Rust SDK documentation and examples using `ServiceExt`, `transport::stdio`, tool routers, resource handlers, and prompt routers.

## Why CLI-Only Ingestion

Ingestion reads local files and writes the managed library. Keeping it in the CLI avoids exposing filesystem access through MCP tools and keeps the server read-only and easier to reason about.

## Future OCR And Vector Plan

OCR, layout-aware parsing, table/figure extraction, local embeddings, hybrid search, reranking, concept graphs, rule packs, and multi-book comparison are tracked in `docs/ROADMAP.md`.

