# First 48 hours

Lead with the main use case: give agents the knowledge in your books to inform system design and implementation. Show a cited principle influencing an architecture choice, then a concrete implementation step or test. Start the clock when the release and demo are publicly usable. Choose a period when the maintainer can answer questions and fix setup failures. This schedule is an operating plan, not a claim about GitHub's ranking algorithm.

## Before the public launch

- [x] Publish only release assets whose required CI and native smoke checks passed. Verify the download links and checksums from a fresh session.
- [ ] Run the one-page setup check using a downloaded archive, then the [system-design exercise](DEMO.md#system-design-example-resilient-job-submission) with the three-page guide from the source checkout. Verify its retry/idempotency citation on page 2.
- [x] Confirm the repository description, topics, README opening, license, client setup, and issue link are accurate.
- [ ] Lead the demo caption with system design and implementation. Distinguish captured CLI/MCP evidence from any model-generated design; the current recording demonstrates retrieval and explicit lesson persistence.
- [ ] Ask the maintainer's selected early testers for the [structured feedback](TESTERS.md); log assisted and unassisted success separately.
- [ ] Resolve a reproducible installation blocker or wrong citation before sending more users through that path.
- [ ] Confirm which accounts will publish and which recipients may be contacted. Recheck destination rules and choose the appropriate flair.
- [x] Record baseline stars, forks, traffic, release-asset downloads, and open bugs with a timestamp. Leave unavailable metrics unknown.

## Hours 0–4

- [x] Publish the main announcement from the selected account with the repository link and real demo. Explain how cited book principles can inform retries, idempotency, or other architecture choices and implementation tests.
- [ ] Submit one appropriate community post once its rules and account eligibility are satisfied. The maintainer writes HN discussion personally.
- [x] Put actual post URLs and timestamps in the log below.
- [ ] Answer questions with concrete setup steps, evidence, and current limitations. Ask for the failing command when someone gets stuck.

## Hours 4–24

- [ ] Reproduce incoming bugs; record OS, installation path, and client. Link duplicate reports to the same issue.
- [ ] Fix the highest-impact onboarding failure, then rerun its reproduction and required checks before releasing a patch.
- [ ] Share a distinct technical explanation in a relevant second community if useful. Do not copy the same promotion into unrelated threads.
- [x] Complete available Registry packaging/listing steps. Record the actual listing URL only after publication succeeds.
- [ ] Record the 24-hour metrics. Ask consenting testers what blocked a useful result, without requesting favorable reviews or votes.

## Hours 24–48

- [ ] Follow up in existing launch threads with fixes that address their reports.
- [ ] Re-test corrected setup steps with someone who encountered the problem, if they agree.
- [ ] Record the 48-hour metrics and choose the next improvement based on repeated user problems.
- [ ] Publish any usage story only with a real result and the person's permission to quote it.
- [ ] For TWiR, use the [current distribution route](DISTRIBUTION.md); do not send a Project/Tooling Updates PR under the discontinued process.

## Measures that help make decisions

An initial aim is **five completed tester reports**, **three unassisted sample-to-citation successes**, and **two independently reported cases where cited book evidence informed a system-design or implementation decision**. These are small-cohort learning targets, not forecasts or Trending thresholds. If no testers have agreed, recruiting them is still outstanding work.

| Measure | What to record | How to interpret it |
| --- | --- | --- |
| First verified citation | Completed testers / testers who attempted the workflow | Primary onboarding signal; retain the raw counts with small samples. |
| Time to first verified citation | Median and individual times, separating install time | Find friction; no speed claim without a defined setup and sample. |
| Unassisted vs assisted use | Counts and the help each tester needed | Successful intervention can hide a documentation gap. |
| Design/implementation value | User-reported decision, verified book principle, resulting implementation step/test, with permission | Shows how evidence was used; does not establish that the overall system or code is correct. |
| Bugs | Reproducible blockers, incorrect citations, other bugs | Prioritize installation failure, data integrity, and source correctness. |
| Reach | Repository views/unique visitors and referrers, when available | Directional visibility; not active-user counts. |
| Downloads | Each release asset's download-count change | Includes repeats and automation; do not equate with installations. |
| Stars/forks | Absolute baseline and change at 24/48 hours | Interest signal; no promised ranking threshold. |

BookMCP has no usage telemetry. Derive successful use from voluntary tester reports, not GitHub downloads. GitHub repository traffic exposes a rolling 14-day view, so save dated snapshots; avoid adding overlapping periods as if they were distinct visitors. [GitHub traffic documentation](https://docs.github.com/en/repositories/viewing-activity-and-data-for-your-repository/viewing-traffic-to-a-repository).

## Launch log

Do not put private recipient names or contact details in the public repository.

| Item | Baseline / planned time | 24 hours | 48 hours |
| --- | --- | --- | --- |
| Release tag and public URL | [v0.1.0](https://github.com/Prem5123/BookMCP/releases/tag/v0.1.0), published 2026-09-07 21:39 UTC | — | — |
| Announcement URLs | [GitHub announcement](https://github.com/Prem5123/BookMCP/discussions/7), 2026-09-07 21:39 UTC; other accounts pending | — | — |
| Registry listing URL | [Active MCP Registry entry](https://registry.modelcontextprotocol.io/v0.1/servers?search=io.github.Prem5123%2Fbookmcp), published 2026-09-07 21:39 UTC | — | — |
| Completed tester reports | Not measured | — | — |
| Unassisted verified citations | Not measured | — | — |
| Evidence-informed design/implementation decisions | Not measured | — | — |
| Repository stars / forks | 0 / 0 at 2026-09-07 21:14 UTC | — | — |
| Views / unique visitors | Baseline recorded locally; includes launch verification traffic | — | — |
| Release-asset downloads | 0 reported per asset at first public check; counters may lag | — | — |
| Reproducible blockers | Not measured | — | — |

The [technical article submission](https://github.com/rust-lang/this-week-in-rust/pull/8713) is open for editorial review. No external tester reports or user success stories have been collected. Social posts and direct invitations remain drafts pending selected accounts and recipients.
