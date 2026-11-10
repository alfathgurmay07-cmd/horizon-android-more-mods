// The dust under a sprinting player's feet: the particles made inside are sprint dust.
fn horizon_physics_sprint(game: &mut crate::game::Game<'_>) -> ::core::ops::ControlFlow<()> {
    use horizon_client_render::native_horizon_physics_sway as sway;
    if sway::particle_source() == sway::SRC_SPRINT {
        return ::core::ops::ControlFlow::Continue(());
    }
    let before = sway::set_particle_source(sway::SRC_SPRINT);
    game.sprint_dust();
    sway::set_particle_source(before);
    ::core::ops::ControlFlow::Break(())
}
