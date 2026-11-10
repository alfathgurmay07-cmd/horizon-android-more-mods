// CampfireBlock.makeParticles: while the mod's smoke is on, it is the only smoke.
fn horizon_physics_campfire_smoke(pos: [i32; 3]) -> ::core::ops::ControlFlow<()> {
    use crate::native_horizon_physics as hp;
    let _ = pos;
    if hp::replaces_vanilla_smoke() {
        ::core::ops::ControlFlow::Break(())
    } else {
        ::core::ops::ControlFlow::Continue(())
    }
}
