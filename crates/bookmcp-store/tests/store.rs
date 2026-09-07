use bookmcp_core::{
    BookId, BookMetadata, Chapter, ChapterId, Chunk, ChunkId, Citation, Page, PageNumber,
};
use bookmcp_store::{BookStore, IngestBatch};
use rusqlite::Connection;
use sha2::{Digest, Sha256};
use std::fs;
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

#[test]
fn read_only_library_never_creates_or_mutates_files() {
    let temp = tempdir().unwrap();
    let missing = temp.path().join("missing");
    assert!(BookStore::open_read_only(&missing).is_err());
    assert!(!missing.exists());
    let mut writer = BookStore::open(temp.path()).unwrap();
    let batch = sample_batch("Original");
    writer.save_ingest(batch.clone()).unwrap();
    let mut reader = BookStore::open_read_only(temp.path()).unwrap();
    assert_eq!(reader.list_books().unwrap(), vec![batch.metadata.clone()]);
    assert!(reader.save_ingest(sample_batch("Changed")).is_err());
    assert!(reader.initialize().is_err());
    assert!(
        reader
            .mark_index_rebuilt(&batch.metadata.book_id, "now")
            .is_err()
    );
    assert!(reader.clear_index_metadata(None).is_err());
    assert!(
        reader
            .stage_original_pdf(&batch.metadata.book_id, reader.database_path())
            .is_err()
    );
    assert!(
        reader
            .save_lesson(
                &batch.metadata.book_id,
                &batch.chunks[0].chunk_id,
                "Title",
                "Body"
            )
            .is_err()
    );
    assert_eq!(writer.list_books().unwrap(), vec![batch.metadata]);
}

#[test]
fn page_ranges_are_inclusive_ordered_and_validated() {
    let temp = tempdir().unwrap();
    let mut store = BookStore::open(temp.path()).unwrap();
    let batch = sample_batch("Range Test");
    store.save_ingest(batch.clone()).unwrap();
    assert_eq!(
        store
            .list_pages_range(
                &batch.metadata.book_id,
                PageNumber::new(2).unwrap(),
                PageNumber::new(2).unwrap()
            )
            .unwrap(),
        vec![batch.pages[1].clone()]
    );
    assert!(
        store
            .list_pages_range(
                &batch.metadata.book_id,
                PageNumber::new(2).unwrap(),
                PageNumber::new(1).unwrap()
            )
            .is_err()
    );
}

#[test]
fn staged_pdfs_are_independent_and_abandoned_stages_are_cleaned_up() {
    let temp = tempdir().unwrap();
    let store = BookStore::open(temp.path()).unwrap();
    let book = BookId::parse("tiny-test").unwrap();
    let source = temp.path().join("source.pdf");
    fs::write(&source, b"first source").unwrap();
    let first = store.stage_original_pdf(&book, &source).unwrap();
    fs::write(&source, b"second source").unwrap();
    let second = store.stage_original_pdf(&book, &source).unwrap();
    let destination = store.commit_staged_original_pdf(first).unwrap();
    assert_eq!(fs::read(&destination).unwrap(), b"first source");
    store.commit_staged_original_pdf(second).unwrap();
    assert_eq!(fs::read(&destination).unwrap(), b"second source");
    let abandoned = store.stage_original_pdf(&book, &source).unwrap();
    drop(abandoned);
    assert_eq!(
        fs::read_dir(temp.path().join("library")).unwrap().count(),
        1
    );
}

#[test]
fn coordinated_ingest_verifies_source_and_rolls_back_pdf_placement_failures() {
    let temp = tempdir().unwrap();
    let mut store = BookStore::open(temp.path()).unwrap();
    let original = sample_batch("Original");
    store.save_ingest(original.clone()).unwrap();
    let source = temp.path().join("source.pdf");
    fs::write(&source, b"replacement PDF").unwrap();
    let mut replacement = sample_batch("Replacement");
    let staged = store
        .stage_original_pdf(&replacement.metadata.book_id, &source)
        .unwrap();
    assert!(
        store
            .save_ingest_with_original(replacement.clone(), staged)
            .is_err()
    );
    assert_eq!(
        store.get_book(&original.metadata.book_id).unwrap(),
        original.metadata
    );

    replacement.metadata.source_sha256 = hex::encode(Sha256::digest(b"replacement PDF"));
    let destination = store
        .library_pdf_path(&replacement.metadata.book_id)
        .unwrap();
    fs::create_dir(&destination).unwrap();
    let staged = store
        .stage_original_pdf(&replacement.metadata.book_id, &source)
        .unwrap();
    assert!(
        store
            .save_ingest_with_original(replacement.clone(), staged)
            .is_err()
    );
    assert_eq!(
        store.get_book(&original.metadata.book_id).unwrap(),
        original.metadata
    );

    fs::remove_dir(&destination).unwrap();
    let staged = store
        .stage_original_pdf(&replacement.metadata.book_id, &source)
        .unwrap();
    store
        .save_ingest_with_original(replacement.clone(), staged)
        .unwrap();
    assert_eq!(
        store.get_book(&replacement.metadata.book_id).unwrap(),
        replacement.metadata
    );
    assert_eq!(fs::read(&destination).unwrap(), b"replacement PDF");
}

#[test]
fn coordinated_ingest_restores_original_pdf_when_database_commit_is_locked() {
    let temp = tempdir().unwrap();
    let mut store = BookStore::open(temp.path()).unwrap();
    let original = sample_batch("Original");
    store.save_ingest(original.clone()).unwrap();
    let source = temp.path().join("source.pdf");
    fs::write(&source, b"original PDF").unwrap();
    let destination = store
        .store_original_pdf(&original.metadata.book_id, &source)
        .unwrap();
    fs::write(&source, b"replacement PDF").unwrap();
    let mut replacement = sample_batch("Replacement");
    replacement.metadata.source_sha256 = hex::encode(Sha256::digest(b"replacement PDF"));
    let staged = store
        .stage_original_pdf(&replacement.metadata.book_id, &source)
        .unwrap();

    let blocker = Connection::open(store.database_path()).unwrap();
    blocker.execute_batch("BEGIN;").unwrap();
    let _: i64 = blocker
        .query_row("SELECT COUNT(*) FROM books", [], |row| row.get(0))
        .unwrap();
    let error = store
        .save_ingest_with_original(replacement, staged)
        .unwrap_err();
    assert!(error.to_string().contains("locked"));
    blocker.execute_batch("ROLLBACK;").unwrap();
    assert_eq!(
        store.get_book(&original.metadata.book_id).unwrap(),
        original.metadata
    );
    assert_eq!(fs::read(&destination).unwrap(), b"original PDF");
    assert_eq!(
        fs::read_dir(temp.path().join("library")).unwrap().count(),
        1
    );
}

#[test]
fn lessons_capture_authoritative_citations_and_survive_changed_or_deleted_sources() {
    let temp = tempdir().unwrap();
    let mut store = BookStore::open(temp.path()).unwrap();
    let batch = sample_batch("Original");
    store.save_ingest(batch.clone()).unwrap();
    let lesson = store
        .save_lesson(
            &batch.metadata.book_id,
            &batch.chunks[0].chunk_id,
            " A lesson ",
            " Interpret the source carefully. ",
        )
        .unwrap();
    assert_eq!(lesson.title, "A lesson");
    assert_eq!(lesson.body, "Interpret the source carefully.");
    assert_eq!(lesson.citation, batch.chunks[0].citation);
    assert_eq!(lesson.source_sha256, batch.metadata.source_sha256);
    assert!(!lesson.created_at.is_empty());
    assert!(!lesson.stale);

    let reader = BookStore::open_read_only(temp.path()).unwrap();
    assert_eq!(
        reader
            .list_lessons(Some(&batch.metadata.book_id), 0, 10)
            .unwrap(),
        vec![lesson.clone()]
    );
    let mut replacement = sample_batch("New Edition");
    replacement.metadata.source_sha256 = "b".repeat(64);
    store.save_ingest(replacement).unwrap();
    let updated = reader.list_lessons(None, 0, 10).unwrap();
    assert_eq!(updated.len(), 1);
    assert!(updated[0].stale);
    assert_eq!(updated[0].citation, lesson.citation);
    assert_eq!(updated[0].source_sha256, lesson.source_sha256);

    let conn = Connection::open(store.database_path()).unwrap();
    conn.execute("DELETE FROM books", []).unwrap();
    assert_eq!(reader.count_lessons(None).unwrap(), 1);
    assert!(reader.list_lessons(None, 0, 10).unwrap()[0].stale);
    store.delete_lesson(&lesson.lesson_id).unwrap();
    assert_eq!(reader.count_lessons(None).unwrap(), 0);
    assert!(store.delete_lesson(&lesson.lesson_id).is_err());
}

#[test]
fn lesson_limits_and_references_are_validated_before_writing() {
    let temp = tempdir().unwrap();
    let mut store = BookStore::open(temp.path()).unwrap();
    let batch = sample_batch("Original");
    store.save_ingest(batch.clone()).unwrap();
    let book = &batch.metadata.book_id;
    let chunk = &batch.chunks[0].chunk_id;
    for (title, body) in [
        ("".to_owned(), "body".to_owned()),
        ("x".repeat(161), "body".to_owned()),
        ("title".to_owned(), " ".to_owned()),
        ("title".to_owned(), "x".repeat(4_001)),
        ("title".to_owned(), "bad\0text".to_owned()),
    ] {
        assert!(store.save_lesson(book, chunk, &title, &body).is_err());
    }
    assert!(
        store
            .save_lesson(book, &ChunkId::parse("missing").unwrap(), "Title", "Body")
            .is_err()
    );
    assert_eq!(store.count_lessons(None).unwrap(), 0);
    for limit in [0, 101, usize::MAX] {
        assert!(store.list_lessons(None, 0, limit).is_err());
    }
    assert!(store.list_lessons(None, usize::MAX, 10).is_err());
    let first = store.save_lesson(book, chunk, "First", "One").unwrap();
    let second = store.save_lesson(book, chunk, "Second", "Two").unwrap();
    assert_eq!(store.list_lessons(Some(book), 0, 1).unwrap(), vec![first]);
    assert_eq!(store.list_lessons(Some(book), 1, 1).unwrap(), vec![second]);
    assert_eq!(store.count_lessons(Some(book)).unwrap(), 2);
    assert_eq!(
        store
            .count_lessons(Some(&BookId::parse("other").unwrap()))
            .unwrap(),
        0
    );
}

#[test]
fn legacy_libraries_are_read_without_migration_and_live_readers_observe_new_lessons() {
    let temp = tempdir().unwrap();
    let mut store = BookStore::open(temp.path()).unwrap();
    let batch = sample_batch("Legacy");
    store.save_ingest(batch.clone()).unwrap();
    let conn = Connection::open(store.database_path()).unwrap();
    conn.execute_batch("DROP TABLE lessons; DELETE FROM schema_migrations WHERE version = 2;")
        .unwrap();
    drop(store);
    let reader = BookStore::open_read_only(temp.path()).unwrap();
    assert_eq!(reader.count_lessons(None).unwrap(), 0);
    assert!(reader.list_lessons(None, 0, 10).unwrap().is_empty());
    let version: i64 = conn
        .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(version, 1);
    let mut migrated = BookStore::open(temp.path()).unwrap();
    let lesson = migrated
        .save_lesson(
            &batch.metadata.book_id,
            &batch.chunks[0].chunk_id,
            "New",
            "A saved lesson",
        )
        .unwrap();
    assert_eq!(reader.list_lessons(None, 0, 10).unwrap(), vec![lesson]);
    assert_eq!(reader.count_lessons(None).unwrap(), 1);
}

#[test]
fn unsupported_or_corrupted_schema_is_reported() {
    let temp = tempdir().unwrap();
    let store = BookStore::open(temp.path()).unwrap();
    let conn = Connection::open(store.database_path()).unwrap();
    conn.execute_batch("DROP TABLE lessons;").unwrap();
    assert!(BookStore::open_read_only(temp.path()).is_err());
    conn.execute("INSERT INTO schema_migrations (version) VALUES (999)", [])
        .unwrap();
    assert!(BookStore::open(temp.path()).is_err());
    assert!(BookStore::open_read_only(temp.path()).is_err());
}

#[test]
fn integrity_check_detects_inconsistent_counts_citations_and_foreign_keys() {
    let temp = tempdir().unwrap();
    let mut store = BookStore::open(temp.path()).unwrap();
    let batch = sample_batch("Integrity");
    store.save_ingest(batch.clone()).unwrap();
    store.check_integrity().unwrap();
    let conn = Connection::open(store.database_path()).unwrap();
    conn.execute("UPDATE books SET chunk_count = 10", [])
        .unwrap();
    assert!(
        store
            .check_integrity()
            .unwrap_err()
            .to_string()
            .contains("declares")
    );
    conn.execute("UPDATE books SET chunk_count = 3", [])
        .unwrap();
    let mut citation = batch.chunks[0].citation.clone();
    citation.page_end = PageNumber::new(2).unwrap();
    conn.execute(
        "UPDATE chunks SET citation_json = ?1 WHERE chunk_id = 'tiny-test-000001'",
        [serde_json::to_string(&citation).unwrap()],
    )
    .unwrap();
    assert!(
        store
            .check_integrity()
            .unwrap_err()
            .to_string()
            .contains("citation")
    );
    conn.execute_batch("PRAGMA foreign_keys = OFF; DELETE FROM books;")
        .unwrap();
    assert!(
        store
            .check_integrity()
            .unwrap_err()
            .to_string()
            .contains("foreign key")
    );
}

#[test]
fn corrupted_source_citations_do_not_create_lessons() {
    let temp = tempdir().unwrap();
    let mut store = BookStore::open(temp.path()).unwrap();
    let batch = sample_batch("Corrupt");
    store.save_ingest(batch.clone()).unwrap();
    let conn = Connection::open(store.database_path()).unwrap();
    conn.execute("UPDATE chunks SET citation_json = 'invalid JSON'", [])
        .unwrap();
    assert!(
        store
            .save_lesson(
                &batch.metadata.book_id,
                &batch.chunks[0].chunk_id,
                "Title",
                "Body"
            )
            .is_err()
    );
    assert_eq!(store.count_lessons(None).unwrap(), 0);
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
