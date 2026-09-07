# Add a generated PDF fixture for a rotated text page

Difficulty: **medium**. Area: PDF ingestion and regression tests.

BookMCP extracts text and attaches physical PDF page citations. The current original fixture is one upright page, and the ingestion tests do not explicitly cover a page with a PDF `/Rotate` entry. A small, reproducible case would establish how that common PDF property affects extraction and citation preservation.

Please add a test that generates an original two-page text PDF with one normal page and one page with `/Rotate 90`. Give each page a distinctive short sentence and keep the fixture independent of third-party books or fonts that cannot be redistributed. The existing `lopdf` test helpers in `crates/bookmcp-ingest/tests/pipeline.rs` are a useful starting point.

Acceptance criteria:

- The fixture is generated in the test or comes with a deterministic generator; no hosted service or external PDF program is required.
- Extraction returns the expected sentence for each physical page, with page numbers 1 and 2 preserved.
- A pipeline regression verifies that a chunk containing the second sentence cites physical page 2.
- If existing behavior fails, include a minimal reproduction and explain the failure. Discuss a focused fix before expanding scope; do not silently weaken expected results to make a test pass.
- Keep the task about text extraction and page provenance. OCR, visual layout reconstruction, and printed-page-label mapping are outside this issue.

Follow [CONTRIBUTING.md](https://github.com/Prem5123/BookMCP/blob/main/CONTRIBUTING.md) and [AGENTS.md](https://github.com/Prem5123/BookMCP/blob/main/AGENTS.md). Run formatting, strict workspace Clippy, and all workspace tests. Contributions use the project's dual MIT/Apache-2.0 license; do not upload private or copyrighted book excerpts.

Before starting, comment with the proposed fixture approach so concurrent work can be coordinated. Include the generated PDF properties, expected text, and validation results in the PR.
