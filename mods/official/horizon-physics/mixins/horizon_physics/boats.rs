// AbstractBoatRenderer on the waves: the game draws its boats into the buffer as ever, then each
// boat on wavy water is lifted, moved and tilted the way the shader moves the water under it
// (the same swell, at the same time). The boat the player rides is lifted fully, and the camera
// hook lifts the eye by the same amount, so the player rides the swell with it.

fn horizon_physics_boats(
    game: &crate::game::Game<'_>,
    camera: glam::DVec3,
    alpha: f64,
    out: &mut Vec<Vertex>,
) -> ::core::ops::ControlFlow<()> {
    use horizon_client::native_horizon_physics as hp;
    use horizon_client_render::native_horizon_physics_sway as sway;
    if !sway::waves_on() || !hp::settings().boats || hp::BOATS.enter() {
        return ::core::ops::ControlFlow::Continue(());
    }
    let start = out.len();
    game.boat_geometry(camera, alpha, out);
    hp::BOATS.leave();
    let facts = game.res.facts();
    let water = |x: i32, y: i32, z: i32| facts.get(game.world.block(x, y, z)).fluid == horizon_client::blocks::Fluid::Water;
    let t = sway::water_time(game.world.game_time, alpha);
    let wind = sway::water_wind();
    let height = sway::water_height();
    // Each boat: its centre (camera-relative), its lift and sideways move, and its turn.
    let mut moves: Vec<(glam::Vec3, glam::Vec3, glam::Quat)> = Vec::new();
    for b in &game.boats {
        let pos = glam::DVec3::new(
            b.previous.x + (b.position.x - b.previous.x) * alpha,
            b.previous.y + (b.position.y - b.previous.y) * alpha,
            b.previous.z + (b.position.z - b.previous.z) * alpha,
        );
        let (bx, bz) = (pos.x.floor() as i32, pos.z.floor() as i32);
        let base = pos.y.floor() as i32;
        let Some(y) = [base, base - 1, base + 1].into_iter().find(|y| water(bx, *y, bz) && !water(bx, *y + 1, bz)) else {
            continue;
        };
        // How open the water is under it: its four corners' marks, as the shader blends them.
        let (fx, fz) = (pos.x - pos.x.floor(), pos.z - pos.z.floor());
        let code = |cx: i32, cz: i32| f64::from(sway::water_code(cx, y, cz, water, y - 16)) / 7.0;
        let open = code(bx, bz) * (1.0 - fx) * (1.0 - fz)
            + code(bx + 1, bz) * fx * (1.0 - fz)
            + code(bx, bz + 1) * (1.0 - fx) * fz
            + code(bx + 1, bz + 1) * fx * fz;
        if open <= 0.0 {
            continue;
        }
        let rel = pos - camera;
        let reach = sway::water_reach(rel.length());
        let m = sway::water_motion_with(pos.x, pos.z, t, open, wind, height);
        // The boat ridden is lifted fully, as the camera is (the camera hook rides the same swell).
        let riding = game.riding == Some(b.id);
        let k = if riding { 1.0 } else { reach as f32 };
        let lift = glam::Vec3::new(m[1] as f32, (m[0] + 0.04) as f32, m[2] as f32) * k;
        let up = glam::Vec3::new((-m[3] * reach) as f32, 1.0, (-m[4] * reach) as f32).normalize();
        let turn = glam::Quat::from_rotation_arc(glam::Vec3::Y, up);
        moves.push((rel.as_vec3(), lift, turn));
    }
    if moves.is_empty() {
        return ::core::ops::ControlFlow::Break(());
    }
    for v in &mut out[start..] {
        let p = glam::Vec3::from_array(v.pos);
        let near = moves
            .iter()
            .map(|m| (m, (p.x - m.0.x).powi(2) + (p.z - m.0.z).powi(2)))
            .filter(|e| e.1 < 2.4 * 2.4)
            .min_by(|a, b| a.1.total_cmp(&b.1));
        if let Some(hit) = near {
            let m = hit.0;
            v.pos = (m.0 + m.2 * (p - m.0) + m.1).to_array();
        }
    }
    ::core::ops::ControlFlow::Break(())
}
