#!/usr/bin/env python3
"""Check that Android HNMOD ports are sourced from mods/official and wired to Options > Mods."""
from pathlib import Path
import re
import tomllib

ROOT = Path(__file__).resolve().parents[1]
CLIENT = ROOT / "crates/horizon-client/src/game/android_bundled.rs"
GAME = ROOT / "crates/horizon-client/src/game.rs"
MOBS = ROOT / "crates/horizon-client/src/mobs.rs"
RENDER = ROOT / "crates/horizon-client-render/src/lib.rs"


def check_path_attrs(source: Path) -> None:
    text = source.read_text(encoding="utf-8")
    paths = re.findall(r'^\s*#\[path\s*=\s*"([^"]+)"\]', text, re.MULTILINE)
    for relative in paths:
        target = (source.parent / relative).resolve()
        if not target.is_file():
            raise SystemExit(f"missing Rust path from {source.relative_to(ROOT)}: {relative}")


def main() -> None:
    check_path_attrs(CLIENT)
    check_path_attrs(GAME)
    check_path_attrs(MOBS)
    check_path_attrs(RENDER)

    client = CLIENT.read_text(encoding="utf-8")
    required_calls = (
        "accurate_block_placement_list::run",
        "connected_textures_list::run",
        "accurate_block_placement_settings::run",
        "connected_textures_settings::run",
        "accurate_block_placement_click::run",
        "connected_textures_click::run",
        "accurate_block_placement_frame::run",
        "connected_textures_frame::run",
    )
    for call in required_calls:
        if call not in client:
            raise SystemExit(f"missing Android hook registration: {call}")

    packages = {
        "accurate-block-placement": ("accurate_block_placement", "1.0.0"),
        "connected-textures": ("connected_textures", "0.1.7"),
        "punchy-moves": ("punchy_moves", "1.2.0"),
    }
    for folder, expected in packages.items():
        package = ROOT / "mods/official" / folder
        meta = tomllib.loads((package / "PORT.toml").read_text(encoding="utf-8"))
        if (meta.get("id"), meta.get("version")) != expected:
            raise SystemExit(f"wrong metadata in mods/official/{folder}/PORT.toml")
        if not (package / "README.md").is_file():
            raise SystemExit(f"missing package README: {folder}")
        source_dir = package / "src" if folder != "punchy-moves" else package / "mixins/punchy_moves"
        if not list(source_dir.glob("*.rs")):
            raise SystemExit(f"missing Rust source: {folder}")

    # Their actual IDs and descriptions are used to create independent in-game rows.
    abp = (ROOT / "mods/official/accurate-block-placement/src/list.rs").read_text(encoding="utf-8")
    ctm = (ROOT / "mods/official/connected-textures/src/list.rs").read_text(encoding="utf-8")
    if "accurate_block_placement" not in abp or "connected_textures" not in ctm:
        raise SystemExit("a port no longer has its own NativeMod entry")

    moves = (ROOT / "mods/official/punchy-moves/mixins/punchy_moves/runtime.rs").read_text(encoding="utf-8")
    if 'const NAME: &str = "Punchy Moves";' not in moves:
        raise SystemExit("Punchy Moves display name is not set")
    if "super::punchy_moves::listed" not in client:
        raise SystemExit("Punchy Moves is missing from Options > Mods registration")

    print("OK: official Android native source packages resolve and register separate Options > Mods rows")


if __name__ == "__main__":
    main()
