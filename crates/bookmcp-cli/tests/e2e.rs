use std::path::Path;

use bookmcp_cli::{Cli, run_with_writer};
use clap::Parser;
use tempfile::tempdir;

#[test]
fn fixture_ingest_list_search_page_chunk_and_rebuild_workflow() {
    let temp = tempdir().unwrap();
    let data_dir = temp.path().join("bookmcp-data");
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/tiny.pdf")
        .canonicalize()
        .unwrap();
    let data_dir_arg = data_dir.to_str().unwrap();
    let fixture_arg = fixture.to_str().unwrap();

    let ingest = run_command([
        "bookmcp",
        "ingest",
        fixture_arg,
        "--title",
        "Tiny Test Book",
        "--book-id",
        "tiny-test",
        "--data-dir",
        data_dir_arg,
    ]);
    assert!(ingest.contains("ingested `Tiny Test Book` as tiny-test"));
    assert!(data_dir.join("library").join("tiny-test.pdf").is_file());

    let list = run_command(["bookmcp", "list", "--data-dir", data_dir_arg]);
    assert!(list.contains("tiny-test"));
    assert!(list.contains("Tiny Test Book"));

    let search = run_command([
        "bookmcp",
        "search",
        "test",
        "--book-id",
        "tiny-test",
        "--data-dir",
        data_dir_arg,
    ]);
    assert!(search.contains("tiny-test-000001"));
    assert!(search.contains("Tiny Test Book by BookMCP Tests, p. 1"));

    let page = run_command([
        "bookmcp",
        "page",
        "tiny-test",
        "1",
        "--data-dir",
        data_dir_arg,
    ]);
    assert!(page.contains("This tiny PDF contains test concepts."));

    let chunk = run_command([
        "bookmcp",
        "chunk",
        "tiny-test",
        "tiny-test-000001",
        "--data-dir",
        data_dir_arg,
    ]);
    assert!(chunk.contains("Use citations for every answer."));

    let rebuild = run_command([
        "bookmcp",
        "rebuild-index",
        "--data-dir",
        data_dir_arg,
        "--book-id",
        "tiny-test",
    ]);
    assert_eq!(rebuild.trim(), "rebuilt keyword index with 1 chunks");
}

fn run_command<const N: usize>(args: [&str; N]) -> String {
    let cli = Cli::parse_from(args);
    let mut output = Vec::new();
    run_with_writer(cli, &mut output).unwrap();
    String::from_utf8(output).unwrap()
}

#[test]
fn duplicate_ingest_is_explicit_and_doctor_detects_source_or_index_damage() {
    let temp = tempdir().unwrap();
    let data_dir = temp.path().to_str().unwrap();
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/tiny.pdf");
    let fixture = fixture.to_str().unwrap();
    let args = [
        "bookmcp",
        "ingest",
        fixture,
        "--book-id",
        "check",
        "--data-dir",
        data_dir,
    ];
    run_command(args);
    let duplicate = run_with_writer(Cli::parse_from(args), &mut Vec::new()).unwrap_err();
    assert!(duplicate.to_string().contains("--force"));
    assert!(
        run_command(["bookmcp", "doctor", "--data-dir", data_dir, "--json"])
            .contains("\"status\": \"ok\"")
    );

    let index = bookmcp_index::IndexManager::create_or_open(temp.path().join("index")).unwrap();
    index.rebuild(&[]).unwrap();
    let damaged = run_with_writer(
        Cli::parse_from(["bookmcp", "doctor", "--data-dir", data_dir]),
        &mut Vec::new(),
    )
    .unwrap_err();
    assert!(damaged.to_string().contains("rebuild-index"));
    run_command(["bookmcp", "rebuild-index", "--data-dir", data_dir]);
    std::fs::write(temp.path().join("library/check.pdf"), b"changed source").unwrap();
    let damaged = run_with_writer(
        Cli::parse_from(["bookmcp", "doctor", "--data-dir", data_dir]),
        &mut Vec::new(),
    )
    .unwrap_err();
    assert!(damaged.to_string().contains("hash mismatch"));
}

#[test]
fn scoped_rebuild_rejects_unknown_book() {
    let temp = tempdir().unwrap();
    let error = run_with_writer(
        Cli::parse_from([
            "bookmcp",
            "rebuild-index",
            "--book-id",
            "absent",
            "--data-dir",
            temp.path().to_str().unwrap(),
        ]),
        &mut Vec::new(),
    )
    .unwrap_err();
    assert!(error.to_string().contains("book not found"));
}

#[test]
fn missing_and_corrupt_indexes_are_rebuilt_without_losing_other_books() {
    let temp = tempdir().unwrap();
    let data_dir = temp.path().to_str().unwrap();
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/tiny.pdf");
    for book in ["first", "second"] {
        run_command([
            "bookmcp",
            "ingest",
            fixture.to_str().unwrap(),
            "--book-id",
            book,
            "--data-dir",
            data_dir,
        ]);
    }
    std::fs::remove_dir_all(temp.path().join("index")).unwrap();
    let missing = run_with_writer(
        Cli::parse_from(["bookmcp", "search", "test", "--data-dir", data_dir]),
        &mut Vec::new(),
    )
    .unwrap_err();
    assert!(missing.to_string().contains("rebuild-index"));
    assert!(!temp.path().join("index").exists());
    run_command([
        "bookmcp",
        "rebuild-index",
        "--book-id",
        "first",
        "--data-dir",
        data_dir,
    ]);
    assert!(
        run_command([
            "bookmcp",
            "search",
            "test",
            "--book-id",
            "second",
            "--data-dir",
            data_dir
        ])
        .contains("second")
    );
    std::fs::write(temp.path().join("index/meta.json"), b"corrupt").unwrap();
    run_command(["bookmcp", "rebuild-index", "--data-dir", data_dir]);
    assert!(run_command(["bookmcp", "doctor", "--data-dir", data_dir]).contains("health: ok"));
    std::fs::remove_dir_all(temp.path().join("index")).unwrap();
    run_command([
        "bookmcp",
        "ingest",
        fixture.to_str().unwrap(),
        "--book-id",
        "third",
        "--data-dir",
        data_dir,
    ]);
    for book in ["first", "second", "third"] {
        assert!(
            run_command([
                "bookmcp",
                "search",
                "test",
                "--book-id",
                book,
                "--data-dir",
                data_dir
            ])
            .contains(book)
        );
    }
}

#[test]
fn competing_cli_writes_fail_explicitly_without_changing_the_library() {
    let temp = tempdir().unwrap();
    let data_dir = temp.path().to_str().unwrap();
    run_command(["bookmcp", "doctor", "--data-dir", data_dir]);
    let lock = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(temp.path().join(".bookmcp-writer.lock"))
        .unwrap();
    lock.lock().unwrap();
    let blocked = run_with_writer(
        Cli::parse_from(["bookmcp", "rebuild-index", "--data-dir", data_dir]),
        &mut Vec::new(),
    )
    .unwrap_err();
    assert!(blocked.to_string().contains("writer lock unavailable"));
    drop(lock);
    run_command(["bookmcp", "rebuild-index", "--data-dir", data_dir]);
}
