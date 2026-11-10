// The large smoke of fire (FireBlock.animateTick): while the mod's smoke is on, it is the only smoke.
fn horizon_physics_fire_smoke(ty: T) -> ::core::ops::ControlFlow<()> {
    use crate::native_horizon_physics as hp;
    if ty == T::LargeSmoke && hp::replaces_vanilla_smoke() {
        ::core::ops::ControlFlow::Break(())
    } else {
        ::core::ops::ControlFlow::Continue(())
    }
}
