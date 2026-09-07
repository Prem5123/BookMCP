# Try BookMCP and report what happened

The first useful result is a passage you can check against its PDF page. Allow about 10–15 minutes after installation; record the actual time rather than treating this as a performance promise.

## Install and try the sample

1. Choose your platform's archive from [GitHub Releases](https://github.com/Prem5123/BookMCP/releases), when available, and verify its supplied SHA-256 checksum. If no suitable archive is published, use the [source installation instructions](../../README.md#quick-start).
2. Run `bookmcp --version` and record the version, OS, CPU architecture, and installation method.
3. Follow the [sample demo](DEMO.md). Keep the fresh temporary library path for the remaining steps.
4. Confirm that `citations` returns `tiny-test-000001`, that its citation names page 1, and that the text matches the PDF. Run `doctor` with the same data directory.
5. If you use an MCP client, connect it to that exact library and try the first agent prompt in the demo. Record client name/version and whether the tool result and final answer contain the correct source.
6. Optionally ingest a text-based PDF you have permission to use. Pick one question with a known answer and one question the PDF cannot answer. Try distinctive keywords if a natural-language query misses the relevant passage.

Scanned PDFs need OCR, which BookMCP does not implement. Citation numbers refer to physical PDF pages starting at 1; printed page labels may differ. A cloud-backed agent may send retrieved passages to its model provider even though BookMCP performs extraction and search locally.

## Feedback form

Copy the form into a [GitHub issue](https://github.com/Prem5123/BookMCP/issues/new) or the maintainer's agreed feedback channel. Share a minimal original/publicly shareable fixture when needed, not your private library or a book you cannot redistribute.

```text
BookMCP version:
OS / CPU architecture:
Install method:
MCP client/version (or CLI only):
Time to installed executable:
Time from executable to first verified citation:

Installation: passed / blocked / not tried
Sample ingestion: passed / blocked / not tried
Sample keyword search: passed / blocked / not tried
Citation matched physical PDF page: yes / no / not checked
MCP connection: passed / blocked / not tried
Agent answer supported by retrieved source: yes / partly / no / not tried
Own-PDF task: useful / partly useful / not useful / not tried

What task were you trying to complete?
Exact step or command where you got stuck:
Expected result:
Actual result and minimal error text:
Did someone need to help you? What help?
Would you use this again for a specific task? Which one?
May the maintainer quote this feedback publicly? yes / no
```

## Maintainer scoring

| Dimension | 0 | 1 | 2 |
| --- | --- | --- | --- |
| Install | Blocked | Works with help | Works without help |
| Sample retrieval | No usable result | Needs correction/help | Correct passage without help |
| Citation | Missing or wrong | Present but not verified | Checked against source page |
| MCP setup | Blocked | Works after configuration help | Works from the documented steps |
| Real task | No useful evidence | Some relevant evidence | Evidence helps a concrete task |

Use `not tried` rather than zero for skipped dimensions. Keep individual observations; a total score can hide a wrong citation behind easy installation. A successful end-to-end tester installs, ingests, retrieves, and verifies a citation without maintainer intervention. Separate that count from assisted successes.

Recruit an initial small group only through the maintainer's chosen contacts: ideally one Windows user, one macOS user, one Linux user, two different MCP clients, and someone with a realistically messy text PDF. These are recruitment targets, not an assertion that testers have agreed. Ask for candid results; never make a positive review or star a condition of participation.

## Invitation draft

> I'm testing BookMCP, an open-source Rust tool that lets coding agents retrieve passages from text-based PDFs with page citations. Would you try the included sample and tell me where installation or retrieval breaks? The instructions are here: https://github.com/Prem5123/BookMCP/blob/main/docs/launch/TESTERS.md. There is no need to share your books or post a positive review; a failed setup is useful feedback too.

Send only to specific recipients selected by the maintainer. Record permission separately before quoting anyone or identifying them publicly.
