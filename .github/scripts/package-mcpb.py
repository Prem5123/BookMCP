"""Combine the five checksum-verified native releases into one portable MCPB."""

import hashlib
import json
from pathlib import Path, PurePosixPath
import re
import stat
import tarfile
import zipfile


TARGETS = (
    "x86_64-unknown-linux-gnu",
    "aarch64-unknown-linux-gnu",
    "x86_64-apple-darwin",
    "aarch64-apple-darwin",
    "x86_64-pc-windows-msvc",
)


def add_file(bundle, name, content, executable=False):
    info = zipfile.ZipInfo(name, date_time=(1980, 1, 1, 0, 0, 0))
    info.create_system = 3
    info.external_attr = (stat.S_IFREG | (0o755 if executable else 0o644)) << 16
    info.compress_type = zipfile.ZIP_DEFLATED
    bundle.writestr(info, content)


def archive_files(path):
    if path.suffix == ".zip":
        with zipfile.ZipFile(path) as archive:
            for member in archive.infolist():
                if not member.is_dir():
                    yield member.filename, archive.read(member)
    else:
        with tarfile.open(path, "r:gz") as archive:
            for member in archive.getmembers():
                if member.isfile():
                    stream = archive.extractfile(member)
                    if stream is None:
                        raise SystemExit(f"Cannot read {member.name} in {path}")
                    yield member.name, stream.read()
                elif not member.isdir():
                    raise SystemExit(f"Unsupported archive entry: {member.name}")


def main():
    metadata = json.loads(Path("server.json").read_text(encoding="utf-8"))
    manifest = json.loads(Path(".github/mcpb/manifest.json").read_text(encoding="utf-8"))
    version = metadata["version"]
    if not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?", version):
        raise SystemExit("server.json must have a concrete semantic version")
    if manifest["version"] != version:
        raise SystemExit("MCPB manifest and server.json versions must match")
    repository = metadata["repository"]["url"]
    if repository != "https://github.com/Prem5123/BookMCP":
        raise SystemExit("Review the release URL before changing the repository")
    output = Path("dist") / f"bookmcp-v{version}.mcpb"
    native_packages = []
    payloads = {}
    for target in TARGETS:
        prefix = f"bookmcp-v{version}-{target}"
        suffix = ".zip" if "windows" in target else ".tar.gz"
        archive = Path("dist") / f"{prefix}{suffix}"
        checksum = Path(f"{archive}.sha256").read_text(encoding="utf-8").split()
        digest = hashlib.sha256(archive.read_bytes()).hexdigest()
        if checksum != [digest, archive.name]:
            raise SystemExit(f"Checksum verification failed for {archive}")
        native_packages.append({"target": target, "filename": archive.name, "sha256": digest})
        binary_name = "bookmcp.exe" if "windows" in target else "bookmcp"
        found_binary = False
        for filename, content in archive_files(archive):
            parts = PurePosixPath(filename).parts
            if not parts or parts[0] != prefix or ".." in parts or "\\" in filename:
                raise SystemExit(f"Unexpected archive path: {filename}")
            relative = "/".join(parts[1:])
            executable = relative == binary_name
            if executable:
                destination = f"server/{target}/{binary_name}"
                found_binary = True
            elif relative.startswith("third-party-licenses/") or relative == "THIRD-PARTY-NOTICES.txt":
                destination = f"licenses/{target}/{relative}"
            elif target == TARGETS[0]:
                # Common documentation and the original sample PDF are included once.
                destination = relative
            else:
                continue
            if not destination or destination in payloads:
                raise SystemExit(f"Duplicate or empty bundle entry: {destination}")
            payloads[destination] = (content, executable)
        if not found_binary:
            raise SystemExit(f"Native executable missing from {archive}")
    payloads["manifest.json"] = ((json.dumps(manifest, indent=2) + "\n").encode(), False)
    payloads["server/bookmcp"] = (Path(".github/mcpb/bookmcp").read_bytes(), True)
    payloads["native-packages.json"] = ((json.dumps(native_packages, indent=2) + "\n").encode(), False)
    with zipfile.ZipFile(output, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=9) as bundle:
        for filename in sorted(payloads):
            content, executable = payloads[filename]
            add_file(bundle, filename, content, executable)
    digest = hashlib.sha256(output.read_bytes()).hexdigest()
    Path(f"{output}.sha256").write_text(f"{digest}  {output.name}\n", encoding="utf-8")
    metadata["packages"] = [{
        "registryType": "mcpb",
        "identifier": f"{repository}/releases/download/v{version}/{output.name}",
        "version": version,
        "fileSha256": digest,
        "transport": {"type": "stdio"},
    }]
    Path("dist/server.json").write_text(json.dumps(metadata, indent=2) + "\n", encoding="utf-8")
    print(f"Created {output} and dist/server.json from five verified native archives")


if __name__ == "__main__":
    main()
