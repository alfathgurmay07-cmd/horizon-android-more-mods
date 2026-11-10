// The pointer moving with the button down on Physics Settings: a slider follows it.
fn horizon_physics_options_drag(game: &mut crate::game::Game<'_>) -> ::core::ops::ControlFlow<Result<()>> {
    use horizon_client::native_horizon_physics as hp;
    if !hp::menu_is_open() {
        return ::core::ops::ControlFlow::Continue(());
    }
    let (w, _) = game.gui_size();
    hp::menu_drag(w.ceil(), game.cursor.0);
    ::core::ops::ControlFlow::Break(Ok(()))
}
