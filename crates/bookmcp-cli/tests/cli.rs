use std::path::PathBuf;

use bookmcp_cli::{Cli, Commands, Transport, run_with_writer};
use clap::{CommandFactory, Parser};
use tempfile::tempdir;

#[test]
fn clap_command_shape_is_valid() {
    Cli::command().debug_assert();
}

#[test]
fn parses_ingest_command_with_overrides() {
    let cli = Cli::parse_from([
        "bookmcp",
        "ingest",
        "tests/fixtures/tiny.pdf",
        "--title",
        "Tiny Test Book",
        "--author",
        "BookMCP Tests",
        "--book-id",
        "tiny-test",
        "--data-dir",
        "./tmp/bookmcp-test",
        "--force",
    ]);

    let Commands::Ingest {
        pdf_path,
        title,
        author,
        book_id,
        data_dir,
        force,
    } = cli.command
    else {
        panic!("expected ingest command");
    };

    assert_eq!(pdf_path, PathBuf::from("tests/fixtures/tiny.pdf"));
    assert_eq!(title.as_deref(), Some("Tiny Test Book"));
    assert_eq!(author.as_deref(), Some("BookMCP Tests"));
    assert_eq!(book_id.unwrap().as_str(), "tiny-test");
    assert_eq!(data_dir.unwrap(), PathBuf::from("./tmp/bookmcp-test"));
    assert!(force);
}

#[test]
fn parses_serve_stdio_transport() {
    let cli = Cli::parse_from(["bookmcp", "serve", "--transport", "stdio"]);

    let Commands::Serve { transport, .. } = cli.command else {
        panic!("expected serve command");
    };

    assert_eq!(transport, Transport::Stdio);
}

#[test]
fn doctor_initializes_store_and_reports_paths() {
    let temp = tempdir().unwrap();
    let cli = Cli::parse_from([
        "bookmcp",
        "doctor",
        "--data-dir",
        temp.path().to_str().unwrap(),
    ]);
    let mut output = Vec::new();

    run_with_writer(cli, &mut output).unwrap();

    let rendered = String::from_utf8(output).unwrap();
    assert!(rendered.contains("BookMCP doctor"));
    assert!(rendered.contains("Data directory"));
    assert!(rendered.contains("Database"));
    assert!(rendered.contains("Status"));
    assert!(rendered.contains("ready"));
    assert!(temp.path().join("bookmcp.sqlite3").is_file());
}
