use bookmcp_core::{
    BookId, BookMetadata, Chapter, ChapterId, Chunk, ChunkId, Citation, Page, PageNumber,
};
use bookmcp_store::{BookStore, IngestBatch};
use tempfile::tempdir;

#[test]
fn open_initializes_database_in_data_dir() {
    let temp = tempdir().unwrap();
    let store = BookStore::open(temp.path()).unwrap();

    assert!(store.database_path().ends_with("bookmcp.sqlite3"));
    assert!(store.database_path().is_file());
    assert!(store.list_books().unwrap().is_empty());
}

#[test]
fn ingest_batch_persists_and_reads_book_content() {
    let temp = tempdir().unwrap();
    let mut store = BookStore::open(temp.path()).unwrap();
    let batch = sample_batch("Tiny Test Book");

    store.save_ingest(batch.clone()).unwrap();

    assert_eq!(store.list_books().unwrap(), vec![batch.metadata.clone()]);
    assert_eq!(
        store.get_book(&batch.metadata.book_id).unwrap(),
        batch.metadata
    );
    assert_eq!(
        store
            .get_page(
                &BookId::parse("tiny-test").unwrap(),
                PageNumber::new(1).unwrap()
            )
            .unwrap()
            .text,
        "This is page one with a test concept."
    );
    assert_eq!(
        store
            .get_chapter(
                &BookId::parse("tiny-test").unwrap(),
                &ChapterId::parse("opening").unwrap()
            )
            .unwrap()
            .title,
        "Opening"
    );
    assert_eq!(
        store
            .get_chunk(
                &BookId::parse("tiny-test").unwrap(),
                &ChunkId::parse("tiny-test-000002").unwrap()
            )
            .unwrap()
            .text,
        "Second chunk contains another test example."
    );
}

#[test]
fn duplicate_ingest_replaces_existing_book_rows() {
    let temp = tempdir().unwrap();
    let mut store = BookStore::open(temp.path()).unwrap();
    store.save_ingest(sample_batch("Old Title")).unwrap();

    let mut replacement = sample_batch("New Title");
    replacement.metadata.chunk_count = 1;
    replacement.chunks.truncate(1);
    replacement.chunks[0].text = "Replacement chunk text.".to_owned();

    store.save_ingest(replacement.clone()).unwrap();

    assert_eq!(
        store.get_book(&replacement.metadata.book_id).unwrap().title,
        "New Title"
    );
    assert_eq!(
        store
            .list_chunks(&replacement.metadata.book_id)
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        store
            .get_chunk(
                &replacement.metadata.book_id,
                &ChunkId::parse("tiny-test-000001").unwrap()
            )
            .unwrap()
            .text,
        "Replacement chunk text."
    );
}

#[test]
fn failed_ingest_does_not_corrupt_existing_book() {
    let temp = tempdir().unwrap();
    let mut store = BookStore::open(temp.path()).unwrap();
    let original = sample_batch("Stable Title");
    store.save_ingest(original.clone()).unwrap();

    let mut invalid = sample_batch("Broken Title");
    invalid.pages.push(invalid.pages[0].clone());

    let err = store.save_ingest(invalid).unwrap_err();

    assert!(err.to_string().contains("storage error"));
    assert_eq!(
        store.get_book(&original.metadata.book_id).unwrap().title,
        "Stable Title"
    );
    assert_eq!(
        store.list_pages(&original.metadata.book_id).unwrap().len(),
        2
    );
}

#[test]
fn chunks_around_returns_ordered_context_window() {
    let temp = tempdir().unwrap();
    let mut store = BookStore::open(temp.path()).unwrap();
    store.save_ingest(sample_batch("Tiny Test Book")).unwrap();

    let around = store
        .get_chunks_around(
            &BookId::parse("tiny-test").unwrap(),
            &ChunkId::parse("tiny-test-000002").unwrap(),
            1,
            1,
        )
        .unwrap();

    let ids = around
        .iter()
        .map(|chunk| chunk.chunk_id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        ids,
        vec!["tiny-test-000001", "tiny-test-000002", "tiny-test-000003"]
    );
}

#[test]
fn index_metadata_can_be_marked_and_cleared() {
    let temp = tempdir().unwrap();
    let mut store = BookStore::open(temp.path()).unwrap();
    let book_id = BookId::parse("tiny-test").unwrap();
    store.save_ingest(sample_batch("Tiny Test Book")).unwrap();

    store
        .mark_index_rebuilt(&book_id, "2026-05-28T23:30:00Z")
        .unwrap();
    assert_eq!(
        store.index_rebuilt_at(&book_id).unwrap(),
        Some("2026-05-28T23:30:00Z".to_owned())
    );

    store.clear_index_metadata(Some(&book_id)).unwrap();
    assert_eq!(store.index_rebuilt_at(&book_id).unwrap(), None);
}

fn sample_batch(title: &str) -> IngestBatch {
    let book_id = BookId::parse("tiny-test").unwrap();
    let page_one = PageNumber::new(1).unwrap();
    let page_two = PageNumber::new(2).unwrap();
    let chapter_id = ChapterId::parse("opening").unwrap();

    let citation_one = Citation::new(
        book_id.clone(),
        title.to_owned(),
        Some("A. Writer".to_owned()),
        page_one,
        page_one,
        Some("Opening".to_owned()),
    )
    .unwrap();
    let citation_two = Citation::new(
        book_id.clone(),
        title.to_owned(),
        Some("A. Writer".to_owned()),
        page_two,
        page_two,
        Some("Opening".to_owned()),
    )
    .unwrap();

    IngestBatch {
        metadata: BookMetadata {
            book_id: book_id.clone(),
            title: title.to_owned(),
            author: Some("A. Writer".to_owned()),
            source_sha256: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                .to_owned(),
            page_count: 2,
            chapter_count: 1,
            chunk_count: 3,
            ingested_at: "2026-05-28T23:30:00Z".to_owned(),
        },
        pages: vec![
            Page {
                book_id: book_id.clone(),
                page_number: page_one,
                text: "This is page one with a test concept.".to_owned(),
                citation: citation_one.clone(),
            },
            Page {
                book_id: book_id.clone(),
                page_number: page_two,
                text: "This is page two with a test example.".to_owned(),
                citation: citation_two.clone(),
            },
        ],
        chapters: vec![Chapter {
            chapter_id: chapter_id.clone(),
            book_id: book_id.clone(),
            title: "Opening".to_owned(),
            page_start: page_one,
            page_end: page_two,
        }],
        chunks: vec![
            Chunk {
                chunk_id: ChunkId::parse("tiny-test-000001").unwrap(),
                book_id: book_id.clone(),
                chapter_id: Some(chapter_id.clone()),
                chapter_title: Some("Opening".to_owned()),
                page_start: page_one,
                page_end: page_one,
                text: "First chunk contains the test concept.".to_owned(),
                citation: citation_one,
            },
            Chunk {
                chunk_id: ChunkId::parse("tiny-test-000002").unwrap(),
                book_id: book_id.clone(),
                chapter_id: Some(chapter_id.clone()),
                chapter_title: Some("Opening".to_owned()),
                page_start: page_two,
                page_end: page_two,
                text: "Second chunk contains another test example.".to_owned(),
                citation: citation_two.clone(),
            },
            Chunk {
                chunk_id: ChunkId::parse("tiny-test-000003").unwrap(),
                book_id,
                chapter_id: Some(chapter_id),
                chapter_title: Some("Opening".to_owned()),
                page_start: page_two,
                page_end: page_two,
                text: "Third chunk closes the example.".to_owned(),
                citation: citation_two,
            },
        ],
    }
}
