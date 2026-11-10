// BannerRenderer as cloth: the banner is drawn as ever, then its flag (not the pole or the bar)
// is bent as a cloth hanging from its top edge: it lifts and blows out with the wind, and waves
// run down it to its free end.
fn horizon_physics_banner(
    this: &BlockEntityRenderer,
    scene: &Scene<'_>,
    pos: [i32; 3],
    kind: &Kind,
    base: Mat4,
    packed: u8,
    lights: (Vec3, Vec3),
    crumbling: bool,
    world_chests: bool,
    out: &mut Vec<Vertex>,
) -> ::core::ops::ControlFlow<()> {
    use crate::native_horizon_physics_sway as sway;
    let Kind::Banner { angle, .. } = kind else {
        return ::core::ops::ControlFlow::Continue(());
    };
    if !sway::banners_on() || pos[0] == i32::MAX || !sway::banner_enter() {
        return ::core::ops::ControlFlow::Continue(());
    }
    let start = out.len();
    this.render_one(scene, pos, kind, base, packed, lights, crumbling, world_chests, &mut *out);
    sway::banner_leave();
    let m = base
        * Mat4::from_translation(Vec3::new(0.5, 0.0, 0.5))
        * Mat4::from_rotation_y((-angle).to_radians())
        * Mat4::from_scale(Vec3::new(2.0 / 3.0, -2.0 / 3.0, -2.0 / 3.0));
    let t = (scene.game_time as f64 + f64::from(scene.partial)) / 20.0;
    sway::banner_deform(&mut out[start..], m.to_cols_array_2d(), pos, t);
    ::core::ops::ControlFlow::Break(())
}
