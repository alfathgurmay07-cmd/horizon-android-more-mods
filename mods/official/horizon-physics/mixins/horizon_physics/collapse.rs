// Game.break_block, then what lost its support falls (PhysicsMod's collapsing structures): the
// blocks that the broken one held up and that no longer reach the ground (a tree whose trunk
// was cut, a bridge whose pillar was taken) are taken out of the world as if broken (their drops
// fall, in survival) and fall as whole blocks that tip over away from the player; the leaves of a
// falling tree come down with it. Only blocks the player breaks.
fn horizon_physics_collapse(game: &mut crate::game::Game<'_>, pos: [i32; 3]) -> ::core::ops::ControlFlow<Result<()>> {
    use horizon_client::native_horizon_physics as hp;
    // A collapse that waited (Collapse Speed): the block is long broken, only the fall is left.
    let replay = hp::take_collapse_replay(pos);
    let st = hp::settings();
    if replay && (!game.native_on(hp::ID) || !st.collapse) {
        return ::core::ops::ControlFlow::Break(Ok(()));
    }
    if !replay && (!game.native_window() || !game.native_on(hp::ID) || !st.collapse || hp::COLLAPSE.enter()) {
        return ::core::ops::ControlFlow::Continue(());
    }
    let result = if replay {
        Ok(())
    } else {
        let r = game.break_block(pos);
        hp::COLLAPSE.leave();
        r
    };
    if result.is_err() || !game.world.is_air(game.world.block(pos[0], pos[1], pos[2])) {
        return ::core::ops::ControlFlow::Break(result);
    }
    if !replay && st.collapse_ticks > 0 {
        hp::queue_collapse(pos, f64::from(st.collapse_ticks) / 20.0);
        return ::core::ops::ControlFlow::Break(result);
    }
    // 0: nothing that holds anything (air, fluids, plants, torches); 1 the ground; 2 leaves; 3
    // anything else.
    let kind = |p: [i32; 3]| -> u8 {
        let state = game.world.block(p[0], p[1], p[2]);
        if game.world.is_air(state) {
            return 0;
        }
        let f = game.res.facts().get(state);
        if f.fluid != horizon_client::blocks::Fluid::None {
            return 0;
        }
        if !f.full_collision && f.outline.is_empty() {
            return 0;
        }
        hp::collapse_kind(&f.name)
    };
    let log = |p: [i32; 3]| -> bool {
        let state = game.world.block(p[0], p[1], p[2]);
        !game.world.is_air(state) && hp::is_log(&game.res.facts().get(state).name)
    };
    const SIDES: [[i32; 3]; 6] = [[1, 0, 0], [-1, 0, 0], [0, 1, 0], [0, -1, 0], [0, 0, 1], [0, 0, -1]];
    let most = st.collapse_limit.max(1) as usize;
    let step = |p: [i32; 3], d: [i32; 3]| [p[0] + d[0], p[1] + d[1], p[2] + d[2]];
    // The blocks the broken one touched, each with all it is joined to: held up when that reaches
    // the ground (or is too big to tell), else falling.
    let mut seen: std::collections::HashSet<[i32; 3]> = std::collections::HashSet::new();
    let mut falling: Vec<[i32; 3]> = Vec::new();
    for d in SIDES {
        let start = step(pos, d);
        if seen.contains(&start) || kind(start) != 3 {
            continue;
        }
        let mut group = vec![start];
        let mut queue = std::collections::VecDeque::from([start]);
        seen.insert(start);
        let mut held = false;
        while let Some(p) = queue.pop_front() {
            for d in SIDES {
                let n = step(p, d);
                if n == pos || seen.contains(&n) {
                    continue;
                }
                match kind(n) {
                    1 => held = true,
                    3 => {
                        seen.insert(n);
                        group.push(n);
                        queue.push_back(n);
                    }
                    _ => {}
                }
                if held || group.len() > most {
                    break;
                }
            }
            if held || group.len() > most {
                held = true;
                break;
            }
        }
        if !held {
            falling.extend(group);
        }
    }
    if falling.is_empty() {
        return ::core::ops::ControlFlow::Break(result);
    }
    // The leaves of a falling tree: those joined to its logs through leaves (up to five blocks
    // out), unless another tree's log is close to them.
    let fallen: std::collections::HashSet<[i32; 3]> = falling.iter().copied().collect();
    let mut leaves: Vec<[i32; 3]> = Vec::new();
    let mut queue: std::collections::VecDeque<([i32; 3], u8)> = std::collections::VecDeque::new();
    let mut near: std::collections::HashSet<[i32; 3]> = std::collections::HashSet::new();
    for p in &falling {
        if log(*p) {
            queue.push_back((*p, 0));
        }
    }
    while let Some((p, dist)) = queue.pop_front() {
        if dist >= 5 || leaves.len() > 400 {
            continue;
        }
        for d in SIDES {
            let n = step(p, d);
            if near.contains(&n) || fallen.contains(&n) || kind(n) != 2 {
                continue;
            }
            near.insert(n);
            let mut other = false;
            'look: for dx in -2..=2 {
                for dy in -2..=2 {
                    for dz in -2..=2 {
                        let q = [n[0] + dx, n[1] + dy, n[2] + dz];
                        if !fallen.contains(&q) && q != pos && log(q) {
                            other = true;
                            break 'look;
                        }
                    }
                }
            }
            if other {
                continue;
            }
            leaves.push(n);
            queue.push_back((n, dist + 1));
        }
    }
    // Pushed over away from the player.
    let push = match game.player.as_ref() {
        Some(p) => [f64::from(pos[0]) + 0.5 - p.position.x, f64::from(pos[2]) + 0.5 - p.position.z],
        None => [1.0, 0.0],
    };
    let drops = !game.creative && st.collapse_drops;
    let mut blocks: Vec<hp::Falling> = Vec::new();
    for (p, is_leaves) in falling.iter().map(|p| (*p, false)).chain(leaves.iter().map(|p| (*p, true))) {
        let state = game.world.block(p[0], p[1], p[2]);
        if let Some((sprite, tint)) = game.particle_sprite(state, p)
            && let Some(r) = game.gui.atlas.sprites.get(&sprite).copied()
        {
            blocks.push(hp::Falling { pos: p, uv: [r.u0, r.v0, r.u1, r.v1], tint, leaves: is_leaves });
        }
        hp::silence(p);
        game.with_level(|b, level, ticks| b.destroy_block(level, ticks, BlockPos::new(p[0], p[1], p[2]), drops));
    }
    hp::collapse(&blocks, pos, push);
    ::core::ops::ControlFlow::Break(result)
}
