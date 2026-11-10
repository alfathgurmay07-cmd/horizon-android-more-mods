// A mob that has just died is drawn once more, into the buffer, while the parts are marked; those
// vertices become a ragdoll (or loose parts) and the mob itself is no longer drawn. The nested
// call below comes back through this hook with the guard set and draws normally.
fn horizon_physics_mob(
    m: &Mob,
    alpha: f32,
    camera: glam::DVec3,
    atlas: Atlas,
    light: u32,
    data: &MobData,
    extra: &[(&Part, [f32; 4], u32, u32)],
    out: &mut Vec<Vertex>,
) -> ::core::ops::ControlFlow<()> {
    use crate::native_horizon_physics as hp;
    if m.alive() || !hp::deaths_on() {
        return ::core::ops::ControlFlow::Continue(());
    }
    if hp::RENDER.enter() {
        return ::core::ops::ControlFlow::Continue(());
    }
    if hp::first_death(m.id) {
        let start = out.len();
        hp::capture_begin();
        render_with(m, alpha, camera, atlas, light, data, extra, out);
        let marks = hp::capture_end();
        let velocity = [
            (m.position.x - m.previous.x) * 20.0,
            (m.position.y - m.previous.y) * 20.0,
            (m.position.z - m.previous.z) * 20.0,
        ];
        hp::spawn_mob(&out[start..], &marks, start, [camera.x, camera.y, camera.z], velocity);
        out.truncate(start);
    }
    hp::RENDER.leave();
    ::core::ops::ControlFlow::Break(())
}
