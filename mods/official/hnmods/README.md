# Extra Horizon HNMOD sources

The folders in this directory are **pinned Git submodules** so the upstream source, authorship, licenses and release
history stay intact. Initialise them with:

```sh
git submodule update --init --recursive
```

## Included source modules

| Mod | Folder | Version | Build status |
|---|---|---:|---|
| BridgingMod | `bridgingmod-horizon/` | 1.0.0 | Packaged by CI as desktop HNMOD |
| Punchy Horizon | `punchy-horizon/` | 0.1.4 | Source pinned; not repackaged because original Punchy code/data carry a custom all-rights-reserved license |
| Continuity | `continuity-horizon/` | 0.1.7 | Upstream packaging script + source tests run in CI |
| 3D Skin Layers | `skinlayers3d-horizon/` | 0.1.0 | Packaged by CI as desktop HNMOD |
| Horizon Sodium | `horizon-sodium/` | 0.4.0 | Upstream packaging script + source tests run in CI |

The workflow **Build Extra Horizon HNMODs** runs on pushes to this integration and can also be started manually from
GitHub Actions. It uploads the four distributable `.hnmod` files as the artifact
`horizon-extra-hnmods`. Punchy v0.1.4 remains available from its author's release page:
https://github.com/MoonFvv/punchy-horizon/releases/tag/v0.1.4

## Important: desktop HNMOD is not Android static integration

This Android fork uses a fixed set of statically compiled Rust modules; it has no phone-side dynamic `.hnmod` loader.
Keeping these sources in folders or packaging them does **not** make them appear in the Android Options > Mods menu or
include them in the APK. Each Android port still needs API adaptation, static Rust module wiring, independent menu
registration and an Android build/device test.

The desktop bundles pin different Horizon revisions. Use each artifact only with a Horizon build compatible with its
`horizon-native.json`. Do not assume all four bundles can be enabled together merely because their archives package.

## Build locally

```sh
git submodule update --init --recursive
python3 tools/package_extra_hnmods.py
```

Rust tests and source checks used by CI are listed in `.github/workflows/build-extra-hnmods.yml`.
