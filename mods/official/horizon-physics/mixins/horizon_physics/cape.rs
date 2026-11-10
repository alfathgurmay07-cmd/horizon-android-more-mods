// CapeLayer as cloth: the cape is posed hanging straight (no flap or lean, the cloth does that
// itself), drawn into a buffer of our own, and that shape is what the cloth hangs from.
fn horizon_physics_cape(
    m: &Mob,
    alpha: f32,
    base: glam::Mat4,
    uv: [f32; 4],
    light: u32,
    data: &MobData,
    out: &mut Vec<Vertex>,
) -> ::core::ops::ControlFlow<()> {
    use crate::native_horizon_physics as hp;
    if !hp::capes_on() {
        return ::core::ops::ControlFlow::Continue(());
    }
    // Cloth Settings' Hide Cloth when Armor equipped.
    if hp::hide_cape_with_armor() && m.item_by_slot(horizon_world::player_inventory::EquipmentSlot::Chest).is_some() {
        return ::core::ops::ControlFlow::Break(());
    }
    let Some(model) = m.kind.layer("player#cape") else {
        return ::core::ops::ControlFlow::Continue(());
    };
    let s = anim::state(m, alpha);
    let mut poses = Poses::new(model);
    anim::setup(m, &s, &mut poses, &data.animations);
    let pose = poses.get("cape");
    let by = glam::Quat::from_rotation_y(-PI)
        * glam::Quat::from_rotation_x(6.0_f32.to_radians())
        * glam::Quat::from_rotation_y(180.0_f32.to_radians());
    let now = glam::Quat::from_euler(glam::EulerRot::ZYX, pose[5], pose[4], pose[3]);
    let (z, y, x) = (now * by).to_euler(glam::EulerRot::ZYX);
    (pose[3], pose[4], pose[5]) = (x, y, z);
    let mut rest: Vec<Vertex> = Vec::new();
    draw(model, base, &poses, uv, 0xFFFF_FFFF, light, 0, &mut rest);
    let pivot = base.transform_point3(glam::Vec3::ZERO);
    if hp::cape(m.id, &rest, [pivot.x, pivot.y, pivot.z], out) {
        ::core::ops::ControlFlow::Break(())
    } else {
        ::core::ops::ControlFlow::Continue(())
    }
}
