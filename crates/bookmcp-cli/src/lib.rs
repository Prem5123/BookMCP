#![forbid(unsafe_code)]

use std::{
    io::Write,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use bookmcp_core::{BookId, BookMetadata, Chunk, PageNumber, SearchQuery, SearchResult};
use bookmcp_index::{IndexManager, SearchOutput, SearchService};
use bookmcp_ingest::{IngestOptions, IngestPipeline, PdfTextExtractor};
use bookmcp_store::BookStore;
use clap::{Parser, Subcommand, ValueEnum};
use serde::Serialize;

/// BookMCP command-line interface.
#[derive(Debug, Parser)]
#[command(name = "bookmcp")]
#[command(about = "Ingest, search, and serve citable PDF book knowledge")]
#[command(version)]
pub struct Cli {
    /// Command to run.
    #[command(subcommand)]
    pub command: Commands,
}

/// Top-level CLI commands.
#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Ingest a text-based PDF into the local BookMCP library.
    Ingest {
        /// PDF path to ingest.
        pdf_path: PathBuf,
        /// Override detected/fallback title.
        #[arg(long)]
        title: Option<String>,
        /// Override detected author.
        #[arg(long)]
        author: Option<String>,
        /// Override generated book ID.
        #[arg(long)]
        book_id: Option<BookId>,
        /// Override BookMCP data directory.
        #[arg(long)]
        data_dir: Option<PathBuf>,
        /// Replace an existing book with the same ID.
        #[arg(long)]
        force: bool,
    },

    /// List ingested books.
    List {
        /// Override BookMCP data directory.
        #[arg(long)]
        data_dir: Option<PathBuf>,
        /// Emit JSON.
        #[arg(long)]
        json: bool,
    },

    /// Search indexed book chunks.
    Search {
        /// Keyword query.
        query: String,
        /// Restrict search to one book.
        #[arg(long)]
        book_id: Option<BookId>,
        /// Maximum result count.
        #[arg(long)]
        top_k: Option<usize>,
        /// Override BookMCP data directory.
        #[arg(long)]
        data_dir: Option<PathBuf>,
        /// Emit JSON.
        #[arg(long)]
        json: bool,
    },

    /// Fetch one extracted page.
    Page {
        /// Book ID.
        book_id: BookId,
        /// One-based page number.
        page_number: u32,
        /// Override BookMCP data directory.
        #[arg(long)]
        data_dir: Option<PathBuf>,
        /// Emit JSON.
        #[arg(long)]
        json: bool,
    },

    /// Fetch one chunk.
    Chunk {
        /// Book ID.
        book_id: BookId,
        /// Chunk ID.
        chunk_id: bookmcp_core::ChunkId,
        /// Override BookMCP data directory.
        #[arg(long)]
        data_dir: Option<PathBuf>,
        /// Emit JSON.
        #[arg(long)]
        json: bool,
    },

    /// Start the MCP server.
    Serve {
        /// Override BookMCP data directory.
        #[arg(long)]
        data_dir: Option<PathBuf>,
        /// MCP transport.
        #[arg(long, value_enum, default_value_t = Transport::Stdio)]
        transport: Transport,
    },

    /// Check local data and index paths.
    Doctor {
        /// Override BookMCP data directory.
        #[arg(long)]
        data_dir: Option<PathBuf>,
    },

    /// Rebuild the Tantivy keyword index from stored chunks.
    RebuildIndex {
        /// Override BookMCP data directory.
        #[arg(long)]
        data_dir: Option<PathBuf>,
        /// Restrict rebuild to one book.
        #[arg(long)]
        book_id: Option<BookId>,
    },
}

/// Supported MCP transport modes.
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum Transport {
    /// JSON-RPC over stdio.
    Stdio,
}

/// Run the CLI using stdout.
pub fn run(cli: Cli) -> Result<()> {
    let stdout = std::io::stdout();
    let mut writer = stdout.lock();
    run_with_writer(cli, &mut writer)
}

/// Run the CLI with an explicit writer. This is useful for tests.
pub fn run_with_writer<W>(cli: Cli, writer: &mut W) -> Result<()>
where
    W: Write,
{
    match cli.command {
        Commands::Ingest {
            pdf_path,
            title,
            author,
            book_id,
            data_dir,
            force,
        } => run_ingest(writer, pdf_path, title, author, book_id, data_dir, force),
        Commands::List { data_dir, json } => run_list(writer, data_dir, json),
        Commands::Search {
            query,
            book_id,
            top_k,
            data_dir,
            json,
        } => run_search(writer, query, book_id, top_k, data_dir, json),
        Commands::Page {
            book_id,
            page_number,
            data_dir,
            json,
        } => run_page(writer, book_id, page_number, data_dir, json),
        Commands::Chunk {
            book_id,
            chunk_id,
            data_dir,
            json,
        } => run_chunk(writer, book_id, chunk_id, data_dir, json),
        Commands::Serve {
            data_dir,
            transport: Transport::Stdio,
        } => run_serve_stdio(data_dir),
        Commands::Doctor { data_dir } => run_doctor(writer, data_dir),
        Commands::RebuildIndex { data_dir, book_id } => {
            run_rebuild_index(writer, data_dir, book_id)
        }
    }
}

fn run_ingest<W>(
    writer: &mut W,
    pdf_path: PathBuf,
    title: Option<String>,
    author: Option<String>,
    book_id: Option<BookId>,
    data_dir: Option<PathBuf>,
    force: bool,
) -> Result<()>
where
    W: Write,
{
    let data_dir = resolve_data_dir(data_dir)?;
    let mut store = BookStore::open(&data_dir)?;
    let output = IngestPipeline::new(PdfTextExtractor).ingest(IngestOptions {
        pdf_path,
        title,
        author,
        book_id,
    })?;

    if !force && store.get_book(&output.batch.metadata.book_id).is_ok() {
        bail!(
            "book `{}` already exists; pass --force to replace it",
            output.batch.metadata.book_id
        );
    }

    let book_id = output.batch.metadata.book_id.clone();
    let staged_pdf = store.stage_original_pdf(&book_id, &output.source_path)?;
    store.save_ingest(output.batch)?;
    store.commit_staged_original_pdf(staged_pdf)?;
    let indexed_chunks = rebuild_index(&store, &data_dir, None)?;

    write_heading(writer, "Ingest complete")?;
    write_fields(
        writer,
        &[
            ("Book", output.report.title),
            ("Book ID", output.report.book_id.to_string()),
            ("Pages", output.report.page_count.to_string()),
            ("Chunks", output.report.chunk_count.to_string()),
            ("Indexed", indexed_chunks.to_string()),
        ],
    )?;
    Ok(())
}

fn run_list<W>(writer: &mut W, data_dir: Option<PathBuf>, json: bool) -> Result<()>
where
    W: Write,
{
    let store = open_store(data_dir)?;
    let books = store.list_books()?;

    if json {
        write_json(writer, &books)?;
    } else if books.is_empty() {
        write_heading(writer, "Books (0)")?;
        writeln!(writer, "No books ingested yet.")?;
        writeln!(
            writer,
            "Run `bookmcp ingest <pdf>` to add a text-based PDF."
        )?;
    } else {
        write_books_table(writer, &books)?;
    }

    Ok(())
}

fn run_search<W>(
    writer: &mut W,
    query: String,
    book_id: Option<BookId>,
    top_k: Option<usize>,
    data_dir: Option<PathBuf>,
    json: bool,
) -> Result<()>
where
    W: Write,
{
    let data_dir = resolve_data_dir(data_dir)?;
    let manager = IndexManager::create_or_open(index_dir(&data_dir))?;
    let output = SearchService::new(manager).search(SearchQuery {
        query,
        book_id,
        top_k,
    })?;

    if json {
        write_search_json(writer, &output)?;
    } else if output.results.is_empty() {
        write_heading(writer, "Search results (0)")?;
        writeln!(
            writer,
            "{}",
            output.message.unwrap_or_else(|| "no results".to_owned())
        )?;
    } else {
        write_search_results(writer, &output.results)?;
    }

    Ok(())
}

fn run_page<W>(
    writer: &mut W,
    book_id: BookId,
    page_number: u32,
    data_dir: Option<PathBuf>,
    json: bool,
) -> Result<()>
where
    W: Write,
{
    let store = open_store(data_dir)?;
    let page = store.get_page(&book_id, PageNumber::new(page_number)?)?;

    if json {
        write_json(writer, &page)?;
    } else {
        write_heading(writer, "Page")?;
        write_fields(
            writer,
            &[
                ("Book ID", page.book_id.to_string()),
                ("Page", page.page_number.to_string()),
                ("Citation", page.citation.format()),
            ],
        )?;
        writeln!(writer)?;
        writeln!(writer, "{}", page.text)?;
    }

    Ok(())
}

fn run_chunk<W>(
    writer: &mut W,
    book_id: BookId,
    chunk_id: bookmcp_core::ChunkId,
    data_dir: Option<PathBuf>,
    json: bool,
) -> Result<()>
where
    W: Write,
{
    let store = open_store(data_dir)?;
    let chunk = store.get_chunk(&book_id, &chunk_id)?;

    if json {
        write_json(writer, &chunk)?;
    } else {
        write_heading(writer, "Chunk")?;
        write_fields(
            writer,
            &[
                ("Book ID", chunk.book_id.to_string()),
                ("Chunk ID", chunk.chunk_id.to_string()),
                ("Pages", page_range(&chunk)),
                ("Citation", chunk.citation.format()),
            ],
        )?;
        writeln!(writer)?;
        writeln!(writer, "{}", chunk.text)?;
    }

    Ok(())
}

fn run_doctor<W>(writer: &mut W, data_dir: Option<PathBuf>) -> Result<()>
where
    W: Write,
{
    let data_dir = resolve_data_dir(data_dir)?;
    let store = BookStore::open(&data_dir)?;
    let index_manager = IndexManager::create_or_open(index_dir(&data_dir))?;

    write_heading(writer, "BookMCP doctor")?;
    write_fields(
        writer,
        &[
            ("Data directory", data_dir.display().to_string()),
            ("Database", store.database_path().display().to_string()),
            ("Index", index_manager.index_path().display().to_string()),
            ("Books", store.list_books()?.len().to_string()),
            ("Status", "ready".to_owned()),
        ],
    )?;
    Ok(())
}

fn run_rebuild_index<W>(
    writer: &mut W,
    data_dir: Option<PathBuf>,
    book_id: Option<BookId>,
) -> Result<()>
where
    W: Write,
{
    let data_dir = resolve_data_dir(data_dir)?;
    let store = BookStore::open(&data_dir)?;
    let indexed_chunks = rebuild_index(&store, &data_dir, book_id.as_ref())?;

    write_heading(writer, "Index rebuilt")?;
    write_fields(writer, &[("Chunks indexed", indexed_chunks.to_string())])?;
    Ok(())
}

fn run_serve_stdio(data_dir: Option<PathBuf>) -> Result<()> {
    let data_dir = resolve_data_dir(data_dir)?;
    init_stderr_logging();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("failed to start async runtime")?;
    runtime
        .block_on(bookmcp_mcp::serve_stdio(data_dir))
        .context("MCP stdio server stopped with an error")
}

fn init_stderr_logging() {
    let _ = tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_target(false)
        .try_init();
}

fn open_store(data_dir: Option<PathBuf>) -> Result<BookStore> {
    let data_dir = resolve_data_dir(data_dir)?;
    BookStore::open(data_dir).map_err(Into::into)
}

fn resolve_data_dir(data_dir: Option<PathBuf>) -> Result<PathBuf> {
    data_dir.map_or_else(|| BookStore::default_data_dir().map_err(Into::into), Ok)
}

fn rebuild_index(store: &BookStore, data_dir: &Path, book_id: Option<&BookId>) -> Result<usize> {
    let chunks = chunks_for_rebuild(store, book_id)?;
    let manager = IndexManager::create_or_open(index_dir(data_dir))?;
    manager.rebuild(&chunks)?;
    Ok(chunks.len())
}

fn chunks_for_rebuild(store: &BookStore, book_id: Option<&BookId>) -> Result<Vec<Chunk>> {
    if let Some(book_id) = book_id {
        return store.list_chunks(book_id).map_err(Into::into);
    }

    let mut chunks = Vec::new();
    for book in store.list_books()? {
        chunks.extend(store.list_chunks(&book.book_id)?);
    }

    Ok(chunks)
}

fn index_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("index")
}

fn write_json<W, T>(writer: &mut W, value: &T) -> Result<()>
where
    W: Write,
    T: Serialize,
{
    serde_json::to_writer_pretty(&mut *writer, value).context("failed to serialize JSON output")?;
    writeln!(writer)?;
    Ok(())
}

fn write_search_json<W>(writer: &mut W, output: &SearchOutput) -> Result<()>
where
    W: Write,
{
    let value = serde_json::json!({
        "results": output.results,
        "message": output.message,
    });
    write_json(writer, &value)
}

fn write_heading<W>(writer: &mut W, title: &str) -> Result<()>
where
    W: Write,
{
    writeln!(writer, "{title}")?;
    writeln!(writer, "{}", "-".repeat(title.chars().count()))?;
    Ok(())
}

fn write_fields<W>(writer: &mut W, fields: &[(&str, String)]) -> Result<()>
where
    W: Write,
{
    let label_width = fields
        .iter()
        .map(|(label, _)| label.chars().count())
        .max()
        .unwrap_or(0);

    for (label, value) in fields {
        write_padded_cell(writer, label, label_width)?;
        writeln!(writer, "  {value}")?;
    }

    Ok(())
}

fn write_books_table<W>(writer: &mut W, books: &[BookMetadata]) -> Result<()>
where
    W: Write,
{
    write_heading(writer, &format!("Books ({})", books.len()))?;
    let rows = books
        .iter()
        .map(|book| {
            vec![
                book.book_id.to_string(),
                book.title.clone(),
                book.author
                    .clone()
                    .unwrap_or_else(|| "unknown author".to_owned()),
                book.page_count.to_string(),
                book.chunk_count.to_string(),
            ]
        })
        .collect::<Vec<_>>();
    write_table(writer, &["ID", "Title", "Author", "Pages", "Chunks"], &rows)
}

fn write_search_results<W>(writer: &mut W, results: &[SearchResult]) -> Result<()>
where
    W: Write,
{
    write_heading(writer, &format!("Search results ({})", results.len()))?;

    for (index, result) in results.iter().enumerate() {
        if index > 0 {
            writeln!(writer)?;
        }

        writeln!(
            writer,
            "#{} {}  score {:.3}",
            index + 1,
            result.chunk_id,
            result.score
        )?;
        write_fields(
            writer,
            &[
                ("Book ID", result.book_id.to_string()),
                ("Pages", page_range_result(result)),
                ("Citation", result.citation.format()),
            ],
        )?;
        writeln!(writer)?;
        writeln!(writer, "{}", one_line(&result.snippet))?;
    }

    Ok(())
}

fn write_table<W>(writer: &mut W, headers: &[&str], rows: &[Vec<String>]) -> Result<()>
where
    W: Write,
{
    let widths = column_widths(headers, rows);
    write_table_row(writer, headers, &widths)?;
    write_separator(writer, &widths)?;

    for row in rows {
        write_table_row(writer, row, &widths)?;
    }

    Ok(())
}

fn column_widths(headers: &[&str], rows: &[Vec<String>]) -> Vec<usize> {
    headers
        .iter()
        .enumerate()
        .map(|(index, header)| {
            rows.iter()
                .filter_map(|row| row.get(index))
                .map(|cell| cell.chars().count())
                .chain(std::iter::once(header.chars().count()))
                .max()
                .unwrap_or(0)
        })
        .collect()
}

fn write_table_row<W, S>(writer: &mut W, cells: &[S], widths: &[usize]) -> Result<()>
where
    W: Write,
    S: AsRef<str>,
{
    for (index, width) in widths.iter().enumerate() {
        if index > 0 {
            write!(writer, "  ")?;
        }
        let cell = cells.get(index).map(AsRef::as_ref).unwrap_or("");
        write_padded_cell(writer, cell, *width)?;
    }
    writeln!(writer)?;
    Ok(())
}

fn write_separator<W>(writer: &mut W, widths: &[usize]) -> Result<()>
where
    W: Write,
{
    for (index, width) in widths.iter().enumerate() {
        if index > 0 {
            write!(writer, "  ")?;
        }
        write!(writer, "{}", "-".repeat(*width))?;
    }
    writeln!(writer)?;
    Ok(())
}

fn write_padded_cell<W>(writer: &mut W, cell: &str, width: usize) -> Result<()>
where
    W: Write,
{
    write!(writer, "{cell}")?;
    for _ in cell.chars().count()..width {
        write!(writer, " ")?;
    }
    Ok(())
}

fn page_range(chunk: &Chunk) -> String {
    if chunk.page_start == chunk.page_end {
        chunk.page_start.to_string()
    } else {
        format!("{}-{}", chunk.page_start, chunk.page_end)
    }
}

fn page_range_result(result: &SearchResult) -> String {
    if result.page_start == result.page_end {
        result.page_start.to_string()
    } else {
        format!("{}-{}", result.page_start, result.page_end)
    }
}

fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}
