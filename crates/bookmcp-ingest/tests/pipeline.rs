use std::{fs, path::Path};

use bookmcp_core::{BookId, BookMcpError, PageNumber};
use bookmcp_ingest::{
    ExtractedPage, ExtractedPdf, ExtractedPdfMetadata, IngestOptions, IngestPipeline, PdfExtractor,
    PdfTextExtractor,
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
