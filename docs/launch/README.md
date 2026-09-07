# BookMCP launch kit

**Give coding agents the knowledge in your books to inform system design and implementation.** The launch should show an agent retrieving relevant principles, comparing architecture tradeoffs with citations, and turning a selected design into implementation guidance. Trending is an uncertain downstream outcome; no star count, launch hour, or posting schedule guarantees placement.

| File | Use |
| --- | --- |
| [DEMO.md](DEMO.md) | Try a system-design workflow using the three-page guide, then your own books. |
| [demo.sh](demo.sh) | Check installation and retrieval with the one-page fixture in a temporary library. |
| [TESTERS.md](TESTERS.md) | Give early testers a short task and a consistent feedback form. |
| [POSTS.md](POSTS.md) | Review social drafts and prepare a human-authored HN discussion. |
| [DISTRIBUTION.md](DISTRIBUTION.md) | Choose publication routes using community rules checked September 7, 2026. |
| [FIRST_48_HOURS.md](FIRST_48_HOURS.md) | Record launch readiness, real outcomes, and follow-up work. |
| [contributor-issue.md](contributor-issue.md) | A concrete PDF regression task for a public contribution issue. |
| [pdf-line-endings.md](pdf-line-endings.md) | A reproduced Git/PDF fixture failure and its byte-preservation fix. |
| [twir-pdf-article-submission.md](twir-pdf-article-submission.md) | Submitted technical-article PR with explicit AI authorship disclosure. |

The technical article has been [submitted to TWiR](https://github.com/rust-lang/this-week-in-rust/pull/8713) for editorial review. Other drafts are preparation; record actual publication links and results in the launch log as actions happen. Social accounts and tester recipients must come from the maintainer.

Lead with a concrete task: design a resilient job system using relevant book passages about retries, idempotency, and consistency; compare the available choices; then derive implementation steps and tests from the reviewed design. Cite the source of each book-derived principle and identify decisions that need additional evidence. Review useful lessons before saving them through the CLI for later retrieval.

Keep the mechanism precise: BookMCP indexes text-based PDFs locally and gives agents relevant passages on demand, with physical PDF page citations. SQLite and BM25 retrieval run without hosted AI calls. A connected cloud agent may send retrieved passages to its model provider. Saved lessons are retrievable notes; models are not trained on the library, and clients decide when to retrieve context. OCR, semantic search, and an HTTP server are not current features.

The [three-page reliable-code guide](../demo/README.md) demonstrates a principle relevant to system design: safe retries through idempotency. Its captured output verifies retrieval and explicit lesson persistence. The one-page `tiny.pdf` fixture checks installation and retrieval; it contains no system-design guidance. Neither sample is a benchmark or proof that an agent produced a better implementation.

This kit was prepared with OpenAI Codex. Community drafts should reflect the maintainer's own experience; Hacker News comments must be written by the person posting them under its current rules.
