# Why an ASCII-looking PDF broke only on Windows

BookMCP's Windows CI failed on a tiny PDF that worked on macOS and Linux. The fix changed three lines in `.gitattributes`; it changed no Rust parsing code. Git's checkout conversion had changed the input bytes. [The fix](https://github.com/Prem5123/BookMCP/commit/8fda9ffde7fb03d99f923c02dbd766671a0fae10).

The original fixture is 795 bytes, with 45 LF characters, no CRLF pairs, and no NUL bytes. Much of it is readable ASCII. Without a PDF attribute, a checkout using `core.autocrlf=true` treats this particular fixture as text and produces 840 bytes: one inserted carriage return for each LF.

That transformation is small enough to miss in an editor and large enough to break the file. This PDF's `startxref` value is `580`, the byte offset of its cross-reference table. After conversion, the table starts at byte `611`, while the recorded value is still `580`. Other embedded offsets and lengths are also vulnerable to byte insertion. Readable characters do not make this a format that tolerates arbitrary text normalization.

The resulting BookMCP error, from its PDF parsing path, is:

```text
error: PDF extraction error: couldn't parse input: invalid file trailer
```

The operating system was the clue, but the checkout policy was the cause. The same conversion and failure can be reproduced on macOS or Linux.

## Reproduce with an isolated checkout

Run this from a BookMCP source checkout on macOS/Linux with Git, Python 3, and the project's Rust/build prerequisites installed. It creates a separate temporary repository and libraries. It does not change your repository's Git configuration or existing PDFs.

```sh
cargo build --locked -p bookmcp-cli
bookmcp_pdf_binary="$(pwd)/target/debug/bookmcp"
bookmcp_pdf_demo=$(mktemp -d "${TMPDIR:-/tmp}/bookmcp-pdf-eol.XXXXXX")
git init --quiet "$bookmcp_pdf_demo"
cp tests/fixtures/tiny.pdf "$bookmcp_pdf_demo/sample.pdf"
git -C "$bookmcp_pdf_demo" -c core.attributesFile=/dev/null \
  -c core.autocrlf=false add sample.pdf
git -C "$bookmcp_pdf_demo" -c core.attributesFile=/dev/null \
  -c core.autocrlf=true checkout-index --all \
  --prefix="$bookmcp_pdf_demo/converted/"
```

`checkout-index --prefix` writes a fresh copy, ensuring checkout conversion actually runs. Now apply the repository-level fix inside the scratch repository and export another copy under the same `core.autocrlf=true` setting:

```sh
printf '%s\n' '*.pdf binary' > "$bookmcp_pdf_demo/.gitattributes"
git -C "$bookmcp_pdf_demo" -c core.attributesFile=/dev/null \
  -c core.autocrlf=true checkout-index --all \
  --prefix="$bookmcp_pdf_demo/fixed/"
python3 - "$bookmcp_pdf_demo" <<'PY'
from pathlib import Path
import sys

root = Path(sys.argv[1])
for name in ("sample.pdf", "converted/sample.pdf", "fixed/sample.pdf"):
    data = (root / name).read_bytes()
    offset = int(data.rsplit(b"startxref", 1)[1].split()[0])
    print(name, "bytes:", len(data), "CRLF:", data.count(b"\r\n"),
          "startxref:", offset, "actual xref:", data.find(b"xref"))
PY
```

The original and fixed copies report `795 / 0 / 580 / 580`; the converted copy reports `840 / 45 / 580 / 611`. The simple `find` expression is suitable for this known fixture, not a general PDF validator.

Run these separately: the first intentionally exits with the extraction error; the second ingests one page and one chunk.

```sh
"$bookmcp_pdf_binary" ingest "$bookmcp_pdf_demo/converted/sample.pdf" \
  --book-id eol-test --data-dir "$bookmcp_pdf_demo/broken-library"
"$bookmcp_pdf_binary" ingest "$bookmcp_pdf_demo/fixed/sample.pdf" \
  --book-id eol-test --data-dir "$bookmcp_pdf_demo/fixed-library"
```

## Preserve bytes at the boundary

Git's built-in `binary` macro unsets `text`, `diff`, and `merge`. Unsetting `text` disables line-ending conversion on both check-in and checkout. The committed `*.pdf binary` rule therefore preserves PDF bytes regardless of this checkout preference. [Git attribute documentation](https://git-scm.com/docs/gitattributes#_using_macro_attributes).

This demonstrates one ASCII-looking fixture, not that Git converts every PDF or that every PDF reader rejects this corruption. It also does not repair bytes already stored incorrectly in Git: restore a known-good source when necessary. For cross-platform fixture failures, compare the checked-out bytes and attributes before changing parser behavior.

*Authorship: Written by OpenAI Codex from BookMCP's recorded CI fix and a fresh local reproduction. The counts, byte offsets, error, and successful corrected ingestion were verified against the included fixture.*
