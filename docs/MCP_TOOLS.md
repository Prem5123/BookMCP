# MCP Tools

BookMCP's MCP server is planned as a read-only stdio server. Tool schemas will use validated core types and safe caps for result sizes and returned text.

## Tools

### `book_list_books`

Input: optional pagination.

Output: books with `book_id`, title, author, page count, chunk count, and ingest time.

### `book_get_metadata`

Input: `book_id`.

Output: metadata, source hash, page count, chapter count, and chunk count.

### `book_get_toc`

Input: `book_id`.

Output: detected chapters/sections, or an explicit fallback message when no TOC is available.

### `book_search`

Input: query, optional `book_id`, optional `top_k`, and mode. Initial mode is `keyword`.

Output: ranked results with citations, snippets, chunk IDs, scores, page ranges, and chapter titles when known.

### `book_get_page`

Input: `book_id`, `page_number`, optional `max_chars`.

Output: extracted page text and citation.

### `book_get_chunk`

Input: `book_id`, `chunk_id`, optional `include_neighbors`.

Output: chunk text, metadata, citation, and optional neighbor IDs/summaries.

### `book_get_context`

Input: `book_id`, `chunk_id`, optional `before`, optional `after`, optional `max_chars`.

Output: surrounding chunks capped by count and character budget.

### `book_find_definitions`

Input: optional `book_id`, term, optional `top_k`.

Output: likely definition passages found through keyword patterns and search.

### `book_find_examples`

Input: optional `book_id`, topic, optional `top_k`.

Output: likely examples from books with citations.

## Resources

- `book://{book_id}/metadata`
- `book://{book_id}/toc`
- `book://{book_id}/page/{page_number}`
- `book://{book_id}/chunk/{chunk_id}`
- `book://{book_id}/chapter/{chapter_id}`

## Prompts

- `ask_book_with_citations`
- `study_chapter`
- `extract_actionable_rules`
- `review_against_book`
- `compare_book_sections`

## Citation Behavior

Every tool output that returns book content must include a citation with book ID, title, page range, and chapter title when available. Large text responses must be capped.

