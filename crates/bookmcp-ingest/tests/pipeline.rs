use std::{fs, path::Path};

use bookmcp_core::{BookId, BookMcpError, PageNumber};
use bookmcp_ingest::{
    ExtractedPage, ExtractedPdf, ExtractedPdfMetadata, IngestOptions, IngestPipeline, PdfExtractor,
    PdfTextExtractor,
};
use lopdf::{
    Document, EncryptionState, EncryptionVersion, Object, Permissions, Stream, dictionary,
};
use tempfile::tempdir;

#[test]
fn pipeline_builds_ingest_batch_from_text_pages() {
    let temp = tempdir().unwrap();
    let pdf_path = temp.path().join("tiny.pdf");
    fs::write(&pdf_path, b"%PDF-1.7\n% tiny fixture shell\n").unwrap();
    let extractor = FakeExtractor {
        pages: vec![
            "Chapter 1\nA tiny test page.".to_owned(),
            "Another page with a test example.".to_owned(),
        ],
        metadata: ExtractedPdfMetadata {
            title: Some("Extractor Title".to_owned()),
            author: Some("Extractor Author".to_owned()),
        },
    };

    let output = IngestPipeline::new(extractor)
        .ingest(IngestOptions {
            pdf_path,
            title: Some("Override Title".to_owned()),
            author: None,
            book_id: Some(BookId::parse("tiny-test").unwrap()),
        })
        .unwrap();

    assert_eq!(output.report.book_id.as_str(), "tiny-test");
    assert_eq!(output.report.title, "Override Title");
    assert_eq!(
        output.batch.metadata.author,
        Some("Extractor Author".to_owned())
    );
    assert_eq!(output.batch.pages.len(), 2);
    assert!(!output.batch.chunks.is_empty());
    assert_eq!(output.batch.chapters[0].title, "Chapter 1");
    assert_eq!(
        output.batch.pages[0].page_number,
        PageNumber::new(1).unwrap()
    );
}

#[test]
fn pipeline_falls_back_to_filename_title_and_slug_book_id() {
    let temp = tempdir().unwrap();
    let pdf_path = temp.path().join("My Tiny Book.pdf");
    fs::write(&pdf_path, b"%PDF-1.7\n% tiny fixture shell\n").unwrap();

    let output = IngestPipeline::new(FakeExtractor::with_pages(vec![
        "This text makes the fake PDF extractable.".to_owned(),
    ]))
    .ingest(IngestOptions {
        pdf_path,
        title: None,
        author: None,
        book_id: None,
    })
    .unwrap();

    assert_eq!(output.batch.metadata.title, "My Tiny Book");
    assert_eq!(output.report.book_id.as_str(), "my-tiny-book");
}

#[test]
fn pipeline_rejects_missing_pdf_paths() {
    let temp = tempdir().unwrap();
    let pdf_path = temp.path().join("missing.pdf");

    let err = IngestPipeline::new(FakeExtractor::with_pages(vec![]))
        .ingest(IngestOptions {
            pdf_path,
            title: None,
            author: None,
            book_id: None,
        })
        .unwrap_err();

    assert!(matches!(err, BookMcpError::Io(_)));
}

#[test]
fn pipeline_rejects_files_that_do_not_look_like_pdfs() {
    let temp = tempdir().unwrap();
    let pdf_path = temp.path().join("not-a-pdf.txt");
    fs::write(&pdf_path, b"plain text").unwrap();

    let err = IngestPipeline::new(FakeExtractor::with_pages(vec!["text".to_owned()]))
        .ingest(IngestOptions {
            pdf_path,
            title: None,
            author: None,
            book_id: None,
        })
        .unwrap_err();

    assert!(matches!(err, BookMcpError::Pdf(_)));
}

#[test]
fn pipeline_detects_empty_or_scanned_like_extraction() {
    let temp = tempdir().unwrap();
    let pdf_path = temp.path().join("scanned.pdf");
    fs::write(&pdf_path, b"%PDF-1.7\n% tiny fixture shell\n").unwrap();

    let err = IngestPipeline::new(FakeExtractor::with_pages(vec![
        String::new(),
        " ".to_owned(),
        "\n".to_owned(),
    ]))
    .ingest(IngestOptions {
        pdf_path,
        title: None,
        author: None,
        book_id: None,
    })
    .unwrap_err();

    assert!(matches!(err, BookMcpError::NoExtractableText));
}

#[test]
fn pdf_text_extractor_reads_tiny_fixture_pdf() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/tiny.pdf")
        .canonicalize()
        .unwrap();

    let extracted = PdfTextExtractor.extract(&fixture).unwrap();

    assert_eq!(extracted.pages.len(), 1);
    assert!(extracted.pages[0].text.contains("Tiny Test Book"));
    assert!(extracted.pages[0].text.contains("test concepts"));
    assert_eq!(extracted.metadata.title, Some("Tiny Test Book".to_owned()));
    assert_eq!(extracted.metadata.author, Some("BookMCP Tests".to_owned()));
}

#[test]
fn encrypted_pdfs_are_rejected_even_when_the_user_password_is_empty() {
    for user_password in ["", "secret"] {
        let temp = tempdir().unwrap();
        let encrypted_path = temp.path().join("encrypted.pdf");
        let mut document = fixture_document();
        document.trailer.set(
            "ID",
            vec![
                Object::string_literal("bookmcp-test"),
                Object::string_literal("bookmcp-test"),
            ],
        );
        let state = EncryptionState::try_from(EncryptionVersion::V2 {
            document: &document,
            owner_password: "owner-secret",
            user_password,
            key_length: 128,
            permissions: Permissions::PRINTABLE,
        })
        .unwrap();
        document.encrypt(&state).unwrap();
        document.save(&encrypted_path).unwrap();

        assert!(matches!(
            PdfTextExtractor.extract(&encrypted_path),
            Err(BookMcpError::PdfEncrypted)
        ));
    }
}

#[test]
fn extraction_reports_a_bad_later_page_instead_of_returning_partial_success() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("bad-second-page.pdf");
    let mut document = fixture_document();
    let first_page = *document.get_pages().get(&1).unwrap();
    let parent = document
        .get_dictionary(first_page)
        .unwrap()
        .get(b"Parent")
        .unwrap()
        .as_reference()
        .unwrap();
    let content_id = document.add_object(Stream::new(
        dictionary! {},
        b"BT /MissingFont 12 Tf (unsupported) Tj ET".to_vec(),
    ));
    let second_page = document.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => parent,
        "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
        "Resources" => dictionary! {},
        "Contents" => content_id,
    });
    let page_tree = document.get_dictionary_mut(parent).unwrap();
    page_tree.set("Kids", vec![first_page.into(), second_page.into()]);
    page_tree.set("Count", 2);
    document.save(&path).unwrap();

    assert!(matches!(
        PdfTextExtractor.extract(&path),
        Err(BookMcpError::Pdf(_))
    ));
}

#[test]
fn real_blank_pdf_requires_extractable_text() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("blank.pdf");
    let mut document = fixture_document();
    let page_id = *document.get_pages().get(&1).unwrap();
    document
        .get_dictionary_mut(page_id)
        .unwrap()
        .remove(b"Contents");
    document.save(&path).unwrap();
    let result = IngestPipeline::new(PdfTextExtractor).ingest(IngestOptions {
        pdf_path: path,
        title: None,
        author: None,
        book_id: None,
    });
    assert!(matches!(result, Err(BookMcpError::NoExtractableText)));
}

#[test]
fn production_extractor_uses_snapshot_without_reopening_path() {
    let bytes =
        fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/tiny.pdf"))
            .unwrap();
    let extracted = PdfTextExtractor
        .extract_bytes(Path::new("/does/not/exist.pdf"), &bytes)
        .unwrap();
    assert!(extracted.pages[0].text.contains("Tiny Test Book"));
}

#[test]
fn pipeline_rejects_a_majority_of_empty_pages() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("mostly-scanned.pdf");
    fs::write(&path, b"%PDF-1.7\n").unwrap();
    let result = IngestPipeline::new(FakeExtractor::with_pages(vec![
        "Cover text".to_owned(),
        "Preface text".to_owned(),
        "Index text".to_owned(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
    ]))
    .ingest(IngestOptions {
        pdf_path: path,
        title: None,
        author: None,
        book_id: None,
    });
    assert!(matches!(result, Err(BookMcpError::OcrRequired { .. })));
}

fn fixture_document() -> Document {
    Document::load(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/tiny.pdf"))
        .unwrap()
}

#[test]
fn real_outline_keeps_repeated_titles_and_resolves_named_destinations() {
    let mut document = fixture_document();
    let page = *document.get_pages().get(&1).unwrap();
    let destination = Object::Array(vec![page.into(), Object::Name(b"Fit".to_vec())]);
    let second = document.add_object(dictionary! {
        "Title" => Object::string_literal("Repeated title"),
        "Dest" => Object::string_literal("opening"),
    });
    let first = document.add_object(dictionary! {
        "Title" => Object::string_literal("Repeated title"),
        "Dest" => destination.clone(),
        "Next" => second,
    });
    let outlines = document.add_object(dictionary! { "First" => first, "Last" => second });
    let names = document.add_object(dictionary! {
        "Names" => vec![Object::string_literal("opening"), destination],
    });
    let catalog = document.catalog_mut().unwrap();
    catalog.set("Outlines", outlines);
    catalog.set("Names", dictionary! { "Dests" => names });
    let mut bytes = Vec::new();
    document.save_to(&mut bytes).unwrap();

    let extracted = PdfTextExtractor
        .extract_bytes(Path::new("fixture.pdf"), &bytes)
        .unwrap();
    assert_eq!(extracted.outline.len(), 2);
    assert!(
        extracted
            .outline
            .iter()
            .all(|item| item.title == "Repeated title" && item.page_number.get() == 1)
    );
}

#[test]
fn cyclic_outline_is_rejected_without_hanging() {
    let mut document = fixture_document();
    let page = *document.get_pages().get(&1).unwrap();
    let first = document.add_object(dictionary! {
        "Title" => Object::string_literal("Cycle"),
        "Dest" => vec![page.into(), Object::Name(b"Fit".to_vec())],
    });
    document
        .get_dictionary_mut(first)
        .unwrap()
        .set("Next", first);
    let outlines = document.add_object(dictionary! { "First" => first });
    document.catalog_mut().unwrap().set("Outlines", outlines);
    let mut bytes = Vec::new();
    document.save_to(&mut bytes).unwrap();

    let error = PdfTextExtractor
        .extract_bytes(Path::new("fixture.pdf"), &bytes)
        .unwrap_err();
    assert!(matches!(error, BookMcpError::Pdf(message) if message.contains("cycle")));
}

#[derive(Clone)]
struct FakeExtractor {
    pages: Vec<String>,
    metadata: ExtractedPdfMetadata,
}

impl FakeExtractor {
    fn with_pages(pages: Vec<String>) -> Self {
        Self {
            pages,
            metadata: ExtractedPdfMetadata::default(),
        }
    }
}

impl PdfExtractor for FakeExtractor {
    fn extract(&self, _path: &Path) -> bookmcp_core::Result<ExtractedPdf> {
        Ok(ExtractedPdf {
            pages: self
                .pages
                .iter()
                .enumerate()
                .map(|(index, text)| {
                    ExtractedPage::new(PageNumber::new((index + 1) as u32).unwrap(), text.clone())
                })
                .collect(),
            metadata: self.metadata.clone(),
            outline: Vec::new(),
        })
    }
}
