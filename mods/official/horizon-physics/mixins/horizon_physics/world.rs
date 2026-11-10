// Once a frame: the pieces take a step against the blocks around them and are drawn with the
// other entities, lit by the light where each piece is. Also looks for fire to smoke and puts
// footprints where the player walks on snow.
fn horizon_physics_world(game: &crate::game::Game<'_>, camera: glam::DVec3, out: &mut Vec<Vertex>) {
    use horizon_client::native_horizon_physics as hp;
    if !hp::loaded() {
        hp::load(crate::game::Game::native_config_dir().ok());
    }
    use horizon_client_render::native_horizon_physics_sway as particles;
    // PVP server compatibility: on a server, none of it. Single player also joins a server
    // (its own integrated one, `integrated`), which is not what this setting is about.
    let on = game.native_on(hp::ID) && !hp::pvp_compat_off(hp::settings().pvp, game.remote.is_some(), game.integrated.is_some());
    hp::set_enabled(on);
    if !on {
        hp::clear();
        particles::set_particle_catch(0);
        let _ = particles::take_particles();
        horizon_client_render::native_horizon_physics_sway::snow_clear();
        return;
    }
    // The game's particles that became pieces.
    particles::set_particle_catch(hp::particle_catch());
    for b in particles::take_particles() {
        if let Some(r) = game.gui.atlas.sprites.get(&b.sprite) {
            hp::spawn_bit(b.at, b.v, [r.u0, r.v0, r.u1, r.v1], b.tint, b.spread);
        }
    }
    let facts = game.res.facts();
    // The weather, for the wind's strength.
    let (rain_level, thunder_level) = game.world.weather.levels(1.0);
    hp::set_weather(rain_level, thunder_level);
    // The Despawn Animation page's REPLAY: pieces in front of the player.
    if hp::replay_wanted()
        && let Some(p) = game.player.as_ref()
        && let Some(r) = game.gui.atlas.sprites.get("block/stone")
    {
        hp::replay_at([p.position.x, p.position.y + 1.5, p.position.z], p.yaw, [r.u0, r.v0, r.u1, r.v1]);
    }
    // Doors near the camera: one that opened or shut swings round its hinge (the mesher leaves
    // its own quads out meanwhile), and the swings are drawn here.
    {
        use horizon_client_render::native_horizon_physics_sway as sway;
        if sway::doors_on() {
            let prop = |state, key: &str| -> Option<String> {
                game.pack.assets.registry.state(state).and_then(|s| s.properties.get(key)).cloned()
            };
            let (cx, cy, cz) = (camera.x.floor() as i32, camera.y.floor() as i32, camera.z.floor() as i32);
            for x in cx - 6..=cx + 6 {
                for z in cz - 6..=cz + 6 {
                    for y in cy - 3..=cy + 3 {
                        let state = game.world.block(x, y, z);
                        let f = facts.get(state);
                        if !f.name.ends_with("_door") || prop(state, "half").as_deref() != Some("lower") {
                            continue;
                        }
                        let Some(old) = sway::door_seen([x, y, z], state.0) else {
                            continue;
                        };
                        let mut was_state = state;
                        was_state.0 = old;
                        let old = was_state;
                        let was = facts.get(old);
                        if was.name != f.name || prop(old, "open") == prop(state, "open") || prop(old, "facing") != prop(state, "facing") {
                            continue;
                        }
                        let (Some(from), Some(to)) = (was.outline.first().copied(), f.outline.first().copied()) else {
                            continue;
                        };
                        let id = f.name.trim_start_matches("minecraft:").trim_start_matches("waxed_");
                        let uv = |half: &str| game.gui.atlas.sprites.get(&format!("block/{id}_{half}")).map(|r| [r.u0, r.v0, r.u1, r.v1]);
                        if let (Some(bottom), Some(top)) = (uv("bottom"), uv("top")) {
                            sway::door_swing([x, y, z], from, to, bottom, top, game.world.layout().min_y());
                        }
                    }
                }
            }
            sway::doors_forget(|p| (p[0] - cx).abs() <= 8 && (p[2] - cz).abs() <= 8 && (p[1] - cy).abs() <= 5);
            let light = |x: i32, y: i32, z: i32| -> u32 {
                let l = game.world.light(x, y, z);
                (u32::from(l >> 4) * 16) | ((u32::from(l & 15) * 16) << 8)
            };
            let lit = hp::FnWorld(|_: i32, _: i32, _: i32| hp::Cell::Air, light);
            for (s, angle) in sway::door_swings() {
                hp::door_draw(s.pos, s.from, s.pivot, angle, s.bottom, s.top, [camera.x, camera.y, camera.z], &lit, out);
            }
        }
    }
    // Smoke: one layer of blocks around the camera each frame.
    if hp::smoke_on() {
        let (y, x0, x1, z0, z1) = hp::scan_layer([camera.x, camera.y, camera.z]);
        let mut found: Vec<([i32; 3], u8)> = Vec::new();
        for x in x0..=x1 {
            for z in z0..=z1 {
                let state = game.world.block(x, y, z);
                if game.world.is_air(state) {
                    continue;
                }
                let kind: u8 = match &*facts.get(state).name {
                    "minecraft:fire" => 1,
                    "minecraft:soul_fire" => 2,
                    "minecraft:campfire" => 3,
                    "minecraft:soul_campfire" => 4,
                    "minecraft:lava" => 5,
                    _ => continue,
                };
                if kind == 3 || kind == 4 {
                    let lit = game
                        .pack
                        .assets
                        .registry
                        .state(state)
                        .and_then(|s| s.properties.get("lit"))
                        .is_some_and(|v| v == "true");
                    if !lit {
                        continue;
                    }
                }
                if kind == 5 && !game.world.is_air(game.world.block(x, y + 1, z)) {
                    continue;
                }
                found.push(([x, y, z], kind));
            }
        }
        hp::scan_found(y, &found);
    }
    // Snow: the player and the mobs on it press it down where they walk (the mesher hook draws
    // the pressed blocks), and each step kicks up a little powder.
    {
        use horizon_client_render::native_horizon_physics_sway as sway;
        sway::snow_world(game.dim as u64, game.world.layout().min_y());
        sway::set_snow_depth(hp::snow_depth());
        sway::set_snow_deep(hp::snow_layers());
        if !hp::snow_on() {
            sway::snow_clear();
        } else {
            // A block's snow top (its y plus its height), None for a block that is not snow.
            let top = |x: i32, y: i32, z: i32| -> Option<f64> {
                let state = game.world.block(x, y, z);
                match &*facts.get(state).name {
                    // A whole block of snow (or powder snow) gives way like a layer does.
                    "minecraft:snow_block" | "minecraft:powder_snow" => Some(f64::from(y) + 1.0),
                    "minecraft:snow" => {
                        let layers = game
                            .pack
                            .assets
                            .registry
                            .state(state)
                            .and_then(|s| s.properties.get("layers"))
                            .and_then(|v| v.parse::<f64>().ok())
                            .unwrap_or(1.0);
                        Some(f64::from(y) + layers / 8.0)
                    }
                    _ => None,
                }
            };
            // The snow under feet at (x, y, z): its top, when the feet are on it or sunk into it.
            let under = |x: f64, y: f64, z: f64| -> Option<f64> {
                let (bx, bz) = (x.floor() as i32, z.floor() as i32);
                for by in [y.floor() as i32, (y - 0.05).floor() as i32, y.floor() as i32 - 1] {
                    if let Some(t) = top(bx, by, bz)
                        && t - y < 0.2
                        && y - t < 0.14
                    {
                        return Some(t);
                    }
                }
                None
            };
            let mut player_top: Option<f64> = None;
            if let Some(p) = game.player.as_ref()
                && p.ground.on_ground
                && !p.flying
                && !p.spectator
                && !p.swimming
            {
                let feet = p.position;
                player_top = under(feet.x, feet.y, feet.z);
                if let Some(t) = player_top {
                    let dir = [p.velocity.x, p.velocity.z];
                    let yaw = f64::from(p.yaw).to_radians();
                    let dir = if dir[0].abs() + dir[1].abs() > 1e-4 { dir } else { [-yaw.sin(), yaw.cos()] };
                    sway::snow_press([feet.x, t, feet.z], 0.6, dir, top);
                }
            }
            if let Some(p) = game.player.as_ref() {
                hp::footstep([p.position.x, p.position.y, p.position.z], p.yaw, player_top);
            }
            let mobs_too = hp::mob_trails_on();
            for m in &game.mobs {
                if !mobs_too || !m.on_ground || m.health <= 0.0 {
                    continue;
                }
                let at = m.position;
                if (at.x - camera.x).abs() > 48.0 || (at.z - camera.z).abs() > 48.0 {
                    continue;
                }
                if let Some(t) = under(at.x, at.y, at.z) {
                    let dir = [at.x - m.previous.x, at.z - m.previous.z];
                    let yaw = f64::from(m.yaw).to_radians();
                    let dir = if dir[0].abs() + dir[1].abs() > 1e-4 { dir } else { [-yaw.sin(), yaw.cos()] };
                    sway::snow_press([at.x, t, at.z], m.kind.width.clamp(0.3, 1.6), dir, top);
                }
            }
        }
    }
    // Rain splashing on the ground around the camera, snow drifting down where it snows, and
    // sand blown off dry ground in the wind.
    if hp::weather_on() {
        let st = hp::settings();
        static SEED: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0x9E37_79B9_7F4A_7C15);
        let mut next = || {
            let mut x = SEED.load(std::sync::atomic::Ordering::Relaxed);
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            SEED.store(x, std::sync::atomic::Ordering::Relaxed);
            (x >> 11) as f64 / (1u64 << 53) as f64
        };
        let rain = rain_level;
        if rain > 0.2 {
            let amount = f64::from(rain) * f64::from(st.rain_amount) * 2.8 + f64::from(thunder_level) * f64::from(st.thunder_amount) * 2.8;
            for _ in 0..amount.round() as usize {
                let x = camera.x + (next() - 0.5) * 24.0;
                let z = camera.z + (next() - 0.5) * 24.0;
                let (bx, bz) = (x.floor() as i32, z.floor() as i32);
                let y = game.rain_height(bx, bz);
                if (f64::from(y) - camera.y).abs() > 16.0 {
                    continue;
                }
                match game.precipitation(bx, y, bz) {
                    Some(false) => {
                        // On water: a ring on the surface; elsewhere: drops thrown up.
                        let below = game.world.block(bx, y - 1, bz);
                        if facts.get(below).fluid == horizon_client::blocks::Fluid::Water {
                            hp::rain_ripple([x, f64::from(y) - 0.11, z]);
                        } else {
                            hp::rain_splash([x, f64::from(y), z]);
                        }
                    }
                    // Snow: a flake starts above, a little at a time (they live long).
                    Some(true) if next() < 0.35 => {
                        let top = (camera.y + 4.0 + next() * 8.0).max(f64::from(y) + 1.0);
                        hp::snowflake([x, top, z]);
                    }
                    _ => {}
                }
            }
        }
        let wind = hp::wind_now();
        if wind > 0.5 {
            for _ in 0..(wind * 2.0).round() as usize {
                let x = camera.x + (next() - 0.5) * 20.0;
                let z = camera.z + (next() - 0.5) * 20.0;
                let (bx, bz) = (x.floor() as i32, z.floor() as i32);
                let y = game.rain_height(bx, bz);
                if (f64::from(y) - camera.y).abs() > 12.0 || game.precipitation(bx, y, bz).is_some() {
                    continue;
                }
                let red = match &*facts.get(game.world.block(bx, y - 1, bz)).name {
                    "minecraft:sand" => false,
                    "minecraft:red_sand" => true,
                    _ => continue,
                };
                hp::sand_dust([x, f64::from(y), z], red);
            }
        }
    }
    // Splashes: the player's feet going into water fast.
    if hp::liquids_on()
        && let Some(p) = game.player.as_ref()
    {
        let feet = p.position;
        let state = game.world.block(feet.x.floor() as i32, (feet.y + 0.1).floor() as i32, feet.z.floor() as i32);
        let wet = facts.get(state).fluid == horizon_client::blocks::Fluid::Water;
        // Velocity is in blocks a tick.
        hp::player_water([feet.x, feet.y, feet.z], wet, -p.velocity.y * 20.0);
    }
    // The player shoves the ragdolls and pieces in their way.
    hp::set_player(game.player.as_ref().filter(|p| !p.spectator).map(|p| {
        ([p.position.x, p.position.y, p.position.z], [p.velocity.x * 20.0, p.velocity.y * 20.0, p.velocity.z * 20.0])
    }));
    let cell = |x: i32, y: i32, z: i32| -> hp::Cell {
        let state = game.world.block(x, y, z);
        if game.world.is_air(state) {
            return hp::Cell::Air;
        }
        let f = facts.get(state);
        if f.full_collision {
            return hp::Cell::Full;
        }
        if f.legacy_solid && !f.outline.is_empty() {
            return hp::Cell::Boxes(f.outline.clone());
        }
        if f.fluid != horizon_client::blocks::Fluid::None {
            return hp::Cell::Fluid;
        }
        hp::Cell::Air
    };
    let light = |x: i32, y: i32, z: i32| -> u32 {
        let l = game.world.light(x, y, z);
        (u32::from(l >> 4) * 16) | ((u32::from(l & 15) * 16) << 8)
    };
    let world = hp::FnWorld(cell, light);
    let w = game.gui.atlas.font.white();
    hp::frame(game.dim as u8, [camera.x, camera.y, camera.z], [w.u0, w.v0, w.u1, w.v1], &world, out);
}
