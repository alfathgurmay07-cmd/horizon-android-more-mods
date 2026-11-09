fn freecam_click(game: &mut crate::game::Game<'_>, mod_id: &str, button: usize) {
    crate::game::umbx_freecam_glue::click(game, mod_id, button);
}
