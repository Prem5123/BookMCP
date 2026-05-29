use std::path::{Path, PathBuf};

use bookmcp_core::{
    BookId, BookMetadata, Chapter, ChapterId, Chunk, ChunkId, Citation, Page, PageNumber,
};
use bookmcp_index::IndexManager;
use bookmcp_mcp::{
    BookFindDefinitionsInput, BookFindExamplesInput, BookGetChunkInput, BookGetContextInput,
    BookGetMetadataInput, BookGetPageInput, BookGetTocInput, BookListBooksInput, BookMcpServer,
    BookSearchInput, SearchModeInput,
};
use bookmcp_store::{BookStore, IngestBatch};
use tempfile::{TempDir, tempdir};

#[test]
fn service_exposes_required_tool_names_resources_and_prompts() {
    let fixture = Fixture::new();
    let service = BookMcpServer::new(fixture.data_dir());

    assert_eq!(
        service.tool_names(),
        vec![
            "book_find_definitions",
            "book_find_examples",
            "book_get_chunk",
            "book_get_context",
            "book_get_metadata",
            "book_get_page",
            "book_get_toc",
            "book_list_books",
            "book_search",
        ]
    );

    let templates = service.resource_templates();
    assert!(
        templates
            .iter()
            .any(|template| template.uri_template == "book://{book_id}/metadata")
    );
    assert!(
        templates
            .iter()
            .any(|template| template.uri_template == "book://{book_id}/page/{page_number}")
    );

    let prompts = service.prompt_catalog();
    let names = prompts
        .iter()
        .map(|prompt| prompt.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        vec![
            "ask_book_with_citations",
            "compare_book_sections",
            "extract_actionable_rules",
            "review_against_book",
            "study_chapter",
        ]
    );
}

#[test]
fn list_metadata_toc_search_page_chunk_and_context_work() {
    let fixture = Fixture::new();
    let service = BookMcpServer::new(fixture.data_dir());
    let book_id = BookId::parse("tiny-test").unwrap();

    let books = service
        .book_list_books(BookListBooksInput {
            offset: None,
            limit: None,
        })
        .unwrap();
    assert_eq!(books.books.len(), 1);
    assert_eq!(books.books[0].book_id.as_str(), "tiny-test");

    let metadata = service
        .book_get_metadata(BookGetMetadataInput {
            book_id: book_id.clone(),
        })
        .unwrap();
    assert_eq!(metadata.metadata.title, "Tiny Test Book");
    assert_eq!(metadata.chunk_count, 3);

    let toc = service
        .book_get_toc(BookGetTocInput {
            book_id: book_id.clone(),
        })
        .unwrap();
    assert_eq!(toc.chapters[0].title, "Opening");
    assert!(toc.message.is_none());

    let search = service
        .book_search(BookSearchInput {
            query: "definition".to_owned(),
            book_id: Some(book_id.clone()),
            top_k: Some(5),
            mode: Some(SearchModeInput::Keyword),
        })
        .unwrap();
    assert!(!search.results.is_empty());
    assert!(
        search.results[0]
            .citation
            .format()
            .contains("Tiny Test Book")
    );

    let page = service
        .book_get_page(BookGetPageInput {
            book_id: book_id.clone(),
            page_number: PageNumber::new(1).unwrap(),
            max_chars: Some(24),
        })
        .unwrap();
    assert!(page.truncated);
    assert!(page.text.chars().count() <= 24);
    assert_eq!(
        page.citation.format(),
        "Tiny Test Book, chapter \"Opening\", p. 1"
    );

    let chunk = service
        .book_get_chunk(BookGetChunkInput {
            book_id: book_id.clone(),
            chunk_id: ChunkId::parse("tiny-test-000002").unwrap(),
            include_neighbors: Some(true),
        })
        .unwrap();
    assert_eq!(chunk.chunk.chunk_id.as_str(), "tiny-test-000002");
    assert_eq!(
        chunk.previous_chunk_id.unwrap().as_str(),
        "tiny-test-000001"
    );
    assert_eq!(chunk.next_chunk_id.unwrap().as_str(), "tiny-test-000003");

    let context = service
        .book_get_context(BookGetContextInput {
            book_id,
            chunk_id: ChunkId::parse("tiny-test-000002").unwrap(),
            before: Some(10),
            after: Some(10),
            max_chars: Some(120),
        })
        .unwrap();
    assert!(context.chunks.len() <= 3);
    assert!(
        context
            .chunks
            .iter()
            .any(|chunk| chunk.chunk_id.as_str() == "tiny-test-000002")
    );
    assert!(context.total_chars <= 120);
}

#[test]
fn definition_and_example_helpers_return_cited_results() {
    let fixture = Fixture::new();
    let service = BookMcpServer::new(fixture.data_dir());
    let book_id = Some(BookId::parse("tiny-test").unwrap());

    let definitions = service
        .book_find_definitions(BookFindDefinitionsInput {
            book_id: book_id.clone(),
            term: "ownership".to_owned(),
            top_k: Some(5),
        })
        .unwrap();
    assert!(
        definitions
            .results
            .iter()
            .any(|result| result.snippet.contains("definition"))
    );
    assert!(
        definitions.results[0]
            .citation
            .format()
            .contains("Tiny Test Book")
    );

    let examples = service
        .book_find_examples(BookFindExamplesInput {
            book_id,
            topic: "ownership".to_owned(),
            top_k: Some(5),
        })
        .unwrap();
    assert!(
        examples
            .results
            .iter()
            .any(|result| result.snippet.contains("example"))
    );
    assert!(
        examples.results[0]
            .citation
            .format()
            .contains("Tiny Test Book")
    );
}

#[test]
fn resources_are_read_only_and_validated() {
    let fixture = Fixture::new();
    let service = BookMcpServer::new(fixture.data_dir());

    let metadata = service
        .read_book_resource("book://tiny-test/metadata")
        .unwrap();
    assert_eq!(metadata.uri, "book://tiny-test/metadata");
    assert!(metadata.text.contains("Tiny Test Book"));

    let page = service
        .read_book_resource("book://tiny-test/page/1")
        .unwrap();
    assert!(page.text.contains("Ownership is a definition"));

    let chunk = service
        .read_book_resource("book://tiny-test/chunk/tiny-test-000001")
        .unwrap();
    assert!(chunk.text.contains("definition"));

    let chapter = service
        .read_book_resource("book://tiny-test/chapter/opening")
        .unwrap();
    assert!(chapter.text.contains("example"));

    let invalid = service
        .read_book_resource("file:///etc/passwd")
        .unwrap_err();
    assert!(invalid.to_string().contains("unsupported resource URI"));

    let traversal = service
        .read_book_resource("book://../metadata")
        .unwrap_err();
    assert!(traversal.to_string().contains("invalid"));
}

#[test]
fn resources_are_capped() {
    let mut batch = sample_batch();
    batch.pages[0].text = "large chapter page ".repeat(2_000);
    let fixture = Fixture::with_batch(batch);
    let service = BookMcpServer::new(fixture.data_dir());

    let chapter = service
        .read_book_resource("book://tiny-test/chapter/opening")
        .unwrap();

    assert!(chapter.text.chars().count() <= 20_000);
    assert!(chapter.text.ends_with("[truncated]"));
}

#[test]
fn prompt_texts_instruct_agents_to_use_tools_and_citations() {
    let service = BookMcpServer::new(PathBuf::from("/tmp/bookmcp-test-only"));

    let prompt = service.get_prompt_text("ask_book_with_citations").unwrap();
    assert!(prompt.contains("book_search"));
    assert!(prompt.contains("book_get_chunk"));
    assert!(prompt.contains("citations"));

    let missing = service.get_prompt_text("missing_prompt").unwrap_err();
    assert!(missing.to_string().contains("unknown prompt"));
}

struct Fixture {
    _temp: TempDir,
    data_dir: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        Self::with_batch(sample_batch())
    }

    fn with_batch(batch: IngestBatch) -> Self {
        let temp = tempdir().unwrap();
        let data_dir = temp.path().join("bookmcp");
        let mut store = BookStore::open(&data_dir).unwrap();
        store.save_ingest(batch).unwrap();
        let chunks = store
            .list_chunks(&BookId::parse("tiny-test").unwrap())
            .unwrap();
        IndexManager::create_or_open(data_dir.join("index"))
            .unwrap()
            .rebuild(&chunks)
            .unwrap();

        Self {
            _temp: temp,
            data_dir,
        }
    }

    fn data_dir(&self) -> &Path {
        &self.data_dir
    }
}

fn sample_batch() -> IngestBatch {
    let book_id = BookId::parse("tiny-test").unwrap();
    let chapter_id = ChapterId::parse("opening").unwrap();
    let pages = [
        (
            PageNumber::new(1).unwrap(),
            "Ownership is a definition for responsibility in this tiny book.",
        ),
        (
            PageNumber::new(2).unwrap(),
            "An ownership example shows how citations keep answers grounded.",
        ),
        (
            PageNumber::new(3).unwrap(),
            "The final page gives another rule and example for agents.",
        ),
    ];
    let citations = pages
        .iter()
        .map(|(page, _)| {
            Citation::new(
                book_id.clone(),
                "Tiny Test Book".to_owned(),
                None,
                *page,
                *page,
                Some("Opening".to_owned()),
            )
            .unwrap()
        })
        .collect::<Vec<_>>();

    IngestBatch {
        metadata: BookMetadata {
            book_id: book_id.clone(),
            title: "Tiny Test Book".to_owned(),
            author: None,
            source_sha256: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                .to_owned(),
            page_count: 3,
            chapter_count: 1,
            chunk_count: 3,
            ingested_at: "2026-05-29T00:00:00Z".to_owned(),
        },
        pages: pages
            .iter()
            .zip(citations.iter())
            .map(|((page_number, text), citation)| Page {
                book_id: book_id.clone(),
                page_number: *page_number,
                text: (*text).to_owned(),
                citation: citation.clone(),
            })
            .collect(),
        chapters: vec![Chapter {
            chapter_id: chapter_id.clone(),
            book_id: book_id.clone(),
            title: "Opening".to_owned(),
            page_start: PageNumber::new(1).unwrap(),
            page_end: PageNumber::new(3).unwrap(),
        }],
        chunks: pages
            .iter()
            .zip(citations)
            .enumerate()
            .map(|(index, ((page_number, text), citation))| Chunk {
                chunk_id: ChunkId::parse(format!("tiny-test-{:06}", index + 1)).unwrap(),
                book_id: book_id.clone(),
                chapter_id: Some(chapter_id.clone()),
                chapter_title: Some("Opening".to_owned()),
                page_start: *page_number,
                page_end: *page_number,
                text: (*text).to_owned(),
                citation,
            })
            .collect(),
    }
}
