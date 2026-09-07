"""Exercise the actual extracted MCPB command, sample ingestion, and stdio RPC."""

import json
import os
from pathlib import Path
import platform
import queue
import stat
import subprocess
import sys
import tempfile
import threading
import zipfile


def main():
    with tempfile.TemporaryDirectory(prefix="bookmcp bundle smoke ") as temporary:
        root = Path(temporary)
        with zipfile.ZipFile(sys.argv[1]) as bundle:
            for member in bundle.infolist():
                destination = root / member.filename
                if not destination.resolve().is_relative_to(root.resolve()):
                    raise SystemExit(f"Unsafe MCPB entry: {member.filename}")
                bundle.extract(member, root)
                if os.name != "nt":
                    destination.chmod(stat.S_IMODE(member.external_attr >> 16))
        manifest = json.loads((root / "manifest.json").read_text(encoding="utf-8"))
        config = dict(manifest["server"]["mcp_config"])
        host = {"Darwin": "darwin", "Linux": "linux", "Windows": "win32"}[platform.system()]
        config.update(config.pop("platform_overrides", {}).get(host, {}))
        library = root / "test library"
        command = config["command"].replace("${__dirname}", str(root))
        arguments = [value.replace("${user_config.library_directory}", str(library)) for value in config["args"]]
        subprocess.run([command, "--version"], check=True, timeout=20)
        subprocess.run([
            command, "ingest", str(root / "tests/fixtures/tiny.pdf"),
            "--book-id", "bundle-test", "--data-dir", str(library),
        ], check=True, timeout=60)
        process = subprocess.Popen(
            [command, *arguments], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
            stderr=subprocess.PIPE, text=True, encoding="utf-8",
        )
        messages = queue.Queue()
        reader = threading.Thread(target=lambda: [messages.put(line) for line in process.stdout], daemon=True)
        reader.start()

        def send(message):
            process.stdin.write(json.dumps({"jsonrpc": "2.0", **message}) + "\n")
            process.stdin.flush()

        def receive(identifier):
            for _ in range(20):
                response = json.loads(messages.get(timeout=20))
                if response.get("id") == identifier:
                    if "error" in response:
                        raise RuntimeError(response)
                    return response["result"]
            raise RuntimeError(f"No response for request {identifier}")

        try:
            send({"id": 1, "method": "initialize", "params": {
                "protocolVersion": "2025-11-25", "capabilities": {},
                "clientInfo": {"name": "bookmcp-bundle-smoke", "version": "1.0.0"},
            }})
            receive(1)
            send({"method": "notifications/initialized"})
            send({"id": 2, "method": "tools/list"})
            names = {tool["name"] for tool in receive(2)["tools"]}
            if names != {tool["name"] for tool in manifest["tools"]}:
                raise RuntimeError("MCPB manifest tool list differs from the running server")
            send({"id": 3, "method": "tools/call", "params": {
                "name": "book_search", "arguments": {"query": "citations", "book_id": "bundle-test"},
            }})
            result = receive(3)
            if result.get("isError") or "bundle-test" not in json.dumps(result):
                raise RuntimeError(f"Bundled sample search failed: {result}")
            process.stdin.close()
            process.wait(timeout=20)
            if process.returncode:
                raise RuntimeError(process.stderr.read())
        finally:
            if process.poll() is None:
                process.kill()
                process.wait(timeout=10)
        print("MCPB passed: native launch, sample ingestion, declared tools, cited search, and clean shutdown")


if __name__ == "__main__":
    main()
