# MCP Tools

BookMCP serves a read-only stdio MCP server. All tools validate IDs and cap returned text. Tool outputs are JSON content.

## `book_list_books`

Input schema:

```json
{ "offset": 0, "limit": 50 }
```

Output schema:

```json
{
  "books": [
    {
      "book_id": "tiny-test",
      "title": "Tiny Test Book",
      "author": "BookMCP Tests",
      "page_count": 1,
      "chunk_count": 1,
      "ingested_at": "2026-05-29T00:00:00Z"
    }
  ]
}
```

## `book_get_metadata`

Input schema:

```json
{ "book_id": "tiny-test" }
```

Output schema:

```json
{
  "metadata": {
    "book_id": "tiny-test",
    "title": "Tiny Test Book",
    "author": "BookMCP Tests",
    "source_sha256": "...",
    "page_count": 1,
    "chapter_count": 0,
    "chunk_count": 1,
    "ingested_at": "2026-05-29T00:00:00Z"
  },
  "source_sha256": "...",
  "page_count": 1,
  "chapter_count": 0,
  "chunk_count": 1
}
```

## `book_get_toc`

Input schema:

```json
{ "book_id": "tiny-test" }
```

Output schema:

```json
{
  "book_id": "tiny-test",
  "chapters": [
    {
      "chapter_id": "chapter-1",
      "book_id": "tiny-test",
      "title": "Chapter 1",
      "page_start": 1,
      "page_end": 12
    }
  ],
  "message": null
}
```

If no chapters are known, `chapters` is empty and `message` explains that page-only structure is available.

## `book_search`

Input schema:

```json
{ "query": "test", "book_id": "tiny-test", "top_k": 10, "mode": "keyword" }
```

Only `keyword` mode is supported. `top_k` is capped to 50.

Output schema:

```json
{
  "results": [
    {
      "book_id": "tiny-test",
      "chunk_id": "tiny-test-000001",
      "score": 0.5,
      "page_start": 1,
      "page_end": 1,
      "chapter_title": null,
      "snippet": "...",
      "citation": {
        "book_id": "tiny-test",
        "title": "Tiny Test Book",
        "author": "BookMCP Tests",
        "page_start": 1,
        "page_end": 1,
        "chapter_title": null
      }
    }
  ],
  "message": null
}
```

## `book_get_page`

Input schema:

```json
{ "book_id": "tiny-test", "page_number": 1, "max_chars": 8000 }
```

Output schema:

```json
{
  "book_id": "tiny-test",
  "page_number": 1,
  "text": "This tiny PDF contains test concepts.",
  "citation": {
    "book_id": "tiny-test",
    "title": "Tiny Test Book",
    "author": "BookMCP Tests",
    "page_start": 1,
    "page_end": 1,
    "chapter_title": null
  },
  "truncated": false
}
```

## `book_get_chunk`

Input schema:

```json
{ "book_id": "tiny-test", "chunk_id": "tiny-test-000001", "include_neighbors": true }
```

Output schema:

```json
{
  "chunk": {
    "chunk_id": "tiny-test-000001",
    "book_id": "tiny-test",
    "chapter_id": null,
    "chapter_title": null,
    "page_start": 1,
    "page_end": 1,
    "text": "...",
    "citation": { "...": "..." }
  },
  "previous_chunk_id": null,
  "next_chunk_id": null
}
```

## `book_get_context`

Input schema:

```json
{ "book_id": "tiny-test", "chunk_id": "tiny-test-000001", "before": 1, "after": 1, "max_chars": 12000 }
```

`before` and `after` are capped to 5. `max_chars` is capped to 20,000.

Output schema:

```json
{
  "chunks": [
    {
      "chunk_id": "tiny-test-000001",
      "page_start": 1,
      "page_end": 1,
      "text": "...",
      "citation": { "...": "..." }
    }
  ],
  "total_chars": 1200,
  "truncated": false
}
```

## `book_find_definitions`

Input schema:

```json
{ "book_id": "tiny-test", "term": "ownership", "top_k": 5 }
```

Output schema is the same as `book_search`. Results matching likely definition wording are ranked first.

## `book_find_examples`

Input schema:

```json
{ "book_id": "tiny-test", "topic": "ownership", "top_k": 5 }
```

Output schema is the same as `book_search`. Results matching likely example wording are ranked first.

## Resources

- `book://{book_id}/metadata`
- `book://{book_id}/toc`
- `book://{book_id}/page/{page_number}`
- `book://{book_id}/chunk/{chunk_id}`
- `book://{book_id}/chapter/{chapter_id}`

The server rejects non-`book://` URIs and invalid IDs.

## Prompts

- `ask_book_with_citations`
- `study_chapter`
- `extract_actionable_rules`
- `review_against_book`
- `compare_book_sections`

Prompts instruct agents to use BookMCP tools/resources before answering and to cite retrieved evidence.

## Citation Behavior

Every tool that returns book content includes citation data or rendered citation text. Agents should cite page or page-range evidence in final answers.
