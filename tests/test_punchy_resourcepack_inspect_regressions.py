import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]


class PunchyResourcePackInspectTests(unittest.TestCase):
    def test_loader_scans_both_animation_and_compat_namespaces(self):
        runtime = (ROOT / "mods/official/punchy/mixins/punchy/runtime.rs").read_text()
        resources = (ROOT / "mods/official/punchy/mixins/punchy/engine/resources.rs").read_text()
        hooks = (ROOT / "crates/horizon-client/src/game/native_hooks.rs").read_text()
        self.assertIn('native_data_tree("assets/punchy/animations")', runtime)
        self.assertIn('native_data_tree("assets/minecraft/punchy/animations")', runtime)
        self.assertIn('native_data_tree("assets/minecraft/punchy/compat")', runtime)
        self.assertIn('native_data_tree("assets/punchy/compat")', runtime)
        self.assertIn("from_active_pack_tree(", runtime)
        self.assertIn("is_punchy_animation_path", resources)
        self.assertIn("is_punchy_compat_path", resources)
        self.assertIn("merge_tuning_prefer_base", resources)
        self.assertIn("merge_tuning_override", resources)
        self.assertIn("pub(super) fn native_data_tree", hooks)

    def test_resource_overlay_exports_pack_priority_for_punchy_json(self):
        packs = (ROOT / "crates/horizon-client-render/src/packs.rs").read_text()
        cache = (ROOT / "crates/horizon-client-render/src/packs/cache.rs").read_text()
        runtime = (ROOT / "mods/official/punchy/mixins/punchy/runtime.rs").read_text()
        self.assertIn('data/horizon_internal/punchy_resource_priorities.json', packs)
        self.assertIn('data/horizon_internal/punchy_resource_stacks.json', packs)
        self.assertIn('data/horizon_internal/punchy_resource_stack/', packs)
        self.assertIn('native_data("data/horizon_internal/punchy_resource_priorities.json")', runtime)
        self.assertIn('native_data("data/horizon_internal/punchy_resource_stacks.json")', runtime)
        self.assertIn('native_data_tree("data/horizon_internal/punchy_resource_stack")', runtime)
        self.assertIn("b.2.cmp(&a.2)", (ROOT / "mods/official/punchy/mixins/punchy/engine/resources.rs").read_text())
        self.assertIn("const VERSION: u32 = 7;", cache)

    def test_same_path_resource_pack_layers_keep_priority_order(self):
        packs = (ROOT / "crates/horizon-client-render/src/packs.rs").read_text()
        runtime = (ROOT / "mods/official/punchy/mixins/punchy/runtime.rs").read_text()
        resources = (ROOT / "mods/official/punchy/mixins/punchy/engine/resources.rs").read_text()
        self.assertIn("stacks.retain", packs)
        self.assertIn("stack_manifest.insert(name.clone(), entries)", packs)
        self.assertIn("entries.push((-1, stored_path))", packs)
        self.assertIn("for (asset_path, layers) in &stack_manifest", runtime)
        self.assertIn("if visible_priority == Some(*priority) { continue; }", runtime)
        self.assertIn("b.2.cmp(&a.2)", resources)
        self.assertIn('profile == "tuning_template" && *priority < 0', resources)

    def test_inspect_alias_selects_pickaxe_clip_for_matching_hand(self):
        runtime = (ROOT / "mods/official/punchy/mixins/punchy/runtime.rs").read_text()
        self.assertIn("fn inspect_clip_aliases", runtime)
        self.assertIn('format!("new_inspect_pickaxe_{side}")', runtime)
        self.assertIn("inspect_clip_aliases(item, kind, left)", runtime)
        self.assertNotIn('"inspect" => &["new_inspect_right", "new_inspect_left"]', runtime)

    def test_pickaxe_clips_have_matching_itemgrip_tracks_and_keyframes(self):
        path = ROOT / "mods/official/punchy/mixins/punchy/engine/data/animations.json"
        animations = json.loads(path.read_text())["animations"]
        for side in ("left", "right"):
            clip = animations[f"new_inspect_pickaxe_{side}"]
            track = clip["bones"][f"itemgrip_{side}"]
            self.assertIn("position", track)
            self.assertIn("rotation", track)
            active_positions = [
                value.get("post", value) if isinstance(value, dict) else value
                for value in track["position"].values()
            ]
            self.assertTrue(any(any(abs(float(component)) > 0.001 for component in value) for value in active_positions))


if __name__ == "__main__":
    unittest.main()
