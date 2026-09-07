use bookmcp_core::{BookId, BookMcpError, ChunkId, Lesson, LessonId, Result};
use rusqlite::{OptionalExtension, Row, TransactionBehavior, params};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use crate::{BookStore, not_found, storage_error, usize_to_i64};

const MAX_LESSONS_PER_PAGE: usize = 100;
const MAX_LESSON_TITLE_CHARS: usize = 160;
const MAX_LESSON_BODY_CHARS: usize = 4_000;

impl BookStore {
    /// Save a user-authored lesson with citation and source version taken from the library.
    pub fn save_lesson(
        &mut self,
        book_id: &BookId,
        chunk_id: &ChunkId,
        title: &str,
        body: &str,
    ) -> Result<Lesson> {
        self.ensure_writable()?;
        let title = validated_text("title", title, MAX_LESSON_TITLE_CHARS)?;
        let body = validated_text("body", body, MAX_LESSON_BODY_CHARS)?;
        let created_at = OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .map_err(storage_error)?;

        // The INSERT SELECT captures the chunk and book version in one SQLite snapshot.
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        let captured = tx.query_row(
            "INSERT INTO lessons (book_id, chunk_id, source_sha256, title, body, citation_json, created_at)
             SELECT b.book_id, c.chunk_id, b.source_sha256, ?3, ?4, c.citation_json, ?5
             FROM books b JOIN chunks c ON c.book_id = b.book_id
             WHERE b.book_id = ?1 AND c.chunk_id = ?2
             RETURNING id, source_sha256, citation_json",
            params![book_id.as_str(), chunk_id.as_str(), title, body, created_at],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?)),
        ).optional().map_err(storage_error)?;
        let (id, source_sha256, citation_json) =
            captured.ok_or_else(|| not_found("chunk", &format!("{book_id}:{chunk_id}")))?;

        let lesson = Lesson {
            lesson_id: lesson_id(id)?,
            book_id: book_id.clone(),
            chunk_id: chunk_id.clone(),
            source_sha256,
            title: title.to_owned(),
            body: body.to_owned(),
            citation: serde_json::from_str(&citation_json).map_err(storage_error)?,
            created_at,
            stale: false,
        };
        if lesson.citation.book_id != *book_id {
            return Err(BookMcpError::Storage(
                "source citation belongs to a different book".to_owned(),
            ));
        }
        tx.commit().map_err(storage_error)?;
        Ok(lesson)
    }

    /// List saved lessons in capture order with a bounded page size.
    /// Lessons whose source has changed or disappeared are explicitly marked stale.
    pub fn list_lessons(
        &self,
        book_id: Option<&BookId>,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<Lesson>> {
        if limit == 0 || limit > MAX_LESSONS_PER_PAGE {
            return Err(BookMcpError::InvalidLimit {
                value: limit,
                max: MAX_LESSONS_PER_PAGE,
            });
        }
        let offset = usize_to_i64(offset, "lesson offset")?;
        let limit = usize_to_i64(limit, "lesson limit")?;
        if !self.lessons_available()? {
            return Ok(Vec::new());
        }
        let mut stmt = self.conn.prepare(
            "SELECT l.id, l.book_id, l.chunk_id, l.source_sha256, l.title, l.body,
                    l.citation_json, l.created_at,
                    (b.source_sha256 IS NULL OR b.source_sha256 != l.source_sha256 OR c.chunk_id IS NULL)
             FROM lessons l
             LEFT JOIN books b ON b.book_id = l.book_id
             LEFT JOIN chunks c ON c.book_id = l.book_id AND c.chunk_id = l.chunk_id
             WHERE (?1 IS NULL OR l.book_id = ?1)
             ORDER BY l.id LIMIT ?2 OFFSET ?3"
        ).map_err(storage_error)?;
        let mut rows = stmt
            .query(params![book_id.map(BookId::as_str), limit, offset])
            .map_err(storage_error)?;
        let mut lessons = Vec::new();
        while let Some(row) = rows.next().map_err(storage_error)? {
            lessons.push(lesson_from_row(row)?);
        }
        Ok(lessons)
    }

    /// Count all lessons or only the lessons associated with one book.
    pub fn count_lessons(&self, book_id: Option<&BookId>) -> Result<usize> {
        if !self.lessons_available()? {
            return Ok(0);
        }
        let count: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM lessons WHERE (?1 IS NULL OR book_id = ?1)",
                params![book_id.map(BookId::as_str)],
                |row| row.get(0),
            )
            .map_err(storage_error)?;
        usize::try_from(count).map_err(storage_error)
    }

    /// Delete a saved lesson by its generated identifier.
    pub fn delete_lesson(&mut self, lesson_id: &LessonId) -> Result<()> {
        self.ensure_writable()?;
        let raw_id = lesson_id
            .as_str()
            .strip_prefix("lesson-")
            .and_then(|value| value.parse::<i64>().ok())
            .filter(|id| *id > 0 && format!("lesson-{id}") == lesson_id.as_str())
            .ok_or_else(|| not_found("lesson", lesson_id.as_str()))?;
        let changed = self
            .conn
            .execute("DELETE FROM lessons WHERE id = ?1", params![raw_id])
            .map_err(storage_error)?;
        if changed == 0 {
            return Err(not_found("lesson", lesson_id.as_str()));
        }
        Ok(())
    }

    fn lessons_available(&self) -> Result<bool> {
        if self.has_lessons {
            return Ok(true);
        }
        let exists = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'lessons')",
            [], |row| row.get::<_, bool>(0),
        ).map_err(storage_error)?;
        if exists {
            return Ok(true);
        }
        let version: i64 = self
            .conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
                [],
                |row| row.get(0),
            )
            .map_err(storage_error)?;
        if version != 1 {
            return Err(BookMcpError::Storage(
                "library lessons table is missing".to_owned(),
            ));
        }
        Ok(false)
    }
}

fn validated_text<'a>(name: &'static str, value: &'a str, max: usize) -> Result<&'a str> {
    let trimmed = value.trim();
    let length = trimmed.chars().count();
    if length == 0 || length > max || trimmed.contains('\0') {
        return Err(BookMcpError::InvalidArgument {
            name,
            reason: format!("must contain 1 to {max} characters without null bytes"),
        });
    }
    Ok(trimmed)
}

fn lesson_id(id: i64) -> Result<LessonId> {
    if id <= 0 {
        return Err(BookMcpError::Storage(
            "invalid stored lesson identifier".to_owned(),
        ));
    }
    LessonId::parse(format!("lesson-{id}"))
}

fn lesson_from_row(row: &Row<'_>) -> Result<Lesson> {
    let citation_json: String = row.get(6).map_err(storage_error)?;
    let lesson = Lesson {
        lesson_id: lesson_id(row.get(0).map_err(storage_error)?)?,
        book_id: BookId::parse(row.get::<_, String>(1).map_err(storage_error)?)?,
        chunk_id: ChunkId::parse(row.get::<_, String>(2).map_err(storage_error)?)?,
        source_sha256: row.get(3).map_err(storage_error)?,
        title: row.get(4).map_err(storage_error)?,
        body: row.get(5).map_err(storage_error)?,
        citation: serde_json::from_str(&citation_json).map_err(storage_error)?,
        created_at: row.get(7).map_err(storage_error)?,
        stale: row.get(8).map_err(storage_error)?,
    };
    if lesson.citation.book_id != lesson.book_id {
        return Err(BookMcpError::Storage(
            "lesson citation belongs to a different book".to_owned(),
        ));
    }
    Ok(lesson)
}
