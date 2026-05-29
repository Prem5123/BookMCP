#![forbid(unsafe_code)]

use std::{
    fs,
    path::{Path, PathBuf},
};

use bookmcp_core::{
    BookMcpError, Chunk, ChunkId, Citation, MAX_QUERY_CHARS, MAX_TOP_K, PageNumber, Result,
    SearchQuery, SearchResult,
};
use tantivy::{
    Index, TantivyDocument, Term,
    collector::TopDocs,
    doc,
    query::{BooleanQuery, Occur, Query, QueryParser, TermQuery},
    schema::{Field, IndexRecordOption, STORED, STRING, Schema, TEXT, Value},
};

const BOOK_ID_FIELD: &str = "book_id";
const CHUNK_ID_FIELD: &str = "chunk_id";
const TEXT_FIELD: &str = "text";
const PAGE_START_FIELD: &str = "page_start";
const PAGE_END_FIELD: &str = "page_end";
const CHAPTER_TITLE_FIELD: &str = "chapter_title";
const CITATION_JSON_FIELD: &str = "citation_json";
const DEFAULT_TOP_K: usize = 10;
const WRITER_MEMORY_BYTES: usize = 50_000_000;

/// Search response with optional user-facing message.
#[derive(Clone, Debug, PartialEq)]
pub struct SearchOutput {
    /// Ranked results.
    pub results: Vec<SearchResult>,
    /// Useful message for empty or no-result searches.
    pub message: Option<String>,
}

/// Manages a persistent Tantivy keyword index.
pub struct IndexManager {
    index: Index,
    fields: IndexFields,
    index_path: PathBuf,
}

impl IndexManager {
    /// Create or open a Tantivy index in `index_dir`.
    pub fn create_or_open(index_dir: impl AsRef<Path>) -> Result<Self> {
        let index_dir = index_dir.as_ref();
        fs::create_dir_all(index_dir)?;
        let schema = build_schema();
        let index = if index_dir.join("meta.json").is_file() {
            Index::open_in_dir(index_dir).map_err(index_error)?
        } else {
            Index::create_in_dir(index_dir, schema).map_err(index_error)?
        };
        let fields = IndexFields::from_schema(&index.schema())?;

        Ok(Self {
            index,
            fields,
            index_path: index_dir.to_path_buf(),
        })
    }

    /// Return the index directory.
    pub fn index_path(&self) -> &Path {
        &self.index_path
    }

    /// Rebuild the keyword index from a complete chunk set.
    pub fn rebuild(&self, chunks: &[Chunk]) -> Result<()> {
        let mut sorted_chunks = chunks.iter().collect::<Vec<_>>();
        sorted_chunks.sort_by(|left, right| {
            left.book_id
                .cmp(&right.book_id)
                .then_with(|| left.chunk_id.cmp(&right.chunk_id))
        });

        let mut writer = self
            .index
            .writer(WRITER_MEMORY_BYTES)
            .map_err(index_error)?;
        writer.delete_all_documents().map_err(index_error)?;

        for chunk in sorted_chunks {
            let citation_json = serde_json::to_string(&chunk.citation).map_err(index_error)?;
            writer
                .add_document(doc!(
                    self.fields.book_id => chunk.book_id.as_str(),
                    self.fields.chunk_id => chunk.chunk_id.as_str(),
                    self.fields.text => chunk.text.as_str(),
                    self.fields.page_start => u64::from(chunk.page_start.get()),
                    self.fields.page_end => u64::from(chunk.page_end.get()),
                    self.fields.chapter_title => chunk.chapter_title.as_deref().unwrap_or(""),
                    self.fields.citation_json => citation_json,
                ))
                .map_err(index_error)?;
        }

        writer.commit().map(|_| ()).map_err(index_error)
    }
}

/// Keyword search service over a Tantivy index.
pub struct SearchService {
    index_manager: IndexManager,
    snippet_builder: SnippetBuilder,
}

impl SearchService {
    /// Create a search service.
    pub fn new(index_manager: IndexManager) -> Self {
        Self {
            index_manager,
            snippet_builder: SnippetBuilder::default(),
        }
    }

    /// Access the index manager for rebuilds.
    pub fn index_manager(&self) -> &IndexManager {
        &self.index_manager
    }

    /// Run BM25 keyword search.
    pub fn search(&self, query: SearchQuery) -> Result<SearchOutput> {
        let raw_query = query.query.trim();
        if raw_query.is_empty() {
            return Ok(SearchOutput {
                results: Vec::new(),
                message: Some("query is empty".to_owned()),
            });
        }

        let query_chars = raw_query.chars().count();
        if query_chars > MAX_QUERY_CHARS {
            return Err(BookMcpError::QueryTooLong {
                actual: query_chars,
                max: MAX_QUERY_CHARS,
            });
        }

        let top_k = query.top_k.unwrap_or(DEFAULT_TOP_K).min(MAX_TOP_K);
        if top_k == 0 {
            return Ok(SearchOutput {
                results: Vec::new(),
                message: Some("top_k is zero".to_owned()),
            });
        }

        let Some(parsed_query) = self.parse_query(raw_query, query.book_id.as_ref())? else {
            return Ok(SearchOutput {
                results: Vec::new(),
                message: Some("query has no searchable terms".to_owned()),
            });
        };

        let reader = self.index_manager.index.reader().map_err(index_error)?;
        let searcher = reader.searcher();
        let top_docs = searcher
            .search(&parsed_query, &TopDocs::with_limit(top_k).order_by_score())
            .map_err(index_error)?;
        let mut results = Vec::with_capacity(top_docs.len());

        for (score, doc_address) in top_docs {
            let doc = searcher
                .doc::<TantivyDocument>(doc_address)
                .map_err(index_error)?;
            let text = required_text(&doc, self.index_manager.fields.text, TEXT_FIELD)?;
            let citation_json = required_text(
                &doc,
                self.index_manager.fields.citation_json,
                CITATION_JSON_FIELD,
            )?;
            let citation: Citation = serde_json::from_str(&citation_json).map_err(index_error)?;
            let chapter_title = optional_text(&doc, self.index_manager.fields.chapter_title)
                .filter(|s| !s.is_empty());

            results.push(SearchResult {
                book_id: bookmcp_core::BookId::parse(required_text(
                    &doc,
                    self.index_manager.fields.book_id,
                    BOOK_ID_FIELD,
                )?)?,
                chunk_id: ChunkId::parse(required_text(
                    &doc,
                    self.index_manager.fields.chunk_id,
                    CHUNK_ID_FIELD,
                )?)?,
                score,
                page_start: PageNumber::new(required_u64(
                    &doc,
                    self.index_manager.fields.page_start,
                    PAGE_START_FIELD,
                )? as u32)?,
                page_end: PageNumber::new(required_u64(
                    &doc,
                    self.index_manager.fields.page_end,
                    PAGE_END_FIELD,
                )? as u32)?,
                chapter_title,
                snippet: self.snippet_builder.build(&text, raw_query),
                citation,
            });
        }

        let message = if results.is_empty() {
            Some("no results".to_owned())
        } else {
            None
        };

        Ok(SearchOutput { results, message })
    }

    fn parse_query(
        &self,
        raw_query: &str,
        book_id: Option<&bookmcp_core::BookId>,
    ) -> Result<Option<Box<dyn Query>>> {
        let mut parser = QueryParser::for_index(
            &self.index_manager.index,
            vec![
                self.index_manager.fields.text,
                self.index_manager.fields.chapter_title,
            ],
        );
        parser.set_conjunction_by_default();

        let text_query = match parser.parse_query(raw_query) {
            Ok(query) => Some(query),
            Err(_) => {
                let fallback = fallback_query_text(raw_query);
                if fallback.is_empty() {
                    None
                } else {
                    Some(parser.parse_query(&fallback).map_err(index_error)?)
                }
            }
        };

        let Some(text_query) = text_query else {
            return Ok(None);
        };

        if let Some(book_id) = book_id {
            let book_filter = Box::new(TermQuery::new(
                Term::from_field_text(self.index_manager.fields.book_id, book_id.as_str()),
                IndexRecordOption::Basic,
            ));
            Ok(Some(Box::new(BooleanQuery::new(vec![
                (Occur::Must, text_query),
                (Occur::Must, book_filter),
            ]))))
        } else {
            Ok(Some(text_query))
        }
    }
}

/// Builds compact text snippets for search results.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SnippetBuilder {
    max_chars: usize,
}

impl SnippetBuilder {
    /// Create a snippet builder with a maximum character count.
    pub fn new(max_chars: usize) -> Self {
        Self { max_chars }
    }

    /// Build a query-centered snippet when possible.
    pub fn build(&self, text: &str, query: &str) -> String {
        let clean_text = text.trim();
        if clean_text.chars().count() <= self.max_chars {
            return clean_text.to_owned();
        }

        let lowered_text = clean_text.to_ascii_lowercase();
        let term = first_search_term(query);
        let center = term
            .and_then(|term| {
                lowered_text
                    .find(&term)
                    .map(|byte_index| byte_to_char_index(clean_text, byte_index))
            })
            .unwrap_or(0);
        let half = self.max_chars / 2;
        let start = center.saturating_sub(half);
        let end = start
            .saturating_add(self.max_chars)
            .min(clean_text.chars().count());
        slice_chars(clean_text, start, end)
    }
}

impl Default for SnippetBuilder {
    fn default() -> Self {
        Self { max_chars: 240 }
    }
}

/// Future boundary for local embedding providers.
pub trait EmbeddingProvider {
    /// Embed text locally.
    fn embed(&self, text: &str) -> Result<Vec<f32>>;
}

/// Future boundary for vector indexes.
pub trait VectorIndex {
    /// Upsert one chunk embedding.
    fn upsert(&self, chunk_id: &ChunkId, embedding: &[f32]) -> Result<()>;
}

#[derive(Clone, Copy, Debug)]
struct IndexFields {
    book_id: Field,
    chunk_id: Field,
    text: Field,
    page_start: Field,
    page_end: Field,
    chapter_title: Field,
    citation_json: Field,
}

impl IndexFields {
    fn from_schema(schema: &Schema) -> Result<Self> {
        Ok(Self {
            book_id: schema.get_field(BOOK_ID_FIELD).map_err(index_error)?,
            chunk_id: schema.get_field(CHUNK_ID_FIELD).map_err(index_error)?,
            text: schema.get_field(TEXT_FIELD).map_err(index_error)?,
            page_start: schema.get_field(PAGE_START_FIELD).map_err(index_error)?,
            page_end: schema.get_field(PAGE_END_FIELD).map_err(index_error)?,
            chapter_title: schema.get_field(CHAPTER_TITLE_FIELD).map_err(index_error)?,
            citation_json: schema.get_field(CITATION_JSON_FIELD).map_err(index_error)?,
        })
    }
}

fn build_schema() -> Schema {
    let mut schema = Schema::builder();
    schema.add_text_field(BOOK_ID_FIELD, STRING | STORED);
    schema.add_text_field(CHUNK_ID_FIELD, STRING | STORED);
    schema.add_text_field(TEXT_FIELD, TEXT | STORED);
    schema.add_u64_field(PAGE_START_FIELD, STORED);
    schema.add_u64_field(PAGE_END_FIELD, STORED);
    schema.add_text_field(CHAPTER_TITLE_FIELD, TEXT | STORED);
    schema.add_text_field(CITATION_JSON_FIELD, STORED);
    schema.build()
}

fn required_text(doc: &TantivyDocument, field: Field, name: &'static str) -> Result<String> {
    doc.get_first(field)
        .and_then(|value| value.as_str())
        .map(str::to_owned)
        .ok_or_else(|| BookMcpError::Index(format!("missing stored text field `{name}`")))
}

fn optional_text(doc: &TantivyDocument, field: Field) -> Option<String> {
    doc.get_first(field)
        .and_then(|value| value.as_str())
        .map(str::to_owned)
}

fn required_u64(doc: &TantivyDocument, field: Field, name: &'static str) -> Result<u64> {
    doc.get_first(field)
        .and_then(|value| value.as_u64())
        .ok_or_else(|| BookMcpError::Index(format!("missing stored u64 field `{name}`")))
}

fn fallback_query_text(raw_query: &str) -> String {
    raw_query
        .split(|ch: char| !ch.is_alphanumeric())
        .filter(|term| !term.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn first_search_term(query: &str) -> Option<String> {
    fallback_query_text(query)
        .split_whitespace()
        .next()
        .map(str::to_ascii_lowercase)
}

fn byte_to_char_index(text: &str, byte_index: usize) -> usize {
    text[..byte_index].chars().count()
}

fn slice_chars(text: &str, start: usize, end: usize) -> String {
    text.chars().skip(start).take(end - start).collect()
}

fn index_error(error: impl std::fmt::Display) -> BookMcpError {
    BookMcpError::Index(error.to_string())
}
