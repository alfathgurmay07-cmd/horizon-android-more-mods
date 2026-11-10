// The player's model in third person, in a boat on the waves: moved by the same swell as the
// boat and the camera (the camera hook measured it), so the player sits in the boat.
fn horizon_physics_player_model(
    game: &crate::game::Game<'_>,
    camera: glam::DVec3,
    alpha: f64,
    out: &mut Vec<horizon_client::mesher::Vertex>,
) -> ::core::ops::ControlFlow<()> {
    use horizon_client::native_horizon_physics as hp;
    let Some(lift) = hp::rider_lift() else {
        return ::core::ops::ControlFlow::Continue(());
    };
    if hp::PLAYER_MODEL.enter() {
        return ::core::ops::ControlFlow::Continue(());
    }
    let start = out.len();
    game.player_model_geometry(camera, alpha, out);
    hp::PLAYER_MODEL.leave();
    for v in &mut out[start..] {
        v.pos[0] += lift[0] as f32;
        v.pos[1] += lift[1] as f32;
        v.pos[2] += lift[2] as f32;
    }
    ::core::ops::ControlFlow::Break(())
}
