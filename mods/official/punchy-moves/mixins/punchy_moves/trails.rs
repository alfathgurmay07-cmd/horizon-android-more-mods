// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Elytra contrails for Punchy Moves, injected as `crate::game::punchy_moves_trails`.
// A port to Horizon of the trails of Elytra Contrails by dbrighthd
// (https://github.com/dbrighthd/elytratrails, MPL-2.0): ribbons from both
// wingtips while gliding fast, thickening over the start ramp and thinning
// over the end ramp (a quarter sine each), fading over their lifetime
// ((1 - age / lifetime)²), fading in near the camera (smoothstep from 0.5 to
// 0.7 blocks), in a colour or a striped flag. The flags' stripe colours are
// the mod's trail textures. This file stays under MPL-2.0 (see LICENSE-MPL);
// the rest of Punchy Moves is MIT.

use super::Game;
use horizon_client::mesher::Vertex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;
use std::time::Instant;

/// The trail looks a preset picks (`config_presets/*.json`).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct TrailConfig {
    pub enabled: bool,
    /// Trails on other players too.
    pub others: bool,
    /// Blocks (`maxWidth`).
    pub width: f32,
    /// Seconds (`trailLifetime`).
    pub lifetime: f32,
    /// Blocks a tick; 0 shows a trail at any speed (`trailMinSpeed`).
    pub min_speed: f32,
    /// Index into [`COLORS`].
    pub color: usize,
    /// Full bright (`glowingTrails`).
    pub glowing: bool,
    /// Fade over the lifetime (`lifeTimeFade`).
    pub fade: bool,
    /// Blocks to full width and from it (`startRampDistance`, `endRampDistance`).
    pub start_ramp: f32,
    pub end_ramp: f32,
    /// Fade in from the wingtip over this many blocks; 0: off (`fadeStart`, `fadeStartDistance`).
    pub fade_in: f32,
    /// Grow from 1x to this many times as wide over the lifetime; 1: off
    /// (`increaseWidthOverTime`, `endingWidthMultiplier`).
    pub grow: f32,
    /// Ragged width (`enableRandomWidth`, `randomWidthVariation`); 0: off.
    pub ragged: f32,
    /// Opacity, 0 to 1.
    pub alpha: f32,
}
impl Default for TrailConfig {
    fn default() -> Self {
        PRESETS[0].1
    }
}

#[allow(clippy::too_many_arguments)]
const fn preset(width: f32, lifetime: f32, color: usize, glowing: bool, fade: bool, start_ramp: f32, end_ramp: f32, fade_in: f32, grow: f32, ragged: f32, alpha: f32) -> TrailConfig {
    TrailConfig {
        enabled: true,
        others: true,
        width,
        lifetime,
        min_speed: 0.8,
        color,
        glowing,
        fade,
        start_ramp,
        end_ramp,
        fade_in,
        grow,
        ragged,
        alpha,
    }
}

/// The mod's presets, by name.
pub(crate) const PRESETS: [(&str, TrailConfig); 7] = [
    ("Default", preset(0.05, 2.5, 0, false, true, 4.0, 10.0, 0.0, 1.0, 0.0, 1.0)),
    ("Subtle", preset(0.075, 2.5, 0, false, true, 4.0, 10.0, 0.0, 1.0, 0.0, 0.78)),
    ("Jet engine", preset(0.07, 2.5, 0, false, true, 0.0, 0.0, 5.0, 20.0, 1.0, 0.6)),
    ("Rainbow", preset(0.25, 2.5, 13, true, false, 4.0, 10.0, 0.0, 1.0, 0.0, 1.0)),
    ("Green glow", preset(0.1, 2.5, 4, true, true, 4.0, 10.0, 0.0, 1.0, 0.0, 1.0)),
    ("Hackerman", preset(0.5, 2.5, 5, true, false, 4.0, 10.0, 0.0, 1.0, 0.0, 1.0)),
    ("Action lines", preset(0.07, 0.6, 0, false, false, 0.0, 10.0, 5.0, 1.0, 0.0, 0.6)),
];

/// Trail colours: plain ones, then the flags (their stripes across the trail, ARGB).
pub(crate) const COLORS: [(&str, &[u32]); 31] = [
    ("White", &[0xFFFF_FFFF]),
    ("Red", &[0xFFFF_4040]),
    ("Orange", &[0xFFFF_9A2E]),
    ("Yellow", &[0xFFFF_E640]),
    ("Green", &[0xFF29_E078]),
    ("Lime", &[0xFF00_FF00]),
    ("Cyan", &[0xFF40_E0FF]),
    ("Blue", &[0xFF40_70FF]),
    ("Purple", &[0xFFA0_50FF]),
    ("Pink", &[0xFFFF_70C0]),
    ("Black", &[0xFF10_1010]),
    ("Gold", &[0xFFFF_C830]),
    ("Sky", &[0xFFA0_D8FF]),
    ("Rainbow", &[0xFFFF_0018, 0xFFFF_A52C, 0xFFFF_FF41, 0xFF00_8018, 0xFF00_00F9, 0xFF86_007D]),
    ("Transgender", &[0xFF55_CDFC, 0xFFF7_A8B8, 0xFFFF_FFFF, 0xFFF7_A8B8, 0xFF55_CDFC]),
    ("Bisexual", &[0xFFD6_0270, 0xFFD6_0270, 0xFF9B_4F96, 0xFF00_38A8, 0xFF00_38A8]),
    ("Pansexual", &[0xFFFF_1B8D, 0xFFFF_DA00, 0xFF1B_B3FF]),
    ("Lesbian", &[0xFFD6_2900, 0xFFFF_9B55, 0xFFFF_FFFF, 0xFFD4_61A6, 0xFFA5_0062]),
    ("Gay", &[0xFF07_8D70, 0xFF98_E8C1, 0xFFFF_FFFF, 0xFF7B_ADE2, 0xFF3D_1A78]),
    ("Nonbinary", &[0xFFFF_F430, 0xFFFF_FFFF, 0xFF9C_59D1, 0xFF00_0000]),
    ("Asexual", &[0xFF00_0000, 0xFFA3_A3A3, 0xFFFF_FFFF, 0xFF80_0070]),
    ("Aromantic", &[0xFF3B_A740, 0xFFA8_D47A, 0xFFFF_FFFF, 0xFFAB_ABAB, 0xFF00_0000]),
    ("Aroace", &[0xFFE3_8D00, 0xFFE7_C601, 0xFFFF_FFFF, 0xFF5F_AAD7, 0xFF1F_3554]),
    ("Agender", &[0xFF00_0000, 0xFFBA_BABA, 0xFFFF_FFFF, 0xFFBA_F484, 0xFFFF_FFFF, 0xFFBA_BABA, 0xFF00_0000]),
    ("Genderfluid", &[0xFFFE_76A2, 0xFFFF_FFFF, 0xFFBF_12D7, 0xFF00_0000, 0xFF30_3CBE]),
    ("Genderqueer", &[0xFFB8_99DD, 0xFFFF_FFFF, 0xFF6B_8E3A]),
    ("Bigender", &[0xFFC4_79A2, 0xFFEB_A3CB, 0xFFD4_C5E6, 0xFFFF_FFFF, 0xFFD4_C5E6, 0xFF98_C5E6, 0xFF6D_82D1]),
    ("Demigirl", &[0xFF7E_7E7E, 0xFFC3_C3C3, 0xFFFD_ADC8, 0xFFFF_FFFF, 0xFFFD_ADC8, 0xFFC3_C3C3, 0xFF7E_7E7E]),
    ("Demiboy", &[0xFF7E_7E7E, 0xFFC3_C3C3, 0xFF9A_D9F2, 0xFFFF_FFFF, 0xFF9A_D9F2, 0xFFC3_C3C3, 0xFF7E_7E7E]),
    ("Polysexual", &[0xFFF6_16BA, 0xFF01_D669, 0xFF15_93F6]),
    ("Omnisexual", &[0xFFF0_A2CC, 0xFFEA_66BC, 0xFF21_0746, 0xFF65_65F7, 0xFF94_A9FA]),
];

/// Samples a second (`maxSamplePerSecond`).
const SAMPLES_PER_SECOND: f64 = 60.0;
/// Where on a wing the trail comes from, across its box: out to the tip,
/// halfway down, at the back (`wingtipHorizontalPosition` 1, `wingtipVerticalPosition` 0.5,
/// `wingtipDepthPosition` 1).
const TIP: [f32; 3] = [1.0, 0.5, 1.0];
/// Full sky and block light.
const FULL_LIGHT: u32 = 240 | 240 << 8;

#[derive(Clone, Copy)]
struct Point {
    pos: glam::DVec3,
    time: f64,
    light: u32,
}

#[derive(Default)]
struct Trail {
    points: VecDeque<Point>,
    /// Still attached to its wingtip: drawn on up to it.
    attached: bool,
    head: Option<glam::DVec3>,
}

#[derive(Default)]
struct Wings {
    sides: [Vec<Trail>; 2],
    seen: f64,
}

struct State {
    start: Instant,
    wings: HashMap<u32, Wings>,
    /// Spare trails, to reuse their allocation.
    spare: Vec<Trail>,
    /// Corner scratch for one trail.
    scratch: Vec<(glam::DVec3, f32, f32, f64, u32)>,
}
static STATE: Mutex<Option<State>> = Mutex::new(None);
static CONFIG: Mutex<Option<TrailConfig>> = Mutex::new(None);

pub(crate) fn config() -> TrailConfig {
    CONFIG.lock().unwrap_or_else(|e| e.into_inner()).unwrap_or_default()
}
pub(crate) fn set_config(c: TrailConfig) {
    *CONFIG.lock().unwrap_or_else(|e| e.into_inner()) = Some(c);
}

/// `ResolvedTrailSettings`-style width ramp: a quarter sine over `ramp` blocks.
fn ramp(d: f32, ramp: f32) -> f32 {
    if ramp <= 0.0 || d > ramp {
        1.0
    } else if d <= 0.0 {
        0.0
    } else {
        (d / (ramp * std::f32::consts::FRAC_2_PI)).sin()
    }
}
/// `slowStartRamp`: (d / over)², clamped.
fn slow_ramp(d: f32, over: f32) -> f32 {
    if over <= 0.0 {
        return if d > 0.0 { 1.0 } else { 0.0 };
    }
    let k = (d / over).clamp(0.0, 1.0);
    k * k
}
/// `cameraDistanceFade`: smoothstep from 0.5 to 0.7 blocks.
fn camera_fade(d: f32) -> f32 {
    let k = ((d - 0.5) / 0.2).clamp(0.0, 1.0);
    k * k * (3.0 - 2.0 * k)
}

/// A part's transform as `render_model` builds it (pivot in pixels, ZYX angles, scale).
fn part_matrix(p: [f32; 9]) -> glam::Mat4 {
    glam::Mat4::from_translation(glam::Vec3::new(p[0], p[1], p[2]) / 16.0)
        * glam::Mat4::from_quat(glam::Quat::from_euler(glam::EulerRot::ZYX, p[5], p[4], p[3]))
        * glam::Mat4::from_scale(glam::Vec3::new(p[6], p[7], p[8]))
}

/// The two wingtips of player `m`'s elytra, camera-relative: as the elytra
/// layer draws it (`wings_geometry`), the point `TIP` across each wing's box.
fn wingtips(game: &Game<'_>, m: &horizon_client::mobs::Mob, camera: glam::DVec3, partial: f32) -> Option<[glam::Vec3; 2]> {
    let model = game.armor_stand_assets.player.get("elytra")?;
    let base = horizon_client::mobs::render::base_transform(m, partial, camera)
        * glam::Mat4::from_translation(glam::Vec3::new(0.0, 0.0, 0.125));
    let angles = m.extra.elytra.angles(partial);
    let crouching = m.extra.crouching;
    let mut tips = [None, None];
    fn walk(
        part: &horizon_client::mobs::Part,
        parent: glam::Mat4,
        angles: [f32; 3],
        crouching: bool,
        tips: &mut [Option<glam::Vec3>; 2],
    ) {
        let mut pose = part.pose;
        let side = match part.name.as_str() {
            "left_wing" => Some(0),
            "right_wing" => Some(1),
            _ => None,
        };
        if let Some(i) = side {
            let invert = if i == 0 { 1.0 } else { -1.0 };
            pose[1] = if crouching { 3.0 } else { 0.0 };
            pose[3] = angles[0];
            pose[4] = invert * angles[1];
            pose[5] = invert * angles[2];
        }
        let m = parent * part_matrix(pose);
        if let Some(i) = side {
            // The wing's box in its own pixels; "out" is away from the pivot.
            let (mut lo, mut hi) = (glam::Vec3::splat(f32::MAX), glam::Vec3::splat(f32::MIN));
            for poly in &part.polygons {
                for v in poly.vertices {
                    let p = glam::Vec3::new(v[0], v[1], v[2]);
                    lo = lo.min(p);
                    hi = hi.max(p);
                }
            }
            if lo.x <= hi.x {
                let (inner, outer) = if hi.x.abs() >= lo.x.abs() { (lo.x, hi.x) } else { (hi.x, lo.x) };
                let p = glam::Vec3::new(
                    inner + (outer - inner) * TIP[0],
                    lo.y + (hi.y - lo.y) * TIP[1],
                    lo.z + (hi.z - lo.z) * TIP[2],
                );
                tips[i] = Some(m.transform_point3(p / 16.0));
            }
        }
        for c in &part.children {
            walk(c, m, angles, crouching, tips);
        }
    }
    walk(model, base, angles, crouching, &mut tips);
    Some([tips[0]?, tips[1]?])
}

fn packed_light(game: &Game<'_>, p: glam::DVec3) -> u32 {
    let light = game.world.light(p.x.floor() as i32, p.y.floor() as i32, p.z.floor() as i32);
    (u32::from(light >> 4) * 16) | ((u32::from(light & 15) * 16) << 8)
}

/// After the text displays: sample every gliding player's wingtips and draw
/// the trails into the entity pass (`normal`, whose span is translucent).
pub(crate) fn world(game: &Game<'_>, camera: glam::DVec3, partial: f64, out: &mut Vec<Vertex>) {
    let cfg = config();
    let mut guard = STATE.lock().unwrap_or_else(|e| e.into_inner());
    if !cfg.enabled || !game.native_on(super::punchy_moves::ID) {
        if let Some(s) = guard.as_mut() {
            s.wings.clear();
        }
        return;
    }
    let s = guard.get_or_insert_with(|| State {
        start: Instant::now(),
        wings: HashMap::new(),
        spare: Vec::new(),
        scratch: Vec::new(),
    });
    let now = s.start.elapsed().as_secs_f64();
    let partial = partial as f32;
    let own = game.player_model.as_ref();
    let own_id = own.map(|m| m.id);
    let first_person = !game.own_body_in_view();
    // Sample: who glides, fast enough, and where their wingtips are.
    let mut sample = |m: &horizon_client::mobs::Mob| {
        let flying = m.extra.fall_flying && m.death_time == 0;
        let w = s.wings.entry(m.id).or_default();
        w.seen = now;
        let speed = ((m.position.x - m.previous.x).powi(2) + (m.position.y - m.previous.y).powi(2) + (m.position.z - m.previous.z).powi(2)).sqrt() as f32;
        let emit = flying && speed >= cfg.min_speed;
        let tips = if emit { wingtips(game, m, camera, partial) } else { None };
        for (side, trails) in w.sides.iter_mut().enumerate() {
            match tips {
                Some(t) => {
                    let pos = camera + t[side].as_dvec3();
                    if !trails.last().is_some_and(|t| t.attached) {
                        let mut t = s.spare.pop().unwrap_or_default();
                        t.points.clear();
                        t.attached = true;
                        trails.push(t);
                    }
                    let t = trails.last_mut().expect("just pushed");
                    let due = t.points.back().is_none_or(|p| now - p.time >= 1.0 / SAMPLES_PER_SECOND);
                    if due {
                        let light = if cfg.glowing { FULL_LIGHT } else { packed_light(game, pos) };
                        t.points.push_back(Point { pos, time: now, light });
                    }
                    t.head = Some(pos);
                }
                None => {
                    if let Some(t) = trails.last_mut() {
                        t.attached = false;
                        t.head = None;
                    }
                }
            }
        }
    };
    if let Some(m) = own {
        sample(m);
    }
    if cfg.others {
        for m in game.mobs.iter() {
            if &*m.kind.id == "minecraft:player" && Some(m.id) != own_id {
                sample(m);
            }
        }
    }
    // Expire old points, empty trails and players gone for a while.
    let life = f64::from(cfg.lifetime.max(0.05));
    for w in s.wings.values_mut() {
        for trails in &mut w.sides {
            for t in trails.iter_mut() {
                while t.points.front().is_some_and(|p| now - p.time > life) {
                    t.points.pop_front();
                }
            }
            let mut i = 0;
            while i < trails.len() {
                if trails[i].points.is_empty() && !trails[i].attached {
                    let t = trails.swap_remove(i);
                    if s.spare.len() < 32 {
                        s.spare.push(t);
                    }
                } else {
                    i += 1;
                }
            }
        }
    }
    s.wings.retain(|_, w| now - w.seen < 5.0 || w.sides.iter().any(|t| !t.is_empty()));
    // Draw.
    let white = horizon_client::gfx::lightning::white_uv(&game.gui.atlas.font.white());
    let stripes = COLORS[cfg.color.min(COLORS.len() - 1)].1;
    let scratch = &mut s.scratch;
    for (id, w) in &s.wings {
        let mine = Some(*id) == own_id;
        for trails in &w.sides {
            for t in trails {
                scratch.clear();
                for p in &t.points {
                    scratch.push((p.pos, 0.0, 0.0, p.time, p.light));
                }
                if let (Some(h), Some(last)) = (t.head, t.points.back())
                    && t.attached
                    && h.distance_squared(last.pos) > 1e-6
                {
                    scratch.push((h, 0.0, 0.0, now, last.light));
                }
                if scratch.len() < 2 {
                    continue;
                }
                // Distance along the trail from its tail.
                let mut along = 0.0f64;
                for i in 1..scratch.len() {
                    along += scratch[i].0.distance(scratch[i - 1].0);
                    scratch[i].1 = along as f32;
                }
                let total = along as f32;
                // Width and alpha at each point.
                for point in scratch.iter_mut() {
                    let (pos, from_tail, _, time, _) = *point;
                    let from_head = total - from_tail;
                    let age = ((now - time) / life).clamp(0.0, 1.0) as f32;
                    let mut width = cfg.width * ramp(from_head, cfg.start_ramp) * ramp(from_tail, cfg.end_ramp);
                    if cfg.grow > 1.0 {
                        width *= 1.0 + (cfg.grow - 1.0) * age;
                    }
                    if cfg.ragged > 0.0 {
                        // A steady wobble along the trail: the same point keeps its width.
                        let n = ((time * 7.3).sin() * 0.5 + (time * 3.1 + 1.7).sin() * 0.5) as f32;
                        width *= (1.0 + n * 0.5 * cfg.ragged).max(0.05);
                    }
                    let mut alpha = cfg.alpha;
                    if cfg.fade {
                        alpha *= (1.0 - age) * (1.0 - age);
                    }
                    if cfg.fade_in > 0.0 {
                        alpha *= slow_ramp(from_head, cfg.fade_in);
                    }
                    alpha *= camera_fade(pos.distance(camera) as f32);
                    if mine && first_person {
                        // Your own trail in first person: in a little behind you.
                        alpha *= slow_ramp(from_head, 1.5);
                    }
                    point.2 = width.max(0.0) * if alpha > 0.002 { 1.0 } else { 0.0 };
                    point.3 = f64::from(alpha.clamp(0.0, 1.0));
                }
                emit(out, scratch, camera, stripes, white);
            }
        }
    }
}

/// Ribbon quads between consecutive points, facing the camera, in stripes.
fn emit(out: &mut Vec<Vertex>, pts: &[(glam::DVec3, f32, f32, f64, u32)], camera: glam::DVec3, stripes: &[u32], white: [f32; 2]) {
    let n = pts.len();
    let side_at = |i: usize| -> glam::Vec3 {
        let a = pts[i.saturating_sub(1)].0;
        let b = pts[(i + 1).min(n - 1)].0;
        let tangent = (b - a).as_vec3();
        let view = (pts[i].0 - camera).as_vec3();
        let s = tangent.cross(view);
        if s.length_squared() > 1e-12 { s.normalize() } else { glam::Vec3::Y }
    };
    let k = stripes.len() as f32;
    let mut prev_side = side_at(0);
    for i in 0..n - 1 {
        let side = side_at(i + 1);
        let (p0, p1) = (pts[i], pts[i + 1]);
        if p0.2 <= 0.0 && p1.2 <= 0.0 {
            prev_side = side;
            continue;
        }
        let r0 = (p0.0 - camera).as_vec3();
        let r1 = (p1.0 - camera).as_vec3();
        for (j, &c) in stripes.iter().enumerate() {
            let (a, b) = (j as f32 / k - 0.5, (j + 1) as f32 / k - 0.5);
            let color = |alpha: f64| {
                horizon_client::hud::rgba(
                    (c >> 16) as u8,
                    (c >> 8) as u8,
                    c as u8,
                    (alpha.clamp(0.0, 1.0) * 255.0).round() as u8,
                )
            };
            let (c0, c1) = (color(p0.3), color(p1.3));
            let v = |pos: glam::Vec3, color: u32, light: u32| Vertex {
                pos: pos.to_array(),
                uv: white,
                layer: 0,
                color,
                light,
                normal: [0; 4],
                glint_uv: [0.0, 0.0],
            };
            out.push(v(r0 + prev_side * (p0.2 * a), c0, p0.4));
            out.push(v(r0 + prev_side * (p0.2 * b), c0, p0.4));
            out.push(v(r1 + side * (p1.2 * b), c1, p1.4));
            out.push(v(r1 + side * (p1.2 * a), c1, p1.4));
        }
        prev_side = side;
    }
}

#[cfg(test)]
pub(crate) fn test_count() -> usize {
    STATE.lock().unwrap().as_ref().map_or(0, |s| s.wings.values().map(|w| w.sides.iter().map(|t| t.iter().map(|t| t.points.len()).sum::<usize>()).sum::<usize>()).sum())
}
