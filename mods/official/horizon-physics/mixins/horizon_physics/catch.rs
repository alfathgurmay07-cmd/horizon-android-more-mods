// ClientLevel.addParticle: a particle the settings want as a physics piece is made one instead.
fn horizon_physics_catch(options: &ParticleOptions, p: [f64; 3], v: [f64; 3]) -> ::core::ops::ControlFlow<()> {
    use crate::native_horizon_physics_sway as sway;
    if sway::catch_particle(options.block.as_ref(), options.item.as_ref(), p, v) {
        return ::core::ops::ControlFlow::Break(());
    }
    ::core::ops::ControlFlow::Continue(())
}
