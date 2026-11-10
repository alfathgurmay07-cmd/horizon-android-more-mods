// ItemEntityRenderer with item physics: a dropped item tumbles while it flies and settles on its
// flattest side on the ground (flat items lie on their face, blocks on a side), with no bobbing
// or spinning. Items being picked up fly to the player as usual. Otherwise this is the game's own
// drawing of dropped items (the same lighting, stack copies, glow outline and culling).
fn horizon_physics_items(
    game: &crate::game::Game<'_>,
    camera: glam::DVec3,
    alpha: f64,
    out: &mut Vec<Vertex>,
) -> ::core::ops::ControlFlow<()> {
    use horizon_client::native_horizon_physics as hp;
    if !hp::items_on() {
        return ::core::ops::ControlFlow::Continue(());
    }
    let live = game.dropped.iter().map(|item| {
        let (p, q) = (item.previous, item.position);
        let at = [
            p.x + (q.x - p.x) * alpha,
            p.y + (q.y - p.y) * alpha,
            p.z + (q.z - p.z) * alpha,
        ];
        (item, at, alpha as f32, true)
    });
    let taken = game.pickups.live.iter().filter_map(|p| match &p.taken {
        super::pickups::Taken::Item(item) => {
            Some((&**item, super::pickups::position(p, alpha), 1.0, false))
        }
        _ => None,
    });
    for (item, at, partial, physics) in live.chain(taken) {
        let entity_start = out.len();
        let q = item.position;
        let pos = glam::DVec3::from_array(at) - camera;
        if pos.length_squared() > 64.0 * 64.0 {
            continue;
        }
        let owner = owner_at([q.x, q.y, q.z], 0.0, "minecraft:item");
        let state = game.resolve_item(
            &item.stack,
            DisplayContext::Ground,
            Some(&owner),
            item.id as i32,
        );
        if state.is_empty() {
            continue;
        }
        let (lo, hi) = state.bounding_box();
        let pose = if physics {
            let size = hi - lo;
            // Thin front to back: a flat item (most items); otherwise a block or a 3D model.
            let flat = size.z < 0.25 * size.x.max(size.y);
            let p = item.previous;
            let velocity = [(q.x - p.x) * 20.0, (q.y - p.y) * 20.0, (q.z - p.z) * 20.0];
            let resting = item.on_ground || ((q.y - p.y).abs() < 1.0e-4 && item.velocity.y.abs() < 1.0e-3);
            let r = hp::item_rotation(item.id, resting, velocity, flat);
            let rot = glam::Quat::from_xyzw(r[0], r[1], r[2], r[3]);
            let center = (lo + hi) * 0.5;
            let half = (hi - lo) * 0.5;
            // How far the turned model reaches below its centre: it rests there on the ground.
            let below = (rot * glam::Vec3::new(half.x, 0.0, 0.0)).y.abs()
                + (rot * glam::Vec3::new(0.0, half.y, 0.0)).y.abs()
                + (rot * glam::Vec3::new(0.0, 0.0, half.z)).y.abs();
            glam::Mat4::from_translation(pos.as_vec3())
                * glam::Mat4::from_translation(glam::Vec3::new(0.0, below + 0.01, 0.0))
                * glam::Mat4::from_quat(rot)
                * glam::Mat4::from_translation(-center)
        } else {
            let age = item.age as f32 + partial;
            let min_offset_y = -lo.y + 0.0625;
            let bob = (age / 10.0 + item.bob_offset).sin() * 0.1 + 0.1;
            let spin = item_render::item_spin(age, item.bob_offset);
            glam::Mat4::from_translation(pos.as_vec3())
                * glam::Mat4::from_translation(glam::Vec3::new(0.0, bob + min_offset_y, 0.0))
                * glam::Mat4::from_rotation_y(spin)
        };
        let packed = game.world.light(
            q.x.floor() as i32,
            (q.y + 0.1).floor() as i32,
            q.z.floor() as i32,
        );
        let seed = i64::from(game.pack.item_renderer.item_id(&item.stack.item))
            + i64::from(item.stack.damage);
        for copy in item_render::cluster_poses(
            &state,
            pose,
            item_render::rendered_amount(item.stack.count),
            seed,
        ) {
            game.submit_item_world(&state, copy, vertex_light(packed), out);
        }
        if game.entity_glowing(item.id) {
            let rgb = super::glowing::team_color(game.mob_team(item.id));
            game.mark_outline(&out[entity_start..], rgb);
        } else {
            game.cull_entity_vertices(out, entity_start);
        }
    }
    ::core::ops::ControlFlow::Break(())
}
