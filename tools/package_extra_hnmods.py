#!/usr/bin/env python3
"""Create deterministic desktop .hnmod archives from pinned native-mod submodules."""
from __future__ import annotations

import argparse
import hashlib
import json
import shutil
import subprocess
import tomllib
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BASE = ROOT / "mods" / "official" / "hnmods"
PACKAGES = (
    "bridgingmod-horizon",
    "continuity-horizon",
    "skinlayers3d-horizon",
    "horizon-sodium",
)


def run(*args: str, cwd: Path | None = None) -> None:
    subprocess.run(args, cwd=cwd, check=True)


def generic_package(source: Path, output: Path) -> Path:
    metadata_path = source / "horizon-native.json"
    metadata = json.loads(metadata_path.read_text(encoding="utf-8"))
    mod_id = metadata["id"]
    version = metadata["version"]
    manifests = list((source / "mixins").glob("*/horizon-mixin.toml"))
    if len(manifests) != 1:
        raise SystemExit(f"{source.name}: expected one mixins/*/horizon-mixin.toml")
    mixin = tomllib.loads(manifests[0].read_text(encoding="utf-8"))
    if mixin["mod"]["id"] != mod_id or mixin["mod"]["version"] != version:
        raise SystemExit(f"{source.name}: native manifest and mixin metadata disagree")
    pin = mixin.get("compatibility", {}).get("horizon_revision")
    if pin and metadata.get("horizon") != pin:
        raise SystemExit(f"{source.name}: Horizon revision mismatch")
    files = {"horizon-native.json": metadata_path.read_bytes()}
    for path in sorted((source / "mixins").rglob("*")):
        if path.is_file():
            files[path.relative_to(source).as_posix()] = path.read_bytes()
    if not any(name.endswith("horizon-mixin.toml") for name in files):
        raise SystemExit(f"{source.name}: missing native mixin manifest")
    output.mkdir(parents=True, exist_ok=True)
    dest = output / f"{mod_id}-{version}.hnmod"
    with zipfile.ZipFile(dest, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=9) as z:
        for name, content in sorted(files.items()):
            info = zipfile.ZipInfo(name, (2026, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            info.external_attr = 0o100644 << 16
            z.writestr(info, content, compress_type=zipfile.ZIP_DEFLATED, compresslevel=9)
    return dest


def package_one(name: str, output: Path) -> Path:
    source = BASE / name
    if not (source / ".git").exists() and not (source / ".git").is_file():
        # GitHub Actions checks out submodules as repositories. Local users must
        # initialise them explicitly before packaging.
        raise SystemExit(f"Missing submodule {source}; run git submodule update --init --recursive")
    if name == "continuity-horizon":
        run("python3", "tools/package.py", cwd=source)
        candidates = sorted((source / "target").glob("*.hnmod"))
        if not candidates:
            raise SystemExit("Continuity packager produced no .hnmod")
        chosen = candidates[-1]
        output.mkdir(parents=True, exist_ok=True)
        dest = output / chosen.name
        shutil.copy2(chosen, dest)
        return dest
    if name == "horizon-sodium":
        run("python3", "tools/package.py", cwd=source)
        candidates = sorted((source / "dist").glob("*.hnmod"))
        if not candidates:
            raise SystemExit("Horizon Sodium packager produced no .hnmod")
        chosen = candidates[-1]
        output.mkdir(parents=True, exist_ok=True)
        dest = output / chosen.name
        shutil.copy2(chosen, dest)
        return dest
    return generic_package(source, output)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=ROOT / ".local" / "extra-hnmods")
    args = parser.parse_args()
    results = []
    for name in PACKAGES:
        artifact = package_one(name, args.output)
        results.append({
            "file": artifact.name,
            "bytes": artifact.stat().st_size,
            "sha256": hashlib.sha256(artifact.read_bytes()).hexdigest(),
        })
    print(json.dumps({"built": results}, indent=2))


if __name__ == "__main__":
    main()
