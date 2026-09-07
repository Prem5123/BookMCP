"""Package a tested native executable and generate an archive checksum."""

import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile


target = sys.argv[1]
if target not in (
    "x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu",
    "x86_64-apple-darwin", "aarch64-apple-darwin", "x86_64-pc-windows-msvc",
):
    raise SystemExit(f"Unsupported release target: {target}")
metadata = json.loads(subprocess.check_output([
    "cargo", "metadata", "--locked", "--filter-platform", target, "--format-version", "1"
]))
version = next(package["version"] for package in metadata["packages"] if package["name"] == "bookmcp-cli")
tag = os.environ.get("GITHUB_REF_NAME", "")
if os.environ.get("GITHUB_REF_TYPE") == "tag" and tag != f"v{version}":
    raise SystemExit(f"Release tag {tag!r} must match CLI version v{version}")
binary_name = "bookmcp.exe" if "windows" in target else "bookmcp"
binary = Path(metadata["target_directory"]) / target / "release" / binary_name
subprocess.run([str(binary.resolve()), "--version"], check=True)
name = f"bookmcp-v{version}-{target}"
Path("dist").mkdir(exist_ok=True)
with tempfile.TemporaryDirectory(prefix=".bookmcp-package-", dir="dist") as temporary:
    staging = Path(temporary) / name
    staging.mkdir(parents=True, exist_ok=True)
    shutil.copy2(binary, staging / binary_name)
    for filename in ["README.md", "CONTRIBUTING.md", "AGENTS.md", "LICENSE-MIT", "LICENSE-APACHE"]:
        shutil.copy2(filename, staging / filename)
    (staging / "docs").mkdir(exist_ok=True)
    for document in Path("docs").glob("*.md"):
        shutil.copy2(document, staging / "docs" / document.name)
    (staging / "docs" / "launch").mkdir(exist_ok=True)
    for filename in ("DEMO.md", "demo.sh"):
        shutil.copy2(Path("docs/launch") / filename, staging / "docs" / "launch" / filename)
    fixtures = staging / "tests" / "fixtures"
    fixtures.mkdir(parents=True, exist_ok=True)
    shutil.copy2("tests/fixtures/tiny.pdf", fixtures / "tiny.pdf")
    notices = []
    fallback_sources = json.loads(Path(".github/licenses/sources.json").read_text(encoding="utf-8"))
    for package in metadata["packages"]:
        if package["id"] in metadata["workspace_members"]:
            continue
        label = f'{package["name"]}-{package["version"]}'
        notices.append(f'{label}: {package.get("license") or "See bundled license file"}\nAuthors: {", ".join(package["authors"])}\n{package.get("repository") or package["source"]}\n')
        source = Path(package["manifest_path"]).parent
        licenses = {
            entry for entry in source.iterdir()
            if entry.is_file() and re.match(r"^(LICENSE|LICENCE|COPYING|NOTICE|COPYRIGHT)($|[._-])", entry.name, re.IGNORECASE)
        }
        if package.get("license_file"):
            licenses.add(source / package["license_file"])
        destination = staging / "third-party-licenses" / label
        destination.mkdir(parents=True, exist_ok=True)
        if not licenses:
            repository = (package.get("repository") or "").rstrip("/")
            fallback = fallback_sources.get(repository)
            if not fallback or package["name"] not in fallback["packages"]:
                raise SystemExit(f"Missing license text for {label}; add an audited .github/licenses fallback")
            if fallback["directory"]:
                licenses.update((Path(".github/licenses") / fallback["directory"]).iterdir())
            elif "MIT" in (package.get("license") or "").split():
                # These upstreams declare MIT but provide no standalone license text.
                # Preserve their manifest and authors, alongside the standard MIT terms.
                shutil.copy2(source / "Cargo.toml", destination / "UPSTREAM-Cargo.toml")
                attribution = f'Upstream package: {label}\nDeclared license: {package["license"]}\nAuthors: {", ".join(package["authors"])}\n\n'
                (destination / "LICENSE-MIT.txt").write_text(attribution + Path(".github/licenses/MIT.txt").read_text(encoding="utf-8"), encoding="utf-8")
            else:
                raise SystemExit(f"No supported fallback license for {label}")
            (destination / "SOURCE.json").write_text(json.dumps(fallback, indent=2) + "\n", encoding="utf-8")
        for license_file in licenses:
            shutil.copy2(license_file, destination / license_file.name)
    (staging / "THIRD-PARTY-NOTICES.txt").write_text("\n".join(notices), encoding="utf-8")
    archive_format = "zip" if "windows" in target else "gztar"
    archive = Path(shutil.make_archive(str(Path("dist") / name), archive_format, temporary, name))
    digest = hashlib.sha256(archive.read_bytes()).hexdigest()
    Path(f"{archive}.sha256").write_text(f"{digest}  {archive.name}\n", encoding="utf-8")
    print(archive)
