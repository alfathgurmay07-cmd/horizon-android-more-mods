// The mouse on the Options screen: "Physics Settings..." opens Physics Settings, and while it is
// open every press is its own.
fn horizon_physics_options_mouse(
    game: &mut crate::game::Game<'_>,
    pressed: bool,
    right: bool,
) -> ::core::ops::ControlFlow<Result<()>> {
    use horizon_client::native_horizon_physics as hp;
    use horizon_client_render::native_horizon_physics_sway as sway;
    let (x, y) = game.cursor;
    if !hp::menu_is_open() {
        if pressed && !right && game.ui.options.page() == Page::Main && sway::physics_button_at(x, y) {
            hp::menu_open();
            return ::core::ops::ControlFlow::Break(game.handle_option_events(vec![Event::ButtonClick]));
        }
        return ::core::ops::ControlFlow::Continue(());
    }
    if right {
        return ::core::ops::ControlFlow::Break(Ok(()));
    }
    game.dragging = pressed;
    let (w, h) = game.gui_size();
    let out = match hp::menu_mouse(w.ceil(), h.ceil(), x, y, pressed) {
        hp::MenuOut::None => Ok(()),
        hp::MenuOut::Click | hp::MenuOut::Close => game.handle_option_events(vec![Event::ButtonClick]),
    };
    ::core::ops::ControlFlow::Break(out)
}
