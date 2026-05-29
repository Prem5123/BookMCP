# BookMCP Implementation Prompt

You are Codex acting as a senior Rust systems engineer. Build a production-quality project called BookMCP.

## Goal

Create a Rust CLI + MCP server that ingests a PDF book and exposes it to AI agents as structured, searchable, citable knowledge. The tool should let an agent search the book, fetch pages/chunks/chapters, inspect metadata/TOC, and use book-specific prompts without dumping the whole PDF into context.

## Important Engineering Standard

Do not write prototype/sloppy code. Build this as if it will be maintained by a professional team. Prefer explicit errors, typed models, tests, clean modules, small functions, and clear documentation over quick hacks.

## Core Product Idea

Input: a PDF book.

Output: a local library/index plus an MCP server exposing the book through tools, resources, and prompts.

## Scope For The First Working Implementation

- Support text-based PDFs end-to-end.
- Detect scanned/image-only PDFs and fail clearly with a helpful `OcrRequired` or `NoExtractableText` error.
- Do not pretend OCR works unless you implement it cleanly.
- Do not bypass DRM, passwords, or encryption.
- Ingestion should happen through the CLI by default.
- The MCP server should be read-only by default. Do not expose arbitrary filesystem reads through MCP.
- The first release must work offline after dependencies are installed. Do not require OpenAI, Anthropic, Gemini, or any hosted API.
- Implement keyword/BM25 search first. Add vector/semantic search only if it can be done cleanly behind an optional feature. Do not expose a fake semantic mode.

## Before Coding

1. Inspect the repository.
2. If the repo is empty, create the project from scratch.
3. If files already exist, preserve user work and adapt the plan.
4. Check current Rust version and toolchain.
5. Check latest usable `rmcp` API/examples before implementing the MCP server. Do not guess an old API.
6. Create a git checkpoint if git is initialized.
7. Create or update `AGENTS.md` with the engineering rules in this prompt so future Codex sessions follow them.

## Repository Name

`bookmcp`

## Suggested Workspace Layout

```text
Cargo.toml
AGENTS.md
README.md
docs/
  ARCHITECTURE.md
  SECURITY.md
  ROADMAP.md
  MCP_TOOLS.md
crates/
  bookmcp-core/
  bookmcp-ingest/
  bookmcp-store/
  bookmcp-index/
  bookmcp-mcp/
  bookmcp-cli/
tests/
  fixtures/
  integration/
```

Use a Rust workspace. Keep crate responsibilities clean.

## 1. `bookmcp-core`

Purpose:
Domain types, configuration, errors, IDs, citations, and shared models.

Must include:

- `BookId`
- `ChunkId`
- `PageNumber`
- `ChapterId`
- `BookMetadata`
- `Page`
- `Chapter`
- `Chunk`
- `Citation`
- `SearchQuery`
- `SearchResult`
- `IngestReport`
- `BookMcpError`

Requirements:

- Validate IDs. Reject path traversal-like IDs such as `../x`, empty strings, slashes, null bytes, and weird whitespace.
- Use `thiserror` for library errors.
- Use `anyhow` only at binary/application boundaries.
- Use `serde` for JSON serialization.
- Use `schemars` where needed for MCP tool schemas.
- Use 1-based page numbers externally.
- Add unit tests for ID validation, page ranges, citation formatting, and error conversion.

## 2. `bookmcp-ingest`

Purpose:
PDF extraction, book normalization, chapter detection, chunking, and ingestion orchestration.

Must include:

- `PdfExtractor` trait.
- A concrete extractor using a Rust PDF text extraction crate. Prefer a maintained crate. If a crate is unreliable, document the reason in `docs/ARCHITECTURE.md`.
- `IngestPipeline`.
- `ChapterDetector`.
- `Chunker`.

Ingest flow:

- Accept a PDF path from CLI.
- Canonicalize the path.
- Verify it exists and is a file.
- Verify extension or content looks like PDF.
- Compute SHA-256 of the PDF.
- Extract page text page-by-page.
- Preserve source page numbers.
- Normalize whitespace without destroying exact quote usefulness.
- Detect empty/scanned PDFs:
  - If most pages have no extractable text, fail with a clear typed error.
- Detect metadata:
  - title from CLI override, PDF metadata, or filename fallback.
  - author from CLI override or PDF metadata if available.
- Detect chapters:
  - Use PDF outline/bookmarks if available.
  - If not available, use conservative heuristics.
  - Never hallucinate chapters. If uncertain, fall back to page-only structure.
- Chunk text:
  - Respect page boundaries and chapter boundaries where possible.
  - Target roughly 2,000-4,000 characters per chunk.
  - Use overlap of roughly 300-600 characters when useful.
  - Preserve `page_start` and `page_end` on every chunk.
  - Every chunk must have a deterministic chunk ID.
  - Every chunk must include citation metadata.

Tests:

- Chunker handles short text, long paragraphs, multi-page text, and empty pages.
- Chunker never produces empty chunks.
- Chunk IDs are deterministic.
- Chapter detector does not fabricate chapters from random body text.
- Ingest detects an empty/scanned-like PDF fixture or simulated extraction result.

## 3. `bookmcp-store`

Purpose:
Persistent local storage for books, pages, chapters, chunks, and ingest metadata.

Suggested storage:

- SQLite via `rusqlite` or another simple Rust SQLite crate.
- Store the original PDF in a managed library directory by default.
- Store normalized text and chunk metadata in SQLite.
- Keep schema migrations simple and explicit.

Data directory:

- Default to a platform-appropriate app data directory using a crate like `directories`.
- Allow override via `--data-dir` and `BOOKMCP_HOME`.

Schema must include:

- `books`
- `pages`
- `chapters`
- `chunks`
- `ingest_runs`

Required operations:

- initialize database
- upsert book by SHA-256 / book_id
- list books
- get book metadata
- get page
- get chapter
- get chunk
- get chunks around a chunk
- delete/rebuild index metadata if needed

Requirements:

- Use transactions for ingestion.
- Handle duplicate ingests gracefully.
- Do not corrupt existing data on failed ingest.
- Add tests using temp directories.

## 4. `bookmcp-index`

Purpose:
Search indexing and retrieval.

Must include:

- Tantivy-based keyword/BM25 search.
- `SearchService`.
- `IndexManager`.
- `SnippetBuilder`.
- Optional trait for future embeddings:
  - `EmbeddingProvider`
  - `VectorIndex`
- Do not expose semantic search unless it is genuinely implemented.

Search behavior:

- Search by `book_id` or across all books.
- Return `top_k` capped to a safe max, for example 50.
- Return:
  - `book_id`
  - `chunk_id`
  - `score`
  - `page_start`
  - `page_end`
  - chapter title if known
  - snippet
  - citation
- Escape or safely parse user query input.
- Do not panic on weird queries.
- If no results, return an empty list with a useful message.

Tests:

- Index and search a small set of chunks.
- Search respects `book_id` filters.
- Search finds exact terms.
- Search handles punctuation, quotes, and empty queries.
- Rebuilding the index is deterministic.

## 5. `bookmcp-mcp`

Purpose:
MCP server implementation using the official Rust MCP SDK, preferably `rmcp`.

Transport:

- Implement stdio transport first.
- Do not implement HTTP until stdio is stable.

MCP server should expose these tools:

### Tool: `book_list_books`

Input:

- none or optional pagination

Output:

- list of books with `book_id`, title, author, page_count, chunk_count, ingested_at

### Tool: `book_get_metadata`

Input:

- `book_id`

Output:

- metadata, source hash, page count, chapter count, chunk count

### Tool: `book_get_toc`

Input:

- `book_id`

Output:

- chapters/sections if detected, otherwise an explicit fallback message

### Tool: `book_search`

Input:

- `query`: string
- `book_id`: optional string
- `top_k`: optional integer
- `mode`: optional enum; initially support only `keyword`

Output:

- ranked search results with citations and snippets

### Tool: `book_get_page`

Input:

- `book_id`
- `page_number`
- `max_chars` optional

Output:

- exact extracted page text plus citation

### Tool: `book_get_chunk`

Input:

- `book_id`
- `chunk_id`
- `include_neighbors` optional bool

Output:

- chunk text, metadata, citation, optional previous/next chunk summaries or IDs

### Tool: `book_get_context`

Input:

- `book_id`
- `chunk_id`
- `before` optional integer
- `after` optional integer
- `max_chars` optional

Output:

- surrounding chunks safely capped

### Tool: `book_find_definitions`

Input:

- `book_id` optional
- `term`
- `top_k` optional

Output:

- likely definition passages found via keyword patterns and search

### Tool: `book_find_examples`

Input:

- `book_id` optional
- `topic`
- `top_k` optional

Output:

- likely examples from the book with citations

MCP resources:

- `book://{book_id}/metadata`
- `book://{book_id}/toc`
- `book://{book_id}/page/{page_number}`
- `book://{book_id}/chunk/{chunk_id}`
- `book://{book_id}/chapter/{chapter_id}`

MCP prompts:

- `ask_book_with_citations`
  Purpose: Ask a question and require the agent to use search/get_chunk before answering.
- `study_chapter`
  Purpose: Turn a chapter into a study guide with citations.
- `extract_actionable_rules`
  Purpose: Extract principles/rules/checklists from a book section.
- `review_against_book`
  Purpose: Review user-provided text/code against principles found in a chosen book.
- `compare_book_sections`
  Purpose: Compare two chapters/sections with citations.

MCP requirements:

- Every tool output must include citations when returning book content.
- Never return huge unbounded text. Use `max_chars` caps.
- Validate every input.
- No arbitrary file reads.
- No arbitrary shell commands.
- No mutation tools by default.
- Log tool calls with `tracing`, but do not log full book text by default.
- Add tests around the service methods even if full MCP protocol integration tests are hard.

## 6. `bookmcp-cli`

Purpose:
User-facing binary.

Binary name:

- `bookmcp`

Commands:

```text
bookmcp ingest <PDF_PATH> [--title <TITLE>] [--author <AUTHOR>] [--book-id <BOOK_ID>] [--data-dir <DIR>] [--force]
bookmcp list [--data-dir <DIR>]
bookmcp search <QUERY> [--book-id <BOOK_ID>] [--top-k <N>] [--data-dir <DIR>]
bookmcp page <BOOK_ID> <PAGE_NUMBER> [--data-dir <DIR>]
bookmcp chunk <BOOK_ID> <CHUNK_ID> [--data-dir <DIR>]
bookmcp serve [--data-dir <DIR>] [--transport stdio]
bookmcp doctor [--data-dir <DIR>]
bookmcp rebuild-index [--data-dir <DIR>] [--book-id <BOOK_ID>]
```

CLI requirements:

- Use `clap`.
- Human-readable output by default.
- Add `--json` where useful.
- Exit codes should be meaningful.
- Errors should be helpful, not raw debug dumps.
- `serve` should write protocol data only to stdout and logs to stderr to avoid corrupting stdio MCP transport.

## Quality Gates

- Add `#![forbid(unsafe_code)]` in our crates unless a dependency forces otherwise. Do not use unsafe in our code.
- No `unwrap()` or `expect()` in library/server code. They are acceptable in tests only when appropriate.
- No broad catch-all swallowing of errors.
- No placeholder implementations returning empty success.
- No TODO/FIXME in production code unless it is in `docs/ROADMAP.md`.
- Use `cargo fmt`.
- Use `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- Use `cargo test --workspace --all-features`.
- Add GitHub Actions CI that runs fmt, clippy, and tests.
- Add meaningful unit and integration tests.
- Public types/functions should have useful doc comments where they form API boundaries.
- Write docs that explain how agents should use the MCP server.

## Security Requirements

- Ingestion may read only the user-provided PDF path.
- MCP tools must not read arbitrary paths.
- Reject malicious book IDs, chunk IDs, and resource URIs.
- Cap query length, `top_k`, `max_chars`, and context window sizes.
- Do not bypass PDF encryption/DRM.
- Do not send book content to external services.
- Make it clear in README that users should only ingest books/documents they have the right to use.
- Add `docs/SECURITY.md` describing the read-only MCP design, data directory boundaries, and known limitations.

## Acceptance Criteria

After implementation, these commands must work:

1. Formatting:

   ```sh
   cargo fmt --check
   ```

2. Linting:

   ```sh
   cargo clippy --workspace --all-targets --all-features -- -D warnings
   ```

3. Tests:

   ```sh
   cargo test --workspace --all-features
   ```

4. CLI help:

   ```sh
   cargo run -p bookmcp-cli -- --help
   ```

5. Ingest a tiny fixture PDF:

   ```sh
   cargo run -p bookmcp-cli -- ingest tests/fixtures/tiny.pdf --title "Tiny Test Book" --book-id tiny-test --data-dir ./tmp/bookmcp-test
   ```

6. List books:

   ```sh
   cargo run -p bookmcp-cli -- list --data-dir ./tmp/bookmcp-test
   ```

7. Search:

   ```sh
   cargo run -p bookmcp-cli -- search "test" --book-id tiny-test --data-dir ./tmp/bookmcp-test
   ```

8. Fetch page:

   ```sh
   cargo run -p bookmcp-cli -- page tiny-test 1 --data-dir ./tmp/bookmcp-test
   ```

9. Start MCP server without crashing:

   ```sh
   cargo run -p bookmcp-cli -- serve --data-dir ./tmp/bookmcp-test --transport stdio
   ```

## Test Fixture

- Create a tiny PDF fixture legally in the repo.
- Prefer generating it via a small test helper/build script if storing binary PDF is awkward.
- The fixture must contain known text so tests can assert search and page retrieval.

## Documentation Requirements

`README.md` must include:

- What BookMCP does.
- What works in v0.
- What does not work yet, especially OCR/scanned PDFs.
- Installation/build instructions.
- CLI examples.
- MCP configuration example for a local stdio server.
- Example agent workflow:
  1. search book
  2. fetch chunk
  3. fetch surrounding context
  4. answer with citations
- Troubleshooting section.

`docs/ARCHITECTURE.md` must include:

- Workspace structure.
- Ingestion pipeline.
- Storage layout.
- Search/index design.
- MCP resources/tools/prompts.
- Why ingestion is CLI-only by default.
- Future vector/OCR plan.

`docs/MCP_TOOLS.md` must include:

- Tool names.
- Input schema.
- Output schema.
- Example calls.
- Citation behavior.

`docs/ROADMAP.md` must include:

- OCR support.
- layout-aware parsing.
- tables/figures extraction.
- local embeddings / hybrid search.
- reranking.
- generated concept graph.
- generated rule packs per book.
- multi-book comparison.

## Implementation Order

1. Bootstrap workspace, `AGENTS.md`, README skeleton, CI.
2. Implement core domain types and errors.
3. Implement storage with SQLite and tests.
4. Implement chunker and ingestion pipeline with extractor trait.
5. Implement PDF extraction for text PDFs.
6. Implement Tantivy indexing/search.
7. Implement CLI commands.
8. Implement MCP server tools/resources/prompts.
9. Add integration tests.
10. Polish docs.
11. Run all quality gates.
12. Provide final summary with:
    - what was implemented
    - key files
    - commands run
    - any limitations
    - next recommended improvements

## Senior Engineer Behavior

- Make small coherent changes.
- Keep modules focused.
- Prefer boring, reliable code.
- When choosing between a clever abstraction and a simple explicit implementation, choose the simple one.
- If a dependency API differs from assumptions, inspect docs/examples and adapt.
- If a requirement is impossible within the current environment, implement the clean boundary, document the limitation, and keep the rest working.
- Never mark the task complete unless fmt, clippy, and tests pass, or clearly report exactly what failed and why.
