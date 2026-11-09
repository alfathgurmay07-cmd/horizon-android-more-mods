import json
import unittest
from pathlib import Path

import struct

ROOT = Path(__file__).resolve().parents[1]
RUNTIME = ROOT / "mods/official/punchy/mixins/punchy/runtime.rs"
ACTIONS = ROOT / "mods/official/punchy/mixins/punchy/engine/actions.rs"
TUNING_RS = ROOT / "mods/official/punchy/mixins/punchy/engine/tuning.rs"
TUNING = ROOT / "mods/official/punchy/mixins/punchy/engine/data/tuning.json"
ANIMATIONS = ROOT / "mods/official/punchy/mixins/punchy/engine/data/animations.json"
ICON = ROOT / "packaging/android/res/drawable-nodpi/launcher_icon.png"


class PunchySpecialUseAnimationTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.runtime = RUNTIME.read_text()
        cls.actions = ACTIONS.read_text()
        cls.tuning_rs = TUNING_RS.read_text()
        cls.tuning = json.loads(TUNING.read_text())
        cls.animations = json.loads(ANIMATIONS.read_text())["animations"]

    def test_android_use_state_includes_active_touch_use(self):
        self.assertIn('let key_use_down = game.native_mapping("key.use").is_some_and(|m| m.0);', self.runtime)
        self.assertIn("let use_down = key_use_down || using.is_some();", self.runtime)

    def test_bow_release_animation_is_configured_to_existing_shot_clips(self):
        bow_entries = self.tuning["item"]["BOW"]["customAnimation"]
        shot = next(entry for entry in bow_entries if entry["type"] == "use_bow_shot")
        self.assertEqual(shot["name"], "use_bow_shoot")
        self.assertIn("use_bow_right", self.animations)
        self.assertIn("use_bow_left", self.animations)
        self.assertIn("use_bow_shoot_right", self.animations)
        self.assertIn("use_bow_shoot_left", self.animations)

    def test_trident_use_and_release_clips_exist(self):
        for name in (
            "use_trident",
            "use_trident_throw",
            "use_trident_throw_riptide",
            "use_trident_throw_loyalty",
        ):
            self.assertIn(name, self.animations)
        self.assertIn('stack.enchantment_level("minecraft:riptide")>0', self.runtime)
        self.assertIn('stack.enchantment_level("minecraft:loyalty")>0', self.runtime)

    def test_spear_charge_stage_is_fed_from_kinetic_item_data(self):
        self.assertIn("horizon_client::spear::Kinetic::of(&stack.item)", self.runtime)
        self.assertIn("action::spear_charge_stage(", self.runtime)
        self.assertIn("spear_stage,spear_maxed,", self.runtime)
        for name in ("use_spear_pose_1", "use_spear_pose_2", "use_spear_pose_3"):
            self.assertIn(name, self.animations)

    def test_food_and_drink_use_registry_consumable_or_food_metadata(self):
        self.assertIn("info.consumable.as_ref()", self.runtime)
        self.assertIn("info.food.is_some()", self.runtime)
        self.assertIn("UseAnimation::Eat", self.runtime)
        self.assertIn("UseAnimation::Drink", self.runtime)
        for name in (
            "eat_start",
            "eat_loop",
            "eat_end",
            "drink_start",
            "drink_loop",
            "drink_end",
        ):
            self.assertIn(name, self.animations)
        defaults = self.tuning["default_animations"]
        self.assertTrue(any(entry.get("type") == "eating" for entry in defaults))
        self.assertTrue(any(entry.get("type") == "drinking" for entry in defaults))

    def test_spear_item_ids_are_classified_and_registry_kinetic_overrides_kind(self):
        self.assertIn('id == "spear" || id.ends_with("_spear")', self.actions)
        self.assertIn('name == "spear" || name.ends_with("_spear")', self.tuning_rs)
        self.assertIn('let kinetic=horizon_client::spear::Kinetic::of(&stack.item);', self.runtime)
        self.assertIn('action::ItemKind::Spear', self.runtime)

    def test_trident_specific_tuning_contains_charge_and_throw_variants(self):
        entries = self.tuning["itemSpecific"]["minecraft:trident"]["customAnimation"]
        by_type = {entry["type"]: entry.get("name") for entry in entries}
        for animation_type, clip in (
            ("use_trident", "use_trident"),
            ("use_trident_throw", "use_trident_throw"),
            ("use_trident_throw_riptide", "use_trident_throw_riptide"),
            ("use_trident_throw_loyalty", "use_trident_throw_loyalty"),
        ):
            self.assertEqual(by_type.get(animation_type), clip)

    def test_bow_release_has_side_correct_clip_fallback(self):
        self.assertIn('"use_bow_shoot_left"', self.runtime)
        self.assertIn('"use_bow_shoot_right"', self.runtime)
        self.assertIn('bow_shot_alias.and_then(|n| self.state.clips.get(n))', self.runtime)

    def test_use_actions_include_all_requested_clip_names(self):
        for name in (
            "use_bow_right", "use_bow_left",
            "use_bow_shoot_right", "use_bow_shoot_left",
            "use_trident", "use_trident_throw", "use_trident_throw_riptide",
            "use_trident_throw_loyalty",
            "use_spear_pose_1", "use_spear_pose_2", "use_spear_pose_3",
            "eat_start", "eat_loop", "eat_end",
        ):
            self.assertIn(name, self.animations)

    def test_android_launcher_icon_is_installed_and_referenced(self):
        icon = ICON.read_bytes()
        self.assertTrue(icon.startswith(b"\x89PNG\r\n\x1a\n"))
        self.assertEqual(struct.unpack(">II", icon[16:24]), (512, 512))
        manifest = (ROOT / "packaging/android/AndroidManifest.xml").read_text()
        self.assertIn('android:icon="@drawable/launcher_icon"', manifest)


if __name__ == "__main__":
    unittest.main()
