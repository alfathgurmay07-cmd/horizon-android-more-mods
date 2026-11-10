// Camera.setup on the waves: riding a boat, or swimming at the surface, the eye rides the swell
// the shader moves the water by (the boat is lifted by the same amount), so what the game thinks
// (above the water, in the boat) is also what is seen.
fn horizon_physics_camera(game: &mut crate::game::Game<'_>, partial: f32) -> ::core::ops::ControlFlow<Option<(glam::DVec3, f32, f32)>> {
    use horizon_client::native_horizon_physics as hp;
    use horizon_client_render::native_horizon_physics_sway as sway;
    if !sway::waves_on() || !hp::settings().boats {
        hp::set_rider_lift(None);
        return ::core::ops::ControlFlow::Continue(());
    }
    if hp::CAMERA.enter() {
        return ::core::ops::ControlFlow::Continue(());
    }
    let view = game.camera_view(partial);
    hp::CAMERA.leave();
    hp::set_rider_lift(None);
    let Some((mut at, yaw, pitch)) = view else {
        return ::core::ops::ControlFlow::Break(None);
    };
    let facts = game.res.facts();
    let water = |x: i32, y: i32, z: i32| facts.get(game.world.block(x, y, z)).fluid == horizon_client::blocks::Fluid::Water;
    // Where on the water: under the boat ridden, else the player when swimming at the surface.
    let alpha = f64::from(partial);
    let riding = game.riding.and_then(|id| game.boats.iter().find(|b| b.id == id));
    let spot = match (riding, game.player.as_ref()) {
        (Some(b), _) => Some(glam::DVec3::new(
            b.previous.x + (b.position.x - b.previous.x) * alpha,
            b.previous.y + (b.position.y - b.previous.y) * alpha,
            b.previous.z + (b.position.z - b.previous.z) * alpha,
        )),
        (None, Some(p)) if p.swimming || water(p.position.x.floor() as i32, (p.position.y + 0.4).floor() as i32, p.position.z.floor() as i32) => {
            Some(glam::DVec3::new(p.position.x, p.position.y, p.position.z))
        }
        _ => None,
    };
    let Some(pos) = spot else {
        return ::core::ops::ControlFlow::Break(Some((at, yaw, pitch)));
    };
    let (bx, bz) = (pos.x.floor() as i32, pos.z.floor() as i32);
    let base = pos.y.floor() as i32;
    let Some(y) = [base, base - 1, base + 1, base + 2].into_iter().find(|y| water(bx, *y, bz) && !water(bx, *y + 1, bz)) else {
        return ::core::ops::ControlFlow::Break(Some((at, yaw, pitch)));
    };
    // Only near the surface: deep under it the swell does not reach the eye.
    let surface = f64::from(y) + 0.889;
    if riding.is_none() && (at.y - surface).abs() > 1.2 {
        return ::core::ops::ControlFlow::Break(Some((at, yaw, pitch)));
    }
    let (fx, fz) = (pos.x - pos.x.floor(), pos.z - pos.z.floor());
    let code = |cx: i32, cz: i32| f64::from(sway::water_code(cx, y, cz, water, y - 16)) / 7.0;
    let open = code(bx, bz) * (1.0 - fx) * (1.0 - fz) + code(bx + 1, bz) * fx * (1.0 - fz) + code(bx, bz + 1) * (1.0 - fx) * fz + code(bx + 1, bz + 1) * fx * fz;
    if open > 0.0 {
        let t = sway::water_time(game.world.game_time, alpha);
        let m = sway::water_motion_with(pos.x, pos.z, t, open, sway::water_wind(), sway::water_height());
        let lift = [m[1], m[0] + 0.04, m[2]];
        // The eye stays on the side of the water the game thinks it is on (above it: no
        // underwater fog; below it: the fog), and on the same side of the waved surface, so a
        // trough never leaves the eye in the fog above the water, nor a crest the eye in the air
        // under it.
        let waved = surface + m[0];
        let raised = at.y + lift[1];
        let y = if at.y >= surface { raised.max(surface + 0.03).max(waved + 0.12) } else { raised.min(surface - 0.03).min(waved - 0.12) };
        at += glam::DVec3::new(lift[0], y - at.y, lift[2]);
        if riding.is_some() {
            hp::set_rider_lift(Some(lift));
        }
    }
    ::core::ops::ControlFlow::Break(Some((at, yaw, pitch)))
}
