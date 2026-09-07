#![forbid(unsafe_code)]

use std::{
    io::Write,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use bookmcp_core::{BookId, Chunk, PageNumber, SearchQuery};
use bookmcp_index::{IndexManager, SearchOutput, SearchService};
use bookmcp_ingest::{IngestOptions, IngestPipeline, PdfTextExtractor};
use bookmcp_store::BookStore;
use clap::{Parser, Subcommand, ValueEnum};
use serde::Serialize;
use sha2::{Digest, Sha256};

mod agent;
mod lessons;

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
    /// Print portable MCP configuration with absolute executable and library paths.
    McpConfig {
        /// Configuration format for the agent client.
        #[arg(value_enum)]
        client: agent::McpClient,
        /// Override BookMCP data directory.
        #[arg(long)]
        data_dir: Option<PathBuf>,
    },

    /// Print a compact library index and retrieval instructions for agent context.
    Context {
        /// Override BookMCP data directory.
        #[arg(long)]
        data_dir: Option<PathBuf>,
        /// Pagination offset for larger libraries.
        #[arg(long, default_value_t = 0)]
        offset: usize,
        /// Emit JSON.
        #[arg(long)]
        json: bool,
    },

    /// Save and manage cited lessons; MCP exposes these as read-only knowledge.
    Lesson {
        #[command(subcommand)]
        command: lessons::LessonCommand,
    },
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
        /// Emit machine-readable health information.
        #[arg(long)]
        json: bool,
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
    if let Commands::Serve { data_dir, .. } = cli.command {
        // Tokio's stdio writer acquires stdout on a blocking thread. Holding a
        // synchronous StdoutLock here would prevent the MCP handshake forever.
        return run_serve_stdio(data_dir);
    }
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
        Commands::McpConfig { client, data_dir } => agent::config(writer, client, data_dir),
        Commands::Context {
            data_dir,
            offset,
            json,
        } => agent::context(writer, data_dir, offset, json),
        Commands::Lesson { command } => lessons::run(writer, command),
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
        Commands::Doctor { data_dir, json } => run_doctor(writer, data_dir, json),
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
    let _writer_lock = lock_library(&data_dir)?;
    let mut store = BookStore::open(&data_dir)?;
    let output = IngestPipeline::new(PdfTextExtractor).ingest(IngestOptions {
        pdf_path,
        title,
        author,
        book_id,
    })?;

    match store.get_book(&output.batch.metadata.book_id) {
        Ok(_) if !force => bail!(
            "book `{}` already exists; pass --force to replace it",
            output.batch.metadata.book_id
        ),
        Ok(_) | Err(bookmcp_core::BookMcpError::NotFound { .. }) => {}
        Err(error) => return Err(error.into()),
    }

    let book_id = output.batch.metadata.book_id.clone();
    let staged_pdf = store.stage_original_pdf(&book_id, &output.source_path)?;
    store.save_ingest_with_original(output.batch, staged_pdf)?;
    let indexed_chunks = rebuild_index(&store, &data_dir, Some(&book_id))
        .context("book saved, but search indexing failed; run `bookmcp rebuild-index` with the same --data-dir to repair")?;

    writeln!(
        writer,
        "ingested `{}` as {} ({} pages, {} chunks, {} indexed)",
        output.report.title,
        output.report.book_id,
        output.report.page_count,
        output.report.chunk_count,
        indexed_chunks
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
        writeln!(writer, "No books ingested.")?;
    } else {
        for book in books {
            writeln!(
                writer,
                "{}\t{}\t{}\t{} pages\t{} chunks",
                book.book_id,
                book.title,
                book.author.unwrap_or_else(|| "unknown author".to_owned()),
                book.page_count,
                book.chunk_count
            )?;
        }
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
    let manager = IndexManager::open_read_only(index_dir(&data_dir))?;
    let output = SearchService::new(manager).search(SearchQuery {
        query,
        book_id,
        top_k,
    })?;

    if json {
        write_search_json(writer, &output)?;
    } else if output.results.is_empty() {
        writeln!(
            writer,
            "{}",
            output.message.unwrap_or_else(|| "no results".to_owned())
        )?;
    } else {
        for result in output.results {
            writeln!(
                writer,
                "{}\t{}\t{:.3}\t{}",
                result.book_id,
                result.chunk_id,
                result.score,
                result.citation.format()
            )?;
            writeln!(writer, "{}", result.snippet)?;
        }
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
        writeln!(writer, "{}", page.citation.format())?;
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
        writeln!(writer, "{}", chunk.citation.format())?;
        writeln!(writer, "{}", chunk.text)?;
    }

    Ok(())
}

fn run_doctor<W>(writer: &mut W, data_dir: Option<PathBuf>, json: bool) -> Result<()>
where
    W: Write,
{
    let data_dir = resolve_data_dir(data_dir)?;
    let _writer_lock = lock_library(&data_dir)?;
    let store = BookStore::open(&data_dir)?;
    let index_manager = IndexManager::create_or_open(index_dir(&data_dir))?;
    store.check_integrity()?;
    let books = store.list_books()?;
    let chunks = chunks_for_rebuild(&store, None)?;
    index_manager.check_chunks(&chunks)
        .context("search index does not match the library; run `bookmcp rebuild-index` with the same --data-dir")?;
    for book in &books {
        let path = store.library_pdf_path(&book.book_id)?;
        let mut original = std::fs::File::open(&path).with_context(|| {
            format!("managed original missing or unreadable: {}", path.display())
        })?;
        let mut hasher = Sha256::new();
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            let length = std::io::Read::read(&mut original, &mut buffer)?;
            if length == 0 {
                break;
            }
            hasher.update(&buffer[..length]);
        }
        if hex::encode(hasher.finalize()) != book.source_sha256 {
            bail!(
                "managed original hash mismatch for {}; restore its original PDF from a trusted backup or ingest it again",
                book.book_id
            );
        }
    }
    if json {
        write_json(
            writer,
            &serde_json::json!({
                "status": "ok", "data_dir": data_dir, "database": store.database_path(),
                "index": index_manager.index_path(), "books": books.len(), "chunks": chunks.len(),
                "lessons": store.count_lessons(None)?
            }),
        )?;
    } else {
        writeln!(writer, "data_dir: {}", data_dir.display())?;
        writeln!(writer, "database: {}", store.database_path().display())?;
        writeln!(writer, "index: {}", index_manager.index_path().display())?;
        writeln!(writer, "books: {}", books.len())?;
        writeln!(writer, "lessons: {}", store.count_lessons(None)?)?;
        writeln!(
            writer,
            "health: ok (database, index, and original PDF hashes verified)"
        )?;
    }
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
    let _writer_lock = lock_library(&data_dir)?;
    let store = BookStore::open(&data_dir)?;
    let indexed_chunks = rebuild_index(&store, &data_dir, book_id.as_ref())?;

    writeln!(writer, "rebuilt keyword index with {indexed_chunks} chunks")?;
    Ok(())
}

fn run_serve_stdio(data_dir: Option<PathBuf>) -> Result<()> {
    let data_dir = resolve_data_dir(data_dir)?;
    init_stderr_logging();
    // Initialize explicitly at CLI startup; MCP request handlers only open existing data.
    {
        let _writer_lock = lock_library(&data_dir)?;
        let store = BookStore::open(&data_dir)?;
        if !index_dir(&data_dir).join("meta.json").is_file() {
            tracing::info!("building missing search index from the local library");
            rebuild_index(&store, &data_dir, None)?;
        } else {
            IndexManager::open_read_only(index_dir(&data_dir)).context(
                "cannot open search index; run `bookmcp rebuild-index` with the same --data-dir",
            )?;
        }
    }
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

/// Serialize CLI mutations before reading the database snapshot used to rebuild search.
fn lock_library(data_dir: &Path) -> Result<std::fs::File> {
    std::fs::create_dir_all(data_dir)?;
    let lock = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(data_dir.join(".bookmcp-writer.lock"))?;
    lock.try_lock().context(
        "library writer lock unavailable; let the current ingest/rebuild finish and retry",
    )?;
    Ok(lock)
}

fn rebuild_index(store: &BookStore, data_dir: &Path, book_id: Option<&BookId>) -> Result<usize> {
    if let Some(book_id) = book_id {
        store.get_book(book_id)?;
    }
    // If the entire derived index disappeared, restoring one book would hide the others.
    let book_id = book_id.filter(|_| index_dir(data_dir).join("meta.json").is_file());
    let chunks = chunks_for_rebuild(store, book_id)?;
    if let Some(book_id) = book_id {
        let manager = IndexManager::create_or_open(index_dir(data_dir))?;
        manager.replace_book(book_id, &chunks)?;
    } else {
        IndexManager::recreate(index_dir(data_dir), &chunks)?;
    }
    Ok(chunks.len())
}

fn chunks_for_rebuild(store: &BookStore, book_id: Option<&BookId>) -> Result<Vec<Chunk>> {
    if let Some(book_id) = book_id {
        store.get_book(book_id)?;
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
