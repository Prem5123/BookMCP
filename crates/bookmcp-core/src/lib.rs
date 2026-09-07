#![forbid(unsafe_code)]

use std::{fmt, str::FromStr};

use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};
use thiserror::Error;

/// Result type used by BookMCP library crates.
pub type Result<T> = std::result::Result<T, BookMcpError>;

/// Maximum number of search results returned by default service boundaries.
pub const MAX_TOP_K: usize = 50;

/// Maximum accepted query length for search-facing APIs.
pub const MAX_QUERY_CHARS: usize = 1_000;

/// Shared typed error for BookMCP library crates.
#[derive(Debug, Error)]
pub enum BookMcpError {
    /// An argument failed a domain boundary constraint.
    #[error("invalid {name}: {reason}")]
    InvalidArgument {
        /// Argument name.
        name: &'static str,
        /// Explanation of the constraint.
        reason: String,
    },
    /// A user- or system-provided identifier failed validation.
    #[error("invalid {kind} `{value}`: {reason}")]
    InvalidId {
        /// Identifier kind, such as `book_id`.
        kind: &'static str,
        /// Rejected value.
        value: String,
        /// Human-readable reason.
        reason: &'static str,
    },

    /// Page numbers exposed by BookMCP are 1-based.
    #[error("page numbers are 1-based; got {value}")]
    InvalidPageNumber {
        /// Rejected page number.
        value: u32,
    },

    /// A page range had its start after its end.
    #[error("invalid page range: start page {start} is after end page {end}")]
    InvalidPageRange {
        /// Start page.
        start: u32,
        /// End page.
        end: u32,
    },

    /// A PDF produced no useful text.
    #[error("no extractable text found in PDF")]
    NoExtractableText,

    /// The PDF appears to need OCR, which is not part of the initial release.
    #[error("OCR required: {reason}")]
    OcrRequired {
        /// Why OCR is required.
        reason: String,
    },

    /// The PDF is encrypted or password protected.
    #[error("PDF is encrypted or password protected")]
    PdfEncrypted,

    /// A requested entity does not exist in the local library.
    #[error("{entity} not found: {id}")]
    NotFound {
        /// Entity kind.
        entity: &'static str,
        /// Requested identifier.
        id: String,
    },

    /// A query was empty after normalization.
    #[error("query must not be empty")]
    EmptyQuery,

    /// A query exceeded the service cap.
    #[error("query too long: {actual} characters exceeds maximum {max}")]
    QueryTooLong {
        /// Actual query length.
        actual: usize,
        /// Maximum query length.
        max: usize,
    },

    /// A requested limit exceeded the service cap.
    #[error("invalid limit {value}; maximum is {max}")]
    InvalidLimit {
        /// Requested value.
        value: usize,
        /// Maximum accepted value.
        max: usize,
    },

    /// Storage layer failure.
    #[error("storage error: {0}")]
    Storage(String),

    /// Search index failure.
    #[error("index error: {0}")]
    Index(String),

    /// Ingestion pipeline failure.
    #[error("ingest error: {0}")]
    Ingest(String),

    /// PDF extraction failure.
    #[error("PDF extraction error: {0}")]
    Pdf(String),

    /// MCP server failure.
    #[error("MCP error: {0}")]
    Mcp(String),

    /// Standard I/O failure.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Trait implemented by validated identifier newtypes.
pub trait ValidatedId: Sized {
    /// Identifier kind used in error messages.
    const KIND: &'static str;

    /// Parse and validate an identifier.
    fn parse(raw: impl Into<String>) -> Result<Self>;

    /// Borrow the validated identifier string.
    fn as_str(&self) -> &str;
}

macro_rules! validated_id {
    ($name:ident, $kind:literal, $doc:literal) => {
        #[doc = $doc]
        #[derive(Clone, Debug, Eq, Hash, JsonSchema, Ord, PartialEq, PartialOrd, Serialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// Parse and validate this identifier.
            pub fn parse(raw: impl Into<String>) -> Result<Self> {
                <Self as ValidatedId>::parse(raw)
            }

            /// Borrow the validated identifier string.
            pub fn as_str(&self) -> &str {
                <Self as ValidatedId>::as_str(self)
            }
        }

        impl ValidatedId for $name {
            const KIND: &'static str = $kind;

            fn parse(raw: impl Into<String>) -> Result<Self> {
                let raw = raw.into();
                validate_identifier(Self::KIND, &raw)?;
                Ok(Self(raw))
            }

            fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl FromStr for $name {
            type Err = BookMcpError;

            fn from_str(raw: &str) -> Result<Self> {
                Self::parse(raw)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let raw = String::deserialize(deserializer)?;
                Self::parse(raw).map_err(serde::de::Error::custom)
            }
        }
    };
}

validated_id!(
    BookId,
    "book_id",
    "Validated identifier for a book in the local library."
);
validated_id!(
    ChunkId,
    "chunk_id",
    "Validated identifier for a text chunk."
);
validated_id!(
    ChapterId,
    "chapter_id",
    "Validated identifier for a detected chapter or section."
);

validated_id!(
    LessonId,
    "lesson_id",
    "Validated identifier for a saved lesson."
);

fn validate_identifier(kind: &'static str, raw: &str) -> Result<()> {
    let reason = if raw.is_empty() {
        Some("identifier must not be empty")
    } else if raw.len() > 128 {
        Some("identifier must not exceed 128 bytes")
    } else if raw == "." || raw == ".." {
        Some("identifier must not be `.` or `..`")
    } else if raw.contains('/') || raw.contains('\\') {
        Some("identifier must not contain path separators")
    } else if raw.contains('\0') {
        Some("identifier must not contain null bytes")
    } else if raw.chars().any(char::is_whitespace) {
        Some("identifier must not contain whitespace")
    } else if !raw.is_ascii() {
        Some("identifier must contain only ASCII characters")
    } else if !raw
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
    {
        Some("identifier may contain only ASCII letters, numbers, `.`, `-`, and `_`")
    } else {
        None
    };

    if let Some(reason) = reason {
        return Err(BookMcpError::InvalidId {
            kind,
            value: raw.to_owned(),
            reason,
        });
    }

    Ok(())
}

/// One-based page number used at all public boundaries.
#[derive(Clone, Copy, Debug, Eq, Hash, JsonSchema, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct PageNumber(#[schemars(range(min = 1))] u32);

impl<'de> Deserialize<'de> for PageNumber {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::new(u32::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

impl PageNumber {
    /// Create a one-based page number.
    pub fn new(value: u32) -> Result<Self> {
        if value == 0 {
            return Err(BookMcpError::InvalidPageNumber { value });
        }

        Ok(Self(value))
    }

    /// Return the one-based page number.
    pub fn get(self) -> u32 {
        self.0
    }

    /// Validate that `start..=end` is an ordered page range.
    pub fn range(start: Self, end: Self) -> Result<()> {
        if start > end {
            return Err(BookMcpError::InvalidPageRange {
                start: start.get(),
                end: end.get(),
            });
        }

        Ok(())
    }
}

impl fmt::Display for PageNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.get())
    }
}

/// Human-readable citation metadata attached to returned book content.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct Citation {
    /// Book identifier.
    pub book_id: BookId,
    /// Display title.
    pub title: String,
    /// Display author when known.
    pub author: Option<String>,
    /// First source page covered by the cited content.
    pub page_start: PageNumber,
    /// Last source page covered by the cited content.
    pub page_end: PageNumber,
    /// Chapter title when known.
    pub chapter_title: Option<String>,
}

impl Citation {
    /// Create a validated citation.
    pub fn new(
        book_id: BookId,
        title: String,
        author: Option<String>,
        page_start: PageNumber,
        page_end: PageNumber,
        chapter_title: Option<String>,
    ) -> Result<Self> {
        PageNumber::range(page_start, page_end)?;

        Ok(Self {
            book_id,
            title,
            author,
            page_start,
            page_end,
            chapter_title,
        })
    }

    /// Format the citation for human-facing CLI and MCP output.
    pub fn format(&self) -> String {
        let mut rendered = self.title.clone();

        if let Some(author) = &self.author {
            rendered.push_str(" by ");
            rendered.push_str(author);
        }

        if let Some(chapter_title) = &self.chapter_title {
            rendered.push_str(", chapter \"");
            rendered.push_str(chapter_title);
            rendered.push('"');
        }

        if self.page_start == self.page_end {
            rendered.push_str(", p. ");
            rendered.push_str(&self.page_start.to_string());
        } else {
            rendered.push_str(", pp. ");
            rendered.push_str(&self.page_start.to_string());
            rendered.push('-');
            rendered.push_str(&self.page_end.to_string());
        }

        rendered
    }
}

/// Metadata stored for an ingested book.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct BookMetadata {
    /// Stable book identifier.
    pub book_id: BookId,
    /// Display title.
    pub title: String,
    /// Display author when known.
    pub author: Option<String>,
    /// SHA-256 hash of the source PDF.
    pub source_sha256: String,
    /// Number of extracted pages.
    pub page_count: u32,
    /// Number of detected chapters.
    pub chapter_count: u32,
    /// Number of generated chunks.
    pub chunk_count: u32,
    /// Ingestion timestamp as an RFC 3339 string.
    pub ingested_at: String,
}

/// Extracted text for one source page.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct Page {
    /// Book identifier.
    pub book_id: BookId,
    /// One-based source page number.
    pub page_number: PageNumber,
    /// Extracted page text.
    pub text: String,
    /// Citation for this page.
    pub citation: Citation,
}

/// Detected chapter or section.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct Chapter {
    /// Chapter identifier.
    pub chapter_id: ChapterId,
    /// Book identifier.
    pub book_id: BookId,
    /// Chapter title.
    pub title: String,
    /// First page in this chapter.
    pub page_start: PageNumber,
    /// Last page in this chapter.
    pub page_end: PageNumber,
}

/// Searchable text chunk.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct Chunk {
    /// Chunk identifier.
    pub chunk_id: ChunkId,
    /// Book identifier.
    pub book_id: BookId,
    /// Chapter identifier when this chunk belongs to a known chapter.
    pub chapter_id: Option<ChapterId>,
    /// Chapter title when known.
    pub chapter_title: Option<String>,
    /// First source page covered by this chunk.
    pub page_start: PageNumber,
    /// Last source page covered by this chunk.
    pub page_end: PageNumber,
    /// Chunk text.
    pub text: String,
    /// Citation for this chunk.
    pub citation: Citation,
}

/// Keyword search request.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SearchQuery {
    /// User query text.
    pub query: String,
    /// Optional book filter.
    pub book_id: Option<BookId>,
    /// Optional result count.
    pub top_k: Option<usize>,
}

/// Search mode exposed by public APIs.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchMode {
    /// Tantivy BM25 keyword search.
    Keyword,
}

/// Search result returned by index and MCP APIs.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
pub struct SearchResult {
    /// Book identifier.
    pub book_id: BookId,
    /// Chunk identifier.
    pub chunk_id: ChunkId,
    /// Ranking score.
    pub score: f32,
    /// First source page covered by the chunk.
    pub page_start: PageNumber,
    /// Last source page covered by the chunk.
    pub page_end: PageNumber,
    /// Chapter title when known.
    pub chapter_title: Option<String>,
    /// Highlighted or plain snippet.
    pub snippet: String,
    /// Citation for the result.
    pub citation: Citation,
}

/// Summary returned after a successful ingest.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct IngestReport {
    /// Book identifier.
    pub book_id: BookId,
    /// Display title.
    pub title: String,
    /// SHA-256 hash of the source PDF.
    pub source_sha256: String,
    /// Number of extracted pages.
    pub page_count: u32,
    /// Number of generated chunks.
    pub chunk_count: u32,
    /// Number of indexed chunks.
    pub indexed_chunks: u32,
}

/// A user-authored interpretation linked to the exact source version it came from.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct Lesson {
    /// Stable identifier for managing the lesson through the CLI.
    pub lesson_id: LessonId,
    /// Source book.
    pub book_id: BookId,
    /// Source chunk at capture time.
    pub chunk_id: ChunkId,
    /// Source PDF hash at capture time.
    pub source_sha256: String,
    /// Short title for the lesson.
    pub title: String,
    /// User-authored interpretation, never represented as a verbatim quote.
    pub body: String,
    /// Citation captured from the source chunk.
    pub citation: Citation,
    /// Capture time in RFC 3339 format.
    pub created_at: String,
    /// True when the source no longer matches the captured version.
    pub stale: bool,
}
