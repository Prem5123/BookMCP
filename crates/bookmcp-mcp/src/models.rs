use bookmcp_core::{
    BookId, BookMetadata, Chapter, Chunk, ChunkId, Citation, Lesson, PageNumber, SearchResult,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Input for `book_list_books`.
#[derive(Clone, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BookListBooksInput {
    /// Optional offset.
    pub offset: Option<usize>,
    /// Optional limit, capped server-side.
    pub limit: Option<usize>,
}

/// Pagination input accepted by the compact library index.
pub type BookGetLibraryIndexInput = BookListBooksInput;

/// Book summary returned by `book_list_books`.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct BookSummary {
    pub book_id: BookId,
    pub title: String,
    pub author: Option<String>,
    pub page_count: u32,
    pub chunk_count: u32,
    pub ingested_at: String,
    pub title_truncated: bool,
    pub author_truncated: bool,
}

impl From<BookMetadata> for BookSummary {
    fn from(metadata: BookMetadata) -> Self {
        let (title, title_truncated) = super::cap_text(&metadata.title, 256);
        let (author, author_truncated) = match metadata.author {
            Some(author) => {
                let (author, truncated) = super::cap_text(&author, 128);
                (Some(author), truncated)
            }
            None => (None, false),
        };
        Self {
            book_id: metadata.book_id,
            title,
            author,
            page_count: metadata.page_count,
            chunk_count: metadata.chunk_count,
            ingested_at: metadata.ingested_at,
            title_truncated,
            author_truncated,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct BookListBooksOutput {
    pub books: Vec<BookSummary>,
    pub total: usize,
    pub next_offset: Option<usize>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct LibraryIndexBook {
    pub book_id: BookId,
    pub title: String,
    pub author: Option<String>,
    pub page_count: u32,
    pub chunk_count: u32,
    pub metadata_uri: String,
    pub toc_uri: String,
    pub lessons_uri: String,
    pub title_truncated: bool,
    pub author_truncated: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct BookLibraryIndexOutput {
    pub books: Vec<LibraryIndexBook>,
    pub total_books: usize,
    pub total_lessons: usize,
    pub next_offset: Option<usize>,
    pub instructions: String,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BookListLessonsInput {
    pub book_id: Option<BookId>,
    pub offset: Option<usize>,
    pub limit: Option<usize>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct BookListLessonsOutput {
    pub lessons: Vec<Lesson>,
    pub total: usize,
    pub next_offset: Option<usize>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BookGetMetadataInput {
    pub book_id: BookId,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct BookGetMetadataOutput {
    pub metadata: BookMetadata,
    pub source_sha256: String,
    pub page_count: u32,
    pub chapter_count: u32,
    pub chunk_count: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BookGetTocInput {
    pub book_id: BookId,
    pub offset: Option<usize>,
    pub limit: Option<usize>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct BookGetTocOutput {
    pub book_id: BookId,
    pub chapters: Vec<Chapter>,
    pub total: usize,
    pub next_offset: Option<usize>,
    pub titles_truncated: bool,
    pub message: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchModeInput {
    Keyword,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BookSearchInput {
    pub query: String,
    pub book_id: Option<BookId>,
    pub top_k: Option<usize>,
    pub mode: Option<SearchModeInput>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
pub struct BookSearchOutput {
    pub results: Vec<SearchResult>,
    pub message: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BookGetPageInput {
    pub book_id: BookId,
    pub page_number: PageNumber,
    pub max_chars: Option<usize>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct BookGetPageOutput {
    pub book_id: BookId,
    pub page_number: PageNumber,
    pub text: String,
    pub citation: Citation,
    pub truncated: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BookGetChunkInput {
    pub book_id: BookId,
    pub chunk_id: ChunkId,
    pub include_neighbors: Option<bool>,
    /// Text budget; defaults to 8,000 and is capped at 20,000 Unicode characters.
    pub max_chars: Option<usize>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct BookGetChunkOutput {
    pub chunk: Chunk,
    pub truncated: bool,
    pub previous_chunk_id: Option<ChunkId>,
    pub next_chunk_id: Option<ChunkId>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BookGetContextInput {
    pub book_id: BookId,
    pub chunk_id: ChunkId,
    pub before: Option<usize>,
    pub after: Option<usize>,
    pub max_chars: Option<usize>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct ContextChunk {
    pub chunk_id: ChunkId,
    pub page_start: PageNumber,
    pub page_end: PageNumber,
    pub text: String,
    pub citation: Citation,
    pub truncated: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct BookGetContextOutput {
    pub chunks: Vec<ContextChunk>,
    pub total_chars: usize,
    pub truncated: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BookFindDefinitionsInput {
    pub book_id: Option<BookId>,
    pub term: String,
    pub top_k: Option<usize>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BookFindExamplesInput {
    pub book_id: Option<BookId>,
    pub topic: String,
    pub top_k: Option<usize>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct ResourceReadOutput {
    pub uri: String,
    pub text: String,
    pub mime_type: String,
}
