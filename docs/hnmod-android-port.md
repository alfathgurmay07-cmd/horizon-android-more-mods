# HNMOD Android port status

This source tree does **not** add a phone-side `.hnmod` loader. The Android build
uses a fixed set of statically compiled Rust modules and the native hooks already
provided by Horizon. The source-only `.hnmod` archives therefore have to be
adapted to this tree's current APIs.

## Ported into the Android source

### Accurate Block Placement 1.0.0

- Moved the portable runtime and Android adapters to `mods/official/accurate-block-placement/src/`; the Android client imports them through `crates/horizon-client/src/game/android_bundled.rs`.
- Wired its keybind, Mods list row, settings page, setting persistence, frame sampling,
  and client tick processing into the current Android bundle.
- The tick adapter is located in `game/remote.rs` so it can reuse Horizon's existing
  private `remote_block_reacts`, `remote_use`, and local prediction path without
  widening those methods' visibility.
- Added a profile-state reset, so cached settings from another launcher profile are
  not carried forward.
- The supplied archive does not include a license file/declaration for the HNMOD's
  Rust port itself (its runtime identifies an MIT-licensed upstream dependency).
  Keep the port local until its author clarifies the port's license.

### Connected Textures 0.1.7

- Moved the renderer runtime to `mods/official/connected-textures/src/native_connected_textures.rs`; the renderer imports it from `crates/horizon-client-render/src/lib.rs`.
- Adapted its mesh pass to run on the current `Mesher::mesh_reusing` output, after
  Horizon finishes building the solid and translucent quad lists.
- Added its Mods row, settings page, `config/connected_textures.txt` persistence,
  profile reset, and resource-pack reload cache invalidation.
- The source archive supplied for this mod contains no license file or license
  declaration. It is therefore integrated only in this user's local working tree;
  do not redistribute this port unless the author grants permission or supplies a
  compatible license.

## Package layout and Mod Menu registration

The source of each port is now separated under:

- `mods/official/accurate-block-placement/`
- `mods/official/connected-textures/`

The Android adapters register `accurate_block_placement` and `connected_textures` as separate
`NativeMod` entries. They therefore have independent rows and settings pages in Options > Mods.
The implementation remains statically linked into the Android application; placing a folder in
`mods/official/` alone does not make Horizon dynamically discover it.

## Basic HUD Android static port

### Basic HUD 0.5.0

- Preserved at `mods/official/basic-hud/` and statically compiled only for Android.
- Registered in Options > Mods with its own settings page and Right Shift layout-editor keybind.
- Wired to HUD drawing, frame/tick, Tab-list rendering, name-tag health/totem overlays, totem events, entity lifecycle cleanup, and local/multiplayer pickup receipts.
- Replaced the original private-field bridge for cooldown data with read-only `Cooldowns::entries()` and `Cooldowns::tick_count()` accessors in `horizon-items`.
- The upstream allocator/thread-detach injection is omitted because its helper only implements the Windows GNU thread-detach path; it is not needed by the Android runtime.
- The source pins Horizon revision `68ac6f647e7022b2c696e81863d7cface28c9416` and horizon-mixins revision `0d6b103c70633d8fd9443d8d0acd3aadaacedb06`.
- The upstream repository declares MIT; its notice is retained in `mods/official/basic-hud/LICENSE`.

Static source-path and hook-registration checks are added for the Android adapters. The Rust toolchain and Android SDK/NDK are unavailable in this editing environment, so compilation, APK packaging, install and on-device behaviour remain unverified.

### Punchy Rust Preview 0.1.2

- Statically bundled into the Android client with its animation, geometry, and tuning data.
- Connected to the game tick, keybinds, successful-use/mining triggers, and first-person hand geometry renderer.
- The Mods row uses the short title `Punchy` so its ON/OFF state remains visible. It is the only enable/disable control; its settings page contains animation speed, reload, and hand-pose selection.
- This remains a preview, not full Punchy parity. Armour/body animation, every custom animation condition, the complete item mesh/physics/attachment renderer, and full Java/Fabric behaviour are not implemented. Android compilation and device testing remain unverified.

## Additional HNMOD packages

The packages below retain their original manifests and source. Basic HUD and Visuality have Android static adapters.

### Visuality Horizon 1.0.2

- Imported to `mods/official/visuality-horizon/` with its engine crate, atlas, settings and source helpers intact.
- The engine is a workspace package and is linked into `horizon-client-render` only for Android.
- Wired to the renderer depth/world-drawn hooks, game tick/frame hooks, Options > Mods row/settings, damage and entity lifecycle signals, session reset, ambient block sampling, and slime landing replacement.
- Its Android adapters use the current `WorldDrawn`/`RenderNeeds` API and the source functions present in this tree; the original private-hook payloads are not compiled as independent mixins.
- The upstream package pins Horizon revision `c9d474dd9116e84c243f8992d75c11314df56f06`; the supplied bundle and Android source therefore still require a full Android compile to confirm API compatibility. The source package includes its MIT notice in the engine source.

The two provided bundles pin different upstream Horizon commits. Basic HUD and Visuality have Android static adapters and separate Options > Mods rows. Android compilation, APK packaging and on-device behaviour have not been validated in this environment.


### Punchy Moves 1.2.0 (ported from Living Motion)

- The source is kept at `mods/official/punchy-moves/`; the in-game name is **Punchy Moves** and the native ID is `punchy_moves`. The existing `Punchy` mod remains a separate entry.
- The animation rig is connected to player model setup and the player body transform. The Android game loop updates per-player motion state once per frame.
- First-person arm, map and held-item transforms receive the mod's hand pose. Elytra contrails are appended to the world/entity geometry pass.
- The mod has its own Options > Mods row, key bindings, settings page and persistent config directory.
- Original source is retained in `mods/official/punchy-moves/original/livingmotion-1.2.0.hnmod`. The main code's MIT notice and the trail renderer's MPL-2.0 terms are retained in `LICENSE` and `NOTICE.md`.
- The original HNMOD targets a different Horizon revision. Its injection manifest is preserved for provenance; the Android port uses static hooks adapted to this project's current source tree.
- Rust compilation, APK packaging and device rendering are not verified in this environment.

## Not separately ported

### Mod Menu 0.3.3

This Android source already has an Options > Mods page with mod rows and individual
settings pages. The supplied Mod Menu HNMOD replaces/extends menu/title-screen UI,
so it is not a safe drop-in and would duplicate the existing Mods entry point. The
existing Android Mods UI remains in place instead of adding a second menu layer.

## Build and verification status

- Static source checks: the source-path, menu-registration, metadata and mod-specific adapter checks are run against the final tree.
- The final source package includes both relocated mod source trees and the updated Rust path declarations.
- Rust compilation, Android packaging, APK install, and in-game behavior: **not run**.
  The editing environment does not have `rustc`, `cargo`, the Android SDK/NDK, or
  Gradle configured (Java and `javac` are present). No compiled APK is included, and
  compatibility on a device is not yet established.

## Apply the patch

From the root of the unmodified `horizon-android-more-mods-android` source directory:

```sh
patch -p1 < horizon-android-hnmod-port.patch
```

Then follow the project's `docs/android.md` build setup. For a public ARM64 build,
the documented flow is:

```sh
rustup target add aarch64-linux-android
python tools/android_prepare.py
python tools/android_build.py --abi arm64-v8a --public
```

The project documents Rust 1.95, Python 3.11+, Android SDK platform 35, Android
build-tools, NDK, a JDK, and Gradle 8.14. Build only after those prerequisites and
the project's private game-data preparation are configured.


## Additional desktop HNMOD sources

Pinned upstream source submodules are located under `mods/official/hnmods/`; HorizonVC is separated under
`mods/server/horizon-vc/`. The GitHub Actions workflow **Build Extra Horizon HNMODs** runs the source checks and
packages four desktop bundles (BridgingMod, Continuity, Skin Layers 3D and Horizon Sodium) as workflow artifacts.
Punchy Horizon v0.1.4 is pinned to its upstream source, but is deliberately not repackaged because its original
implementation and animation data carry a custom all-rights-reserved license. HorizonVC's standalone core tests run
separately; its full server hooks still depend on the Detimes `mod-hooks-fake-players` branch.

These additions do not create Android adapters or activate the modules in the APK. Each client mod still needs to be
ported to the fixed Android Rust module list, registered in Options > Mods and tested in an Android build.
