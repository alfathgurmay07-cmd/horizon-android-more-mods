// A press on the title screen while the quick setup shows is the quick setup's.
fn horizon_physics_setup_click(
    _title: &mut Title,
    x: f32,
    y: f32,
    size: (f32, f32),
) -> ::core::ops::ControlFlow<(Action, bool)> {
    use crate::native_horizon_physics as hp;
    if !hp::setup_visible() {
        return ::core::ops::ControlFlow::Continue(());
    }
    let hit = hp::setup_click(size.0.ceil(), size.1.ceil(), x, y);
    ::core::ops::ControlFlow::Break((Action::None, hit))
}
