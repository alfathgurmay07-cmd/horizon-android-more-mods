from pathlib import Path
import json
import re
import tomllib

ROOT = Path(__file__).resolve().parents[1]
PACKAGE = ROOT / "mods/official/punchy-moves"


def test_punchy_moves_package_metadata_and_source_archive():
    meta = tomllib.loads((PACKAGE / "PORT.toml").read_text(encoding="utf-8"))
    assert meta["id"] == "punchy_moves"
    assert meta["name"] == "Punchy Moves"
    assert meta["version"] == "1.2.0"
    assert (PACKAGE / meta["source_archive"]).is_file()
    original = json.loads((PACKAGE / "horizon-native.json").read_text(encoding="utf-8"))
    assert original["id"] == "punchy_moves"
    assert original["version"] == "1.2.0"


def test_punchy_moves_rust_paths_resolve():
    paths = (
        ROOT / "crates/horizon-client/src/game.rs",
        ROOT / "crates/horizon-client/src/mobs.rs",
    )
    for source in paths:
        content = source.read_text(encoding="utf-8")
        for relative in re.findall(r'^\s*#\[path\s*=\s*"([^"]+)"\]', content, re.MULTILINE):
            target = (source.parent / relative).resolve()
            assert target.is_file(), f"{source.relative_to(ROOT)}: missing {relative}"


def test_punchy_moves_native_mod_and_runtime_hooks_are_registered():
    game = (ROOT / "crates/horizon-client/src/game.rs").read_text(encoding="utf-8")
    mobs = (ROOT / "crates/horizon-client/src/mobs.rs").read_text(encoding="utf-8")
    anim = (ROOT / "crates/horizon-client/src/mobs/anim.rs").read_text(encoding="utf-8")
    render = (ROOT / "crates/horizon-client/src/mobs/render.rs").read_text(encoding="utf-8")
    bundled = (ROOT / "crates/horizon-client/src/game/android_bundled.rs").read_text(encoding="utf-8")
    runtime = (PACKAGE / "mixins/punchy_moves/runtime.rs").read_text(encoding="utf-8")
    trails = (PACKAGE / "mixins/punchy_moves/trails.rs").read_text(encoding="utf-8")
    assert "mod punchy_moves;" in game
    assert "mod punchy_moves_trails;" in game
    assert "pub mod punchy_moves;" in mobs
    assert "super::punchy_moves::pose_start(m, s);" in anim
    assert "super::punchy_moves::pose(p);" in anim
    assert "super::punchy_moves::body(m, alpha, base);" in render
    for call in (
        "super::punchy_moves::key",
        "super::punchy_moves::listed",
        "super::punchy_moves::settings_page",
        "super::punchy_moves::settings_click",
        "super::punchy_moves::frame",
    ):
        assert call in bundled
    assert 'pub(crate) const ID: &str = "punchy_moves";' in runtime
    assert 'const NAME: &str = "Punchy Moves";' in runtime
    assert "anim::state(m, alpha).pos" in (PACKAGE / "mixins/punchy_moves/rig.rs").read_text(encoding="utf-8")
    assert "p.fall_flying" in runtime
    assert "horizon_client::mobs::anim::state(m, partial).pos" not in runtime
    assert "horizon_client::hud::rgba" in trails
    assert "world_primitives" not in trails
    assert "punchy_moves::hand_pose" not in game
    punchy_runtime = (ROOT / "mods/official/punchy/mixins/punchy/runtime.rs").read_text(encoding="utf-8")
    punchy_engine = (ROOT / "mods/official/punchy/mixins/punchy/engine/engine.rs").read_text(encoding="utf-8")
    assert "hand_motion_input(game)" in punchy_runtime
    assert "motion_delta(&s, false" in punchy_runtime
    assert "motion_delta(&s, true" in punchy_runtime
    assert 'pub mod hand_motion;' in punchy_engine
    assert "first_person: bool" not in runtime
    assert "first_person_amount" not in runtime
    assert "punchy_moves::world" in game
    assert "self.mark_translucent_entity(trail_start, \"minecraft:player\")" in game
    assert "super::punchy_moves::ID" in trails
    assert 'Display name: `Punchy Moves`' in (PACKAGE / "README.md").read_text(encoding="utf-8")


def test_android_body_transform_binding_is_mutable_before_matrix_updates():
    source = (ROOT / "crates/horizon-client/src/mobs/render.rs").read_text(encoding="utf-8")
    lines = source.splitlines()
    needle = "let mut base = super::punchy_moves::body(m, alpha, base);"
    assert any(line.strip() == needle for line in lines)
    index = next(i for i, line in enumerate(lines) if line.strip() == needle)
    following = "\n".join(lines[index + 1 : index + 5])
    assert "base *= glam::Mat4::from_scale" in following
    assert "base *= species_scale(m, alpha);" in following
