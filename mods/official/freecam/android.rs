use horizon_client::Result;
use winit::event::WindowEvent;
use winit::keyboard::KeyCode;

pub(super) fn key(game: &mut crate::game::Game<'_>, key: &str, pressed: bool) {
    crate::game::umbx_freecam_glue::key(game, key, pressed);
}

pub(super) fn tick(game: &mut crate::game::Game<'_>) {
    crate::game::umbx_freecam_glue::tick(game);
}

pub(super) fn list(mods: &mut Vec<crate::game::native_hooks::NativeMod>) {
    crate::game::umbx_freecam_glue::listed(mods);
}

pub(super) fn settings(
    game: &crate::game::Game<'_>,
    mod_id: &str,
    buttons: &mut Vec<String>,
) {
    crate::game::umbx_freecam_glue::page(game, mod_id, buttons);
}

pub(super) fn click(game: &mut crate::game::Game<'_>, mod_id: &str, button: usize) {
    crate::game::umbx_freecam_glue::click(game, mod_id, button);
}

pub(super) fn wheel(game: &mut crate::game::Game<'_>, steps: f32, claimed: &mut bool) {
    crate::game::umbx_freecam_glue::wheel(game, steps, claimed);
}

pub(super) fn window(
    game: &mut crate::game::Game<'_>,
    event: &WindowEvent,
) -> std::ops::ControlFlow<()> {
    crate::game::umbx_freecam_glue::window(game, event)
}

pub(super) fn mapped(
    game: &mut crate::game::Game<'_>,
    key: KeyCode,
    pressed: bool,
    repeat: bool,
) -> std::ops::ControlFlow<Result<()>> {
    crate::game::umbx_freecam_glue::mapped(game, key, pressed, repeat)
}

pub(super) fn turn(xo: f64, yo: f64) -> std::ops::ControlFlow<()> {
    crate::game::umbx_freecam_glue::turn(xo, yo)
}

pub(super) fn camera(
    partial: f32,
) -> std::ops::ControlFlow<Option<(glam::DVec3, f32, f32)>> {
    crate::game::umbx_freecam_glue::camera(partial)
}
