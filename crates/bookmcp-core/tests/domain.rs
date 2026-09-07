use std::io;

use bookmcp_core::{BookId, BookMcpError, ChapterId, ChunkId, Citation, PageNumber};

#[test]
fn ids_accept_safe_ascii_identifiers() {
    let book_id = BookId::parse("tiny-test_2026.v1").unwrap();
    let chunk_id = ChunkId::parse("tiny-test-000001").unwrap();
    let chapter_id = ChapterId::parse("chapter.01").unwrap();

    assert_eq!(book_id.as_str(), "tiny-test_2026.v1");
    assert_eq!(chunk_id.as_str(), "tiny-test-000001");
    assert_eq!(chapter_id.as_str(), "chapter.01");
    assert_eq!(book_id.to_string(), "tiny-test_2026.v1");
}

#[test]
fn ids_reject_path_like_or_ambiguous_values() {
    for raw in [
        "",
        ".",
        "..",
        "../x",
        "book/1",
        "book\\1",
        "two words",
        " leading",
        "trailing ",
        "line\nbreak",
        "tab\tid",
        "nul\0id",
        "snowman-☃",
    ] {
        assert!(BookId::parse(raw).is_err(), "{raw:?} should be invalid");
        assert!(ChunkId::parse(raw).is_err(), "{raw:?} should be invalid");
        assert!(ChapterId::parse(raw).is_err(), "{raw:?} should be invalid");
    }
}

#[test]
fn page_numbers_are_one_based_and_validate_ranges() {
    let first = PageNumber::new(1).unwrap();
    let third = PageNumber::new(3).unwrap();

    assert_eq!(first.get(), 1);
    assert_eq!(third.get(), 3);
    assert!(PageNumber::new(0).is_err());
    assert!(PageNumber::range(first, third).is_ok());
    assert!(PageNumber::range(third, first).is_err());
}

#[test]
fn citation_formats_single_pages_and_page_ranges() {
    let book_id = BookId::parse("tiny-test").unwrap();
    let single = Citation::new(
        book_id.clone(),
        "Tiny Test Book".to_owned(),
        Some("A. Writer".to_owned()),
        PageNumber::new(1).unwrap(),
        PageNumber::new(1).unwrap(),
        Some("Opening".to_owned()),
    )
    .unwrap();

    let range = Citation::new(
        book_id,
        "Tiny Test Book".to_owned(),
        None,
        PageNumber::new(2).unwrap(),
        PageNumber::new(4).unwrap(),
        None,
    )
    .unwrap();

    assert_eq!(
        single.format(),
        "Tiny Test Book by A. Writer, chapter \"Opening\", p. 1"
    );
    assert_eq!(range.format(), "Tiny Test Book, pp. 2-4");
}

#[test]
fn citation_rejects_reversed_page_ranges() {
    let err = Citation::new(
        BookId::parse("tiny-test").unwrap(),
        "Tiny Test Book".to_owned(),
        None,
        PageNumber::new(5).unwrap(),
        PageNumber::new(4).unwrap(),
        None,
    )
    .unwrap_err();

    assert!(matches!(err, BookMcpError::InvalidPageRange { .. }));
}

#[test]
fn io_errors_convert_to_typed_bookmcp_errors() {
    let err: BookMcpError = io::Error::new(io::ErrorKind::NotFound, "missing fixture").into();

    assert!(matches!(err, BookMcpError::Io(_)));
    assert!(err.to_string().contains("I/O error"));
}

#[test]
fn validated_ids_round_trip_through_json() {
    let id = BookId::parse("tiny-test").unwrap();
    let json = serde_json::to_string(&id).unwrap();
    let decoded: BookId = serde_json::from_str(&json).unwrap();

    assert_eq!(decoded, id);

    let invalid = serde_json::from_str::<BookId>("\"../x\"");
    assert!(invalid.is_err());
}

#[test]
fn deserialization_preserves_page_number_invariant() {
    assert!(serde_json::from_str::<PageNumber>("0").is_err());
    assert!(serde_json::from_str::<PageNumber>("-1").is_err());
    assert_eq!(serde_json::from_str::<PageNumber>("1").unwrap().get(), 1);
}

#[test]
fn identifiers_have_a_bounded_length() {
    assert!(BookId::parse("a".repeat(128)).is_ok());
    assert!(BookId::parse("a".repeat(129)).is_err());
}
