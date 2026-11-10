// Keys on Physics Settings: Escape goes back (a page to the categories, the categories to
// Options); the others do nothing there.
fn horizon_physics_options_key(
    game: &mut crate::game::Game<'_>,
    key: KeyCode,
    text: Option<&str>,
) -> ::core::ops::ControlFlow<Result<()>> {
    use horizon_client::native_horizon_physics as hp;
    let _ = (game, text);
    if !hp::menu_is_open() {
        return ::core::ops::ControlFlow::Continue(());
    }
    if key == KeyCode::Escape {
        hp::menu_escape();
    }
    ::core::ops::ControlFlow::Break(Ok(()))
}
