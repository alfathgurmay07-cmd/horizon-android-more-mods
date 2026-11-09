fn freecam_key(game: &mut crate::game::Game<'_>, key: &str, pressed: bool) {
    crate::game::umbx_freecam_glue::key(game, key, pressed);
}
