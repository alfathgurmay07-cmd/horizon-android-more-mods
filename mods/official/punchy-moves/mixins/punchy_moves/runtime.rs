// Punchy Moves for Horizon: the game side, injected as `crate::game::punchy_moves`.
// Once a frame it hands every player's movement to the rig
// (`horizon_client::mobs::punchy_moves`), which the model hooks read, and
// keeps the settings, the page in Options > Mods and the keys. Elytra trails
// are in `crate::game::punchy_moves_trails` (MPL-2.0).

use super::Game;
use super::punchy_moves_trails as trails;
use horizon_client::mobs::punchy_moves as rig;
use super::native_hooks::NativeMod;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::sync::Mutex;
use std::time::Instant;

pub(crate) const ID: &str = "punchy_moves";
const NAME: &str = "Punchy Moves";
const VERSION: &str = "1.2.0";
const DESCRIPTION: &str = "Livelier third-person player movement: lean, bank, bounce, landing squash, jump poses, swings that follow through, mining, eating and drinking poses, maps in both hands, compasses, lanterns, climbing hand over hand, item inspect, elytra rolls, twirls and contrails, swim strokes.";
const KEY_TOGGLE: (&str, &str, &str) = ("toggle", "Toggle Punchy Moves", "key.keyboard.y");
const KEY_INSPECT: (&str, &str, &str) = ("inspect", "Inspect held item", "key.keyboard.i");
const KEY_TWIRL_LEFT: (&str, &str, &str) = ("twirl_left", "Elytra twirl left", "key.keyboard.comma");
const KEY_TWIRL_RIGHT: (&str, &str, &str) = ("twirl_right", "Elytra twirl right", "key.keyboard.period");
const KEYS: [(&str, &str, &str); 4] = [KEY_TOGGLE, KEY_INSPECT, KEY_TWIRL_LEFT, KEY_TWIRL_RIGHT];

/// `config.json`.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
struct Config {
    enabled: bool,
    others: bool,
    lean: f32,
    bank: f32,
    bounce: f32,
    landing: f32,
    arms: f32,
    air: f32,
    idle: f32,
    elytra: f32,
    swim: f32,
    items: f32,
    climb: f32,
    actions: f32,
    steady_head: bool,
    twirls: bool,
    trails: trails::TrailConfig,
}
impl Default for Config {
    fn default() -> Self {
        Self::from((rig::Settings::default(), true, trails::TrailConfig::default()))
    }
}
impl From<(rig::Settings, bool, trails::TrailConfig)> for Config {
    fn from((s, twirls, trails): (rig::Settings, bool, trails::TrailConfig)) -> Self {
        Self {
            enabled: s.enabled,
            others: s.others,
            lean: s.lean,
            bank: s.bank,
            bounce: s.bounce,
            landing: s.landing,
            arms: s.arms,
            air: s.air,
            idle: s.idle,
            elytra: s.elytra,
            swim: s.swim,
            items: s.items,
            climb: s.climb,
            actions: s.actions,
            steady_head: s.steady_head,
            twirls,
            trails,
        }
    }
}
impl Config {
    fn settings(self) -> rig::Settings {
        let a = |v: f32| if v.is_finite() { v.clamp(0.0, 2.0) } else { 1.0 };
        rig::Settings {
            enabled: self.enabled,
            others: self.others,
            lean: a(self.lean),
            bank: a(self.bank),
            bounce: a(self.bounce),
            landing: a(self.landing),
            arms: a(self.arms),
            air: a(self.air),
            idle: a(self.idle),
            elytra: a(self.elytra),
            swim: a(self.swim),
            items: a(self.items),
            climb: a(self.climb),
            actions: a(self.actions),
            steady_head: self.steady_head,
        }
    }
    fn trails(self) -> trails::TrailConfig {
        let mut t = self.trails;
        let f = |v: f32, lo: f32, hi: f32, d: f32| if v.is_finite() { v.clamp(lo, hi) } else { d };
        t.width = f(t.width, 0.0, 2.0, 0.05);
        t.lifetime = f(t.lifetime, 0.05, 30.0, 2.5);
        t.min_speed = f(t.min_speed, 0.0, 10.0, 0.8);
        t.alpha = f(t.alpha, 0.0, 1.0, 1.0);
        t.color = t.color.min(trails::COLORS.len() - 1);
        t
    }
}

struct State {
    loaded: bool,
    dir: Option<std::path::PathBuf>,
    last: Option<Instant>,
    twirls: bool,
    /// Keys pressed since the last frame.
    inspect: bool,
    twirl: i8,
}
static STATE: Mutex<State> = Mutex::new(State {
    loaded: false,
    dir: None,
    last: None,
    twirls: true,
    inspect: false,
    twirl: 0,
});

fn state() -> std::sync::MutexGuard<'static, State> {
    STATE.lock().unwrap_or_else(|e| e.into_inner())
}

thread_local! {
    /// This frame's players, kept to fill again without allocating.
    static INPUTS: RefCell<Vec<rig::Input>> = const { RefCell::new(Vec::new()) };
}

fn ensure_loaded(game: &Game<'_>) {
    let mut s = state();
    if s.loaded {
        return;
    }
    let dir = game.native_data_dir(ID);
    let cfg = std::fs::read_to_string(dir.join("config.json"))
        .ok()
        .and_then(|t| serde_json::from_str::<Config>(&t).ok())
        .unwrap_or_default();
    rig::set_settings(cfg.settings());
    trails::set_config(cfg.trails());
    s.twirls = cfg.twirls;
    s.dir = Some(dir);
    s.loaded = true;
}

fn save() {
    let s = state();
    let Some(dir) = &s.dir else { return };
    let _ = std::fs::create_dir_all(dir);
    let cfg = Config::from((rig::settings(), s.twirls, trails::config()));
    if let Ok(text) = serde_json::to_string_pretty(&cfg) {
        let tmp = dir.join("config.json.tmp");
        if std::fs::write(&tmp, text).is_ok() {
            let _ = std::fs::rename(&tmp, dir.join("config.json"));
        }
    }
}

// ---------------------------------------------------------------- once a frame

pub(crate) fn frame(game: &mut Game<'_>) {
    let now = Instant::now();
    let (dt, inspect, twirl) = {
        let mut s = state();
        let dt = s.last.map_or(0.0, |t| now.duration_since(t).as_secs_f32());
        s.last = Some(now);
        let twirl = if s.twirls { s.twirl } else { 0 };
        let keys = (std::mem::take(&mut s.inspect), twirl);
        s.twirl = 0;
        (dt, keys.0, keys.1)
    };
    if !game.native_on(ID) {
        rig::update(&[], dt);
        return;
    }
    ensure_loaded(game);
    let g: &Game<'_> = game;
    INPUTS.with(|cell| {
        let mut inputs = cell.borrow_mut();
        inputs.clear();
        if let Some(m) = g.player_model.as_ref() {
            let (busy, gliding, swimming) =
                g.player.as_ref().map_or((false, false, false), |p| (p.sleeping, p.fall_flying, p.swimming));
            let mut input = rig::Input::with(m, busy, gliding, swimming);
            input.mining = g.mining.is_some();
            input.inspect = inspect;
            input.twirl = twirl;
            inputs.push(input);
        }
        for m in g.mobs.iter() {
            if &*m.kind.id == "minecraft:player" && g.player_model.as_ref().is_none_or(|p| p.id != m.id) {
                inputs.push(rig::Input::of(m, false));
            }
        }
        rig::update(&inputs, dt);
    });
}

// ---------------------------------------------------------------- the world: trails

pub(crate) fn world(game: &Game<'_>, camera: glam::DVec3, partial: f64, out: &mut Vec<horizon_client::mesher::Vertex>) {
    if !game.native_window() {
        return;
    }
    ensure_loaded(game);
    trails::world(game, camera, partial, out);
}

// ---------------------------------------------------------------- Options > Mods, the keys

pub(crate) fn listed(mods: &mut Vec<NativeMod>) {
    mods.push(NativeMod {
        id: ID.into(),
        title: format!("{NAME} {VERSION}"),
        description: DESCRIPTION.into(),
        keys: KEYS.iter().map(|(n, l, d)| [(*n).to_owned(), (*l).to_owned(), (*d).to_owned()]).collect(),
    });
}

/// A setting's field in [`rig::Settings`].
type Field = fn(&mut rig::Settings) -> &mut f32;
const AMOUNTS: [(&str, Field); 12] = [
    ("Lean into runs", |s| &mut s.lean),
    ("Bank into turns", |s| &mut s.bank),
    ("Stride bounce", |s| &mut s.bounce),
    ("Landing squash", |s| &mut s.landing),
    ("Arm swing", |s| &mut s.arms),
    ("Jump pose", |s| &mut s.air),
    ("Idle breathing", |s| &mut s.idle),
    ("Elytra flight", |s| &mut s.elytra),
    ("Swimming", |s| &mut s.swim),
    ("Item poses (eat, map, lantern)", |s| &mut s.items),
    ("Climbing", |s| &mut s.climb),
    ("Swings, mining, inspect", |s| &mut s.actions),
];

fn on_off(b: bool) -> &'static str {
    if b { "ON" } else { "OFF" }
}

/// The page's buttons, in order: (label, what a click does).
enum Button {
    Enabled,
    Others,
    Amount(usize),
    SteadyHead,
    Twirls,
    Trails,
    TrailsOthers,
    TrailPreset,
    TrailColor,
    TrailWidth,
    TrailLength,
    TrailSpeed,
    TrailGlow,
    TrailFade,
    Reset,
}
fn buttons() -> Vec<Button> {
    let mut b = vec![Button::Enabled, Button::Others];
    b.extend((0..AMOUNTS.len()).map(Button::Amount));
    b.extend([
        Button::SteadyHead,
        Button::Twirls,
        Button::Trails,
        Button::TrailsOthers,
        Button::TrailPreset,
        Button::TrailColor,
        Button::TrailWidth,
        Button::TrailLength,
        Button::TrailSpeed,
        Button::TrailGlow,
        Button::TrailFade,
        Button::Reset,
    ]);
    b
}

const WIDTHS: [f32; 7] = [0.03, 0.05, 0.075, 0.1, 0.15, 0.25, 0.5];
const LENGTHS: [f32; 6] = [0.6, 1.0, 2.5, 4.0, 6.0, 10.0];
const SPEEDS: [f32; 5] = [0.0, 0.5, 0.75, 0.8, 1.2];

/// The next value after `v` in `steps`, round again.
fn next(v: f32, steps: &[f32]) -> f32 {
    steps.iter().copied().find(|s| *s > v + 1e-4).unwrap_or(steps[0])
}

pub(crate) fn settings_page(game: &mut Game<'_>, mod_id: &str, out: &mut Vec<String>) {
    if mod_id != ID {
        return;
    }
    ensure_loaded(game);
    let mut s = rig::settings();
    let t = trails::config();
    let twirls = state().twirls;
    let preset = trails::PRESETS.iter().find(|(_, p)| {
        let mut p = *p;
        p.enabled = t.enabled;
        p.others = t.others;
        p.min_speed = t.min_speed;
        p == t
    });
    for b in buttons() {
        out.push(match b {
            Button::Enabled => format!("Punchy Moves: {}", on_off(s.enabled)),
            Button::Others => format!("Other players: {}", on_off(s.others)),
            Button::Amount(i) => format!("{}: {:.0}%", AMOUNTS[i].0, *AMOUNTS[i].1(&mut s) * 100.0),
            Button::SteadyHead => format!("Steady head: {}", on_off(s.steady_head)),
            Button::Twirls => format!("Elytra twirls (, and .): {}", on_off(twirls)),
            Button::Trails => format!("Elytra trails: {}", on_off(t.enabled)),
            Button::TrailsOthers => format!("Other players' trails: {}", on_off(t.others)),
            Button::TrailPreset => format!("Trail preset: {}", preset.map_or("Custom", |p| p.0)),
            Button::TrailColor => format!("Trail colour: {}", trails::COLORS[t.color.min(trails::COLORS.len() - 1)].0),
            Button::TrailWidth => format!("Trail width: {:.2} blocks", t.width),
            Button::TrailLength => format!("Trail duration: {:.1} s", t.lifetime),
            Button::TrailSpeed => format!("Trail minimum speed: {:.2}", t.min_speed),
            Button::TrailGlow => format!("Glowing trails: {}", on_off(t.glowing)),
            Button::TrailFade => format!("Trails fade out: {}", on_off(t.fade)),
            Button::Reset => "Reset to defaults".to_owned(),
        });
    }
}

pub(crate) fn settings_click(game: &mut Game<'_>, mod_id: &str, button: usize) {
    if mod_id != ID {
        return;
    }
    ensure_loaded(game);
    let Some(b) = buttons().into_iter().nth(button) else { return };
    let mut s = rig::settings();
    let mut t = trails::config();
    match b {
        Button::Enabled => s.enabled = !s.enabled,
        Button::Others => s.others = !s.others,
        Button::Amount(i) => {
            // 0% to 200% in steps of 25%, round again.
            let v = AMOUNTS[i].1(&mut s);
            *v = if *v >= 1.999 { 0.0 } else { ((*v / 0.25).round() + 1.0) * 0.25 };
        }
        Button::SteadyHead => s.steady_head = !s.steady_head,
        Button::Twirls => {
            let mut st = state();
            st.twirls = !st.twirls;
        }
        Button::Trails => t.enabled = !t.enabled,
        Button::TrailsOthers => t.others = !t.others,
        Button::TrailPreset => {
            let i = trails::PRESETS.iter().position(|(_, p)| p.width == t.width && p.color == t.color && p.lifetime == t.lifetime);
            let (enabled, others, min_speed) = (t.enabled, t.others, t.min_speed);
            t = trails::PRESETS[i.map_or(0, |i| (i + 1) % trails::PRESETS.len())].1;
            (t.enabled, t.others, t.min_speed) = (enabled, others, min_speed);
        }
        Button::TrailColor => t.color = (t.color + 1) % trails::COLORS.len(),
        Button::TrailWidth => t.width = next(t.width, &WIDTHS),
        Button::TrailLength => t.lifetime = next(t.lifetime, &LENGTHS),
        Button::TrailSpeed => t.min_speed = next(t.min_speed, &SPEEDS),
        Button::TrailGlow => t.glowing = !t.glowing,
        Button::TrailFade => t.fade = !t.fade,
        Button::Reset => {
            s = rig::Settings::default();
            t = trails::TrailConfig::default();
            state().twirls = true;
        }
    }
    rig::set_settings(s);
    trails::set_config(t);
    save();
}

fn bound(game: &Game<'_>, k: (&str, &str, &str), key: &str) -> bool {
    game.native_mod_key(ID, k.0, k.2) == key
}

pub(crate) fn key(game: &mut Game<'_>, key: &str, pressed: bool) {
    if !pressed || !game.native_window() || !game.native_playing() || !game.native_on(ID) {
        return;
    }
    ensure_loaded(game);
    if bound(game, KEY_TOGGLE, key) {
        let mut s = rig::settings();
        s.enabled = !s.enabled;
        rig::set_settings(s);
        save();
        game.native_chat(if s.enabled { "[Punchy Moves] On." } else { "[Punchy Moves] Off: vanilla animations." });
    } else if bound(game, KEY_INSPECT, key) {
        state().inspect = true;
    } else if bound(game, KEY_TWIRL_LEFT, key) {
        state().twirl = -1;
    } else if bound(game, KEY_TWIRL_RIGHT, key) {
        state().twirl = 1;
    }
}
