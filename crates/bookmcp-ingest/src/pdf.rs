use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    panic::catch_unwind,
    path::Path,
};

use bookmcp_core::{BookMcpError, PageNumber, Result};
use lopdf::{Dictionary, Document, Object, decode_text_string};

use crate::{ExtractedPage, ExtractedPdf, ExtractedPdfMetadata, OutlineItem, PdfExtractor};

/// PDF text extractor backed by the `pdf-extract` crate.
#[derive(Clone, Copy, Debug, Default)]
pub struct PdfTextExtractor;

impl PdfExtractor for PdfTextExtractor {
    fn extract(&self, path: &Path) -> Result<ExtractedPdf> {
        let bytes = fs::read(path)?;
        self.extract_bytes(path, &bytes)
    }

    fn extract_bytes(&self, _path: &Path, bytes: &[u8]) -> Result<ExtractedPdf> {
        // The dependency can panic for malformed fonts or content streams. Never
        // allow that to become a partially successful ingest or an application crash.
        catch_unwind(|| extract_document(bytes)).map_err(|_| {
            BookMcpError::Pdf("PDF parser failed on malformed or unsupported content".to_owned())
        })?
    }
}

fn extract_document(bytes: &[u8]) -> Result<ExtractedPdf> {
    let document = Document::load_mem(bytes).map_err(pdf_error)?;
    // lopdf may authenticate empty-password documents while loading. Preserve
    // the boundary even for these PDFs, and never pass them to text extraction.
    if document.trailer.get(b"Encrypt").is_ok() || document.encryption_state.is_some() {
        return Err(BookMcpError::PdfEncrypted);
    }
    let mut pages = Vec::new();
    for page_number in document.get_pages().keys().copied() {
        let mut text = String::new();
        let mut output = pdf_extract::PlainTextOutput::new(&mut text);
        pdf_extract::output_doc_page(&document, &mut output, page_number)
            .map_err(|error| BookMcpError::Pdf(format!("page {page_number}: {error}")))?;
        pages.push(ExtractedPage::new(PageNumber::new(page_number)?, text));
    }

    Ok(ExtractedPdf {
        pages,
        metadata: extract_pdf_metadata(&document),
        outline: extract_outline_items(&document)?,
    })
}

fn extract_pdf_metadata(document: &Document) -> ExtractedPdfMetadata {
    let Some(info) = info_dictionary(document) else {
        return ExtractedPdfMetadata::default();
    };

    ExtractedPdfMetadata {
        title: metadata_text(info, b"Title"),
        author: metadata_text(info, b"Author"),
    }
}

fn info_dictionary(document: &Document) -> Option<&Dictionary> {
    document
        .trailer
        .get(b"Info")
        .ok()
        .and_then(|object| document.dereference(object).ok())
        .and_then(|(_, object)| object.as_dict().ok())
}

fn metadata_text(info: &Dictionary, key: &[u8]) -> Option<String> {
    info.get(key)
        .ok()
        .and_then(|object| decode_text_string(object).ok())
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn extract_outline_items(document: &Document) -> Result<Vec<OutlineItem>> {
    let catalog = document.catalog().map_err(pdf_error)?;
    let Ok(outlines) = document.get_dict_in_dict(catalog, b"Outlines") else {
        return Ok(Vec::new());
    };
    let named = named_destinations(document, catalog)?;
    let page_numbers = document
        .get_pages()
        .into_iter()
        .map(|(number, id)| (id, number))
        .collect::<BTreeMap<_, _>>();
    let mut next = outlines.get(b"First").ok();
    let mut visited = BTreeSet::new();
    let mut items = Vec::new();
    let mut count = 0;
    while let Some(object) = next {
        count += 1;
        if count > MAX_OUTLINE_NODES || object.as_reference().is_ok_and(|id| !visited.insert(id)) {
            return Err(BookMcpError::Pdf(
                "PDF outline contains a cycle or too many entries".to_owned(),
            ));
        }
        let (_, object) = document.dereference(object).map_err(pdf_error)?;
        let node = object.as_dict().map_err(pdf_error)?;
        let title = node
            .get(b"Title")
            .ok()
            .and_then(|title| document.dereference(title).ok())
            .and_then(|(_, title)| decode_text_string(title).ok());
        let destination = node.get(b"Dest").ok().or_else(|| {
            let action = document.get_dict_in_dict(node, b"A").ok()?;
            if action.get(b"S").ok()?.as_name().ok()? != b"GoTo" {
                return None;
            }
            action.get(b"D").ok()
        });
        if let (Some(title), Some(destination)) = (title, destination)
            && let Some(number) = destination_page(document, destination, &named, &page_numbers, 0)
            && !title.trim().is_empty()
        {
            items.push(OutlineItem {
                title: title.trim().to_owned(),
                page_number: PageNumber::new(number)?,
            });
        }
        // Only top-level bookmarks form chapters. Nested sections need no traversal.
        next = node.get(b"Next").ok();
    }
    Ok(items)
}

const MAX_OUTLINE_NODES: usize = 10_000;

fn named_destinations(
    document: &Document,
    catalog: &Dictionary,
) -> Result<BTreeMap<Vec<u8>, Object>> {
    let mut destinations = BTreeMap::new();
    if let Ok(legacy) = document.get_dict_in_dict(catalog, b"Dests") {
        for (name, destination) in legacy.iter() {
            destinations.insert(name.clone(), destination.clone());
        }
    }
    let mut pending = Vec::new();
    if let Ok(names) = document.get_dict_in_dict(catalog, b"Names")
        && let Ok(dests) = names.get(b"Dests")
    {
        pending.push(dests);
    }
    let mut visited = BTreeSet::new();
    let mut count = 0;
    while let Some(object) = pending.pop() {
        count += 1;
        if count > MAX_OUTLINE_NODES || object.as_reference().is_ok_and(|id| !visited.insert(id)) {
            return Err(BookMcpError::Pdf(
                "PDF destination tree contains a cycle or too many entries".to_owned(),
            ));
        }
        let (_, object) = document.dereference(object).map_err(pdf_error)?;
        let node = object.as_dict().map_err(pdf_error)?;
        if let Ok(names) = node.get(b"Names").and_then(Object::as_array) {
            for pair in names.chunks_exact(2) {
                if let Ok(name) = pair[0].as_str() {
                    destinations.insert(name.to_vec(), pair[1].clone());
                }
            }
        }
        if let Ok(kids) = node.get(b"Kids").and_then(Object::as_array) {
            pending.extend(kids.iter());
        }
    }
    Ok(destinations)
}

fn destination_page(
    document: &Document,
    destination: &Object,
    named: &BTreeMap<Vec<u8>, Object>,
    pages: &BTreeMap<lopdf::ObjectId, u32>,
    depth: usize,
) -> Option<u32> {
    if depth >= 16 {
        return None;
    }
    let (_, destination) = document.dereference(destination).ok()?;
    match destination {
        Object::Array(values) => pages.get(&values.first()?.as_reference().ok()?).copied(),
        Object::Dictionary(dictionary) => destination_page(
            document,
            dictionary.get(b"D").ok()?,
            named,
            pages,
            depth + 1,
        ),
        Object::Name(name) | Object::String(name, _) => {
            destination_page(document, named.get(name)?, named, pages, depth + 1)
        }
        _ => None,
    }
}

fn pdf_error(error: lopdf::Error) -> BookMcpError {
    match error {
        lopdf::Error::AlreadyEncrypted
        | lopdf::Error::Decryption(_)
        | lopdf::Error::UnsupportedSecurityHandler(_) => BookMcpError::PdfEncrypted,
        error => BookMcpError::Pdf(error.to_string()),
    }
}
