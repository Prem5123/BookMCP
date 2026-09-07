# Optional TWiR technical-article submission

Prepared September 7, 2026; not submitted. Publish and review [the article](pdf-line-endings.md) before opening a PR so the canonical link resolves.

TWiR still accepts article suggestions. Its policy asks LLM-written articles to disclose that authorship, which this note does explicitly. Its category guidance says material with little Rust content may be accepted under Miscellaneous, at editorial discretion. This is a Git/PDF fixture debugging note arising from a Rust CLI; it is not a Rust code walkthrough, and acceptance is uncertain. [Article and category guidance](https://github.com/rust-lang/this-week-in-rust#llm-written-articles).

The separate [Project/Tooling Updates restriction](https://github.com/rust-lang/this-week-in-rust#projectstooling-updates) still applies to launch announcements. This note has a reproducible failure, byte-offset measurements, and a repair explanation; it makes no release announcement or promotional call to action.

Target: `rust-lang/this-week-in-rust`, branch `main`, `draft/2026-09-09-this-week-in-rust.md`, **Miscellaneous**, issue 668. Inspected blob: `c0ad4b720f0cb860a8f18d40d2d1de9cfdbc588c`. Refresh the draft before applying [the prepared patch](twir-pdf-article.patch).

PR title:

```text
Add PDF fixture line-ending debugging note
```

PR body:

```text
Suggests “Why an ASCII-looking PDF broke only on Windows” for Miscellaneous.

The note reproduces a Git checkout conversion that changed a Rust CLI's PDF fixture from 795 to 840 bytes, shifted its xref table while retaining the old byte offset, and caused PDF extraction to fail. It includes isolated reproduction commands and the .gitattributes fix.

The article explicitly discloses OpenAI Codex authorship. Its measurements and commands were reproduced locally. It focuses on Git/PDF fixture handling and contains little Rust-specific source, so Miscellaneous is a proposal for editorial judgment; please decline or recategorize if it falls outside the newsletter's scope. It is not a project release announcement.
```

Do not claim editorial acceptance or remove the authorship disclosure to obtain a listing. No GitHub PR or other external publication has been created by this preparation.
