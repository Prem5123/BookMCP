# A reproducible BookMCP demo

Use the original one-page `tests/fixtures/tiny.pdf` included in the repository and release archives. This proves the retrieval workflow with shareable content. It is not a retrieval-quality benchmark for full books.

## Run the CLI sample

Install a published native binary or follow the [source installation instructions](../../README.md#quick-start). From a checkout or extracted archive on macOS/Linux:

```sh
sh docs/launch/demo.sh
```

If the executable is not on `PATH`, supply its path:

```sh
BOOKMCP_BIN=./bookmcp sh docs/launch/demo.sh
```

For a development checkout, build first and select the development executable:

```sh
cargo build --locked -p bookmcp-cli
BOOKMCP_BIN=./target/debug/bookmcp sh docs/launch/demo.sh
```

The script creates a fresh temporary library, prints its path, and leaves it available for inspection. It ingests, searches, reads the source, saves one example lesson through the CLI, demonstrates an empty keyword search, and runs `doctor`. It does not connect to a model. Each invocation uses a new library, so no `--force` or cleanup is required to repeat it.

Expected sample observations:

- One book, one physical PDF page, and one chunk.
- Searching `citations` finds `tiny-test-000001` on page 1.
- Reading that chunk shows the original sentence: “Use citations for every answer.”
- The citation identifies **Tiny Test Book by BookMCP Tests, p. 1**.
- The saved lesson is a separate user interpretation with its source citation.
- Searching `cobaltzeppelin` has no hits. A BM25 score is not a confidence percentage.

On Windows, run these commands in PowerShell from the extracted archive. Use `bookmcp` in place of `.\bookmcp.exe` if it is installed on `PATH`:

```powershell
$BookMcpDemoDir = Join-Path ([System.IO.Path]::GetTempPath()) ("bookmcp-demo-" + [guid]::NewGuid().ToString("N"))
.\bookmcp.exe --version
.\bookmcp.exe ingest tests/fixtures/tiny.pdf --book-id tiny-test --title "Tiny Test Book" --data-dir "$BookMcpDemoDir"
.\bookmcp.exe search citations --book-id tiny-test --data-dir "$BookMcpDemoDir"
.\bookmcp.exe chunk tiny-test tiny-test-000001 --data-dir "$BookMcpDemoDir"
.\bookmcp.exe page tiny-test 1 --data-dir "$BookMcpDemoDir"
.\bookmcp.exe doctor --data-dir "$BookMcpDemoDir"
Write-Output $BookMcpDemoDir
```

## Connect a client to the same library

Use the absolute directory printed by the demo in these commands, replacing `/absolute/path/to/demo-library`:

```sh
bookmcp mcp-config codex --data-dir /absolute/path/to/demo-library
bookmcp mcp-config claude --data-dir /absolute/path/to/demo-library
```

Choose the format for your client and merge the generated entry with existing configuration. Restart the client and check its MCP status. See the [client setup](../../README.md#connect-your-agent) for registration commands.

Try this prompt:

> Use BookMCP's library index. Search tiny-test for citations, then fetch the matching chunk. What does this sample advise? Include its title and physical PDF page citation. Separate the source wording from your interpretation.

Expected evidence: the page-1 chunk above. The wording of a model's answer will vary; show its real tool calls and check the cited page.

Then test an absent claim:

> Search tiny-test for cobaltzeppelin. If the tools return no relevant evidence, say that the sample does not support an answer. Do not invent a quotation or citation.

After running the shell demo, test saved lessons:

> List the saved lessons for tiny-test. Read the cited source chunk for “Cite the evidence.” Explain the distinction between the saved note and the book text, and report whether the note is marked stale.

## A useful full-book demonstration

Use a text-layer PDF you are allowed to ingest and show publicly. Select one passage before recording, note its physical PDF page, and ingest it into a separate demo library. Choose a real task that the passage can inform, such as reviewing an error-handling design or explaining a concurrency tradeoff.

> Use BookMCP to find evidence in BOOK_ID about TOPIC. Fetch the strongest source chunk and enough surrounding context to check it. Explain how that advice applies to TASK. Cite each book-derived claim and label your own inference. State what the retrieved evidence does not establish.

Replace the three capitalized terms with your book ID, a distinctive topic from the source, and a concrete task. Show ingestion, the focused query, retrieval, the actual answer, and the matching PDF page. Do not present a scripted answer as a live result or imply that one successful example measures general accuracy.

For a short capture, keep the visual sequence simple: **question → source retrieval → page citation → original page**. State when elapsed time has been shortened. Keep paths and recordings limited to the demo library. BookMCP makes no hosted AI calls; an attached agent's provider settings still apply to retrieved passages.
