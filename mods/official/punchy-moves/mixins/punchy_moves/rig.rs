// Punchy Moves' animation rig, injected as `horizon_client::mobs::punchy_moves`.
//
// Every player gets a few springs, stepped once a frame from how they move:
// a forward lean that overshoots when they start and stop, a bank into turns
// and strafes, a stride bounce, a squash on landing that springs back, and an
// airborne pose. The model hooks read the springs: `body` tilts the whole
// model (as setupRotations would), `pose` adds to the part angles after
// vanilla's setupAnim. Nothing here allocates once the maps have grown.

use super::Mob;
use super::anim::{Poses, State};
use crate::fast_hash::FxHashMap;
use std::cell::Cell;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, RwLock};

/// What the menu sets. Amounts are 0 (off) to 2 (double); 1 is the default.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Settings {
    pub enabled: bool,
    /// Animate other players too, not just you.
    pub others: bool,
    pub lean: f32,
    pub bank: f32,
    pub bounce: f32,
    pub landing: f32,
    pub arms: f32,
    pub air: f32,
    pub idle: f32,
    /// Rolling into turns and spreading the arms with an elytra.
    pub elytra: f32,
    /// Rolling with the strokes and into turns when swimming.
    pub swim: f32,
    /// Eating, drinking, maps, compasses, lanterns held as you would hold them.
    pub items: f32,
    /// Reaching up hand over hand on ladders and vines.
    pub climb: f32,
    /// Swings that follow through with the body, the mining rhythm, the inspect pose.
    pub actions: f32,
    /// The head stays level while the body leans.
    pub steady_head: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            enabled: true,
            others: true,
            lean: 1.0,
            bank: 1.0,
            bounce: 1.0,
            landing: 1.0,
            arms: 1.0,
            air: 1.0,
            idle: 1.0,
            elytra: 1.0,
            swim: 1.0,
            items: 1.0,
            climb: 1.0,
            actions: 1.0,
            steady_head: true,
        }
    }
}

static SETTINGS: Mutex<Settings> = Mutex::new(Settings {
    enabled: true,
    others: true,
    lean: 1.0,
    bank: 1.0,
    bounce: 1.0,
    landing: 1.0,
    arms: 1.0,
    air: 1.0,
    idle: 1.0,
    elytra: 1.0,
    swim: 1.0,
    items: 1.0,
    climb: 1.0,
    actions: 1.0,
    steady_head: true,
});

pub fn settings() -> Settings {
    *SETTINGS.lock().unwrap_or_else(|e| e.into_inner())
}
pub fn set_settings(s: Settings) {
    *SETTINGS.lock().unwrap_or_else(|e| e.into_inner()) = s;
}

/// What a hand holds, as far as the animations care.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Held {
    #[default]
    Other,
    /// Eaten: food (`UseAnim.EAT`).
    Food,
    /// Drunk: potions, milk, honey (`UseAnim.DRINK`).
    Drink,
    /// A filled map: held out in both hands when the other is free.
    Map,
    /// A compass or a clock: held up to read.
    Dial,
    /// A torch or a lantern: held out a little.
    Light,
}
impl Held {
    pub fn of(item: Option<&str>) -> Self {
        let Some(id) = item else { return Self::Other };
        let id = id.strip_prefix("minecraft:").unwrap_or(id);
        match id {
            "potion" | "milk_bucket" | "honey_bottle" | "ominous_bottle" => Self::Drink,
            "filled_map" => Self::Map,
            "compass" | "recovery_compass" | "clock" => Self::Dial,
            _ if id.ends_with("torch") || id.ends_with("lantern") => Self::Light,
            _ => Self::Other,
        }
    }
    /// What a hand that is being used holds: a drink or (anything not used
    /// some other way) food.
    fn used(item: Option<&str>) -> Self {
        let id = item.map(|i| i.strip_prefix("minecraft:").unwrap_or(i)).unwrap_or("");
        match Self::of(item) {
            Self::Drink => Self::Drink,
            _ if id.is_empty()
                || matches!(
                    id,
                    "bow" | "crossbow" | "trident" | "shield" | "spyglass" | "goat_horn" | "brush"
                        | "bundle" | "ender_eye" | "fishing_rod"
                )
                || id.ends_with("spear")
                || id.ends_with("bundle") =>
            {
                Self::Other
            }
            _ => Self::Food,
        }
    }
}

/// A damped spring: `x` chases a target with stiffness `k` and damping `d`.
#[derive(Clone, Copy, Debug, Default)]
struct Spring {
    x: f32,
    v: f32,
}
impl Spring {
    fn step(&mut self, target: f32, k: f32, d: f32, dt: f32) {
        // Semi-implicit Euler, in sub-steps so a slow frame stays stable.
        let n = (dt / (1.0 / 120.0)).ceil().clamp(1.0, 8.0);
        let h = dt / n;
        for _ in 0..n as u32 {
            self.v += ((target - self.x) * k - self.v * d) * h;
            self.x += self.v * h;
        }
    }
}

/// One player's springs and what they remember.
#[derive(Clone, Copy, Debug, Default)]
struct Rig {
    weight: Spring,
    lean: Spring,
    bank: Spring,
    squash: Spring,
    air: Spring,
    rise: Spring,
    /// Gliding with an elytra, swimming: 0 to 1.
    glide: Spring,
    swim: Spring,
    /// Roll about the body's long axis (radians, right wing down +).
    roll: Spring,
    /// 0 level flight, 1 straight down.
    dive: Spring,
    /// Seconds spent gliding or swimming, for the flutter and the strokes.
    clock: f32,
    /// Hands: eating or drinking, a map, a dial, a light (0 to 1 each).
    eat: Spring,
    map: Spring,
    dial: Spring,
    light: Spring,
    climb: Spring,
    /// Blocks climbed, for the hand-over-hand rhythm.
    climbed: f32,
    /// A twirl (barrel roll): seconds into it, and which way (0: none).
    twirl_t: f32,
    twirl_dir: f32,
    /// Mining (you only: the game knows when you break a block).
    mining: Spring,
    /// Seconds into an inspect, or below 0 when not inspecting.
    inspect_t: f32,
    /// Forward speed, slowly: the lean overshoots by how far speed is ahead of it.
    fwd_slow: f32,
    was_airborne: bool,
    /// The fastest fall since leaving the ground, blocks a tick.
    fall: f32,
    seen: u32,
}

/// What the model hooks read, per entity id.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Out {
    /// 0 to 1: how much of the rig shows (fades out when swimming, gliding...).
    pub weight: f32,
    /// Radians: forward (+) and toward the right (+).
    pub lean: f32,
    pub bank: f32,
    /// Squash (+) or stretch (-), as a fraction of height.
    pub squash: f32,
    /// 0 to 1: in the air; `rise` 1 while going up.
    pub air: f32,
    pub rise: f32,
    pub sprint: f32,
    pub bounce: f32,
    pub arms: f32,
    pub idle: f32,
    pub steady_head: bool,
    pub glide: f32,
    pub swim: f32,
    pub roll: f32,
    pub dive: f32,
    /// Blocks a tick, any direction.
    pub speed: f32,
    pub clock: f32,
    /// The hand poses, 0 to 1, and which hand (true: the left).
    pub eat: f32,
    pub drink: bool,
    pub eat_left: bool,
    pub map: f32,
    pub dial: f32,
    pub dial_left: bool,
    pub light: f32,
    pub light_left: bool,
    pub climb: f32,
    pub climbed: f32,
    /// Radians of twirl about the long axis while gliding.
    pub twirl: f32,
    pub mining: f32,
    /// Seconds into an inspect (below 0: none).
    pub inspect: f32,
    pub actions: f32,
    pub sprinting: f32,
    pub glide_now: bool,
    pub main_left: bool,
}

struct Rigs {
    rigs: FxHashMap<u32, Rig>,
    frame: u32,
}
static RIGS: Mutex<Option<Rigs>> = Mutex::new(None);
static OUT: RwLock<Option<FxHashMap<u32, Out>>> = RwLock::new(None);
/// Bumped by every `update`: a thread's copy of `OUT` older than this is stale.
static GENERATION: AtomicU64 = AtomicU64::new(0);

/// Everything about one player that a frame's update needs.
#[derive(Clone, Copy, Debug)]
pub struct Input {
    pub id: u32,
    /// Blocks moved last tick.
    pub delta: [f64; 3],
    /// Degrees the body turned last tick.
    pub turn: f32,
    pub body_yaw: f32,
    pub on_ground: bool,
    pub sprinting: bool,
    pub crouching: bool,
    /// Degrees the look turned last tick (an elytra flies where you look).
    pub look_turn: f32,
    /// Riding, sleeping, dead, spinning: no rig.
    pub busy: bool,
    pub in_water: bool,
    pub gliding: bool,
    pub swimming: bool,
    pub climbing: bool,
    /// What each hand holds; which one is used (eating, drinking); which is the main hand.
    pub main: Held,
    pub off: Held,
    pub using: Option<Held>,
    pub using_left: bool,
    pub main_left: bool,
    /// Start a twirl this frame: -1 to the left, 1 to the right.
    pub twirl: i8,
    /// Breaking a block (you only).
    pub mining: bool,
    /// Start an inspect this frame (you only).
    pub inspect: bool,
}

impl Input {
    /// `busy`: riding, sleeping and so on, as the game knows it; `gliding` and
    /// `swimming` are added to what the model says.
    pub fn of(m: &Mob, busy: bool) -> Self {
        Self::with(m, busy, false, false)
    }
    pub fn with(m: &Mob, busy: bool, gliding: bool, swimming: bool) -> Self {
        let wrap = |a: f32| (a + 180.0).rem_euclid(360.0) - 180.0;
        Self {
            id: m.id,
            delta: [m.position.x - m.previous.x, m.position.y - m.previous.y, m.position.z - m.previous.z],
            turn: wrap(m.body_yaw - m.previous_body_yaw),
            body_yaw: m.body_yaw,
            on_ground: m.on_ground,
            sprinting: m.extra.sprinting,
            crouching: m.extra.crouching,
            look_turn: wrap(m.yaw - m.previous_yaw),
            busy: busy
                || m.extra.passenger
                || m.extra.auto_spin_attack
                || m.death_time > 0
                || m.extra.synced_pose == 2,
            in_water: m.fluids.in_water || m.fluids.in_lava,
            gliding: gliding || m.extra.fall_flying,
            swimming: swimming || m.extra.swim.amount(1.0) > 0.5,
            climbing: m.climbing && !m.on_ground,
            main: Held::of(m.extra.main_hand.as_deref()),
            off: Held::of(m.extra.off_hand.as_deref()),
            using: (m.extra.using_item > 0).then(|| {
                Held::used(if m.extra.using_off_hand { m.extra.off_hand.as_deref() } else { m.extra.main_hand.as_deref() })
            }),
            using_left: m.extra.using_off_hand != m.main_arm_left(),
            main_left: m.main_arm_left(),
            twirl: 0,
            mining: false,
            inspect: false,
        }
    }
}

/// Steps every player's springs by `dt` seconds. `players` is this frame's
/// players (you first); the ones not listed are forgotten after a second.
pub fn update(players: &[Input], dt: f32) {
    let set = settings();
    let dt = dt.clamp(0.0, 0.1);
    let mut guard = RIGS.lock().unwrap_or_else(|e| e.into_inner());
    let rigs = guard.get_or_insert_with(|| Rigs { rigs: FxHashMap::default(), frame: 0 });
    rigs.frame = rigs.frame.wrapping_add(1);
    let frame = rigs.frame;
    let mut out = OUT.write().unwrap_or_else(|e| e.into_inner());
    let out = out.get_or_insert_with(FxHashMap::default);
    for (i, p) in players.iter().enumerate() {
        if !set.enabled || (i > 0 && !set.others) {
            out.remove(&p.id);
            continue;
        }
        let r = rigs.rigs.entry(p.id).or_insert_with(|| Rig { inspect_t: -1.0, ..Rig::default() });
        r.seen = frame;
        let o = step(r, p, &set, dt);
        out.insert(p.id, o);
    }
    rigs.rigs.retain(|_, r| frame.wrapping_sub(r.seen) < 120);
    out.retain(|id, _| rigs.rigs.get(id).is_some_and(|r| r.seen == frame));
    GENERATION.fetch_add(1, Ordering::Release);
}

fn step(r: &mut Rig, p: &Input, set: &Settings, dt: f32) -> Out {
    let yaw = p.body_yaw.to_radians();
    let (sin, cos) = yaw.sin_cos();
    let [dx, dy, dz] = p.delta;
    // Facing (-sin, cos); the right hand points (-cos, -sin).
    let fwd = (-(dx as f32) * sin + dz as f32 * cos).clamp(-1.0, 1.0);
    let side = (-(dx as f32) * cos - dz as f32 * sin).clamp(-1.0, 1.0);
    let horizontal = (fwd * fwd + side * side).sqrt();
    let vy = dy as f32;

    r.weight.step(if p.busy { 0.0 } else { 1.0 }, 200.0, 28.0, dt);
    if p.busy && r.weight.x < 0.02 {
        // Gone: exactly vanilla, not a hair off.
        r.weight = Spring::default();
    }
    // Speed now against speed a moment ago: lean forward when setting off,
    // back when stopping, more than the speed alone would.
    r.fwd_slow += (fwd - r.fwd_slow) * (1.0 - (-dt * 4.0).exp());
    let surge = (fwd - r.fwd_slow).clamp(-0.3, 0.3);
    let sprint = if p.sprinting { 1.0 } else { 0.0 };
    let crouch = if p.crouching { 0.4 } else { 1.0 };
    let lean = ((fwd * 0.55 + surge * 1.6 + sprint * fwd * 0.35) * set.lean * crouch).clamp(-0.3, 0.42);
    r.lean.step(lean, 90.0, 13.0, dt);
    // Bank into turns, more the faster you go; and toward a strafe.
    let moving = (horizontal / 0.2).min(1.0);
    let bank = ((p.turn * 0.018 * moving + side * 0.6) * set.bank).clamp(-0.32, 0.32);
    r.bank.step(bank, 70.0, 11.0, dt);

    // Gliding and swimming: the body lies along the way it goes (vanilla turns
    // it); roll it into turns, and with the strokes when swimming.
    let speed = ((dx * dx + dy * dy + dz * dz) as f32).sqrt();
    r.glide.step(if p.gliding { 1.0 } else { 0.0 }, 120.0, 22.0, dt);
    r.swim.step(if p.swimming && !p.gliding { 1.0 } else { 0.0 }, 120.0, 22.0, dt);
    if p.gliding || p.swimming {
        r.clock += dt;
    } else {
        r.clock = 0.0;
    }
    let fast = (speed / 1.2).clamp(0.0, 1.0);
    let roll = if p.gliding {
        // An elytra goes where you look: roll by how fast that turns, more at speed.
        (p.look_turn * 0.075 * (0.4 + 0.6 * fast) * set.elytra).clamp(-1.1, 1.1)
    } else if p.swimming {
        let stroke = (r.clock * 5.2).sin() * 0.22 * (speed / 0.2).min(1.0);
        ((p.turn * 0.04).clamp(-0.5, 0.5) + stroke) * set.swim
    } else {
        0.0
    };
    // Softer than the ground springs: a glider's roll lags and settles.
    r.roll.step(roll, 30.0, 7.0, dt);
    let dive = if speed > 0.05 { (-vy / speed).clamp(0.0, 1.0) } else { 0.0 };
    r.dive.step(if p.gliding { dive } else { 0.0 }, 40.0, 11.0, dt);

    // Hands. Eating or drinking: the used hand to the mouth.
    let eating = matches!(p.using, Some(Held::Food | Held::Drink));
    r.eat.step(if eating { 1.0 } else { 0.0 }, 140.0, 22.0, dt);
    // A map with the other hand free, not in use: held in both hands.
    let map_two = !eating && p.using.is_none() && ((p.main == Held::Map && p.off == Held::Other) || (p.off == Held::Map && p.main == Held::Other));
    r.map.step(if map_two { 1.0 } else { 0.0 }, 110.0, 20.0, dt);
    let dial = !eating && !map_two && (p.main == Held::Dial || p.off == Held::Dial);
    r.dial.step(if dial { 1.0 } else { 0.0 }, 110.0, 20.0, dt);
    let light = !eating && (p.main == Held::Light || p.off == Held::Light);
    r.light.step(if light { 1.0 } else { 0.0 }, 90.0, 18.0, dt);
    // Climbing: hand over hand, in time with the height climbed.
    r.climb.step(if p.climbing && !p.busy { 1.0 } else { 0.0 }, 120.0, 22.0, dt);
    if p.climbing {
        r.climbed += vy.abs() * dt * 20.0;
    }
    // A twirl: a full roll, eased in and out, while gliding.
    if p.twirl != 0 && r.twirl_dir == 0.0 && p.gliding {
        r.twirl_dir = f32::from(p.twirl);
        r.twirl_t = 0.0;
    }
    let twirl = if r.twirl_dir != 0.0 {
        r.twirl_t += dt;
        let k = (r.twirl_t / TWIRL_SECONDS).min(1.0);
        let angle = r.twirl_dir * std::f32::consts::TAU * (k * k * (3.0 - 2.0 * k));
        if k >= 1.0 || !p.gliding {
            r.twirl_dir = 0.0;
        }
        angle
    } else {
        0.0
    };

    // Mining: the swing keeps a rhythm; an inspect runs its course unless
    // something else starts.
    r.mining.step(if p.mining { 1.0 } else { 0.0 }, 120.0, 22.0, dt);
    if p.inspect && r.inspect_t < 0.0 {
        r.inspect_t = 0.0;
    } else if p.inspect {
        // Pressed again: stop.
        r.inspect_t = -1.0;
    }
    if r.inspect_t >= 0.0 {
        r.inspect_t += dt;
        if r.inspect_t > INSPECT_SECONDS || p.mining || p.using.is_some() || p.busy || p.gliding {
            r.inspect_t = -1.0;
        }
    }

    // In the air: a pose to blend to, and the fall to land from.
    let airborne = !p.on_ground && !p.in_water && !p.busy && !p.gliding;
    r.air.step(if airborne { 1.0 } else { 0.0 }, 160.0, 24.0, dt);
    r.rise.step(if airborne && vy > 0.02 { 1.0 } else { 0.0 }, 160.0, 24.0, dt);
    if airborne {
        r.fall = r.fall.max(-vy);
        if !r.was_airborne && vy > 0.2 {
            // Take-off: a quick stretch.
            r.squash.v -= 2.2 * set.landing;
        }
    } else if r.was_airborne && !p.gliding {
        // Landing: squash by how hard, then spring back with a little bounce.
        let hard = ((r.fall - 0.1) / 0.6).clamp(0.0, 1.0);
        r.squash.v += (1.5 + 5.0 * hard) * set.landing;
        r.fall = 0.0;
    }
    r.was_airborne = airborne || (p.gliding && !p.on_ground);
    if p.gliding {
        r.fall = r.fall.max(-vy * 0.5);
    }
    r.squash.step(0.0, 260.0, 9.0, dt);
    r.squash.x = r.squash.x.clamp(-0.12, 0.25);

    let w = r.weight.x.clamp(0.0, 1.0);
    Out {
        weight: w,
        lean: r.lean.x,
        bank: r.bank.x,
        squash: r.squash.x,
        air: r.air.x.clamp(0.0, 1.0) * set.air,
        rise: r.rise.x.clamp(0.0, 1.0) * set.air,
        sprint,
        bounce: set.bounce,
        arms: set.arms,
        idle: set.idle,
        steady_head: set.steady_head,
        glide: r.glide.x.clamp(0.0, 1.0),
        swim: r.swim.x.clamp(0.0, 1.0),
        roll: r.roll.x,
        dive: r.dive.x.clamp(0.0, 1.0),
        speed,
        clock: r.clock,
        eat: r.eat.x.clamp(0.0, 1.0) * set.items,
        drink: p.using == Some(Held::Drink),
        eat_left: p.using_left,
        map: r.map.x.clamp(0.0, 1.0) * set.items,
        dial: r.dial.x.clamp(0.0, 1.0) * set.items,
        dial_left: if p.main == Held::Dial { p.main_left } else { !p.main_left },
        light: r.light.x.clamp(0.0, 1.0) * set.items,
        light_left: if p.main == Held::Light { p.main_left } else { !p.main_left },
        climb: r.climb.x.clamp(0.0, 1.0) * set.climb,
        climbed: r.climbed,
        twirl,
        mining: r.mining.x.clamp(0.0, 1.0) * set.actions,
        inspect: if set.actions > 0.0 { r.inspect_t } else { -1.0 },
        actions: set.actions,
        sprinting: sprint,
        glide_now: p.gliding,
        main_left: p.main_left,
    }
}

/// How long an item inspect takes.
pub const INSPECT_SECONDS: f32 = 2.6;

/// The rig as it stands for player `id` (None: off, or not animated).
pub fn current(id: u32) -> Option<Out> {
    out_of(id)
}

/// 0 to 1 to 0 over an inspect: in, hold, out (smoothstep at both ends).
pub fn inspect_blend(t: f32) -> f32 {
    if t < 0.0 {
        return 0.0;
    }
    let k = (t / 0.35).min(1.0).min(((INSPECT_SECONDS - t) / 0.35).max(0.0));
    k * k * (3.0 - 2.0 * k)
}

/// How long a twirl takes.
pub const TWIRL_SECONDS: f32 = 0.85;

thread_local! {
    /// This thread's copy of `OUT` and the generation it was taken at: a
    /// player's body and each armour layer read the rig, so the models of a
    /// frame read a copy instead of taking the lock each time.
    static LOCAL: std::cell::RefCell<(u64, FxHashMap<u32, Out>)> =
        std::cell::RefCell::new((u64::MAX, FxHashMap::default()));
}

fn out_of(id: u32) -> Option<Out> {
    let generation = GENERATION.load(Ordering::Acquire);
    LOCAL.with(|local| {
        let mut local = local.borrow_mut();
        if local.0 != generation {
            let out = OUT.read().unwrap_or_else(|e| e.into_inner());
            match out.as_ref() {
                // `clone_from` keeps the copy's allocation.
                Some(o) => local.1.clone_from(o),
                None => local.1.clear(),
            }
            local.0 = generation;
        }
        local.1.get(&id).copied().filter(|o| o.weight > 0.001)
    })
}

// ---------------------------------------------------------------- the model hooks

thread_local! {
    /// The player `anim::setup` is posing on this thread, for `pose`.
    static POSING: Cell<Option<(u32, State)>> = const { Cell::new(None) };
}

/// At the start of `anim::setup`: who is being posed.
pub fn pose_start(m: &Mob, s: &State) {
    POSING.with(|c| c.set(Some((m.id, *s))));
}

/// After vanilla posed a player (`PlayerModel.setupAnim`): the arms, legs and
/// head on top.
pub fn pose(p: &mut Poses) {
    let Some((id, s)) = POSING.with(|c| c.take()) else { return };
    let Some(o) = out_of(id) else { return };
    apply(p, &s, &o);
}

/// The part angles `o` adds to a pose (radians, `[x, y, z, xRot, yRot, zRot, ...]`).
pub fn apply(p: &mut Poses, s: &State, o: &Out) {
    // How much of each set of moves shows: on foot, gliding, swimming.
    let w = o.weight * (1.0 - o.glide.max(o.swim));
    let gw = o.weight * o.glide;
    let sw = o.weight * o.swim;
    let phase = s.pos * 0.6662;
    let stride = phase.cos();
    let speed = s.speed.clamp(0.0, 1.0);
    let busy_arms = s.swing > 0.0;
    let arm = o.arms * w;
    // Arms: a fuller swing at a sprint, trailing behind a lean, out to the
    // sides and up in the air; a slow breath when standing.
    // Vanilla swings the arms a radian at full speed: a little more, a bit
    // more again at a sprint.
    let swing = stride * (0.12 + 0.12 * o.sprint) * speed * arm;
    let trail = o.lean * 0.6 * arm;
    let spread = (0.35 * o.air + 0.12 * o.rise) * arm;
    let lift = -0.55 * o.rise * arm;
    let breath = ((s.age * 0.07).sin() * 0.035 + 0.02) * (1.0 - speed).max(0.0) * o.idle * w;
    // Gliding: arms out like wings, swept back and tucked in as you dive;
    // the outside arm of a turn reaches further. Legs a little apart, fluttering.
    let fast = (o.speed / 1.2).clamp(0.0, 1.0);
    let wings = (0.6 * (1.0 - o.dive) + 0.08) * gw;
    let sweep = (0.25 + 0.55 * o.dive) * gw;
    let reach = o.roll * 0.3 * gw;
    let flutter = (o.clock * 9.0).sin() * 0.07 * (0.3 + 0.7 * fast) * gw;
    // Swimming: a flutter kick that follows the speed.
    let kick = (o.clock * 10.0).sin() * 0.3 * (o.speed / 0.2).min(1.0) * sw;
    if !busy_arms {
        add(p, "right_arm", -swing + trail + lift + sweep, 0.0, spread + breath + wings - reach);
        add(p, "left_arm", swing + trail + lift * 0.6 + sweep, 0.0, -spread - breath - wings - reach);
    } else {
        add(p, "right_arm", 0.0, 0.0, spread * 0.5);
        add(p, "left_arm", swing + trail + sweep, 0.0, -spread - wings);
    }
    // Legs: vanilla's stride is already wide at a sprint; in the air, one knee up.
    let tuck = 0.45 * o.air * w;
    let apart = 0.04 * o.air * w + 0.1 * gw;
    add(p, "right_leg", -tuck * (0.4 + 0.6 * o.rise) + flutter + kick, 0.0, apart);
    add(p, "left_leg", tuck * 0.35 - flutter - kick, 0.0, -apart);
    // The head: stays level against the body's lean, bank and roll.
    if o.steady_head {
        add(p, "head", -o.lean * 0.85 * w, 0.0, o.bank * 0.7 * w);
    }
    hands(p, s, o);
}

/// The arm on side `left` (the model's own left).
fn arm(left: bool) -> &'static str {
    if left { "left_arm" } else { "right_arm" }
}

/// Eating, drinking, a map, a dial, a light; and climbing. These move an arm
/// to a pose rather than adding to it, blended in by the hand's spring. An
/// attack's swing keeps its arm.
fn hands(p: &mut Poses, s: &State, o: &Out) {
    let w = o.weight;
    let swinging = s.swing > 0.0;
    // Toward the body's middle is -yRot for the right arm, +yRot for the left.
    let inward = |left: bool, v: f32| if left { v } else { -v };
    if o.eat > 0.001 && !swinging {
        let b = o.eat * w;
        // To the mouth; a chew (or a slow tilt for a drink).
        let (x, head) = if o.drink {
            (-1.75 - (s.age * 0.25).sin().abs() * 0.08, -0.3)
        } else {
            (-1.45 + (s.age * 1.7).sin() * 0.09, 0.12)
        };
        blend(p, arm(o.eat_left), x, inward(o.eat_left, 0.55), 0.0, b);
        add(p, "head", head * b, 0.0, 0.0);
    }
    if o.map > 0.001 && !swinging {
        // Both hands forward and in, the head down to read.
        let b = o.map * w;
        blend(p, "right_arm", -0.95, inward(false, 0.38), 0.0, b);
        blend(p, "left_arm", -0.95, inward(true, 0.38), 0.0, b);
        add(p, "head", 0.4 * b, 0.0, 0.0);
    }
    if o.dial > 0.001 && !swinging {
        let b = o.dial * w;
        blend(p, arm(o.dial_left), -0.9, inward(o.dial_left, 0.3), 0.0, b);
        add(p, "head", 0.3 * b, 0.0, 0.0);
    }
    if o.light > 0.001 && !swinging {
        // Held out ahead a little, and steadier than a swinging arm.
        let b = o.light * w * 0.8;
        let side = if o.light_left { -0.12 } else { 0.12 };
        blend(p, arm(o.light_left), -0.45, 0.0, side, b);
    }
    // An attack follows through with the body: the shoulders turn into the
    // swing and back, and the head stays on the target.
    if swinging && o.actions > 0.0 {
        let k = (s.swing * std::f32::consts::PI).sin() * o.actions * w;
        let twist = 0.38 * k * if o.main_left { -1.0 } else { 1.0 };
        add(p, "body", 0.06 * k, twist, 0.0);
        add(p, "head", 0.0, -twist * 0.8, 0.0);
        add(p, "right_arm", -0.25 * k, twist * 0.5, 0.0);
        add(p, "left_arm", 0.2 * k, twist * 0.5, 0.0);
    }
    // Mining: the arm raised higher between strokes, the body leaning in.
    if o.mining > 0.001 {
        let b = o.mining * w;
        let stroke = (s.age * 0.9).sin();
        add(p, arm(o.main_left), (-0.35 + stroke * 0.12) * b, 0.0, 0.0);
        add(p, "body", 0.12 * b, 0.0, 0.0);
        add(p, "head", 0.15 * b, 0.0, 0.0);
    }
    // Inspecting the held item: brought up to the chest and turned over.
    let ib = inspect_blend(o.inspect) * w * o.actions;
    if ib > 0.001 && !swinging {
        let turn = (o.inspect * 2.2).sin();
        let inward = if o.main_left { 0.45 } else { -0.45 };
        blend(p, arm(o.main_left), -1.15 + turn * 0.12, inward + turn * 0.35, 0.1 * turn, ib);
        add(p, "head", 0.35 * ib, -0.15 * ib, 0.0);
    }
    if o.climb > 0.001 {
        // Hand over hand up the ladder; the legs step with them.
        let b = o.climb * w;
        let k = (o.climbed * 2.2).sin();
        blend(p, "right_arm", -2.55 + k * 0.45, 0.0, 0.08, b);
        blend(p, "left_arm", -2.55 - k * 0.45, 0.0, -0.08, b);
        blend(p, "right_leg", -0.25 - k * 0.45, 0.0, 0.0, b);
        blend(p, "left_leg", -0.25 + k * 0.45, 0.0, 0.0, b);
        add(p, "head", -0.35 * b, 0.0, 0.0);
    }
}

/// Moves a part's angles `b` of the way to `x`, `y`, `z`.
fn blend(p: &mut Poses, part: &str, x: f32, y: f32, z: f32, b: f32) {
    if b <= 0.001 {
        return;
    }
    let q = p.get(part);
    q[3] += (x - q[3]) * b;
    q[4] += (y - q[4]) * b;
    q[5] += (z - q[5]) * b;
}

fn add(p: &mut Poses, part: &str, x: f32, y: f32, z: f32) {
    // Nothing to add: leave the pose (and its version) alone.
    if x.abs() + y.abs() + z.abs() < 1e-5 {
        return;
    }
    // A part the model lacks gets the pose map's dummy, which nothing reads.
    let q = p.get(part);
    q[3] += x;
    q[4] += y;
    q[5] += z;
}

/// At the end of `setupRotations` (`species_rotations`): the whole model leans,
/// banks, bounces with the stride and squashes, about the feet.
pub fn body(m: &Mob, alpha: f32, value: glam::Mat4) -> glam::Mat4 {
    if &*m.kind.id != "minecraft:player" {
        return value;
    }
    let Some(o) = out_of(m.id) else { return value };
    value * matrix(&o, super::anim::state(m, alpha).pos, {
        let speed = m.previous_limb_speed + (m.limb_speed - m.previous_limb_speed) * alpha;
        speed.clamp(0.0, 1.0)
    })
}

/// The whole-model transform for `o` at walk position `pos` and limb speed `speed`.
pub fn matrix(o: &Out, pos: f32, speed: f32) -> glam::Mat4 {
    let w = o.weight * (1.0 - o.glide.max(o.swim));
    // Gliding or swimming the model lies along its way (vanilla's turn): roll
    // it about its long axis, which is its Y here. Right side down is +Y.
    let roll = glam::Mat4::from_rotation_y((o.roll + o.twirl) * o.weight * o.glide.max(o.swim));
    // Highest as the legs pass each other, lowest at full stride; level on average.
    let stride = (pos * 0.6662).sin().abs();
    let bob = (stride - 0.64) * 0.11 * speed * o.bounce * w;
    let sq = o.squash * w;
    // Here the model faces -Z with its right at +X: forward is a negative
    // X rotation, toward the right a negative Z rotation.
    roll * glam::Mat4::from_translation(glam::Vec3::new(0.0, bob, 0.0))
        * glam::Mat4::from_rotation_x(-o.lean * w)
        * glam::Mat4::from_rotation_z(-o.bank * w)
        * glam::Mat4::from_scale(glam::Vec3::new(1.0 + sq * 0.5, 1.0 - sq, 1.0 + sq * 0.5))
}
