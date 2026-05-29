use bookmcp_core::{BookId, Chunk, ChunkId, Citation, PageNumber, SearchQuery};
use bookmcp_index::{EmbeddingProvider, IndexManager, SearchService, VectorIndex};
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
