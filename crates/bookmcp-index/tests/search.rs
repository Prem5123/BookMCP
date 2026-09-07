use bookmcp_core::{BookId, Chunk, ChunkId, Citation, PageNumber, SearchQuery};
use bookmcp_index::{EmbeddingProvider, IndexManager, SearchService, VectorIndex};
use tantivy::doc;
use tempfile::tempdir;

#[test]
fn index_and_search_finds_exact_terms() {
    let temp = tempdir().unwrap();
    let manager = IndexManager::create_or_open(temp.path()).unwrap();
    manager.rebuild(&sample_chunks()).unwrap();
    let service = SearchService::new(manager);

    let output = service
        .search(SearchQuery {
            query: "ownership".to_owned(),
            book_id: None,
            top_k: Some(10),
        })
        .unwrap();

    assert!(output.message.is_none());
    let rust_result = output
        .results
        .iter()
        .find(|result| result.chunk_id.as_str() == "rust-book-000001")
        .unwrap();
    assert!(rust_result.snippet.to_lowercase().contains("ownership"));
    assert_eq!(rust_result.citation.format(), "Rust Book, p. 1");
}

#[test]
fn search_respects_book_id_filters() {
    let temp = tempdir().unwrap();
    let manager = IndexManager::create_or_open(temp.path()).unwrap();
    manager.rebuild(&sample_chunks()).unwrap();
    let service = SearchService::new(manager);

    let output = service
        .search(SearchQuery {
            query: "ownership".to_owned(),
            book_id: Some(BookId::parse("systems-book").unwrap()),
            top_k: Some(10),
        })
        .unwrap();

    assert_eq!(output.results.len(), 1);
    assert_eq!(output.results[0].book_id.as_str(), "systems-book");
    assert_eq!(output.results[0].chunk_id.as_str(), "systems-book-000001");
}

#[test]
fn search_caps_top_k_to_safe_maximum() {
    let temp = tempdir().unwrap();
    let manager = IndexManager::create_or_open(temp.path()).unwrap();
    manager.rebuild(&sample_chunks()).unwrap();
    let service = SearchService::new(manager);

    let output = service
        .search(SearchQuery {
            query: "test".to_owned(),
            book_id: None,
            top_k: Some(5_000),
        })
        .unwrap();

    assert!(output.results.len() <= 50);
}

#[test]
fn search_handles_punctuation_quotes_and_empty_queries() {
    let temp = tempdir().unwrap();
    let manager = IndexManager::create_or_open(temp.path()).unwrap();
    manager.rebuild(&sample_chunks()).unwrap();
    let service = SearchService::new(manager);

    let punctuation = service
        .search(SearchQuery {
            query: "\"ownership???".to_owned(),
            book_id: None,
            top_k: Some(10),
        })
        .unwrap();
    assert!(!punctuation.results.is_empty());

    let empty = service
        .search(SearchQuery {
            query: "   ".to_owned(),
            book_id: None,
            top_k: Some(10),
        })
        .unwrap();
    assert!(empty.results.is_empty());
    assert_eq!(empty.message.as_deref(), Some("query is empty"));
}

#[test]
fn rebuilding_index_is_deterministic_for_same_chunks() {
    let temp = tempdir().unwrap();
    let manager = IndexManager::create_or_open(temp.path()).unwrap();
    let chunks = sample_chunks();
    manager.rebuild(&chunks).unwrap();
    let service = SearchService::new(manager);

    let first = service
        .search(SearchQuery {
            query: "test".to_owned(),
            book_id: None,
            top_k: Some(10),
        })
        .unwrap()
        .results
        .into_iter()
        .map(|result| result.chunk_id)
        .collect::<Vec<_>>();

    service.index_manager().rebuild(&chunks).unwrap();

    let second = service
        .search(SearchQuery {
            query: "test".to_owned(),
            book_id: None,
            top_k: Some(10),
        })
        .unwrap()
        .results
        .into_iter()
        .map(|result| result.chunk_id)
        .collect::<Vec<_>>();

    assert_eq!(first, second);
}

#[test]
fn future_vector_traits_are_object_safe_boundaries() {
    struct NoopEmbeddings;
    impl EmbeddingProvider for NoopEmbeddings {
        fn embed(&self, text: &str) -> bookmcp_core::Result<Vec<f32>> {
            Ok(vec![text.len() as f32])
        }
    }

    struct NoopVectorIndex;
    impl VectorIndex for NoopVectorIndex {
        fn upsert(&self, chunk_id: &ChunkId, embedding: &[f32]) -> bookmcp_core::Result<()> {
            assert_eq!(chunk_id.as_str(), "rust-book-000001");
            assert_eq!(embedding, &[9.0]);
            Ok(())
        }
    }

    let provider: &dyn EmbeddingProvider = &NoopEmbeddings;
    let vector_index: &dyn VectorIndex = &NoopVectorIndex;
    let embedding = provider.embed("ownership").unwrap();
    vector_index
        .upsert(&ChunkId::parse("rust-book-000001").unwrap(), &embedding)
        .unwrap();
}

#[test]
fn replacing_one_book_preserves_other_books_and_refreshes_existing_readers() {
    let temp = tempdir().unwrap();
    let manager = IndexManager::create_or_open(temp.path()).unwrap();
    manager.rebuild(&sample_chunks()).unwrap();
    let service = SearchService::new(IndexManager::open_read_only(temp.path()).unwrap());
    let search = |text: &str| {
        service
            .search(SearchQuery {
                query: text.to_owned(),
                book_id: None,
                top_k: Some(50),
            })
            .unwrap()
    };
    assert_eq!(search("ownership").results.len(), 2);

    let book = BookId::parse("rust-book").unwrap();
    manager
        .replace_book(
            &book,
            &[chunk(
                "rust-book",
                "rust-book-000003",
                "Concurrency primitives",
                3,
            )],
        )
        .unwrap();
    assert_eq!(search("ownership").results.len(), 1);
    assert_eq!(
        search("ownership").results[0].book_id.as_str(),
        "systems-book"
    );
    assert_eq!(search("concurrency").results.len(), 1);
    assert!(search("borrowing").results.is_empty());

    manager.replace_book(&book, &[]).unwrap();
    assert!(search("concurrency").results.is_empty());
    assert_eq!(search("ownership").results.len(), 1);
}

#[test]
fn invalid_replacements_leave_the_committed_index_untouched() {
    let temp = tempdir().unwrap();
    let manager = IndexManager::create_or_open(temp.path()).unwrap();
    manager.rebuild(&sample_chunks()).unwrap();
    let book = BookId::parse("rust-book").unwrap();
    assert!(manager.replace_book(&book, &sample_chunks()).is_err());
    let duplicate = sample_chunks()[0].clone();
    assert!(manager.rebuild(&[duplicate.clone(), duplicate]).is_err());
    let mut mismatched_citation = sample_chunks()[0].clone();
    mismatched_citation.citation.book_id = BookId::parse("wrong-book").unwrap();
    assert!(manager.rebuild(&[mismatched_citation]).is_err());

    let service = SearchService::new(manager);
    assert_eq!(
        service
            .search(SearchQuery {
                query: "ownership".to_owned(),
                book_id: None,
                top_k: None,
            })
            .unwrap()
            .results
            .len(),
        2
    );
}

#[test]
fn read_only_open_never_creates_an_index_and_rejects_writes() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("missing");
    assert!(IndexManager::open_read_only(&path).is_err());
    assert!(!path.exists());
    IndexManager::create_or_open(&path)
        .unwrap()
        .rebuild(&sample_chunks())
        .unwrap();
    let reader = IndexManager::open_read_only(&path).unwrap();
    assert!(reader.rebuild(&[]).is_err());
    assert!(
        reader
            .replace_book(&BookId::parse("rust-book").unwrap(), &[])
            .is_err()
    );
}

#[test]
fn query_cap_counts_untrimmed_input() {
    let temp = tempdir().unwrap();
    let service = SearchService::new(IndexManager::create_or_open(temp.path()).unwrap());
    for query in [" ".repeat(1_001), format!("{}ownership", " ".repeat(1_000))] {
        assert!(matches!(
            service.search(SearchQuery {
                query,
                book_id: None,
                top_k: None
            }),
            Err(bookmcp_core::BookMcpError::QueryTooLong { .. })
        ));
    }
}

#[test]
fn integrity_check_detects_missing_extra_and_equal_count_stale_index_records() {
    let temp = tempdir().unwrap();
    let manager = IndexManager::create_or_open(temp.path()).unwrap();
    let chunks = sample_chunks();
    manager.rebuild(&chunks).unwrap();
    manager.check_chunks(&chunks).unwrap();
    let mut changed = chunks.clone();
    changed[0].text = "Updated source text with the same ID and record count.".to_owned();
    assert!(
        manager
            .check_chunks(&changed)
            .unwrap_err()
            .to_string()
            .contains("stale")
    );
    assert!(
        manager
            .check_chunks(&chunks[..1])
            .unwrap_err()
            .to_string()
            .contains("unexpected")
    );
    manager
        .replace_book(&BookId::parse("rust-book").unwrap(), &[])
        .unwrap();
    assert!(
        manager
            .check_chunks(&chunks)
            .unwrap_err()
            .to_string()
            .contains("missing")
    );
    manager.check_chunks(&chunks[2..]).unwrap();
}

#[test]
fn corrupted_stored_page_numbers_cannot_wrap_into_valid_citations() {
    let temp = tempdir().unwrap();
    let manager = IndexManager::create_or_open(temp.path()).unwrap();
    let index = tantivy::Index::open_in_dir(temp.path()).unwrap();
    let schema = index.schema();
    let field = |name| schema.get_field(name).unwrap();
    let citation = serde_json::to_string(&sample_chunks()[0].citation).unwrap();
    let mut writer = index.writer(50_000_000).unwrap();
    writer
        .add_document(doc!(
            field("book_id") => "rust-book",
            field("chunk_id") => "rust-book-000001",
            field("text") => "ownership",
            field("chapter_title") => "",
            field("citation_json") => citation,
            field("page_start") => u64::from(u32::MAX) + 2,
            field("page_end") => u64::from(u32::MAX) + 2,
        ))
        .unwrap();
    writer.commit().unwrap();
    let error = SearchService::new(manager)
        .search(SearchQuery {
            query: "ownership".to_owned(),
            book_id: None,
            top_k: None,
        })
        .unwrap_err();
    assert!(error.to_string().contains("outside u32 range"));
}

#[test]
fn recreate_recovers_corrupt_or_missing_metadata_and_existing_search_services() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("index");
    let manager = IndexManager::create_or_open(&path).unwrap();
    let chunks = sample_chunks();
    manager.rebuild(&chunks).unwrap();
    let service = SearchService::new(IndexManager::open_read_only(&path).unwrap());
    let search = || {
        service.search(SearchQuery {
            query: "ownership".to_owned(),
            book_id: None,
            top_k: Some(50),
        })
    };
    assert_eq!(search().unwrap().results.len(), 2);

    std::fs::write(path.join("meta.json"), b"invalid index JSON").unwrap();
    assert!(IndexManager::create_or_open(&path).is_err());
    assert!(search().is_err());
    IndexManager::recreate(&path, &chunks[2..]).unwrap();
    assert_eq!(search().unwrap().results.len(), 1);
    IndexManager::open_read_only(&path)
        .unwrap()
        .check_chunks(&chunks[2..])
        .unwrap();

    std::fs::remove_file(path.join("meta.json")).unwrap();
    IndexManager::recreate(&path, &chunks).unwrap();
    assert_eq!(search().unwrap().results.len(), 2);
    assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 1);
}

#[test]
fn recreate_build_errors_preserve_the_previous_index() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("index");
    let chunks = sample_chunks();
    IndexManager::recreate(&path, &chunks).unwrap();
    let duplicate = chunks[0].clone();
    assert!(IndexManager::recreate(&path, &[duplicate.clone(), duplicate]).is_err());
    IndexManager::open_read_only(&path)
        .unwrap()
        .check_chunks(&chunks)
        .unwrap();
    assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 1);
}

fn sample_chunks() -> Vec<Chunk> {
    vec![
        chunk(
            "rust-book",
            "rust-book-000001",
            "Ownership gives Rust memory safety without garbage collection. This test passage is exact.",
            1,
        ),
        chunk(
            "rust-book",
            "rust-book-000002",
            "Borrowing examples show references and lifetimes in a practical test.",
            2,
        ),
        chunk(
            "systems-book",
            "systems-book-000001",
            "Systems design uses ownership of responsibilities as a testable boundary.",
            4,
        ),
    ]
}

fn chunk(book_id: &str, chunk_id: &str, text: &str, page: u32) -> Chunk {
    let book_id = BookId::parse(book_id).unwrap();
    let page = PageNumber::new(page).unwrap();
    let citation = Citation::new(
        book_id.clone(),
        if book_id.as_str() == "rust-book" {
            "Rust Book".to_owned()
        } else {
            "Systems Book".to_owned()
        },
        None,
        page,
        page,
        None,
    )
    .unwrap();

    Chunk {
        chunk_id: ChunkId::parse(chunk_id).unwrap(),
        book_id,
        chapter_id: None,
        chapter_title: None,
        page_start: page,
        page_end: page,
        text: text.to_owned(),
        citation,
    }
}
