# Horizon Physics

- Mod ID: `horizon_physics`
- Supplied HNMOD version: `0.20.7`
- Pinned Horizon revision: `fdc87aecdbd116bf1e08adcd062fcbaea8c2e2ca`
- Pinned horizon-mixins revision: `d9c83d7096b7ea5b83678da2efdfd6161f74997e`

The complete supplied native source bundle is preserved under `mixins/horizon_physics/`.
The solver, setup UI and renderer sway module are linked into the Android client and renderer.
The native gameplay/render hooks, mod-list settings, title setup screen, options page, particle
capture and terrain changes are wired into the Android static bundle. The shader hook was adapted
to this fork's `CheckedStage`/Naga material builder. The reference `fxc.rs` hook remains preserved
but is not wired: this fork has no matching `gfx::reuse::On::create_shader_module` entry point,
and the Android material path builds Naga modules directly.

The supplied HNMOD does not state a license for the port. Keep this local and do not redistribute
publicly until the mod author confirms the applicable license.
