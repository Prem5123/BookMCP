use std::{io::Write, path::PathBuf};

use anyhow::Result;
use bookmcp_core::{BookId, ChunkId, LessonId};
use clap::Subcommand;

/// Explicit local lesson mutations and retrieval.
#[derive(Debug, Subcommand)]
pub enum LessonCommand {
    /// Save a reviewed lesson tied to a currently stored source chunk.
    Add {
        book_id: BookId,
        chunk_id: ChunkId,
        /// Short lesson title (up to 160 characters).
        #[arg(long)]
        title: String,
        /// Interpretation or actionable rule (up to 4,000 characters).
        #[arg(long)]
        body: String,
        #[arg(long)]
        data_dir: Option<PathBuf>,
        #[arg(long)]
        json: bool,
    },
    /// List saved lessons, including source citations and stale status.
    List {
        #[arg(long)]
        book_id: Option<BookId>,
        #[arg(long, default_value_t = 0)]
        offset: usize,
        #[arg(long, default_value_t = 20)]
        limit: usize,
        #[arg(long)]
        data_dir: Option<PathBuf>,
        #[arg(long)]
        json: bool,
    },
    /// Remove one lesson by its exact ID.
    Remove {
        lesson_id: LessonId,
        #[arg(long)]
        data_dir: Option<PathBuf>,
    },
}

pub fn run<W: Write>(writer: &mut W, command: LessonCommand) -> Result<()> {
    match command {
        LessonCommand::Add {
            book_id,
            chunk_id,
            title,
            body,
            data_dir,
            json,
        } => {
            let data_dir = super::resolve_data_dir(data_dir)?;
            let _writer_lock = super::lock_library(&data_dir)?;
            let mut store = super::open_store(Some(data_dir))?;
            let lesson = store.save_lesson(&book_id, &chunk_id, &title, &body)?;
            if json {
                super::write_json(writer, &lesson)?;
            } else {
                writeln!(
                    writer,
                    "saved {}: {}\n{}",
                    lesson.lesson_id,
                    lesson.title,
                    lesson.citation.format()
                )?;
            }
        }
        LessonCommand::List {
            book_id,
            offset,
            limit,
            data_dir,
            json,
        } => {
            let store = super::open_store(data_dir)?;
            let lessons = store.list_lessons(book_id.as_ref(), offset, limit)?;
            let total = store.count_lessons(book_id.as_ref())?;
            let next = offset.saturating_add(lessons.len());
            let next_offset = (next < total).then_some(next);
            if json {
                super::write_json(
                    writer,
                    &serde_json::json!({"lessons": lessons, "total": total, "next_offset": next_offset}),
                )?;
            } else {
                if lessons.is_empty() {
                    writeln!(writer, "No saved lessons.")?;
                }
                for lesson in lessons {
                    writeln!(
                        writer,
                        "{}\t{}{}\n{}\n{}",
                        lesson.lesson_id,
                        lesson.title,
                        if lesson.stale { " [stale source]" } else { "" },
                        lesson.body,
                        lesson.citation.format()
                    )?;
                }
                if let Some(next_offset) = next_offset {
                    writeln!(writer, "More lessons: use --offset {next_offset}")?;
                }
            }
        }
        LessonCommand::Remove {
            lesson_id,
            data_dir,
        } => {
            let data_dir = super::resolve_data_dir(data_dir)?;
            let _writer_lock = super::lock_library(&data_dir)?;
            super::open_store(Some(data_dir))?.delete_lesson(&lesson_id)?;
            writeln!(writer, "removed {lesson_id}")?;
        }
    }
    Ok(())
}
