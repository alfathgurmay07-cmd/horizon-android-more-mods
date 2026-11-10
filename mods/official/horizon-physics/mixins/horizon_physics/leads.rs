// Leads as ropes: the game draws its leads into a buffer of our own, where each one is read for
// its two ends and replaced by a rope that sags, swings and lies on blocks. The knots stay.
fn horizon_physics_leads(
    game: &crate::game::Game<'_>,
    camera: glam::DVec3,
    alpha: f64,
    out: &mut Vec<Vertex>,
) -> ::core::ops::ControlFlow<()> {
    use horizon_client::native_horizon_physics as hp;
    if !hp::ropes_on() || hp::LEADS.enter() {
        return ::core::ops::ControlFlow::Continue(());
    }
    let mut drawn: Vec<Vertex> = Vec::new();
    game.leash_geometry(camera, alpha, &mut drawn);
    hp::LEADS.leave();
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
    hp::leads(&drawn, [camera.x, camera.y, camera.z], &world, out);
    ::core::ops::ControlFlow::Break(())
}
