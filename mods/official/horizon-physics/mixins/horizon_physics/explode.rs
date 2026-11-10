// ClientPacketListener.handleExplosion: the blast throws the blocks it breaks, with a fireball,
// sparks and smoke. A wind charge's burst (no block particles) is left alone. A blast under water
// has no fire or smoke: it throws a spout of water up through the surface instead (the surface is
// found by going up from the blast through the water above it).
fn horizon_physics_explode(game: &mut crate::game::Game<'_>, p: &play::Explode) {
    use horizon_client::native_horizon_physics as hp;
    if !game.native_window() || !game.native_on(hp::ID) || p.block_particles.is_empty() {
        return;
    }
    let facts = game.res.facts();
    let water = |x: i32, y: i32, z: i32| facts.get(game.world.block(x, y, z)).fluid == horizon_client::blocks::Fluid::Water;
    let (cx, cy, cz) = (p.center[0].floor() as i32, p.center[1].floor() as i32, p.center[2].floor() as i32);
    if water(cx, cy, cz) {
        let mut top = cy;
        while top < cy + 40 && water(cx, top + 1, cz) {
            top += 1;
        }
        hp::explosion_wet(p.center, p.radius, f64::from(top) + 0.9);
    } else {
        hp::explosion(p.center, p.radius);
    }
}
