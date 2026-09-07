use std::{
    io::{BufRead, BufReader, Write},
    path::Path,
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc::{self, Receiver},
    time::Duration,
};

use serde_json::{Value, json};
use tempfile::tempdir;

struct McpProcess {
    child: Child,
    input: Option<ChildStdin>,
    lines: Receiver<String>,
    next_id: u64,
}

impl McpProcess {
    fn start(data_dir: &Path) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_bookmcp"))
            .args(["serve", "--data-dir"])
            .arg(data_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let input = child.stdin.take().unwrap();
        let output = child.stdout.take().unwrap();
        let (sender, lines) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(output).lines() {
                let Ok(line) = line else { break };
                if sender.send(line).is_err() {
                    break;
                }
            }
        });
        Self {
            child,
            input: Some(input),
            lines,
            next_id: 0,
        }
    }

    fn send(&mut self, message: Value) {
        let input = self.input.as_mut().unwrap();
        writeln!(input, "{message}").unwrap();
        input.flush().unwrap();
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        self.next_id += 1;
        let id = self.next_id;
        self.send(json!({"jsonrpc":"2.0", "id":id, "method":method, "params":params}));
        loop {
            let line = self
                .lines
                .recv_timeout(Duration::from_secs(20))
                .expect("MCP response before deadline");
            let response: Value =
                serde_json::from_str(&line).expect("stdout must contain JSON-RPC only");
            if response["id"] == id {
                return response;
            }
        }
    }

    fn tool(&mut self, name: &str, arguments: Value) -> Value {
        let response = self.request("tools/call", json!({"name":name,"arguments":arguments}));
        assert!(response.get("error").is_none(), "{response}");
        assert_ne!(response["result"]["isError"], true, "{response}");
        serde_json::from_str(response["result"]["content"][0]["text"].as_str().unwrap()).unwrap()
    }
    fn finish(mut self) {
        self.input.take();
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                assert!(
                    status.success(),
                    "MCP should exit successfully when the client disconnects"
                );
                return;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "MCP should stop after stdin closes"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

impl Drop for McpProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn cli(data_dir: &Path, arguments: &[&str]) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_bookmcp"))
        .args(arguments)
        .arg("--data-dir")
        .arg(data_dir)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    if arguments.contains(&"--json") {
        serde_json::from_slice(&output.stdout).unwrap()
    } else {
        Value::Null
    }
}

#[test]
fn actual_stdio_transport_supports_discovery_citations_prompts_and_live_lessons() {
    let temp = tempdir().unwrap();
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/tiny.pdf");
    cli(
        temp.path(),
        &[
            "ingest",
            fixture.to_str().unwrap(),
            "--book-id",
            "tiny-test",
            "--title",
            "Tiny Test Book",
        ],
    );
    let mut server = McpProcess::start(temp.path());
    let init = server.request("initialize", json!({"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"bookmcp-integration-test","version":"1.0"}}));
    assert_eq!(init["result"]["capabilities"]["tools"], json!({}));
    assert!(
        init["result"]["instructions"]
            .as_str()
            .unwrap()
            .contains("book_get_library_index")
    );
    server.send(json!({"jsonrpc":"2.0","method":"notifications/initialized"}));

    let tools = server.request("tools/list", json!({}));
    for tool in tools["result"]["tools"].as_array().unwrap() {
        assert_eq!(tool["annotations"]["readOnlyHint"], true, "{tool}");
        assert_eq!(tool["annotations"]["openWorldHint"], false, "{tool}");
    }
    let names: Vec<_> = tools["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|tool| tool["name"].as_str().unwrap())
        .collect();
    for expected in [
        "book_get_library_index",
        "book_search",
        "book_get_chunk",
        "book_list_lessons",
    ] {
        assert!(names.contains(&expected), "missing {expected}: {names:?}");
    }
    assert!(
        !names.iter().any(|name| name.contains("save")
            || name.contains("ingest")
            || name.contains("delete"))
    );

    let catalog = server.tool("book_get_library_index", json!({}));
    assert_eq!(catalog["total_books"], 1);
    let results = server.tool(
        "book_search",
        json!({"query":"citations","book_id":"tiny-test","top_k":1}),
    );
    let chunk_id = results["results"][0]["chunk_id"].as_str().unwrap();
    let chunk = server.tool(
        "book_get_chunk",
        json!({"book_id":"tiny-test","chunk_id":chunk_id}),
    );
    assert!(
        chunk["chunk"]["text"]
            .as_str()
            .unwrap()
            .contains("Use citations for every answer.")
    );
    assert_eq!(chunk["chunk"]["citation"]["page_start"], 1);

    let prompt = server.request("prompts/get", json!({"name":"ask_book_with_citations","arguments":{"question":"How should I cite this book?","book_id":"tiny-test"}}));
    assert!(
        prompt["result"]["messages"][0]["content"]["text"]
            .as_str()
            .unwrap()
            .contains("How should I cite this book?")
    );
    let missing = server.request("prompts/get", json!({"name":"ask_book_with_citations"}));
    assert_eq!(missing["error"]["code"], -32602);
    let resource = server.request("resources/read", json!({"uri":"bookmcp://library"}));
    assert!(
        resource["result"]["contents"][0]["text"]
            .as_str()
            .unwrap()
            .contains("tiny-test")
    );
    for arguments in [
        json!({"book_id":"../bad","page_number":1}),
        json!({"book_id":"tiny-test","page_number":0}),
    ] {
        let invalid = server.request(
            "tools/call",
            json!({"name":"book_get_page","arguments":arguments}),
        );
        assert!(
            invalid.get("error").is_some() || invalid["result"]["isError"] == true,
            "{invalid}"
        );
    }
    let zero = server.request("tools/call", json!({"name":"book_get_page","arguments":{"book_id":"tiny-test","page_number":1,"max_chars":0}}));
    assert_eq!(zero["error"]["code"], -32602);
    let absent = server.request(
        "tools/call",
        json!({"name":"book_get_page","arguments":{"book_id":"absent","page_number":1}}),
    );
    assert_eq!(absent["result"]["isError"], true);
    let absent_resource = server.request("resources/read", json!({"uri":"book://absent/page/1"}));
    assert_eq!(absent_resource["error"]["code"], -32002);

    let lesson = cli(
        temp.path(),
        &[
            "lesson",
            "add",
            "tiny-test",
            chunk_id,
            "--title",
            "Cite evidence",
            "--body",
            "Attach page citations when using a book passage.",
            "--json",
        ],
    );
    let lessons = server.tool("book_list_lessons", json!({"book_id":"tiny-test"}));
    assert_eq!(lessons["lessons"][0]["lesson_id"], lesson["lesson_id"]);
    assert_eq!(lessons["lessons"][0]["stale"], false);
    cli(
        temp.path(),
        &["lesson", "remove", lesson["lesson_id"].as_str().unwrap()],
    );
    assert_eq!(server.tool("book_list_lessons", json!({}))["total"], 0);

    // The same running server observes a new book and a scoped rebuild retains both books.
    cli(
        temp.path(),
        &[
            "ingest",
            fixture.to_str().unwrap(),
            "--book-id",
            "second",
            "--title",
            "Second Book",
        ],
    );
    cli(temp.path(), &["rebuild-index", "--book-id", "tiny-test"]);
    assert_eq!(
        server.tool("book_get_library_index", json!({}))["total_books"],
        2
    );
    let found = server.tool(
        "book_search",
        json!({"query":"citations","book_id":"second"}),
    );
    assert_eq!(found["results"][0]["book_id"], "second");

    server.finish();
    std::fs::remove_dir_all(temp.path().join("index")).unwrap();
    let mut recovered = McpProcess::start(temp.path());
    let init = recovered.request("initialize", json!({"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"recovery-test","version":"1"}}));
    assert!(init.get("error").is_none(), "{init}");
    recovered.send(json!({"jsonrpc":"2.0","method":"notifications/initialized"}));
    let found = recovered.tool(
        "book_search",
        json!({"query":"citations","book_id":"second"}),
    );
    assert_eq!(found["results"][0]["book_id"], "second");
    recovered.finish();
}

#[test]
fn empty_library_initializes_and_mcp_reports_empty_catalog() {
    let temp = tempdir().unwrap();
    let mut server = McpProcess::start(&temp.path().join("new-library"));
    let init = server.request("initialize", json!({"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"test","version":"1"}}));
    assert!(init.get("error").is_none(), "{init}");
    server.send(json!({"jsonrpc":"2.0","method":"notifications/initialized"}));
    assert_eq!(
        server.tool("book_get_library_index", json!({}))["total_books"],
        0
    );
    server.finish();
}

#[test]
fn generated_client_configuration_preserves_paths_with_spaces_and_does_not_create_library() {
    let temp = tempdir().unwrap();
    let data_dir = temp.path().join("library with spaces");
    let output = Command::new(env!("CARGO_BIN_EXE_bookmcp"))
        .args(["mcp-config", "claude", "--data-dir"])
        .arg(&data_dir)
        .output()
        .unwrap();
    assert!(output.status.success());
    let config: Value = serde_json::from_slice(&output.stdout).unwrap();
    let server = &config["mcpServers"]["bookmcp"];
    assert!(Path::new(server["command"].as_str().unwrap()).is_absolute());
    assert_eq!(server["args"], json!(["serve", "--data-dir", data_dir]));
    assert!(!data_dir.exists());
    let output = Command::new(env!("CARGO_BIN_EXE_bookmcp"))
        .args(["mcp-config", "codex", "--data-dir"])
        .arg(&data_dir)
        .output()
        .unwrap();
    assert!(output.status.success());
    let toml = String::from_utf8(output.stdout).unwrap();
    assert!(toml.starts_with("[mcp_servers.bookmcp]\n"));
    let args = toml
        .lines()
        .find_map(|line| line.strip_prefix("args = "))
        .unwrap();
    assert_eq!(serde_json::from_str::<Value>(args).unwrap(), server["args"]);
}
