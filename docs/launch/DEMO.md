# Bring book knowledge into system design and implementation

Use BookMCP to give an agent relevant book principles while it compares system designs and plans implementation. The workflow is **design question → retrieve book evidence → compare tradeoffs with citations → review the design → derive implementation steps and tests**.

## System-design example: resilient job submission

The original [three-page guide](../demo/reliable-code.pdf), *A Small Guide to Reliable Code*, covers typed failures on page 1, retries and idempotency on page 2, and keeping evidence with engineering notes on page 3. The [45-second demonstration and transcript](https://github.com/Prem5123/BookMCP/tree/main/docs/demo) capture real CLI/MCP retrieval and explicit lesson persistence. They do not show a model producing a design or implementation.

From a source checkout or extracted release archive on macOS/Linux, with `bookmcp` installed:

```sh
bookmcp_system_demo=$(mktemp -d "${TMPDIR:-/tmp}/bookmcp-system-design.XXXXXX")
bookmcp ingest docs/demo/reliable-code.pdf --book-id reliable-code --data-dir "$bookmcp_system_demo"
bookmcp search idempotency --book-id reliable-code --data-dir "$bookmcp_system_demo"
bookmcp chunk reliable-code reliable-code-000002 --data-dir "$bookmcp_system_demo"
bookmcp mcp-config codex --data-dir "$bookmcp_system_demo"
```

Use `mcp-config claude` for that configuration format. Merge the entry with your client's existing servers and restart the client. The generated configuration points to the same isolated library. On Windows, supply a new absolute directory with `--data-dir` to these commands; the original PDF is included in both the source checkout and native archive.

Try this design prompt:

> Design a job-submission API whose clients may retry after losing a response. Use BookMCP's library index, then search reliable-code for idempotency and failures. Fetch the source chunks. Compare naive retries with request deduplication, citing the book principles behind the comparison. Propose a request flow and failure behavior. Separate the source's advice from your design assumptions, and identify consistency decisions the guide does not settle.

The verifiable page-2 principle is to recognize repeated requests with an idempotency key and return the stored original result. Check that the agent cites that passage. The short guide does not specify a complete transaction strategy, key-retention policy, or message-delivery guarantee; those choices require further evidence and requirements.

After reviewing the proposed design, try:

> Turn the selected design into an implementation plan: API contract, persistent state, request-handling pseudocode, and tests for duplicate submissions and lost responses. Keep the cited principles attached to the decisions they inform. Label assumptions and identify what must be tested before the design can be trusted.

To carry a useful principle into later work:

> Draft a short lesson about the chosen retry design, with its supporting book and chunk IDs. Distinguish the retrieved principle from our implementation choice. I will review it before saving it through the CLI.

Only save a reviewed lesson with an authorized CLI command. In a later session, have the agent retrieve the lesson and verify its source and stale status before applying it. This is retrieval of an explicit note, not model training or automatic memory.

## Check basic installation and retrieval

The separate original one-page `tests/fixtures/tiny.pdf` is included in the repository and release archives. It checks the retrieval path with shareable content; it contains no system-design guidance and is not a full-book retrieval benchmark.

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

## Connect a client to the tiny sample library

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

## Apply the workflow to your system-design books

Ingest text-layer PDFs you are allowed to use into the same chosen library. Describe the system's workload, latency, durability, and failure constraints before asking for a design. For a public demonstration, use sources you may show publicly and identify relevant physical PDF pages in advance.

> Use BookMCP's library index to find sources relevant to designing our resilient job system. Search focused keywords such as retries, idempotency, transactions, and consistency, then read the relevant chunks and surrounding context. Compare two architecture options against our stated constraints. For each tradeoff, cite the retrieved principle and distinguish your inference. Identify conflicting advice or missing evidence. After we select a design, derive implementation steps and failure tests from those decisions.

BookMCP uses keyword search; the client/model performs the comparison and reasoning. Try distinctive source terms when a query misses a passage. Retrieval does not put entire books into every conversation or establish that a design is correct. Show the actual retrieved evidence, the resulting design choices, and how a reviewer checked them.

For a short capture, show **design question → book principle and citation → architecture decision → implementation test**. Keep captured tool output distinct from any model-generated proposal, and state when elapsed time has been shortened. One example is not a claim of generally better code. BookMCP makes no hosted AI calls; an attached agent's provider settings still apply to retrieved passages.
