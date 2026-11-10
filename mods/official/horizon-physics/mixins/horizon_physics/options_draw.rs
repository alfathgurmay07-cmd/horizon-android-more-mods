// The Options screen, while Physics Settings is open: Physics Settings in its place.
fn horizon_physics_options_draw(game: &mut crate::game::Game<'_>, batch: &mut HudBatch) -> ::core::ops::ControlFlow<()> {
    if !horizon_client::native_horizon_physics::menu_is_open() {
        return ::core::ops::ControlFlow::Continue(());
    }
    horizon_client::native_horizon_physics_setup::draw_menu(&game.gui.atlas, batch, game.cursor);
    ::core::ops::ControlFlow::Break(())
}
