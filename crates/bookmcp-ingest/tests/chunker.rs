use bookmcp_core::{BookId, Page, PageNumber};
use bookmcp_ingest::{ChapterDetector, Chunker, ChunkerConfig, ExtractedPage, OutlineItem};

#[test]
fn chunker_handles_short_text_as_one_chunk() {
    let chunker = Chunker::new(ChunkerConfig {
        target_chars: 2_000,
        overlap_chars: 300,
    })
    .unwrap();
    let pages = vec![ExtractedPage::new(
        PageNumber::new(1).unwrap(),
        "A compact page about test-driven systems.".to_owned(),
    )];

    let chunks = chunker
        .chunk(
            &BookId::parse("tiny-test").unwrap(),
            "Tiny Test Book",
            Some("A. Writer"),
            &pages,
            &[],
        )
        .unwrap();

    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].chunk_id.as_str(), "tiny-test-000001");
    assert_eq!(chunks[0].page_start.get(), 1);
    assert_eq!(chunks[0].page_end.get(), 1);
    assert!(chunks[0].text.contains("test-driven systems"));
}

#[test]
fn chunker_splits_long_paragraphs_and_never_emits_empty_chunks() {
    let chunker = Chunker::new(ChunkerConfig {
        target_chars: 80,
        overlap_chars: 20,
    })
    .unwrap();
    let long_text = "alpha beta gamma delta epsilon zeta eta theta iota kappa lambda ".repeat(5);
    let pages = vec![ExtractedPage::new(PageNumber::new(1).unwrap(), long_text)];

    let chunks = chunker
        .chunk(
            &BookId::parse("tiny-test").unwrap(),
            "Tiny Test Book",
            None,
            &pages,
            &[],
        )
        .unwrap();

    assert!(chunks.len() > 1);
    assert!(chunks.iter().all(|chunk| !chunk.text.trim().is_empty()));
    assert_eq!(chunks[0].chunk_id.as_str(), "tiny-test-000001");
    assert_eq!(chunks[1].chunk_id.as_str(), "tiny-test-000002");
}

#[test]
fn chunker_tracks_multi_page_ranges_and_skips_empty_pages() {
    let chunker = Chunker::new(ChunkerConfig {
        target_chars: 120,
        overlap_chars: 0,
    })
    .unwrap();
    let pages = vec![
        ExtractedPage::new(PageNumber::new(1).unwrap(), "   ".to_owned()),
        ExtractedPage::new(
            PageNumber::new(2).unwrap(),
            "Page two introduces a test concept.".to_owned(),
        ),
        ExtractedPage::new(
            PageNumber::new(3).unwrap(),
            "Page three extends the same concept with an example.".to_owned(),
        ),
    ];

    let chunks = chunker
        .chunk(
            &BookId::parse("tiny-test").unwrap(),
            "Tiny Test Book",
            None,
            &pages,
            &[],
        )
        .unwrap();

    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].page_start.get(), 2);
    assert_eq!(chunks[0].page_end.get(), 3);
}

#[test]
fn chunk_ids_are_deterministic_for_same_input() {
    let chunker = Chunker::default();
    let pages = vec![ExtractedPage::new(
        PageNumber::new(1).unwrap(),
        "Deterministic text for repeatable chunk IDs.".to_owned(),
    )];
    let book_id = BookId::parse("tiny-test").unwrap();

    let first = chunker
        .chunk(&book_id, "Tiny Test Book", None, &pages, &[])
        .unwrap();
    let second = chunker
        .chunk(&book_id, "Tiny Test Book", None, &pages, &[])
        .unwrap();

    assert_eq!(
        first
            .iter()
            .map(|chunk| chunk.chunk_id.clone())
            .collect::<Vec<_>>(),
        second
            .iter()
            .map(|chunk| chunk.chunk_id.clone())
            .collect::<Vec<_>>()
    );
}

#[test]
fn chapter_detector_does_not_fabricate_chapters_from_body_text() {
    let pages = vec![
        ExtractedPage::new(
            PageNumber::new(1).unwrap(),
            "This page has a sentence about a chapter in life, but not a heading.".to_owned(),
        ),
        ExtractedPage::new(
            PageNumber::new(2).unwrap(),
            "Another ordinary paragraph without section structure.".to_owned(),
        ),
    ];

    let chapters = ChapterDetector
        .detect(&BookId::parse("tiny-test").unwrap(), &pages, &[])
        .unwrap();

    assert!(chapters.is_empty());
}

#[test]
fn chapter_detector_uses_conservative_chapter_headings() {
    let pages = vec![
        ExtractedPage::new(
            PageNumber::new(1).unwrap(),
            "Chapter 1\nFoundations\nThe body starts here.".to_owned(),
        ),
        ExtractedPage::new(
            PageNumber::new(2).unwrap(),
            "Chapter 2: Practice\nMore body text.".to_owned(),
        ),
    ];

    let chapters = ChapterDetector
        .detect(&BookId::parse("tiny-test").unwrap(), &pages, &[])
        .unwrap();

    assert_eq!(chapters.len(), 2);
    assert_eq!(chapters[0].title, "Chapter 1");
    assert_eq!(chapters[0].page_start.get(), 1);
    assert_eq!(chapters[0].page_end.get(), 1);
    assert_eq!(chapters[1].title, "Chapter 2: Practice");
    assert_eq!(chapters[1].page_start.get(), 2);
    assert_eq!(chapters[1].page_end.get(), 2);
}

#[test]
fn chapter_detector_uses_outline_before_heading_heuristics() {
    let pages = vec![
        ExtractedPage::new(
            PageNumber::new(1).unwrap(),
            "Ordinary preface text without a heading.".to_owned(),
        ),
        ExtractedPage::new(
            PageNumber::new(2).unwrap(),
            "Chapter 99\nThis heading should not win over the outline.".to_owned(),
        ),
    ];
    let outline = vec![
        OutlineItem {
            title: "Opening".to_owned(),
            page_number: PageNumber::new(1).unwrap(),
        },
        OutlineItem {
            title: "Practice".to_owned(),
            page_number: PageNumber::new(2).unwrap(),
        },
    ];

    let chapters = ChapterDetector
        .detect(&BookId::parse("tiny-test").unwrap(), &pages, &outline)
        .unwrap();

    assert_eq!(chapters.len(), 2);
    assert_eq!(chapters[0].title, "Opening");
    assert_eq!(chapters[0].page_start.get(), 1);
    assert_eq!(chapters[0].page_end.get(), 1);
    assert_eq!(chapters[1].title, "Practice");
}

#[test]
fn extracted_page_converts_to_core_page_with_citation() {
    let page = ExtractedPage::new(
        PageNumber::new(1).unwrap(),
        "A page that can become a core model.".to_owned(),
    );

    let core_page: Page = page
        .to_page(
            BookId::parse("tiny-test").unwrap(),
            "Tiny Test Book",
            Some("A. Writer"),
        )
        .unwrap();

    assert_eq!(core_page.page_number.get(), 1);
    assert_eq!(
        core_page.citation.format(),
        "Tiny Test Book by A. Writer, p. 1"
    );
}
