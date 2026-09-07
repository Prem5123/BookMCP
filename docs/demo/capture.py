#!/usr/bin/env python3
"""Create original demo material and capture real BookMCP CLI/MCP responses locally."""
import argparse
import json
import os
from pathlib import Path
import queue
import subprocess
import tempfile
import threading
import textwrap

from reportlab.pdfgen import canvas

HERE = Path(__file__).resolve().parent
GUIDE = "A Small Guide to Reliable Code"
PAGES = [
    ("Chapter 1: Make failure explicit", "An operation can fail even when its inputs look reasonable. "
     "Represent expected failures as typed errors. Add context at the application boundary so the "
     "person running the program can tell what failed and what to do next.\n\n"
     "Avoid returning an empty success when an operation never completed. Tests should exercise "
     "the failure path as well as the happy path. A clear error is more useful than a plausible result."),
    ("Chapter 2: Make retries safe", "A retry should not create the same effect twice. Use an "
     "idempotency key to recognize a repeated request before applying its side effect. Store the "
     "key and the result together so a later retry can return the original result.\n\n"
     "For example, a job submission can carry a stable request identifier. If the first response "
     "is lost, submitting the same identifier should find the existing job. Test the duplicate "
     "request explicitly; a retry loop alone does not make an operation safe."),
    ("Chapter 3: Keep evidence close", "A useful engineering note includes a claim, the reason "
     "it matters, and a source that another reader can inspect. Distinguish your interpretation "
     "from the source's exact words. A page reference lets a teammate verify the reasoning.\n\n"
     "When a source changes, review notes derived from it. A saved lesson is a working "
     "interpretation, not a substitute for testing the current implementation. Keep examples "
     "small enough that a reader can reproduce them."),
]


def make_pdf():
    pdf = canvas.Canvas(str(HERE / "reliable-code.pdf"), pagesize=(612, 792), invariant=1)
    pdf.setTitle(GUIDE)
    pdf.setAuthor("BookMCP contributors")
    for number, (heading, body) in enumerate(PAGES, 1):
        pdf.bookmarkPage(f"chapter-{number}")
        pdf.addOutlineEntry(heading, f"chapter-{number}", level=0)
        pdf.setFillColorRGB(.95, .93, .89)
        pdf.rect(0, 0, 612, 792, fill=1, stroke=0)
        pdf.setFillColorRGB(.09, .17, .14)
        pdf.setFont("Helvetica-Bold", 11)
        pdf.drawString(54, 738, GUIDE.upper())
        pdf.setFont("Helvetica-Bold", 22)
        pdf.drawString(54, 662, heading)
        text = pdf.beginText(54, 613)
        text.setFont("Helvetica", 13)
        text.setLeading(23)
        for paragraph in body.split("\n\n"):
            for line in textwrap.wrap(paragraph, width=70):
                text.textLine(line)
            text.textLine("")
        pdf.drawText(text)
        pdf.setFont("Helvetica", 9)
        pdf.drawString(54, 65, "Original BookMCP demo text. MIT OR Apache-2.0. No third-party book content.")
        pdf.drawRightString(558, 42, str(number))
        pdf.showPage()
    pdf.save()


class Mcp:
    def __init__(self, binary, env, transcript, session):
        self.log, self.session, self.counter = transcript, session, 0
        self.process = subprocess.Popen([binary, "serve"], env=env, stdin=subprocess.PIPE,
                                        stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
        self.lines = queue.Queue()
        threading.Thread(target=lambda: [self.lines.put(line) for line in self.process.stdout],
                         daemon=True).start()
        self.request("initialize", {"protocolVersion": "2025-11-25", "capabilities": {},
                     "clientInfo": {"name": "bookmcp-demo", "version": "1.0"}})
        self.send({"jsonrpc": "2.0", "method": "notifications/initialized"})

    def send(self, message):
        self.process.stdin.write(json.dumps(message) + "\n")
        self.process.stdin.flush()

    def request(self, method, params):
        self.counter += 1
        request = {"jsonrpc": "2.0", "id": self.counter, "method": method, "params": params}
        self.send(request)
        while True:
            response = json.loads(self.lines.get(timeout=20))
            if response.get("id") == self.counter:
                break
        if "error" in response or response.get("result", {}).get("isError"):
            raise RuntimeError(response)
        self.log.append({"session": self.session, "request": request, "response": response})
        return response["result"]

    def tool(self, name, arguments):
        result = self.request("tools/call", {"name": name, "arguments": arguments})
        return json.loads(result["content"][0]["text"])

    def close(self):
        self.process.stdin.close()
        try:
            status = self.process.wait(timeout=5)
            if status:
                raise RuntimeError(f"MCP exited {status}")
        finally:
            if self.process.poll() is None:
                self.process.kill()
                self.process.wait()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bookmcp", default=str(HERE.parents[1] / "target/debug/bookmcp"))
    args = parser.parse_args()
    binary = str(Path(args.bookmcp).resolve())
    make_pdf()
    log = {"disclosure": "Actual local commands and JSON-RPC responses; presentation uses excerpts.",
           "cli": [], "mcp": []}
    with tempfile.TemporaryDirectory(prefix="bookmcp-demo-") as temporary:
        env = dict(os.environ, BOOKMCP_HOME=temporary)

        def cli(arguments):
            result = subprocess.run([binary, *arguments], cwd=HERE, env=env, capture_output=True,
                                    text=True, check=True, timeout=45)
            log["cli"].append({"command": ["bookmcp", *arguments], "stdout": result.stdout,
                               "stderr": result.stderr, "exit_code": result.returncode})
            return result.stdout

        cli(["--version"])
        cli(["ingest", "reliable-code.pdf", "--book-id", "reliable-code"])
        cli(["search", "idempotency", "--book-id", "reliable-code"])
        first = Mcp(binary, env, log["mcp"], "before-saving")
        try:
            catalog = first.tool("book_get_library_index", {})
            found = first.tool("book_search", {"query": "idempotency", "book_id": "reliable-code"})
            chunk_id = found["results"][0]["chunk_id"]
            chunk = first.tool("book_get_chunk", {"book_id": "reliable-code", "chunk_id": chunk_id})
            assert catalog["total_books"] == 1
            assert chunk["chunk"]["citation"]["page_start"] == 2, chunk
        finally:
            first.close()
        cli(["chunk", "reliable-code", chunk_id])
        lesson = json.loads(cli(["lesson", "add", "reliable-code", chunk_id, "--title", "Safe retries",
                     "--body", "Use an idempotency key before retrying side effects.", "--json"]))
        fresh = Mcp(binary, env, log["mcp"], "fresh-session-after-saving")
        try:
            lessons = fresh.tool("book_list_lessons", {"book_id": "reliable-code"})
            assert lessons["lessons"][0]["lesson_id"] == lesson["lesson_id"]
            assert lessons["lessons"][0]["stale"] is False
        finally:
            fresh.close()
        cli(["doctor", "--json"])
    # Data-directory names are environment-specific; keep this committed transcript portable.
    payload = json.dumps(log, indent=2).replace(temporary, "<temporary-demo-library>")
    (HERE / "capture.json").write_text(payload + "\n")
    print("Captured ingestion, citation on page 2, saved lesson, and fresh MCP retrieval.")


if __name__ == "__main__":
    main()
