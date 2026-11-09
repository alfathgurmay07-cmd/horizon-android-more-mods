import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def source(path):
    return (ROOT / path).read_text()


class FreecamPlusAndroidWiring(unittest.TestCase):
    def test_freecam_plus_runtime_is_detached_camera(self):
        runtime = source("mods/official/freecam/mixins/freecam/runtime.rs")
        for item in (
            'pub const ID: &str = "freecam_plus";',
            "pub fn start(",
            "pub fn stop(",
            "pub fn hold(",
            "pub fn turn(",
            "pub fn tick(",
            "pub fn view(",
            "pub fn wheel(",
            "pub fn slide(",
        ):
            self.assertIn(item, runtime)

    def test_runtime_and_glue_are_loaded_by_android_game(self):
        game = source("crates/horizon-client/src/game.rs")
        self.assertIn('path = "../../../mods/official/freecam/mixins/freecam/runtime.rs"', game)
        self.assertIn('path = "../../../mods/official/freecam/mixins/freecam/glue.rs"', game)
        self.assertIn('path = "../../../mods/official/freecam/android.rs"', game)
        self.assertIn("freecam_hooks::mapped(self, key, pressed, repeat)", game)
        self.assertIn("freecam_hooks::window(self, &event)", game)

    def test_camera_position_and_look_override_player_camera(self):
        perspective = source("crates/horizon-client/src/game/perspective.rs")
        self.assertIn("super::freecam_hooks::camera(partial)", perspective)
        self.assertIn("super::freecam_hooks::turn(xo, yo)", perspective)

    def test_all_android_native_mod_callbacks_are_wired(self):
        bundled = source("crates/horizon-client/src/game/android_bundled.rs")
        for item in (
            "super::freecam_hooks::key(",
            "super::freecam_hooks::wheel(",
            "super::freecam_hooks::tick(",
            "super::freecam_hooks::list(",
            "super::freecam_hooks::settings(",
            "super::freecam_hooks::click(",
        ):
            self.assertIn(item, bundled)
        self.assertNotIn("super::freecam_hooks::frame(", bundled)
        self.assertNotIn("super::freecam_hooks::hud(", bundled)

    def test_touchscreen_movement_is_redirected_to_freecam(self):
        touch = source("crates/horizon-client/src/game/android_input.rs")
        self.assertGreaterEqual(touch.count("super::freecam_hooks::mapped(self, key, down, false)"), 2)
        self.assertIn("super::freecam_hooks::mapped(self, key, false, false)", touch)
        glue = source("mods/official/freecam/mixins/freecam/glue.rs")
        for key in ("KeyW", "KeyS", "KeyA", "KeyD", "Space", "ShiftLeft", "ControlLeft"):
            self.assertIn("KeyCode::" + key, glue)
        self.assertEqual(glue.count("pub(super) fn mapped("), 1)

    def test_freecam_camera_tracks_independent_rotation_and_position(self):
        glue = source("mods/official/freecam/mixins/freecam/glue.rs")
        self.assertIn("freecam::turn(xo, yo)", glue)
        self.assertIn("freecam::view(partial)", glue)
        self.assertIn("freecam::tick(", glue)
        self.assertIn("game.camera_type = CameraType::ThirdPersonBack", glue)
        self.assertIn("freecam::start(eye, game.yaw, game.pitch, view)", glue)

    def test_mod_catalog_and_legacy_settings_migrate(self):
        mods = source("crates/horizon-client-render/src/mod_list.rs")
        self.assertIn('"freecam_plus"', mods)
        self.assertIn('"Freecam+"', mods)
        self.assertIn('enabled.remove("freecam")', mods)
        self.assertIn('disabled.remove("freecam")', mods)
        self.assertIn('keys.remove("freecam")', mods)
        self.assertIn('keys.entry("freecam_plus".to_owned())', mods)
        self.assertEqual(mods.count("ANDROID_BUNDLED: ["), 1)

    def test_mixins_manifest_helpers_exist(self):
        manifest = source("mods/official/freecam/mixins/freecam/horizon-mixin.toml")
        for hook in ("camera", "click", "key", "list", "settings", "tick", "turn", "wheel", "window"):
            self.assertIn(f'helper = "hooks/{hook}.rs"', manifest)
            self.assertTrue((ROOT / f"mods/official/freecam/mixins/freecam/hooks/{hook}.rs").is_file())
        self.assertIn('id = "freecam_plus"', manifest)
        metadata = source("mods/official/freecam/horizon-native.json")
        self.assertIn('"id": "freecam_plus"', metadata)

    def test_no_duplicate_freecam_mapped_adapter(self):
        glue = source("mods/official/freecam/mixins/freecam/glue.rs")
        self.assertEqual(glue.count("pub(super) fn mapped("), 1)

    def test_source_snapshot_has_no_removed_feature_markers(self):
        removed_a = "horizon" + "matica"
        removed_b = "lite" + "matica"
        for path in ROOT.rglob("*"):
            if not path.is_file() or ".git" in path.parts:
                continue
            if path.suffix not in (".rs", ".toml", ".json", ".md", ".py", ".yml", ".yaml"):
                continue
            text = path.read_text(errors="ignore").lower()
            self.assertNotIn(removed_a, text, str(path))
            self.assertNotIn(removed_b, text, str(path))


if __name__ == "__main__":
    unittest.main()
