fn freecam_tick(game: &mut crate::game::Game<'_>) {
    crate::game::umbx_freecam_glue::tick(game);
}
