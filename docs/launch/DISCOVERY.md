# Discovery and directory submissions

Snapshot: September 8, 2026, approximately 07:07 UTC. Topics were read from the GitHub repository API for a selected sample on the daily [overall Trending](https://github.com/trending) and [Rust Trending](https://github.com/trending/rust) pages. Trending changes throughout the day.

## Relevant topics on Trending projects

| Project | Page | Relevant observed topics |
| --- | --- | --- |
| [context-mode](https://github.com/mksglu/context-mode) | Overall | `claude-code`, `codex`, `mcp`, `mcp-server`, `mcp-tools` |
| [ECC](https://github.com/affaan-m/ECC) | Overall | `ai-agents`, `claude-code`, `developer-tools`, `mcp` |
| [deer-flow](https://github.com/bytedance/deer-flow) | Overall | `ai-agents`, `agentic-workflow`, `multi-agent` |
| [MarkItDown](https://github.com/microsoft/markitdown) | Overall | `pdf`, `markdown` |
| [HyperFrames](https://github.com/heygen-com/hyperframes) | Overall | `mcp`, `ai` |
| [jcode](https://github.com/1jehuang/jcode) | Rust | `cli`, `mcp`, `rust`, `coding-agent` |
| [Konnect](https://github.com/mixelpixx/Konnect) | Rust | `mcp-server`, `mcp-tools` |
| [funes](https://github.com/huggingface/funes) | Rust | `ai-agents`, `claude-code`, `codex`, `mcp`, `rust` |

This is a selected comparison, not evidence of how GitHub ranks Trending. [GitHub describes topics](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/classifying-your-repository-with-topics) as classification and discovery metadata, with a limit of 20. Adding a topic does not establish that a project will trend.

## BookMCP metadata applied

The repository keeps its system-design focus and now has 18 topics:

```text
ai-agents, bm25, claude-code, cli, codex, developer-tools,
distributed-systems, information-retrieval, knowledge-base, local-first,
mcp, mcp-server, mcp-tools, model-context-protocol, pdf, rust,
software-architecture, system-design
```

The six additions are `ai-agents`, `cli`, `developer-tools`, `mcp-tools`, `model-context-protocol`, and `information-retrieval`. The first four occur in the sample. The last two describe BookMCP's actual protocol and keyword retrieval; they are not claimed popularity signals.

The About description identifies BookMCP as a local Rust MCP server with PDF search and page citations for system design and implementation. Its website link goes directly to the README quick start. The README places the system-design example, downloads, setup feedback, and a conditional invitation to star near the demo.

## Distribution status

The [v0.1.0 release](https://github.com/Prem5123/BookMCP/releases/tag/v0.1.0), [official MCP Registry listing](https://registry.modelcontextprotocol.io/v0.1/servers?search=io.github.Prem5123%2Fbookmcp), and [GitHub announcement](https://github.com/Prem5123/BookMCP/discussions/7) are public. The [TWiR article PR](https://github.com/rust-lang/this-week-in-rust/pull/8713) remained open at this check.

An existing [Glama listing](https://glama.ai/mcp/servers/Prem5123/BookMCP) was verified on September 8. It indexes the repository and README; this does not mean Glama can host or install the local server. No duplicate submission was sent.

Directory submissions are requests for editorial review; an open issue or PR is not an accepted listing. These submissions were opened on September 8 after checking contribution rules and existing entries:

| Directory | Submission | Status at submission |
| --- | --- | --- |
| TensorBlock/awesome-mcp-servers | [PR #2233](https://github.com/TensorBlock/awesome-mcp-servers/pull/2233), Knowledge Management & Memory entry | Open; awaiting review |
| abordage/awesome-mcp | [PR #109](https://github.com/abordage/awesome-mcp/pull/109), repository entry under AI Memory & RAG / RAG | Open; awaiting review |

Both submissions disclose maintainer affiliation and Codex assistance. BookMCP provides BM25 keyword retrieval for agent context; categorization under RAG does not imply vector or semantic search. Check these submissions before sending another.

Use the [distribution guidance](DISTRIBUTION.md) for account and destination requirements. Social posts require a selected account. Measure subsequent traffic and voluntary tester feedback separately from automated builds, downloads, and maintainer verification.
