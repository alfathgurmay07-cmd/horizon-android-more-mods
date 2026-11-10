// Player.attack: a ragdoll or a piece in front of the player is knocked away by the hit (the
// game's own attack goes on as ever).
fn horizon_physics_attack(game: &crate::game::Game<'_>) {
    use horizon_client::native_horizon_physics as hp;
    if !game.native_on(hp::ID) {
        return;
    }
    let (Some(p), Some(eye)) = (game.player.as_ref(), game.eye_position()) else {
        return;
    };
    let _ = hp::punch(eye, p.yaw, p.pitch);
}
