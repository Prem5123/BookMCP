# Roadmap

The current release delivers text-PDF ingestion, local BM25 retrieval, cited MCP tools/resources/prompts, compact agent context, and explicit CLI lesson capture with read-only MCP retrieval. The focus is reliable evidence retrieval and easy client setup.

Future work, not current capabilities:

- OCR for scanned PDFs, with an explicit local engine and per-page confidence.
- Layout-aware parsing and better handling of multi-column text.
- Structured tables/figures extraction with source-page provenance.
- Local embeddings and hybrid search behind an optional feature; no hosted API requirement.
- Local reranking with reproducible retrieval evaluation.
- Evidence-linked concept graphs and generated rule packs.
- Guided multi-book comparison (current search can already span books).
- HTTP MCP transport after authentication and deployment boundaries are designed.
- A crash-recovery journal spanning managed PDF publication and SQLite commits.
- Bounded-memory bulk ingestion and OS-level isolation for untrusted PDFs.
- Printed-page-label mapping, distinct from physical PDF page numbers.
- Search over saved notes as a separate, clearly attributed knowledge source.

Contributions should include meaningful regression tests and legally shareable fixtures. Keep future capabilities out of exposed schemas until they work end to end.
