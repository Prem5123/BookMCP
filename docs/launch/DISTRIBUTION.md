# Distribution routes

Rules checked September 7, 2026. Recheck them at submission time, especially account restrictions and community flair. Publication is a separate action from preparing these materials; all routes remain subject to platform or editorial decisions.

| Route | Concrete contribution | Remaining dependency |
| --- | --- | --- |
| GitHub Releases | Tested archives, checksums, accurate notes, and sample | Successful native release workflow and actual publication. |
| Official MCP Registry | Valid `server.json` referencing a published Cargo package or MCPB release bundle | Package publication, namespace authentication, and successful Registry validation. |
| Maintainer's X/Bluesky/LinkedIn/Mastodon | One clear demonstration and repository link | Selected account, access, final factual review. |
| r/mcp | Disclosed maintainer showcase with runnable demo | Selected Reddit account and current rule/flair check. |
| r/rust → TWiR consideration | Substantive Rust project post with implementation detail | Maintainer's personal contribution, account, and editorial selection. |
| Show HN | Runnable software link and personal discussion | Eligible maintainer account, sample readiness, human-authored comments. |
| TWiR CFP | A public, bounded contributor issue | Create the real issue, then link it in the current draft's CFP section. |

## Registry packaging

The official Registry supports local stdio servers distributed as Cargo packages from crates.io. Installation requires the Rust toolchain; the client then invokes the installed binary. Cargo ownership validation needs a visible `mcp-name:` token in the package README matching the Registry name. HTML comments are removed by crates.io rendering and do not work for that check.

MCPB packages can instead carry prebuilt binaries hosted on GitHub or GitLab Releases, with an artifact SHA-256 in `server.json`. This can avoid a Rust toolchain requirement for users. A repository URL or ordinary binary archive alone is not a completed MCPB package. [Official package requirements](https://modelcontextprotocol.io/registry/package-types).

## Developer communities

**r/mcp:** disclose maintenance/affiliation and use `showcase`. The current rules prohibit waitlist promotion, astroturfing, and AI-generated slop. Show a working retrieval flow and respond to specific client problems. [Community rules](https://www.reddit.com/r/mcp/).

**r/rust:** connect the post to Rust implementation choices, use a descriptive title, and include substance that supports technical discussion. The current rules prohibit low-effort material and allow moderators to remove submissions that appear AI-generated. Use the draft's verified facts as an outline and add your own experience. [Community rules](https://www.reddit.com/r/rust/).

**Show HN:** the project must be personally worked on, runnable by readers, and accompanied by the maker's availability to discuss it. Do not solicit upvotes or comments. The maintainer writes discussion personally; HN currently prohibits generated or AI-edited text in comments. Its restriction notice can block newer participants. [Show HN](https://news.ycombinator.com/showhn.html), [HN rules](https://news.ycombinator.com/newsguidelines.html), [restriction notice](https://news.ycombinator.com/showlim).

## TWiR: use the current process

**Do not submit a Project/Tooling Updates PR.** TWiR discontinued those submissions after issue 664 and now considers project/tool links posted to r/rust. A general invitation to submit PRs in the newsletter does not override this category-specific rule. [Current README](https://github.com/rust-lang/this-week-in-rust#projectstooling-updates), [editors' August 10 announcement](https://github.com/rust-lang/this-week-in-rust/issues/8575).

The next draft verified through the GitHub API on September 7 was **issue 668**, [`draft/2026-09-09-this-week-in-rust.md`](https://github.com/rust-lang/this-week-in-rust/blob/main/draft/2026-09-09-this-week-in-rust.md), blob `c0ad4b720f0cb860a8f18d40d2d1de9cfdbc588c`. This identifies the inspected revision, not a guarantee that it remains current.

The **CFP - Projects** section still accepts concrete contribution tasks. Its requirements include an OSI-approved project license, a public issue, difficulty level, and contribution-guideline link. BookMCP's dual MIT/Apache-2.0 license satisfies the license condition. The [prepared issue](contributor-issue.md) describes a focused rotated-page PDF regression task; publish it only if a maintainer will review contributions. This is a contribution request, not an announcement relabeled to bypass the tooling rule. [CFP requirements](https://github.com/rust-lang/this-week-in-rust#call-for-participation-guidelines).

Once that issue exists, add one entry under `### CFP - Projects` in the then-current draft. Replace `ACTUAL_ISSUE_URL` with the published URL; no issue number has been invented:

```markdown
* [BookMCP - Add a generated PDF fixture for a rotated text page](ACTUAL_ISSUE_URL)
```

Suggested PR title: **Add BookMCP rotated-page regression task to CFP**

Suggested PR body, after verifying the public issue:

```text
Adds a bounded BookMCP PDF regression task to CFP - Projects.

The public issue specifies medium difficulty, original/generated fixture requirements, expected text and physical-page citation checks, and contribution guidance. BookMCP is dual MIT/Apache-2.0 licensed.

The link points to the open task, not a release announcement.
```

Use canonical links with the issue's actual title and no tracking parameters. Do not add this to **Calls for Testing**, which the draft reserves for Rust project/RFC implementation testing. A TWiR PR is not ready until the real public issue exists and the current draft has been refreshed.
