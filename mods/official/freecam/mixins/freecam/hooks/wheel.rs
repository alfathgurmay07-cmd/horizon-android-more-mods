fn freecam_wheel(game: &mut crate::game::Game<'_>, steps: f32, claimed: &mut bool) {
    crate::game::umbx_freecam_glue::wheel(game, steps, claimed);
}
