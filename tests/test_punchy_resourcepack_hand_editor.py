from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
RUNTIME = ROOT / "mods/official/punchy/mixins/punchy/runtime.rs"
RESOURCES = ROOT / "mods/official/punchy/mixins/punchy/engine/resources.rs"
HOOKS = ROOT / "crates/horizon-client/src/game/native_hooks.rs"
BUNDLED = ROOT / "crates/horizon-client/src/game/android_bundled.rs"


class PunchyResourcePackAndHandEditorTests(unittest.TestCase):
    def test_active_pack_paths_are_supported(self):
        source = RESOURCES.read_text()
        for path in (
            "assets/minecraft/punchy/animations/player.animation.json",
            "assets/punchy/animations/player_3_p.animation.json",
            "assets/minecraft/punchy/animations/player_3_p.animation.json",
            "assets/punchy/geo/player.geo.json",
            "assets/minecraft/punchy/compat/tuning_template.json",
        ):
            self.assertIn(path, source)
        self.assertIn("from_active_pack", source)

    def test_pack_assets_merge_with_bundled_assets(self):
        source = RESOURCES.read_text()
        self.assertIn("merge_json", source)
        self.assertIn("missing_resource_pack_assets_keep_embedded_resources", source)

    def test_builtin_animation_pack_is_the_default_resource_source(self):
        import zipfile

        pack_path = ROOT / "mods/official/punchy/mixins/punchy/engine/data/punchy_builtin.zip"
        self.assertTrue(pack_path.is_file())
        with zipfile.ZipFile(pack_path) as pack:
            names = set(pack.namelist())
            self.assertIn("pack.mcmeta", names)
            self.assertIn("assets/minecraft/punchy/animations/player.animation.json", names)
            self.assertIn("assets/punchy/geo/player.geo.json", names)
            self.assertIn("assets/minecraft/punchy/compat/tuning_template.json", names)
        resource_source = RESOURCES.read_text()
        runtime = RUNTIME.read_text()
        pack_source = (ROOT / "crates/horizon-client/src/game/packs.rs").read_text()
        self.assertIn('include_bytes!("data/punchy_builtin.zip")', resource_source)
        self.assertIn("builtin_resource_pack()", resource_source)
        self.assertIn("install_builtin_resource_pack", runtime)
        self.assertIn('"assets/minecraft/resourcepacks/punchy.zip"', runtime)
        self.assertIn('ids.push("punchy".to_owned())', pack_source)
        self.assertIn('required: p.id == "punchy"', pack_source)

    def test_custom_resource_packs_are_above_punchy_builtin(self):
        source = (ROOT / "crates/horizon-client/src/game/packs.rs").read_text()
        startup = source.split("pub fn startup_root(", 1)[1].split("fn info(", 1)[0]
        self.assertLess(startup.index('ids.push("punchy".to_owned())'), startup.index("ids.extend(selected"))
        self.assertIn('filter(|id| id.as_str() != "punchy")', startup)
        self.assertIn('super::punchy::install_builtin_resource_pack(vanilla)?', startup)

    def test_native_data_allows_safe_asset_paths_with_size_limit(self):
        source = HOOKS.read_text()
        method = source.split("pub(super) fn native_data(&self, path: &str)", 1)[1].split("\n    }", 1)[0]
        self.assertIn('path.starts_with("assets/")', method)
        self.assertIn('path.starts_with("data/")', method)
        self.assertIn("32 * 1024 * 1024", method)

    def test_resource_reload_invalidates_punchy_cache(self):
        self.assertIn("super::punchy::resources_reloaded();", BUNDLED.read_text())
        self.assertIn("pub(super) fn resources_reloaded()", RUNTIME.read_text())

    def test_hand_editor_exposes_reference_targets_and_scopes(self):
        reference_path = Path("/mnt/data/punchy_reference/punchy/client/menu/core/ui/HandEditorPanel.java")
        if reference_path.is_file():
            reference = reference_path.read_text()
            for token in ("EDIT_ITEM", "EDIT_ARM", "EDIT_LINE", "SCOPE_ITEM", "SCOPE_TAG", "SCOPE_GROUP", "SCOPE_ALL", "mirrorPose", "resetPose"):
                self.assertIn(token, reference)
        runtime = RUNTIME.read_text()
        for token in ("Hand Editor", "Main hand", "Off hand", "Offset X", "Rotation X", "Scale", "Pivot X", "Mirror selected pose to other hand", "Reset selected pose", '"item"', '"tag"', '"group"', '"all"', "Save new Hand Editor profile", "Undo", "Redo"):
            self.assertIn(token, runtime)

    def test_hand_editor_persists_and_applies_arm_and_item_transforms(self):
        source = RUNTIME.read_text()
        self.assertIn('"hand_editor.json"', source)
        self.assertIn("editor_transform", source)
        self.assertIn('editor_transform(&s, &id, left, "arm", &tags)', source)
        self.assertIn('editor_transform(&s, &id, left, "item", &tags)', source)
        self.assertIn("state.editor.save(game)", source)

    def test_runtime_load_uses_selected_resource_pack_assets(self):
        source = RUNTIME.read_text()
        self.assertIn('game.native_data_tree("assets/punchy/animations")', source)
        self.assertIn('game.native_data_tree("assets/minecraft/punchy/animations")', source)
        self.assertIn('game.native_data_tree("assets/minecraft/punchy/compat")', source)
        self.assertIn('game.native_data_tree("assets/punchy/compat")', source)
        self.assertIn("resources::from_active_pack_tree", source)
        self.assertIn("load(game, &mut s)", source)


if __name__ == "__main__":
    unittest.main()
