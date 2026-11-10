// An item's particles: eating, or (five at once) a tool or armour piece that broke.
fn horizon_physics_eat(game: &mut crate::game::Game<'_>, stack: &ItemStack, count: u32) -> ::core::ops::ControlFlow<()> {
    use horizon_client_render::native_horizon_physics_sway as sway;
    let src = if count == 5 { sway::SRC_ITEM_BREAK } else { sway::SRC_EAT };
    if sway::particle_source() == src {
        return ::core::ops::ControlFlow::Continue(());
    }
    let before = sway::set_particle_source(src);
    game.eat_particles(stack, count);
    sway::set_particle_source(before);
    ::core::ops::ControlFlow::Break(())
}
