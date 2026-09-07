# Try BookMCP and report what happened

The main task is to use knowledge from a book to inform a system-design choice and its implementation plan. Start by verifying that retrieval works, then check whether the agent applies a relevant principle accurately. Allow about 10–15 minutes after installation for the basic sample; a design exercise takes additional time. Record actual times rather than treating these as performance promises.

## Install and try the sample

1. Choose your platform's archive from [GitHub Releases](https://github.com/Prem5123/BookMCP/releases), when available, and verify its supplied SHA-256 checksum. If no suitable archive is published, use the [source installation instructions](../../README.md#quick-start).
2. Run `bookmcp --version` and record the version, OS, CPU architecture, and installation method.
3. Follow the [one-page setup check](DEMO.md#check-basic-installation-and-retrieval). Keep the fresh temporary library path for the remaining steps.
4. Confirm that `citations` returns `tiny-test-000001`, that its citation names page 1, and that the text matches the PDF. Run `doctor` with the same data directory.
5. If you use an MCP client, connect it to that exact library and try the tiny-sample citation prompt. Record client name/version and whether the tool result and final answer contain the correct source.
6. Try the [system-design example](DEMO.md#system-design-example-resilient-job-submission) using the original three-page reliable-code guide from the source checkout. Check the page-2 retry/idempotency passage, then evaluate the agent's proposed job-submission design and implementation tests. The guide leaves some consistency decisions open; record whether the agent recognizes that gap.
7. Optionally use your own system-design book or other text-based PDF you have permission to ingest. Identify a real architecture question, relevant source passages, and one question the book cannot settle. Try distinctive keywords if a natural-language query misses the relevant passage.

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
Design comparison distinguishes source principles from assumptions: yes / partly / no / not tried
Implementation plan follows the reviewed design: yes / partly / no / not tried
Missing evidence or unresolved consistency choices acknowledged: yes / partly / no / not tried
Own-PDF task: useful / partly useful / not useful / not tried

What task were you trying to complete?
Which book principle informed an architecture or implementation choice?
Which source page did you verify?
What implementation test or requirement followed from that principle?
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
| Design reasoning | Unsupported or misapplied principle | Relevant evidence with unclear assumptions | Cited principle informs a design choice; assumptions are explicit |
| Implementation guidance | Does not follow the design | Partially actionable | Specific implementation steps/tests follow the reviewed design |

Use `not tried` rather than zero for skipped dimensions. Keep individual observations; a total score can hide a wrong citation behind easy installation. A successful end-to-end tester installs, ingests, retrieves, and verifies a citation without maintainer intervention. Separate that count from assisted successes.

Recruit an initial small group only through the maintainer's chosen contacts: ideally one Windows user, one macOS user, one Linux user, two different MCP clients, and developers working through a real system-design or implementation decision. Include someone with a realistically messy text PDF. These are recruitment targets, not an assertion that testers have agreed. Ask for candid results; never make a positive review or star a condition of participation.

## Invitation draft

> I'm testing BookMCP, which gives coding agents access to book knowledge for system design and implementation, with page citations. Would you try the job-system example and tell me whether a retrieved principle helps the agent propose a design and useful implementation tests? Setup failures are useful feedback too. Instructions: https://github.com/Prem5123/BookMCP/blob/main/docs/launch/TESTERS.md. There is no need to share your books or post a positive review.

Send only to specific recipients selected by the maintainer. Record permission separately before quoting anyone or identifying them publicly.
