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
