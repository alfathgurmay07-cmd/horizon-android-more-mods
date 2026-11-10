// A broken block falls apart into pieces of its own texture instead of the usual particles. Only
// in the window's game: the server thread has no picture to draw them in.
fn horizon_physics_destroy(
    game: &mut crate::game::Game<'_>,
    pos: [i32; 3],
    state: BlockStateId,
) -> ::core::ops::ControlFlow<()> {
    use horizon_client::native_horizon_physics as hp;
    use horizon_client_render::native_horizon_physics_sway as sway;
    // The game's own particles, kept with the pieces (below): the original runs.
    if sway::particle_source() == sway::SRC_KEEP {
        return ::core::ops::ControlFlow::Continue(());
    }
    // A block a collapse took away: it falls whole, no pieces.
    if hp::take_silenced(pos) {
        return ::core::ops::ControlFlow::Break(());
    }
    if !game.native_window() || !game.native_on(hp::ID) || !hp::settings().blocks {
        return ::core::ops::ControlFlow::Continue(());
    }
    let Some((sprite, tint)) = game.particle_sprite(state, pos) else {
        return ::core::ops::ControlFlow::Continue(());
    };
    let Some(r) = game.gui.atlas.sprites.get(&sprite).copied() else {
        return ::core::ops::ControlFlow::Continue(());
    };
    let boxes = game.particle_shape(pos, state);
    if boxes.is_empty() {
        return ::core::ops::ControlFlow::Continue(());
    }
    hp::spawn_block(pos, &boxes, [r.u0, r.v0, r.u1, r.v1], tint);
    // MC Break Particles: the game's own too.
    if hp::settings().mc_break {
        let before = sway::set_particle_source(sway::SRC_KEEP);
        game.destroy_particles(pos, state);
        sway::set_particle_source(before);
    }
    ::core::ops::ControlFlow::Break(())
}
