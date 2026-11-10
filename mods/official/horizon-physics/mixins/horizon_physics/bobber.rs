// FishingHookRenderer's bobber on the waves: the game draws its bobbers into the buffer as ever,
// then each one on wavy water is lifted and moved the way the shader moves the water under it (the
// same swell, at the same time), so it rides the surface instead of sinking into it or floating
// above it.
fn horizon_physics_bobber(
    game: &crate::game::Game<'_>,
    camera: glam::DVec3,
    alpha: f64,
    out: &mut Vec<Vertex>,
) -> ::core::ops::ControlFlow<()> {
    use horizon_client::native_horizon_physics as hp;
    use horizon_client_render::native_horizon_physics_sway as sway;
    if !sway::waves_on() || hp::BOBBER.enter() {
        return ::core::ops::ControlFlow::Continue(());
    }
    let start = out.len();
    game.fishing_geometry(camera, alpha, out);
    hp::BOBBER.leave();
    let facts = game.res.facts();
    let water = |x: i32, y: i32, z: i32| facts.get(game.world.block(x, y, z)).fluid == horizon_client::blocks::Fluid::Water;
    let t = sway::water_time(game.world.game_time, alpha);
    let wind = sway::water_wind();
    let height = sway::water_height();
    // The game pushes four corners a bobber, camera-relative, around its centre; every bobber's
    // corners are moved by the lift of the water under that bobber.
    let hooks: Vec<glam::DVec3> = game
        .hook_position(alpha)
        .into_iter()
        .chain(game.remote_hook_ends(alpha).into_iter().map(|(h, _)| h))
        .collect();
    let mut moves: Vec<(glam::Vec3, glam::Vec3)> = Vec::new();
    for hook in hooks {
        let lift = sway::float_lift(hook.to_array(), camera.to_array(), t, wind, height, &water);
        if let Some(l) = lift {
            moves.push(((hook - camera).as_vec3(), glam::Vec3::new(l[0] as f32, l[1] as f32, l[2] as f32)));
        }
    }
    if moves.is_empty() {
        return ::core::ops::ControlFlow::Break(());
    }
    for quad in out[start..].chunks_mut(4) {
        let centre = quad.iter().fold(glam::Vec3::ZERO, |a, v| a + glam::Vec3::from_array(v.pos)) / quad.len() as f32;
        let near = moves
            .iter()
            .map(|m| (m, (centre - m.0).length_squared()))
            .filter(|e| e.1 < 0.5 * 0.5)
            .min_by(|a, b| a.1.total_cmp(&b.1));
        if let Some(hit) = near {
            for v in quad.iter_mut() {
                v.pos = (glam::Vec3::from_array(v.pos) + hit.0 .1).to_array();
            }
        }
    }
    ::core::ops::ControlFlow::Break(())
}
