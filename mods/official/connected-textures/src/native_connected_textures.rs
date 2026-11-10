// Connected Textures for Horizon: the faces of glass, glass panes, bookshelves and sandstone join
// up with the same block beside them, so a window of glass is one pane with one frame round it, a
// wall of bookshelves is one long shelf and a sandstone pillar has no bands between its blocks.
//
// Written from scratch for Horizon. It works on the game's own textures: no texture is shipped.
// A face of a connected block is cut, in its sprite's own space, into the thin strips along its
// edges (a sixteenth of the sprite, a pixel of a 16x16 texture), its corners and its middle. A
// strip along an edge that joins a neighbour shows the texture just inside it instead of the
// frame, so the frame goes away there; a corner where both edges join but the block across the
// corner does not keeps its frame pixel, the small inner corner a connected window has.
//
// The section mesher's hook (`mesh.rs`) hands each section's quads here after Horizon made them.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

pub const ID: &str = "connected_textures";
pub const NAME: &str = "Connected Textures";
pub const VERSION: &str = "0.1.7";
pub const DESCRIPTION: &str = "By tariksabanuz. Glass, glass panes, bookshelves and sandstone connect to the same block beside them: \
one frame round a whole window, one long bookshelf, sandstone walls without seams. Uses your own textures and resource packs.";

/// The parts (`PARTS` bits).
pub const GLASS: u32 = 1;
pub const PANES: u32 = 2;
pub const BOOKSHELVES: u32 = 4;
pub const SANDSTONE: u32 = 8;
const ALL: u32 = GLASS | PANES | BOOKSHELVES | SANDSTONE;

static ON: AtomicBool = AtomicBool::new(true);
/// Whether the mod is on in Options > Mods (the frame hook sets it).
static ACTIVE: AtomicBool = AtomicBool::new(true);
static PARTS: AtomicU32 = AtomicU32::new(ALL);
static LOADED: AtomicBool = AtomicBool::new(false);
/// A setting changed: every section is meshed again (the frame hook asks the game).
static REMESH: AtomicBool = AtomicBool::new(false);
static DIR: Mutex<Option<std::path::PathBuf>> = Mutex::new(None);

/// The sprite's frame: a sixteenth of it.
pub const EDGE: f32 = 1.0 / 16.0;
pub const SEAM_INSET: f32 = 512.0 / 32768.0;
/// `TerrainVertex` UV steps per sprite.
pub const UV_STEPS: f32 = 32768.0;

pub fn seam_clamp(x: f32, lo: f32, hi: f32) -> f32 {
    let inset = SEAM_INSET.min((hi - lo).max(0.0) * 0.5);
    let (a, b) = (lo + inset, hi - inset);
    if a <= b { x.clamp(a, b) } else { x }
}
/// The bit of a terrain vertex's layer that marks it opaque.
const OPAQUE: u16 = 0x8000;

pub fn enabled() -> bool {
    ON.load(Ordering::Relaxed) && ACTIVE.load(Ordering::Relaxed) && PARTS.load(Ordering::Relaxed) != 0
}

fn part_on(part: u32) -> bool {
    PARTS.load(Ordering::Relaxed) & part != 0
}

/// The mod is on or off in Options > Mods: when that changes, everything is meshed again.
pub fn set_active(on: bool) {
    if ACTIVE.swap(on, Ordering::Relaxed) != on {
        REMESH.store(true, Ordering::Relaxed);
    }
}

pub fn take_remesh() -> bool {
    REMESH.swap(false, Ordering::Relaxed)
}

pub fn loaded() -> bool {
    LOADED.load(Ordering::Relaxed)
}

/// Clears cached config and resource-derived rules when Android changes profiles.
pub fn reset() {
    ON.store(true, Ordering::Relaxed);
    ACTIVE.store(true, Ordering::Relaxed);
    PARTS.store(ALL, Ordering::Relaxed);
    LOADED.store(false, Ordering::Relaxed);
    REMESH.store(false, Ordering::Relaxed);
    if let Ok(mut dir) = DIR.lock() {
        *dir = None;
    }
    if let Ok(mut rules) = RULES.lock() {
        *rules = None;
    }
}

/// Drops only the resource-derived atlas/state table after a successful pack reload.
pub fn invalidate_rules() {
    if let Ok(mut rules) = RULES.lock() {
        *rules = None;
    }
}

/// Reads `connected_textures.txt` from the game's config folder (`key=0|1` lines).
pub fn load(dir: Option<std::path::PathBuf>) {
    LOADED.store(true, Ordering::Relaxed);
    let Some(dir) = dir else {
        return;
    };
    let text = std::fs::read_to_string(dir.join("connected_textures.txt")).unwrap_or_default();
    let (on, parts) = parse(&text);
    ON.store(on, Ordering::Relaxed);
    PARTS.store(parts, Ordering::Relaxed);
    if let Ok(mut d) = DIR.lock() {
        *d = Some(dir);
    }
}

/// The settings of a settings file's text: on, and the parts.
pub fn parse(text: &str) -> (bool, u32) {
    let (mut on, mut parts) = (true, ALL);
    for line in text.lines() {
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let yes = v.trim() != "0";
        let bit = match k.trim() {
            "enabled" => {
                on = yes;
                continue;
            }
            "glass" => GLASS,
            "panes" => PANES,
            "bookshelves" => BOOKSHELVES,
            "sandstone" => SANDSTONE,
            _ => continue,
        };
        if yes {
            parts |= bit;
        } else {
            parts &= !bit;
        }
    }
    (on, parts)
}

fn save() {
    let Some(dir) = DIR.lock().ok().and_then(|d| d.clone()) else {
        return;
    };
    let parts = PARTS.load(Ordering::Relaxed);
    let b = |bit: u32| u8::from(parts & bit != 0);
    let text = format!(
        "enabled={}\nglass={}\npanes={}\nbookshelves={}\nsandstone={}\n",
        u8::from(ON.load(Ordering::Relaxed)),
        b(GLASS),
        b(PANES),
        b(BOOKSHELVES),
        b(SANDSTONE)
    );
    let _ = std::fs::write(dir.join("connected_textures.txt"), text);
}

fn on_off(v: bool) -> &'static str {
    if v { "ON" } else { "OFF" }
}

/// The buttons of the mod's settings page.
pub fn setting_labels() -> Vec<String> {
    let parts = PARTS.load(Ordering::Relaxed);
    vec![
        format!("Connected Textures: {}", on_off(ON.load(Ordering::Relaxed))),
        format!("Glass: {}", on_off(parts & GLASS != 0)),
        format!("Glass Panes: {}", on_off(parts & PANES != 0)),
        format!("Bookshelves: {}", on_off(parts & BOOKSHELVES != 0)),
        format!("Sandstone: {}", on_off(parts & SANDSTONE != 0)),
    ]
}

/// A settings button was clicked: its setting flips, is saved, and the world is meshed again.
pub fn setting_click(button: usize) {
    match button {
        0 => {
            ON.fetch_xor(true, Ordering::Relaxed);
        }
        1..=4 => {
            PARTS.fetch_xor(1 << (button - 1), Ordering::Relaxed);
        }
        _ => return,
    }
    save();
    REMESH.store(true, Ordering::Relaxed);
}

thread_local! {
    static INSIDE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// The mesher's hook calls the mesher again from inside itself: true once, until `leave`.
pub fn enter() -> bool {
    INSIDE.with(|c| !c.replace(true))
}

pub fn leave() {
    INSIDE.with(|c| c.set(false));
}

// ---------------------------------------------------------------------------------------------
// Which blocks connect, and how.

/// How a block's faces connect.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Method {
    /// All four edges of every face (glass).
    Full,
    /// All four edges of the side faces (glass panes).
    Pane,
    /// Left and right on the side faces (bookshelves, cut and chiseled sandstone).
    Horizontal,
    /// A side face with the same block above it shows this sprite layer instead: the block's
    /// smooth top, so a column of sandstone has no bands between its blocks.
    Top(u16),
}

#[derive(Clone, Copy, Debug)]
pub struct Group {
    pub method: Method,
    /// The sprite layer of the faces that connect (the block's own texture).
    pub tile: u16,
    pub part: u32,
    /// How wide the sprite's frame is on its high-u side (chiseled sandstone: three pixels).
    pub hi_u: f32,
    /// A pane's edge sprite (`glass_pane_top`): its tops and bottoms between two panes are
    /// left out, as vanilla's own culling would with a cullface on them.
    pub edge: Option<u16>,
}

/// The table for one set of loaded resources: each block state's group (0: none), each group.
pub struct Rules {
    key: [usize; 3],
    by_state: Vec<u16>,
    pub groups: Vec<Group>,
    /// Sprite layers some group connects (a quick test before anything else).
    tiles: Vec<bool>,
}

impl Rules {
    pub fn is_empty(&self) -> bool {
        self.groups.is_empty()
    }
    pub fn has_tile(&self, layer: u16) -> bool {
        self.tiles.get(usize::from(layer & !OPAQUE)).copied().unwrap_or(false)
    }
    /// The group of block state `state`, 1-based (0: none).
    pub fn group(&self, state: u32) -> u16 {
        self.by_state.get(state as usize).copied().unwrap_or(0)
    }
}

const COLORS: [&str; 16] = [
    "white", "orange", "magenta", "light_blue", "yellow", "lime", "pink", "gray", "light_gray", "cyan",
    "purple", "blue", "brown", "green", "red", "black",
];

/// The blocks that connect: block, its texture, how, and its part.
pub fn blocks() -> Vec<(String, String, Option<String>, Method, u32)> {
    // (block, connecting texture, replacement texture for Top, method, part)
    let mut v: Vec<(String, String, Option<String>, Method, u32)> = Vec::new();
    v.push(("glass".into(), "glass".into(), None, Method::Full, GLASS));
    v.push(("tinted_glass".into(), "tinted_glass".into(), None, Method::Full, GLASS));
    v.push(("glass_pane".into(), "glass".into(), None, Method::Pane, PANES));
    for c in COLORS {
        v.push((format!("{c}_stained_glass"), format!("{c}_stained_glass"), None, Method::Full, GLASS));
        v.push((format!("{c}_stained_glass_pane"), format!("{c}_stained_glass"), None, Method::Pane, PANES));
    }
    v.push(("bookshelf".into(), "bookshelf".into(), None, Method::Horizontal, BOOKSHELVES));
    for s in ["sandstone", "red_sandstone"] {
        v.push((format!("cut_{s}"), format!("cut_{s}"), None, Method::Horizontal, SANDSTONE));
        v.push((format!("chiseled_{s}"), format!("chiseled_{s}"), None, Method::Horizontal, SANDSTONE));
        v.push((s.into(), s.into(), Some(format!("{s}_bottom")), Method::Top(0), SANDSTONE));
    }
    v
}

static RULES: Mutex<Option<Arc<Rules>>> = Mutex::new(None);

/// The table for the resources `key` names (they change on a resource reload): `states` block
/// states, `name` a state's block, `layer` a sprite's layer.
pub fn rules(
    key: [usize; 3],
    states: usize,
    name: &dyn Fn(u32) -> Option<String>,
    layer: &dyn Fn(&str) -> Option<u16>,
) -> Arc<Rules> {
    let mut g = RULES.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(r) = g.as_ref()
        && r.key == key
    {
        return r.clone();
    }
    let sprite = |n: &str| -> Option<u16> {
        layer(&format!("minecraft:block/{n}")).or_else(|| layer(&format!("block/{n}")))
    };
    let mut groups = Vec::new();
    let mut index = std::collections::HashMap::new();
    for (block, tile, top, method, part) in blocks() {
        let Some(t) = sprite(&tile) else {
            continue;
        };
        let method = match (method, top) {
            (Method::Top(_), Some(top)) => match sprite(&top) {
                Some(l) => Method::Top(l),
                None => continue,
            },
            (m, _) => m,
        };
        let hi_u = if block.starts_with("chiseled_") { 3.0 * EDGE } else { EDGE };
        let edge = if method == Method::Pane { sprite(&format!("{block}_top")) } else { None };
        groups.push(Group { method, tile: t, part, hi_u, edge });
        index.insert(format!("minecraft:{block}"), groups.len() as u16);
    }
    let mut by_state = vec![0u16; states];
    for (id, slot) in by_state.iter_mut().enumerate() {
        if let Some(n) = name(id as u32) {
            let n = if n.contains(':') { n } else { format!("minecraft:{n}") };
            if let Some(&gi) = index.get(&n) {
                *slot = gi;
            }
        }
    }
    let most = groups.iter().map(|g| usize::from(g.tile.max(g.edge.unwrap_or(0))) + 1).max().unwrap_or(0);
    let mut tiles = vec![false; most];
    for g in &groups {
        tiles[usize::from(g.tile)] = true;
        if let Some(e) = g.edge {
            tiles[usize::from(e)] = true;
        }
    }
    let r = Arc::new(Rules { key, by_state, groups, tiles });
    *g = Some(r.clone());
    r
}

/// A terrain vertex as the hook hands it: section-relative position, UV in `UV_STEPS`, sprite
/// layer, light, colour.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct V {
    pub pos: [f32; 3],
    pub uv: [u16; 2],
    pub layer: u16,
    pub light: u16,
    pub color: u32,
}

/// The quads of `src` (four vertices each) with the connected faces cut and joined, or None
/// when nothing changed. `at` gives the block state at a section-relative cell (the border of
/// two blocks round the section included).
pub fn process(r: &Rules, src: &[V], at: &dyn Fn(i32, i32, i32) -> u32) -> Option<Vec<V>> {
    if src.len() % 4 != 0 || r.is_empty() {
        return None;
    }
    let mut out: Option<Vec<V>> = None;
    for (qi, q) in src.chunks_exact(4).enumerate() {
        let quad = [q[0], q[1], q[2], q[3]];
        let done = if r.has_tile(q[0].layer) { connect(r, &quad, at) } else { None };
        match done {
            Some(rep) => {
                let o = out.get_or_insert_with(|| {
                    let mut v = Vec::with_capacity(src.len() + 64);
                    v.extend_from_slice(&src[..qi * 4]);
                    v
                });
                o.extend(rep);
            }
            None => {
                if let Some(o) = out.as_mut() {
                    o.extend_from_slice(q);
                }
            }
        }
    }
    out
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

/// The axis a vector mostly points along, as a unit step, if it points along one only.
fn axis(v: [f32; 3]) -> Option<[i32; 3]> {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len < 1e-6 {
        return None;
    }
    let k = (0..3).max_by(|&a, &b| v[a].abs().total_cmp(&v[b].abs()))?;
    if (0..3).any(|j| j != k && v[j].abs() > len * 1e-3) {
        return None;
    }
    let mut d = [0; 3];
    d[k] = if v[k] > 0.0 { 1 } else { -1 };
    Some(d)
}

fn add(a: [i32; 3], b: [i32; 3]) -> [i32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn neg(a: [i32; 3]) -> [i32; 3] {
    [-a[0], -a[1], -a[2]]
}

/// One quad of a connecting block, joined to its neighbours, or None to keep it as it is.
fn connect(r: &Rules, q: &[V; 4], at: &dyn Fn(i32, i32, i32) -> u32) -> Option<Vec<V>> {
    let layer = q[0].layer & !OPAQUE;
    let p: [[f32; 3]; 4] = [q[0].pos, q[1].pos, q[2].pos, q[3].pos];
    let (e1, e2, e3) = (sub(p[1], p[0]), sub(p[2], p[0]), sub(p[3], p[0]));
    let n = axis(cross(e1, e2))?;
    let c = [0, 1, 2].map(|k| (p[0][k] + p[1][k] + p[2][k] + p[3][k]) / 4.0);
    let cell = [0, 1, 2].map(|k| (c[k] - n[k] as f32 * 0.01).floor() as i32);
    let get = |d: [i32; 3]| at(d[0], d[1], d[2]);
    let g = r.group(get(cell));
    if g == 0 {
        return None;
    }
    let group = r.groups[usize::from(g - 1)];
    if group.edge == Some(layer) {
        // The top or bottom of a pane, on its block's edge, with the same pane over it: hidden.
        let on_edge = (c[1] - cell[1] as f32).abs() < 1e-3 || (c[1] - cell[1] as f32 - 1.0).abs() < 1e-3;
        let over = r.group(get(add(cell, n))) == g;
        return (n[1] != 0 && on_edge && over && part_on(group.part)).then(Vec::new);
    }
    if group.tile != layer || !part_on(group.part) {
        return None;
    }
    let side = n[1] == 0;
    let same = |d: [i32; 3]| r.group(get(d)) == g;
    // A neighbour joins when it is the same block and its own face here is not hidden.
    let joins = |d: [i32; 3]| same(add(cell, d)) && !same(add(add(cell, d), n));
    match group.method {
        Method::Top(top) => {
            if !side || !same(add(cell, [0, 1, 0])) {
                return None;
            }
            let mut out = q.to_vec();
            for v in &mut out {
                v.layer = (v.layer & OPAQUE) | top;
            }
            return Some(out);
        }
        Method::Pane | Method::Horizontal if !side => return None,
        _ => {}
    }
    let uv: [[f32; 2]; 4] = q.map(|v| [f32::from(v.uv[0]) / UV_STEPS, f32::from(v.uv[1]) / UV_STEPS]);
    if uv.iter().any(|t| t[0] < -1e-4 || t[0] > 1.0001 || t[1] < -1e-4 || t[1] > 1.0001) {
        return None;
    }
    // Which of the quad's two edges (q0→q1 "a", q0→q3 "b") the sprite's u runs along.
    let (du_a, du_b) = (uv[1][0] - uv[0][0], uv[3][0] - uv[0][0]);
    let (dv_a, dv_b) = (uv[1][1] - uv[0][1], uv[3][1] - uv[0][1]);
    let u_on_a = du_a.abs() > 1e-5 && du_b.abs() < 1e-5 && dv_a.abs() < 1e-5 && dv_b.abs() > 1e-5;
    let u_on_b = du_b.abs() > 1e-5 && du_a.abs() < 1e-5 && dv_b.abs() < 1e-5 && dv_a.abs() > 1e-5;
    if !u_on_a && !u_on_b {
        return None;
    }
    let scale = |v: [f32; 3], s: f32| [v[0] * s, v[1] * s, v[2] * s];
    let (dir_u, dir_v) = if u_on_a {
        (axis(scale(e1, du_a.signum()))?, axis(scale(e3, dv_b.signum()))?)
    } else {
        (axis(scale(e3, du_b.signum()))?, axis(scale(e1, dv_a.signum()))?)
    };
    let horizontal_only = group.method == Method::Horizontal;
    let use_u = !horizontal_only || dir_u[1] == 0;
    let use_v = !horizontal_only || dir_v[1] == 0;
    // The sprite's edges: u = 0 lies towards -dir_u, u = 1 towards +dir_u; the same for v.
    let lo_u = use_u && joins(neg(dir_u));
    let hi_u = use_u && joins(dir_u);
    let lo_v = use_v && joins(neg(dir_v));
    let hi_v = use_v && joins(dir_v);
    if !(lo_u || hi_u || lo_v || hi_v) {
        return None;
    }
    let corner = |su: i32, sv: i32| -> bool {
        let d = add(scale_i(dir_u, su), scale_i(dir_v, sv));
        joins(d)
    };
    let range = |k: usize| -> (f32, f32) {
        let lo = uv.iter().map(|t| t[k]).fold(f32::INFINITY, f32::min);
        let hi = uv.iter().map(|t| t[k]).fold(f32::NEG_INFINITY, f32::max);
        (lo, hi)
    };
    let cuts = |(lo, hi): (f32, f32), far: f32| -> Vec<f32> {
        let mut c = vec![lo];
        for x in [EDGE, 1.0 - far] {
            if x > lo + 1e-5 && x < hi - 1e-5 {
                c.push(x);
            }
        }
        c.push(hi);
        c
    };
    let (us, vs) = (cuts(range(0), group.hi_u), cuts(range(1), EDGE));
    // From sprite u (or v) to the quad's parameter along its edge.
    let u0 = uv[0][0];
    let v0 = uv[0][1];
    let (du, dv) = if u_on_a { (du_a, dv_b) } else { (du_b, dv_a) };
    let pu = |u: f32| (u - u0) / du;
    let pv = |v: f32| (v - v0) / dv;
    let band = |x: f32, far: f32| -> u8 {
        if x < EDGE {
            0
        } else if x > 1.0 - far {
            2
        } else {
            1
        }
    };
    let mut out = Vec::with_capacity(36);
    for wu in us.windows(2) {
        for wv in vs.windows(2) {
            let (bu, bv) = (band((wu[0] + wu[1]) * 0.5, group.hi_u), band((wv[0] + wv[1]) * 0.5, EDGE));
            let mut su = match bu {
                0 if lo_u => EDGE,
                2 if hi_u => -group.hi_u,
                _ => 0.0,
            };
            let mut sv = match bv {
                0 if lo_v => EDGE,
                2 if hi_v => -EDGE,
                _ => 0.0,
            };
            if su != 0.0 && sv != 0.0 && !corner(if bu == 0 { -1 } else { 1 }, if bv == 0 { -1 } else { 1 }) {
                // Both edges join, the block across the corner does not: its frame pixel stays.
                su = 0.0;
                sv = 0.0;
            }
            let (a_lo, a_hi, b_lo, b_hi) = if u_on_a {
                let (x, y) = (pu(wu[0]), pu(wu[1]));
                let (s, t) = (pv(wv[0]), pv(wv[1]));
                (x.min(y), x.max(y), s.min(t), s.max(t))
            } else {
                let (x, y) = (pv(wv[0]), pv(wv[1]));
                let (s, t) = (pu(wu[0]), pu(wu[1]));
                (x.min(y), x.max(y), s.min(t), s.max(t))
            };
            for (a, b) in [(a_lo, b_lo), (a_hi, b_lo), (a_hi, b_hi), (a_lo, b_hi)] {
                let mut v = lerp(q, a, b);
                let mut t = [f32::from(v.uv[0]) / UV_STEPS + su, f32::from(v.uv[1]) / UV_STEPS + sv];
                let (ulo, uhi) = (wu[0].min(wu[1]) + su, wu[0].max(wu[1]) + su);
                let (vlo, vhi) = (wv[0].min(wv[1]) + sv, wv[0].max(wv[1]) + sv);
                t[0] = seam_clamp(t[0], ulo, uhi);
                t[1] = seam_clamp(t[1], vlo, vhi);
                v.uv = t.map(|x| (x * UV_STEPS).round().clamp(0.0, 65535.0) as u16);
                out.push(v);
            }
        }
    }
    Some(out)
}

fn scale_i(d: [i32; 3], s: i32) -> [i32; 3] {
    [d[0] * s, d[1] * s, d[2] * s]
}

/// The vertex at (`a` along q0→q1, `b` along q0→q3) of the quad, everything bilinear.
fn lerp(q: &[V; 4], a: f32, b: f32) -> V {
    let w = [(1.0 - a) * (1.0 - b), a * (1.0 - b), a * b, (1.0 - a) * b];
    let mix = |f: &dyn Fn(&V) -> f32| -> f32 { (0..4).map(|i| w[i] * f(&q[i])).sum() };
    let pos = [0, 1, 2].map(|k| mix(&|v| v.pos[k]));
    let uv = [0, 1].map(|k| mix(&|v| f32::from(v.uv[k])).round().clamp(0.0, 65535.0) as u16);
    let byte = |x: u32, s: u32| ((x >> s) & 255) as f32;
    let light = [0u32, 8].iter().fold(0u16, |acc, &s| {
        acc | ((mix(&|v| byte(u32::from(v.light), s)).round().clamp(0.0, 255.0) as u16) << s)
    });
    let color = [0u32, 8, 16, 24].iter().fold(0u32, |acc, &s| {
        acc | ((mix(&|v| byte(v.color, s)).round().clamp(0.0, 255.0) as u32) << s)
    });
    V { pos, uv, layer: q[0].layer, light, color }
}

#[cfg(test)]
mod seam_tests {
    use super::{SEAM_INSET, seam_clamp};

    #[test]
    fn seam_clamp_keeps_samples_inside_their_piece() {
        assert_eq!(seam_clamp(-1.0, 0.0, 0.5), SEAM_INSET);
        assert_eq!(seam_clamp(1.0, 0.0, 0.5), 0.5 - SEAM_INSET);
        assert_eq!(seam_clamp(0.25, 0.0, 0.5), 0.25);
    }

    #[test]
    fn seam_clamp_does_not_invert_narrow_resource_pack_faces() {
        assert!((seam_clamp(-1.0, 0.25, 0.25) - 0.25).abs() < 1e-6);
        assert!((seam_clamp(0.255, 0.25, 0.26) - 0.255).abs() < 1e-6);
        assert!((seam_clamp(1.0, 0.25, 0.26) - 0.255).abs() < 1e-6);
        assert_eq!(seam_clamp(0.3, 0.4, 0.2), 0.3);
    }
}
