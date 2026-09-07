use std::path::{Path, PathBuf};

use bookmcp_core::{
    BookId, BookMcpError, BookMetadata, Chapter, ChapterId, Chunk, ChunkId, Citation, Page,
    PageNumber,
};
use bookmcp_index::IndexManager;
use bookmcp_mcp::{
    BookFindDefinitionsInput, BookFindExamplesInput, BookGetChunkInput, BookGetContextInput,
    BookGetMetadataInput, BookGetPageInput, BookGetTocInput, BookListBooksInput,
    BookListLessonsInput, BookMcpServer, BookSearchInput, SearchModeInput,
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
            "book_get_library_index",
            "book_get_metadata",
            "book_get_page",
            "book_get_toc",
            "book_list_books",
            "book_list_lessons",
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
            "capture_book_lesson",
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
            offset: None,
            limit: None,
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
            max_chars: None,
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

#[test]
fn compact_index_and_pagination_expose_a_complete_library_map_without_passages() {
    let fixture = Fixture::new();
    let mut store = BookStore::open(fixture.data_dir()).unwrap();
    for index in 0..35 {
        store
            .save_ingest(batch_for_book(&format!("extra-{index:02}")))
            .unwrap();
    }
    let service = BookMcpServer::new(fixture.data_dir());
    let index = service
        .book_get_library_index(BookListBooksInput::default())
        .unwrap();
    assert_eq!(index.total_books, 36);
    assert_eq!(index.books.len(), 20);
    assert_eq!(index.next_offset, Some(20));
    assert_eq!(index.total_lessons, 0);
    assert!(index.instructions.contains("book_search"));
    assert!(index.instructions.contains("reference data"));
    assert!(
        !serde_json::to_string(&index)
            .unwrap()
            .contains("Ownership is a definition")
    );
    for book in &index.books {
        assert_eq!(
            book.metadata_uri,
            format!("book://{}/metadata", book.book_id)
        );
        assert_eq!(book.lessons_uri, format!("book://{}/lessons", book.book_id));
    }
    let second = service
        .book_get_library_index(BookListBooksInput {
            offset: index.next_offset,
            limit: None,
        })
        .unwrap();
    assert_eq!(second.books.len(), 16);
    assert_eq!(second.next_offset, None);
    let mut ids = index
        .books
        .iter()
        .chain(second.books.iter())
        .map(|book| book.book_id.clone())
        .collect::<Vec<_>>();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), 36);
    let resource = service.read_book_resource("bookmcp://library").unwrap();
    assert_eq!(resource.mime_type, "application/json");
    let parsed: serde_json::Value = serde_json::from_str(&resource.text).unwrap();
    assert_eq!(parsed["total_books"], 36);

    let (first, cursor) = service.list_resource_page(None).unwrap();
    assert_eq!(first.len(), 100);
    assert_eq!(first[0].uri, "bookmcp://library");
    let (second, end) = service.list_resource_page(cursor.as_deref()).unwrap();
    assert_eq!(second.len(), 9);
    assert_eq!(end, None);
    for cursor in [
        "",
        "100",
        "v1:+1",
        "v1:-1",
        "v1:../",
        "v1:184467440737095516160",
    ] {
        assert!(
            service.list_resource_page(Some(cursor)).is_err(),
            "accepted cursor {cursor}"
        );
    }
}

#[test]
fn oversized_metadata_has_explicit_summary_flags_and_never_produces_broken_json_resources() {
    let fixture = Fixture::new();
    let mut store = BookStore::open(fixture.data_dir()).unwrap();
    let mut batch = sample_batch();
    batch.metadata.title = "\"".repeat(30_000);
    batch.metadata.author = Some("author ".repeat(1_000));
    batch.chapters[0].title = "chapter ".repeat(1_000);
    store.save_ingest(batch).unwrap();
    let service = BookMcpServer::new(fixture.data_dir());
    let index = service
        .book_get_library_index(BookListBooksInput::default())
        .unwrap();
    assert!(index.books[0].title_truncated && index.books[0].author_truncated);
    assert_eq!(index.books[0].title.chars().count(), 256);
    assert_eq!(index.books[0].author.as_ref().unwrap().chars().count(), 128);
    let toc = service.read_book_resource("book://tiny-test/toc").unwrap();
    assert_eq!(toc.mime_type, "application/json");
    let toc: serde_json::Value = serde_json::from_str(&toc.text).unwrap();
    assert_eq!(toc["titles_truncated"], true);
    assert!(matches!(
        service.read_book_resource("book://tiny-test/metadata"),
        Err(BookMcpError::InvalidArgument { .. })
    ));
    let resource = service.read_book_resource("bookmcp://library").unwrap();
    assert!(resource.text.chars().count() <= 20_000);
    assert!(serde_json::from_str::<serde_json::Value>(&resource.text).is_ok());
}

#[test]
fn context_retains_the_target_under_tiny_unicode_budgets_and_reports_exact_fit() {
    let mut batch = sample_batch();
    batch.chunks[0].text = "Previous chunk ".repeat(1_000);
    batch.chunks[1].text = "é界🙂target".to_owned();
    let target_text = batch.chunks[1].text.clone();
    let fixture = Fixture::with_batch(batch);
    let service = BookMcpServer::new(fixture.data_dir());
    let context = service
        .book_get_context(BookGetContextInput {
            book_id: BookId::parse("tiny-test").unwrap(),
            chunk_id: ChunkId::parse("tiny-test-000002").unwrap(),
            before: Some(5),
            after: Some(5),
            max_chars: Some(3),
        })
        .unwrap();
    assert_eq!(context.chunks.len(), 1);
    assert_eq!(context.chunks[0].chunk_id.as_str(), "tiny-test-000002");
    assert_eq!(context.chunks[0].text, "é界🙂");
    assert_eq!(context.total_chars, 3);
    assert!(context.truncated && context.chunks[0].truncated);
    let exact = service
        .book_get_context(BookGetContextInput {
            book_id: BookId::parse("tiny-test").unwrap(),
            chunk_id: ChunkId::parse("tiny-test-000002").unwrap(),
            before: Some(0),
            after: Some(0),
            max_chars: Some(target_text.chars().count()),
        })
        .unwrap();
    assert_eq!(exact.chunks[0].text, target_text);
    assert!(!exact.truncated);
    assert!(!exact.chunks[0].truncated);
}

#[test]
fn chunk_text_is_capped_and_neighbors_remain_navigable() {
    let mut batch = sample_batch();
    batch.chunks[0].text = "é".repeat(30_000);
    let fixture = Fixture::with_batch(batch);
    let service = BookMcpServer::new(fixture.data_dir());
    let chunk = service
        .book_get_chunk(BookGetChunkInput {
            book_id: BookId::parse("tiny-test").unwrap(),
            chunk_id: ChunkId::parse("tiny-test-000001").unwrap(),
            include_neighbors: Some(true),
            max_chars: Some(usize::MAX),
        })
        .unwrap();
    assert_eq!(chunk.chunk.text.chars().count(), 20_000);
    assert!(chunk.truncated);
    assert_eq!(chunk.next_chunk_id.unwrap().as_str(), "tiny-test-000002");
    assert!(
        service
            .read_book_resource("book://tiny-test/chunk/tiny-test-000001")
            .unwrap()
            .text
            .ends_with("[truncated]")
    );
}

#[test]
fn rejects_zero_budgets_bad_ids_unknown_fields_and_unavailable_books() {
    let fixture = Fixture::new();
    let service = BookMcpServer::new(fixture.data_dir());
    assert!(matches!(
        service.book_list_books(BookListBooksInput {
            offset: None,
            limit: Some(0)
        }),
        Err(BookMcpError::InvalidLimit { .. })
    ));
    assert!(matches!(
        service.book_get_page(BookGetPageInput {
            book_id: BookId::parse("tiny-test").unwrap(),
            page_number: PageNumber::new(1).unwrap(),
            max_chars: Some(0),
        }),
        Err(BookMcpError::InvalidLimit { .. })
    ));
    for id in ["", "..", "../secret", "a/b", "a\\b", "a b", "a\0b"] {
        assert!(
            serde_json::from_value::<BookGetMetadataInput>(serde_json::json!({"book_id":id}))
                .is_err()
        );
    }
    assert!(
        serde_json::from_value::<BookGetPageInput>(
            serde_json::json!({"book_id":"tiny-test", "page_number":0})
        )
        .is_err()
    );
    assert!(
        serde_json::from_value::<BookGetMetadataInput>(
            serde_json::json!({"book_id":"tiny-test", "path":"/etc/passwd"})
        )
        .is_err()
    );
    assert!(matches!(
        service.book_get_toc(BookGetTocInput {
            book_id: BookId::parse("missing").unwrap(),
            offset: None,
            limit: None
        }),
        Err(BookMcpError::NotFound { .. })
    ));
    for query in ["".to_owned(), " ".to_owned(), "x".repeat(1_001)] {
        assert!(
            service
                .book_search(BookSearchInput {
                    query,
                    book_id: None,
                    top_k: None,
                    mode: None
                })
                .is_err()
        );
    }
}

#[test]
fn prompt_arguments_are_required_validated_and_included_exactly() {
    let service = BookMcpServer::new("/tmp/bookmcp-prompt-template-only");
    let arguments = serde_json::json!({"question":"What is ownership?\nKeep Unicode: café.", "book_id":"tiny-test"});
    let rendered = service
        .render_prompt("ask_book_with_citations", arguments.as_object())
        .unwrap();
    assert!(rendered.contains("What is ownership?\\nKeep Unicode: café."));
    assert!(rendered.contains("tiny-test"));
    assert!(rendered.contains("insufficient"));
    for arguments in [
        serde_json::json!({}),
        serde_json::json!({"question": false}),
        serde_json::json!({"question":" "}),
        serde_json::json!({"question":"ok", "unexpected":"ignored?"}),
        serde_json::json!({"question":"ok", "book_id":"../bad"}),
        serde_json::json!({"question":"x".repeat(20_001)}),
    ] {
        assert!(
            service
                .render_prompt("ask_book_with_citations", arguments.as_object())
                .is_err(),
            "accepted {arguments}"
        );
    }
    let capture = service.render_prompt("capture_book_lesson", serde_json::json!({"book_id":"tiny-test", "chunk_id":"tiny-test-000002", "lesson":"Check citations"}).as_object()).unwrap();
    assert!(capture.contains("bookmcp lesson add"));
    assert!(capture.contains("Do not claim the lesson was saved"));
    assert!(capture.contains("Check citations"));
}

#[test]
fn saved_lessons_are_live_paginated_cited_and_marked_stale_after_reingest() {
    let fixture = Fixture::new();
    let service = BookMcpServer::new(fixture.data_dir());
    assert_eq!(
        service
            .book_list_lessons(BookListLessonsInput::default())
            .unwrap()
            .total,
        0
    );
    let mut store = BookStore::open(fixture.data_dir()).unwrap();
    let book_id = BookId::parse("tiny-test").unwrap();
    let chunk_id = ChunkId::parse("tiny-test-000001").unwrap();
    store
        .save_lesson(
            &book_id,
            &chunk_id,
            "Verify",
            "Check source evidence before answering.",
        )
        .unwrap();
    store
        .save_lesson(&book_id, &chunk_id, "Cite", "Cite the PDF page.")
        .unwrap();
    let first = service
        .book_list_lessons(BookListLessonsInput {
            book_id: Some(book_id),
            offset: None,
            limit: Some(1),
        })
        .unwrap();
    assert_eq!(first.total, 2);
    assert_eq!(first.next_offset, Some(1));
    assert_eq!(first.lessons[0].title, "Verify");
    assert!(!first.lessons[0].stale);
    assert_eq!(first.lessons[0].citation.page_start.get(), 1);
    assert_eq!(
        service
            .book_get_library_index(BookListBooksInput::default())
            .unwrap()
            .total_lessons,
        2
    );
    let text = service
        .read_book_resource("book://tiny-test/lessons")
        .unwrap()
        .text;
    assert!(text.contains("user notes"));
    assert!(text.contains("Tiny Test Book"));
    let mut replacement = sample_batch();
    replacement.metadata.source_sha256 = "c".repeat(64);
    store.save_ingest(replacement).unwrap();
    assert!(
        service
            .book_list_lessons(BookListLessonsInput::default())
            .unwrap()
            .lessons
            .iter()
            .all(|lesson| lesson.stale)
    );
}

#[test]
fn read_only_requests_do_not_create_a_missing_library() {
    let temp = tempdir().unwrap();
    let absent = temp.path().join("missing");
    let service = BookMcpServer::new(&absent);
    assert!(
        service
            .book_list_books(BookListBooksInput::default())
            .is_err()
    );
    assert!(
        service
            .book_search(BookSearchInput {
                query: "query".to_owned(),
                book_id: None,
                top_k: None,
                mode: None
            })
            .is_err()
    );
    assert!(!absent.exists());
}

fn batch_for_book(raw: &str) -> IngestBatch {
    let mut batch = sample_batch();
    let book_id = BookId::parse(raw).unwrap();
    batch.metadata.book_id = book_id.clone();
    for page in &mut batch.pages {
        page.book_id = book_id.clone();
        page.citation.book_id = book_id.clone();
    }
    for chapter in &mut batch.chapters {
        chapter.book_id = book_id.clone();
    }
    for chunk in &mut batch.chunks {
        chunk.book_id = book_id.clone();
        chunk.citation.book_id = book_id.clone();
    }
    batch
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
