# Security and data boundaries

BookMCP is a local library tool. Ingest only PDFs and documents you have the right to use.

## MCP boundary

The stdio server exposes retrieval tools and prompts. It has no filesystem-path arguments, ingestion, lesson-save, shell-execution, network, or deletion tools. It accepts only validated book/chunk/chapter IDs and known resource URI shapes. IDs are bounded ASCII identifiers; separators, whitespace, nulls, and traversal identifiers are rejected. Page numbers start at one, including during JSON deserialization. Search queries, results, returned text, context windows, and catalog pages are bounded.

`bookmcp serve` initializes the selected library at CLI startup. Request handlers open the database and index for reading and never create a missing library. A caller selects the data directory at process startup, not through MCP input. The CLI owns ingestion, migrations, indexing, and lesson mutations.

Book text, PDF metadata, prompt arguments, and saved lessons are untrusted reference content. Server instructions tell agents not to treat it as higher-priority instructions. This guidance does not replace a client's own protections against prompt injection. Lessons are interpretations linked to captured citations and source hashes, not verified quotations. Changed source versions are marked stale.

## Local data and privacy

SQLite, normalized text, lessons, the BM25 index, and original PDF copies live in the chosen data directory. BookMCP makes no hosted AI calls, performs no automatic telemetry, and requires no API credentials. Dependencies are downloaded during installation; ingestion and retrieval can then run offline.

An MCP response becomes input to the connected agent. A cloud-backed client may send those passages to its model provider. Its account, retention, and privacy settings apply. Use a local model and client when you need the entire workflow offline.

The library is not encrypted at rest. Protect its directory with operating-system permissions and disk encryption as appropriate. Avoid placing it in a public repository or shared/synchronized folder if that is not intended. Logs use stderr and record tool names and identifiers, not full source text or lesson bodies by default.

## PDF handling

Ingestion reads the supplied source PDF, takes a byte snapshot for parsing/hashing, and stores a managed copy only after checking the copied hash. It does not fetch PDF links, follow embedded actions to read other files, or run external PDF commands. Encrypted/password-protected documents are rejected before text is exposed. There is no OCR, DRM bypass, or password recovery.

PDF parsing runs in-process using Rust dependencies. This is not a hardened sandbox for hostile files. Very large or pathological PDFs can still consume significant CPU and memory; limit resources at the process/container boundary for untrusted bulk ingestion. Malformed page extraction fails explicitly instead of silently returning a partial book.

## Persistence and recovery

SQLite ingestion uses transactions. Managed originals are uniquely staged; ordinary file-publication failures roll back the database, and failed database commits attempt to restore the prior original. A filesystem publication and a SQLite commit are separate durable operations, so sudden power loss between them is not guaranteed crash-atomic. The search index is derived data: a failed index update leaves a clear repair instruction, and `rebuild-index` stages a fresh index from SQLite before replacing a stale or damaged index. A scoped rebuild retains the other books.

Stop ingestion and serving before copying the complete data directory for a backup. Keep the SQLite file, `library/`, and saved lessons together; `index/` can be rebuilt. CLI mutations serialize through a data-directory writer lock; competing writes fail with a retry message. Library API callers must also serialize directory replacement with their own writes. Tantivy may acquire metadata locks during reads, so the index directory must permit its normal lock operations even though MCP cannot modify book content. Keep trusted backups of source PDFs for recovery after disk failure.

## Reporting a vulnerability

Use GitHub's private vulnerability reporting for this repository if enabled. Otherwise contact the maintainer privately via their GitHub profile before posting sensitive exploit details. Include affected version, a minimal reproduction, impact, and a legally shareable fixture where possible. Do not include private book content or credentials in public issues.
