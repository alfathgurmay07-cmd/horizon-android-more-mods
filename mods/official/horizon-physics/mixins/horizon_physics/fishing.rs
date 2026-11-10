// FishingHookRenderer's line as a rope: from a quarter block above the hook to the rod, it sags,
// swings and lies on blocks.
fn horizon_physics_fishing(
    game: &crate::game::Game<'_>,
    alpha: f64,
    size: [u32; 2],
) -> ::core::ops::ControlFlow<Vec<Line>> {
    use horizon_client::native_horizon_physics as hp;
    if !hp::ropes_on() {
        return ::core::ops::ControlFlow::Continue(());
    }
    let ends: Vec<(glam::DVec3, glam::DVec3)> = game
        .hook_position(alpha)
        .zip(game.rod_hand_position(alpha, size))
        .into_iter()
        .chain(game.remote_hook_ends(alpha))
        .collect();
    let width = horizon_client::gfx::outline_width(size[0]);
    let facts = game.res.facts();
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
        hp::Cell::Air
    };
    let world = hp::FnWorld(cell, |_: i32, _: i32, _: i32| 0u32);
    // Flowing water: a water block's level (0 a source, up to 7 the thinnest, 8 and more falling)
    // and, from its neighbours, which way it runs: toward the lower surface (a bigger level).
    let level = |x: i32, y: i32, z: i32| -> Option<i32> {
        let state = game.world.block(x, y, z);
        if facts.get(state).fluid != horizon_client::blocks::Fluid::Water {
            return None;
        }
        Some(
            game.pack
                .assets
                .registry
                .state(state)
                .and_then(|s| s.properties.get("level"))
                .and_then(|v| v.parse::<i32>().ok())
                .unwrap_or(0),
        )
    };
    let current = |p: glam::DVec3| -> (f64, f64) {
        let (x, y, z) = (p.x.floor() as i32, p.y.floor() as i32, p.z.floor() as i32);
        let Some(here) = level(x, y, z) else {
            return (0.0, 0.0);
        };
        let mut flow = (0.0, 0.0);
        for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            if let Some(n) = level(x + dx, y, z + dz) {
                let d = f64::from(n.min(8) - here.min(8));
                flow.0 += f64::from(dx) * d;
                flow.1 += f64::from(dz) * d;
            }
        }
        flow
    };
    let follow = hp::rod_current();
    // On waves the bobber rides the swell (the bobber hook), and so does the line's end on it; the rod's hand stands for the camera
    // in the distance fade.
    let wavy = {
        use horizon_client_render::native_horizon_physics_sway as sway;
        sway::waves_on().then(|| (sway::water_time(game.world.game_time, alpha), sway::water_wind(), sway::water_height()))
    };
    let water = |x: i32, y: i32, z: i32| facts.get(game.world.block(x, y, z)).fluid == horizon_client::blocks::Fluid::Water;
    let mut lines = Vec::new();
    for (hook, hand) in ends {
        let lift = wavy
            .and_then(|(t, wind, height)| {
                horizon_client_render::native_horizon_physics_sway::float_lift(hook.to_array(), hand.to_array(), t, wind, height, &water)
            })
            .map_or(glam::DVec3::ZERO, |l| glam::DVec3::new(l[0], l[1], l[2]));
        let a = hook + lift + glam::DVec3::new(0.0, 0.25, 0.0);
        let mut pts = hp::rope(1, [a.x, a.y, a.z], [hand.x, hand.y, hand.z], 0.05, &world);
        // In flowing water the line bows downstream, most in the middle, the hook end held.
        if follow {
            let flow = current(hook);
            let mag = (flow.0 * flow.0 + flow.1 * flow.1).sqrt();
            if mag > 1e-3 {
                let dir = (flow.0 / mag, flow.1 / mag);
                let bow = (mag * 0.25).min(0.6);
                let last = pts.len().saturating_sub(1).max(1) as f64;
                for (i, p) in pts.iter_mut().enumerate() {
                    let wet = level(p[0].floor() as i32, p[1].floor() as i32, p[2].floor() as i32).is_some();
                    if wet {
                        let k = bow * (std::f64::consts::PI * i as f64 / last).sin();
                        p[0] += dir.0 * k;
                        p[2] += dir.1 * k;
                    }
                }
            }
        }
        for w in pts.windows(2) {
            lines.push(horizon_client::gizmos::Line { start: w[0], end: w[1], color: 0xFF00_0000, width });
        }
    }
    ::core::ops::ControlFlow::Break(lines)
}
