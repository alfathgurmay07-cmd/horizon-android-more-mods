// The title screen, while the quick setup has not been done: the quick setup in its place.
fn horizon_physics_setup_draw(
    _title: &Title,
    atlas: &HudAtlas,
    batch: &mut HudBatch,
    cursor: (f32, f32),
    _hide_splash: bool,
) -> ::core::ops::ControlFlow<()> {
    if !crate::native_horizon_physics::setup_visible() {
        return ::core::ops::ControlFlow::Continue(());
    }
    crate::native_horizon_physics_setup::draw(atlas, batch, cursor);
    ::core::ops::ControlFlow::Break(())
}
