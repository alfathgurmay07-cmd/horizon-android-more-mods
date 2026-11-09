from pathlib import Path
import unittest
from PIL import Image

ROOT = Path(__file__).resolve().parents[1]


class PunchyEditorUiIconTests(unittest.TestCase):
    def test_native_slider_render_and_event_flow_is_connected(self):
        render = (ROOT / "crates/horizon-client-render/src/options_ui.rs").read_text()
        hooks = (ROOT / "crates/horizon-client/src/game/native_hooks.rs").read_text()
        bundled = (ROOT / "crates/horizon-client/src/game/android_bundled.rs").read_text()
        options = (ROOT / "crates/horizon-client/src/game/options.rs").read_text()
        self.assertIn("WKind::NativeSlider", render)
        self.assertIn("Event::NativeSlider", render)
        self.assertIn("native_settings_slider_event", hooks)
        self.assertIn("super::punchy::sliders", bundled)
        self.assertIn("super::punchy::slider", bundled)
        self.assertIn("self.native_settings_slider_event(i, slider, value, finished)", options)

    def test_editor_has_reference_sections_targets_and_steps(self):
        runtime = (ROOT / "mods/official/punchy/mixins/punchy/runtime.rs").read_text()
        renderer = (ROOT / "crates/horizon-client-render/src/options_ui.rs").read_text()
        for section in ("Position", "Rotation", "Size", "Pivot"):
            self.assertIn(f'section: "{section}"', runtime)
        for step in ("step: 0.005", "step: 0.5", "step: 0.01", "step: 0.05", "step: 1.0"):
            self.assertIn(step, runtime)
        for target in ("Hand ·", "Edit ·", "Applies to ·"):
            self.assertIn(target, runtime)
        for target in ("HAND EDITOR", "POSE ACTIONS", "PROFILES AND GROUPS", "SELECTOR"):
            self.assertIn(target, renderer)
        self.assertIn('buttons.push(format!("{marker} {choice}"))', runtime)
        self.assertIn('44.. =>', runtime)
        self.assertIn('fn editor_scope_transform(', runtime)
        self.assertIn('let current_transform = editor_scope_transform(', runtime)

    def test_android_launcher_uses_the_supplied_icon_resource(self):
        manifest = (ROOT / "packaging/android/AndroidManifest.xml").read_text()
        self.assertIn('android:icon="@drawable/launcher_icon"', manifest)
        icon = Image.open(ROOT / "packaging/android/res/drawable-nodpi/launcher_icon.png")
        self.assertEqual(icon.size, (512, 512))
        self.assertEqual(icon.mode, "RGBA")


if __name__ == "__main__":
    unittest.main()
