"""Validate the release metadata and its MCPB before Registry publication."""

import hashlib
import json
from pathlib import Path
import re
import sys
import zipfile


def main():
    tag = sys.argv[1]
    if not re.fullmatch(r"v[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?", tag):
        raise SystemExit("Expected a semantic release tag such as v0.1.0")
    metadata = json.loads(Path("dist/server.json").read_text(encoding="utf-8"))
    if metadata["name"] != "io.github.Prem5123/bookmcp" or metadata["version"] != tag[1:]:
        raise SystemExit("Registry name/version does not match the requested release")
    expected_name = f"bookmcp-{tag}.mcpb"
    expected_url = f"https://github.com/Prem5123/BookMCP/releases/download/{tag}/{expected_name}"
    packages = metadata.get("packages", [])
    if len(packages) != 1:
        raise SystemExit("Expected one portable MCPB package")
    package = packages[0]
    if (package["registryType"], package["identifier"], package["version"], package["transport"]) != (
        "mcpb", expected_url, tag[1:], {"type": "stdio"},
    ):
        raise SystemExit("Unexpected Registry package URL, version, type, or transport")
    archive = Path("dist") / expected_name
    digest = hashlib.sha256(archive.read_bytes()).hexdigest()
    if package["fileSha256"] != digest:
        raise SystemExit("MCPB hash does not match server.json")
    if Path(f"{archive}.sha256").read_text(encoding="utf-8").split() != [digest, expected_name]:
        raise SystemExit("MCPB checksum sidecar mismatch")
    with zipfile.ZipFile(archive) as bundle:
        manifest = json.loads(bundle.read("manifest.json"))
        if manifest["name"] != "bookmcp" or manifest["version"] != tag[1:]:
            raise SystemExit("MCPB manifest does not match the release")
        for path in (
            "server/bookmcp",
            "server/x86_64-unknown-linux-gnu/bookmcp",
            "server/aarch64-unknown-linux-gnu/bookmcp",
            "server/x86_64-apple-darwin/bookmcp",
            "server/aarch64-apple-darwin/bookmcp",
            "server/x86_64-pc-windows-msvc/bookmcp.exe",
        ):
            member = bundle.getinfo(path)
            if not member.file_size or not (member.external_attr >> 16) & 0o111:
                raise SystemExit(f"Missing or non-executable MCPB entry: {path}")
    print(f"Verified {metadata['name']} {metadata['version']} and MCPB SHA-256 {digest}")


if __name__ == "__main__":
    main()
