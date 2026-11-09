fn freecam_window(game: &mut crate::game::Game<'_>, event: &winit::event::WindowEvent) -> core::ops::ControlFlow<()> {
    crate::game::umbx_freecam_glue::window(game, event)
}
