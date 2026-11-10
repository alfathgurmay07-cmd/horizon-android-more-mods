# Punchy Moves

- Display name: `Punchy Moves`
- Native mod ID: `punchy_moves`
- Source version: `1.2.0`
- Game target: `26.3`
- Original archive: `original/livingmotion-1.2.0.hnmod`

## Android port

Punchy Moves is statically bundled into the Android client; the app does not load this `.hnmod` dynamically. The player rig is adapted to the project's current `mobs::anim` and `mobs::render` paths. Runtime frame sampling, settings, key bindings, third-person player transforms and elytra trail geometry are connected to the Android native bundle. First-person arm, item and locomotion animation is owned by the separate Punchy mod. The original mod named `Punchy` remains separate; this port uses `punchy_moves` so their settings, key bindings and configuration folders do not collide.

The port preserves the MIT license for the main animation code and the MPL-2.0 terms for `trails.rs`; see `LICENSE` and `NOTICE.md`. The target tree and supplied bundle pin different Horizon revisions, so the original injection table is retained only in the original archive. The Android source uses direct static integration points adapted to this tree instead of runtime mixin loading.

## Verification

Source integration checks and focused Python tests are included in the final package. Rust compilation, APK packaging and on-device rendering were not run in the editing environment because `cargo`, `rustc`, and the Android toolchain are unavailable.
