// A block the server changes. Turned to air: if an explosion did it, its pieces are blown away
// (the block is still there here, so its sprite and shape can be read). Water or lava poured into
// it: the liquid splashes out before it settles.
fn horizon_physics_aired(game: &mut crate::game::Game<'_>, pos: [i32; 3], state: i32) {
    use horizon_client::native_horizon_physics as hp;
    if state < 0 || !game.native_window() {
        return;
    }
    let new = BlockStateId(state as u32);
    let old = game.world.block(pos[0], pos[1], pos[2]);
    let facts = game.res.facts();
    if hp::liquids_on() {
        let fluid = facts.get(new).fluid;
        if fluid != horizon_client::blocks::Fluid::None && facts.get(old).fluid != fluid {
            let source = game
                .pack
                .assets
                .registry
                .state(new)
                .and_then(|s| s.properties.get("level"))
                .is_none_or(|l| l == "0");
            if source {
                hp::pour(pos, fluid == horizon_client::blocks::Fluid::Lava);
            }
        }
    }
    if !hp::explosions_on() || facts.is_air(old) || !facts.is_air(new) {
        return;
    }
    let Some((sprite, tint)) = game.particle_sprite(old, pos) else {
        return;
    };
    let Some(r) = game.gui.atlas.sprites.get(&sprite).copied() else {
        return;
    };
    let boxes = game.particle_shape(pos, old);
    hp::block_aired(pos, &boxes, [r.u0, r.v0, r.u1, r.v1], tint);
}
