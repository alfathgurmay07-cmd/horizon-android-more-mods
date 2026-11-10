from pathlib import Path
import json
import unittest

ROOT = Path(__file__).resolve().parents[1]


class PunchyAndroidInteractionTests(unittest.TestCase):
    def test_successful_android_interactions_reach_punchy(self):
        source = (ROOT / "crates/horizon-client/src/game/remote.rs").read_text()
        for name in ("remote_use_entity", "remote_use_item_on", "remote_use_item"):
            start = source.index(f"fn {name}(")
            end = source.index("\n    }", start) + len("\n    }")
            wrapper = source[start:end]
            self.assertIn(f"self.{name}_inner(hand)", wrapper)
            self.assertIn('#[cfg(target_os = "android")]', wrapper)
            self.assertIn("super::punchy::used(self, hand, success);", wrapper)
            self.assertIn("success", wrapper)

    def test_mining_attempt_reaches_punchy_before_prediction(self):
        source = (ROOT / "crates/horizon-client/src/game/remote.rs").read_text()
        start = source.index("fn remote_mining(&mut self, start: bool)")
        end = source.index("\n    }", start) + len("\n    }")
        mining = source[start:end]
        self.assertIn('#[cfg(target_os = "android")]', mining)
        self.assertIn("if (start", mining)
        self.assertIn("remote.destroy_delay == 0", mining)
        self.assertIn("self.pick().is_some()", mining)
        self.assertIn("super::punchy::mining_attempt(self);", mining)
        self.assertLess(mining.index("super::punchy::mining_attempt(self);"), mining.index("let debug_stick"))

    def test_success_events_are_queued_and_consumed_without_losing_taps(self):
        runtime = (ROOT / "mods/official/punchy/mixins/punchy/runtime.rs").read_text()
        self.assertIn("successful_use: VecDeque<action::Hand>", runtime)
        self.assertIn("state.successful_use.push_back(hand(used_hand));", runtime)
        self.assertIn("s.successful_use.pop_front()", runtime)
        self.assertIn("let was_use_down=s.use_down;", runtime)
        self.assertIn("use_down && !was_use_down", runtime)

    def test_registry_block_items_select_full_block_attack_and_mining_animations(self):
        runtime = (ROOT / "mods/official/punchy/mixins/punchy/runtime.rs").read_text()
        self.assertIn(".filter(|info| info.block.is_some())", runtime)
        self.assertIn('map(|_| "FULL_BLOCK".to_owned())', runtime)
        self.assertIn("kind: state.item_kinds[index].as_deref()", runtime)
        self.assertIn('phase_selection(s, "mining", action::Hand::Main, speed)', runtime)
        tuning = json.loads((ROOT / "mods/official/punchy/mixins/punchy/engine/data/tuning.json").read_text())
        full_block = tuning["item"]["FULL_BLOCK"]["customAnimation"]
        self.assertIn({"type": "attack", "var_1": "interact"}, full_block)
        self.assertIn({"type": "mining_v2", "start": "full_block_mining_start", "looping": "full_block_mining_loop", "end": "full_block_mining_end"}, full_block)
        animations = json.loads((ROOT / "mods/official/punchy/mixins/punchy/engine/data/animations.json").read_text())["animations"]
        for name in ("interact", "full_block_mining_start", "full_block_mining_loop", "full_block_mining_end"):
            self.assertIn(name, animations)

    def test_full_block_item_and_arm_transforms_use_registry_context(self):
        runtime = (ROOT / "mods/official/punchy/mixins/punchy/runtime.rs").read_text()
        self.assertIn("kind: s.item_kinds[h].as_deref()", runtime)
        self.assertIn("transform_for(&context, left, true)", runtime)
        self.assertIn("transform_for(&context, left, false)", runtime)
        tuning = json.loads((ROOT / "mods/official/punchy/mixins/punchy/engine/data/tuning.json").read_text())
        full_block = tuning["item"]["FULL_BLOCK"]
        self.assertIn("transforms", full_block)

    def test_generic_held_use_falls_back_to_interaction_clip(self):
        actions = (ROOT / "mods/official/punchy/mixins/punchy/engine/actions.rs").read_text()
        self.assertIn('ItemKind::Drink => ("use_drink", ActionKind::Drink, Source::SustainedUse)', actions)
        self.assertIn('_ => ("use_item", ActionKind::Interact, Source::SustainedUse)', actions)
        self.assertIn('clips.resolve("interact", u.hand)', actions)
        self.assertIn('kind == ActionKind::Interact', actions)
        self.assertIn('"interact"', (ROOT / "mods/official/punchy/mixins/punchy/engine/data/animations.json").read_text())


if __name__ == "__main__":
    unittest.main()
