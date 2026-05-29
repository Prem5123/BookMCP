#![forbid(unsafe_code)]

use std::{
    env, fmt, fs,
    path::{Path, PathBuf},
};

use bookmcp_core::{
    BookId, BookMcpError, BookMetadata, Chapter, ChapterId, Chunk, ChunkId, Citation, Page,
    PageNumber, Result,
};
use directories::ProjectDirs;
use rusqlite::{Connection, OptionalExtension, Row, Transaction, params};

const DATABASE_FILE_NAME: &str = "bookmcp.sqlite3";

/// Complete set of normalized records produced by one ingest run.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IngestBatch {
    /// Book metadata.
    pub metadata: BookMetadata,
    /// Extracted pages.
    pub pages: Vec<Page>,
    /// Detected chapters.
    pub chapters: Vec<Chapter>,
    /// Searchable chunks.
    pub chunks: Vec<Chunk>,
}

/// Prepared source PDF copy waiting to be moved into the managed library.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagedOriginalPdf {
    temp_path: PathBuf,
    destination: PathBuf,
}

impl StagedOriginalPdf {
    /// Final managed library path for this staged PDF.
    pub fn destination(&self) -> &Path {
        &self.destination
    }
}

/// SQLite-backed persistent store for BookMCP library records.
pub struct BookStore {
    conn: Connection,
    database_path: PathBuf,
}

impl BookStore {
    /// Open or create a BookMCP store under a data directory.
    pub fn open(data_dir: impl AsRef<Path>) -> Result<Self> {
        let data_dir = data_dir.as_ref();
        fs::create_dir_all(data_dir)?;
        let database_path = data_dir.join(DATABASE_FILE_NAME);
        Self::open_database(database_path)
    }

    /// Open or create a store at an explicit SQLite database path.
    pub fn open_database(database_path: impl AsRef<Path>) -> Result<Self> {
        let database_path = database_path.as_ref().to_path_buf();
        if let Some(parent) = database_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(&database_path).map_err(storage_error)?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")
            .map_err(storage_error)?;

        let store = Self {
            conn,
            database_path,
        };
        store.initialize()?;
        Ok(store)
    }

    /// Return the default BookMCP data directory.
    pub fn default_data_dir() -> Result<PathBuf> {
        if let Ok(home) = env::var("BOOKMCP_HOME")
            && !home.trim().is_empty()
        {
            return Ok(PathBuf::from(home));
        }

        ProjectDirs::from("dev", "BookMCP", "BookMCP")
            .map(|dirs| dirs.data_dir().to_path_buf())
            .ok_or_else(|| BookMcpError::Storage("could not determine data directory".to_owned()))
    }

    /// Path to the SQLite database file.
    pub fn database_path(&self) -> &Path {
        &self.database_path
    }

    /// Return the managed library path for a stored source PDF.
    pub fn library_pdf_path(&self, book_id: &BookId) -> Result<PathBuf> {
        Ok(self
            .data_dir()?
            .join("library")
            .join(format!("{book_id}.pdf")))
    }

    /// Copy a source PDF into a temporary file next to its final library path.
    pub fn stage_original_pdf(
        &self,
        book_id: &BookId,
        source_path: impl AsRef<Path>,
    ) -> Result<StagedOriginalPdf> {
        let source_path = source_path.as_ref();
        let metadata = fs::metadata(source_path)?;
        if !metadata.is_file() {
            return Err(BookMcpError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("{} is not a file", source_path.display()),
            )));
        }

        let destination = self.library_pdf_path(book_id)?;
        let library_dir = destination
            .parent()
            .ok_or_else(|| BookMcpError::Storage("library path has no parent".to_owned()))?;
        fs::create_dir_all(library_dir)?;
        let temp_destination = destination.with_extension("pdf.tmp");
        fs::copy(source_path, &temp_destination)?;
        Ok(StagedOriginalPdf {
            temp_path: temp_destination,
            destination,
        })
    }

    /// Move a staged source PDF into its final managed library path.
    pub fn commit_staged_original_pdf(&self, staged: StagedOriginalPdf) -> Result<PathBuf> {
        fs::rename(&staged.temp_path, &staged.destination)?;
        Ok(staged.destination)
    }

    /// Copy a source PDF into BookMCP's managed library directory.
    pub fn store_original_pdf(
        &self,
        book_id: &BookId,
        source_path: impl AsRef<Path>,
    ) -> Result<PathBuf> {
        let staged = self.stage_original_pdf(book_id, source_path)?;
        let destination = self.commit_staged_original_pdf(staged)?;
        Ok(destination)
    }

    /// Initialize the current schema.
    pub fn initialize(&self) -> Result<()> {
        self.conn
            .execute_batch(
                r#"
                CREATE TABLE IF NOT EXISTS schema_migrations (
                    version INTEGER PRIMARY KEY,
                    applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
                );

                INSERT OR IGNORE INTO schema_migrations (version) VALUES (1);

                CREATE TABLE IF NOT EXISTS books (
                    book_id TEXT PRIMARY KEY NOT NULL,
                    title TEXT NOT NULL,
                    author TEXT,
                    source_sha256 TEXT NOT NULL,
                    page_count INTEGER NOT NULL,
                    chapter_count INTEGER NOT NULL,
                    chunk_count INTEGER NOT NULL,
                    ingested_at TEXT NOT NULL
                );

                CREATE TABLE IF NOT EXISTS pages (
                    book_id TEXT NOT NULL,
                    page_number INTEGER NOT NULL,
                    text TEXT NOT NULL,
                    citation_json TEXT NOT NULL,
                    PRIMARY KEY (book_id, page_number),
                    FOREIGN KEY (book_id) REFERENCES books(book_id) ON DELETE CASCADE
                );

                CREATE TABLE IF NOT EXISTS chapters (
                    book_id TEXT NOT NULL,
                    chapter_id TEXT NOT NULL,
                    title TEXT NOT NULL,
                    page_start INTEGER NOT NULL,
                    page_end INTEGER NOT NULL,
                    PRIMARY KEY (book_id, chapter_id),
                    FOREIGN KEY (book_id) REFERENCES books(book_id) ON DELETE CASCADE
                );

                CREATE TABLE IF NOT EXISTS chunks (
                    book_id TEXT NOT NULL,
                    chunk_id TEXT NOT NULL,
                    chapter_id TEXT,
                    chapter_title TEXT,
                    page_start INTEGER NOT NULL,
                    page_end INTEGER NOT NULL,
                    text TEXT NOT NULL,
                    citation_json TEXT NOT NULL,
                    ordinal INTEGER NOT NULL,
                    PRIMARY KEY (book_id, chunk_id),
                    FOREIGN KEY (book_id) REFERENCES books(book_id) ON DELETE CASCADE
                );

                CREATE INDEX IF NOT EXISTS idx_chunks_book_ordinal
                    ON chunks(book_id, ordinal);

                CREATE TABLE IF NOT EXISTS ingest_runs (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    book_id TEXT NOT NULL,
                    source_sha256 TEXT NOT NULL,
                    ingested_at TEXT NOT NULL,
                    status TEXT NOT NULL,
                    message TEXT,
                    FOREIGN KEY (book_id) REFERENCES books(book_id) ON DELETE CASCADE
                );

                CREATE TABLE IF NOT EXISTS index_metadata (
                    book_id TEXT PRIMARY KEY NOT NULL,
                    rebuilt_at TEXT NOT NULL,
                    FOREIGN KEY (book_id) REFERENCES books(book_id) ON DELETE CASCADE
                );
                "#,
            )
            .map_err(storage_error)
    }

    /// Save one ingest batch transactionally, replacing existing rows for the book.
    pub fn save_ingest(&mut self, batch: IngestBatch) -> Result<()> {
        validate_batch_book_ids(&batch)?;

        let tx = self.conn.transaction().map_err(storage_error)?;
        upsert_book(&tx, &batch.metadata)?;
        clear_book_children(&tx, &batch.metadata.book_id)?;

        for page in &batch.pages {
            insert_page(&tx, page)?;
        }

        for chapter in &batch.chapters {
            insert_chapter(&tx, chapter)?;
        }

        for (ordinal, chunk) in batch.chunks.iter().enumerate() {
            insert_chunk(&tx, chunk, usize_to_i64(ordinal, "chunk ordinal")?)?;
        }

        insert_ingest_run(&tx, &batch.metadata, "success", None)?;
        tx.commit().map_err(storage_error)
    }

    /// List all stored books.
    pub fn list_books(&self) -> Result<Vec<BookMetadata>> {
        let mut stmt = self
            .conn
            .prepare(
                r#"
                SELECT book_id, title, author, source_sha256, page_count,
                       chapter_count, chunk_count, ingested_at
                FROM books
                ORDER BY title COLLATE NOCASE, book_id
                "#,
            )
            .map_err(storage_error)?;
        let mut rows = stmt.query([]).map_err(storage_error)?;
        let mut books = Vec::new();

        while let Some(row) = rows.next().map_err(storage_error)? {
            books.push(book_from_row(row)?);
        }

        Ok(books)
    }

    /// Fetch metadata for one book.
    pub fn get_book(&self, book_id: &BookId) -> Result<BookMetadata> {
        let mut stmt = self
            .conn
            .prepare(
                r#"
                SELECT book_id, title, author, source_sha256, page_count,
                       chapter_count, chunk_count, ingested_at
                FROM books
                WHERE book_id = ?1
                "#,
            )
            .map_err(storage_error)?;
        let mut rows = stmt
            .query(params![book_id.as_str()])
            .map_err(storage_error)?;

        if let Some(row) = rows.next().map_err(storage_error)? {
            book_from_row(row)
        } else {
            Err(not_found("book", book_id.as_str()))
        }
    }

    /// List pages for one book in page order.
    pub fn list_pages(&self, book_id: &BookId) -> Result<Vec<Page>> {
        let mut stmt = self
            .conn
            .prepare(
                r#"
                SELECT book_id, page_number, text, citation_json
                FROM pages
                WHERE book_id = ?1
                ORDER BY page_number
                "#,
            )
            .map_err(storage_error)?;
        let mut rows = stmt
            .query(params![book_id.as_str()])
            .map_err(storage_error)?;
        let mut pages = Vec::new();

        while let Some(row) = rows.next().map_err(storage_error)? {
            pages.push(page_from_row(row)?);
        }

        Ok(pages)
    }

    /// Fetch one extracted page.
    pub fn get_page(&self, book_id: &BookId, page_number: PageNumber) -> Result<Page> {
        let mut stmt = self
            .conn
            .prepare(
                r#"
                SELECT book_id, page_number, text, citation_json
                FROM pages
                WHERE book_id = ?1 AND page_number = ?2
                "#,
            )
            .map_err(storage_error)?;
        let mut rows = stmt
            .query(params![book_id.as_str(), page_number.get()])
            .map_err(storage_error)?;

        if let Some(row) = rows.next().map_err(storage_error)? {
            page_from_row(row)
        } else {
            Err(not_found(
                "page",
                &format!("{}:{}", book_id.as_str(), page_number.get()),
            ))
        }
    }

    /// Fetch one detected chapter.
    pub fn get_chapter(&self, book_id: &BookId, chapter_id: &ChapterId) -> Result<Chapter> {
        let mut stmt = self
            .conn
            .prepare(
                r#"
                SELECT book_id, chapter_id, title, page_start, page_end
                FROM chapters
                WHERE book_id = ?1 AND chapter_id = ?2
                "#,
            )
            .map_err(storage_error)?;
        let mut rows = stmt
            .query(params![book_id.as_str(), chapter_id.as_str()])
            .map_err(storage_error)?;

        if let Some(row) = rows.next().map_err(storage_error)? {
            chapter_from_row(row)
        } else {
            Err(not_found(
                "chapter",
                &format!("{}:{}", book_id.as_str(), chapter_id.as_str()),
            ))
        }
    }

    /// List chapters for one book in page order.
    pub fn list_chapters(&self, book_id: &BookId) -> Result<Vec<Chapter>> {
        let mut stmt = self
            .conn
            .prepare(
                r#"
                SELECT book_id, chapter_id, title, page_start, page_end
                FROM chapters
                WHERE book_id = ?1
                ORDER BY page_start, chapter_id
                "#,
            )
            .map_err(storage_error)?;
        let mut rows = stmt
            .query(params![book_id.as_str()])
            .map_err(storage_error)?;
        let mut chapters = Vec::new();

        while let Some(row) = rows.next().map_err(storage_error)? {
            chapters.push(chapter_from_row(row)?);
        }

        Ok(chapters)
    }

    /// List chunks for one book in chunk order.
    pub fn list_chunks(&self, book_id: &BookId) -> Result<Vec<Chunk>> {
        let mut stmt = self
            .conn
            .prepare(
                r#"
                SELECT book_id, chunk_id, chapter_id, chapter_title,
                       page_start, page_end, text, citation_json
                FROM chunks
                WHERE book_id = ?1
                ORDER BY ordinal
                "#,
            )
            .map_err(storage_error)?;
        let mut rows = stmt
            .query(params![book_id.as_str()])
            .map_err(storage_error)?;
        let mut chunks = Vec::new();

        while let Some(row) = rows.next().map_err(storage_error)? {
            chunks.push(chunk_from_row(row)?);
        }

        Ok(chunks)
    }

    /// Fetch one chunk.
    pub fn get_chunk(&self, book_id: &BookId, chunk_id: &ChunkId) -> Result<Chunk> {
        let mut stmt = self
            .conn
            .prepare(
                r#"
                SELECT book_id, chunk_id, chapter_id, chapter_title,
                       page_start, page_end, text, citation_json
                FROM chunks
                WHERE book_id = ?1 AND chunk_id = ?2
                "#,
            )
            .map_err(storage_error)?;
        let mut rows = stmt
            .query(params![book_id.as_str(), chunk_id.as_str()])
            .map_err(storage_error)?;

        if let Some(row) = rows.next().map_err(storage_error)? {
            chunk_from_row(row)
        } else {
            Err(not_found(
                "chunk",
                &format!("{}:{}", book_id.as_str(), chunk_id.as_str()),
            ))
        }
    }

    /// Fetch chunks around a target chunk, including the target.
    pub fn get_chunks_around(
        &self,
        book_id: &BookId,
        chunk_id: &ChunkId,
        before: usize,
        after: usize,
    ) -> Result<Vec<Chunk>> {
        let ordinal = self.chunk_ordinal(book_id, chunk_id)?;
        let before = usize_to_i64(before, "before window")?;
        let after = usize_to_i64(after, "after window")?;
        let start = ordinal.saturating_sub(before);
        let end = ordinal.saturating_add(after);

        let mut stmt = self
            .conn
            .prepare(
                r#"
                SELECT book_id, chunk_id, chapter_id, chapter_title,
                       page_start, page_end, text, citation_json
                FROM chunks
                WHERE book_id = ?1 AND ordinal BETWEEN ?2 AND ?3
                ORDER BY ordinal
                "#,
            )
            .map_err(storage_error)?;
        let mut rows = stmt
            .query(params![book_id.as_str(), start, end])
            .map_err(storage_error)?;
        let mut chunks = Vec::new();

        while let Some(row) = rows.next().map_err(storage_error)? {
            chunks.push(chunk_from_row(row)?);
        }

        Ok(chunks)
    }

    /// Record that a book's search index metadata has been rebuilt.
    pub fn mark_index_rebuilt(&self, book_id: &BookId, rebuilt_at: &str) -> Result<()> {
        self.conn
            .execute(
                r#"
                INSERT INTO index_metadata (book_id, rebuilt_at)
                VALUES (?1, ?2)
                ON CONFLICT(book_id) DO UPDATE SET rebuilt_at = excluded.rebuilt_at
                "#,
                params![book_id.as_str(), rebuilt_at],
            )
            .map(|_| ())
            .map_err(storage_error)
    }

    /// Fetch index rebuild metadata for a book.
    pub fn index_rebuilt_at(&self, book_id: &BookId) -> Result<Option<String>> {
        self.conn
            .query_row(
                "SELECT rebuilt_at FROM index_metadata WHERE book_id = ?1",
                params![book_id.as_str()],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage_error)
    }

    /// Clear index rebuild metadata for one book or all books.
    pub fn clear_index_metadata(&self, book_id: Option<&BookId>) -> Result<()> {
        match book_id {
            Some(book_id) => self
                .conn
                .execute(
                    "DELETE FROM index_metadata WHERE book_id = ?1",
                    params![book_id.as_str()],
                )
                .map(|_| ())
                .map_err(storage_error),
            None => self
                .conn
                .execute("DELETE FROM index_metadata", [])
                .map(|_| ())
                .map_err(storage_error),
        }
    }

    fn chunk_ordinal(&self, book_id: &BookId, chunk_id: &ChunkId) -> Result<i64> {
        self.conn
            .query_row(
                "SELECT ordinal FROM chunks WHERE book_id = ?1 AND chunk_id = ?2",
                params![book_id.as_str(), chunk_id.as_str()],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage_error)?
            .ok_or_else(|| {
                not_found(
                    "chunk",
                    &format!("{}:{}", book_id.as_str(), chunk_id.as_str()),
                )
            })
    }

    fn data_dir(&self) -> Result<PathBuf> {
        self.database_path
            .parent()
            .map(Path::to_path_buf)
            .ok_or_else(|| BookMcpError::Storage("database path has no parent".to_owned()))
    }
}

fn validate_batch_book_ids(batch: &IngestBatch) -> Result<()> {
    let expected = &batch.metadata.book_id;

    for page in &batch.pages {
        if &page.book_id != expected {
            return Err(BookMcpError::Storage(format!(
                "page {} belongs to book {}, expected {}",
                page.page_number, page.book_id, expected
            )));
        }
    }

    for chapter in &batch.chapters {
        if &chapter.book_id != expected {
            return Err(BookMcpError::Storage(format!(
                "chapter {} belongs to book {}, expected {}",
                chapter.chapter_id, chapter.book_id, expected
            )));
        }
    }

    for chunk in &batch.chunks {
        if &chunk.book_id != expected {
            return Err(BookMcpError::Storage(format!(
                "chunk {} belongs to book {}, expected {}",
                chunk.chunk_id, chunk.book_id, expected
            )));
        }
    }

    Ok(())
}

fn upsert_book(tx: &Transaction<'_>, metadata: &BookMetadata) -> Result<()> {
    tx.execute(
        r#"
        INSERT INTO books (
            book_id, title, author, source_sha256, page_count,
            chapter_count, chunk_count, ingested_at
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        ON CONFLICT(book_id) DO UPDATE SET
            title = excluded.title,
            author = excluded.author,
            source_sha256 = excluded.source_sha256,
            page_count = excluded.page_count,
            chapter_count = excluded.chapter_count,
            chunk_count = excluded.chunk_count,
            ingested_at = excluded.ingested_at
        "#,
        params![
            metadata.book_id.as_str(),
            metadata.title,
            metadata.author,
            metadata.source_sha256,
            metadata.page_count,
            metadata.chapter_count,
            metadata.chunk_count,
            metadata.ingested_at
        ],
    )
    .map(|_| ())
    .map_err(storage_error)
}

fn clear_book_children(tx: &Transaction<'_>, book_id: &BookId) -> Result<()> {
    for sql in [
        "DELETE FROM index_metadata WHERE book_id = ?1",
        "DELETE FROM chunks WHERE book_id = ?1",
        "DELETE FROM chapters WHERE book_id = ?1",
        "DELETE FROM pages WHERE book_id = ?1",
        "DELETE FROM ingest_runs WHERE book_id = ?1",
    ] {
        tx.execute(sql, params![book_id.as_str()])
            .map_err(storage_error)?;
    }

    Ok(())
}

fn insert_page(tx: &Transaction<'_>, page: &Page) -> Result<()> {
    let citation_json = citation_json(&page.citation)?;
    tx.execute(
        r#"
        INSERT INTO pages (book_id, page_number, text, citation_json)
        VALUES (?1, ?2, ?3, ?4)
        "#,
        params![
            page.book_id.as_str(),
            page.page_number.get(),
            page.text,
            citation_json
        ],
    )
    .map(|_| ())
    .map_err(storage_error)
}

fn insert_chapter(tx: &Transaction<'_>, chapter: &Chapter) -> Result<()> {
    tx.execute(
        r#"
        INSERT INTO chapters (book_id, chapter_id, title, page_start, page_end)
        VALUES (?1, ?2, ?3, ?4, ?5)
        "#,
        params![
            chapter.book_id.as_str(),
            chapter.chapter_id.as_str(),
            chapter.title,
            chapter.page_start.get(),
            chapter.page_end.get()
        ],
    )
    .map(|_| ())
    .map_err(storage_error)
}

fn insert_chunk(tx: &Transaction<'_>, chunk: &Chunk, ordinal: i64) -> Result<()> {
    let citation_json = citation_json(&chunk.citation)?;
    let chapter_id = chunk.chapter_id.as_ref().map(ChapterId::as_str);

    tx.execute(
        r#"
        INSERT INTO chunks (
            book_id, chunk_id, chapter_id, chapter_title,
            page_start, page_end, text, citation_json, ordinal
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
        "#,
        params![
            chunk.book_id.as_str(),
            chunk.chunk_id.as_str(),
            chapter_id,
            chunk.chapter_title,
            chunk.page_start.get(),
            chunk.page_end.get(),
            chunk.text,
            citation_json,
            ordinal
        ],
    )
    .map(|_| ())
    .map_err(storage_error)
}

fn insert_ingest_run(
    tx: &Transaction<'_>,
    metadata: &BookMetadata,
    status: &str,
    message: Option<&str>,
) -> Result<()> {
    tx.execute(
        r#"
        INSERT INTO ingest_runs (book_id, source_sha256, ingested_at, status, message)
        VALUES (?1, ?2, ?3, ?4, ?5)
        "#,
        params![
            metadata.book_id.as_str(),
            metadata.source_sha256,
            metadata.ingested_at,
            status,
            message
        ],
    )
    .map(|_| ())
    .map_err(storage_error)
}

fn book_from_row(row: &Row<'_>) -> Result<BookMetadata> {
    let page_count = u32_from_i64(row.get(4).map_err(storage_error)?, "page_count")?;
    let chapter_count = u32_from_i64(row.get(5).map_err(storage_error)?, "chapter_count")?;
    let chunk_count = u32_from_i64(row.get(6).map_err(storage_error)?, "chunk_count")?;

    Ok(BookMetadata {
        book_id: BookId::parse(row.get::<_, String>(0).map_err(storage_error)?)?,
        title: row.get(1).map_err(storage_error)?,
        author: row.get(2).map_err(storage_error)?,
        source_sha256: row.get(3).map_err(storage_error)?,
        page_count,
        chapter_count,
        chunk_count,
        ingested_at: row.get(7).map_err(storage_error)?,
    })
}

fn page_from_row(row: &Row<'_>) -> Result<Page> {
    let citation_json: String = row.get(3).map_err(storage_error)?;

    Ok(Page {
        book_id: BookId::parse(row.get::<_, String>(0).map_err(storage_error)?)?,
        page_number: PageNumber::new(u32_from_i64(
            row.get(1).map_err(storage_error)?,
            "page_number",
        )?)?,
        text: row.get(2).map_err(storage_error)?,
        citation: serde_json::from_str(&citation_json).map_err(storage_error)?,
    })
}

fn chapter_from_row(row: &Row<'_>) -> Result<Chapter> {
    Ok(Chapter {
        book_id: BookId::parse(row.get::<_, String>(0).map_err(storage_error)?)?,
        chapter_id: ChapterId::parse(row.get::<_, String>(1).map_err(storage_error)?)?,
        title: row.get(2).map_err(storage_error)?,
        page_start: PageNumber::new(u32_from_i64(
            row.get(3).map_err(storage_error)?,
            "page_start",
        )?)?,
        page_end: PageNumber::new(u32_from_i64(
            row.get(4).map_err(storage_error)?,
            "page_end",
        )?)?,
    })
}

fn chunk_from_row(row: &Row<'_>) -> Result<Chunk> {
    let chapter_id = row
        .get::<_, Option<String>>(2)
        .map_err(storage_error)?
        .map(ChapterId::parse)
        .transpose()?;
    let citation_json: String = row.get(7).map_err(storage_error)?;

    Ok(Chunk {
        book_id: BookId::parse(row.get::<_, String>(0).map_err(storage_error)?)?,
        chunk_id: ChunkId::parse(row.get::<_, String>(1).map_err(storage_error)?)?,
        chapter_id,
        chapter_title: row.get(3).map_err(storage_error)?,
        page_start: PageNumber::new(u32_from_i64(
            row.get(4).map_err(storage_error)?,
            "page_start",
        )?)?,
        page_end: PageNumber::new(u32_from_i64(
            row.get(5).map_err(storage_error)?,
            "page_end",
        )?)?,
        text: row.get(6).map_err(storage_error)?,
        citation: serde_json::from_str(&citation_json).map_err(storage_error)?,
    })
}

fn citation_json(citation: &Citation) -> Result<String> {
    serde_json::to_string(citation).map_err(storage_error)
}

fn u32_from_i64(value: i64, field: &'static str) -> Result<u32> {
    u32::try_from(value).map_err(|_| {
        BookMcpError::Storage(format!("stored {field} value {value} is outside u32 range"))
    })
}

fn usize_to_i64(value: usize, field: &'static str) -> Result<i64> {
    i64::try_from(value).map_err(|_| {
        BookMcpError::Storage(format!(
            "requested {field} value {value} is outside i64 range"
        ))
    })
}

fn not_found(entity: &'static str, id: &str) -> BookMcpError {
    BookMcpError::NotFound {
        entity,
        id: id.to_owned(),
    }
}

fn storage_error(error: impl fmt::Display) -> BookMcpError {
    BookMcpError::Storage(error.to_string())
}
