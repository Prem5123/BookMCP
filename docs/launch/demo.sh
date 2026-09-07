#!/bin/sh
set -eu

# Run from a checkout or an extracted release archive. Every write goes to a
# freshly created temporary library that remains available for inspection.
bookmcp_demo_root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
bookmcp_demo_binary=${BOOKMCP_BIN:-bookmcp}
if ! command -v "$bookmcp_demo_binary" >/dev/null 2>&1; then
    printf '%s\n' 'Install bookmcp first, or set BOOKMCP_BIN to its executable path.' >&2
    exit 1
fi
bookmcp_demo_library=$(mktemp -d "${TMPDIR:-/tmp}/bookmcp-demo.XXXXXX")

printf 'Demo library: %s\n' "$bookmcp_demo_library"
"$bookmcp_demo_binary" --version
"$bookmcp_demo_binary" ingest "$bookmcp_demo_root/tests/fixtures/tiny.pdf" \
    --book-id tiny-test --title 'Tiny Test Book' --data-dir "$bookmcp_demo_library"
"$bookmcp_demo_binary" context --data-dir "$bookmcp_demo_library"
"$bookmcp_demo_binary" search citations --book-id tiny-test --top-k 3 \
    --data-dir "$bookmcp_demo_library"
"$bookmcp_demo_binary" chunk tiny-test tiny-test-000001 \
    --data-dir "$bookmcp_demo_library"
"$bookmcp_demo_binary" page tiny-test 1 --data-dir "$bookmcp_demo_library"
"$bookmcp_demo_binary" lesson add tiny-test tiny-test-000001 \
    --title 'Cite the evidence' \
    --body 'Attach a source page citation when applying advice from a book.' \
    --data-dir "$bookmcp_demo_library"
"$bookmcp_demo_binary" lesson list --book-id tiny-test --data-dir "$bookmcp_demo_library"
"$bookmcp_demo_binary" search cobaltzeppelin --book-id tiny-test \
    --data-dir "$bookmcp_demo_library"
"$bookmcp_demo_binary" doctor --data-dir "$bookmcp_demo_library"

printf '\nLibrary retained at: %s\n' "$bookmcp_demo_library"
printf '%s\n' 'To connect this library, use this same absolute path with mcp-config or serve --data-dir.'
