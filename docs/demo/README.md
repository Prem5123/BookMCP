# BookMCP demo

[bookmcp-demo.gif](bookmcp-demo.gif) is a silent, 45-second walkthrough. [poster.png](poster.png) is its still preview. The 1280×720 H.264 MP4 is rendered as `bookmcp-demo.mp4` and kept out of Git for release attachments.

The guide is original BookMCP demo content under **MIT OR Apache-2.0**, matching the repository licenses. [reliable-code.pdf](reliable-code.pdf) contains three pages and PDF bookmarks. Its text and generator are in `capture.py`.

The demonstration executes the actual CLI and two actual MCP stdio processes. It ingests the PDF, searches for idempotency, retrieves the page-2 chunk, saves a user-authored lesson through the CLI, then retrieves the same lesson from a fresh MCP process. Both processes must exit successfully. `doctor` must report a healthy library. [capture.json](capture.json) preserves the commands and JSON-RPC responses; only the random temporary-library path is redacted.

The presentation wraps text, collapses extraction whitespace, selects output fields, and shortens pauses for readability. It is a designed replay of captured output, not screen-recorded live agent interaction. It does not show an AI-generated answer or imply that MCP can save lessons. Book content stays local throughout capture and rendering. Renderer setup may download tooling, browser binaries, and fonts.

To capture again from the repository root, with Python 3.10+ and a built BookMCP binary:

```sh
cargo build --locked -p bookmcp-cli
python3 -m venv docs/demo/.venv
. docs/demo/.venv/bin/activate
python -m pip install -r docs/demo/requirements.txt
python docs/demo/capture.py
python docs/demo/build.py
```

On Windows, activate the virtual environment with `docs\demo\.venv\Scripts\Activate.ps1` and pass `--bookmcp target/debug/bookmcp.exe` to `capture.py`. The script uses an isolated temporary library and leaves your library untouched. Timestamps in a new capture will differ.

To render, install Node.js 22+, FFmpeg, and the pinned dependencies:

```sh
cd docs/demo
npm ci
npm run check
npm run dev
# After reviewing the local timeline, stop the preview and render:
npm run render
ffmpeg -y -i bookmcp-demo.mp4 -filter_complex "[0:v]fps=10,scale=960:-1:flags=lanczos,split[a][b];[a]palettegen=stats_mode=diff[p];[b][p]paletteuse=dither=bayer:bayer_scale=3" -loop 0 bookmcp-demo.gif
ffmpeg -y -ss 18 -i bookmcp-demo.mp4 -frames:v 1 poster.png
```

The delivered video was checked with HyperFrames' strict lint, runtime, layout, and contrast checks at 3, 9, 18, 29, and 40 seconds (zero findings), and all five rendered midpoint frames were visually reviewed. FFprobe verified 45 seconds, 1280×720 at 24 fps for MP4 and 960×540 with 450 frames for GIF. Keep output claims tied to `capture.json` when editing; regenerate the HTML with `build.py`.
