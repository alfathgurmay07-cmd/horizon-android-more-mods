// Mesher.mesh_reusing: Horizon meshes the section as always, then the faces of the blocks that
// connect are joined to their neighbours (`ctm::process`).
fn connected_textures_mesh(
    mesher: &mut Mesher<'_>,
    region: &[[Option<&Chunk>; 3]; 3],
    sy: i32,
    out: &mut SectionMesh,
    scratch: &mut MeshScratch,
) -> ::core::ops::ControlFlow<SectionMesh> {
    use crate::native_connected_textures as ct;
    if !ct::enabled() || !ct::enter() {
        return ::core::ops::ControlFlow::Continue(());
    }
    let mut done = mesher.mesh_reusing(region, sy, ::core::mem::take(out), scratch);
    ct::leave();
    let registry = mesher.assets.registry;
    let layers = &mesher.render.sprite_layers;
    let key = [
        registry as *const _ as usize,
        mesher.render as *const _ as usize,
        layers.len(),
    ];
    let rules = ct::rules(
        key,
        registry.len(),
        &|id| registry.state(BlockStateId(id)).map(|s| s.name.clone()),
        &|name| layers.get(name).map(|&l| l as u16),
    );
    if rules.is_empty() {
        return ::core::ops::ControlFlow::Break(done);
    }
    let pad = &scratch.padded;
    let at = |x: i32, y: i32, z: i32| -> u32 {
        if (-2..18).contains(&x) && (-2..18).contains(&y) && (-2..18).contains(&z) {
            pad.blocks[pi(x, y, z)]
        } else {
            0
        }
    };
    for list in [&mut done.solid, &mut done.translucent] {
        if list.len() % 4 != 0 || !list.iter().any(|v| rules.has_tile(v.layer)) {
            continue;
        }
        let src: Vec<ct::V> = list
            .iter()
            .map(|v| ct::V { pos: v.pos, uv: v.uv, layer: v.layer, light: v.light, color: v.color })
            .collect();
        if let Some(new) = ct::process(&rules, &src, &at) {
            *list = new
                .into_iter()
                .map(|v| TerrainVertex { pos: v.pos, uv: v.uv, layer: v.layer, light: v.light, color: v.color })
                .collect();
        }
    }
    ::core::ops::ControlFlow::Break(done)
}
