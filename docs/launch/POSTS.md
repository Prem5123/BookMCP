# Launch post drafts

These are unsent drafts for the maintainer's selected accounts. Confirm that linked files and release assets are public before posting. The text avoids claims about users, benchmarks, token savings, or completed releases that have not been measured. Tailor each post to the destination; availability of a draft does not establish account access or authorization to send it.

## Short post for X or Bluesky

```text
BookMCP brings the knowledge in your books into system design and implementation. Agents retrieve cited principles, compare tradeoffs, and turn a chosen design into implementation guidance.

https://github.com/Prem5123/BookMCP
```

Attach the [reliable-code demonstration](../demo/README.md): a retrieved safe-retry principle, its page citation, and an explicitly saved lesson. Its captured CLI/MCP output does not show a model designing or implementing a system. The linked README explains the client privacy boundary; do not describe a cloud-connected workflow as entirely offline.

## Longer post for LinkedIn or Mastodon

```text
Give your coding agent the knowledge in your books when it designs a system or plans an implementation.

That's the main use case for BookMCP. Ingest text-based PDF books, then have the agent retrieve relevant passages with page citations as it works.

For example: design a resilient job system. Search the library for retries, idempotency, and consistency; read the relevant principles; compare architecture tradeoffs with citations; then turn the reviewed design into implementation steps and tests. The agent should distinguish what the books support from its own assumptions and flag missing evidence.

Useful lessons can be reviewed and saved through the CLI, with their sources, for agents to retrieve in later sessions. The library supplies reference material on demand; it doesn't train the model or automatically put every book into context.

SQLite and BM25 search run locally. BookMCP needs no embedding service or model API key. Retrieved passages become input to your connected agent, so its provider and privacy settings still apply.

The demo uses an original three-page guide to retrieve a safe-retry principle on page 2 and save a source-linked lesson. Current limits: no OCR, semantic search, or layout reconstruction.

I'd value a concrete example of a book principle informing an architecture choice or implementation plan, along with the source you checked.

https://github.com/Prem5123/BookMCP
```

## r/mcp showcase draft

Title: **BookMCP: give agents book knowledge for system design and implementation**

Flair: **showcase**, if the current composer offers it.

```text
Disclosure: I maintain BookMCP, with implementation assistance from OpenAI Codex.

BookMCP gives coding agents access to the knowledge in your text-based PDF books, with system design as the main use case. For a resilient job system, an agent can look up retry and idempotency principles, compare design tradeoffs against the retrieved evidence, and turn the selected architecture into implementation guidance.

Ingest books through the Rust CLI. The agent starts with a compact library map, searches with BM25, then retrieves a chunk or neighboring context with physical PDF page citations. It should label its own reasoning and say when the library doesn't support a design choice.

The MCP surface is read-only. A separate CLI command saves reviewed lessons with source links; agents can retrieve those notes in later sessions. BookMCP itself makes no hosted AI calls. An attached cloud agent may send the returned passages to its model provider.

The three-page reliable-code demo retrieves a safe-retry principle and saves a cited lesson. A separate one-page fixture checks the basic setup. This is reference retrieval, not model training or automatic whole-book context. No OCR or semantic search is implemented.

I'd like to learn whether a retrieved book principle helps you make an architecture decision or specify an implementation test. Setup and citation failures are useful too: include client/version, the failing step, and a minimal error. Please don't upload private books.

Repository: https://github.com/Prem5123/BookMCP
Sample: https://github.com/Prem5123/BookMCP/blob/main/docs/launch/DEMO.md
```

Review and personalize this with actual maintainer experience before submitting. The community permits disclosed self-promotion and requires the showcase tag, while forbidding waitlists, astroturfing, and AI-generated slop. [Current r/mcp rules](https://www.reddit.com/r/mcp/).

## r/rust project draft

Title: **BookMCP: a Rust CLI bringing cited book knowledge to agent system design**

```text
I maintain BookMCP, a Rust workspace for giving coding agents access to book knowledge during system design and implementation. It extracts text-based PDFs into a local SQLite library and Tantivy BM25 index, then exposes bounded retrieval through a stdio MCP server. OpenAI Codex assisted with implementation.

The intended workflow is to ground a design discussion in specific book passages: look up retry/idempotency principles for a job system, compare the tradeoffs with citations, and turn the reviewed design into implementation steps and tests. Retrieving a passage doesn't establish that a design is correct; the agent and reviewer still need to check its relevance and assumptions.

The boundary is deliberate: ingestion and saved-lesson mutations live in the CLI; MCP callers retrieve by validated IDs and cannot submit arbitrary filesystem paths. Search returns previews and source IDs, and callers can fetch the actual chunk before citing its physical PDF pages. SQLite is the source of truth and the search index can be rebuilt from stored chunks.

The workspace separates domain types, ingestion, storage, indexing, MCP, and CLI code. Tests include a real stdio subprocess workflow and PDF extraction error cases. An original three-page programming guide demonstrates safe-retry evidence and source-linked lessons; a separate one-page fixture checks the basic retrieval path.

I would appreciate specific feedback on text extraction edge cases and client setup. OCR, multi-column layout reconstruction, and semantic search remain future work. No performance comparison is claimed here.

Code and setup: https://github.com/Prem5123/BookMCP
Architecture: https://github.com/Prem5123/BookMCP/blob/main/docs/ARCHITECTURE.md
```

Add a concrete design choice or debugging lesson the maintainer can discuss firsthand. The current rules require Rust relevance and useful context; submissions appearing to contain AI-generated content may be removed at moderator discretion. Treat this as a factual outline for a personal post, not text to mass-publish. [Current r/rust rules](https://www.reddit.com/r/rust/).

## Hacker News: title and preparation only

Suggested title: **Show HN: BookMCP – Give agents book knowledge for system design**

Submission URL: <https://github.com/Prem5123/BookMCP>

The maintainer should personally write the introductory comment and all discussion. No generated comment is supplied: HN's current guidelines say, “Don't post generated text or AI-edited text.” [HN guidelines](https://news.ycombinator.com/newsguidelines.html).

Facts to verify before writing in your own words:

- Your actual reason for making this and one system-design or implementation task you have used it for, if any.
- What users can run today, including source installation or published binaries.
- Why you chose keyword retrieval and a read-only MCP surface.
- Which book principle informed a design choice, if that has actually happened, and how you checked the citation.
- What the captured three-page demo proves, what the one-page setup fixture proves, and what remains limited.
- How Codex assisted implementation, without inventing a development history.

Make the sample easy to run and stay available to discuss it. HN prohibits asking friends for votes or comments. Its current Show HN restriction notice may prevent a newer participant from submitting until they have contributed to the community. [Show HN guidelines](https://news.ycombinator.com/showhn.html), [restriction notice](https://news.ycombinator.com/showlim).
