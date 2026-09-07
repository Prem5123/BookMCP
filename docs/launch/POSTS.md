# Launch post drafts

These are unsent drafts for the maintainer's selected accounts. Confirm that linked files and release assets are public before posting. The text avoids claims about users, benchmarks, token savings, or completed releases that have not been measured. Tailor each post to the destination; availability of a draft does not establish account access or authorization to send it.

## Short post for X or Bluesky

```text
BookMCP gives coding agents searchable passages from text-based PDFs, with page citations. Rust CLI + read-only MCP; local BM25 search, no embedding service.

Try the sample and tell me where setup breaks:
https://github.com/Prem5123/BookMCP
```

Attach a real retrieval demonstration if available. The linked README explains the client privacy boundary. Do not describe a cloud-connected workflow as entirely offline.

## Longer post for LinkedIn or Mastodon

```text
I'm sharing BookMCP: a Rust CLI and read-only MCP server for turning text-based PDFs into searchable passages your coding agent can cite.

Ingest a PDF, let the agent search a focused term, fetch the source chunk, and check its physical PDF page citation. You can also review and save source-linked lessons through the CLI for later sessions.

SQLite and BM25 search run locally. BookMCP needs no embedding service or model API key. Retrieved passages become input to your connected agent, so its provider and privacy settings still apply.

The repository includes a small original PDF and a repeatable demo. Current limits: no OCR, semantic search, or layout reconstruction.

I'd value feedback on installation, client setup, and whether you can verify a useful citation from your own text PDF.

https://github.com/Prem5123/BookMCP
```

## r/mcp showcase draft

Title: **BookMCP: local PDF keyword search with page citations over read-only MCP**

Flair: **showcase**, if the current composer offers it.

```text
Disclosure: I maintain BookMCP, with implementation assistance from OpenAI Codex.

It is a Rust CLI and stdio MCP server for text-based PDF books. Ingestion happens through the CLI. The agent starts with a compact library map, searches with BM25, then retrieves a chunk or neighboring context with physical PDF page citations.

The MCP surface is read-only. A separate CLI command saves reviewed lessons with source links; agents can retrieve those notes in later sessions. BookMCP itself makes no hosted AI calls. An attached cloud agent may send the returned passages to its model provider.

The repository includes an original one-page PDF, setup examples, and a runnable sample. No OCR or semantic search is implemented.

I'm looking for concrete setup and citation failures: client/version, the step that failed, and a minimal error. Please don't upload private books.

Repository: https://github.com/Prem5123/BookMCP
Sample: https://github.com/Prem5123/BookMCP/blob/main/docs/launch/DEMO.md
```

Review and personalize this with actual maintainer experience before submitting. The community permits disclosed self-promotion and requires the showcase tag, while forbidding waitlists, astroturfing, and AI-generated slop. [Current r/mcp rules](https://www.reddit.com/r/mcp/).

## r/rust project draft

Title: **BookMCP: a Rust CLI for local PDF retrieval with citable read-only MCP tools**

```text
I maintain BookMCP, a Rust workspace that extracts text-based PDFs into a local SQLite library and Tantivy BM25 index, then exposes bounded retrieval through a stdio MCP server. OpenAI Codex assisted with implementation.

The boundary is deliberate: ingestion and saved-lesson mutations live in the CLI; MCP callers retrieve by validated IDs and cannot submit arbitrary filesystem paths. Search returns previews and source IDs, and callers can fetch the actual chunk before citing its physical PDF pages. SQLite is the source of truth and the search index can be rebuilt from stored chunks.

The workspace separates domain types, ingestion, storage, indexing, MCP, and CLI code. Tests include a real stdio subprocess workflow and PDF extraction error cases. The sample is an original one-page PDF, so the basic workflow can be tried without finding a book to share.

I would appreciate specific feedback on text extraction edge cases and client setup. OCR, multi-column layout reconstruction, and semantic search remain future work. No performance comparison is claimed here.

Code and setup: https://github.com/Prem5123/BookMCP
Architecture: https://github.com/Prem5123/BookMCP/blob/main/docs/ARCHITECTURE.md
```

Add a concrete design choice or debugging lesson the maintainer can discuss firsthand. The current rules require Rust relevance and useful context; submissions appearing to contain AI-generated content may be removed at moderator discretion. Treat this as a factual outline for a personal post, not text to mass-publish. [Current r/rust rules](https://www.reddit.com/r/rust/).

## Hacker News: title and preparation only

Suggested title: **Show HN: BookMCP – Give coding agents citable passages from local PDFs**

Submission URL: <https://github.com/Prem5123/BookMCP>

The maintainer should personally write the introductory comment and all discussion. No generated comment is supplied: HN's current guidelines say, “Don't post generated text or AI-edited text.” [HN guidelines](https://news.ycombinator.com/newsguidelines.html).

Facts to verify before writing in your own words:

- Your actual reason for making this and one task you have used it for, if any.
- What users can run today, including source installation or published binaries.
- Why you chose keyword retrieval and a read-only MCP surface.
- What the sample proves, and what remains limited.
- How Codex assisted implementation, without inventing a development history.

Make the sample easy to run and stay available to discuss it. HN prohibits asking friends for votes or comments. Its current Show HN restriction notice may prevent a newer participant from submitting until they have contributed to the community. [Show HN guidelines](https://news.ycombinator.com/showhn.html), [restriction notice](https://news.ycombinator.com/showlim).
