// Horizon Physics, in the renderer: water that waves and plants, leaves and vines that sway.
//
// Horizon's terrain is meshed once per chunk, so nothing on the CPU can move it every frame. A
// shader pack's terrain pipelines (Horizon's own packs, `gfx/native_material.rs`) get their
// vertex work from a short piece of WGSL around the pack's `material`; the bundle's hook adds a
// displacement there. Which vertex is what comes from its sprite layer: the game side hands this
// module the block atlas's sprite names (`set`), and the WGSL gets a table of those layers.
// Without a Horizon shader pack nothing here runs.

use std::fmt::Write as _;
use std::sync::Mutex;

/// What a sprite does: 0 nothing.
pub const LEAVES: u8 = 1;
/// A plant standing on the ground, its foot still (grass, flowers, crops, saplings).
pub const PLANT: u8 = 2;
/// The top half of a tall plant (its foot is the bottom half's top).
pub const PLANT_TOP: u8 = 3;
pub const VINES: u8 = 4;
pub const WATER: u8 = 5;
pub const SEAGRASS: u8 = 6;
pub const SEAGRASS_TOP: u8 = 7;
/// Stacked underwater plants (kelp): the whole column sways together.
pub const KELP: u8 = 8;
pub const LAVA: u8 = 9;

const FLOWERS: &[&str] = &[
    "dandelion", "poppy", "blue_orchid", "allium", "azure_bluet", "red_tulip", "orange_tulip",
    "white_tulip", "pink_tulip", "oxeye_daisy", "cornflower", "lily_of_the_valley", "wither_rose",
    "torchflower", "closed_eyeblossom", "open_eyeblossom", "short_grass", "fern", "dead_bush",
    "bush", "short_dry_grass", "tall_dry_grass", "firefly_bush",
    "sweet_berry_bush_stage0", "sweet_berry_bush_stage1", "sweet_berry_bush_stage2",
    "sweet_berry_bush_stage3", "nether_sprouts", "warped_roots", "crimson_roots",
];
const DOUBLES: &[&str] = &["tall_grass", "large_fern", "lilac", "rose_bush", "peony", "sunflower"];
const HANGING: &[&str] = &[
    "vine", "weeping_vines", "weeping_vines_plant", "twisting_vines", "twisting_vines_plant",
    "cave_vines", "cave_vines_lit", "cave_vines_plant", "cave_vines_plant_lit", "hanging_roots",
    "pale_hanging_moss", "pale_hanging_moss_tip", "spore_blossom",
];

/// The kind of the block atlas sprite `name` (`minecraft:block/oak_leaves`), 0 for none.
pub fn kind(name: &str) -> u8 {
    let Some(n) = name.strip_prefix("minecraft:block/").or_else(|| name.strip_prefix("block/")) else {
        return 0;
    };
    if n.ends_with("_leaves") {
        return LEAVES;
    }
    if FLOWERS.contains(&n) || n.ends_with("_sapling") || n.ends_with("_propagule") {
        return PLANT;
    }
    for crop in ["wheat_stage", "carrots_stage", "potatoes_stage", "beetroots_stage"] {
        if n.starts_with(crop) {
            return PLANT;
        }
    }
    for d in DOUBLES {
        if let Some(rest) = n.strip_prefix(d) {
            match rest {
                "_bottom" => return PLANT,
                "_top" | "_front" => return PLANT_TOP,
                _ => {}
            }
        }
    }
    if HANGING.contains(&n) {
        return VINES;
    }
    match n {
        "water_still" | "water_flow" => WATER,
        "lava_still" | "lava_flow" => LAVA,
        "seagrass" | "tall_seagrass_bottom" => SEAGRASS,
        "tall_seagrass_top" => SEAGRASS_TOP,
        "kelp" | "kelp_plant" => KELP,
        _ => 0,
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Sway {
    /// Which resources the table was made from (the game's render assets).
    pub key: usize,
    /// (layer, kind), sorted by layer.
    pub layers: Vec<(u32, u8)>,
    pub waves: bool,
    pub plants: bool,
    /// The wind's strength: 0 calm .. 3 a storm.
    pub wind: f32,
    /// What else sways: leaves, vines, plants under water.
    pub leaves: bool,
    pub vines: bool,
    pub water_plants: bool,
    /// How high the waves are and how far things sway (1 as the mod comes), and the froth on
    /// the crests.
    pub wave_height: f32,
    pub sway: f32,
    pub foam: bool,
    /// Plants bend away from the player walking through them, and how fast the waves roll.
    pub push: bool,
    pub wave_speed: f32,
}

/// The Ocean and Dynamic Blocks pages' settings, as the shader takes them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SwayOptions {
    pub leaves: bool,
    pub vines: bool,
    pub water_plants: bool,
    pub wave_height: f32,
    pub sway: f32,
    pub foam: bool,
    pub push: bool,
    pub wave_speed: f32,
}

impl Default for SwayOptions {
    fn default() -> Self {
        SwayOptions { leaves: true, vines: true, water_plants: true, wave_height: 1.0, sway: 1.0, foam: true, push: true, wave_speed: 1.0 }
    }
}

static OPTIONS: Mutex<Option<SwayOptions>> = Mutex::new(None);

/// The settings `set` builds with from now on.
pub fn set_options(o: SwayOptions) {
    *OPTIONS.lock().unwrap_or_else(|e| e.into_inner()) = Some(o);
}

fn options() -> SwayOptions {
    OPTIONS.lock().ok().and_then(|g| *g).unwrap_or_default()
}

static SWAY: Mutex<Option<Sway>> = Mutex::new(None);

/// The game side's state: the resources `key` with their sprite layers (only read when `key`
/// changed), and the settings. Returns true when the shader must be built again.
pub fn set<'a>(key: usize, sprites: impl FnOnce() -> Vec<(&'a str, u32)>, waves: bool, plants: bool, wind: f32) -> bool {
    let mut g = SWAY.lock().unwrap_or_else(|e| e.into_inner());
    let layers = match g.as_ref() {
        Some(s) if s.key == key => s.layers.clone(),
        _ => {
            let mut l: Vec<(u32, u8)> = sprites().into_iter().filter_map(|(n, l)| match kind(n) {
                0 => None,
                k => Some((l, k)),
            }).collect();
            l.sort_unstable();
            l.dedup_by_key(|x| x.0);
            l
        }
    };
    MARKS.store(waves, std::sync::atomic::Ordering::Relaxed);
    let o = options();
    let new = Sway {
        key,
        layers,
        waves,
        plants,
        wind,
        leaves: o.leaves,
        vines: o.vines,
        water_plants: o.water_plants,
        wave_height: o.wave_height,
        sway: o.sway,
        foam: o.foam,
        push: o.push,
        wave_speed: o.wave_speed,
    };
    let changed = g.as_ref() != Some(&new);
    *g = Some(new);
    changed
}

pub fn current() -> Option<Sway> {
    SWAY.lock().ok().and_then(|g| g.clone())
}

/// What the built-in pack's material holds, so the patch knows the water shading is ours to do
/// (another pack's water is that pack's business: it only gets the waves).
pub const BUILTIN_MARK: &str = "HORIZON_PHYSICS_BUILTIN";

/// The built-in pack (`horizon-shaders.toml`, `material.wgsl`): vanilla's look, so the mod's
/// waves and sway run without a shader pack picked.
pub const BUILTIN_MANIFEST: &str = "schema = 1
id = \"horizon_physics_vanilla\"
name = \"Horizon Physics (vanilla look)\"
version = \"1.0.0\"
api = \"0.1\"
description = \"Vanilla's look, for Horizon Physics' waves, water and swaying plants.\"

[material]
file = \"material.wgsl\"
";

pub const BUILTIN_MATERIAL: &str = "// HORIZON_PHYSICS_BUILTIN: vanilla's material, albedo * tint * lightmap.
fn material(input: MaterialInput) -> Surface {
    return Surface(input.albedo * input.tint * vec4<f32>(input.lightmap, 1.0));
}
";

/// The WGSL for `s`: `horizon_physics_sway(pos, uv, layer)`, the displacement of a terrain
/// vertex (camera-relative `pos`, its UV inside its sprite), and `horizon_physics_surface`, the
/// water's look (only when `shade`: the built-in pack), both reading the pack's `frame`.
pub fn wgsl(s: &Sway) -> Option<String> {
    wgsl_with(s, false)
}

pub fn wgsl_with(s: &Sway, shade: bool) -> Option<String> {
    let wanted = |k: u8| match k {
        WATER | LAVA => s.waves,
        LEAVES => s.leaves,
        VINES => s.vines,
        SEAGRASS | SEAGRASS_TOP | KELP => s.water_plants,
        _ => s.plants,
    };
    let mut by_kind: [Vec<u32>; 10] = Default::default();
    for &(l, k) in &s.layers {
        if wanted(k) && (k as usize) < by_kind.len() {
            by_kind[k as usize].push(l);
        }
    }
    if by_kind.iter().all(|v| v.is_empty()) {
        return None;
    }
    let mut w = String::new();
    w.push_str("fn horizon_physics_kind(layer: u32) -> u32 {\n    switch (layer & 0x7fffu) {\n");
    for (k, layers) in by_kind.iter().enumerate() {
        if layers.is_empty() {
            continue;
        }
        let cases: Vec<String> = layers.iter().map(|l| format!("{l}u")).collect();
        let _ = writeln!(w, "        case {}: {{ return {k}u; }}", cases.join(", "));
    }
    w.push_str("        default: { return 0u; }\n    }\n}\n");
    let wind = s.wind.max(0.0);
    let _ = write!(w, "const HORIZON_PHYSICS_WIND: f32 = {wind:.3};\nconst HORIZON_PHYSICS_SHADE: bool = {shade};\n");
    let _ = write!(
        w,
        "const HORIZON_PHYSICS_WAVE_HEIGHT: f32 = {:.3};\nconst HORIZON_PHYSICS_SWAY_STRENGTH: f32 = {:.3};\nconst HORIZON_PHYSICS_FOAM: f32 = {:.1};\nconst HORIZON_PHYSICS_PUSH: f32 = {:.1};\nconst HORIZON_PHYSICS_WAVE_SPEED: f32 = {:.3};\n",
        s.wave_height.max(0.0),
        s.sway.max(0.0),
        if s.foam { 1.0 } else { 0.0 },
        if s.push { 1.0 } else { 0.0 },
        s.wave_speed.max(0.05)
    );
    w.push_str(&WGSL_BODY.replace("//SWELL//", &swell_wgsl()));
    Some(w)
}

/// The open sea's swell: (direction x, direction z, wavelength, amplitude, phase, pace). Eight
/// waves from around the wind's direction, none a whole multiple of another, each rising and
/// falling along its crests (`wave_group`): a sea that never repeats. None shorter than 4 blocks,
/// which the mesh (a corner a block) could not show; the shorter ones are in the light only.
pub const OCEAN: [[f64; 6]; 8] = [
    [0.8290, 0.5592, 34.0, 0.4370, 0.0, 0.42],
    [0.4695, 0.8829, 23.7, 0.2875, 1.7, 0.43],
    [0.9998, -0.0175, 17.3, 0.2070, 3.4, 0.45],
    [-0.1045, 0.9945, 12.9, 0.1380, 5.1, 0.47],
    [0.9397, 0.3420, 9.7, 0.0978, 6.8, 0.5],
    [0.1564, 0.9877, 7.3, 0.0633, 8.3, 0.52],
    [0.9135, -0.4067, 5.4, 0.0403, 9.9, 0.55],
    [0.7314, 0.6820, 4.3, 0.0253, 11.2, 0.58],
];
/// A lake's or river's lapping.
pub const LAKE: [[f64; 6]; 5] = [
    [0.8290, 0.5592, 14.0, 0.1265, 0.0, 0.55],
    [0.2756, 0.9613, 9.6, 0.0920, 1.7, 0.55],
    [0.9976, 0.0698, 6.9, 0.0633, 3.4, 0.58],
    [-0.3256, 0.9455, 5.1, 0.0403, 5.1, 0.6],
    [0.8988, -0.4384, 4.2, 0.0253, 6.6, 0.62],
];
/// How much of the swell moves the sea's surface sideways (Gerstner's steepness).
pub const CHOP: f64 = 0.28;

fn swell_wgsl() -> String {
    let calls = |name: &str, table: &[[f64; 6]], what: &str| {
        let mut f = format!("fn {name}(p: vec2<f32>, t: f32) -> {what} {{\n    var r = {what}(0.0);\n");
        for w in table {
            let call = if what == "vec3<f32>" { "horizon_physics_wave" } else { "horizon_physics_chop_wave" };
            let _ = writeln!(f, "    r += {call}(vec4({:?}, {:?}, {:?}, {:?}), p, t, {:?}, {:?});", w[0] as f32, w[1] as f32, w[2] as f32, w[3] as f32, w[4] as f32, w[5] as f32);
        }
        f.push_str("    return r;\n}\n");
        f
    };
    let mut out = String::new();
    out.push_str(&calls("horizon_physics_ocean", &OCEAN, "vec3<f32>"));
    out.push_str(&calls("horizon_physics_lake", &LAKE, "vec3<f32>"));
    out.push_str("// Gerstner's sideways motion of the sea's surface: the water gathers under the crests, which\n// turn sharp, and spreads in the troughs.\nfn horizon_physics_chop_wave(w: vec4<f32>, p: vec2<f32>, t: f32, seed: f32, pace: f32) -> vec2<f32> {\n    let k = 6.2831853 / w.z;\n    let ph = k * dot(w.xy, p) - sqrt(9.8 * k) * pace * t + seed;\n    return -w.xy * (w.w * cos(ph) * horizon_physics_group(w, p, t, seed, pace));\n}\n");
    out.push_str(&calls("horizon_physics_chop_sum", &OCEAN, "vec2<f32>"));
    let _ = writeln!(out, "fn horizon_physics_chop(p: vec2<f32>, t: f32, open: f32) -> vec2<f32> {{\n    return horizon_physics_chop_sum(p, t) * ({:?} * horizon_physics_storm() * HORIZON_PHYSICS_WAVE_HEIGHT * horizon_physics_sea(open));\n}}", CHOP as f32);
    out
}

fn smoothstep(a: f64, b: f64, x: f64) -> f64 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The water's clock the shader uses, from the world's game time (ticks) and the partial tick.
pub fn water_time(game_time: i64, partial: f64) -> f64 {
    let f = ((game_time % 24000) as f32 + partial as f32) / 24000.0;
    let speed = current().map_or(1.0, |s| s.wave_speed.max(0.05));
    f64::from(f * 1200.0 * speed)
}

/// The water surface's movement at world point (x, z), time `t` (`water_time`), openness `open`
/// (0..1) and wind setting `wind`, as the shader moves it: (up, sideways x, sideways z, dh/dx,
/// dh/dz), before the distance fade.
pub fn water_motion(x: f64, z: f64, t: f64, open: f64, wind: f64) -> [f64; 5] {
    water_motion_with(x, z, t, open, wind, 1.0)
}

/// `water_motion` with the waves `height` times as high (the Ocean page's Wave Height).
pub fn water_motion_with(x: f64, z: f64, t: f64, open: f64, wind: f64, height: f64) -> [f64; 5] {
    let storm = (0.42 + wind * 0.3).clamp(0.15, 1.6) * height;
    let sea = smoothstep(0.5, 0.9, open);
    let lake = (open * 3.0).clamp(0.0, 1.0) * (1.0 - sea);
    let mut r = [0.0; 5];
    for (table, k) in [(&OCEAN[..], sea), (&LAKE[..], lake)] {
        if k <= 0.0 {
            continue;
        }
        for w in table {
            let kk = std::f64::consts::TAU / w[2];
            let ph = kk * (w[0] * x + w[1] * z) - (9.8 * kk).sqrt() * w[5] * t + w[4];
            let g = wave_group(w, x, z, t);
            let c = w[3] * kk * ph.cos() * g;
            r[0] += w[3] * ph.sin() * g * k * storm;
            r[3] += c * w[0] * k * storm;
            r[4] += c * w[1] * k * storm;
        }
    }
    for w in &OCEAN {
        let kk = std::f64::consts::TAU / w[2];
        let ph = kk * (w[0] * x + w[1] * z) - (9.8 * kk).sqrt() * w[5] * t + w[4];
        let s = -w[3] * ph.cos() * wave_group(w, x, z, t) * CHOP * storm * sea;
        r[1] += w[0] * s;
        r[2] += w[1] * s;
    }
    r[0] -= 0.04;
    r[0] = r[0].max(-water_floor(open));
    r
}

/// How much of wave `w` there is at (x, z), time `t`: waves come in groups, so each rises and
/// falls along its crests and slowly moves on (`horizon_physics_group`).
pub fn wave_group(w: &[f64; 6], x: f64, z: f64, t: f64) -> f64 {
    let kk = std::f64::consts::TAU / w[2];
    0.8 + 0.45 * (0.13 * kk * (-w[1] * x + w[0] * z) + w[4] * 2.7 + 0.07 * t * w[5]).sin()
}

/// How far a trough may go down where the water is `open` (`horizon_physics_floor`).
pub fn water_floor(open: f64) -> f64 {
    match (open * 7.0).round() as i32 {
        ..=1 => 0.35,
        2 => 1.1,
        3 => 1.9,
        c => 2.8 + f64::from(c - 4),
    }
}

/// The step of the mesher's mark, and the height vanilla gives a still surface's corners.
pub const MARK_STEP: f32 = 0.0001;
pub const STILL_TOP: f32 = 0.888_888_9 - 0.001;

/// How far the wave under a floating thing (a bobber) lifts and moves it: the lift the boats get,
/// from the water at `pos` (None where there is no open water under it), at time `t`
/// (`water_time`), the distance fade measured from `camera`. (sideways x, up, sideways z.)
pub fn float_lift(pos: [f64; 3], camera: [f64; 3], t: f64, wind: f64, height: f64, water: &dyn Fn(i32, i32, i32) -> bool) -> Option<[f64; 3]> {
    let (bx, bz) = (pos[0].floor() as i32, pos[2].floor() as i32);
    let base = pos[1].floor() as i32;
    let y = [base, base - 1, base + 1].into_iter().find(|y| water(bx, *y, bz) && !water(bx, *y + 1, bz))?;
    let (fx, fz) = (pos[0] - pos[0].floor(), pos[2] - pos[2].floor());
    let code = |cx: i32, cz: i32| f64::from(water_code(cx, y, cz, water, y - 16)) / 7.0;
    let open = code(bx, bz) * (1.0 - fx) * (1.0 - fz)
        + code(bx + 1, bz) * fx * (1.0 - fz)
        + code(bx, bz + 1) * (1.0 - fx) * fz
        + code(bx + 1, bz + 1) * fx * fz;
    if open <= 0.0 {
        return None;
    }
    let rel = [pos[0] - camera[0], pos[1] - camera[1], pos[2] - camera[2]];
    let reach = water_reach((rel[0] * rel[0] + rel[1] * rel[1] + rel[2] * rel[2]).sqrt());
    let m = water_motion_with(pos[0], pos[2], t, open, wind, height);
    Some([m[1] * reach, (m[0] + 0.04) * reach, m[2] * reach])
}

/// How open the water is at a surface corner: the corner at section-relative (`cx`, `cz`) of
/// surface cells at height `y` (`water` says which cells hold water, the section's border
/// included). 0 at the shore (a corner touching anything but surface water), then by depth: 1
/// for a puddle up to 7 for deep water, at most 3 within two blocks of a shore.
pub fn water_code(cx: i32, y: i32, cz: i32, water: impl Fn(i32, i32, i32) -> bool, bottom: i32) -> u32 {
    let surface = |x: i32, z: i32| water(x, y, z) && !water(x, y + 1, z);
    let mut depth = u32::MAX;
    for (dx, dz) in [(-1, -1), (-1, 0), (0, -1), (0, 0)] {
        let (x, z) = (cx + dx, cz + dz);
        if !surface(x, z) {
            return 0;
        }
        let mut d = 0u32;
        while y - d as i32 >= bottom && water(x, y - d as i32, z) {
            d += 1;
        }
        depth = depth.min(d);
    }
    let mut code = match depth {
        0..=1 => 1,
        2 => 2,
        3 => 3,
        4 | 5 => 4,
        6..=8 => 5,
        9..=12 => 6,
        _ => 7,
    };
    for dx in -2..=1 {
        for dz in -2..=1 {
            if (-1..=0).contains(&dx) && (-1..=0).contains(&dz) {
                continue;
            }
            if !water(cx + dx, y, cz + dz) {
                code = code.min(3);
            }
        }
    }
    code
}

/// How far out the shader moves the water (`horizon_physics_reach`), at `dist` blocks.
pub fn water_reach(dist: f64) -> f64 {
    1.0 - smoothstep(72.0, 104.0, dist)
}

/// The wind setting the shader was built with.
pub fn water_wind() -> f64 {
    current().map_or(1.0, |s| f64::from(s.wind))
}

/// How high the shader's waves are (Wave Height).
pub fn water_height() -> f64 {
    current().map_or(1.0, |s| f64::from(s.wave_height))
}

/// Whether the waves are on in the shader.
pub fn waves_on() -> bool {
    current().is_some_and(|s| s.waves)
}

/// Whether the mesher marks water corners (on unless the waves are off; the marks are far too
/// small to see, so marking a surface that will not wave costs nothing).
static MARKS: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);

pub fn marks_water() -> bool {
    MARKS.load(std::sync::atomic::Ordering::Relaxed)
}

thread_local! {
    static IN_FLUID: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// The fluid hook's guard against itself (it calls the function it hooks): true once.
pub fn fluid_enter() -> bool {
    IN_FLUID.with(|c| !c.replace(true))
}

pub fn fluid_leave() {
    IN_FLUID.with(|c| c.set(false));
}

const WGSL_BODY: &str = r#"
// One wave of the swell: direction (x, z), wavelength and amplitude in blocks; deep-water
// dispersion (omega = sqrt(g k)) gives each its own speed, slowed to a calm Minecraft sea.
// Returns (height, dh/dx, dh/dz).
fn horizon_physics_wave(w: vec4<f32>, p: vec2<f32>, t: f32, seed: f32, pace: f32) -> vec3<f32> {
    let k = 6.2831853 / w.z;
    let omega = sqrt(9.8 * k) * pace;
    let ph = k * dot(w.xy, p) - omega * t + seed;
    let g = horizon_physics_group(w, p, t, seed, pace);
    let c = w.w * k * cos(ph) * g;
    return vec3(w.w * sin(ph) * g, c * w.x, c * w.y);
}
// Waves come in groups: each rises and falls along its crests and slowly moves on (the game's
// `wave_group`, for the boats and the camera).
fn horizon_physics_group(w: vec4<f32>, p: vec2<f32>, t: f32, seed: f32, pace: f32) -> f32 {
    let k = 6.2831853 / w.z;
    return 0.8 + 0.45 * sin(0.13 * k * dot(vec2(-w.y, w.x), p) + seed * 2.7 + 0.07 * t * pace);
}
// The water's clock: the world's time, so the game can follow the same waves (a boat on them).
fn horizon_physics_water_time() -> f32 {
    return frame.game_time * 1200.0 * HORIZON_PHYSICS_WAVE_SPEED;
}
// How rough the water is: the wind setting.
fn horizon_physics_storm() -> f32 {
    return clamp(0.42 + HORIZON_PHYSICS_WIND * 0.3, 0.15, 1.6);
}
//SWELL//
// The water at world point p (x, z) whose openness is `open` (0 at the shore or in a puddle, 1
// on deep open water): how much of the sea's swell and of a lake's lapping it has.
fn horizon_physics_sea(open: f32) -> f32 {
    return smoothstep(0.5, 0.9, open);
}
fn horizon_physics_swell(p: vec2<f32>, t: f32, open: f32) -> vec3<f32> {
    let sea = horizon_physics_sea(open);
    let lake = clamp(open * 3.0, 0.0, 1.0) * (1.0 - sea);
    var r = vec3(0.0);
    if (sea > 0.0) { r += horizon_physics_ocean(p, t) * sea; }
    if (lake > 0.0) { r += horizon_physics_lake(p, t) * lake; }
    return r * (horizon_physics_storm() * HORIZON_PHYSICS_WAVE_HEIGHT);
}
// The mesher's mark on a water surface's corner (Horizon Physics' fluid hook): its height raised
// by `code` ten-thousandths of a block over vanilla's 8/9 - 0.001, code 0..7 its openness.
// Unmarked (another shape, flowing water): 0, still.
fn horizon_physics_open(position: vec3<f32>, layer: u32) -> f32 {
    if (horizon_physics_kind(layer) != 5u) { return 0.0; }
    let d = (fract(position.y) - 0.8878889) / 0.0001;
    if (d < -0.45 || d > 7.45) { return 0.0; }
    return round(d) / 7.0;
}
// How far a trough may go down where the water is `open`: never below the bottom (the marks'
// depth: a puddle or a shallow shore a little, the open sea all the way), so the seabed's
// blocks never poke up through the water.
fn horizon_physics_floor(open: f32) -> f32 {
    let code = round(open * 7.0);
    if (code <= 1.0) { return 0.35; }
    if (code <= 2.0) { return 1.1; }
    if (code <= 3.0) { return 1.9; }
    return 2.8 + (code - 4.0);
}
// How far out the water moves: all of it close by, none past the shape's reach.
fn horizon_physics_reach(dist: f32) -> f32 {
    return 1.0 - smoothstep(72.0, 104.0, dist);
}
// Ripples: only in the surface's light, never in its shape. Returns the slope.
fn horizon_physics_ripples(p: vec2<f32>, t: f32) -> vec2<f32> {
    var r = horizon_physics_wave(vec4(0.71, 0.70, 1.9, 0.012), p, t, 0.0, 0.7);
    r += horizon_physics_wave(vec4(-0.45, 0.89, 1.3, 0.008), p, t, 2.3, 0.7);
    r += horizon_physics_wave(vec4(0.95, 0.31, 0.95, 0.006), p, t, 4.6, 0.7);
    r += horizon_physics_wave(vec4(0.18, -0.98, 0.7, 0.004), p, t, 6.9, 0.7);
    r += horizon_physics_wave(vec4(-0.86, -0.51, 0.52, 0.003), p, t, 9.2, 0.7);
    r += horizon_physics_wave(vec4(0.55, -0.83, 0.37, 0.002), p, t, 11.5, 0.7);
    return r.yz * (0.6 + 0.6 * horizon_physics_storm());
}
// The wind over the land: a direction that turns slowly, gusts that travel across fields.
fn horizon_physics_wind_dir(t: f32) -> vec2<f32> {
    let a = 0.7 + t * 0.013 + 0.6 * sin(t * 0.071);
    return vec2(cos(a), sin(a));
}
fn horizon_physics_wind(world: vec3<f32>, t: f32) -> f32 {
    let dir = horizon_physics_wind_dir(t);
    // Along the wind, a gust is a band that sweeps over the grass.
    let along = dot(world.xz, dir);
    let gust = 0.55 + 0.3 * sin(along * 0.11 - t * 0.9) + 0.15 * sin(along * 0.37 - t * 1.7 + world.y * 0.2);
    let strength = 0.25 + 0.45 * HORIZON_PHYSICS_WIND;
    return max(gust, 0.05) * strength;
}
// A flutter in two directions at a world point (each plant its own phase).
fn horizon_physics_flutter(p: vec3<f32>, t: f32, rate: f32) -> vec2<f32> {
    let a = dot(p.xz, vec2(0.37, 0.23)) * 6.0 + p.y * 1.9;
    return vec2(sin(t * rate + a) + 0.35 * sin(t * rate * 2.7 + a * 1.3),
                cos(t * rate * 0.83 + a * 1.1) + 0.35 * cos(t * rate * 2.1 + a * 0.7));
}
// How a plant leans at height h (0 its foot, 1 its top) of one block: in the wind, plus flutter.
fn horizon_physics_lean(root: vec3<f32>, t: f32, h: f32, stiff: f32) -> vec3<f32> {
    let dir = horizon_physics_wind_dir(t);
    let w = horizon_physics_wind(root, t);
    let f = horizon_physics_flutter(root, t, 1.1 + 0.6 * w);
    let lean = (dir * (0.5 + 0.5 * w) * w + f * (0.12 + 0.2 * w)) * stiff;
    // A bent stalk: the top moves most, and comes down a little as it bends.
    let x = lean * h * h;
    return vec3(x.x, -dot(x, x) * 0.35, x.y);
}
fn horizon_physics_sway(pos: vec3<f32>, raw: vec3<f32>, uv: vec2<f32>, layer: u32) -> vec3<f32> {
    let kind = horizon_physics_kind(layer);
    if (kind == 0u) { return vec3(0.0); }
    let dist = length(pos);
    // Only close by: far away it would not show, and it stops at the same distance for all.
    let near = select(1.0 - smoothstep(48.0, 64.0, dist), horizon_physics_reach(dist), kind == 5u);
    if (near <= 0.0) { return vec3(0.0); }
    let world = pos + frame.camera_position;
    let t = frame.time;
    // How high up the sprite the vertex is (its foot 0).
    let h = clamp(1.0 - uv.y, 0.0, 1.0);
    let root = vec3(floor(world.x) + 0.5, floor(world.y), floor(world.z) + 0.5);
    var off = vec3(0.0);
    switch (kind) {
        case 1u: {
            // Leaves: every corner rustles on its own, more in the wind.
            let w = horizon_physics_wind(world, t);
            let f = horizon_physics_flutter(world * 1.7, t, 1.4 + w * 0.8);
            let dir = horizon_physics_wind_dir(t);
            off = vec3(f.x * 0.016 + dir.x * 0.02 * w, f.y * 0.01, f.y * 0.016 + dir.y * 0.02 * w) * (0.5 + w);
        }
        case 2u: { off = horizon_physics_lean(root, t, h, 0.16); }
        case 3u: {
            // The top half of a tall plant: its foot is where the bottom half's top went.
            let below = vec3(root.x, root.y - 1.0, root.z);
            off = horizon_physics_lean(below, t, (1.0 + h) * 0.5, 0.16) * 4.0;
        }
        case 4u: {
            // Hanging vines: a slow swing, its phase running down the vine, and the wind.
            let dir = horizon_physics_wind_dir(t);
            let w = horizon_physics_wind(world, t);
            let ph = dot(floor(world.xz), vec2(0.71, 0.53)) - world.y * 0.45;
            let swing = vec2(sin(t * 0.7 + ph), cos(t * 0.55 + ph * 1.3)) * 0.045;
            let x = swing * (0.6 + 0.6 * w) + dir * 0.05 * w;
            off = vec3(x.x, 0.0, x.y);
        }
        case 5u, 9u: {
            // Only the surface: corners at whole blocks (the bed, full water) stay.
            let f = fract(world.y);
            if (f > 0.02 && f < 0.98) {
                if (kind == 5u) {
                    // As open as the mesher marked it: the shore and puddles stay put.
                    let open = horizon_physics_open(raw, layer);
                    if (open > 0.0) {
                        let tw = horizon_physics_water_time();
                        let s = horizon_physics_swell(world.xz, tw, open);
                        let side = horizon_physics_chop(world.xz, tw, open);
                        off = vec3(side.x, max(s.x - 0.04, -horizon_physics_floor(open)), side.y);
                    }
                } else {
                    // Lava: a slow, heavy heave.
                    let tt = t * 0.3;
                    let wave = sin(world.x * 0.7 + tt * 1.3) * 0.5 + sin(world.z * 0.9 - tt * 1.7) * 0.35;
                    off = vec3(0.0, wave * 0.04, 0.0);
                }
            }
        }
        case 6u: {
            // Seagrass: the current, slow and smooth.
            let f = horizon_physics_flutter(root, t * 0.45, 1.2);
            off = vec3(f.x, 0.0, f.y) * 0.13 * h * h;
        }
        case 7u: {
            let below = vec3(root.x, root.y - 1.0, root.z);
            let f = horizon_physics_flutter(below, t * 0.45, 1.2);
            let hh = (1.0 + h) * 0.5;
            off = vec3(f.x, 0.0, f.y) * 0.52 * hh * hh;
        }
        case 8u: {
            // Kelp: the whole stalk in the current, more higher up.
            let f = horizon_physics_flutter(vec3(root.x, 0.0, root.z), t * 0.4, 1.0);
            let bend = sin(t * 0.6 + world.y * 0.35 + dot(root.xz, vec2(0.3, 0.7)));
            off = vec3(f.x + bend * 0.5, 0.0, f.y - bend * 0.3) * 0.08;
        }
        default: {}
    }
    // Walking through: a plant (grass, flowers, seagrass) bends away from the player's feet,
    // the more the closer, its top the most.
    if (HORIZON_PHYSICS_PUSH > 0.5 && (kind == 2u || kind == 3u || kind == 6u || kind == 7u)) {
        let top_half = kind == 3u || kind == 7u;
        let foot = select(root, vec3(root.x, root.y - 1.0, root.z), top_half);
        let hh = select(h, (1.0 + h) * 0.5, top_half);
        let feet = frame.camera_position - vec3(0.0, 1.62, 0.0);
        let d = foot.xz - feet.xz;
        let r = length(d);
        let dy = foot.y - feet.y;
        if (r < 1.2 && dy > -1.5 && dy < 0.6) {
            let k = (1.0 - smoothstep(0.25, 1.2, r)) * (1.0 - smoothstep(0.2, 0.6, abs(dy + 0.2)));
            let away = d / max(r, 0.05);
            let bend = hh * hh * k * 0.55;
            let lift = select(1.0, 2.0, top_half);
            off += vec3(away.x * bend * lift, -bend * 0.45 * lift, away.y * bend * lift);
        }
    }
    // The water's own shape keeps its height setting; the plants sway as far as set.
    let k = select(HORIZON_PHYSICS_SWAY_STRENGTH, 1.0, kind == 5u || kind == 9u);
    return off * (near * k);
}
// The water's surface, lit as water: the swell and ripples bend the light, the sky is mirrored
// more at a glancing angle (Fresnel), the sun glitters on it, light shines through the crests.
// `color` is the material's; `lightmap` vanilla's light colour here.
// Small noise for the water's foam and glints: a hash of the cell's corners, smoothly between them.
// An integer hash of the lattice point (no `sin`: far from the world's origin that turns into
// stripes).
fn horizon_physics_hash(p: vec2<f32>) -> f32 {
    let c = bitcast<vec2<u32>>(vec2<i32>(floor(p)));
    var h = (c.x * 73856093u) ^ (c.y * 19349663u) ^ 0x9e3779b9u;
    h = h ^ (h >> 16u);
    h = h * 0x7feb352du;
    h = h ^ (h >> 15u);
    h = h * 0x846ca68bu;
    h = h ^ (h >> 16u);
    return f32(h & 0xffffffu) / 16777216.0;
}
fn horizon_physics_noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = horizon_physics_hash(i);
    let b = horizon_physics_hash(i + vec2(1.0, 0.0));
    let c = horizon_physics_hash(i + vec2(0.0, 1.0));
    let d = horizon_physics_hash(i + vec2(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}
fn horizon_physics_fbm(p: vec2<f32>) -> f32 {
    return horizon_physics_noise(p) * 0.55 + horizon_physics_noise(p * 2.13 + vec2(5.2, 1.3)) * 0.3
        + horizon_physics_noise(p * 4.41 + vec2(2.7, 8.1)) * 0.15;
}
// The fine waves on top of the swell, only in the light: fourteen small waves, each shorter than
// the one before, spread around the wind's direction (most of them with it). `fw`: how much water
// one pixel covers; a wave shorter than that would only flicker, so it fades out.
fn horizon_physics_detail(p: vec2<f32>, t: f32, wind: vec2<f32>, fw: f32) -> vec2<f32> {
    var g = vec2(0.0);
    var len = 2.8;
    var amp = 0.045;
    let base = atan2(wind.y, wind.x);
    for (var i = 0; i < 14; i++) {
        let fi = f32(i);
        let a = base + sin(fi * 2.39996) * 1.35;
        let d = vec2(cos(a), sin(a));
        let k = 6.2831853 / len;
        let ph = k * dot(d, p) - sqrt(9.8 * k) * 0.55 * t + fi * 1.73;
        let fade = clamp(1.0 - fw * k * 0.4, 0.0, 1.0);
        g += d * (amp * k * cos(ph) * fade);
        len *= 0.79;
        amp *= 0.76;
    }
    return g;
}
// Rain on the water: rings spreading from where each drop hit, two layers of drops so they
// never line up. Returns the slope they add.
fn horizon_physics_rain(p: vec2<f32>, t: f32) -> vec2<f32> {
    var g = vec2(0.0);
    for (var layer = 0; layer < 3; layer++) {
        let fl = f32(layer);
        let q = p * 1.7 + vec2(fl * 0.37, fl * 0.71);
        let cell = floor(q);
        let h = horizon_physics_hash(cell + vec2(fl * 17.0, fl * 5.0));
        let center = cell + 0.3 + 0.4 * vec2(h, fract(h * 7.13));
        let life = fract(t * 0.85 + h * 5.0 + fl * 0.33);
        let d = q - center;
        let r = length(d);
        let x = (r - life * 0.55) * 22.0;
        let ring = sin(x * 1.6) * exp(-x * x * 0.08) * (1.0 - life) * (1.0 - life);
        g += d / max(r, 0.001) * ring * 0.45;
    }
    return g;
}
// The sun on water: GGX, the microfacet highlight (a long, bright sun path on rough water, a
// sharp disc on calm water).
fn horizon_physics_ggx(n: vec3<f32>, v: vec3<f32>, l: vec3<f32>, rough: f32) -> f32 {
    let h = normalize(v + l);
    let nh = max(dot(n, h), 0.0);
    let nl = max(dot(n, l), 0.0);
    let nv = max(dot(n, v), 0.001);
    let a = rough * rough;
    let a2 = a * a;
    let dd = nh * nh * (a2 - 1.0) + 1.0;
    let d = a2 / (3.14159265 * dd * dd);
    let k = a * 0.5;
    let g = (nl / (nl * (1.0 - k) + k)) * (nv / (nv * (1.0 - k) + k));
    let f = 0.02 + 0.98 * pow(1.0 - max(dot(h, v), 0.0), 5.0);
    return d * g * f / (4.0 * nv + 0.001);
}
fn horizon_physics_surface(layer: u32, pos: vec3<f32>, light: vec2<f32>, lightmap: vec3<f32>, open: f32, color: vec4<f32>) -> vec4<f32> {
    // Before any branch (derivatives need the whole quad): the face's own normal, and how much
    // water one pixel covers.
    let geo = cross(dpdx(pos), dpdy(pos));
    let fw = length(fwidth(pos.xz));
    if (!HORIZON_PHYSICS_SHADE || horizon_physics_kind(layer) != 5u) { return color; }
    // A side face of the water (its top moved with the swell, its foot did not) is water too: it
    // gets the surface's look, not the game's flat texture. Its openness is interpolated between
    // the marked top and the unmarked foot, so it is taken as open water.
    let up = abs(geo.y) > 0.6 * length(geo);
    let water_open = select(max(open, 0.5), open, up);
    let world = pos + frame.camera_position;
    let tw = horizon_physics_water_time();
    let dist = length(pos);
    let reach = horizon_physics_reach(dist);
    let sea = horizon_physics_sea(water_open);
    let s = horizon_physics_swell(world.xz, tw, water_open);
    // Seen from under the water: vanilla's look. Under it is where the camera is below this
    // water's surface at the camera's own place, not below this point of it: a crest higher
    // than the eye, seen from above the water, is still seen from above.
    let floor_here = horizon_physics_floor(water_open);
    let level = world.y - max(s.x - 0.04, -floor_here) * reach;
    let cam = frame.camera_position;
    let at_cam = level + max(horizon_physics_swell(cam.xz, tw, water_open).x - 0.04, -floor_here);
    if (cam.y < at_cam) { return color; }
    let storm = horizon_physics_storm();
    let wind = horizon_physics_wind_dir(tw * 0.02);
    // The surface's normal at this pixel: the swell's slope and the fine waves over it (fewer
    // on a sheltered shore, calmer in calm weather).
    let rough_water = (0.45 + 0.55 * clamp(water_open * 3.0, 0.0, 1.0)) * (0.55 + 0.45 * storm);
    // Gusts roughen the water in drifting patches (cat's paws): darker, rougher water beside calm,
    // glassy streaks.
    let gust = horizon_physics_fbm(world.xz * 0.035 + wind * tw * 0.015);
    let paws = 0.45 + 1.1 * smoothstep(0.25, 0.75, gust);
    let fine = horizon_physics_detail(world.xz, tw, wind, fw) * rough_water * paws;
    // The smallest chop, from noise: no wave's lines, just a restless surface close by.
    let micro_fade = clamp(1.0 - fw * 3.0, 0.0, 1.0) * (1.0 - smoothstep(10.0, 40.0, dist));
    let mp = world.xz * 2.3 + wind * tw * 0.06;
    let e = 0.15;
    let m0 = horizon_physics_fbm(mp);
    let micro = vec2(horizon_physics_fbm(mp + vec2(e, 0.0)) - m0, horizon_physics_fbm(mp + vec2(0.0, e)) - m0) / e;
    let rain_fade = frame.rain * smoothstep(0.3, 0.9, light.y) * (1.0 - smoothstep(12.0, 32.0, dist)) * clamp(1.0 - fw * 4.0, 0.0, 1.0);
    var drops = vec2(0.0);
    if (rain_fade > 0.01) { drops = horizon_physics_rain(world.xz, frame.time) * rain_fade; }
    let slope = s.yz * max(reach, 0.25) + fine + micro * 0.07 * micro_fade * rough_water + drops;
    let n = normalize(vec3(-slope.x, 1.0, -slope.y));
    let v = normalize(-pos);
    let cosv = clamp(dot(n, v), 0.0, 1.0);
    let fresnel = clamp(0.035 + 0.965 * pow(1.0 - cosv, 5.0) + 0.05, 0.0, 1.0);
    var refl = reflect(-v, n);
    refl.y = abs(refl.y);
    // Light: the sky light (dark at night, under trees and in caves).
    let bright = max(max(lightmap.r, lightmap.g), lightmap.b);
    let skylight = smoothstep(0.0, 1.0, light.y);
    let sun = normalize(frame.sun_direction);
    let day = smoothstep(-0.05, 0.2, sun.y);
    let sun_rgb = mix(vec3(1.0, 0.55, 0.3), vec3(1.0, 0.95, 0.85), smoothstep(0.0, 0.35, sun.y));
    // The sky it mirrors: haze at the horizon, the sky's own colour higher up, a glow round the
    // sun, and clouds drifting over (soft streaks, darker on their undersides).
    let zenith = frame.sky_color * 0.85;
    var sky = mix(frame.fog_color * 1.05, zenith, pow(clamp(refl.y, 0.0, 1.0), 0.45));
    let around = max(dot(refl, sun), 0.0);
    sky += sun_rgb * (pow(around, 12.0) * 0.35 + pow(around, 3.0) * 0.08) * day;
    let cloud_uv = refl.xz / max(refl.y + 0.15, 0.05) * 1.6 + wind * tw * 0.004;
    let cloud = smoothstep(0.5, 0.85, horizon_physics_fbm(cloud_uv)) * smoothstep(0.02, 0.25, refl.y);
    sky = mix(sky, mix(frame.fog_color, vec3(1.0), 0.75) * (0.55 + 0.45 * day), cloud * 0.55);
    sky *= 0.2 + 0.8 * skylight;
    // The sun's path, sharp and bright, and the glints where a small wave catches it.
    let l = sun;
    // A sharp highlight, and the wide glittering path the sun lays across rough water.
    let spec_sun = horizon_physics_ggx(n, v, l, mix(0.05, 0.16, rough_water)) * max(dot(n, l), 0.0)
        + horizon_physics_ggx(n, v, l, mix(0.22, 0.38, rough_water)) * max(dot(n, l), 0.0) * 1.6;
    let glints = pow(max(dot(refl, sun), 0.0), 1500.0) * 14.0;
    let sparkle = (min(spec_sun, 40.0) * 0.55 + glints) * day * skylight;
    let moon_dir = normalize(frame.moon_direction);
    let moon = (pow(max(dot(refl, moon_dir), 0.0), 600.0) * 3.0 + pow(max(dot(refl, moon_dir), 0.0), 40.0) * 0.08) * (1.0 - day) * skylight;
    // The water itself: clear and greenish where it is shallow (the shore, a pond), deep blue
    // where it is deep and water_open, tinted by the biome's water colour.
    let height = clamp(s.x / (0.75 * storm * max(HORIZON_PHYSICS_WAVE_HEIGHT, 0.05)), -1.0, 1.0);
    let tint = color.rgb / max(max(color.r, max(color.g, color.b)), 0.001);
    let deepness = smoothstep(0.0, 1.0, water_open);
    let shallow_rgb = vec3(0.08, 0.4, 0.4);
    let deep_rgb = mix(vec3(0.025, 0.12, 0.17), vec3(0.014, 0.075, 0.14), sea);
    var water = mix(shallow_rgb, deep_rgb, deepness);
    water *= mix(vec3(1.0), tint, 0.35);
    // Lit from above by the sky, the slopes facing the sun a little more.
    let ambient = (0.35 + 0.65 * bright) * (0.25 + 0.75 * mix(0.18, 1.0, day) * skylight);
    water *= ambient * (0.8 + 0.45 * max(dot(n, sun), 0.0) * day);
    // The daylight scattered inside the water: it glows a little from within.
    water += vec3(0.0, 0.045, 0.06) * day * skylight * (1.0 - 0.5 * deepness);
    // Crests lighter, troughs darker: the swell's shape shows even with the sun behind you.
    water *= 0.7 + 0.6 * (height * 0.5 + 0.5);
    // The sun shining through a crest from behind: the bright turquoise of a wave's thin top.
    let toward = pow(clamp(dot(-v, normalize(vec3(sun.x, 0.0, sun.z) + vec3(0.0, 0.0001, 0.0))) * 0.5 + 0.5, 0.0, 1.0), 3.0);
    let thin = smoothstep(0.15, 0.95, height) * (0.4 + 0.6 * clamp(dot(n, v) * -0.5 + 0.8, 0.0, 1.0));
    water += vec3(0.05, 0.48, 0.42) * thin * toward * day * skylight * 0.55 * (0.3 + 0.7 * sea);
    // A little of the game's own water texture, so it still feels like Minecraft up close.
    let near = 1.0 - smoothstep(4.0, 20.0, dist);
    water = mix(water, color.rgb * ambient * 0.9, 0.12 * near);
    var rgb = mix(water, sky, clamp(fresnel, 0.0, 1.0));
    rgb += sun_rgb * sparkle + vec3(0.75, 0.82, 1.0) * moon;
    // Foam: white caps where the crests break (high and steep, more in a storm), broken up by
    // noise into streaks along the wind; and a line of froth lapping at the shore.
    let steep = length(s.yz);
    let caps = smoothstep(0.55, 1.0, height) * smoothstep(0.08, 0.3, steep + 0.12 * storm) * smoothstep(0.5, 1.3, storm) * sea;
    let along = vec2(dot(world.xz, wind), dot(world.xz, vec2(-wind.y, wind.x)));
    // Lace: foam is a web of bubbles, so it shows as a pattern that fills in as it thickens.
    let lace = horizon_physics_fbm(vec2(along.x * 1.1, along.y * 2.6) + vec2(-tw * 0.35, 0.0)) * 0.6
        + horizon_physics_fbm(world.xz * 4.7 + vec2(tw * 0.08, 0.0)) * 0.4;
    let cover = clamp(caps * 0.55, 0.0, 0.6);
    let cap_foam = smoothstep(1.0 - cover, 1.0 - cover + 0.1, lace) * smoothstep(0.0, 0.2, cover) * 0.85;
    let streak = 0.0;
    // The shore's water (the mesher marks it barely water_open), not the water_open water nor unmarked water.
    let shore = smoothstep(0.0, 0.1, water_open) * (1.0 - smoothstep(0.15, 0.2, water_open));
    let lap = 0.5 + 0.5 * sin(dot(world.xz, vec2(0.37, 0.29)) * 2.0 - tw * 1.3 + horizon_physics_noise(world.xz * 0.7) * 6.0);
    let shore_cover = shore * (lap * 0.6 + 0.4);
    let shore_lace = horizon_physics_fbm(world.xz * 3.7 + vec2(tw * 0.05, -tw * 0.03));
    let shore_foam = smoothstep(1.0 - shore_cover, 1.0 - shore_cover + 0.15, shore_lace) * 0.9;
    let foam = clamp(max(max(cap_foam, shore_foam), streak) * HORIZON_PHYSICS_FOAM, 0.0, 0.9);
    let foam_rgb = vec3(0.93, 0.96, 0.98) * (0.25 + 0.75 * ambient / max(0.35 + 0.65 * bright, 0.001));
    rgb = mix(rgb, foam_rgb * (0.85 + 0.15 * max(dot(n, sun), 0.0)), foam);
    // Far away the water melts into the haze like the land does.
    rgb = mix(rgb, sky, smoothstep(90.0, 240.0, dist) * 0.25);
    // See-through where shallow, nearly opaque where deep; the mirror is opaque.
    let body_alpha = mix(0.5, 0.9, deepness);
    let a = clamp(mix(body_alpha, 1.0, fresnel) + sparkle * 0.2 + foam, 0.0, 1.0);
    return vec4(rgb, a);
}
"#;

/// Horizon's terrain shader for DirectX 12's FXC compiler: `packed_corner` writes the corner's
/// position one component at a time at an index known only on the GPU (`pos[axis] = ...`), and
/// FXC refuses that ("array reference cannot be used as an l-value"): the game crashed at its
/// start on DirectX 12. The same position, put together whole. None when `text` has no such
/// lines (another shader, or a Horizon that changed them).
pub fn fxc_terrain(text: &str) -> Option<String> {
    const OLD: &str = "    var pos = block;
    pos[axis] = block[axis] + side;
    pos[(axis + 1u) % 3u] = block[(axis + 1u) % 3u] + du;
    pos[(axis + 2u) % 3u] = block[(axis + 2u) % 3u] + dw;
";
    const NEW: &str = "    // Horizon Physics: the corner whole, not one component at a time (FXC).
    let pos = block + select(select(vec3(side, du, dw), vec3(dw, side, du), axis == 1u),
        vec3(du, dw, side), axis == 2u);
";
    if !text.contains(OLD) {
        return None;
    }
    Some(text.replace(OLD, NEW))
}

static SHADER_BUSY: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// True when the shader hook was already asking the game for its own shader text (the hook's
/// nested call, which the game answers itself).
pub fn shader_enter() -> bool {
    SHADER_BUSY.swap(true, std::sync::atomic::Ordering::SeqCst)
}

pub fn shader_leave() {
    SHADER_BUSY.store(false, std::sync::atomic::Ordering::SeqCst);
}

/// The material stage's WGSL (the game's `ENTRIES`, as it comes in the shader text after the pack's
/// own source) with the displacement and, for the built-in pack (`source` its stage), the
/// water's look; None when nothing sways or the text is not the one this was written for.
pub fn patch(entries: &str, source: &str) -> Option<String> {
    const VERTEX: &str = "    let pos = horizon_vertex_move(VertexInput(position + origin, uv,\n        horizon_sprite_material(layer & 0x7fffu), light_norm, vec4(color.rgb, 1.0), face, u32(fluid),\n        position, horizon_sprite_tags(layer & 0x7fffu), emission));";
    const FIELD: &str = "    @location(10) @interpolate(flat) emission: f32,\n}";
    const EMISSION: &str = "    out.emission = emission;";
    const COLOR: &str = "    return material(MaterialInput(albedo, in.tint, in.light, in.lightmap, in.position, in.uv,\n        translucent, horizon_sprite_tags(in.layer & 0x7fffu),\n        horizon_sprite_material(in.layer & 0x7fffu), in.clip.xy / frame.resolution,\n        in.face, fluid, in.emission, normal, tangent, colored_light, 0.0)).color;";
    if [VERTEX, FIELD, EMISSION, COLOR]
        .iter()
        .any(|anchor| entries.matches(anchor).count() != 1)
    {
        return None;
    }
    let s = current()?;
    let w = wgsl_with(&s, source.contains(BUILTIN_MARK))?;
    let vertex = VERTEX.replace(
        "let pos = horizon_vertex_move",
        "let pos_base = horizon_vertex_move",
    )
    .replace(
        "        position, horizon_sprite_tags(layer & 0x7fffu), emission));",
        "        position, horizon_sprite_tags(layer & 0x7fffu), emission));\n    let pos = pos_base + horizon_physics_sway(pos_base, position, uv, layer);",
    );
    let color = COLOR.replace(
        "    return material(MaterialInput(",
        "    return horizon_physics_surface(in.layer, in.position, in.light, in.lightmap, in.hp_open, material(MaterialInput(",
    )
    .replace("0.0)).color;", "0.0)).color);");
    let patched = entries
        .replace(VERTEX, &vertex)
        .replace(FIELD, "    @location(10) @interpolate(flat) emission: f32,\n    @location(11) hp_open: f32,\n}")
        .replace(EMISSION, "    out.emission = emission;\n    out.hp_open = horizon_physics_open(position, layer);")
        .replace(COLOR, &color);
    Some(format!("{patched}\n{w}"))
}

// ---------------------------------------------------------------------------------------------
// Snow that gives way (PhysicsMod's snow terrain deformation): where the player and the mobs
// walk, the snow's top is pressed down into a trough with a lip of pushed-up snow along it. The
// game side presses (`snow_press`) into a height map kept per block (`SNOW_GRID` cells a side);
// the sections whose blocks changed are meshed again (`snow_take_dirty`), and the mesher hook
// swaps each pressed block's flat top for a fine grid at those heights (`snow_top`), shaded like
// the game shades its faces. The prints stay as long as the blocks are loaded, up to `SNOW_MAX`
// blocks (the oldest go first).

/// Cells along a block's side.
pub const SNOW_GRID: usize = 8;
const SNOW_PTS: usize = SNOW_GRID + 1;
/// Pressed blocks kept at most.
pub const SNOW_MAX: usize = 3000;

/// One pressed block: how far each grid point is pressed down (negative: pushed up), its snow's
/// top (in the block, 0..1) and when it was last pressed.
#[derive(Clone, Debug)]
pub struct SnowDent {
    pub depth: [f32; SNOW_PTS * SNOW_PTS],
    pub top: f32,
    stamp: u64,
}

#[derive(Default)]
struct Snow {
    world: u64,
    min_y: i32,
    blocks: std::collections::HashMap<(i32, i32, i32), SnowDent>,
    dirty: std::collections::HashSet<(i32, i32, i32)>,
    clock: u64,
}

static SNOW: Mutex<Option<Snow>> = Mutex::new(None);

fn snow_section(min_y: i32, b: (i32, i32, i32)) -> (i32, i32, i32) {
    (b.0 >> 4, (b.1 - min_y) >> 4, b.2 >> 4)
}

/// The world the prints belong to (`key`: the dimension, `min_y` its bottom). Another world
/// forgets them.
pub fn snow_world(key: u64, min_y: i32) {
    let mut g = SNOW.lock().unwrap_or_else(|e| e.into_inner());
    let s = g.get_or_insert_with(Snow::default);
    if s.world != key || s.min_y != min_y {
        // The old world's sections are not this world's: nothing to mesh again for them.
        s.blocks.clear();
        s.dirty.clear();
        s.world = key;
        s.min_y = min_y;
    }
}

/// Forgets every print, and meshes their blocks again flat.
pub fn snow_clear() {
    let mut g = SNOW.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(s) = g.as_mut()
        && !s.blocks.is_empty()
    {
        let min_y = s.min_y;
        let sections: Vec<(i32, i32, i32)> = s.blocks.keys().map(|b| snow_section(min_y, *b)).collect();
        s.dirty.extend(sections);
        s.blocks.clear();
    }
}

/// How much a press at (`cx`, `cz`) (radius `r`, depth `d`, drawn out `stretch` times along the
/// unit direction (`fx`, `fz`)) moves the snow at (`x`, `z`): down inside, a lip pushed up just
/// outside, nothing further out.
pub fn snow_profile(x: f64, z: f64, c: [f64; 4], r: f64, d: f64) -> f64 {
    let (cx, cz, fx, fz) = (c[0], c[1], c[2], c[3]);
    let (dx, dz) = (x - cx, z - cz);
    let along = (dx * fx + dz * fz) / 1.25;
    let across = -dx * fz + dz * fx;
    let q = (along * along + across * across).sqrt() / r;
    if q < 0.3 {
        d
    } else if q < 1.0 {
        // A rounded wall from the floor up to the surface.
        let t = (q - 0.3) / 0.7;
        d * (1.0 - t * t * (3.0 - 2.0 * t))
    } else if q < 1.6 {
        // The lip: the snow pushed aside, a little above the surface.
        let t = (q - 1.0) / 0.6;
        -d * 0.3 * (std::f64::consts::PI * t).sin()
    } else {
        0.0
    }
}

/// Presses the snow at `feet` (x, top of the snow, z), a body `width` wide moving along
/// (`fx`, `fz`). `top` gives a block's snow top (its block y plus its height), None for a block
/// that is not snow. Returns whether anything changed.
static SNOW_DEPTH: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0x3F80_0000);

/// How deep the trails are, of the most a foot sinks (the Snow page's Trail Depth).
pub fn set_snow_depth(k: f32) {
    SNOW_DEPTH.store(k.to_bits(), std::sync::atomic::Ordering::Relaxed);
}

static SNOW_DEEP: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Thick snow is pressed down deeper (the Snow page's Deep Snow Layers): feet sink up to 0,6 of a
/// block into it, not 0,3.
pub fn set_snow_deep(on: bool) {
    SNOW_DEEP.store(on, std::sync::atomic::Ordering::Relaxed);
}

pub fn snow_press(feet: [f64; 3], width: f64, dir: [f64; 2], top: impl Fn(i32, i32, i32) -> Option<f64>) -> bool {
    let surface = feet[1];
    let by = (surface - 1e-3).floor() as i32;
    let height = surface - f64::from(by);
    if !(0.05..=1.0001).contains(&height) {
        return false;
    }
    // As deep as a foot sinks in vanilla: a layer of snow stands an eighth of a block above what
    // the feet stand on (the thinnest layer: down to almost the ground).
    let cap = if SNOW_DEEP.load(std::sync::atomic::Ordering::Relaxed) { 0.6 } else { 0.3 };
    let deepest = (height - 1.0 / 32.0).min(cap).min(height * 0.65);
    let depth = deepest * f64::from(f32::from_bits(SNOW_DEPTH.load(std::sync::atomic::Ordering::Relaxed))).clamp(0.05, 1.0);
    if depth <= 0.005 {
        return false;
    }
    let r = (width * 0.75).clamp(0.3, 1.1);
    let len = (dir[0] * dir[0] + dir[1] * dir[1]).sqrt();
    let (fx, fz) = if len > 1e-6 { (dir[0] / len, dir[1] / len) } else { (1.0, 0.0) };
    let shape = [feet[0], feet[2], fx, fz];
    let same = |x: i32, z: i32| top(x, by, z).is_some_and(|t| (t - surface).abs() < 1e-3);
    let reach = r * 1.6 * 1.25 + 0.01;
    let (x0, x1) = ((feet[0] - reach).floor() as i32, (feet[0] + reach).floor() as i32);
    let (z0, z1) = ((feet[2] - reach).floor() as i32, (feet[2] + reach).floor() as i32);
    let mut g = SNOW.lock().unwrap_or_else(|e| e.into_inner());
    let s = g.get_or_insert_with(Snow::default);
    s.clock += 1;
    let clock = s.clock;
    let min_y = s.min_y;
    let mut changed = false;
    for bx in x0..=x1 {
        for bz in z0..=z1 {
            if !same(bx, bz) {
                continue;
            }
            let mut dent: Option<[f32; SNOW_PTS * SNOW_PTS]> = None;
            for i in 0..SNOW_PTS {
                for j in 0..SNOW_PTS {
                    let x = f64::from(bx) + i as f64 / SNOW_GRID as f64;
                    let z = f64::from(bz) + j as f64 / SNOW_GRID as f64;
                    let mut v = snow_profile(x, z, shape, r, depth);
                    if v.abs() < 1e-4 {
                        continue;
                    }
                    // A point on the block's edge is shared with the next block: it moves only
                    // if every block it touches is the same snow, so the surface stays whole.
                    let xs: &[i32] = if i == 0 { &[-1, 0] } else if i == SNOW_GRID { &[0, 1] } else { &[0] };
                    let zs: &[i32] = if j == 0 { &[-1, 0] } else if j == SNOW_GRID { &[0, 1] } else { &[0] };
                    if !xs.iter().all(|dx| zs.iter().all(|dz| same(bx + dx, bz + dz))) {
                        v = 0.0;
                    }
                    if v == 0.0 {
                        continue;
                    }
                    let d = dent.get_or_insert_with(|| {
                        s.blocks.get(&(bx, by, bz)).map_or([0.0; SNOW_PTS * SNOW_PTS], |b| b.depth)
                    });
                    let k = i * SNOW_PTS + j;
                    let old = f64::from(d[k]);
                    // Pressing wins over the lip; a lip only rises where nothing was pressed.
                    let new = if v > 0.0 { old.max(v) } else if old <= 0.0 { old.min(v) } else { old };
                    if (new - old).abs() > 0.004 {
                        d[k] = new as f32;
                        changed = true;
                    }
                }
            }
            if let Some(d) = dent {
                if d.iter().all(|v| v.abs() < 1e-4) {
                    continue;
                }
                let key = (bx, by, bz);
                let fresh = s.blocks.get(&key).is_none_or(|b| b.depth != d);
                s.blocks.insert(key, SnowDent { depth: d, top: height as f32, stamp: clock });
                if fresh {
                    s.dirty.insert(snow_section(min_y, key));
                }
            }
        }
    }
    if s.blocks.len() > SNOW_MAX {
        let mut ages: Vec<((i32, i32, i32), u64)> = s.blocks.iter().map(|(k, b)| (*k, b.stamp)).collect();
        ages.sort_unstable_by_key(|a| a.1);
        for (k, _) in ages.iter().take(s.blocks.len() - SNOW_MAX) {
            s.blocks.remove(k);
            s.dirty.insert(snow_section(min_y, *k));
        }
    }
    changed
}

/// The sections to mesh again (chunk x, section index from the bottom, chunk z).
/// At most ten times a second: a walk through snow would otherwise mesh its sections every frame.
pub fn snow_take_dirty() -> Vec<(i32, i32, i32)> {
    static LAST: Mutex<Option<std::time::Instant>> = Mutex::new(None);
    if let Ok(mut last) = LAST.lock() {
        let now = std::time::Instant::now();
        if last.is_some_and(|t| now.duration_since(t) < std::time::Duration::from_millis(100)) {
            return Vec::new();
        }
        *last = Some(now);
    }
    let mut g = SNOW.lock().unwrap_or_else(|e| e.into_inner());
    g.as_mut().map(|s| s.dirty.drain().collect()).unwrap_or_default()
}

/// The pressed blocks of the section (chunk `cx`, index `sy`, chunk `cz`), each with its
/// section-relative position.
pub fn snow_in_section(cx: i32, sy: i32, cz: i32) -> Vec<([i32; 3], SnowDent)> {
    let g = SNOW.lock().unwrap_or_else(|e| e.into_inner());
    let Some(s) = g.as_ref() else {
        return Vec::new();
    };
    if s.blocks.is_empty() {
        return Vec::new();
    }
    let base = [cx * 16, s.min_y + sy * 16, cz * 16];
    let mut out = Vec::new();
    for (k, d) in &s.blocks {
        if snow_section(s.min_y, *k) == (cx, sy, cz) {
            out.push(([k.0 - base[0], k.1 - base[1], k.2 - base[2]], d.clone()));
        }
    }
    out
}

/// One corner of a flat top: where it is in the block (0 or 1 along x and z) and what the game
/// gave it: its sprite position, light (sky low byte, block high byte) and colour.
#[derive(Clone, Copy, Debug)]
pub struct SnowCorner {
    pub at: [f32; 2],
    pub uv: [f32; 2],
    pub light: u16,
    pub color: u32,
}

/// The pressed top of a block whose flat top had `corners` at height `y` (section-relative), its
/// cells as quads (four corners each, counter-clockwise seen from above): position, sprite
/// position, light, colour. The colour takes the game's face shading by the slope, and the
/// bottom of a print is a little darker, as the light reaches it less.
pub fn snow_top(origin: [f32; 3], y: f32, dent: &SnowDent, corners: &[SnowCorner; 4]) -> Vec<([f32; 3], [f32; 2], u16, u32)> {
    let n = SNOW_GRID;
    let step = 1.0 / n as f32;
    let h = |i: usize, j: usize| -dent.depth[i.min(n) * SNOW_PTS + j.min(n)];
    // Bilinear in the four corners, by where in the block.
    let lerp = |fx: f32, fz: f32, f: &dyn Fn(&SnowCorner) -> f32| -> f32 {
        let mut acc = 0.0;
        for c in corners {
            let wx = if c.at[0] > 0.5 { fx } else { 1.0 - fx };
            let wz = if c.at[1] > 0.5 { fz } else { 1.0 - fz };
            acc += wx * wz * f(c);
        }
        acc
    };
    let byte = |v: u32, s: u32| ((v >> s) & 255) as f32;
    let point = |i: usize, j: usize| -> ([f32; 3], [f32; 2], u16, u32) {
        let (fx, fz) = (i as f32 * step, j as f32 * step);
        let uv = [lerp(fx, fz, &|c| c.uv[0]), lerp(fx, fz, &|c| c.uv[1])];
        let sky = lerp(fx, fz, &|c| f32::from(c.light & 255)).round().clamp(0.0, 255.0) as u16;
        let block = lerp(fx, fz, &|c| f32::from(c.light >> 8)).round().clamp(0.0, 255.0) as u16;
        // The slope, lit from a low sun: the side facing it stays bright, the far wall of a print
        // falls into a soft blue shadow, and the bottom is darker still (as in thick snow).
        let dx = (h(i + 1, j) - h(i.saturating_sub(1), j)) / (step * if i == 0 || i == n { 1.0 } else { 2.0 });
        let dz = (h(i, j + 1) - h(i, j.saturating_sub(1))) / (step * if j == 0 || j == n { 1.0 } else { 2.0 });
        let len = (dx * dx + 1.0 + dz * dz).sqrt();
        let (nx, ny, nz) = (-dx / len, 1.0 / len, -dz / len);
        let sun = [-0.45f32, 0.80, -0.40];
        let sl = (sun[0] * sun[0] + sun[1] * sun[1] + sun[2] * sun[2]).sqrt();
        let lit = ((nx * sun[0] + ny * sun[1] + nz * sun[2]) / sl).max(0.0) / (sun[1] / sl);
        let deep = (dent.depth[i * SNOW_PTS + j]).max(0.0);
        let k = (0.50 + 0.50 * lit.min(1.0) - deep * 1.1).clamp(0.38, 1.0);
        let cold = 1.0 - k;
        let tint = [1.0 - cold * 0.55, 1.0 - cold * 0.28, 1.0];
        let mut color = 0u32;
        for (c, s) in [0u32, 8, 16].into_iter().enumerate() {
            let v = lerp(fx, fz, &|q| byte(q.color, s)) * k * tint[c];
            color |= (v.round().clamp(0.0, 255.0) as u32) << s;
        }
        color |= corners[0].color & 0xff00_0000;
        let pos = [origin[0] + fx, y + h(i, j), origin[2] + fz];
        (pos, uv, sky | block << 8, color)
    };
    let mut out = Vec::with_capacity(n * n * 4);
    for i in 0..n {
        for j in 0..n {
            out.push(point(i, j));
            out.push(point(i, j + 1));
            out.push(point(i + 1, j + 1));
            out.push(point(i + 1, j));
        }
    }
    out
}

// ---------------------------------------------------------------------------------------------
// The game's particles as physics pieces. The renderer's particle engine hands each particle it
// would add here first (`catch_particle`); what the settings want as pieces waits in a queue
// for the game side, which makes them small physics pieces of the particle's sprite. Which
// kind a particle is comes from where it was made: the game's crack, sprint and eat functions
// set the source around their own work (`particle_source`).

/// Where the particles now being made come from.
pub const SRC_OTHER: u8 = 0;
pub const SRC_CRACK: u8 = 1;
pub const SRC_SPRINT: u8 = 2;
pub const SRC_EAT: u8 = 3;
pub const SRC_ITEM_BREAK: u8 = 4;
/// The game's own break particles, kept (Settings: MC break particles).
pub const SRC_KEEP: u8 = 5;

static SOURCE: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(SRC_OTHER);
/// Which kinds become pieces: bit 0 crack, 1 sprint, 2 eat, 3 item break, 4 other block
/// particles.
static CATCH: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

/// One particle to make a piece of: its sprite (HUD atlas name) and tint, where, its velocity
/// (blocks a tick).
#[derive(Clone, Debug)]
pub struct ParticleBit {
    pub sprite: String,
    pub tint: u32,
    pub at: [f64; 3],
    pub v: [f64; 3],
    /// Whether the game's particle would have spread its velocity (Particle's constructor does,
    /// for the dust of sprinting, the cracks of mining and other block particles).
    pub spread: bool,
}

static BITS: Mutex<Vec<ParticleBit>> = Mutex::new(Vec::new());

pub fn particle_source() -> u8 {
    SOURCE.load(std::sync::atomic::Ordering::Relaxed)
}

/// Sets where particles come from; returns what it was (to put back).
pub fn set_particle_source(src: u8) -> u8 {
    SOURCE.swap(src, std::sync::atomic::Ordering::Relaxed)
}

/// Which kinds the settings want as pieces (see `CATCH`).
pub fn set_particle_catch(mask: u8) {
    CATCH.store(mask, std::sync::atomic::Ordering::Relaxed);
}

/// A particle the engine is about to add: true when it became a piece instead.
pub fn catch_particle(block: Option<&(String, u32)>, item: Option<&Vec<String>>, at: [f64; 3], v: [f64; 3]) -> bool {
    let mask = CATCH.load(std::sync::atomic::Ordering::Relaxed);
    if mask == 0 {
        return false;
    }
    let src = particle_source();
    let wanted = match src {
        SRC_CRACK => mask & 1 != 0,
        SRC_SPRINT => mask & 2 != 0,
        SRC_EAT => mask & 4 != 0,
        SRC_ITEM_BREAK => mask & 8 != 0,
        SRC_KEEP => false,
        _ => (mask & 16 != 0 && block.is_some()) || (mask & 32 != 0 && block.is_none() && item.is_some_and(|l| l.iter().any(|n| thrown(n)))),
    };
    if !wanted {
        return false;
    }
    let (sprite, tint) = match (block, item) {
        (Some((s, t)), _) => (s.clone(), *t),
        (None, Some(layers)) if !layers.is_empty() => (layers[0].clone(), 0xFF_FFFF),
        _ => return false,
    };
    let mut q = BITS.lock().unwrap_or_else(|e| e.into_inner());
    if q.len() >= 256 {
        return true;
    }
    let spread = matches!(src, SRC_CRACK | SRC_SPRINT) || (src != SRC_EAT && src != SRC_ITEM_BREAK && block.is_some());
    q.push(ParticleBit { sprite, tint, at, v, spread });
    true
}

/// A thrown item's sprite (a snowball or an egg breaking where it hit).
fn thrown(sprite: &str) -> bool {
    let n = sprite.rsplit('/').next().unwrap_or(sprite);
    n == "snowball" || n == "egg" || n == "blue_egg" || n == "brown_egg"
}

/// The pieces waiting to be made.
pub fn take_particles() -> Vec<ParticleBit> {
    std::mem::take(&mut *BITS.lock().unwrap_or_else(|e| e.into_inner()))
}

// ---------------------------------------------------------------------------------------------
// Options' "Physics Settings..." button: the options page lays it out here (in the renderer),
// the game's mouse hook finds it here.

static PHYSICS_BUTTON: Mutex<Option<[f32; 4]>> = Mutex::new(None);
static PHYSICS_WANTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);
static IN_LAYOUT: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Whether the button shows (the mod is on).
pub fn physics_button_wanted() -> bool {
    PHYSICS_WANTED.load(std::sync::atomic::Ordering::Relaxed)
}

pub fn set_physics_button_wanted(on: bool) {
    PHYSICS_WANTED.store(on, std::sync::atomic::Ordering::Relaxed);
}

/// True when the layout hook is not inside its own call already (and now is).
pub fn layout_enter() -> bool {
    !IN_LAYOUT.swap(true, std::sync::atomic::Ordering::Relaxed)
}

pub fn layout_leave() {
    IN_LAYOUT.store(false, std::sync::atomic::Ordering::Relaxed);
}

pub fn set_physics_button(rect: [f32; 4]) {
    *PHYSICS_BUTTON.lock().unwrap_or_else(|e| e.into_inner()) = Some(rect);
}

/// Whether (`x`, `y`) is on the button where it was last laid out.
pub fn physics_button_at(x: f32, y: f32) -> bool {
    PHYSICS_BUTTON
        .lock()
        .ok()
        .and_then(|g| *g)
        .is_some_and(|r| physics_wanted_and_in(r, x, y))
}

fn physics_wanted_and_in(r: [f32; 4], x: f32, y: f32) -> bool {
    physics_button_wanted() && x >= r[0] && x < r[0] + r[2] && y >= r[1] && y < r[1] + r[3]
}

// ---------------------------------------------------------------------------------------------
// Banners as cloth: the flag of each banner, drawn every frame by the block entity renderer, is
// bent the way a hanging cloth goes: it lifts and blows out with the wind, and waves run down it.

static BANNERS: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);
static DOORS_ON: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);

/// The Cloth page's Banner Physics and the Dynamic Blocks page's Swinging Doors.
pub fn set_cloth(banners: bool, doors: bool) {
    BANNERS.store(banners, std::sync::atomic::Ordering::Relaxed);
    DOORS_ON.store(doors, std::sync::atomic::Ordering::Relaxed);
}

pub fn banners_on() -> bool {
    BANNERS.load(std::sync::atomic::Ordering::Relaxed)
}

pub fn doors_on() -> bool {
    DOORS_ON.load(std::sync::atomic::Ordering::Relaxed)
}

thread_local! {
    static IN_BANNER: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// The banner hook's guard against itself: true once.
pub fn banner_enter() -> bool {
    IN_BANNER.with(|c| !c.replace(true))
}

pub fn banner_leave() {
    IN_BANNER.with(|c| c.set(false));
}

/// The wind over the land at time `t` (seconds): its direction (x, z) and strength, as the
/// terrain shader blows the plants.
pub fn wind_vector(t: f64) -> [f64; 3] {
    let a = 0.7 + t * 0.013 + 0.6 * (t * 0.071).sin();
    let strength = current().map_or(1.0, |s| f64::from(s.wind));
    [a.cos(), a.sin(), strength]
}

/// An affine matrix (column-major, as glam's `to_cols_array_2d`): a point through it, a direction
/// through it, and its inverse.
fn affine_point(m: &[[f32; 4]; 4], p: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|r| m[0][r] * p[0] + m[1][r] * p[1] + m[2][r] * p[2] + m[3][r])
}

fn affine_dir(m: &[[f32; 4]; 4], p: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|r| m[0][r] * p[0] + m[1][r] * p[1] + m[2][r] * p[2])
}

fn affine_inverse(m: &[[f32; 4]; 4]) -> Option<[[f32; 4]; 4]> {
    let a = |r: usize, c: usize| m[c][r];
    let det = a(0, 0) * (a(1, 1) * a(2, 2) - a(1, 2) * a(2, 1)) - a(0, 1) * (a(1, 0) * a(2, 2) - a(1, 2) * a(2, 0))
        + a(0, 2) * (a(1, 0) * a(2, 1) - a(1, 1) * a(2, 0));
    if det.abs() < 1e-12 {
        return None;
    }
    let i = 1.0 / det;
    // The 3x3 part's inverse, row r column c.
    let inv3 = [
        [(a(1, 1) * a(2, 2) - a(1, 2) * a(2, 1)) * i, (a(0, 2) * a(2, 1) - a(0, 1) * a(2, 2)) * i, (a(0, 1) * a(1, 2) - a(0, 2) * a(1, 1)) * i],
        [(a(1, 2) * a(2, 0) - a(1, 0) * a(2, 2)) * i, (a(0, 0) * a(2, 2) - a(0, 2) * a(2, 0)) * i, (a(0, 2) * a(1, 0) - a(0, 0) * a(1, 2)) * i],
        [(a(1, 0) * a(2, 1) - a(1, 1) * a(2, 0)) * i, (a(0, 1) * a(2, 0) - a(0, 0) * a(2, 1)) * i, (a(0, 0) * a(1, 1) - a(0, 1) * a(1, 0)) * i],
    ];
    let t = [m[3][0], m[3][1], m[3][2]];
    let mut out = [[0.0f32; 4]; 4];
    for c in 0..3 {
        for r in 0..3 {
            out[c][r] = inv3[r][c];
        }
    }
    for r in 0..3 {
        out[3][r] = -(inv3[r][0] * t[0] + inv3[r][1] * t[1] + inv3[r][2] * t[2]);
    }
    out[3][3] = 1.0;
    Some(out)
}

/// Bends the flag of a banner drawn into `verts` (whole quads, as the renderer drew them) under
/// model matrix `m` (column-major; the banner's, before its parts: model units of a sixteenth
/// of a block, y down), at banner position `pos`, time `t` (seconds). The pole and the bar stay.
pub fn banner_deform(verts: &mut [crate::mesher::Vertex], m: [[f32; 4]; 4], pos: [i32; 3], t: f64) {
    if verts.len() % 4 != 0 || verts.is_empty() {
        return;
    }
    let Some(inv) = affine_inverse(&m) else {
        return;
    };
    let local = |p: [f32; 3]| affine_point(&inv, p).map(|c| c * 16.0);
    // Which quads are the flag's: the flag lies in front of the pole (its back face at the
    // pole's front), the bar and the pole are narrow.
    let flag: Vec<bool> = verts
        .chunks_exact(4)
        .map(|q| {
            let l: Vec<[f32; 3]> = q.iter().map(|v| local(v.pos)).collect();
            let (x0, x1) = l.iter().fold((f32::MAX, f32::MIN), |a, p| (a.0.min(p[0]), a.1.max(p[0])));
            let (y0, y1) = l.iter().fold((f32::MAX, f32::MIN), |a, p| (a.0.min(p[1]), a.1.max(p[1])));
            l.iter().any(|p| p[2] < -1.5) || (l.iter().all(|p| (p[2] + 1.0).abs() < 0.1) && x1 - x0 > 3.0 && y1 - y0 > 3.0)
        })
        .collect();
    let (mut top, mut bottom) = (f32::MAX, f32::MIN);
    for (q, f) in verts.chunks_exact(4).zip(&flag) {
        if *f {
            for v in q {
                let y = local(v.pos)[1];
                top = top.min(y);
                bottom = bottom.max(y);
            }
        }
    }
    let height = bottom - top;
    if height < 4.0 {
        return;
    }
    let w = wind_vector(t);
    let n = affine_dir(&m, [0.0, 0.0, 1.0]);
    let nl = (n[0] * n[0] + n[2] * n[2]).sqrt().max(1e-6);
    // How hard the wind blows into the flag's face (its sign: which way it goes out).
    let into = (w[0] as f32 * n[0] / nl + w[1] as f32 * n[2] / nl) * w[2] as f32;
    let seed = (pos[0] as f32 * 0.37 + pos[2] as f32 * 0.71 + pos[1] as f32 * 0.13).rem_euclid(1.0) * 6.28;
    let time = t as f32;
    let gust = 0.75 + 0.25 * (time * 0.9 + seed).sin();
    let lift = (into * gust * 0.32).clamp(-1.05, 1.05);
    let strength = (w[2] as f32).clamp(0.2, 3.0);
    let (s, c) = lift.sin_cos();
    for (q, f) in verts.chunks_exact_mut(4).zip(&flag) {
        if !*f {
            continue;
        }
        for v in q {
            let mut l = local(v.pos);
            let d = l[1] - top;
            let k = (d / height).clamp(0.0, 1.0);
            // Lifted out from its top edge by the wind.
            l[1] = top + d * c;
            l[2] += d * s;
            // Waves running down the cloth, bigger at the free end and in a strong wind.
            let amp = k.powf(1.3) * (0.5 + 0.9 * strength);
            l[2] += amp * ((k * 7.0 - time * (3.0 + strength) + l[0] * 0.18 + seed).sin() * 0.8 + (k * 13.0 - time * 5.1 + seed * 2.0).sin() * 0.25);
            l[0] += amp * 0.25 * (k * 5.0 - time * 2.3 + seed).sin();
            v.pos = affine_point(&m, l.map(|c| c / 16.0));
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Swinging doors: when a door opens or shuts, the block's own (meshed) door is hidden for a
// moment and a panel swings round its hinge instead, overshoots and settles.

#[derive(Clone, Debug)]
pub struct DoorSwing {
    /// The lower half's block.
    pub pos: [i32; 3],
    /// The panel before (block space, one half's box), the hinge (x, z in block space) and the
    /// quarter turn's sign.
    pub from: [f64; 6],
    pub pivot: [f64; 2],
    pub sign: f64,
    /// The lower and upper halves' sprites (HUD atlas uv) and tint.
    pub bottom: [f32; 4],
    pub top: [f32; 4],
    start: std::time::Instant,
    min_y: i32,
}

struct Doors {
    swings: Vec<DoorSwing>,
    dirty: std::collections::HashSet<(i32, i32, i32)>,
    /// Doors seen near the camera: lower half → its state.
    seen: std::collections::HashMap<[i32; 3], u32>,
}

static DOORS: Mutex<Option<Doors>> = Mutex::new(None);

fn with_doors<R>(f: impl FnOnce(&mut Doors) -> R) -> R {
    let mut g = DOORS.lock().unwrap_or_else(|e| e.into_inner());
    f(g.get_or_insert_with(|| Doors { swings: Vec::new(), dirty: Default::default(), seen: Default::default() }))
}

/// How long a swing takes, settling included (seconds).
pub const DOOR_TIME: f64 = 0.9;

/// How far round a swing is at `t` seconds (0 the old place, 1 the new): fast, past the stop,
/// back, and still.
pub fn door_progress(t: f64) -> f64 {
    if t >= DOOR_TIME {
        return 1.0;
    }
    let t = t.max(0.0);
    1.0 - (-7.5 * t).exp() * ((11.0 * t).cos() + 0.68 * (11.0 * t).sin())
}

/// The quarter turn that takes box `from` onto box `to` (block space): the hinge in the column
/// they share, and the turn's sign. None when they do not share a corner.
pub fn door_hinge(from: [f64; 6], to: [f64; 6]) -> Option<([f64; 2], f64)> {
    let ix = (from[0].max(to[0]), from[3].min(to[3]));
    let iz = (from[2].max(to[2]), from[5].min(to[5]));
    if ix.1 - ix.0 < 1e-3 || iz.1 - iz.0 < 1e-3 {
        return None;
    }
    let pivot = [(ix.0 + ix.1) * 0.5, (iz.0 + iz.1) * 0.5];
    let fc = [(from[0] + from[3]) * 0.5 - pivot[0], (from[2] + from[5]) * 0.5 - pivot[1]];
    let tc = [(to[0] + to[3]) * 0.5 - pivot[0], (to[2] + to[5]) * 0.5 - pivot[1]];
    // Turning (x, z) by +90 degrees about y: (x, z) -> (z, -x).
    let plus = [fc[1], -fc[0]];
    let d = |a: [f64; 2]| (a[0] - tc[0]).powi(2) + (a[1] - tc[1]).powi(2);
    let sign = if d(plus) < d([-plus[0], -plus[1]]) { 1.0 } else { -1.0 };
    Some((pivot, sign))
}

fn section_of(p: [i32; 3], min_y: i32) -> (i32, i32, i32) {
    (p[0] >> 4, (p[1] - min_y) >> 4, p[2] >> 4)
}

/// The door at `pos` (its lower half) was `state` this frame. Returns the state it was last
/// seen as when that differs (the caller then starts a swing).
pub fn door_seen(pos: [i32; 3], state: u32) -> Option<u32> {
    with_doors(|d| d.seen.insert(pos, state).filter(|old| *old != state))
}

/// Forgets the doors not near the camera any more.
pub fn doors_forget(keep: impl Fn([i32; 3]) -> bool) {
    with_doors(|d| d.seen.retain(|p, _| keep(*p)));
}

/// A door starts swinging from box `from` to box `to`.
pub fn door_swing(pos: [i32; 3], from: [f64; 6], to: [f64; 6], bottom: [f32; 4], top: [f32; 4], min_y: i32) {
    if !doors_on() {
        return;
    }
    let Some((pivot, sign)) = door_hinge(from, to) else {
        return;
    };
    with_doors(|d| {
        d.swings.retain(|s| s.pos != pos);
        if d.swings.len() >= 32 {
            return;
        }
        d.swings.push(DoorSwing { pos, from, pivot, sign, bottom, top, start: std::time::Instant::now(), min_y });
        d.dirty.insert(section_of(pos, min_y));
        d.dirty.insert(section_of([pos[0], pos[1] + 1, pos[2]], min_y));
    });
}

/// The swings now, each with its angle (radians); finished ones are dropped (their sections
/// are meshed again, with the door back).
pub fn door_swings() -> Vec<(DoorSwing, f64)> {
    with_doors(|d| {
        let now = std::time::Instant::now();
        let mut out = Vec::new();
        let mut done = Vec::new();
        d.swings.retain(|s| {
            let t = now.duration_since(s.start).as_secs_f64();
            if t >= DOOR_TIME {
                done.push((s.pos, s.min_y));
                return false;
            }
            out.push((s.clone(), s.sign * std::f64::consts::FRAC_PI_2 * door_progress(t)));
            true
        });
        for (p, min_y) in done {
            d.dirty.insert(section_of(p, min_y));
            d.dirty.insert(section_of([p[0], p[1] + 1, p[2]], min_y));
        }
        out
    })
}

/// The sections to mesh again for the doors.
pub fn doors_take_dirty() -> Vec<(i32, i32, i32)> {
    with_doors(|d| d.dirty.drain().collect())
}

/// The blocks of the section (chunk `cx`, index `sy`, chunk `cz`) whose door the mesher leaves
/// out (swinging now), section-relative.
pub fn doors_in_section(cx: i32, sy: i32, cz: i32) -> Vec<[i32; 3]> {
    with_doors(|d| {
        let mut out = Vec::new();
        for s in &d.swings {
            for dy in 0..2 {
                let p = [s.pos[0], s.pos[1] + dy, s.pos[2]];
                if section_of(p, s.min_y) == (cx, sy, cz) {
                    out.push([p[0] & 15, (p[1] - s.min_y) & 15, p[2] & 15]);
                }
            }
        }
        out
    })
}
