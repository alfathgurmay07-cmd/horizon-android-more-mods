fn freecam_page(game: &mut crate::game::Game<'_>, mod_id: &str, buttons: &mut Vec<String>) {
    crate::game::umbx_freecam_glue::page(game, mod_id, buttons);
}
