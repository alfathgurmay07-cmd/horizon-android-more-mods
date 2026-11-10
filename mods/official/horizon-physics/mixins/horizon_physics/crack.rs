// The cracks of mining: the particles made inside are the crack's (they may become pieces).
fn horizon_physics_crack(
    game: &mut crate::game::Game<'_>,
    pos: [i32; 3],
    face: [i32; 3],
    state: BlockStateId,
) -> ::core::ops::ControlFlow<()> {
    use horizon_client_render::native_horizon_physics_sway as sway;
    if sway::particle_source() == sway::SRC_CRACK {
        return ::core::ops::ControlFlow::Continue(());
    }
    let before = sway::set_particle_source(sway::SRC_CRACK);
    game.crack_particle(pos, face, state);
    sway::set_particle_source(before);
    ::core::ops::ControlFlow::Break(())
}
