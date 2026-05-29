#![forbid(unsafe_code)]

use std::{
    fmt, fs,
    path::{Path, PathBuf},
};

use bookmcp_core::{
    BookId, BookMcpError, BookMetadata, Chapter, ChapterId, Chunk, ChunkId, Citation, IngestReport,
    Page, PageNumber, Result,
};
use bookmcp_store::IngestBatch;
use sha2::{Digest, Sha256};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

const DEFAULT_TARGET_CHARS: usize = 3_000;
const DEFAULT_OVERLAP_CHARS: usize = 400;

/// One page extracted from a PDF before persistence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExtractedPage {
    /// One-based page number from the source PDF.
    pub page_number: PageNumber,
    /// Extracted page text.
    pub text: String,
}

impl ExtractedPage {
    /// Create an extracted page.
    pub fn new(page_number: PageNumber, text: String) -> Self {
        Self { page_number, text }
    }

    /// Convert an extracted page to the shared core page model.
    pub fn to_page(&self, book_id: BookId, title: &str, author: Option<&str>) -> Result<Page> {
        let citation = Citation::new(
            book_id.clone(),
            title.to_owned(),
            author.map(str::to_owned),
            self.page_number,
            self.page_number,
            None,
        )?;

        Ok(Page {
            book_id,
            page_number: self.page_number,
            text: self.text.clone(),
            citation,
        })
    }
}

/// Metadata extracted from a PDF when available.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ExtractedPdfMetadata {
    /// Extracted title.
    pub title: Option<String>,
    /// Extracted author.
    pub author: Option<String>,
}

/// Outline item extracted from PDF bookmarks.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutlineItem {
    /// Outline title.
    pub title: String,
    /// First page for the outline item.
    pub page_number: PageNumber,
}

/// Text and metadata extracted from a PDF.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExtractedPdf {
    /// Extracted pages.
    pub pages: Vec<ExtractedPage>,
    /// Extracted metadata.
    pub metadata: ExtractedPdfMetadata,
    /// Extracted outline/bookmark items.
    pub outline: Vec<OutlineItem>,
}

/// Boundary for PDF text extraction implementations.
pub trait PdfExtractor {
    /// Extract text, metadata, and outline information from a PDF.
    fn extract(&self, path: &Path) -> Result<ExtractedPdf>;
}

/// PDF text extractor backed by the `pdf-extract` crate.
#[derive(Clone, Copy, Debug, Default)]
pub struct PdfTextExtractor;

impl PdfExtractor for PdfTextExtractor {
    fn extract(&self, path: &Path) -> Result<ExtractedPdf> {
        let raw_pages = pdf_extract::extract_text_by_pages(path).map_err(pdf_error)?;
        let pages = raw_pages
            .into_iter()
            .enumerate()
            .map(|(index, text)| {
                Ok(ExtractedPage::new(
                    PageNumber::new(index_to_page_number(index)?)?,
                    text,
                ))
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(ExtractedPdf {
            pages,
            metadata: ExtractedPdfMetadata::default(),
            outline: Vec::new(),
        })
    }
}

/// Chunking configuration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChunkerConfig {
    /// Target chunk size in characters.
    pub target_chars: usize,
    /// Overlap used when splitting long page text.
    pub overlap_chars: usize,
}

impl Default for ChunkerConfig {
    fn default() -> Self {
        Self {
            target_chars: DEFAULT_TARGET_CHARS,
            overlap_chars: DEFAULT_OVERLAP_CHARS,
        }
    }
}

/// Converts extracted pages into citable chunks.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Chunker {
    config: ChunkerConfig,
}

impl Chunker {
    /// Create a chunker with validated configuration.
    pub fn new(config: ChunkerConfig) -> Result<Self> {
        if config.target_chars == 0 {
            return Err(BookMcpError::InvalidLimit {
                value: 0,
                max: usize::MAX,
            });
        }

        if config.overlap_chars >= config.target_chars {
            return Err(BookMcpError::InvalidLimit {
                value: config.overlap_chars,
                max: config.target_chars.saturating_sub(1),
            });
        }

        Ok(Self { config })
    }

    /// Chunk extracted pages with deterministic IDs and citation metadata.
    pub fn chunk(
        &self,
        book_id: &BookId,
        title: &str,
        author: Option<&str>,
        pages: &[ExtractedPage],
        chapters: &[Chapter],
    ) -> Result<Vec<Chunk>> {
        let mut chunks = Vec::new();
        let context = ChunkContext {
            book_id,
            title,
            author,
            chapters,
        };
        let mut buffer = String::new();
        let mut page_start = None;
        let mut page_end = None;

        for page in pages {
            let page_text = page.text.trim();
            if page_text.is_empty() {
                continue;
            }

            for part in split_text(
                page_text,
                self.config.target_chars,
                self.config.overlap_chars,
            ) {
                let separator_len = usize::from(!buffer.is_empty());
                if !buffer.is_empty()
                    && char_count(&buffer) + char_count(&part) + separator_len
                        > self.config.target_chars
                {
                    self.push_chunk(
                        &mut chunks,
                        &context,
                        &buffer,
                        page_start.ok_or_else(|| {
                            BookMcpError::Ingest("missing chunk start page".to_owned())
                        })?,
                        page_end.ok_or_else(|| {
                            BookMcpError::Ingest("missing chunk end page".to_owned())
                        })?,
                    )?;
                    buffer.clear();
                }

                if buffer.is_empty() {
                    page_start = Some(page.page_number);
                } else {
                    buffer.push('\n');
                }

                buffer.push_str(&part);
                page_end = Some(page.page_number);
            }
        }

        if !buffer.trim().is_empty() {
            self.push_chunk(
                &mut chunks,
                &context,
                &buffer,
                page_start.ok_or_else(|| {
                    BookMcpError::Ingest("missing final chunk start page".to_owned())
                })?,
                page_end.ok_or_else(|| {
                    BookMcpError::Ingest("missing final chunk end page".to_owned())
                })?,
            )?;
        }

        Ok(chunks)
    }

    fn push_chunk(
        &self,
        chunks: &mut Vec<Chunk>,
        context: &ChunkContext<'_>,
        text: &str,
        page_start: PageNumber,
        page_end: PageNumber,
    ) -> Result<()> {
        let ordinal = chunks.len() + 1;
        let chunk_id = ChunkId::parse(format!("{}-{ordinal:06}", context.book_id.as_str()))?;
        let chapter = chapter_for_range(context.chapters, page_start, page_end);
        let chapter_id = chapter.map(|chapter| chapter.chapter_id.clone());
        let chapter_title = chapter.map(|chapter| chapter.title.clone());
        let citation = Citation::new(
            context.book_id.clone(),
            context.title.to_owned(),
            context.author.map(str::to_owned),
            page_start,
            page_end,
            chapter_title.clone(),
        )?;

        chunks.push(Chunk {
            chunk_id,
            book_id: context.book_id.clone(),
            chapter_id,
            chapter_title,
            page_start,
            page_end,
            text: text.trim().to_owned(),
            citation,
        });

        Ok(())
    }
}

struct ChunkContext<'a> {
    book_id: &'a BookId,
    title: &'a str,
    author: Option<&'a str>,
    chapters: &'a [Chapter],
}

/// Conservative chapter detector.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ChapterDetector;

impl ChapterDetector {
    /// Detect chapters from outlines first, then conservative headings.
    pub fn detect(
        &self,
        book_id: &BookId,
        pages: &[ExtractedPage],
        outline: &[OutlineItem],
    ) -> Result<Vec<Chapter>> {
        if !outline.is_empty() {
            return chapters_from_outline(book_id, pages, outline);
        }

        chapters_from_headings(book_id, pages)
    }
}

/// Options passed into one ingest run.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IngestOptions {
    /// User-provided PDF path.
    pub pdf_path: PathBuf,
    /// Optional title override.
    pub title: Option<String>,
    /// Optional author override.
    pub author: Option<String>,
    /// Optional validated book ID override.
    pub book_id: Option<BookId>,
}

/// Output from ingestion before persistence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IngestOutput {
    /// Records ready to save transactionally.
    pub batch: IngestBatch,
    /// Human-facing ingest summary.
    pub report: IngestReport,
    /// Canonical source PDF path.
    pub source_path: PathBuf,
}

/// Coordinates PDF extraction, normalization, chapter detection, and chunking.
#[derive(Clone, Debug)]
pub struct IngestPipeline<E> {
    extractor: E,
    chapter_detector: ChapterDetector,
    chunker: Chunker,
}

impl<E> IngestPipeline<E>
where
    E: PdfExtractor,
{
    /// Create a pipeline with default chapter detection and chunking.
    pub fn new(extractor: E) -> Self {
        Self {
            extractor,
            chapter_detector: ChapterDetector,
            chunker: Chunker::default(),
        }
    }

    /// Create a pipeline with explicit components.
    pub fn with_components(
        extractor: E,
        chapter_detector: ChapterDetector,
        chunker: Chunker,
    ) -> Self {
        Self {
            extractor,
            chapter_detector,
            chunker,
        }
    }

    /// Run ingestion and return records ready for storage/indexing.
    pub fn ingest(&self, options: IngestOptions) -> Result<IngestOutput> {
        let source_path = canonical_pdf_path(&options.pdf_path)?;
        let source_bytes = fs::read(&source_path)?;
        if !looks_like_pdf(&source_path, &source_bytes) {
            return Err(BookMcpError::Pdf(format!(
                "{} does not look like a PDF",
                source_path.display()
            )));
        }

        let source_sha256 = sha256_hex(&source_bytes);
        let extracted = self.extractor.extract(&source_path)?;
        let pages = normalize_pages(extracted.pages);
        detect_extractable_text(&pages)?;

        let title = select_title(options.title, extracted.metadata.title, &source_path);
        let author = options.author.or(extracted.metadata.author);
        let book_id = match options.book_id {
            Some(book_id) => book_id,
            None => BookId::parse(slug_for_title(&title, &source_sha256))?,
        };

        let chapters = self
            .chapter_detector
            .detect(&book_id, &pages, &extracted.outline)?;
        let core_pages = pages
            .iter()
            .map(|page| page.to_page(book_id.clone(), &title, author.as_deref()))
            .collect::<Result<Vec<_>>>()?;
        let chunks = self
            .chunker
            .chunk(&book_id, &title, author.as_deref(), &pages, &chapters)?;
        let ingested_at = OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .map_err(|error| BookMcpError::Ingest(error.to_string()))?;
        let page_count = usize_to_u32(pages.len(), "page count")?;
        let chapter_count = usize_to_u32(chapters.len(), "chapter count")?;
        let chunk_count = usize_to_u32(chunks.len(), "chunk count")?;

        let metadata = BookMetadata {
            book_id: book_id.clone(),
            title: title.clone(),
            author,
            source_sha256: source_sha256.clone(),
            page_count,
            chapter_count,
            chunk_count,
            ingested_at,
        };
        let report = IngestReport {
            book_id,
            title,
            source_sha256,
            page_count,
            chunk_count,
            indexed_chunks: 0,
        };
        let batch = IngestBatch {
            metadata,
            pages: core_pages,
            chapters,
            chunks,
        };

        Ok(IngestOutput {
            batch,
            report,
            source_path,
        })
    }
}

fn canonical_pdf_path(path: &Path) -> Result<PathBuf> {
    let canonical = fs::canonicalize(path)?;
    let metadata = fs::metadata(&canonical)?;
    if !metadata.is_file() {
        return Err(BookMcpError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("{} is not a file", canonical.display()),
        )));
    }

    Ok(canonical)
}

fn looks_like_pdf(path: &Path, bytes: &[u8]) -> bool {
    let extension_matches = path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("pdf"));
    extension_matches || bytes.starts_with(b"%PDF-")
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn normalize_pages(pages: Vec<ExtractedPage>) -> Vec<ExtractedPage> {
    pages
        .into_iter()
        .map(|page| ExtractedPage {
            page_number: page.page_number,
            text: normalize_page_text(&page.text),
        })
        .collect()
}

fn normalize_page_text(text: &str) -> String {
    let mut normalized = Vec::new();
    let mut previous_blank = false;

    for raw_line in text.lines() {
        let line = normalize_line(raw_line);
        if line.is_empty() {
            if !previous_blank && !normalized.is_empty() {
                normalized.push(String::new());
            }
            previous_blank = true;
        } else {
            normalized.push(line);
            previous_blank = false;
        }
    }

    while normalized.last().is_some_and(String::is_empty) {
        normalized.pop();
    }

    normalized.join("\n")
}

fn normalize_line(line: &str) -> String {
    line.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn detect_extractable_text(pages: &[ExtractedPage]) -> Result<()> {
    if pages.is_empty() {
        return Err(BookMcpError::NoExtractableText);
    }

    let non_empty_pages = pages
        .iter()
        .filter(|page| !page.text.trim().is_empty())
        .count();

    if non_empty_pages == 0 {
        return Err(BookMcpError::NoExtractableText);
    }

    let empty_pages = pages.len() - non_empty_pages;
    if pages.len() >= 3 && empty_pages * 100 / pages.len() >= 60 {
        return Err(BookMcpError::OcrRequired {
            reason: format!(
                "{empty_pages} of {} pages had no extractable text",
                pages.len()
            ),
        });
    }

    Ok(())
}

fn select_title(
    override_title: Option<String>,
    extracted_title: Option<String>,
    path: &Path,
) -> String {
    override_title
        .filter(|title| !title.trim().is_empty())
        .or_else(|| extracted_title.filter(|title| !title.trim().is_empty()))
        .unwrap_or_else(|| filename_title(path))
}

fn filename_title(path: &Path) -> String {
    path.file_stem()
        .map(|stem| stem.to_string_lossy().trim().to_owned())
        .filter(|title| !title.is_empty())
        .unwrap_or_else(|| "Untitled Book".to_owned())
}

fn slug_for_title(title: &str, source_sha256: &str) -> String {
    let mut slug = String::new();
    let mut last_was_dash = false;

    for byte in title.bytes() {
        let next = if byte.is_ascii_alphanumeric() {
            Some(byte.to_ascii_lowercase() as char)
        } else if matches!(byte, b' ' | b'\t' | b'\n' | b'\r' | b'-' | b'_') {
            Some('-')
        } else {
            None
        };

        if let Some(ch) = next {
            if ch == '-' {
                if !last_was_dash && !slug.is_empty() {
                    slug.push(ch);
                }
                last_was_dash = true;
            } else {
                slug.push(ch);
                last_was_dash = false;
            }
        }
    }

    while slug.ends_with('-') {
        slug.pop();
    }

    if slug.is_empty() {
        let suffix = source_sha256
            .get(..12)
            .map(str::to_owned)
            .unwrap_or_else(|| "unknown".to_owned());
        format!("book-{suffix}")
    } else {
        slug
    }
}

fn chapters_from_outline(
    book_id: &BookId,
    pages: &[ExtractedPage],
    outline: &[OutlineItem],
) -> Result<Vec<Chapter>> {
    let mut outline = outline.to_vec();
    outline.sort_by_key(|item| item.page_number);
    let last_page = pages
        .last()
        .map(|page| page.page_number)
        .or_else(|| outline.last().map(|item| item.page_number));
    let Some(last_page) = last_page else {
        return Ok(Vec::new());
    };

    let mut chapters = Vec::new();
    for (index, item) in outline.iter().enumerate() {
        let next_start = outline.get(index + 1).map(|next| next.page_number);
        let page_end = next_start
            .and_then(|next| next.get().checked_sub(1))
            .map(PageNumber::new)
            .transpose()?
            .unwrap_or(last_page);

        if item.page_number <= page_end {
            chapters.push(Chapter {
                chapter_id: chapter_id_from_title(&item.title, index + 1)?,
                book_id: book_id.clone(),
                title: item.title.clone(),
                page_start: item.page_number,
                page_end,
            });
        }
    }

    Ok(chapters)
}

fn chapters_from_headings(book_id: &BookId, pages: &[ExtractedPage]) -> Result<Vec<Chapter>> {
    let mut starts = Vec::new();

    for page in pages {
        let Some(first_line) = page.text.lines().find(|line| !line.trim().is_empty()) else {
            continue;
        };
        let candidate = first_line.trim();
        if looks_like_chapter_heading(candidate) {
            starts.push((candidate.to_owned(), page.page_number));
        }
    }

    let mut chapters = Vec::new();
    for (index, (title, page_start)) in starts.iter().enumerate() {
        let page_end = starts
            .get(index + 1)
            .and_then(|(_, next_start)| next_start.get().checked_sub(1))
            .map(PageNumber::new)
            .transpose()?
            .or_else(|| pages.last().map(|page| page.page_number))
            .unwrap_or(*page_start);

        if *page_start <= page_end {
            chapters.push(Chapter {
                chapter_id: chapter_id_from_title(title, index + 1)?,
                book_id: book_id.clone(),
                title: title.clone(),
                page_start: *page_start,
                page_end,
            });
        }
    }

    Ok(chapters)
}

fn looks_like_chapter_heading(line: &str) -> bool {
    if line.len() > 96 {
        return false;
    }

    let lower = line.to_ascii_lowercase();
    let Some(rest) = lower.strip_prefix("chapter ") else {
        return false;
    };

    rest.chars().next().is_some_and(|ch| ch.is_ascii_digit())
}

fn chapter_id_from_title(title: &str, ordinal: usize) -> Result<ChapterId> {
    let slug = slug_for_title(title, "chapter");
    if slug.is_empty() {
        ChapterId::parse(format!("chapter-{ordinal:06}"))
    } else {
        ChapterId::parse(slug)
    }
}

fn chapter_for_range(
    chapters: &[Chapter],
    page_start: PageNumber,
    page_end: PageNumber,
) -> Option<&Chapter> {
    chapters
        .iter()
        .find(|chapter| chapter.page_start <= page_end && chapter.page_end >= page_start)
}

fn split_text(text: &str, target_chars: usize, overlap_chars: usize) -> Vec<String> {
    let total_chars = char_count(text);
    if total_chars <= target_chars {
        return vec![text.to_owned()];
    }

    let step = target_chars.saturating_sub(overlap_chars).max(1);
    let mut parts = Vec::new();
    let mut start = 0;

    while start < total_chars {
        let end = (start + target_chars).min(total_chars);
        let part = slice_chars(text, start, end).trim().to_owned();
        if !part.is_empty() {
            parts.push(part);
        }
        if end == total_chars {
            break;
        }
        start = start.saturating_add(step);
    }

    parts
}

fn slice_chars(text: &str, start: usize, end: usize) -> String {
    text.chars().skip(start).take(end - start).collect()
}

fn char_count(text: &str) -> usize {
    text.chars().count()
}

fn index_to_page_number(index: usize) -> Result<u32> {
    let one_based = index
        .checked_add(1)
        .ok_or_else(|| BookMcpError::Ingest("page index overflow".to_owned()))?;
    usize_to_u32(one_based, "page number")
}

fn usize_to_u32(value: usize, field: &'static str) -> Result<u32> {
    u32::try_from(value)
        .map_err(|_| BookMcpError::Ingest(format!("{field} {value} is outside u32 range")))
}

fn pdf_error(error: impl fmt::Display) -> BookMcpError {
    let message = error.to_string();
    let lower = message.to_ascii_lowercase();

    if lower.contains("encrypt") || lower.contains("password") {
        BookMcpError::PdfEncrypted
    } else {
        BookMcpError::Pdf(message)
    }
}
