// Without a shader pack picked, Horizon draws terrain with a shader no mod can reach. So, while
// the waves or the sway are on and no pack is picked, the game runs the mod's own pack: vanilla's
// look through the material stage, which the mod's shader hook gives the waves, the water's light
// and the swaying plants. Picking any pack hands the world back to that pack.
fn horizon_physics_builtin(game: &mut crate::game::Game<'_>) {
    use horizon_client::native_horizon_physics as hp;
    use horizon_client_render::native_horizon_physics_sway as sway;
    // The pieces' landing sounds, and the wind's (out under the sky).
    for (event, at, volume, pitch) in hp::take_sounds() {
        game.audio.play(event, Some(at), volume, pitch);
    }
    if let Some(p) = game.player.as_ref() {
        let eye = [p.position.x.floor() as i32, (p.position.y + 1.6).floor() as i32, p.position.z.floor() as i32];
        let outdoors = (game.world.light(eye[0], eye[1], eye[2]) >> 4) >= 12;
        let at = [p.position.x, p.position.y + 1.6, p.position.z];
        if let Some((volume, pitch)) = hp::wind_sound(outdoors) {
            game.audio.play("minecraft:item.elytra.flying", Some(at), volume, pitch);
        }
    }
    // Collapses that waited their ticks fall now.
    for pos in hp::due_collapses() {
        hp::set_collapse_replay(pos);
        let _ = game.break_block(pos);
    }
    // The sections where the snow was pressed are meshed again.
    let sections = game.world.layout().height() / 16;
    for k in sway::snow_take_dirty().into_iter().chain(sway::doors_take_dirty()) {
        if (0..sections as i32).contains(&k.1) {
            game.world.dirty_sections.insert(k);
        }
    }
    static DEV: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    if *DEV.get_or_init(|| std::env::var_os("HORIZON_SHADER_PACK").is_some()) {
        return;
    }
    if !game.settings.horizon.shader_pack.is_empty() || !game.native_shaders.picked.is_empty() {
        hp::set_builtin_running(false);
        return;
    }
    if !hp::loaded() {
        hp::load(crate::game::Game::native_config_dir().ok());
    }
    let st = hp::settings();
    let wanted = game.native_on(hp::ID) && (st.waves || st.plants || st.sway_leaves || st.sway_vines || st.sway_water);
    if !wanted {
        if hp::builtin_running() {
            hp::set_builtin_running(false);
            game.native_shaders = super::native_shaders::NativeShaders::picking("");
            if let Some(renderer) = &mut game.renderer {
                renderer.drop_shader_packs();
            }
        }
        return;
    }
    let Some(dir) = hp::builtin_pack_dir(crate::game::Game::native_config_dir().ok()) else {
        return;
    };
    if !hp::builtin_running() {
        // Written once a session: the files on disk are the mod's, kept as this version made them.
        let files = [("horizon-shaders.toml", sway::BUILTIN_MANIFEST), ("material.wgsl", sway::BUILTIN_MATERIAL)];
        let ok = std::fs::create_dir_all(&dir).is_ok()
            && files.iter().all(|file| {
                let path = dir.join(file.0);
                std::fs::read_to_string(&path).is_ok_and(|t| t == file.1) || std::fs::write(&path, file.1).is_ok()
            });
        if !ok {
            return;
        }
        hp::set_builtin_running(true);
        game.native_shaders = super::native_shaders::NativeShaders::picking("");
    }
    game.tick_native_shader_pack(&dir);
}
