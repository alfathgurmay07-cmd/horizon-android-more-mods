# Official mod sources

This tree contains the Android-native ports and source-preserving imports used by Horizon.

## Android static native modules

- `accurate-block-placement/` and `connected-textures/` are Android-native source ports.
- `appleskin-horizon/` is wired into the Android HUD and Options > Mods.
- `freecam/` is Freecam+ 1.0.1 with detached camera, movement, turning, wheel and settings hooks.
- Every bundled native module starts disabled; enable it explicitly in Options > Mods.

## Imported HNMOD sources

| Folder | Native mod ID | Version | Android status |
|---|---|---:|---|
| `freecam/` | `freecam_plus` | 1.0.1 | Android static port of Freecam+ with detached camera and flight controls |
| `motion-blur/` | `motion_blur` | 0.1.0 | source-only, not wired |
| `punchy/` | `punchy` | 0.1.2 | source-only preview; renderer parity incomplete |
| `basic-hud/` | `basic_hud` | 0.5.0 | statically wired to Android hooks; compile/device validation pending |
| `visuality-horizon/` | `visuality_horizon` | 1.0.2 | statically wired to Android game and renderer hooks; compile/device validation pending |

Keeping an HNMOD folder under `mods/official/` does not by itself compile its modules or install its hooks. Basic HUD and Visuality are statically adapted to the Android client and registered in Options > Mods; both still need an Android build and device test.

Voice Chat and Inventory Profiles were removed from this tree at the user's request. The 3D Skins mod remains removed.

## User-provided Android static ports

| Folder | Mod ID | Version | Integration |
|---|---|---:|---|
| `jade/` | `jade` | 26.3.6 | Android game, HUD, configuration, commands and display hooks |
| `armorhud/` | `armorhud` | 1.0.0 | Android settings, keybind and HUD |
| `resourcify/` | `resourcify` | 0.1.1 | Android resource-pack browser in Options |
