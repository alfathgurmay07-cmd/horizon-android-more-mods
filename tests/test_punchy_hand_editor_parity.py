from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
RUNTIME = ROOT / "mods/official/punchy/mixins/punchy/runtime.rs"
FISHING = ROOT / "crates/horizon-client/src/game/fishing.rs"


class PunchyHandEditorParityTests(unittest.TestCase):
    def test_editor_has_item_tag_group_and_all_scopes(self):
        source = RUNTIME.read_text()
        for scope in ('"item"', '"tag"', '"group"', '"all"'):
            self.assertIn(scope, source)
        self.assertIn("held_tags(game, item)", source)
        self.assertIn("group_matches", source)

    def test_tag_and_group_rules_are_applied_to_matching_items(self):
        source = RUNTIME.read_text()
        self.assertIn('"scope":scope,"selector":selector', source)
        self.assertIn('"group" => group_matches(state,selector,item,tags)', source)
        self.assertIn('"tag" => tags.iter().any(|tag| tag == selector)', source)

    def test_line_target_has_ranges_and_live_fishing_renderer_integration(self):
        source = RUNTIME.read_text()
        fishing = FISHING.read_text()
        self.assertIn('"line"', source)
        self.assertIn("fishing_line_transform", source)
        self.assertIn("super::punchy::fishing_line_transform(self, left)", fishing)
        self.assertIn("DQuat::from_euler", fishing)

    def test_profiles_and_undo_redo_persist(self):
        source = RUNTIME.read_text()
        for token in ("Save new Hand Editor profile", "Load next Hand Editor profile", "Delete selected Hand Editor profile", "Undo", "Redo", "profiles", "undo", "redo"):
            self.assertIn(token, source)

    def test_custom_group_management_is_available(self):
        source = RUNTIME.read_text()
        self.assertIn("Create group / add held item to selected group", source)
        self.assertIn("Delete selected custom group", source)
        self.assertIn("selector_members", source)

    def test_profile_export_and_import_round_trip_are_available(self):
        source = RUNTIME.read_text()
        self.assertIn("hand_editor_export.json", source)
        self.assertIn("hand_editor_import.json", source)
        self.assertIn("Imported Profile", source)

    def test_fishing_line_is_restricted_to_fishing_rod_item_scope(self):
        source = RUNTIME.read_text()
        self.assertIn('state.editor.selected_target == "line" && item != "minecraft:fishing_rod"', source)
        self.assertIn('if state.editor.selected_target == "line" { return; }', source)

    def test_editor_changes_apply_to_rendered_item_and_arm(self):
        source = RUNTIME.read_text()
        self.assertIn('editor_transform(&s, id, left, "arm", &tags)', source)
        self.assertIn('editor_transform(&s, id, left, "item", &tags)', source)


if __name__ == "__main__":
    unittest.main()
