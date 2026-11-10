// The wheel on Physics Settings: the categories scroll.
fn horizon_physics_options_scroll(game: &mut crate::game::Game<'_>, amount: f32) -> ::core::ops::ControlFlow<Result<()>> {
    use horizon_client::native_horizon_physics as hp;
    if !hp::menu_is_open() {
        return ::core::ops::ControlFlow::Continue(());
    }
    let (_, h) = game.gui_size();
    hp::menu_scroll(h.ceil(), amount);
    ::core::ops::ControlFlow::Break(Ok(()))
}
