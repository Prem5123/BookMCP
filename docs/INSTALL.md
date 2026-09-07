# Install BookMCP

Download the archive for your operating system and processor from the [latest release](https://github.com/Prem5123/BookMCP/releases/latest). The matching `.sha256` file lets you check that the download is intact. Archives include the executable, documentation, original sample PDF, and dependency licenses.

## macOS and Linux

On macOS, choose `aarch64-apple-darwin` for Apple Silicon or `x86_64-apple-darwin` for Intel. On Linux, `uname -m` reports `x86_64` or `aarch64`; choose the corresponding Linux archive. The Linux binaries are built on Ubuntu 22.04 with glibc; Alpine/musl needs a source build.

Download both the `.tar.gz` and its `.tar.gz.sha256` file into the same folder. For example, on an Apple Silicon Mac:

```sh
cd ~/Downloads
shasum -a 256 -c bookmcp-v0.1.0-aarch64-apple-darwin.tar.gz.sha256
tar -xzf bookmcp-v0.1.0-aarch64-apple-darwin.tar.gz
cd bookmcp-v0.1.0-aarch64-apple-darwin
./bookmcp --version
```

Use your downloaded filename on other platforms. Linux also supports `sha256sum -c <checksum-file>`. Continue only when the checksum reports `OK`.

Install the executable in your user account:

```sh
mkdir -p "$HOME/.local/bin"
cp bookmcp "$HOME/.local/bin/bookmcp"
export PATH="$HOME/.local/bin:$PATH"
bookmcp --version
```

To preserve the path across terminals, add `export PATH="$HOME/.local/bin:$PATH"` to your shell configuration (`~/.zshrc` or `~/.bashrc`). The export above affects the current terminal only. For desktop MCP clients, use `bookmcp mcp-config codex` or `bookmcp mcp-config claude` to get absolute paths.

The first release is not Apple-notarized. If macOS blocks execution, follow [Apple's instructions for opening an app from an unidentified developer](https://support.apple.com/guide/mac-help/open-a-mac-app-from-an-unknown-developer-mh40616/mac) after checking the source and checksum, or build from source.

## Windows

Download the Windows x64 `.zip` and `.zip.sha256` files. Open PowerShell in your Downloads folder:

```powershell
$archive = 'bookmcp-v0.1.0-x86_64-pc-windows-msvc.zip'
$expected = (Get-Content "$archive.sha256").Split(' ')[0]
$actual = (Get-FileHash $archive -Algorithm SHA256).Hash
if ($actual -ne $expected) { throw 'Checksum mismatch. Download the archive again.' }
Expand-Archive $archive -DestinationPath .
Set-Location 'bookmcp-v0.1.0-x86_64-pc-windows-msvc'
.\bookmcp.exe --version
```

You can run the executable with its full path. To use `bookmcp` from any folder, move `bookmcp.exe` to a permanent location such as a `BookMCP` folder in your user profile, add that folder to your **user** `Path` through Windows Environment Variables, and open a new terminal. Run `bookmcp mcp-config codex` or `bookmcp mcp-config claude` for configuration that uses absolute paths.

## First successful search

From the extracted archive folder, with `bookmcp` on `PATH`:

```sh
bookmcp ingest tests/fixtures/tiny.pdf --book-id tiny-test --title "Tiny Test Book"
bookmcp search "citations" --book-id tiny-test
bookmcp page tiny-test 1
bookmcp doctor
```

You should see a search result with a page citation and a healthy library report. Use `./bookmcp` on macOS/Linux or `.\bookmcp.exe` on Windows if you skipped adding the executable to `PATH`.

Next, [connect Codex, Claude Code, or another MCP client](../README.md#connect-your-agent). The client launches the server automatically.

## MCP bundle for compatible desktop clients

The release also includes [bookmcp-v0.1.0.mcpb](https://github.com/Prem5123/BookMCP/releases/download/v0.1.0/bookmcp-v0.1.0.mcpb), a bundle containing all five native executables. Use it with a client that supports MCPB installation. Codex and Claude Code can use the CLI configuration steps in the [README](../README.md#connect-your-agent).

Ingest your PDFs with the CLI first, then run `bookmcp doctor` and note the data directory. During bundle installation, select that same folder for **BookMCP library directory**. The bundle exposes the read-only MCP interface; adding books and saving lessons still use the CLI. A missing or different directory gives the client a different library.

The bundle has its own `.sha256` checksum. It does not require a Rust or Python runtime. Supported architectures are macOS Intel/Apple Silicon, Linux x86-64/ARM64 with glibc, and Windows x86-64.

## Build from source

Install Rust 1.96+ with [rustup](https://rustup.rs/) and your platform's C/C++ build toolchain. Then:

```sh
git clone https://github.com/Prem5123/BookMCP.git
cd BookMCP
cargo install --locked --path crates/bookmcp-cli
bookmcp --version
```

SQLite is bundled. Installation does not require an API key, a Python runtime, or a separate database service.
