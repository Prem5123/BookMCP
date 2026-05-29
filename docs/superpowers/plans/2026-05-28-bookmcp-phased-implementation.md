# BookMCP Phased Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build BookMCP as a production-quality Rust CLI and read-only MCP server for ingesting, indexing, searching, and citing text-based PDF books.

**Architecture:** Implement a Rust workspace in vertical phases. Start with strongly validated shared domain types, then add storage, ingestion, indexing, CLI workflows, MCP service methods, protocol wiring, integration tests, and documentation polish.

**Tech Stack:** Rust 1.96, Cargo workspace, `thiserror`, `serde`, `schemars`, SQLite via `rusqlite`, Tantivy keyword search, `pdf-extract` for text PDF extraction, `rmcp` 1.7.0 for MCP stdio.

---

### Phase 1: Workspace And Core Domain

**Files:**

- Create: `Cargo.toml`
- Create: `.github/workflows/ci.yml`
- Create: `AGENTS.md`
- Create: `README.md`
- Create: `docs/ARCHITECTURE.md`
- Create: `docs/SECURITY.md`
- Create: `docs/MCP_TOOLS.md`
- Create: `docs/ROADMAP.md`
- Create: `crates/bookmcp-core/Cargo.toml`
- Create: `crates/bookmcp-core/src/lib.rs`
- Create: `crates/bookmcp-core/tests/domain.rs`

- [ ] Write failing tests for ID validation, page numbers, citations, and error conversion.
- [ ] Run `cargo test -p bookmcp-core` and verify the tests fail because the domain API is missing.
- [ ] Implement validated IDs, page numbers, citations, shared models, and `BookMcpError`.
- [ ] Run `cargo test -p bookmcp-core` and verify the tests pass.
- [ ] Run `cargo fmt --check`.

### Phase 2: Store

**Files:**

- Modify: `Cargo.toml`
- Create: `crates/bookmcp-store/Cargo.toml`
- Create: `crates/bookmcp-store/src/lib.rs`
- Create: `crates/bookmcp-store/tests/store.rs`

- [ ] Write failing store tests covering database initialization, transactional ingest, duplicate upsert, reads by book/page/chunk/chapter, and failed-ingest rollback.
- [ ] Implement SQLite schema creation and explicit migrations.
- [ ] Implement repository methods over `bookmcp-core` models.
- [ ] Run `cargo test -p bookmcp-store`.

### Phase 3: Ingestion

**Files:**

- Modify: `Cargo.toml`
- Create: `crates/bookmcp-ingest/Cargo.toml`
- Create: `crates/bookmcp-ingest/src/lib.rs`
- Create: `crates/bookmcp-ingest/tests/chunker.rs`
- Create: `crates/bookmcp-ingest/tests/pipeline.rs`
- Create: `tests/fixtures/tiny.pdf`

- [ ] Write failing tests for chunking, deterministic chunk IDs, conservative chapter detection, and scanned-like extraction failure.
- [ ] Implement `PdfExtractor`, `ChapterDetector`, `Chunker`, and `IngestPipeline`.
- [ ] Add text-PDF extraction using `pdf-extract`; document extraction limitations in `docs/ARCHITECTURE.md`.
- [ ] Run `cargo test -p bookmcp-ingest`.

### Phase 4: Keyword Index

**Files:**

- Modify: `Cargo.toml`
- Create: `crates/bookmcp-index/Cargo.toml`
- Create: `crates/bookmcp-index/src/lib.rs`
- Create: `crates/bookmcp-index/tests/search.rs`

- [ ] Write failing tests for indexing chunks, exact-term search, book filters, punctuation/quotes, empty queries, and deterministic rebuilds.
- [ ] Implement `IndexManager`, `SearchService`, `SnippetBuilder`, `EmbeddingProvider`, and `VectorIndex`.
- [ ] Run `cargo test -p bookmcp-index`.

### Phase 5: CLI

**Files:**

- Modify: `Cargo.toml`
- Create: `crates/bookmcp-cli/Cargo.toml`
- Create: `crates/bookmcp-cli/src/main.rs`
- Create: `crates/bookmcp-cli/tests/cli.rs`

- [ ] Write failing CLI tests for help, ingest fixture, list, search, page, chunk, doctor, and rebuild-index.
- [ ] Implement `clap` commands with human-readable output and useful JSON flags.
- [ ] Ensure `serve` writes protocol data only to stdout and logs to stderr.
- [ ] Run `cargo test -p bookmcp-cli`.

### Phase 6: MCP

**Files:**

- Modify: `Cargo.toml`
- Create: `crates/bookmcp-mcp/Cargo.toml`
- Create: `crates/bookmcp-mcp/src/lib.rs`
- Create: `crates/bookmcp-mcp/tests/service.rs`

- [ ] Write failing tests around service methods for all required tools and content caps.
- [ ] Implement read-only service methods over store and index APIs.
- [ ] Wire `rmcp` stdio using the current `rmcp` 1.7.0 API.
- [ ] Add resources and prompts.
- [ ] Run `cargo test -p bookmcp-mcp`.

### Phase 7: End-To-End Gates

**Files:**

- Modify: `README.md`
- Modify: `docs/ARCHITECTURE.md`
- Modify: `docs/MCP_TOOLS.md`
- Create: `tests/integration/bookmcp_e2e.rs`

- [ ] Run `cargo fmt --check`.
- [ ] Run `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- [ ] Run `cargo test --workspace --all-features`.
- [ ] Run `cargo run -p bookmcp-cli -- --help`.
- [ ] Run the fixture ingest/list/search/page/serve acceptance commands from `BOOKMCP_IMPLEMENTATION_PROMPT.md`.
- [ ] Update docs with exact behavior, limitations, and troubleshooting evidence.

