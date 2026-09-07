#![forbid(unsafe_code)]

use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

use bookmcp_core::{
    BookId, BookMcpError, Chunk, ChunkId, Citation, MAX_QUERY_CHARS, MAX_TOP_K, PageNumber, Result,
    SearchQuery, SearchResult,
};
use tantivy::{
    DocAddress, Index, TantivyDocument, Term,
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
    read_only: bool,
}

impl IndexManager {
    /// Build and publish a complete replacement index, including recovery from corrupt metadata.
    /// Callers must serialize all writers for the library across this operation. Existing search
    /// services observe the replacement on their next search. Publication errors restore the
    /// previous directory; a process crash during the directory swap may require another rebuild.
    pub fn recreate(index_dir: impl AsRef<Path>, chunks: &[Chunk]) -> Result<()> {
        let index_dir = index_dir.as_ref();
        let parent = index_dir
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(parent)?;
        match fs::symlink_metadata(index_dir) {
            Ok(metadata) if !metadata.is_dir() || metadata.file_type().is_symlink() => {
                return Err(BookMcpError::Index(
                    "index recovery requires a directory, not a file or symbolic link".to_owned(),
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        let staging = tempfile::Builder::new()
            .prefix(".bookmcp-index-rebuild-")
            .tempdir_in(parent)?;
        let rebuilt = staging.path().join("rebuilt");
        Self::create_or_open(&rebuilt)?.rebuild(chunks)?;

        publish_rebuilt(index_dir, staging)
    }

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
            read_only: false,
        })
    }

    /// Open an existing keyword index without creating it or allowing writes.
    pub fn open_read_only(index_dir: impl AsRef<Path>) -> Result<Self> {
        let index_dir = index_dir.as_ref();
        if !index_dir.join("meta.json").is_file() {
            return Err(BookMcpError::Index(
                "search index is missing; run `bookmcp rebuild-index` first".to_owned(),
            ));
        }
        let index = Index::open_in_dir(index_dir).map_err(index_error)?;
        let fields = IndexFields::from_schema(&index.schema())?;
        Ok(Self {
            index,
            fields,
            index_path: index_dir.to_path_buf(),
            read_only: true,
        })
    }

    /// Return the index directory.
    pub fn index_path(&self) -> &Path {
        &self.index_path
    }

    /// Compare every live indexed document with the authoritative chunks, including text and citations.
    pub fn check_chunks(&self, chunks: &[Chunk]) -> Result<()> {
        let mut expected = HashMap::with_capacity(chunks.len());
        for chunk in chunks {
            validate_chunk(chunk)?;
            if expected
                .insert((chunk.book_id.clone(), chunk.chunk_id.clone()), chunk)
                .is_some()
            {
                return Err(BookMcpError::Index(
                    "duplicate authoritative chunks".to_owned(),
                ));
            }
        }
        let reader = self.index.reader().map_err(index_error)?;
        let searcher = reader.searcher();
        for (segment_ordinal, segment) in searcher.segment_readers().iter().enumerate() {
            let segment_ordinal = u32::try_from(segment_ordinal).map_err(index_error)?;
            for doc_id in 0..segment.max_doc() {
                if segment.is_deleted(doc_id) {
                    continue;
                }
                let doc = searcher
                    .doc::<TantivyDocument>(DocAddress::new(segment_ordinal, doc_id))
                    .map_err(index_error)?;
                let book_id =
                    BookId::parse(required_text(&doc, self.fields.book_id, BOOK_ID_FIELD)?)?;
                let chunk_id =
                    ChunkId::parse(required_text(&doc, self.fields.chunk_id, CHUNK_ID_FIELD)?)?;
                let chunk = expected.remove(&(book_id.clone(), chunk_id.clone())).ok_or_else(|| {
                    BookMcpError::Index(format!("unexpected or duplicate indexed chunk {book_id}:{chunk_id}; run `bookmcp rebuild-index`"))
                })?;
                let citation: Citation = serde_json::from_str(&required_text(
                    &doc,
                    self.fields.citation_json,
                    CITATION_JSON_FIELD,
                )?)
                .map_err(index_error)?;
                if required_text(&doc, self.fields.text, TEXT_FIELD)? != chunk.text
                    || required_page(&doc, self.fields.page_start, PAGE_START_FIELD)?
                        != chunk.page_start
                    || required_page(&doc, self.fields.page_end, PAGE_END_FIELD)? != chunk.page_end
                    || required_text(&doc, self.fields.chapter_title, CHAPTER_TITLE_FIELD)?
                        != chunk.chapter_title.as_deref().unwrap_or("")
                    || citation != chunk.citation
                {
                    return Err(BookMcpError::Index(format!(
                        "stale indexed chunk {book_id}:{chunk_id}; run `bookmcp rebuild-index`"
                    )));
                }
            }
        }
        if !expected.is_empty() {
            return Err(BookMcpError::Index(format!(
                "{} chunks are missing from the search index; run `bookmcp rebuild-index`",
                expected.len()
            )));
        }
        Ok(())
    }

    /// Rebuild the keyword index from a complete chunk set.
    pub fn rebuild(&self, chunks: &[Chunk]) -> Result<()> {
        self.write_chunks(None, chunks)
    }

    /// Atomically replace one book's indexed chunks while retaining every other book.
    /// An empty chunk set removes only the specified book from the index.
    pub fn replace_book(&self, book_id: &BookId, chunks: &[Chunk]) -> Result<()> {
        self.write_chunks(Some(book_id), chunks)
    }

    fn write_chunks(&self, book_id: Option<&BookId>, chunks: &[Chunk]) -> Result<()> {
        if self.read_only {
            return Err(BookMcpError::Index("search index is read-only".to_owned()));
        }
        let mut identifiers = HashSet::with_capacity(chunks.len());
        for chunk in chunks {
            if book_id.is_some_and(|expected| expected != &chunk.book_id) {
                return Err(BookMcpError::Index(format!(
                    "chunk {} belongs to a different book",
                    chunk.chunk_id
                )));
            }
            if !identifiers.insert((&chunk.book_id, &chunk.chunk_id)) {
                return Err(BookMcpError::Index(format!(
                    "duplicate chunk {} in book {}",
                    chunk.chunk_id, chunk.book_id
                )));
            }
            validate_chunk(chunk)?;
        }
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
        if let Some(book_id) = book_id {
            writer.delete_term(Term::from_field_text(self.fields.book_id, book_id.as_str()));
        } else {
            writer.delete_all_documents().map_err(index_error)?;
        }

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
        let query_chars = query.query.chars().count();
        if query_chars > MAX_QUERY_CHARS {
            return Err(BookMcpError::QueryTooLong {
                actual: query_chars,
                max: MAX_QUERY_CHARS,
            });
        }
        let raw_query = query.query.trim();
        if raw_query.is_empty() {
            return Ok(SearchOutput {
                results: Vec::new(),
                message: Some("query is empty".to_owned()),
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
                page_start: required_page(
                    &doc,
                    self.index_manager.fields.page_start,
                    PAGE_START_FIELD,
                )?,
                page_end: required_page(&doc, self.index_manager.fields.page_end, PAGE_END_FIELD)?,
                chapter_title,
                snippet: self.snippet_builder.build(&text, raw_query),
                citation,
            });
            if let Some(result) = results.last() {
                PageNumber::range(result.page_start, result.page_end)?;
                if result.citation.book_id != result.book_id
                    || result.citation.page_start != result.page_start
                    || result.citation.page_end != result.page_end
                {
                    return Err(BookMcpError::Index(
                        "stored citation does not match its search result".to_owned(),
                    ));
                }
            }
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
        if schema != &build_schema() {
            return Err(BookMcpError::Index(
                "search index schema is incompatible with this BookMCP version".to_owned(),
            ));
        }
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

fn required_page(doc: &TantivyDocument, field: Field, name: &'static str) -> Result<PageNumber> {
    let value = required_u64(doc, field, name)?;
    let page = u32::try_from(value).map_err(|_| {
        BookMcpError::Index(format!("stored {name} value {value} is outside u32 range"))
    })?;
    PageNumber::new(page)
}

fn validate_chunk(chunk: &Chunk) -> Result<()> {
    PageNumber::range(chunk.page_start, chunk.page_end)?;
    if chunk.citation.book_id != chunk.book_id
        || chunk.citation.page_start != chunk.page_start
        || chunk.citation.page_end != chunk.page_end
    {
        return Err(BookMcpError::Index(format!(
            "citation does not match chunk {}",
            chunk.chunk_id
        )));
    }
    Ok(())
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

fn publish_rebuilt(index_dir: &Path, staging: tempfile::TempDir) -> Result<()> {
    let rebuilt = staging.path().join("rebuilt");
    let previous = staging.path().join("previous");
    let had_previous = match fs::rename(index_dir, &previous) {
        Ok(()) => true,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        Err(error) => return Err(error.into()),
    };
    if let Err(publish_error) = fs::rename(&rebuilt, index_dir) {
        if had_previous && let Err(restore_error) = fs::rename(&previous, index_dir) {
            let recovery = staging.keep();
            return Err(BookMcpError::Index(format!(
                "index publication failed ({publish_error}); restoring the previous index failed ({restore_error}); recovery files retained at {}",
                recovery.display()
            )));
        }
        return Err(BookMcpError::Index(format!(
            "index publication failed: {publish_error}"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::publish_rebuilt;
    use std::fs;

    #[test]
    fn failed_index_publication_restores_the_previous_directory() {
        let root = tempfile::tempdir().unwrap();
        let index = root.path().join("index");
        fs::create_dir(&index).unwrap();
        fs::write(index.join("original"), b"preserve this index").unwrap();
        let staging = tempfile::tempdir_in(root.path()).unwrap();
        // A lost staging directory forces failure after the original has been moved aside.
        let error = publish_rebuilt(&index, staging).unwrap_err();
        assert!(error.to_string().contains("publication failed"));
        assert_eq!(
            fs::read(index.join("original")).unwrap(),
            b"preserve this index"
        );
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
    }
}
