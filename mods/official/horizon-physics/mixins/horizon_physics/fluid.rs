// Mesher.mesh_reusing, then two things in the section it made:
// - each still water surface corner is raised by a mark far too small to see (ten-thousandths of
//   a block): how open the water is there, from its depth and its shore (`water_code`). The
//   terrain shader reads it back: the sea swells, rivers lap, the shore and puddles stay still.
// - each block of snow someone walked through gets, instead of its flat top, a fine grid pressed
//   down where they went (`snow_top`).
fn horizon_physics_fluid(
    mesher: &mut Mesher<'_>,
    region: &[[Option<&Chunk>; 3]; 3],
    sy: i32,
    out: &mut SectionMesh,
    scratch: &mut MeshScratch,
) -> ::core::ops::ControlFlow<SectionMesh> {
    use crate::native_horizon_physics_sway as sway;
    let dents = match region[1][1] {
        Some(c) => sway::snow_in_section(c.x, sy, c.z),
        None => Vec::new(),
    };
    let doors = match region[1][1] {
        Some(c) => sway::doors_in_section(c.x, sy, c.z),
        None => Vec::new(),
    };
    let marks = sway::marks_water();
    if (!marks && dents.is_empty() && doors.is_empty()) || !sway::fluid_enter() {
        return ::core::ops::ControlFlow::Continue(());
    }
    let mut done = mesher.mesh_reusing(region, sy, ::core::mem::take(out), scratch);
    sway::fluid_leave();
    if marks {
        let info = &mesher.tables.info;
        let pad = &scratch.padded;
        let water = |x: i32, y: i32, z: i32| -> bool {
            if !(-2..18).contains(&x) || !(-2..18).contains(&y) || !(-2..18).contains(&z) {
                return false;
            }
            matches!(info[pad.blocks[pi(x, y, z)] as usize].fluid.kind(), FluidKind::Water | FluidKind::FlowingWater)
        };
        for v in &mut done.translucent {
            let y = (v.pos[1] - sway::STILL_TOP).round() as i32;
            if (v.pos[1] - y as f32 - sway::STILL_TOP).abs() > 4e-5 {
                continue;
            }
            let (cx, cz) = (v.pos[0].round() as i32, v.pos[2].round() as i32);
            if !(water(cx, y, cz) || water(cx - 1, y, cz) || water(cx, y, cz - 1) || water(cx - 1, y, cz - 1)) {
                continue;
            }
            let code = sway::water_code(cx, y, cz, water, -2);
            v.pos[1] += code as f32 * sway::MARK_STEP;
        }
    }
    // A swinging door is drawn by the world hook: its own quads (door sprites, inside its block)
    // are left out until it settles.
    if !doors.is_empty() && done.solid.len() % 4 == 0 {
        let door_layers: std::collections::HashSet<u32> = mesher
            .render
            .sprite_layers
            .iter()
            .filter(|(n, _)| n.contains("_door_") && !n.contains("trapdoor"))
            .map(|(_, l)| *l)
            .collect();
        let inside = |q: &[TerrainVertex], c: [i32; 3]| {
            q.iter().all(|v| {
                (0..3).all(|k| v.pos[k] >= c[k] as f32 - 1e-3 && v.pos[k] <= c[k] as f32 + 1.0 + 1e-3)
            })
        };
        let mut solid: Vec<TerrainVertex> = Vec::with_capacity(done.solid.len());
        for q in done.solid.chunks_exact(4) {
            let door = door_layers.contains(&u32::from(q[0].layer & 0x7fff)) && doors.iter().any(|c| inside(q, *c));
            if !door {
                solid.extend_from_slice(q);
            }
        }
        done.solid = solid;
    }
    let snow = ["minecraft:block/snow", "block/snow", "snow"]
        .iter()
        .find_map(|n| mesher.render.sprite_layers.get(*n).copied());
    if let Some(snow) = snow
        && !dents.is_empty()
        && done.solid.len() % 4 == 0
    {
        let mut solid: Vec<TerrainVertex> = Vec::with_capacity(done.solid.len() + dents.len() * 256);
        for q in done.solid.chunks_exact(4) {
            let y = q[0].pos[1];
            let flat = q.iter().all(|v| u32::from(v.layer & 0x7fff) == snow && (v.pos[1] - y).abs() < 1e-4);
            // Facing up: counter-clockwise seen from above.
            let (a, b) = (
                [q[1].pos[0] - q[0].pos[0], q[1].pos[2] - q[0].pos[2]],
                [q[2].pos[0] - q[0].pos[0], q[2].pos[2] - q[0].pos[2]],
            );
            let up = a[1] * b[0] - a[0] * b[1] > 0.0;
            let mx = q.iter().map(|v| v.pos[0]).fold(f32::INFINITY, f32::min);
            let mz = q.iter().map(|v| v.pos[2]).fold(f32::INFINITY, f32::min);
            let whole = (mx - mx.round()).abs() < 1e-4
                && (mz - mz.round()).abs() < 1e-4
                && q.iter().all(|v| {
                    let (ax, az) = (v.pos[0] - mx, v.pos[2] - mz);
                    ((ax).abs() < 1e-4 || (ax - 1.0).abs() < 1e-4) && ((az).abs() < 1e-4 || (az - 1.0).abs() < 1e-4)
                });
            let cell = [mx.round() as i32, (y - 1e-3).floor() as i32, mz.round() as i32];
            let dent = if flat && up && whole {
                dents.iter().find(|d| d.0 == cell && ((y - cell[1] as f32) - d.1.top).abs() < 2e-3)
            } else {
                None
            };
            let Some(dent) = dent else {
                solid.extend_from_slice(q);
                continue;
            };
            let corners = [0, 1, 2, 3].map(|k| sway::SnowCorner {
                at: [(q[k].pos[0] - mx).round(), (q[k].pos[2] - mz).round()],
                uv: [f32::from(q[k].uv[0]), f32::from(q[k].uv[1])],
                light: q[k].light,
                color: q[k].color,
            });
            for (pos, uv, light, color) in sway::snow_top([mx, y, mz], y, &dent.1, &corners) {
                solid.push(TerrainVertex {
                    pos,
                    uv: [uv[0].round().clamp(0.0, 65535.0) as u16, uv[1].round().clamp(0.0, 65535.0) as u16],
                    layer: q[0].layer,
                    light,
                    color,
                });
            }
        }
        done.solid = solid;
    }
    ::core::ops::ControlFlow::Break(done)
}
