// Horizon Physics: crumbling blocks, ragdolls, mobs that fall apart and item physics.
// Written from scratch for Horizon. Nothing here touches gameplay: every piece is a client-side
// decoration, drawn with the entity pipeline and simulated with a small rigid-body solver
// (position-based dynamics with substeps) against the blocks of the world.
//
// The hooks of the bundle call into this module:
// - a block breaking hands over its sprite and shape (`spawn_block`);
// - a mob's first frame of dying is drawn into a buffer, cut into its model parts and turned into
//   bodies joined at the joints (`capture_*`, `spawn_mob`);
// - once a frame the world is stepped and drawn (`frame`);
// - dropped items ask for their orientation (`item_rotation`).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};

use crate::mesher::Vertex;

pub const ID: &str = "horizon_physics";
pub const NAME: &str = "Horizon Physics";
pub const VERSION: &str = "0.20.7";
pub const DESCRIPTION: &str =
    "By tariksabanuz. Blocks crumble and blow apart, ragdolls, items, smoke, ropes, capes, liquids, waves and swaying plants.";

// ---------------------------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Settings {
    pub blocks: bool,
    /// 0 off, 1 ragdoll, 2 blocky (the parts come loose), 3 fractured (the parts break up).
    pub deaths: u8,
    pub items: bool,
    /// Blocks blown apart, fireballs, sparks and smoke.
    pub explosions: bool,
    /// Smoke rising from fire, campfires and lava.
    pub smoke: bool,
    /// Footprints in snow.
    pub snow: bool,
    /// Leads and fishing lines that sag and swing.
    pub ropes: bool,
    /// Capes as cloth.
    pub capes: bool,
    /// Water that waves (needs a Horizon shader pack).
    pub waves: bool,
    /// Leaves, plants, vines and seagrass that sway (needs a Horizon shader pack).
    pub plants: bool,
    /// Placed water and lava splash as droplets; splashes when you fall in (beta).
    pub liquids: bool,
    /// 0 calm, 1 breeze, 2 strong.
    pub wind: u8,
    /// 0 Earth, 1 Moon, 2 Mars, 3 Pluto, 4 zero.
    pub gravity: u8,
    /// 0 few, 1 normal, 2 many.
    pub pieces: u8,
    /// 0 10 s, 1 30 s, 2 60 s, 3 5 min.
    pub lifetime: u8,
    /// How a broken block comes apart: 0 blocky (whole), 1 particles, 2 crumbled voxel, 3
    /// fractured.
    pub block_style: u8,
    /// Rain splashes on the ground.
    pub weather: bool,
    /// What a broken block held up falls.
    pub collapse: bool,
    /// The quick setup was done (or skipped).
    pub setup: bool,
    /// Physics objects (bodies) at most at once.
    pub max_objects: u32,
    /// Also the game's own break particles, with the pieces.
    pub mc_break: bool,
    /// How fast the physics runs (1 real time).
    pub sim_speed: f32,
    /// How fast dropped items turn as they tumble.
    pub item_spin: f32,
    /// On a server (multiplayer): no physics, as the server's players see none.
    pub pvp: bool,
    /// The game's particles as small physics pieces: a broken tool, the cracks of mining, the
    /// dust of sprinting, eating, and every other block particle.
    pub item_break: bool,
    pub crack: bool,
    pub sprint: bool,
    pub eat: bool,
    pub block_particles: bool,
    /// How long those pieces stay (tens of seconds) and how much longer some stay (seconds).
    pub particle_life: f32,
    pub particle_var: f32,
    /// How long a dead mob's pieces stay (tens of seconds) and how much longer some stay
    /// (seconds).
    pub mob_life: f32,
    pub mob_var: f32,
    /// How pieces go when their time is up: 0 shrink, 1 sink into the ground, 2 just go.
    pub anim: u8,
    /// How hard a ragdoll's joints hold (0 they tear at a knock), and how much red mist a torn
    /// joint lets out.
    pub joint_force: f32,
    pub joint_blood: f32,
    /// Ragdolls at most at once (the oldest go first).
    pub ragdoll_limit: u32,
    /// How long a broken block's pieces stay (tens of seconds).
    pub block_life: f32,
    /// How much longer some of them stay (seconds).
    pub block_var: f32,
    /// How far from the camera a broken block still falls apart (blocks).
    pub activation: f32,
    /// How big the pieces are (bigger: fewer).
    pub obj_scale: f32,
    /// How far away capes are still cloth (blocks).
    pub cape_range: f32,
    /// No cape while a chest piece is worn.
    pub hide_cape: bool,
    /// How loud the pieces land.
    pub impact_volume: f32,
    /// How loud the wind blows.
    pub wind_volume: f32,
    /// The wind pushes smoke, capes, ropes and plants.
    pub wind_forces: bool,
    /// Rain splashes (and thunder's extra) a frame.
    pub rain_amount: u32,
    pub thunder_amount: u32,
    /// The wind's strength in clear weather, rain and thunder.
    pub wind_clear: f32,
    pub wind_rain: f32,
    pub wind_thunder: f32,
    /// How opaque the rain, snow and dust particles are.
    pub op_rain: f32,
    pub op_snow: f32,
    pub op_dust: f32,
    /// A collapse drops what the blocks drop (in survival).
    pub collapse_drops: bool,
    /// Ticks before what lost its support falls.
    pub collapse_ticks: u32,
    /// Blocks at most that one break can bring down.
    pub collapse_limit: u32,
    /// How high the waves are.
    pub wave_height: f32,
    /// Boats ride the waves.
    pub boats: bool,
    /// Froth on the crests.
    pub foam: bool,
    /// What sways: leaves, vines, water plants (and `plants`: grass, flowers, crops).
    pub sway_leaves: bool,
    pub sway_vines: bool,
    pub sway_water: bool,
    /// How far they sway.
    pub sway_strength: f32,
    /// Grass, flowers and seagrass bend away from you as you walk through them.
    pub plant_push: bool,
    /// How fast the waves roll.
    pub wave_speed: f32,
    /// Ragdolls and pieces are knocked away when you hit them.
    pub punch: bool,
    /// Banners' flags are cloth in the wind, and doors swing open and shut.
    pub banners: bool,
    pub doors: bool,
    /// Snowballs and eggs shatter into pieces where they hit.
    pub projectiles: bool,
    /// How many droplets poured water and lava make.
    pub liquid_amount: f32,
    /// Falling into water splashes.
    pub splashes: bool,
    /// Mobs leave trails in snow too.
    pub mob_trails: bool,
    /// How deep a trail is (of the eighth of a block feet sink).
    pub snow_depth: f32,
    /// Steps kick up powder.
    pub snow_powder: bool,
    /// Fires throw glowing embers.
    pub embers: bool,
    /// How much smoke, and how thick.
    pub smoke_amount: f32,
    pub smoke_opacity: f32,
    /// The game's own smoke is left out where the mod makes its own.
    pub vanilla_smoke: bool,
    /// The despawn animations: what each slot does, and which slots are in the list (the first
    /// seven are the mod's own; the rest are made with Create Animation). `anim` is the one used.
    pub anims: [AnimDef; ANIM_SLOTS],
    pub anim_used: u16,
    /// How fine the physics objects are: 0 normal, 1 high, 2 ultra (more and smaller pieces, more
    /// solver steps).
    pub resolution: u8,
    /// Ragdoll fixes for big (the Wither) and tiny (chickens, bats, bees) mobs.
    pub ragdoll_fix: bool,
    /// An explosion under water throws a spout of water up through the surface.
    pub water_boom: bool,
    /// A fishing line bows downstream in flowing water.
    pub rod_current: bool,
    /// Deep snow (thick layers) is pressed down deeper by feet.
    pub snow_layers: bool,
    /// A collapse raises dust and crumbles the high blocks.
    pub collapse_dust: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            blocks: true,
            deaths: 1,
            items: true,
            explosions: true,
            smoke: true,
            snow: true,
            ropes: true,
            capes: true,
            waves: true,
            plants: true,
            liquids: true,
            wind: 1,
            gravity: 0,
            pieces: 1,
            lifetime: 1,
            block_style: 2,
            weather: true,
            collapse: true,
            setup: false,
            max_objects: 1000,
            mc_break: false,
            sim_speed: 1.0,
            item_spin: 1.0,
            pvp: false,
            item_break: true,
            crack: true,
            sprint: true,
            eat: true,
            block_particles: true,
            particle_life: 0.1,
            particle_var: 3.0,
            mob_life: 4.0,
            mob_var: 3.0,
            anim: 0,
            joint_force: 1.0,
            joint_blood: 1.0,
            ragdoll_limit: 16,
            block_life: 4.0,
            block_var: 3.0,
            activation: 96.0,
            obj_scale: 1.0,
            cape_range: 48.0,
            hide_cape: false,
            impact_volume: 1.0,
            wind_volume: 1.0,
            wind_forces: true,
            rain_amount: 5,
            thunder_amount: 2,
            wind_clear: 0.1,
            wind_rain: 1.0,
            wind_thunder: 1.4,
            op_rain: 1.0,
            op_snow: 1.0,
            op_dust: 1.0,
            collapse_drops: true,
            collapse_ticks: 10,
            collapse_limit: 100,
            wave_height: 1.0,
            boats: true,
            foam: true,
            sway_leaves: true,
            sway_vines: true,
            sway_water: true,
            sway_strength: 1.0,
            plant_push: true,
            wave_speed: 1.0,
            punch: true,
            banners: true,
            doors: true,
            projectiles: true,
            liquid_amount: 1.0,
            splashes: true,
            mob_trails: true,
            snow_depth: 1.0,
            snow_powder: true,
            embers: true,
            smoke_amount: 1.0,
            smoke_opacity: 1.0,
            vanilla_smoke: true,
            anims: std::array::from_fn(anim_default),
            anim_used: (1 << BUILTIN_ANIMS.len()) - 1,
            resolution: 0,
            ragdoll_fix: true,
            water_boom: true,
            rod_current: true,
            snow_layers: true,
            collapse_dust: true,
        }
    }
}

const DEATHS: [&str; 4] = ["Off", "Ragdoll", "Blocky", "Fractured"];
pub const BLOCK_STYLES: [&str; 4] = ["Blocky", "Particles", "Crumbled voxel", "Fractured"];
const GRAVITY: [(&str, f64); 5] =
    [("Earth", 1.0), ("Moon", 0.165), ("Mars", 0.38), ("Pluto", 0.063), ("Zero", 0.0)];
const PIECES: [(&str, usize); 3] = [("Few", 4), ("Normal", 8), ("Many", 16)];
const LIFETIME: [(&str, f64); 4] = [("10 s", 10.0), ("30 s", 30.0), ("60 s", 60.0), ("5 min", 300.0)];
const WIND: [(&str, f64); 3] = [("Calm", 0.0), ("Breeze", 1.0), ("Strong", 2.4)];
/// Earth's pull on the pieces, in blocks per second squared. Real gravity (9.81) looks slow at
/// the scale of blocks; twice that reads as a real fall.
const EARTH: f64 = 20.0;

/// A despawn animation: how the pieces move as their time runs out (`ANIM_MOTIONS`), what they
/// give off when they are gone (`ANIM_PARTICLES`), over how many seconds, and how many puffs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnimDef {
    pub motion: u8,
    pub particle: u8,
    pub duration: f32,
    pub amount: u8,
}

pub const ANIM_SLOTS: usize = 12;
pub const ANIM_MOTIONS: [&str; 5] = ["shrink", "sink", "rise", "bounce", "none"];
pub const ANIM_PARTICLES: [&str; 6] = ["none", "portal", "splash", "smoke", "poof", "explosion"];
/// The mod's own animations (the first slots).
pub const BUILTIN_ANIMS: [(&str, AnimDef); 7] = [
    ("default", AnimDef { motion: 0, particle: 0, duration: 0.5, amount: 0 }),
    ("portal", AnimDef { motion: 0, particle: 1, duration: 0.8, amount: 10 }),
    ("splash", AnimDef { motion: 1, particle: 2, duration: 1.0, amount: 8 }),
    ("vanish", AnimDef { motion: 0, particle: 0, duration: 0.12, amount: 0 }),
    ("explosion", AnimDef { motion: 0, particle: 5, duration: 0.2, amount: 6 }),
    ("poof", AnimDef { motion: 4, particle: 4, duration: 0.05, amount: 6 }),
    ("bounce", AnimDef { motion: 3, particle: 0, duration: 1.2, amount: 0 }),
];

/// What slot `i` holds when the mod comes (a made one starts as a copy of `default`).
pub fn anim_default(i: usize) -> AnimDef {
    BUILTIN_ANIMS.get(i).map_or(BUILTIN_ANIMS[0].1, |b| b.1)
}

/// Slot `i`'s name in the list.
pub fn anim_name(s: &Settings, i: usize) -> String {
    let _ = s;
    match BUILTIN_ANIMS.get(i) {
        Some(b) => b.0.to_string(),
        None => format!("custom {}", i + 1 - BUILTIN_ANIMS.len()),
    }
}

/// The slots in the list, in order.
pub fn anim_slots(s: &Settings) -> Vec<usize> {
    (0..ANIM_SLOTS).filter(|i| s.anim_used & (1 << i) != 0).collect()
}

/// The slot after `cur` in the list (round to the first).
pub fn next_anim(s: &Settings, cur: u8) -> u8 {
    let list = anim_slots(s);
    list.iter().copied().find(|&i| i > cur as usize).or(list.first().copied()).unwrap_or(0) as u8
}

/// The animation used now (the default one when its slot was removed).
pub fn anim_now(s: &Settings) -> AnimDef {
    let i = s.anim as usize % ANIM_SLOTS;
    if s.anim_used & (1 << i) != 0 { s.anims[i] } else { BUILTIN_ANIMS[0].1 }
}

static ANIM_EDIT: AtomicUsize = AtomicUsize::new(0);

/// The slot the animation editor shows.
pub fn anim_edit() -> usize {
    ANIM_EDIT.load(Ordering::Relaxed) % ANIM_SLOTS
}

pub fn set_anim_edit(i: usize) {
    ANIM_EDIT.store(i % ANIM_SLOTS, Ordering::Relaxed);
}

/// The weather now: rain and thunder, 0 to 1 (the world hook sets them each frame).
static RAIN: AtomicU32 = AtomicU32::new(0);
static THUNDER: AtomicU32 = AtomicU32::new(0);

pub fn set_weather(rain: f32, thunder: f32) {
    RAIN.store(rain.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
    THUNDER.store(thunder.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
}

pub fn weather_levels() -> (f32, f32) {
    (f32::from_bits(RAIN.load(Ordering::Relaxed)), f32::from_bits(THUNDER.load(Ordering::Relaxed)))
}

fn on_upper(b: bool) -> &'static str {
    if b { "ON" } else { "OFF" }
}

fn decimal(v: f32) -> String {
    format!("{v:.2}").replace('.', ",")
}

fn on_off(b: bool) -> &'static str {
    if b { "On" } else { "Off" }
}

impl Settings {
    pub fn labels(&self) -> Vec<String> {
        vec![
            format!("Crumbling blocks: {}", on_off(self.blocks)),
            format!("Mob deaths: {}", DEATHS[self.deaths as usize % 4]),
            format!("Item physics: {}", on_off(self.items)),
            format!("Explosions: {}", on_off(self.explosions)),
            format!("Smoke and fire: {}", on_off(self.smoke)),
            format!("Snow footprints: {}", on_off(self.snow)),
            format!("Leads and fishing lines: {}", on_off(self.ropes)),
            format!("Cloth capes: {}", on_off(self.capes)),
            format!("Water waves (shader pack): {}", on_off(self.waves)),
            format!("Swaying plants (shader pack): {}", on_off(self.plants)),
            format!("Liquids (beta): {}", on_off(self.liquids)),
            format!("Wind: {}", WIND[self.wind as usize % 3].0),
            format!("Gravity: {}", GRAVITY[self.gravity as usize % 5].0),
            format!("Pieces per block: {}", PIECES[self.pieces as usize % 3].0),
            format!("Pieces stay for: {}", LIFETIME[self.lifetime as usize % 4].0),
            "Clear all pieces".to_string(),
            format!("Block pieces: {}", BLOCK_STYLES[self.block_style as usize % 4]),
            format!("Collapsing structures: {}", on_off(self.collapse)),
            format!("Weather particles: {}", on_off(self.weather)),
            "Run the quick setup again".to_string(),
        ]
    }

    /// Button `i` was clicked. Returns true when the pieces should be cleared.
    pub fn click(&mut self, i: usize) -> bool {
        match i {
            0 => self.blocks = !self.blocks,
            1 => self.deaths = (self.deaths + 1) % 4,
            2 => self.items = !self.items,
            3 => self.explosions = !self.explosions,
            4 => self.smoke = !self.smoke,
            5 => self.snow = !self.snow,
            6 => self.ropes = !self.ropes,
            7 => self.capes = !self.capes,
            8 => self.waves = !self.waves,
            9 => self.plants = !self.plants,
            10 => self.liquids = !self.liquids,
            11 => self.wind = (self.wind + 1) % 3,
            12 => self.gravity = (self.gravity + 1) % 5,
            13 => self.pieces = (self.pieces + 1) % 3,
            14 => self.lifetime = (self.lifetime + 1) % 4,
            15 => return true,
            16 => self.block_style = (self.block_style + 1) % 4,
            17 => self.collapse = !self.collapse,
            18 => self.weather = !self.weather,
            19 => self.setup = false,
            _ => {}
        }
        false
    }

    pub fn to_text(&self) -> String {
        let mut t = self.base_text();
        t.push_str(&format!(
            "max_objects={}\nmc_break={}\nsim_speed={}\nitem_spin={}\npvp={}\nitem_break={}\ncrack={}\nsprint={}\neat={}\nblock_particles={}\nparticle_life={}\nparticle_var={}\nmob_life={}\nmob_var={}\nanim={}\njoint_force={}\njoint_blood={}\nragdoll_limit={}\n",
            self.max_objects,
            self.mc_break as u8,
            self.sim_speed,
            self.item_spin,
            self.pvp as u8,
            self.item_break as u8,
            self.crack as u8,
            self.sprint as u8,
            self.eat as u8,
            self.block_particles as u8,
            self.particle_life,
            self.particle_var,
            self.mob_life,
            self.mob_var,
            self.anim,
            self.joint_force,
            self.joint_blood,
            self.ragdoll_limit
        ));
        t.push_str(&format!("block_life={}\n", self.block_life));
        t.push_str(&format!("block_var={}\n", self.block_var));
        t.push_str(&format!("activation={}\n", self.activation));
        t.push_str(&format!("obj_scale={}\n", self.obj_scale));
        t.push_str(&format!("cape_range={}\n", self.cape_range));
        t.push_str(&format!("hide_cape={}\n", self.hide_cape as u8));
        t.push_str(&format!("impact_volume={}\n", self.impact_volume));
        t.push_str(&format!("wind_volume={}\n", self.wind_volume));
        t.push_str(&format!("wind_forces={}\n", self.wind_forces as u8));
        t.push_str(&format!("rain_amount={}\n", self.rain_amount));
        t.push_str(&format!("thunder_amount={}\n", self.thunder_amount));
        t.push_str(&format!("wind_clear={}\n", self.wind_clear));
        t.push_str(&format!("wind_rain={}\n", self.wind_rain));
        t.push_str(&format!("wind_thunder={}\n", self.wind_thunder));
        t.push_str(&format!("op_rain={}\n", self.op_rain));
        t.push_str(&format!("op_snow={}\n", self.op_snow));
        t.push_str(&format!("op_dust={}\n", self.op_dust));
        t.push_str(&format!("collapse_drops={}\n", self.collapse_drops as u8));
        t.push_str(&format!("collapse_ticks={}\n", self.collapse_ticks));
        t.push_str(&format!("collapse_limit={}\n", self.collapse_limit));
        t.push_str(&format!("wave_height={}\n", self.wave_height));
        t.push_str(&format!("boats={}\n", self.boats as u8));
        t.push_str(&format!("foam={}\n", self.foam as u8));
        t.push_str(&format!("sway_leaves={}\n", self.sway_leaves as u8));
        t.push_str(&format!("sway_vines={}\n", self.sway_vines as u8));
        t.push_str(&format!("sway_water={}\n", self.sway_water as u8));
        t.push_str(&format!("sway_strength={}\n", self.sway_strength));
        t.push_str(&format!("plant_push={}\n", self.plant_push as u8));
        t.push_str(&format!("wave_speed={}\n", self.wave_speed));
        t.push_str(&format!("punch={}\n", self.punch as u8));
        t.push_str(&format!("banners={}\ndoors={}\n", self.banners as u8, self.doors as u8));
        t.push_str(&format!("projectiles={}\n", self.projectiles as u8));
        t.push_str(&format!("liquid_amount={}\n", self.liquid_amount));
        t.push_str(&format!("splashes={}\n", self.splashes as u8));
        t.push_str(&format!("mob_trails={}\n", self.mob_trails as u8));
        t.push_str(&format!("snow_depth={}\n", self.snow_depth));
        t.push_str(&format!("snow_powder={}\n", self.snow_powder as u8));
        t.push_str(&format!("embers={}\n", self.embers as u8));
        t.push_str(&format!("smoke_amount={}\n", self.smoke_amount));
        t.push_str(&format!("smoke_opacity={}\n", self.smoke_opacity));
        t.push_str(&format!("vanilla_smoke={}\n", self.vanilla_smoke as u8));
        t.push_str(&format!("anim_used={}\n", self.anim_used));
        t.push_str(&format!(
            "resolution={}\nragdoll_fix={}\nwater_boom={}\nrod_current={}\nsnow_layers={}\ncollapse_dust={}\n",
            self.resolution, self.ragdoll_fix as u8, self.water_boom as u8, self.rod_current as u8, self.snow_layers as u8, self.collapse_dust as u8
        ));
        for (i, a) in self.anims.iter().enumerate() {
            t.push_str(&format!("anim{i}={},{},{},{}\n", a.motion, a.particle, a.duration, a.amount));
        }
        t
    }

    fn base_text(&self) -> String {
        format!(
            "blocks={}\ndeaths={}\nitems={}\nexplosions={}\nsmoke={}\nsnow={}\nropes={}\ncapes={}\nwaves={}\nplants={}\nliquids={}\nwind={}\ngravity={}\npieces={}\nlifetime={}\nblock_style={}\nweather={}\ncollapse={}\nsetup_done={}\n",
            self.blocks as u8,
            self.deaths,
            self.items as u8,
            self.explosions as u8,
            self.smoke as u8,
            self.snow as u8,
            self.ropes as u8,
            self.capes as u8,
            self.waves as u8,
            self.plants as u8,
            self.liquids as u8,
            self.wind,
            self.gravity,
            self.pieces,
            self.lifetime,
            self.block_style,
            self.weather as u8,
            self.collapse as u8,
            self.setup as u8
        )
    }

    pub fn from_text(text: &str) -> Self {
        let mut s = Self::default();
        for line in text.lines() {
            let Some((k, v)) = line.split_once('=') else { continue };
            if let Some(i) = k.trim().strip_prefix("anim").and_then(|i| i.parse::<usize>().ok()) {
                let n: Vec<f32> = v.split(',').filter_map(|x| x.trim().parse().ok()).collect();
                if i < ANIM_SLOTS && n.len() == 4 {
                    s.anims[i] = AnimDef {
                        motion: (n[0] as u8) % ANIM_MOTIONS.len() as u8,
                        particle: (n[1] as u8) % ANIM_PARTICLES.len() as u8,
                        duration: n[2].clamp(0.05, 3.0),
                        amount: n[3].clamp(0.0, 32.0) as u8,
                    };
                }
                continue;
            }
            if let Ok(f) = v.trim().parse::<f32>() {
                let on = f != 0.0;
                match k.trim() {
                    "max_objects" => s.max_objects = (f as u32).clamp(100, 10000),
                    "mc_break" => s.mc_break = on,
                    "sim_speed" => s.sim_speed = f.clamp(0.1, 2.0),
                    "item_spin" => s.item_spin = f.clamp(0.0, 3.0),
                    "pvp" => s.pvp = on,
                    "item_break" => s.item_break = on,
                    "crack" => s.crack = on,
                    "sprint" => s.sprint = on,
                    "eat" => s.eat = on,
                    "block_particles" => s.block_particles = on,
                    "particle_life" => s.particle_life = f.clamp(0.0, 1.0),
                    "particle_var" => s.particle_var = f.clamp(0.0, 10.0),
                    "mob_life" => s.mob_life = f.clamp(0.0, 30.0),
                    "mob_var" => s.mob_var = f.clamp(0.0, 10.0),
                    "anim" => s.anim = (f as u8) % ANIM_SLOTS as u8,
                    "joint_force" => s.joint_force = f.clamp(0.0, 10.0),
                    "joint_blood" => s.joint_blood = f.clamp(0.0, 5.0),
                    "ragdoll_limit" => s.ragdoll_limit = (f as u32).clamp(1, 64),
                    "block_life" => s.block_life = f.clamp(0.0, 30.0),
                    "block_var" => s.block_var = f.clamp(0.0, 10.0),
                    "activation" => s.activation = f.clamp(8.0, 256.0),
                    "obj_scale" => s.obj_scale = f.clamp(0.25, 3.0),
                    "cape_range" => s.cape_range = f.clamp(4.0, 128.0),
                    "hide_cape" => s.hide_cape = on,
                    "impact_volume" => s.impact_volume = f.clamp(0.0, 2.0),
                    "wind_volume" => s.wind_volume = f.clamp(0.0, 2.0),
                    "wind_forces" => s.wind_forces = on,
                    "rain_amount" => s.rain_amount = (f as u32).clamp(0, 20),
                    "thunder_amount" => s.thunder_amount = (f as u32).clamp(0, 20),
                    "wind_clear" => s.wind_clear = f.clamp(0.0, 3.0),
                    "wind_rain" => s.wind_rain = f.clamp(0.0, 3.0),
                    "wind_thunder" => s.wind_thunder = f.clamp(0.0, 3.0),
                    "op_rain" => s.op_rain = f.clamp(0.0, 1.0),
                    "op_snow" => s.op_snow = f.clamp(0.0, 1.0),
                    "op_dust" => s.op_dust = f.clamp(0.0, 1.0),
                    "collapse_drops" => s.collapse_drops = on,
                    "collapse_ticks" => s.collapse_ticks = (f as u32).clamp(0, 100),
                    "collapse_limit" => s.collapse_limit = (f as u32).clamp(10, 1000),
                    "wave_height" => s.wave_height = f.clamp(0.0, 6.0),
                    "boats" => s.boats = on,
                    "foam" => s.foam = on,
                    "sway_leaves" => s.sway_leaves = on,
                    "sway_vines" => s.sway_vines = on,
                    "sway_water" => s.sway_water = on,
                    "sway_strength" => s.sway_strength = f.clamp(0.0, 3.0),
                    "plant_push" => s.plant_push = on,
                    "wave_speed" => s.wave_speed = f.clamp(0.25, 6.0),
                    "punch" => s.punch = on,
                    "banners" => s.banners = on,
                    "doors" => s.doors = on,
                    "projectiles" => s.projectiles = on,
                    "liquid_amount" => s.liquid_amount = f.clamp(0.0, 3.0),
                    "splashes" => s.splashes = on,
                    "mob_trails" => s.mob_trails = on,
                    "snow_depth" => s.snow_depth = f.clamp(0.1, 1.0),
                    "snow_powder" => s.snow_powder = on,
                    "embers" => s.embers = on,
                    "smoke_amount" => s.smoke_amount = f.clamp(0.0, 3.0),
                    "smoke_opacity" => s.smoke_opacity = f.clamp(0.1, 2.0),
                    "vanilla_smoke" => s.vanilla_smoke = on,
                    "resolution" => s.resolution = (f as u8).min(2),
                    "ragdoll_fix" => s.ragdoll_fix = on,
                    "water_boom" => s.water_boom = on,
                    "rod_current" => s.rod_current = on,
                    "snow_layers" => s.snow_layers = on,
                    "collapse_dust" => s.collapse_dust = on,
                    "anim_used" => s.anim_used = (f as u16) & ((1 << ANIM_SLOTS) - 1),
                    _ => {}
                }
            }
            let Ok(n) = v.trim().parse::<u8>() else { continue };
            match k.trim() {
                "blocks" => s.blocks = n != 0,
                "deaths" => s.deaths = n % 4,
                "items" => s.items = n != 0,
                "explosions" => s.explosions = n != 0,
                "smoke" => s.smoke = n != 0,
                "snow" => s.snow = n != 0,
                "ropes" => s.ropes = n != 0,
                "capes" => s.capes = n != 0,
                "waves" => s.waves = n != 0,
                "plants" => s.plants = n != 0,
                "liquids" => s.liquids = n != 0,
                "wind" => s.wind = n % 3,
                "gravity" => s.gravity = n % 5,
                "pieces" => s.pieces = n % 3,
                "lifetime" => s.lifetime = n % 4,
                "block_style" => s.block_style = n % 4,
                "weather" => s.weather = n != 0,
                "collapse" => s.collapse = n != 0,
                "setup_done" => s.setup = n != 0,
                _ => {}
            }
        }
        s
    }

    pub fn gravity(&self) -> f64 {
        EARTH * GRAVITY[self.gravity as usize % 5].1
    }

    /// The wind's strength now: the Weather page's strength for clear weather, rain and
    /// thunder, by how much it rains and thunders (none with Wind Forces off).
    pub fn wind(&self) -> f64 {
        if !self.wind_forces {
            return 0.0;
        }
        let (rain, thunder) = weather_levels();
        let w = self.wind_clear + (self.wind_rain - self.wind_clear) * rain + (self.wind_thunder - self.wind_rain) * thunder;
        f64::from(w.max(0.0))
    }

    /// Pieces a broken block makes: bigger pieces, fewer of them.
    pub fn pieces_per_block(&self) -> usize {
        let _ = PIECES;
        ((8.0 * self.resolution_factor() / f64::from(self.obj_scale.max(0.1)).powf(1.5)).round() as usize).clamp(1, 96)
    }

    /// How many times more pieces the resolution makes (1 normal, 1,6 high, 2,5 ultra).
    pub fn resolution_factor(&self) -> f64 {
        [1.0, 1.6, 2.5][self.resolution as usize % 3]
    }

    /// Solver steps a frame at this resolution (finer for small, fast pieces).
    pub fn substeps(&self) -> usize {
        [SUBSTEPS, 6, 8][self.resolution as usize % 3]
    }

    /// How long a broken block's pieces stay at least (seconds); some stay up to `block_var`
    /// seconds longer.
    pub fn lifetime(&self) -> f64 {
        let _ = LIFETIME;
        (f64::from(self.block_life) * 10.0).max(0.5)
    }
}

static SETTINGS: Mutex<Option<(Settings, Option<PathBuf>)>> = Mutex::new(None);

/// Reads the settings file once (the folder comes from the game's native config dir).
pub fn load(dir: Option<PathBuf>) {
    let mut g = SETTINGS.lock().unwrap_or_else(|e| e.into_inner());
    if g.is_some() {
        return;
    }
    let path = dir.map(|d| d.join("horizon_physics.txt"));
    let s = path
        .as_ref()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .map(|t| Settings::from_text(&t))
        .unwrap_or_default();
    *g = Some((s, path));
}

static BUILTIN: AtomicBool = AtomicBool::new(false);

/// Whether the game runs the mod's own vanilla-look pack (no pack picked, waves or sway on).
pub fn builtin_running() -> bool {
    BUILTIN.load(Ordering::Relaxed)
}

pub fn set_builtin_running(on: bool) {
    BUILTIN.store(on, Ordering::Relaxed);
}

/// Where the mod's own pack is written: beside its settings.
pub fn builtin_pack_dir(config: Option<PathBuf>) -> Option<PathBuf> {
    config.map(|d| d.join("horizon_physics_pack"))
}

pub fn loaded() -> bool {
    SETTINGS.lock().map(|g| g.is_some()).unwrap_or(true)
}

/// The settings the mod runs with. Until the quick setup is done (or skipped) everything is off:
/// the game starts as it would without the mod (no physics, no waves, no shader pack of the
/// mod's own), and finishing the quick setup turns on what it chose.
pub fn settings() -> Settings {
    let s = SETTINGS.lock().ok().and_then(|g| g.as_ref().map(|s| s.0));
    match s {
        Some(s) if s.setup => s,
        Some(s) => s.all_off(),
        None => Settings::default().all_off(),
    }
}

impl Settings {
    /// These settings with every effect off (the numbers kept for when they come on).
    pub fn all_off(mut self) -> Self {
        self.blocks = false;
        self.deaths = 0;
        self.items = false;
        self.explosions = false;
        self.smoke = false;
        self.snow = false;
        self.ropes = false;
        self.capes = false;
        self.waves = false;
        self.plants = false;
        self.liquids = false;
        self.weather = false;
        self.collapse = false;
        self.mc_break = false;
        self.item_break = false;
        self.crack = false;
        self.sprint = false;
        self.eat = false;
        self.block_particles = false;
        self.boats = false;
        self.foam = false;
        self.sway_leaves = false;
        self.sway_vines = false;
        self.sway_water = false;
        self.plant_push = false;
        self.punch = false;
        self.banners = false;
        self.doors = false;
        self.projectiles = false;
        self.splashes = false;
        self.mob_trails = false;
        self.snow_powder = false;
        self.embers = false;
        self.wind_forces = false;
        self.water_boom = false;
        self.rod_current = false;
        self.snow_layers = false;
        self.collapse_dust = false;
        self
    }
}

pub fn setting_labels() -> Vec<String> {
    settings().labels()
}

pub fn setting_click(i: usize) {
    let clear_all = {
        let mut g = SETTINGS.lock().unwrap_or_else(|e| e.into_inner());
        let entry = g.get_or_insert_with(|| (Settings::default(), None));
        let clear_all = entry.0.click(i);
        if let Some(p) = &entry.1 {
            if let Some(parent) = p.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::write(p, entry.0.to_text());
        }
        clear_all
    };
    if clear_all {
        clear();
    }
}

/// Changes the settings and saves them (the quick setup's choices).
pub fn update_settings(f: impl FnOnce(&mut Settings)) {
    let mut g = SETTINGS.lock().unwrap_or_else(|e| e.into_inner());
    let entry = g.get_or_insert_with(|| (Settings::default(), None));
    f(&mut entry.0);
    if let Some(p) = &entry.1 {
        if let Some(parent) = p.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(p, entry.0.to_text());
    }
}

// ---------------------------------------------------------------------------------------------
// Math
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct V3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

pub const fn v3(x: f64, y: f64, z: f64) -> V3 {
    V3 { x, y, z }
}

impl std::ops::Add for V3 {
    type Output = V3;
    fn add(self, o: V3) -> V3 {
        v3(self.x + o.x, self.y + o.y, self.z + o.z)
    }
}
impl std::ops::Sub for V3 {
    type Output = V3;
    fn sub(self, o: V3) -> V3 {
        v3(self.x - o.x, self.y - o.y, self.z - o.z)
    }
}
impl std::ops::Mul<f64> for V3 {
    type Output = V3;
    fn mul(self, k: f64) -> V3 {
        v3(self.x * k, self.y * k, self.z * k)
    }
}
impl std::ops::Neg for V3 {
    type Output = V3;
    fn neg(self) -> V3 {
        v3(-self.x, -self.y, -self.z)
    }
}

impl V3 {
    pub fn dot(self, o: V3) -> f64 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }
    pub fn cross(self, o: V3) -> V3 {
        v3(self.y * o.z - self.z * o.y, self.z * o.x - self.x * o.z, self.x * o.y - self.y * o.x)
    }
    pub fn len(self) -> f64 {
        self.dot(self).sqrt()
    }
    pub fn norm(self) -> V3 {
        let l = self.len();
        if l > 1e-12 { self * (1.0 / l) } else { V3::default() }
    }
    pub fn mul_each(self, o: V3) -> V3 {
        v3(self.x * o.x, self.y * o.y, self.z * o.z)
    }
    pub fn get(self, k: usize) -> f64 {
        match k {
            0 => self.x,
            1 => self.y,
            _ => self.z,
        }
    }
    pub fn set(&mut self, k: usize, v: f64) {
        match k {
            0 => self.x = v,
            1 => self.y = v,
            _ => self.z = v,
        }
    }
    pub fn axis(k: usize, s: f64) -> V3 {
        let mut a = V3::default();
        a.set(k, s);
        a
    }
    fn from_f32(a: [f32; 3]) -> V3 {
        v3(f64::from(a[0]), f64::from(a[1]), f64::from(a[2]))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quat {
    pub w: f64,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Default for Quat {
    fn default() -> Self {
        Quat::IDENTITY
    }
}

impl Quat {
    pub const IDENTITY: Quat = Quat { w: 1.0, x: 0.0, y: 0.0, z: 0.0 };

    pub fn mul(self, o: Quat) -> Quat {
        Quat {
            w: self.w * o.w - self.x * o.x - self.y * o.y - self.z * o.z,
            x: self.w * o.x + self.x * o.w + self.y * o.z - self.z * o.y,
            y: self.w * o.y - self.x * o.z + self.y * o.w + self.z * o.x,
            z: self.w * o.z + self.x * o.y - self.y * o.x + self.z * o.w,
        }
    }
    pub fn conj(self) -> Quat {
        Quat { w: self.w, x: -self.x, y: -self.y, z: -self.z }
    }
    pub fn normalize(self) -> Quat {
        let l = (self.w * self.w + self.x * self.x + self.y * self.y + self.z * self.z).sqrt();
        if l < 1e-12 {
            return Quat::IDENTITY;
        }
        Quat { w: self.w / l, x: self.x / l, y: self.y / l, z: self.z / l }
    }
    pub fn rotate(self, v: V3) -> V3 {
        let u = v3(self.x, self.y, self.z);
        let t = u.cross(v) * 2.0;
        v + t * self.w + u.cross(t)
    }
    pub fn axis_angle(axis: V3, angle: f64) -> Quat {
        let a = axis.norm();
        let (s, c) = (angle * 0.5).sin_cos();
        Quat { w: c, x: a.x * s, y: a.y * s, z: a.z * s }
    }
    /// The rotation whose matrix has these orthonormal columns.
    pub fn from_axes(a: V3, b: V3, c: V3) -> Quat {
        let (m00, m11, m22) = (a.x, b.y, c.z);
        let tr = m00 + m11 + m22;
        let q = if tr > 0.0 {
            let s = (tr + 1.0).sqrt() * 2.0;
            Quat { w: 0.25 * s, x: (b.z - c.y) / s, y: (c.x - a.z) / s, z: (a.y - b.x) / s }
        } else if m00 > m11 && m00 > m22 {
            let s = (1.0 + m00 - m11 - m22).sqrt() * 2.0;
            Quat { w: (b.z - c.y) / s, x: 0.25 * s, y: (b.x + a.y) / s, z: (c.x + a.z) / s }
        } else if m11 > m22 {
            let s = (1.0 + m11 - m00 - m22).sqrt() * 2.0;
            Quat { w: (c.x - a.z) / s, x: (b.x + a.y) / s, y: 0.25 * s, z: (c.y + b.z) / s }
        } else {
            let s = (1.0 + m22 - m00 - m11).sqrt() * 2.0;
            Quat { w: (a.y - b.x) / s, x: (c.x + a.z) / s, y: (c.y + b.z) / s, z: 0.25 * s }
        };
        q.normalize()
    }
    /// The shortest rotation taking unit vector `from` to unit vector `to`.
    pub fn between(from: V3, to: V3) -> Quat {
        let d = from.dot(to).clamp(-1.0, 1.0);
        if d > 0.999_999 {
            return Quat::IDENTITY;
        }
        if d < -0.999_999 {
            let mut axis = v3(1.0, 0.0, 0.0).cross(from);
            if axis.len() < 1e-6 {
                axis = v3(0.0, 0.0, 1.0).cross(from);
            }
            return Quat::axis_angle(axis, std::f64::consts::PI);
        }
        let c = from.cross(to);
        Quat { w: 1.0 + d, x: c.x, y: c.y, z: c.z }.normalize()
    }
    pub fn slerp(self, mut o: Quat, t: f64) -> Quat {
        let mut d = self.w * o.w + self.x * o.x + self.y * o.y + self.z * o.z;
        if d < 0.0 {
            o = Quat { w: -o.w, x: -o.x, y: -o.y, z: -o.z };
            d = -d;
        }
        if d > 0.9995 {
            return Quat {
                w: self.w + (o.w - self.w) * t,
                x: self.x + (o.x - self.x) * t,
                y: self.y + (o.y - self.y) * t,
                z: self.z + (o.z - self.z) * t,
            }
            .normalize();
        }
        let th = d.acos();
        let s = th.sin();
        let (a, b) = (((1.0 - t) * th).sin() / s, (t * th).sin() / s);
        Quat {
            w: self.w * a + o.w * b,
            x: self.x * a + o.x * b,
            y: self.y * a + o.y * b,
            z: self.z * a + o.z * b,
        }
    }
    /// Turned by angular velocity `w` for `h` seconds.
    fn integrate(self, w: V3, h: f64) -> Quat {
        let dq = Quat { w: 0.0, x: w.x, y: w.y, z: w.z }.mul(self);
        Quat {
            w: self.w + 0.5 * h * dq.w,
            x: self.x + 0.5 * h * dq.x,
            y: self.y + 0.5 * h * dq.y,
            z: self.z + 0.5 * h * dq.z,
        }
        .normalize()
    }
}

/// A small, fast random source (xorshift64*); the look needs no more.
#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }
    pub fn next(&mut self) -> f64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        (self.0.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 11) as f64 / (1u64 << 53) as f64
    }
    pub fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.next()
    }
    pub fn unit(&mut self) -> V3 {
        v3(self.range(-1.0, 1.0), self.range(-1.0, 1.0), self.range(-1.0, 1.0)).norm()
    }
}

// ---------------------------------------------------------------------------------------------
// The world, as the solver sees it
// ---------------------------------------------------------------------------------------------

/// What fills one block cell.
#[derive(Clone, Debug)]
pub enum Cell {
    Air,
    /// Water or lava: the pieces sink slowly.
    Fluid,
    Full,
    /// Partial collision boxes in block space (`[x0, y0, z0, x1, y1, z1]`), as slabs and stairs.
    Boxes(std::sync::Arc<[[f64; 6]]>),
}

pub trait World {
    fn cell(&self, x: i32, y: i32, z: i32) -> Cell;
    /// Packed light as entity vertices take it: sky * 16 in bits 0..8, block * 16 in 8..16.
    fn light(&self, x: i32, y: i32, z: i32) -> u32;
}

/// A world made of two functions, for the hooks (which can hold only a function each).
pub struct FnWorld<C, L>(pub C, pub L);

impl<C, L> World for FnWorld<C, L>
where
    C: Fn(i32, i32, i32) -> Cell,
    L: Fn(i32, i32, i32) -> u32,
{
    fn cell(&self, x: i32, y: i32, z: i32) -> Cell {
        (self.0)(x, y, z)
    }
    fn light(&self, x: i32, y: i32, z: i32) -> u32 {
        (self.1)(x, y, z)
    }
}

fn floor(v: f64) -> i32 {
    v.floor() as i32
}

/// Where point `p` (which was at `prev` a moment before) is inside a block: the way out and how
/// deep it is.
fn penetration(world: &dyn World, p: V3, prev: V3, ignore: Option<[i32; 3]>) -> Option<(V3, f64)> {
    let cell = [floor(p.x), floor(p.y), floor(p.z)];
    if ignore == Some(cell) {
        return None;
    }
    let o = v3(f64::from(cell[0]), f64::from(cell[1]), f64::from(cell[2]));
    let full: [[f64; 6]; 1] = [[0.0, 0.0, 0.0, 1.0, 1.0, 1.0]];
    let c = world.cell(cell[0], cell[1], cell[2]);
    let boxes: &[[f64; 6]] = match &c {
        Cell::Air | Cell::Fluid => return None,
        Cell::Full => &full,
        Cell::Boxes(b) => b,
    };
    let is_full = matches!(c, Cell::Full);
    for b in boxes {
        let lo = o + v3(b[0], b[1], b[2]);
        let hi = o + v3(b[3], b[4], b[5]);
        let inside = (0..3).all(|k| p.get(k) >= lo.get(k) && p.get(k) < hi.get(k));
        if !inside {
            continue;
        }
        // The face the point came in through: of the faces it crossed, the last one crossed.
        let mut best: Option<(V3, f64, f64)> = None;
        for k in 0..3 {
            let (a, b0, c0) = (prev.get(k), lo.get(k), hi.get(k));
            let d = p.get(k) - a;
            if a <= b0 && d.abs() > 1e-12 {
                let t = (b0 - a) / d;
                if best.is_none_or(|x| t > x.2) {
                    best = Some((V3::axis(k, -1.0), p.get(k) - b0, t));
                }
            }
            if a >= c0 && d.abs() > 1e-12 {
                let t = (c0 - a) / d;
                if best.is_none_or(|x| t > x.2) {
                    best = Some((V3::axis(k, 1.0), c0 - p.get(k), t));
                }
            }
        }
        if let Some((n, depth, _)) = best {
            return Some((n, depth + 1e-4));
        }
        // Already inside before: the shallowest way out that does not lead into another block.
        let mut out: Option<(V3, f64)> = None;
        for k in 0..3 {
            for s in [-1.0, 1.0] {
                let depth = if s > 0.0 { hi.get(k) - p.get(k) } else { p.get(k) - lo.get(k) };
                if is_full {
                    let mut nb = cell;
                    nb[k] += s as i32;
                    if matches!(world.cell(nb[0], nb[1], nb[2]), Cell::Full) {
                        continue;
                    }
                }
                if out.is_none_or(|x| depth < x.1) {
                    out = Some((V3::axis(k, s), depth));
                }
            }
        }
        let (n, depth) = out.unwrap_or((v3(0.0, 1.0, 0.0), hi.y - p.y));
        return Some((n, depth + 1e-4));
    }
    None
}

// ---------------------------------------------------------------------------------------------
// Bodies
// ---------------------------------------------------------------------------------------------

/// One vertex of a piece's mesh, in the piece's own frame.
#[derive(Clone, Copy, Debug)]
pub struct MeshVertex {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
    pub normal_w: i8,
    pub uv: [f32; 2],
    pub layer: u32,
    pub color: u32,
    pub glint_uv: [f32; 2],
}

#[derive(Clone, Copy, Debug)]
struct Contact {
    /// The touching corner, in the body's frame.
    local: V3,
    n: V3,
    lambda: f64,
    vn_before: f64,
}

#[derive(Clone, Debug)]
pub struct Body {
    pub x: V3,
    pub q: Quat,
    pub v: V3,
    pub w: V3,
    px: V3,
    pq: Quat,
    pub half: V3,
    inv_mass: f64,
    inv_inertia: V3,
    pub verts: Vec<MeshVertex>,
    contacts: Vec<Contact>,
    /// The hardest it hit anything this step (blocks a second into the surface).
    hit: f64,
}

impl Body {
    pub fn new(x: V3, q: Quat, half: V3, verts: Vec<MeshVertex>) -> Body {
        let h = v3(half.x.max(0.02), half.y.max(0.02), half.z.max(0.02));
        // Density 1, a box; small pieces are not made heavier than they are.
        let mass = (8.0 * h.x * h.y * h.z).max(1e-4);
        let (a, b, c) = (2.0 * h.x, 2.0 * h.y, 2.0 * h.z);
        let inertia = v3(
            mass / 12.0 * (b * b + c * c),
            mass / 12.0 * (a * a + c * c),
            mass / 12.0 * (a * a + b * b),
        );
        Body {
            x,
            q,
            v: V3::default(),
            w: V3::default(),
            px: x,
            pq: q,
            half: h,
            inv_mass: 1.0 / mass,
            inv_inertia: v3(1.0 / inertia.x, 1.0 / inertia.y, 1.0 / inertia.z),
            verts,
            contacts: Vec::new(),
            hit: 0.0,
        }
    }

    fn inv_inertia_world(&self, v: V3) -> V3 {
        self.q.rotate(self.q.conj().rotate(v).mul_each(self.inv_inertia))
    }

    /// The inverse mass felt at offset `r` along `n`.
    fn gen_inv_mass(&self, r: V3, n: V3) -> f64 {
        let rn = r.cross(n);
        self.inv_mass + rn.dot(self.inv_inertia_world(rn))
    }

    fn move_by(&mut self, p: V3, r: V3) {
        self.x = self.x + p * self.inv_mass;
        let dw = self.inv_inertia_world(r.cross(p));
        let dq = Quat { w: 0.0, x: dw.x, y: dw.y, z: dw.z }.mul(self.q);
        self.q = Quat {
            w: self.q.w + 0.5 * dq.w,
            x: self.q.x + 0.5 * dq.x,
            y: self.q.y + 0.5 * dq.y,
            z: self.q.z + 0.5 * dq.z,
        }
        .normalize();
    }

    fn push(&mut self, p: V3, r: V3) {
        self.v = self.v + p * self.inv_mass;
        self.w = self.w + self.inv_inertia_world(r.cross(p));
    }

    fn corners(&self) -> [V3; 8] {
        let h = self.half * 0.999;
        std::array::from_fn(|i| {
            v3(
                if i & 1 == 0 { -h.x } else { h.x },
                if i & 2 == 0 { -h.y } else { h.y },
                if i & 4 == 0 { -h.z } else { h.z },
            )
        })
    }

    fn radius(&self) -> f64 {
        self.half.len()
    }

    /// Where a piece touches the blocks: its corners and, for a piece bigger than a crumb, also
    /// the middles of its edges and faces, so it cannot balance on a block's edge between two
    /// corners (a ragdoll propped up on it).
    fn samples(&self) -> Vec<V3> {
        let h = self.half * 0.999;
        if h.x.max(h.y).max(h.z) < 0.12 {
            return self.corners().to_vec();
        }
        let mut out = Vec::with_capacity(26);
        for ix in [-1.0, 0.0, 1.0] {
            for iy in [-1.0, 0.0, 1.0] {
                for iz in [-1.0, 0.0, 1.0] {
                    if ix == 0.0 && iy == 0.0 && iz == 0.0 {
                        continue;
                    }
                    out.push(v3(h.x * ix, h.y * iy, h.z * iz));
                }
            }
        }
        out
    }
}

/// Moves `a`'s point at `ra` by `corr` (and `b`'s at `rb` the other way), split by how easily
/// each moves there. Returns the size of the correction (the constraint's lambda).
fn correct(a: &mut Body, b: Option<(&mut Body, V3)>, corr: V3, ra: V3) -> f64 {
    let c = corr.len();
    if c < 1e-9 {
        return 0.0;
    }
    let n = corr * (1.0 / c);
    let wa = a.gen_inv_mass(ra, n);
    let wb = b.as_ref().map_or(0.0, |(b, rb)| b.gen_inv_mass(*rb, n));
    if wa + wb < 1e-12 {
        return 0.0;
    }
    let dl = c / (wa + wb);
    let p = n * dl;
    a.move_by(p, ra);
    if let Some((b, rb)) = b {
        b.move_by(-p, rb);
    }
    dl
}

#[derive(Clone, Copy, Debug)]
pub struct Joint {
    pub a: usize,
    pub b: usize,
    pub ra: V3,
    pub rb: V3,
}

#[derive(Clone, Debug)]
pub struct Object {
    pub first: usize,
    pub count: usize,
    pub joints: Vec<Joint>,
    pub age: f64,
    pub life: f64,
    /// The block a crumbling block's pieces start in: ignored until the block is gone.
    ignore: Option<[i32; 3]>,
    still: f64,
    pub sleeping: bool,
    check: f64,
    /// A dead mob (counted against the ragdoll limit).
    pub mob: bool,
    /// Seconds before it may make another landing sound.
    quiet: f64,
}

const SUBSTEPS: usize = 4;
const STATIC_FRICTION: f64 = 0.6;
const DYNAMIC_FRICTION: f64 = 0.45;
const RESTITUTION: f64 = 0.25;
const MAX_BODIES: usize = 1000;
/// Blocks a second and radians a second a piece may move at most.
const MAX_SPEED: f64 = 30.0;
const MAX_SPIN: f64 = 40.0;

pub struct Physics {
    pub bodies: Vec<Body>,
    pub objects: Vec<Object>,
    rng: Rng,
    time: f64,
    /// Bodies at most (the oldest objects go first).
    pub max_bodies: usize,
    /// How far a ragdoll's joint may be pulled apart before it tears (None: never).
    pub joint_break: Option<f64>,
    /// Where joints tore this step (drained by the caller, for the red mist).
    pub torn: Vec<V3>,
    /// How pieces go at the end.
    pub anim: AnimDef,
    /// Where pieces went this step (their centre, and how big they were): drained by the
    /// caller, for the animation's puffs.
    pub gone: Vec<(V3, f64)>,
    /// Pieces that landed hard this step: where, how fast, and whether a mob's.
    pub impacts: Vec<(V3, f64, bool)>,
    /// Ragdolls (dead mobs) at most at once.
    pub ragdoll_limit: usize,
    /// Solver steps a frame.
    pub substeps: usize,
    /// How many times more pieces a fractured mob makes (see `Settings::resolution_factor`).
    pub resolution: f64,
    /// Big mobs' joints hold more, tiny mobs stay whole when fractured.
    pub ragdoll_fix: bool,
}

impl Default for Physics {
    fn default() -> Self {
        Physics {
            bodies: Vec::new(),
            objects: Vec::new(),
            rng: Rng::new(0x5EED),
            time: 0.0,
            max_bodies: MAX_BODIES,
            joint_break: None,
            torn: Vec::new(),
            anim: BUILTIN_ANIMS[0].1,
            gone: Vec::new(),
            impacts: Vec::new(),
            ragdoll_limit: 16,
            substeps: SUBSTEPS,
            resolution: 1.0,
            ragdoll_fix: true,
        }
    }
}

fn two(bodies: &mut [Body], i: usize, j: usize) -> (&mut Body, &mut Body) {
    if i < j {
        let (a, b) = bodies.split_at_mut(j);
        (&mut a[i], &mut b[0])
    } else {
        let (a, b) = bodies.split_at_mut(i);
        (&mut b[0], &mut a[j])
    }
}

impl Physics {
    pub fn len(&self) -> usize {
        self.bodies.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bodies.is_empty()
    }

    pub fn clear(&mut self) {
        self.bodies.clear();
        self.objects.clear();
    }

    pub fn add(&mut self, bodies: Vec<Body>, joints: Vec<Joint>, life: f64, ignore: Option<[i32; 3]>) {
        if bodies.is_empty() {
            return;
        }
        // Too many pieces: the oldest go first.
        while self.bodies.len() + bodies.len() > self.max_bodies.max(bodies.len()) && !self.objects.is_empty() {
            let mut keep = vec![true; self.objects.len()];
            keep[0] = false;
            self.retain(&keep);
        }
        let first = self.bodies.len();
        let count = bodies.len();
        self.bodies.extend(bodies);
        self.objects.push(Object {
            first,
            count,
            joints,
            age: 0.0,
            life,
            ignore,
            still: 0.0,
            sleeping: false,
            check: 0.0,
            mob: false,
            quiet: 0.0,
        });
    }

    fn retain(&mut self, keep: &[bool]) {
        let mut bodies = Vec::with_capacity(self.bodies.len());
        let mut objects = Vec::with_capacity(self.objects.len());
        let old = std::mem::take(&mut self.bodies);
        let mut old: Vec<Option<Body>> = old.into_iter().map(Some).collect();
        for (o, &k) in std::mem::take(&mut self.objects).into_iter().zip(keep) {
            if !k {
                continue;
            }
            let first = bodies.len();
            for i in o.first..o.first + o.count {
                if let Some(b) = old[i].take() {
                    bodies.push(b);
                }
            }
            objects.push(Object { first, ..o });
        }
        self.bodies = bodies;
        self.objects = objects;
    }

    /// Advances the world by `dt` seconds.
    pub fn step(&mut self, dt: f64, gravity: f64, world: &dyn World) {
        let dt = dt.clamp(0.0, 0.05);
        if dt <= 0.0 || self.objects.is_empty() {
            return;
        }
        self.time += dt;
        let substeps = self.substeps.clamp(1, 16);
        let h = dt / substeps as f64;
        // Which body belongs to which object, and whether it moves.
        let mut owner = vec![0usize; self.bodies.len()];
        for (oi, o) in self.objects.iter().enumerate() {
            for i in o.first..o.first + o.count {
                owner[i] = oi;
            }
        }
        // Ignore a breaking block's own cell only until it has turned to air.
        for o in &mut self.objects {
            if let Some(c) = o.ignore
                && (o.age > 0.5 || matches!(world.cell(c[0], c[1], c[2]), Cell::Air | Cell::Fluid))
            {
                o.ignore = None;
            }
        }
        for b in &mut self.bodies {
            b.hit = 0.0;
        }
        for _ in 0..substeps {
            self.substep(h, gravity, world, &owner);
        }
        // Landings: an object that hit something hard makes a sound (then keeps quiet a moment).
        for o in &mut self.objects {
            o.quiet -= dt;
            if o.sleeping || o.quiet > 0.0 {
                continue;
            }
            let bodies = &self.bodies[o.first..o.first + o.count];
            if let Some(b) = bodies.iter().max_by(|a, b| a.hit.total_cmp(&b.hit))
                && b.hit > 2.5
                && b.half.x + b.half.y + b.half.z > 0.09
                && self.impacts.len() < 16
            {
                self.impacts.push((b.x, b.hit, o.mob));
                o.quiet = 0.3;
            }
        }
        self.after_step(dt, world);
    }

    fn substep(&mut self, h: f64, gravity: f64, world: &dyn World, owner: &[usize]) {
        // Move.
        for o in &self.objects {
            if o.sleeping {
                continue;
            }
            for b in &mut self.bodies[o.first..o.first + o.count] {
                b.px = b.x;
                b.pq = b.q;
                let mut g = gravity;
                let c = b.x;
                if matches!(world.cell(floor(c.x), floor(c.y), floor(c.z)), Cell::Fluid) {
                    g *= 0.15;
                    let k = (1.0 - 4.0 * h).max(0.0);
                    b.v = b.v * k;
                    b.w = b.w * k;
                }
                b.v.y -= g * h;
                b.x = b.x + b.v * h;
                b.q = b.q.integrate(b.w, h);
            }
        }
        // Joints of ragdolls; one pulled too far apart tears.
        let limit = self.joint_break;
        let fix = self.ragdoll_fix;
        for o in &mut self.objects {
            if o.sleeping {
                continue;
            }
            let mut torn = Vec::new();
            for (k, j) in o.joints.iter().enumerate() {
                let (a, b) = two(&mut self.bodies, o.first + j.a, o.first + j.b);
                let ra = a.q.rotate(j.ra);
                let rb = b.q.rotate(j.rb);
                let corr = (b.x + rb) - (a.x + ra);
                // A big mob's parts (the Wither's) swing wide: their joints may stretch in step with
                // the parts, where the plain limit tore them at the first knock.
                let reach = a.half.x.max(a.half.y).max(a.half.z) + b.half.x.max(b.half.y).max(b.half.z);
                let hold = if fix { (reach / 0.7).clamp(1.0, 2.5) } else { 1.0 };
                if limit.is_some_and(|l| corr.len() > l * hold) {
                    torn.push(k);
                    self.torn.push(a.x + ra + corr * 0.5);
                    continue;
                }
                correct(a, Some((b, rb)), corr, ra);
            }
            for k in torn.into_iter().rev() {
                o.joints.remove(k);
            }
        }
        // Blocks.
        for o in &self.objects {
            if o.sleeping {
                continue;
            }
            for b in &mut self.bodies[o.first..o.first + o.count] {
                b.contacts.clear();
                for c in b.samples() {
                    let p = b.x + b.q.rotate(c);
                    let prev = b.px + b.pq.rotate(c);
                    let Some((n, depth)) = penetration(world, p, prev, o.ignore) else {
                        continue;
                    };
                    let r = p - b.x;
                    let vn_before = (b.v + b.w.cross(r)).dot(n);
                    let lambda = correct(b, None, n * depth, r);
                    // Static friction: a corner that slid less than the push stays put.
                    let r = b.q.rotate(c);
                    let p = b.x + r;
                    let d = p - prev;
                    let slide = d - n * d.dot(n);
                    if slide.len() < STATIC_FRICTION * depth {
                        correct(b, None, -slide, r);
                    }
                    b.hit = b.hit.max(-vn_before);
                    b.contacts.push(Contact { local: c, n, lambda, vn_before });
                }
            }
        }
        // Pieces against each other.
        self.collide_bodies(owner);
        // Velocities from the moves.
        for o in &self.objects {
            if o.sleeping {
                continue;
            }
            for b in &mut self.bodies[o.first..o.first + o.count] {
                b.v = (b.x - b.px) * (1.0 / h);
                let mut dq = b.q.mul(b.pq.conj());
                if dq.w < 0.0 {
                    dq = Quat { w: -dq.w, x: -dq.x, y: -dq.y, z: -dq.z };
                }
                b.w = v3(dq.x, dq.y, dq.z) * (2.0 / h);
                // A piece pushed out of a block (or another piece) deep inside it is moved out,
                // not fired out: the move must not turn into a speed no fall or blast gives.
                let speed = b.v.len();
                if speed > MAX_SPEED {
                    b.v = b.v * (MAX_SPEED / speed);
                }
                let spin = b.w.len();
                if spin > MAX_SPIN {
                    b.w = b.w * (MAX_SPIN / spin);
                }
                // Bounce and sliding friction at the corners that touched.
                let contacts = std::mem::take(&mut b.contacts);
                for c in &contacts {
                    let r = b.q.rotate(c.local);
                    let vel = b.v + b.w.cross(r);
                    let vn = vel.dot(c.n);
                    let vt = vel - c.n * vn;
                    let mut dv = V3::default();
                    let vt_len = vt.len();
                    if vt_len > 1e-9 {
                        let f = (DYNAMIC_FRICTION * c.lambda / h).min(vt_len);
                        dv = dv - vt * (f / vt_len);
                    }
                    let e = if c.vn_before < -1.0 { RESTITUTION } else { 0.0 };
                    dv = dv + c.n * (-vn + (-e * c.vn_before).max(0.0));
                    let l = dv.len();
                    if l > 1e-9 {
                        let dir = dv * (1.0 / l);
                        let w = b.gen_inv_mass(r, dir);
                        if w > 1e-12 {
                            b.push(dir * (l / w), r);
                        }
                    }
                }
                b.contacts = contacts;
                // A little air drag, so spinning pieces settle.
                b.w = b.w * (1.0 - 0.6 * h);
                b.v = b.v * (1.0 - 0.05 * h);
            }
        }
    }

    fn collide_bodies(&mut self, owner: &[usize]) {
        let n = self.bodies.len();
        if n < 2 {
            return;
        }
        let mut order: Vec<usize> = (0..n).collect();
        order.sort_by(|&a, &b| {
            (self.bodies[a].x.x - self.bodies[a].radius())
                .partial_cmp(&(self.bodies[b].x.x - self.bodies[b].radius()))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        for ii in 0..n {
            let i = order[ii];
            let (xi, ri) = (self.bodies[i].x, self.bodies[i].radius());
            for &j in &order[ii + 1..] {
                let (xj, rj) = (self.bodies[j].x, self.bodies[j].radius());
                if xj.x - rj > xi.x + ri {
                    break;
                }
                if owner[i] == owner[j] {
                    continue;
                }
                let (si, sj) = (self.objects[owner[i]].sleeping, self.objects[owner[j]].sleeping);
                if si && sj {
                    continue;
                }
                if (xi - xj).len() > ri + rj {
                    continue;
                }
                self.pair(i, j, si, sj);
                self.pair(j, i, sj, si);
            }
        }
    }

    /// Pushes `a`'s corners out of `b`.
    fn pair(&mut self, a: usize, b: usize, a_static: bool, b_static: bool) {
        if a_static {
            return;
        }
        let corners = self.bodies[a].corners();
        for c in corners {
            let (ba, bb) = two(&mut self.bodies, a, b);
            let p = ba.x + ba.q.rotate(c);
            let lp = bb.q.conj().rotate(p - bb.x);
            let mut best: Option<(usize, f64)> = None;
            for k in 0..3 {
                let pen = bb.half.get(k) - lp.get(k).abs();
                if pen <= 0.0 {
                    best = None;
                    break;
                }
                if best.is_none_or(|x| pen < x.1) {
                    best = Some((k, pen));
                }
            }
            // `best` is None when the corner is outside on some axis.
            let inside = (0..3).all(|k| lp.get(k).abs() < bb.half.get(k));
            let (Some((k, pen)), true) = (best, inside) else { continue };
            let s = if lp.get(k) >= 0.0 { 1.0 } else { -1.0 };
            let nw = bb.q.rotate(V3::axis(k, s));
            let ra = p - ba.x;
            let rb = p - bb.x;
            if b_static {
                correct(ba, None, nw * pen, ra);
            } else {
                correct(ba, Some((bb, rb)), nw * pen, ra);
            }
        }
    }

    fn after_step(&mut self, dt: f64, world: &dyn World) {
        let mut keep = vec![true; self.objects.len()];
        for (oi, o) in self.objects.iter_mut().enumerate() {
            o.age += dt;
            if o.age > o.life {
                keep[oi] = false;
                if self.gone.len() < 64 {
                    let bodies = &self.bodies[o.first..o.first + o.count];
                    let n = bodies.len().max(1) as f64;
                    let c = bodies.iter().fold(V3::default(), |a, b| a + b.x) * (1.0 / n);
                    let size = bodies.iter().map(|b| (b.half.x + b.half.y + b.half.z) / 1.5).sum::<f64>().min(2.0);
                    self.gone.push((c, size));
                }
                continue;
            }
            let bodies = &mut self.bodies[o.first..o.first + o.count];
            if o.sleeping {
                // Wake when nothing holds the pieces up any more (the block below was broken).
                o.check += dt;
                if o.check > 0.5 {
                    o.check = 0.0;
                    let held = bodies.iter().any(|b| {
                        b.corners().iter().any(|&c| {
                            let p = b.x + b.q.rotate(c) - v3(0.0, 0.05, 0.0);
                            matches!(
                                world.cell(floor(p.x), floor(p.y), floor(p.z)),
                                Cell::Full | Cell::Boxes(_)
                            )
                        })
                    });
                    if !held {
                        o.sleeping = false;
                        o.still = 0.0;
                    }
                }
                continue;
            }
            let calm = bodies.iter().all(|b| b.v.len() < 0.12 && b.w.len() < 0.5);
            let touching = bodies.iter().any(|b| !b.contacts.is_empty());
            if calm && touching {
                o.still += dt;
            } else {
                o.still = 0.0;
            }
            if o.still > 0.8 {
                o.sleeping = true;
                for b in bodies.iter_mut() {
                    b.v = V3::default();
                    b.w = V3::default();
                }
            }
            // Pieces that fell out of the world.
            if bodies.iter().all(|b| b.x.y < -200.0) {
                keep[oi] = false;
            }
        }
        if keep.iter().any(|k| !k) {
            self.retain(&keep);
        }
    }

    /// Draws every piece, camera-relative, into `out`.
    pub fn emit(&self, camera: V3, world: &dyn World, out: &mut Vec<Vertex>) {
        for o in &self.objects {
            // The end, over the animation's last seconds: k runs from 1 down to 0.
            let (s, lift) = despawn_pose(&self.anim, o.life - o.age);
            for b in &self.bodies[o.first..o.first + o.count] {
                let light = world.light(floor(b.x.x), floor(b.x.y), floor(b.x.z));
                let at = b.x - camera + v3(0.0, lift, 0.0);
                for v in &b.verts {
                    let lp = v3(
                        f64::from(v.pos[0] * s),
                        f64::from(v.pos[1] * s),
                        f64::from(v.pos[2] * s),
                    );
                    let p = at + b.q.rotate(lp);
                    let n = b.q.rotate(V3::from_f32(v.normal));
                    let pack = |x: f64| (x.clamp(-1.0, 1.0) * 127.0) as i8;
                    out.push(Vertex {
                        pos: [p.x as f32, p.y as f32, p.z as f32],
                        uv: v.uv,
                        layer: v.layer,
                        color: v.color,
                        light,
                        normal: [pack(n.x), pack(n.y), pack(n.z), v.normal_w],
                        glint_uv: v.glint_uv,
                    });
                }
            }
        }
    }

    fn rng(&mut self) -> &mut Rng {
        &mut self.rng
    }
}

/// How a piece looks with `left` seconds to go under despawn animation `a`: its scale, and how
/// far it is moved up (down when negative).
pub fn despawn_pose(a: &AnimDef, left: f64) -> (f32, f64) {
    let d = f64::from(a.duration).max(0.01);
    let k = (left / d).clamp(0.0, 1.0);
    let done = 1.0 - k;
    match a.motion {
        // Shrink away.
        0 => (k as f32, 0.0),
        // Sink into the ground.
        1 => (1.0, -done * 0.6),
        // Float up as it shrinks.
        2 => (k.sqrt() as f32, done * done * 0.9),
        // Hop up a few times, smaller each time, and shrink on the last.
        3 => {
            let hop = (done * std::f64::consts::PI * 3.0).sin().abs() * 0.35 * k;
            ((k / 0.25).min(1.0) as f32, hop)
        }
        _ => (1.0, 0.0),
    }
}

// ---------------------------------------------------------------------------------------------
// Crumbling blocks
// ---------------------------------------------------------------------------------------------

/// Splits box `b` (block space) into about `n` smaller boxes of uneven sizes.
pub fn split_box(b: [f64; 6], n: usize, rng: &mut Rng) -> Vec<[f64; 6]> {
    let mut boxes = vec![b];
    while boxes.len() < n.max(1) {
        // Split the biggest box across its longest side, somewhere near the middle.
        let (i, _) = boxes
            .iter()
            .enumerate()
            .map(|(i, b)| (i, (b[3] - b[0]) * (b[4] - b[1]) * (b[5] - b[2])))
            .fold((0, -1.0), |a, x| if x.1 > a.1 { x } else { a });
        let b = boxes[i];
        let size = [b[3] - b[0], b[4] - b[1], b[5] - b[2]];
        let mut k = if size[0] >= size[1] && size[0] >= size[2] {
            0
        } else if size[1] >= size[2] {
            1
        } else {
            2
        };
        // Now and then across another side, so not every piece is a cube.
        if rng.next() < 0.15 {
            k = (k + 1 + (rng.next() * 2.0) as usize) % 3;
        }
        if size[k] < 0.08 {
            break;
        }
        let at = b[k] + size[k] * rng.range(0.35, 0.65);
        let mut lo = b;
        let mut hi = b;
        lo[k + 3] = at;
        hi[k] = at;
        boxes[i] = lo;
        boxes.push(hi);
    }
    boxes
}

/// The six faces of box `b` (block space, block at `origin`), textured with the part of the
/// sprite `uv` that the box covers, so the pieces fit back together into the block.
pub fn box_mesh(b: [f64; 6], uv: [f32; 4], color: u32) -> (V3, V3, Vec<MeshVertex>) {
    let lo = v3(b[0], b[1], b[2]);
    let hi = v3(b[3], b[4], b[5]);
    let c = (lo + hi) * 0.5;
    let h = (hi - lo) * 0.5;
    let map = |u: f64, v: f64| -> [f32; 2] {
        let u = u.clamp(0.0, 1.0) as f32;
        let v = v.clamp(0.0, 1.0) as f32;
        [uv[0] + (uv[2] - uv[0]) * u, uv[1] + (uv[3] - uv[1]) * v]
    };
    let mut verts = Vec::with_capacity(24);
    // For each face: its normal and its four corners in block space, in order around the face.
    let faces: [([f32; 3], [V3; 4]); 6] = [
        ([1.0, 0.0, 0.0], [v3(hi.x, lo.y, lo.z), v3(hi.x, hi.y, lo.z), v3(hi.x, hi.y, hi.z), v3(hi.x, lo.y, hi.z)]),
        ([-1.0, 0.0, 0.0], [v3(lo.x, lo.y, hi.z), v3(lo.x, hi.y, hi.z), v3(lo.x, hi.y, lo.z), v3(lo.x, lo.y, lo.z)]),
        ([0.0, 1.0, 0.0], [v3(lo.x, hi.y, lo.z), v3(lo.x, hi.y, hi.z), v3(hi.x, hi.y, hi.z), v3(hi.x, hi.y, lo.z)]),
        ([0.0, -1.0, 0.0], [v3(lo.x, lo.y, hi.z), v3(lo.x, lo.y, lo.z), v3(hi.x, lo.y, lo.z), v3(hi.x, lo.y, hi.z)]),
        ([0.0, 0.0, 1.0], [v3(lo.x, lo.y, hi.z), v3(hi.x, lo.y, hi.z), v3(hi.x, hi.y, hi.z), v3(lo.x, hi.y, hi.z)]),
        ([0.0, 0.0, -1.0], [v3(hi.x, lo.y, lo.z), v3(lo.x, lo.y, lo.z), v3(lo.x, hi.y, lo.z), v3(hi.x, hi.y, lo.z)]),
    ];
    for (n, corners) in faces {
        for p in corners {
            let tex = if n[0] != 0.0 {
                map(if n[0] > 0.0 { 1.0 - p.z } else { p.z }, 1.0 - p.y)
            } else if n[1] != 0.0 {
                map(p.x, p.z)
            } else {
                map(if n[2] > 0.0 { p.x } else { 1.0 - p.x }, 1.0 - p.y)
            };
            let l = p - c;
            verts.push(MeshVertex {
                pos: [l.x as f32, l.y as f32, l.z as f32],
                normal: n,
                normal_w: -127,
                uv: tex,
                layer: 0,
                color,
                glint_uv: [0.0, 0.0],
            });
        }
    }
    (c, h, verts)
}

/// How a broken block's box `b` (block space, `share` of the block's volume) comes apart in
/// `style` (`Settings::block_style`), `want` pieces for a whole block: each piece's box, its turn
/// and its velocity and spin. The game and the quick setup's previews both use it.
pub fn block_pieces(style: u8, b: [f64; 6], share: f64, want: usize, rng: &mut Rng) -> Vec<([f64; 6], Quat, V3, V3)> {
    let mid = v3((b[0] + b[3]) * 0.5, (b[1] + b[4]) * 0.5, (b[2] + b[5]) * 0.5);
    let mut out = Vec::new();
    match style % 4 {
        // Blocky: the block pops out whole and tumbles.
        0 => {
            let v = v3(rng.range(-0.4, 0.4), rng.range(1.6, 2.6), rng.range(-0.4, 0.4));
            out.push((b, Quat::IDENTITY, v, rng.unit() * rng.range(1.0, 3.0)));
        }
        // Particles: a cloud of small cubes, a quarter of a block each, thrown about.
        1 => {
            let n = ((4.0 * share.cbrt()).round() as usize).clamp(1, 4);
            let size = [(b[3] - b[0]) / n as f64, (b[4] - b[1]) / n as f64, (b[5] - b[2]) / n as f64];
            for i in 0..n {
                for j in 0..n {
                    for k in 0..n {
                        let lo = [b[0] + size[0] * i as f64, b[1] + size[1] * j as f64, b[2] + size[2] * k as f64];
                        // A little smaller than its cell, so they read as bits.
                        let s = 0.18;
                        let c = [lo[0] + size[0] * 0.5, lo[1] + size[1] * 0.5, lo[2] + size[2] * 0.5];
                        let piece = [
                            c[0] - size[0] * (0.5 - s), c[1] - size[1] * (0.5 - s), c[2] - size[2] * (0.5 - s),
                            c[0] + size[0] * (0.5 - s), c[1] + size[1] * (0.5 - s), c[2] + size[2] * (0.5 - s),
                        ];
                        let out_dir = (v3(c[0], c[1], c[2]) - mid).norm();
                        let r = rng.unit();
                        let v = v3(out_dir.x * 2.2 + r.x * 1.2, rng.range(1.0, 3.2), out_dir.z * 2.2 + r.z * 1.2);
                        out.push((piece, Quat::IDENTITY, v, rng.unit() * rng.range(2.0, 9.0)));
                    }
                }
            }
        }
        // Crumbled voxel: blocky lumps of the block's own pixels.
        2 => {
            let n = ((want as f64 * share).round() as usize).max(1);
            for piece in split_box(b, n, rng) {
                let c = v3((piece[0] + piece[3]) * 0.5, (piece[1] + piece[4]) * 0.5, (piece[2] + piece[5]) * 0.5);
                let out_dir = (c - mid).norm();
                let r = rng.unit();
                let v = v3(out_dir.x * 0.6 + r.x * 0.4, rng.range(0.3, 1.6), out_dir.z * 0.6 + r.z * 0.4);
                out.push((piece, Quat::IDENTITY, v, rng.unit() * rng.range(0.5, 4.0)));
            }
        }
        // Fractured: shards, cut every which way, turned as they break off.
        _ => {
            let n = ((want as f64 * share * 1.5).round() as usize).max(2);
            for piece in split_box(b, n, rng) {
                let c = v3((piece[0] + piece[3]) * 0.5, (piece[1] + piece[4]) * 0.5, (piece[2] + piece[5]) * 0.5);
                // Thinner along one side: a shard rather than a lump.
                let mut p = piece;
                let k = (rng.next() * 3.0) as usize % 3;
                let shrink = (p[k + 3] - p[k]) * rng.range(0.15, 0.35);
                p[k] += shrink * 0.5;
                p[k + 3] -= shrink * 0.5;
                let q = Quat::axis_angle(rng.unit(), rng.range(0.2, 0.7));
                let out_dir = (c - mid).norm();
                let r = rng.unit();
                let v = v3(out_dir.x * 1.1 + r.x * 0.5, rng.range(0.5, 2.0), out_dir.z * 1.1 + r.z * 0.5);
                out.push((p, q, v, rng.unit() * rng.range(1.0, 6.0)));
            }
        }
    }
    out
}

/// RGB tint (0xRRGGBB) as an entity vertex colour (ABGR bytes).
pub fn tint_color(tint: u32) -> u32 {
    let (r, g, b) = ((tint >> 16) & 255, (tint >> 8) & 255, tint & 255);
    0xFF00_0000 | b << 16 | g << 8 | r
}

impl Physics {
    /// The block at `pos` (outline `boxes` in block space) falls apart into textured pieces.
    pub fn spawn_block(&mut self, pos: [i32; 3], boxes: &[[f64; 6]], uv: [f32; 4], tint: u32, s: &Settings) {
        let total: f64 = boxes.iter().map(|b| (b[3] - b[0]) * (b[4] - b[1]) * (b[5] - b[2])).sum();
        if total <= 1e-6 {
            return;
        }
        let o = v3(f64::from(pos[0]), f64::from(pos[1]), f64::from(pos[2]));
        let color = tint_color(tint);
        let mut bodies = Vec::new();
        for b in boxes {
            let vol = (b[3] - b[0]) * (b[4] - b[1]) * (b[5] - b[2]);
            for (piece, q, v, w) in block_pieces(s.block_style, *b, vol / total, s.pieces_per_block(), self.rng()) {
                let (c, h, verts) = box_mesh(piece, uv, color);
                let mut body = Body::new(o + c, q, h, verts);
                body.v = v;
                body.w = w;
                bodies.push(body);
            }
        }
        let life = s.lifetime() + self.rng.range(0.0, f64::from(s.block_var).max(0.0));
        self.add(bodies, Vec::new(), life, Some(pos));
    }

    /// A particle of the game's become a small piece of its sprite (`uv`, the whole sprite; the
    /// piece shows a quarter of it), at `at` with velocity `v` (blocks a second).
    pub fn spawn_bit(&mut self, at: V3, v: V3, uv: [f32; 4], tint: u32, life: f64) {
        let size = self.rng.range(0.05, 0.09);
        let (du, dv) = ((uv[2] - uv[0]) * 0.25, (uv[3] - uv[1]) * 0.25);
        let (ou, ov) = ((self.rng.next() * 3.0).floor() as f32, (self.rng.next() * 3.0).floor() as f32);
        let part = [uv[0] + du * ou, uv[1] + dv * ov, uv[0] + du * (ou + 1.0), uv[1] + dv * (ov + 1.0)];
        let (_, h, verts) = box_mesh([0.0, 0.0, 0.0, size, size, size], part, tint_color(tint));
        let mut body = Body::new(at, Quat::axis_angle(self.rng.unit(), self.rng.range(0.0, 3.0)), h, verts);
        body.v = v;
        body.w = self.rng.unit() * self.rng.range(1.0, 8.0);
        self.add(vec![body], Vec::new(), life, None);
    }

    /// Blocks that lost their support (`collapse`), falling as they were: each a whole block
    /// (leaves in a few clumps), tipping over away from the cut (`push`), the higher the more,
    /// as a tree falls over from its foot at `foot`.
    pub fn spawn_falling(&mut self, blocks: &[Falling], foot: V3, push: V3, s: &Settings) {
        let mut bodies = Vec::new();
        let side = v3(-push.z, 0.0, push.x);
        for b in blocks {
            let o = v3(f64::from(b.pos[0]), f64::from(b.pos[1]), f64::from(b.pos[2]));
            let color = tint_color(b.tint);
            let up = (o.y + 0.5 - foot.y).max(0.0);
            // Leaves in clumps; the high blocks of a structure crumble as they come down.
            let crumble = s.collapse_dust && up > 3.0 && self.rng.next() < 0.3;
            let pieces = if b.leaves || crumble { split_box([0.0, 0.0, 0.0, 1.0, 1.0, 1.0], 2, self.rng()) } else { vec![[0.005, 0.005, 0.005, 0.995, 0.995, 0.995]] };
            for piece in pieces {
                let (c, h, verts) = box_mesh(piece, b.uv, color);
                let mut body = Body::new(o + c, Quat::IDENTITY, h, verts);
                // Turning about the foot: the higher, the faster it goes over.
                body.v = push * (0.55 * up) + v3(self.rng.range(-0.15, 0.15), self.rng.range(-0.1, 0.2), self.rng.range(-0.15, 0.15));
                body.w = side * (-0.55) + self.rng.unit() * self.rng.range(0.1, 0.6);
                bodies.push(body);
            }
        }
        let life = s.lifetime() + self.rng.range(0.0, f64::from(s.block_var).max(0.0));
        self.add(bodies, Vec::new(), life, None);
    }
}

/// A block that falls in a collapse: where it was, its sprite (atlas uv) and tint, and whether it
/// is leaves.
#[derive(Clone, Copy, Debug)]
pub struct Falling {
    pub pos: [i32; 3],
    pub uv: [f32; 4],
    pub tint: u32,
    pub leaves: bool,
}

// ---------------------------------------------------------------------------------------------
// Mobs: catching the model parts of a dying mob
// ---------------------------------------------------------------------------------------------

/// Set while this module draws a dying mob itself (the hook's own nested call).
pub struct Guard(AtomicBool);

impl Guard {
    pub const fn new() -> Self {
        Self(AtomicBool::new(false))
    }
    /// True when the guard was already set.
    pub fn enter(&self) -> bool {
        self.0.swap(true, Ordering::SeqCst)
    }
    pub fn leave(&self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

impl Default for Guard {
    fn default() -> Self {
        Self::new()
    }
}

pub static RENDER: Guard = Guard::new();
/// Set while the lead hook draws the game's own leads into its buffer.
pub static BOATS: Guard = Guard::new();
/// The fishing bobbers' hook asking the game for its own bobbers (the nested call goes through).
pub static BOBBER: Guard = Guard::new();
/// Set while the camera hook asks the game for its own camera.
pub static CAMERA: Guard = Guard::new();
/// Set while the player-model hook draws the player itself.
pub static PLAYER_MODEL: Guard = Guard::new();

static RIDER: Mutex<Option<[f64; 3]>> = Mutex::new(None);

/// How far the boat the player rides is moved by the swell this frame (the camera hook sets
/// it, the player model is moved by it too, so the player sits in the boat).
pub fn set_rider_lift(lift: Option<[f64; 3]>) {
    *RIDER.lock().unwrap_or_else(|e| e.into_inner()) = lift;
}

pub fn rider_lift() -> Option<[f64; 3]> {
    *RIDER.lock().unwrap_or_else(|e| e.into_inner())
}

/// A swinging door's panel (`sway::door_swings`): the box `from` of each half (lower with sprite
/// `bottom`, upper with `top`), turned by `angle` radians about the hinge (`pivot`, x and z in
/// block space), drawn camera-relative into `out`, lit by the door's own light.
pub fn door_draw(pos: [i32; 3], from: [f64; 6], pivot: [f64; 2], angle: f64, bottom: [f32; 4], top: [f32; 4], camera: [f64; 3], world: &dyn World, out: &mut Vec<Vertex>) {
    let turn = Quat::axis_angle(v3(0.0, 1.0, 0.0), angle);
    let hinge = v3(f64::from(pos[0]) + pivot[0], f64::from(pos[1]), f64::from(pos[2]) + pivot[1]);
    let cam = v3(camera[0], camera[1], camera[2]);
    for (dy, uv) in [(0, bottom), (1, top)] {
        let light = world.light(pos[0], pos[1] + dy, pos[2]);
        let (c, _, verts) = box_mesh(from, uv, 0xFFFF_FFFF);
        let centre = v3(f64::from(pos[0]), f64::from(pos[1] + dy), f64::from(pos[2])) + c;
        for v in &verts {
            let p = centre + V3::from_f32(v.pos) - hinge;
            let p = hinge + turn.rotate(p) - cam;
            let n = turn.rotate(V3::from_f32(v.normal));
            let pack = |x: f64| (x.clamp(-1.0, 1.0) * 127.0) as i8;
            out.push(Vertex {
                pos: [p.x as f32, p.y as f32, p.z as f32],
                uv: v.uv,
                layer: v.layer,
                color: v.color,
                light,
                normal: [pack(n.x), pack(n.y), pack(n.z), v.normal_w],
                glint_uv: v.glint_uv,
            });
        }
    }
}
pub static LEADS: Guard = Guard::new();

static CAPTURE: Mutex<Option<Vec<usize>>> = Mutex::new(None);
static CAPTURING: AtomicBool = AtomicBool::new(false);
/// PVP server compatibility: the mod's world effects are off while the window plays on someone
/// else's server (a dedicated server, or a LAN world it joined). The window of a single-player
/// world is also "connected" (`connected`), but to its own integrated server (`own_server`): there
/// is no other player to stay fair to, so the effects stay on.
pub fn pvp_compat_off(setting: bool, connected: bool, own_server: bool) -> bool {
    setting && connected && !own_server
}

/// Whether the mod is on in Options > Mods (kept by the frame hook, read by the mob hook).
static ENABLED: AtomicBool = AtomicBool::new(true);

pub fn set_enabled(on: bool) {
    ENABLED.store(on, Ordering::Relaxed);
}

pub fn capture_begin() {
    if let Ok(mut c) = CAPTURE.lock() {
        *c = Some(Vec::new());
        CAPTURING.store(true, Ordering::SeqCst);
    }
}

/// A model part starts at vertex `len` of the buffer.
pub fn capture_mark(len: usize) {
    if !CAPTURING.load(Ordering::Relaxed) {
        return;
    }
    if let Ok(mut c) = CAPTURE.lock()
        && let Some(marks) = c.as_mut()
    {
        marks.push(len);
    }
}

pub fn capture_end() -> Vec<usize> {
    CAPTURING.store(false, Ordering::SeqCst);
    CAPTURE.lock().ok().and_then(|mut c| c.take()).unwrap_or_default()
}

/// A run of captured vertices: one model part (with any layers drawn over it).
struct Segment {
    verts: Vec<Vertex>,
    lo: V3,
    hi: V3,
}

impl Segment {
    fn volume(&self) -> f64 {
        let d = self.hi - self.lo;
        d.x.max(0.01) * d.y.max(0.01) * d.z.max(0.01)
    }
    fn center(&self) -> V3 {
        (self.lo + self.hi) * 0.5
    }
}

fn bounds(verts: &[Vertex], camera: V3) -> (V3, V3) {
    let mut lo = v3(f64::MAX, f64::MAX, f64::MAX);
    let mut hi = v3(f64::MIN, f64::MIN, f64::MIN);
    for v in verts {
        let p = V3::from_f32(v.pos) + camera;
        for k in 0..3 {
            lo.set(k, lo.get(k).min(p.get(k)));
            hi.set(k, hi.get(k).max(p.get(k)));
        }
    }
    (lo, hi)
}

fn overlap(a: (V3, V3), b: (V3, V3), margin: f64) -> Option<(V3, V3)> {
    let lo = v3(a.0.x.max(b.0.x), a.0.y.max(b.0.y), a.0.z.max(b.0.z)) - v3(margin, margin, margin);
    let hi = v3(a.1.x.min(b.1.x), a.1.y.min(b.1.y), a.1.z.min(b.1.z)) + v3(margin, margin, margin);
    if lo.x < hi.x && lo.y < hi.y && lo.z < hi.z { Some((lo, hi)) } else { None }
}

/// Cuts the captured vertices into model parts. Layers drawn over a part (an outer skin layer,
/// armour) land on the part they cover.
fn segments(verts: &[Vertex], marks: &[usize], camera: V3) -> Vec<Segment> {
    let mut cuts: Vec<usize> = marks.iter().copied().filter(|&m| m < verts.len()).collect();
    cuts.push(0);
    cuts.push(verts.len());
    cuts.sort_unstable();
    cuts.dedup();
    let mut segs: Vec<Segment> = Vec::new();
    for w in cuts.windows(2) {
        let part = &verts[w[0]..w[1]];
        if part.len() < 3 {
            if let Some(last) = segs.last_mut() {
                last.verts.extend_from_slice(part);
            }
            continue;
        }
        let (lo, hi) = bounds(part, camera);
        let seg = Segment { verts: part.to_vec(), lo, hi };
        // An overlay: mostly the same box as an earlier part.
        let host = segs.iter().position(|t| {
            let c = seg.center();
            let inside = (0..3).all(|k| c.get(k) > t.lo.get(k) - 0.1 && c.get(k) < t.hi.get(k) + 0.1);
            let ratio = seg.volume() / t.volume();
            let shared = overlap((t.lo, t.hi), (seg.lo, seg.hi), 0.0).map_or(0.0, |(l, h)| {
                let d = h - l;
                d.x * d.y * d.z
            });
            inside && (0.4..2.6).contains(&ratio) && shared > 0.45 * seg.volume().min(t.volume())
        });
        match host {
            Some(i) => {
                let t = &mut segs[i];
                t.verts.extend_from_slice(&seg.verts);
                for k in 0..3 {
                    t.lo.set(k, t.lo.get(k).min(seg.lo.get(k)));
                    t.hi.set(k, t.hi.get(k).max(seg.hi.get(k)));
                }
            }
            None => segs.push(seg),
        }
    }
    segs
}

/// A body for a part: a box along the part's own axes (from its face normals) around its vertices.
fn part_body(seg: &Segment, camera: V3) -> Body {
    let mut normals: Vec<V3> = Vec::new();
    for quad in seg.verts.chunks(4) {
        if quad.len() < 3 {
            continue;
        }
        let a = V3::from_f32(quad[0].pos);
        let n = (V3::from_f32(quad[1].pos) - a).cross(V3::from_f32(quad[2].pos) - a);
        if n.len() > 1e-7 {
            normals.push(n.norm());
        }
    }
    let ax = normals.first().copied().unwrap_or(v3(1.0, 0.0, 0.0));
    let other = normals
        .iter()
        .copied()
        .find(|n| n.dot(ax).abs() < 0.5)
        .unwrap_or(if ax.y.abs() < 0.9 { v3(0.0, 1.0, 0.0) } else { v3(1.0, 0.0, 0.0) });
    let by = (other - ax * other.dot(ax)).norm();
    let cz = ax.cross(by);
    let mut lo = v3(f64::MAX, f64::MAX, f64::MAX);
    let mut hi = v3(f64::MIN, f64::MIN, f64::MIN);
    for v in &seg.verts {
        let p = V3::from_f32(v.pos) + camera;
        let l = v3(p.dot(ax), p.dot(by), p.dot(cz));
        for k in 0..3 {
            lo.set(k, lo.get(k).min(l.get(k)));
            hi.set(k, hi.get(k).max(l.get(k)));
        }
    }
    let mid = (lo + hi) * 0.5;
    let center = ax * mid.x + by * mid.y + cz * mid.z;
    let half = (hi - lo) * 0.5;
    let q = Quat::from_axes(ax, by, cz);
    let inv = q.conj();
    let verts = seg
        .verts
        .iter()
        .map(|v| {
            let p = inv.rotate(V3::from_f32(v.pos) + camera - center);
            let n = inv.rotate(v3(
                f64::from(v.normal[0]) / 127.0,
                f64::from(v.normal[1]) / 127.0,
                f64::from(v.normal[2]) / 127.0,
            ));
            // The hurt-red overlay of the dying mob goes; an item's foil stays.
            let layer = match v.layer {
                1 => 0,
                8 => 7,
                l => l,
            };
            MeshVertex {
                pos: [p.x as f32, p.y as f32, p.z as f32],
                normal: [n.x as f32, n.y as f32, n.z as f32],
                normal_w: v.normal[3],
                uv: v.uv,
                layer,
                color: v.color,
                glint_uv: v.glint_uv,
            }
        })
        .collect();
    Body::new(center, q, half, verts)
}

/// Joints between parts that touch: a tree from the biggest part (the body), each joint where the
/// two parts meet. Parts that touch nothing hang from the nearest part.
fn joints(segs: &[Segment]) -> Vec<(usize, usize, V3)> {
    let n = segs.len();
    if n < 2 {
        return Vec::new();
    }
    let root = (0..n)
        .max_by(|&a, &b| segs[a].volume().partial_cmp(&segs[b].volume()).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap_or(0);
    let mut inside = vec![false; n];
    inside[root] = true;
    let mut out = Vec::new();
    for _ in 1..n {
        let mut best: Option<(usize, usize, f64, V3)> = None;
        for a in (0..n).filter(|&a| inside[a]) {
            for b in (0..n).filter(|&b| !inside[b]) {
                let (sa, sb) = (&segs[a], &segs[b]);
                let (score, at) = match overlap((sa.lo, sa.hi), (sb.lo, sb.hi), 0.04) {
                    Some((l, h)) => {
                        let d = h - l;
                        (1.0 + d.x * d.y * d.z, (l + h) * 0.5)
                    }
                    None => {
                        // Not touching: hang it from the closest point of the other.
                        let c = sb.center();
                        let p = v3(
                            c.x.clamp(sa.lo.x, sa.hi.x),
                            c.y.clamp(sa.lo.y, sa.hi.y),
                            c.z.clamp(sa.lo.z, sa.hi.z),
                        );
                        (-(p - c).len(), (p + c) * 0.5)
                    }
                };
                if best.is_none_or(|x| score > x.2) {
                    best = Some((a, b, score, at));
                }
            }
        }
        let Some((a, b, _, at)) = best else { break };
        inside[b] = true;
        out.push((a, b, at));
    }
    out
}

impl Physics {
    /// A mob died: its parts (captured vertices, camera-relative) become a ragdoll or fall apart.
    /// `velocity` is the mob's motion in blocks per second.
    pub fn spawn_mob(
        &mut self,
        verts: &[Vertex],
        marks: &[usize],
        camera: V3,
        velocity: V3,
        mode: u8,
        life: f64,
    ) {
        let segs = segments(verts, marks, camera);
        if segs.is_empty() {
            return;
        }
        let mut bodies: Vec<Body> = segs.iter().map(|s| part_body(s, camera)).collect();
        // Fractured: each part breaks up into pieces of its own skin.
        if mode == 3 {
            let mut pieces = Vec::new();
            for b in &bodies {
                let (mut lo, mut hi) = ([f32::MAX; 2], [f32::MIN; 2]);
                for v in &b.verts {
                    for k in 0..2 {
                        lo[k] = lo[k].min(v.uv[k]);
                        hi[k] = hi[k].max(v.uv[k]);
                    }
                }
                let (layer, color) = b.verts.first().map_or((0, u32::MAX), |v| (v.layer, v.color));
                let size = b.half * 2.0;
                // A tiny part (a chicken's beak, a bat's wing) stays whole: split, its shards would
                // be thinner than the least a body can be and swell into boxes.
                let tiny = size.x.max(size.y).max(size.z) < 0.12;
                let top = ((7.0 * self.resolution).round() as usize).clamp(7, 20);
                let n = if tiny && self.ragdoll_fix {
                    1
                } else {
                    ((size.x * size.y * size.z * 90.0 * self.resolution).round() as usize).clamp(2, top)
                };
                for unit in split_box([0.0, 0.0, 0.0, 1.0, 1.0, 1.0], n, &mut self.rng) {
                    let (c, h, mut verts) = box_mesh(unit, [lo[0], lo[1], hi[0], hi[1]], color);
                    for v in &mut verts {
                        v.layer = layer;
                        v.pos = [v.pos[0] * size.x as f32, v.pos[1] * size.y as f32, v.pos[2] * size.z as f32];
                    }
                    let local = v3((c.x - 0.5) * size.x, (c.y - 0.5) * size.y, (c.z - 0.5) * size.z);
                    let half = v3(h.x * size.x, h.y * size.y, h.z * size.z);
                    pieces.push(Body::new(b.x + b.q.rotate(local), b.q, half, verts));
                }
            }
            bodies = pieces;
        }
        let mid = bodies.iter().fold(V3::default(), |a, b| a + b.x) * (1.0 / bodies.len() as f64);
        // Knocked away from the camera (the player that landed the blow, mostly).
        let away = v3(mid.x - camera.x, 0.0, mid.z - camera.z).norm();
        let base = velocity * 0.5 + away * 1.2 + v3(0.0, 1.2, 0.0);
        let mut joint_list = Vec::new();
        if mode == 2 || mode == 3 {
            for b in &mut bodies {
                let out = (b.x - mid).norm();
                let r = self.rng.unit();
                b.v = base + out * 2.5 + r * 0.8 + v3(0.0, 1.0, 0.0);
                b.w = self.rng.unit() * self.rng.range(2.0, 8.0);
            }
        } else {
            for (a, b, at) in joints(&segs) {
                let ra = bodies[a].q.conj().rotate(at - bodies[a].x);
                let rb = bodies[b].q.conj().rotate(at - bodies[b].x);
                joint_list.push(Joint { a, b, ra, rb });
            }
            let spin = self.rng.unit() * 1.5;
            for b in &mut bodies {
                let r = self.rng.unit();
                b.v = base + r * 0.4;
                b.w = spin;
            }
        }
        self.add(bodies, joint_list, life, None);
        if let Some(o) = self.objects.last_mut() {
            o.mob = true;
        }
        // Too many dead mobs: the oldest go.
        while self.objects.iter().filter(|o| o.mob).count() > self.ragdoll_limit.max(1) {
            let first = self.objects.iter().position(|o| o.mob).unwrap_or(0);
            let keep: Vec<bool> = (0..self.objects.len()).map(|i| i != first).collect();
            self.retain(&keep);
        }
    }
}

// ---------------------------------------------------------------------------------------------
// The shared state the hooks reach
// ---------------------------------------------------------------------------------------------

struct State {
    physics: Physics,
    /// Mobs already turned into pieces, by entity id, with when that was.
    dead: HashMap<u32, f64>,
    clock: Option<std::time::Instant>,
    last_dt: f64,
    camera: V3,
    dim: u8,
    items: HashMap<u32, (Quat, u64)>,
    frame: u64,
    env: Env,
}

static STATE: Mutex<Option<State>> = Mutex::new(None);

fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
    let mut g = STATE.lock().unwrap_or_else(|e| e.into_inner());
    let s = g.get_or_insert_with(|| State {
        physics: Physics::default(),
        dead: HashMap::new(),
        clock: None,
        last_dt: 1.0 / 60.0,
        camera: V3::default(),
        dim: 255,
        items: HashMap::new(),
        frame: 0,
        env: Env::default(),
    });
    f(s)
}

pub fn clear() {
    with(|s| {
        s.physics.clear();
        s.items.clear();
        s.env.clear();
    });
}

pub fn piece_count() -> usize {
    with(|s| s.physics.len())
}

/// The game's particles to make pieces of, as `sway::set_particle_catch` takes them.
pub fn particle_catch() -> u8 {
    let st = settings();
    if !ENABLED.load(Ordering::Relaxed) {
        return 0;
    }
    u8::from(st.crack) | u8::from(st.sprint) << 1 | u8::from(st.eat) << 2 | u8::from(st.item_break) << 3 | u8::from(st.block_particles) << 4 | u8::from(st.projectiles) << 5
}

/// The velocity Minecraft's `Particle` constructor gives a particle asked for `v` (blocks a
/// tick): a little jitter, then turned into a short hop of its own speed, a tenth of a block a
/// tick up. The sprint dust asks for (-4 vx, 1.5, -4 vz) and gets a hop, not a rocket.
pub fn particle_spread(v: [f64; 3], rng: &mut Rng) -> [f64; 3] {
    let mut d = [0.0; 3];
    for (k, c) in d.iter_mut().enumerate() {
        *c = v[k] + (rng.next() * 2.0 - 1.0) * 0.4;
    }
    let speed = (rng.next() + rng.next() + 1.0) * 0.15;
    let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt().max(1e-6);
    [d[0] / len * speed * 0.4, d[1] / len * speed * 0.4 + 0.1, d[2] / len * speed * 0.4]
}

/// A particle as a small physics piece (`uv` its sprite in the atlas), `v` in blocks a tick, as
/// the game asked for it (`spread`: through `particle_spread`, as the game's particle would).
pub fn spawn_bit(at: [f64; 3], v: [f64; 3], uv: [f32; 4], tint: u32, spread: bool) {
    let st = settings();
    with(|s| {
        let life = f64::from(st.particle_life) * 30.0 + s.physics.rng.range(0.0, f64::from(st.particle_var));
        let v = if spread { particle_spread(v, &mut s.physics.rng) } else { v };
        let v = v3(v[0], v[1], v[2]) * 20.0;
        s.physics.spawn_bit(v3(at[0], at[1], at[2]), v, uv, tint, life.max(0.3));
    });
}

/// Rain landing on the ground at `at`: a few drops thrown up.
pub fn rain_splash(at: [f64; 3]) {
    let st = settings();
    if !ENABLED.load(Ordering::Relaxed) || !st.weather || st.op_rain <= 0.0 {
        return;
    }
    with(|s| {
        let x = v3(at[0], at[1] + 0.02, at[2]);
        for _ in 0..3 {
            let rng = &mut s.env.rng;
            let r = rng.unit();
            let v = v3(r.x * 0.8, rng.range(1.0, 2.2), r.z * 0.8);
            let mut puff = Puff::new(FxKind::Dust, x, v, rng.range(0.012, 0.022), rng.range(0.22, 0.34), [0.62, 0.72, 0.95], [0.75, 0.82, 0.97], rng);
            puff.alpha *= f64::from(st.op_rain);
            s.env.puff(puff);
        }
    });
}

/// A snowflake drifting down from `at` (the weather hook puts them above the ground around the
/// camera where it snows).
pub fn snowflake(at: [f64; 3]) {
    let st = settings();
    if !ENABLED.load(Ordering::Relaxed) || !st.weather || st.op_snow <= 0.0 {
        return;
    }
    with(|s| {
        let rng = &mut s.env.rng;
        let v = v3(rng.range(-0.3, 0.3), -rng.range(0.6, 1.0), rng.range(-0.3, 0.3));
        let mut puff = Puff::new(FxKind::Dust, v3(at[0], at[1], at[2]), v, rng.range(0.02, 0.035), rng.range(4.0, 6.0), [0.97, 0.98, 1.0], [0.93, 0.95, 1.0], rng);
        puff.alpha *= f64::from(st.op_snow);
        s.env.puff(puff);
    });
}

/// Sand blown off the ground at `at` by the wind (deserts and beaches in wind).
pub fn sand_dust(at: [f64; 3], red: bool) {
    let st = settings();
    if !ENABLED.load(Ordering::Relaxed) || !st.weather || st.op_dust <= 0.0 {
        return;
    }
    let w = wind_at(with(|s| s.env.clock), st.wind());
    with(|s| {
        let rng = &mut s.env.rng;
        let v = w * rng.range(0.6, 1.1) + v3(0.0, rng.range(0.6, 1.4), 0.0);
        let (a, b) = if red { ([0.72, 0.42, 0.22], [0.8, 0.55, 0.35]) } else { ([0.86, 0.8, 0.6], [0.92, 0.88, 0.72]) };
        let mut puff = Puff::new(FxKind::Dust, v3(at[0], at[1] + 0.05, at[2]), v, rng.range(0.03, 0.06), rng.range(1.2, 2.2), a, b, rng);
        puff.alpha *= 0.7 * f64::from(st.op_dust);
        s.env.puff(puff);
    });
}

/// The wind's strength now (0 with Wind Forces off).
pub fn wind_now() -> f64 {
    settings().wind()
}

pub fn weather_on() -> bool {
    ENABLED.load(Ordering::Relaxed) && settings().weather
}

/// Called by the block-breaking hook.
pub fn spawn_block(pos: [i32; 3], boxes: &[[f64; 6]], uv: [f32; 4], tint: u32) {
    let st = settings();
    if !st.blocks {
        return;
    }
    with(|s| {
        let c = v3(f64::from(pos[0]) + 0.5, f64::from(pos[1]) + 0.5, f64::from(pos[2]) + 0.5);
        if (c - s.camera).len() > f64::from(st.activation) {
            return;
        }
        s.physics.spawn_block(pos, boxes, uv, tint, &st);
    });
}

/// Set while the collapse hook breaks blocks itself (its own nested call).
pub static COLLAPSE: Guard = Guard::new();

/// Collapses waiting their ticks (Collapse Speed): the broken block, and when it falls.
static WAITING: Mutex<Vec<([i32; 3], std::time::Instant)>> = Mutex::new(Vec::new());
static REPLAYING: Mutex<Option<[i32; 3]>> = Mutex::new(None);

/// What the block at `pos` held up falls in `seconds`.
pub fn queue_collapse(pos: [i32; 3], seconds: f64) {
    let mut w = WAITING.lock().unwrap_or_else(|e| e.into_inner());
    if w.len() < 64 && !w.iter().any(|(p, _)| *p == pos) {
        w.push((pos, std::time::Instant::now() + std::time::Duration::from_secs_f64(seconds.max(0.0))));
    }
}

/// The collapses whose time came.
pub fn due_collapses() -> Vec<[i32; 3]> {
    let now = std::time::Instant::now();
    let mut w = WAITING.lock().unwrap_or_else(|e| e.into_inner());
    let due: Vec<[i32; 3]> = w.iter().filter(|(_, t)| *t <= now).map(|(p, _)| *p).collect();
    w.retain(|(_, t)| *t > now);
    due
}

/// The collapse hook is called for `pos` to let its waiting collapse fall.
pub fn set_collapse_replay(pos: [i32; 3]) {
    *REPLAYING.lock().unwrap_or_else(|e| e.into_inner()) = Some(pos);
}

/// True once when the call for `pos` is a waiting collapse's.
pub fn take_collapse_replay(pos: [i32; 3]) -> bool {
    let mut g = REPLAYING.lock().unwrap_or_else(|e| e.into_inner());
    if *g == Some(pos) {
        *g = None;
        true
    } else {
        false
    }
}
static SILENT: Mutex<Vec<[i32; 3]>> = Mutex::new(Vec::new());

/// Blocks the collapse takes away: their breaking makes no particles (they fall whole instead).
pub fn silence(pos: [i32; 3]) {
    if let Ok(mut v) = SILENT.lock() {
        v.push(pos);
        if v.len() > 1024 {
            v.drain(..512);
        }
    }
}

/// True once for a block the collapse took away.
pub fn take_silenced(pos: [i32; 3]) -> bool {
    SILENT.lock().is_ok_and(|mut v| match v.iter().position(|p| *p == pos) {
        Some(i) => {
            v.swap_remove(i);
            true
        }
        None => false,
    })
}

/// The blocks that lost their support when the block at `foot` was broken, falling over away
/// from `push` (a horizontal direction).
pub fn collapse(blocks: &[Falling], foot: [i32; 3], push: [f64; 2]) {
    let st = settings();
    if !st.collapse || blocks.is_empty() {
        return;
    }
    let len = (push[0] * push[0] + push[1] * push[1]).sqrt();
    let dir = if len > 1e-6 { v3(push[0] / len, 0.0, push[1] / len) } else { v3(1.0, 0.0, 0.0) };
    let foot = v3(f64::from(foot[0]) + 0.5, f64::from(foot[1]), f64::from(foot[2]) + 0.5);
    with(|s| {
        s.physics.spawn_falling(blocks, foot, dir, &st);
        if st.collapse_dust {
            s.env.collapse_dust(blocks);
        }
    });
}

/// What a block is to a collapse, by its id: 1 the ground (it holds up what stands on it), 2
/// leaves (they hold nothing up), 3 anything else that can fall.
pub fn collapse_kind(name: &str) -> u8 {
    const GROUND: &[&str] = &[
        "minecraft:stone", "minecraft:deepslate", "minecraft:granite", "minecraft:diorite", "minecraft:andesite",
        "minecraft:tuff", "minecraft:calcite", "minecraft:dirt", "minecraft:coarse_dirt", "minecraft:rooted_dirt",
        "minecraft:grass_block", "minecraft:podzol", "minecraft:mycelium", "minecraft:farmland", "minecraft:dirt_path",
        "minecraft:mud", "minecraft:clay", "minecraft:sand", "minecraft:red_sand", "minecraft:gravel",
        "minecraft:sandstone", "minecraft:red_sandstone", "minecraft:terracotta", "minecraft:snow_block",
        "minecraft:ice", "minecraft:packed_ice", "minecraft:blue_ice", "minecraft:netherrack", "minecraft:soul_sand",
        "minecraft:soul_soil", "minecraft:basalt", "minecraft:blackstone", "minecraft:end_stone", "minecraft:obsidian",
        "minecraft:moss_block", "minecraft:pale_moss_block", "minecraft:magma_block", "minecraft:dripstone_block",
        "minecraft:smooth_basalt", "minecraft:crimson_nylium", "minecraft:warped_nylium", "minecraft:bedrock",
        "minecraft:powder_snow", "minecraft:suspicious_sand", "minecraft:suspicious_gravel",
    ];
    if name.ends_with("_leaves") {
        return 2;
    }
    if GROUND.contains(&name) || name.ends_with("_ore") || name.ends_with("_terracotta") || name.ends_with("_concrete_powder") {
        return 1;
    }
    3
}

/// Whether a block id is a log or a stem (what leaves hang from).
pub fn is_log(name: &str) -> bool {
    name.ends_with("_log") || name.ends_with("_wood") || name.ends_with("_stem") || name.ends_with("_hyphae")
}

/// Whether mob deaths become pieces at all.
pub fn deaths_on() -> bool {
    ENABLED.load(Ordering::Relaxed) && settings().deaths != 0
}

/// True once per mob: the first frame its death is seen.
pub fn first_death(id: u32) -> bool {
    with(|s| {
        if s.dead.contains_key(&id) {
            return false;
        }
        let now = s.physics.time;
        s.dead.insert(id, now);
        // Forget old deaths (ids are not reused within a session, but keep the map small).
        if s.dead.len() > 512 {
            s.dead.retain(|_, t| now - *t < 60.0);
        }
        true
    })
}

/// Called by the mob hook with the dying mob's vertices.
pub fn spawn_mob(verts: &[Vertex], marks: &[usize], start: usize, camera: [f64; 3], velocity: [f64; 3]) {
    let st = settings();
    if st.deaths == 0 {
        return;
    }
    let marks: Vec<usize> = marks.iter().filter_map(|m| m.checked_sub(start)).collect();
    let cam = v3(camera[0], camera[1], camera[2]);
    let vel = v3(velocity[0], velocity[1], velocity[2]);
    with(|s| {
        let before = s.physics.objects.len();
        let life = f64::from(st.mob_life) * 10.0 + s.env.rng.range(0.0, f64::from(st.mob_var)).max(0.0);
        s.physics.spawn_mob(verts, &marks, cam, vel, st.deaths, life.max(1.0));
        // A mob killed by a blast is thrown by it.
        if s.physics.objects.len() > before {
            let now = s.env.clock;
            let blasts: Vec<Blast> = s.env.blasts.iter().filter(|b| now - b.at < 0.6).copied().collect();
            for b in blasts {
                s.physics.blast(b.center, b.radius * 2.0, 0.6, before);
            }
        }
    });
}

/// Once a frame: step the pieces and draw them. `dim` tells dimensions apart; a change, or a big
/// jump of the camera (a teleport, another world), clears them. `white` is a plain white texel of
/// the atlas, for smoke, sparks and footprints.
static PLAYER: Mutex<Option<(V3, V3)>> = Mutex::new(None);

/// The player this frame: feet and velocity (blocks a second), or None (spectating, no player).
/// Ragdolls and pieces they walk into are pushed away.
pub fn set_player(player: Option<([f64; 3], [f64; 3])>) {
    if let Ok(mut p) = PLAYER.lock() {
        *p = player.map(|(f, v)| (v3(f[0], f[1], f[2]), v3(v[0], v[1], v[2])));
    }
}

pub fn frame(dim: u8, camera: [f64; 3], white: [f32; 4], world: &dyn World, out: &mut Vec<Vertex>) {
    let st = settings();
    with(|s| {
        let cam = v3(camera[0], camera[1], camera[2]);
        if s.dim != dim || (cam - s.camera).len() > 300.0 {
            s.physics.clear();
            s.dead.clear();
            s.items.clear();
            s.env.clear();
            s.dim = dim;
        }
        s.camera = cam;
        let now = std::time::Instant::now();
        let dt = s.clock.map_or(0.0, |c| now.duration_since(c).as_secs_f64());
        s.clock = Some(now);
        s.last_dt = dt.clamp(0.0, 0.1);
        s.frame += 1;
        let frame = s.frame;
        s.items.retain(|_, (_, seen)| frame - *seen < 120);
        blow_pending(s, &st);
        if let Some((feet, vel)) = PLAYER.lock().ok().and_then(|p| *p) {
            s.physics.shove(feet, vel);
        }
        s.physics.max_bodies = st.max_objects as usize;
        s.physics.joint_break = (st.joint_force < 9.99).then(|| 0.08 + 0.3 * f64::from(st.joint_force));
        s.physics.anim = anim_now(&st);
        s.physics.ragdoll_limit = st.ragdoll_limit as usize;
        s.physics.substeps = st.substeps();
        s.physics.resolution = st.resolution_factor();
        s.physics.ragdoll_fix = st.ragdoll_fix;
        s.physics.step(dt * f64::from(st.sim_speed), st.gravity(), world);
        // A torn joint lets out a little red mist.
        for at in std::mem::take(&mut s.physics.torn) {
            let n = (st.joint_blood * 6.0).round() as usize;
            for _ in 0..n {
                let rng = &mut s.env.rng;
                let r = rng.unit();
                let puff = Puff::new(FxKind::Dust, at, r * 1.2 + v3(0.0, 0.6, 0.0), rng.range(0.03, 0.06), rng.range(0.5, 0.9), [0.62, 0.05, 0.04], [0.4, 0.02, 0.02], rng);
                s.env.puff(puff);
            }
        }
        // What the despawn animation gives off where pieces went.
        let anim = s.physics.anim;
        for (at, size) in std::mem::take(&mut s.physics.gone) {
            s.env.despawn_puffs(&anim, at, size);
        }
        // Landings, for the game to play (the builtin hook has the audio).
        let impacts = std::mem::take(&mut s.physics.impacts);
        if st.impact_volume > 0.0 {
            let mut q = SOUNDS.lock().unwrap_or_else(|e| e.into_inner());
            for (at, speed, mob) in impacts.into_iter().take(3) {
                if (at - cam).len() > 32.0 || q.len() >= 16 {
                    continue;
                }
                let volume = st.impact_volume * ((speed - 2.5) / 8.0).clamp(0.08, 0.6) as f32;
                let pitch = s.env.rng.range(0.8, 1.25) as f32;
                let sound = if mob { "minecraft:entity.generic.small_fall" } else { "minecraft:block.stone.hit" };
                q.push((sound, [at.x, at.y, at.z], volume, pitch));
            }
        }
        s.physics.emit(cam, world, out);
        s.env.step(s.last_dt, cam, frame, &st, world);
        s.env.emit(cam, white, world, out);
    });
}

/// The player hit something: a piece in the way, from the eye along the look (yaw and pitch in
/// degrees), is knocked away. True when one was.
pub fn punch(eye: [f64; 3], yaw: f32, pitch: f32) -> bool {
    let st = settings();
    if !ENABLED.load(Ordering::Relaxed) || !st.punch {
        return false;
    }
    let (y, p) = (f64::from(yaw).to_radians(), f64::from(pitch).to_radians());
    let dir = v3(-y.sin() * p.cos(), -p.sin(), y.cos() * p.cos());
    let hit = with(|s| s.physics.punch(v3(eye[0], eye[1], eye[2]), dir, 3.0, 6.0));
    if hit && st.impact_volume > 0.0 {
        let mut q = SOUNDS.lock().unwrap_or_else(|e| e.into_inner());
        q.push(("minecraft:entity.player.attack.weak", eye, 0.6 * st.impact_volume, 1.0));
    }
    hit
}

/// Rain landing on water at `at` (its surface): a ring spreading on the water and a few drops.
pub fn rain_ripple(at: [f64; 3]) {
    let st = settings();
    if !ENABLED.load(Ordering::Relaxed) || !st.weather || st.op_rain <= 0.0 {
        return;
    }
    with(|s| {
        let x = v3(at[0], at[1] + 0.01, at[2]);
        let rng = &mut s.env.rng;
        let mut ring = Puff::new(FxKind::Foam, x, V3::default(), rng.range(0.06, 0.1), rng.range(0.45, 0.7), [0.8, 0.86, 0.95], [0.7, 0.78, 0.9], rng);
        ring.alpha *= 0.55 * f64::from(st.op_rain);
        s.env.puff(ring);
        for _ in 0..2 {
            let rng = &mut s.env.rng;
            let r = rng.unit();
            let v = v3(r.x * 0.4, rng.range(0.8, 1.6), r.z * 0.4);
            let mut d = Puff::new(FxKind::Dust, x, v, rng.range(0.01, 0.018), rng.range(0.2, 0.3), [0.62, 0.72, 0.95], [0.75, 0.82, 0.97], rng);
            d.alpha *= f64::from(st.op_rain);
            s.env.puff(d);
        }
    });
}

/// Sounds waiting for the game: (event, position, volume, pitch).
static SOUNDS: Mutex<Vec<(&'static str, [f64; 3], f32, f32)>> = Mutex::new(Vec::new());

/// The sounds the pieces made since the last call.
pub fn take_sounds() -> Vec<(&'static str, [f64; 3], f32, f32)> {
    std::mem::take(&mut *SOUNDS.lock().unwrap_or_else(|e| e.into_inner()))
}

static WIND_NEXT: Mutex<Option<std::time::Instant>> = Mutex::new(None);

/// The wind's sound: every ten seconds or so, a gust (volume, pitch) while it blows and the
/// player is out under the sky (`outdoors`). None when it is not time.
pub fn wind_sound(outdoors: bool) -> Option<(f32, f32)> {
    let st = settings();
    if !ENABLED.load(Ordering::Relaxed) || st.wind_volume <= 0.0 || !outdoors {
        return None;
    }
    let w = st.wind() as f32;
    if w < 0.35 {
        return None;
    }
    let now = std::time::Instant::now();
    let mut g = WIND_NEXT.lock().unwrap_or_else(|e| e.into_inner());
    if g.is_some_and(|t| now < t) {
        return None;
    }
    let (wait, pitch) = with(|s| (s.env.rng.range(7.0, 13.0), s.env.rng.range(0.55, 0.75) as f32));
    *g = Some(now + std::time::Duration::from_secs_f64(wait / f64::from(w.max(0.5))));
    Some(((0.05 + 0.06 * w).min(0.25) * st.wind_volume, pitch))
}

static REPLAY: AtomicBool = AtomicBool::new(false);

/// The Despawn Animation page's REPLAY: a few pieces fall in front of the player and go with
/// the animation chosen.
pub fn replay_anim() {
    REPLAY.store(true, Ordering::Relaxed);
}

pub fn replay_wanted() -> bool {
    REPLAY.load(Ordering::Relaxed)
}

/// Makes the replay's pieces: in front of the player at `eye`, facing `yaw`, of sprite `uv`.
pub fn replay_at(eye: [f64; 3], yaw: f32, uv: [f32; 4]) {
    if !REPLAY.swap(false, Ordering::Relaxed) || !ENABLED.load(Ordering::Relaxed) {
        return;
    }
    let st = settings();
    let a = anim_now(&st);
    with(|s| {
        let r = f64::from(yaw).to_radians();
        let ahead = v3(eye[0], eye[1], eye[2]) + v3(-r.sin(), 0.0, r.cos()) * 2.5;
        let life = 1.0 + f64::from(a.duration);
        for _ in 0..6 {
            let rng = s.physics.rng();
            let size = rng.range(0.15, 0.3);
            let at = ahead + v3(rng.range(-0.5, 0.5), rng.range(-0.2, 0.4), rng.range(-0.5, 0.5));
            let (_, h, verts) = box_mesh([0.0, 0.0, 0.0, size, size, size], uv, tint_color(0xFF_FFFF));
            let mut body = Body::new(at, Quat::axis_angle(rng.unit(), rng.range(0.0, 3.0)), h, verts);
            body.v = v3(rng.range(-0.6, 0.6), rng.range(0.5, 1.5), rng.range(-0.6, 0.6));
            body.w = rng.unit() * rng.range(1.0, 5.0);
            s.physics.add(vec![body], Vec::new(), life, None);
        }
    });
}

impl Env {
    /// The puffs of despawn animation `a` where pieces `size` big went, at `at`.
    pub fn despawn_puffs(&mut self, a: &AnimDef, at: V3, size: f64) {
        let n = ((f64::from(a.amount) * (0.5 + size).min(1.5)).round() as usize).min(40);
        for _ in 0..n {
            let rng = &mut self.rng;
            let r = rng.unit();
            let x = at + r * (0.15 + size * 0.3);
            match a.particle {
                1 => {
                    let v = r * rng.range(0.3, 0.9) + v3(0.0, rng.range(0.3, 1.0), 0.0);
                    let p = Puff::new(FxKind::Ember, x, v, rng.range(0.02, 0.04), rng.range(0.6, 1.2), [0.75, 0.35, 1.0], [0.35, 0.08, 0.6], rng);
                    self.puff(p);
                }
                2 => {
                    let v = v3(r.x * rng.range(0.6, 1.6), rng.range(1.5, 3.0), r.z * rng.range(0.6, 1.6));
                    let life = rng.range(0.8, 1.4);
                    self.add_drop(x, v, false, life);
                }
                3 => {
                    let v = r * 0.3 + v3(0.0, rng.range(0.4, 0.9), 0.0);
                    let mut p = Puff::new(FxKind::Smoke, x, v, rng.range(0.12, 0.2), rng.range(1.2, 2.2), [0.3, 0.3, 0.3], [0.6, 0.6, 0.62], rng);
                    p.alpha = 0.6;
                    self.puff(p);
                }
                4 => {
                    let v = r * rng.range(0.6, 1.4) + v3(0.0, 0.3, 0.0);
                    let mut p = Puff::new(FxKind::Smoke, x, v, rng.range(0.07, 0.12), rng.range(0.4, 0.8), [0.96, 0.96, 0.96], [0.85, 0.85, 0.86], rng);
                    p.alpha = 0.8;
                    self.puff(p);
                }
                5 => {
                    let v = r * rng.range(0.5, 1.5);
                    let flame = Puff::new(FxKind::Flame, x, v, rng.range(0.12, 0.22), rng.range(0.25, 0.45), [1.0, 0.85, 0.4], [0.7, 0.2, 0.05], rng);
                    let v = r * 0.4 + v3(0.0, 0.6, 0.0);
                    let mut smoke = Puff::new(FxKind::Smoke, x, v, rng.range(0.15, 0.25), rng.range(1.0, 1.8), [0.2, 0.2, 0.2], [0.5, 0.5, 0.5], rng);
                    smoke.alpha = 0.55;
                    self.puff(flame);
                    self.puff(smoke);
                }
                _ => {}
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Item physics
// ---------------------------------------------------------------------------------------------

pub fn items_on() -> bool {
    ENABLED.load(Ordering::Relaxed) && settings().items
}

/// The orientation (x, y, z, w) of dropped item `id` this frame. In the air it tumbles the way it
/// flies; on the ground it settles on its flattest side: flat items (`flat`) lie on their face,
/// blocks on whichever side is closest to down.
pub fn item_rotation(id: u32, grounded: bool, velocity: [f64; 3], flat: bool) -> [f32; 4] {
    with(|s| {
        let dt = s.last_dt;
        let frame = s.frame;
        let start = {
            let mut r = Rng::new(u64::from(id) ^ 0xA5A5);
            let yaw = Quat::axis_angle(v3(0.0, 1.0, 0.0), r.range(0.0, std::f64::consts::TAU));
            if flat { yaw.mul(Quat::axis_angle(v3(1.0, 0.0, 0.0), -std::f64::consts::FRAC_PI_2)) } else { yaw }
        };
        let entry = s.items.entry(id).or_insert((start, frame));
        entry.1 = frame;
        let vel = v3(velocity[0], velocity[1], velocity[2]);
        let mut q = entry.0;
        if !grounded {
            let flat_v = v3(vel.x, 0.0, vel.z);
            let axis = if flat_v.len() > 0.05 { v3(0.0, 1.0, 0.0).cross(flat_v).norm() } else { v3(1.0, 0.0, 0.3).norm() };
            let rate = (3.0 + 6.0 * vel.len().min(1.5)) * f64::from(settings().item_spin);
            q = Quat::axis_angle(axis, rate * dt).mul(q).normalize();
        } else {
            // The local axis closest to straight up (only the face axis for flat items) is turned
            // the rest of the way, and the item eases into that.
            let up = v3(0.0, 1.0, 0.0);
            let axes: &[usize] = if flat { &[2] } else { &[0, 1, 2] };
            let mut best = (up, -2.0);
            for &k in axes {
                for sgn in [-1.0, 1.0] {
                    let a = q.rotate(V3::axis(k, sgn));
                    if a.y > best.1 {
                        best = (a, a.y);
                    }
                }
            }
            let target = Quat::between(best.0, up).mul(q).normalize();
            q = q.slerp(target, (dt * 12.0).min(1.0)).normalize();
        }
        entry.0 = q;
        [q.x as f32, q.y as f32, q.z as f32, q.w as f32]
    })
}


// ---------------------------------------------------------------------------------------------
// Explosions, smoke, footprints, ropes and capes
// ---------------------------------------------------------------------------------------------

/// Full block and sky light, for things that glow.
const GLOW: u32 = 240 | 240 << 8;

/// An entity vertex colour (ABGR bytes) from 0..1 RGB.
pub fn mix3(a: [f64; 3], b: [f64; 3], t: f64) -> [f64; 3] {
    let t = t.clamp(0.0, 1.0);
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
}

/// The wind at time `t` (seconds): a direction that wanders slowly and gusts, in blocks per
/// second, times `strength`.
pub fn wind_at(t: f64, strength: f64) -> V3 {
    if strength <= 0.0 {
        return V3::default();
    }
    let angle = 0.7 + t * 0.013 + 0.6 * (t * 0.071).sin();
    let gust = 0.65 + 0.25 * (t * 0.53).sin() + 0.1 * (t * 1.7 + 1.3).sin();
    v3(angle.cos(), 0.0, angle.sin()) * (1.6 * gust * strength)
}

fn solid(world: &dyn World, p: V3) -> bool {
    match world.cell(floor(p.x), floor(p.y), floor(p.z)) {
        Cell::Full => true,
        Cell::Boxes(b) => {
            let l = v3(p.x - p.x.floor(), p.y - p.y.floor(), p.z - p.z.floor());
            b.iter().any(|b| (0..3).all(|k| l.get(k) >= b[k] && l.get(k) < b[k + 3]))
        }
        _ => false,
    }
}

/// A soft, round puff in the picture: opaque in the middle, clear at its uneven rim, facing the
/// camera (or lying flat), maybe drawn out along a direction.
pub struct Shape {
    /// Camera-relative centre.
    pub at: V3,
    pub radius: f64,
    pub roll: f64,
    pub seed: u32,
    pub rgb: [f64; 3],
    /// The middle's colour, when it differs from the rim's.
    pub center_rgb: Option<[f64; 3]>,
    pub alpha: f64,
    /// How much of the opacity the middle keeps against the ring halfway out (1: as much).
    pub core: f64,
    pub light: u32,
    /// Lies flat (a print, froth), instead of facing the camera.
    pub flat: bool,
    /// Drawn out along a direction, by a factor.
    pub stretch: Option<(V3, f64)>,
    pub segments: usize,
    /// Its rim's shape instead of a round one: points (along `u`, along `w`), times the radius.
    pub outline: Option<&'static [(f64, f64)]>,
    /// How opaque the rim is against the ring halfway out (0: it fades to clear).
    pub edge: f64,
}

/// A boot's sole, toe along +u: 20 points around it, the length 1 (toe at 0.5, heel at -0.5).
/// The ball is widest, the arch narrow, the heel round.
static SOLE: [(f64, f64); 20] = [
    (0.5, 0.0), (0.47, 0.15), (0.4, 0.245), (0.3, 0.285), (0.18, 0.28), (0.06, 0.245),
    (-0.06, 0.195), (-0.18, 0.19), (-0.3, 0.21), (-0.42, 0.18), (-0.5, 0.0),
    (-0.42, -0.18), (-0.3, -0.21), (-0.18, -0.19), (-0.06, -0.18), (0.06, -0.225),
    (0.18, -0.265), (0.3, -0.27), (0.4, -0.23), (0.47, -0.135),
];

/// `gfx::TRANSLUCENT_PARTICLE_OVERLAY`: quads of vanilla's translucent particle layer.
const TRANSLUCENT_PARTICLE: u32 = 15;

fn hash01(seed: u32, i: u32) -> f64 {
    let mut x = seed ^ i.wrapping_mul(0x9E37_79B9);
    x ^= x >> 16;
    x = x.wrapping_mul(0x7FEB_352D);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846C_A68B);
    x ^= x >> 16;
    f64::from(x) / f64::from(u32::MAX)
}

fn rgba(c: [f64; 3], a: f64) -> u32 {
    let b = |x: f64| (x.clamp(0.0, 1.0) * 255.0).round() as u32;
    b(a) << 24 | b(c[2]) << 16 | b(c[1]) << 8 | b(c[0])
}

impl Shape {
    pub fn new(at: V3, radius: f64, roll: f64, seed: u32, rgb: [f64; 3], alpha: f64, light: u32) -> Shape {
        Shape { at, radius, roll, seed, rgb, center_rgb: None, alpha, core: 1.0, light, flat: false, stretch: None, segments: 10, outline: None, edge: 0.0 }
    }

    /// The puff as a fan of quads: the middle, a ring halfway out, the rim (clear).
    pub fn draw(&self, white: [f32; 4], out: &mut Vec<Vertex>) {
        let uv = [(white[0] + white[2]) * 0.5, (white[1] + white[3]) * 0.5];
        let n = if self.flat {
            v3(0.0, 1.0, 0.0)
        } else {
            let l = self.at.len();
            if l < 1e-6 {
                return;
            }
            self.at * (-1.0 / l)
        };
        let helper = if n.y.abs() > 0.95 { v3(1.0, 0.0, 0.0) } else { v3(0.0, 1.0, 0.0) };
        let mut u = n.cross(helper).norm();
        let mut w = n.cross(u).norm();
        let (sr, cr) = self.roll.sin_cos();
        (u, w) = (u * cr + w * sr, w * cr - u * sr);
        let mut su = 1.0;
        if let Some((dir, k)) = self.stretch {
            let d = dir - n * dir.dot(n);
            if d.len() > 1e-6 {
                u = d.norm();
                w = n.cross(u).norm();
                su = k;
            }
        }
        let mid_rgb = self.rgb;
        let center_rgb = self.center_rgb.unwrap_or(self.rgb);
        let center = rgba(center_rgb, self.alpha * self.core);
        let mid = rgba(mix3(center_rgb, mid_rgb, 0.6), self.alpha * 0.82);
        let rim = rgba(mid_rgb, self.alpha * 0.82 * self.edge);
        let segs = self.outline.map_or(self.segments.max(4), <[(f64, f64)]>::len);
        let ring: Vec<(V3, V3)> = (0..segs)
            .map(|i| {
                if let Some(o) = self.outline {
                    let (a, b) = o[i];
                    let dir = u * (a * su) + w * b;
                    return (self.at + dir * (self.radius * 0.55), self.at + dir * self.radius);
                }
                let a = std::f64::consts::TAU * i as f64 / segs as f64;
                let r = self.radius * (0.8 + 0.2 * hash01(self.seed, i as u32));
                let dir = u * (a.cos() * su) + w * a.sin();
                (self.at + dir * (r * 0.55), self.at + dir * r)
            })
            .collect();
        let mut put = |p: V3, color: u32| {
            out.push(Vertex {
                pos: [p.x as f32, p.y as f32, p.z as f32],
                uv,
                // Horizon's translucent particle layer: drawn after water and glass, so the
                // puffs never hide them.
                layer: TRANSLUCENT_PARTICLE,
                color,
                light: self.light,
                normal: [0; 4],
                glint_uv: [0.0, 0.0],
            });
        };
        for i in 0..segs {
            let (m0, r0) = ring[i];
            let (m1, r1) = ring[(i + 1) % segs];
            // The middle: a triangle (its last corner twice).
            put(self.at, center);
            put(m0, mid);
            put(m1, mid);
            put(m1, mid);
            // The ring out to the rim.
            put(m0, mid);
            put(r0, rim);
            put(r1, rim);
            put(m1, mid);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FxKind {
    /// Rises on its warmth, swirls, drifts with the wind, spreads under ceilings and thins out.
    Smoke,
    /// Falls and bounces, glowing, drawn as a streak along its flight.
    Spark,
    /// A glowing ball of fire that swells and dies away quickly.
    Flame,
    /// A glowing speck from a fire, carried up by the hot air.
    Ember,
    /// A powder of snow kicked up, falling back slowly.
    Dust,
    /// White froth lying on the water where a splash came down.
    Foam,
}

#[derive(Clone, Copy, Debug)]
pub struct Puff {
    pub x: V3,
    pub v: V3,
    pub size: f64,
    pub age: f64,
    pub life: f64,
    pub from: [f64; 3],
    pub to: [f64; 3],
    pub kind: FxKind,
    /// Its turn in the picture, and how fast that turns.
    roll: f64,
    spin: f64,
    seed: u32,
    /// How opaque it starts.
    pub alpha: f64,
    /// Seconds it has spent under a ceiling (smoke there lingers).
    held: f64,
    /// Seconds before it shows (a fireball's smoke comes after the fire).
    delay: f64,
}

const MAX_PUFFS: usize = 600;

impl Puff {
    fn new(kind: FxKind, x: V3, v: V3, size: f64, life: f64, from: [f64; 3], to: [f64; 3], rng: &mut Rng) -> Puff {
        let alpha = match kind {
            FxKind::Smoke => 0.55,
            FxKind::Flame => 0.9,
            FxKind::Spark | FxKind::Ember => 1.0,
            FxKind::Dust => 0.6,
            FxKind::Foam => 0.6,
        };
        Puff {
            x,
            v,
            size,
            age: 0.0,
            life,
            from,
            to,
            kind,
            roll: rng.range(0.0, std::f64::consts::TAU),
            spin: rng.range(-0.5, 0.5),
            seed: (rng.next() * 4_294_967_295.0) as u32,
            alpha,
            held: 0.0,
            delay: 0.0,
        }
    }

    /// How big the puff is now, against its starting size.
    pub fn scale_pub(&self) -> f64 {
        self.scale()
    }

    fn scale(&self) -> f64 {
        let t = (self.age / self.life).clamp(0.0, 1.0);
        match self.kind {
            // Smoke keeps growing as it mixes with the air.
            FxKind::Smoke => 0.55 + 2.4 * (1.0 - (-self.age * 0.45).exp()),
            FxKind::Spark => 1.0 - 0.5 * t * t,
            FxKind::Flame => (0.55 + 0.9 * (t * 4.0).min(1.0)) * ((1.0 - t) / 0.4).min(1.0).max(0.3),
            FxKind::Ember => 1.0 - 0.6 * t,
            FxKind::Dust => 0.7 + 1.6 * t,
            FxKind::Foam => 0.5 + 1.2 * t.sqrt(),
        }
    }

    /// How opaque it is now (0 gone).
    fn opacity(&self) -> f64 {
        let t = (self.age / self.life).clamp(0.0, 1.0);
        let fade_in = |s: f64| (self.age / s).clamp(0.0, 1.0);
        self.alpha
            * match self.kind {
                FxKind::Smoke => fade_in(0.5) * (1.0 - t).powf(0.9),
                FxKind::Flame => fade_in(0.05) * (1.0 - t).powf(0.8),
                FxKind::Spark => ((1.0 - t) / 0.25).min(1.0),
                FxKind::Ember => {
                    let flicker = 0.7 + 0.3 * (self.age * 23.0 + f64::from(self.seed % 100)).sin();
                    flicker * ((1.0 - t) / 0.3).min(1.0)
                }
                FxKind::Dust => fade_in(0.08) * (1.0 - t).powf(1.2),
                FxKind::Foam => fade_in(0.1) * (1.0 - t),
            }
    }
}

/// An explosion seen in the last moments.
#[derive(Clone, Copy, Debug)]
pub struct Blast {
    pub center: V3,
    pub radius: f64,
    pub at: f64,
}

/// A block that has just turned to air (from the server), in case an explosion explains it.
#[derive(Clone, Debug)]
pub struct Aired {
    pub pos: [i32; 3],
    pub boxes: Vec<[f64; 6]>,
    pub uv: [f32; 4],
    pub tint: u32,
    pub at: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct Print {
    pub x: V3,
    pub yaw: f64,
    pub age: f64,
}

#[derive(Clone, Debug)]
pub struct Rope {
    pub pts: Vec<V3>,
    prev: Vec<V3>,
    seen: u64,
    group: u8,
}

#[derive(Clone, Debug)]
pub struct Cloth {
    pub pts: Vec<V3>,
    prev: Vec<V3>,
    seen: u64,
    width: f64,
    height: f64,
}

/// The cloth grid of a cape: points across and down.
const CAPE_W: usize = 6;
const CAPE_H: usize = 11;

/// Everything that is not a rigid piece.
pub struct Env {
    pub puffs: Vec<Puff>,
    pub blasts: Vec<Blast>,
    aired: Vec<Aired>,
    /// Fire, campfires and lava near the camera, by position, with their kind.
    pub emitters: HashMap<[i32; 3], u8>,
    scan: i32,
    pub prints: Vec<Print>,
    last_step: Option<V3>,
    left: bool,
    pub ropes: Vec<Rope>,
    pub capes: HashMap<u32, Cloth>,
    /// Seconds since the start, for the wind.
    pub clock: f64,
    rng: Rng,
    /// Explosions waiting for the next frame's world (their pieces need the blocks).
    pending: Vec<(Blast, Vec<Aired>)>,
    frame: u64,
    camera: V3,
    /// Liquid droplets.
    pub drops: Vec<Drop>,
    /// Whether the player's feet were in water last frame.
    was_in_water: bool,
    /// The explosion being made is under water: no fire, no smoke.
    wet: bool,
}

impl Default for Env {
    fn default() -> Self {
        Env {
            puffs: Vec::new(),
            blasts: Vec::new(),
            aired: Vec::new(),
            emitters: HashMap::new(),
            scan: 0,
            prints: Vec::new(),
            last_step: None,
            left: false,
            ropes: Vec::new(),
            capes: HashMap::new(),
            clock: 0.0,
            rng: Rng::new(0xF1CE),
            pending: Vec::new(),
            frame: 0,
            camera: V3::default(),
            drops: Vec::new(),
            was_in_water: false,
            wet: false,
        }
    }
}

impl Env {
    pub fn clear(&mut self) {
        self.puffs.clear();
        self.blasts.clear();
        self.aired.clear();
        self.emitters.clear();
        self.prints.clear();
        self.last_step = None;
        self.ropes.clear();
        self.capes.clear();
        self.pending.clear();
        self.drops.clear();
    }

    fn puff(&mut self, p: Puff) {
        if self.puffs.len() >= MAX_PUFFS {
            // The oldest smoke makes room.
            if let Some(i) = self.puffs.iter().position(|q| q.kind == FxKind::Smoke) {
                self.puffs.swap_remove(i);
            } else {
                return;
            }
        }
        self.puffs.push(p);
    }

    /// An explosion at `center`: the fireball, sparks and smoke, and the blocks it removed (those
    /// that already turned to air, and those that will within a moment) are blown away.
    pub fn explode(&mut self, center: V3, radius: f64) {
        let r = radius.clamp(0.5, 12.0);
        let wet = self.wet;
        let rng = &mut self.rng;
        let mut new = Vec::new();
        // The fireball: white hot in the middle, orange, then dark red as it burns out.
        let flames = if wet { 0 } else { (10.0 + r * 4.0) as usize };
        for _ in 0..flames {
            let d = rng.unit();
            let x = center + d * rng.range(0.0, r * 0.4);
            let v = d * rng.range(1.0, 2.5 + r * 0.5) + v3(0.0, rng.range(0.5, 2.0), 0.0);
            let hot = [1.0, rng.range(0.72, 0.85), rng.range(0.28, 0.42)];
            let size = rng.range(0.35, 0.6) * (0.7 + r * 0.18);
            new.push(Puff::new(FxKind::Flame, x, v, size, rng.range(0.45, 0.85), hot, [0.62, 0.1, 0.02], rng));
        }
        let sparks = if wet { 0 } else { (16.0 + r * 10.0) as usize };
        for _ in 0..sparks {
            let d = rng.unit();
            let v = d * rng.range(5.0, 9.0 + r * 2.0) + v3(0.0, rng.range(2.0, 6.0), 0.0);
            new.push(Puff::new(FxKind::Spark, center + d * 0.3, v, rng.range(0.02, 0.035), rng.range(0.8, 2.2), [1.0, 0.92, 0.6], [1.0, 0.35, 0.05], rng));
        }
        // The smoke after it: dark and thick, rolling up into a column.
        let smoke = if wet { 0 } else { (14.0 + r * 6.0) as usize };
        for _ in 0..smoke {
            let d = rng.unit();
            let x = center + d * rng.range(0.0, r * 0.6);
            let v = d * rng.range(0.4, 1.6) + v3(0.0, rng.range(1.0, 2.6), 0.0);
            let g = rng.range(0.16, 0.26);
            let mut p = Puff::new(FxKind::Smoke, x, v, rng.range(0.4, 0.7) * (0.7 + r * 0.12), rng.range(5.0, 9.0), [g, g * 0.95, g * 0.9], [0.5, 0.5, 0.52], rng);
            p.alpha = 0.75;
            p.delay = rng.range(0.0, 0.25);
            new.push(p);
        }
        for p in new {
            self.puff(p);
        }
        let blast = Blast { center, radius: r, at: self.clock };
        self.blasts.push(blast);
        // Blocks that went first, then the explosion: they belong to it.
        let now = self.clock;
        let (mine, rest): (Vec<Aired>, Vec<Aired>) = std::mem::take(&mut self.aired)
            .into_iter()
            .partition(|a| now - a.at < 0.6 && in_blast(&blast, a.pos));
        self.aired = rest;
        self.pending.push((blast, mine));
    }

    /// A block at `pos` just turned to air. Inside a fresh blast it is blown away; otherwise it is
    /// remembered for a moment, in case its explosion comes next.
    pub fn aired(&mut self, a: Aired) {
        let now = self.clock;
        self.aired.retain(|x| now - x.at < 0.6);
        if let Some(b) = self.blasts.iter().rev().find(|b| now - b.at < 1.0 && in_blast(b, a.pos)).copied() {
            self.pending.push((b, vec![a]));
            return;
        }
        if self.aired.len() < 4096 {
            self.aired.push(a);
        }
    }

    pub fn step(&mut self, dt: f64, camera: V3, frame: u64, st: &Settings, world: &dyn World) {
        self.clock += dt;
        self.frame = frame;
        self.camera = camera;
        let now = self.clock;
        self.blasts.retain(|b| now - b.at < 2.0);
        let wind = wind_at(now, st.wind());
        let g = st.gravity();
        // Fire, campfires and lava give off smoke and embers.
        if st.smoke && dt > 0.0 {
            self.emitters.retain(|p, _| {
                let d = v3(f64::from(p[0]), f64::from(p[1]), f64::from(p[2])) - camera;
                d.len() < 40.0
            });
            let list: Vec<([i32; 3], u8)> = self.emitters.iter().map(|(p, k)| (*p, *k)).collect();
            for (p, kind) in list {
                self.emit_fire(p, kind, dt, st);
            }
        }
        // The smoke keeps apart from itself, so where it cannot rise it spreads.
        let spread = self.smoke_spread();
        let t_now = self.clock;
        let mut i = 0;
        while i < self.puffs.len() {
            let push = spread.get(i).copied().unwrap_or_default();
            let p = &mut self.puffs[i];
            if p.delay > 0.0 {
                p.delay -= dt;
                i += 1;
                continue;
            }
            // Smoke caught under a ceiling ages slowly: it gathers there.
            p.age += if p.held > 0.3 { dt * 0.35 } else { dt };
            if p.age >= p.life {
                self.puffs.swap_remove(i);
                continue;
            }
            p.roll += p.spin * dt;
            let ph = f64::from(p.seed % 1000) * 0.0063;
            match p.kind {
                FxKind::Smoke => {
                    let t = p.age / p.life;
                    // Warm smoke rises and cools; swirls in the air; the wind takes it.
                    let lift = 1.5 * (1.0 - t).max(0.0).sqrt();
                    let swirl = v3((p.x.y * 1.3 + t_now * 0.7 + ph).sin(), 0.0, (p.x.y * 1.1 + t_now * 0.9 + ph * 1.7).cos()) * 0.55;
                    let pull = (wind - v3(p.v.x, 0.0, p.v.z)) * 0.55;
                    p.v = p.v + (pull + swirl + push * 2.5 + v3(0.0, lift, 0.0)) * dt;
                    p.v = p.v * (1.0 - 0.9 * dt).max(0.0);
                    let r = (p.size * p.scale() * 0.45).clamp(0.08, 0.6);
                    let mut next = p.x + p.v * dt;
                    if solid(world, next + v3(0.0, r, 0.0)) {
                        // A ceiling: it stops rising and flows out sideways along it.
                        p.held += dt;
                        p.v.y = p.v.y.min(0.0) * 0.2;
                        next.y = p.x.y.min(next.y);
                        let out = v3(push.x, 0.0, push.z);
                        p.v = p.v + out * (4.0 * dt);
                    } else {
                        p.held = (p.held - dt * 0.5).max(0.0);
                    }
                    // Walls: it slides along them.
                    for k in [0usize, 2] {
                        let mut probe = p.x;
                        probe.set(k, next.get(k) + r * p.v.get(k).signum());
                        if solid(world, probe) {
                            let vk = p.v.get(k);
                            p.v.set(k, -vk * 0.25);
                            next.set(k, p.x.get(k));
                        }
                    }
                    if solid(world, next) {
                        next = p.x;
                    }
                    p.x = next;
                }
                FxKind::Flame => {
                    p.v = p.v * (1.0 - 3.0 * dt).max(0.0) + v3(0.0, 1.8 * dt, 0.0);
                    p.x = p.x + p.v * dt;
                }
                FxKind::Ember => {
                    let swirl = v3((t_now * 3.1 + ph).sin(), 0.0, (t_now * 2.7 + ph * 1.3).cos()) * 1.4;
                    p.v = p.v + (swirl + v3(0.0, 0.9, 0.0) + (wind - p.v) * 0.3) * dt;
                    p.v = p.v * (1.0 - 1.1 * dt).max(0.0);
                    let next = p.x + p.v * dt;
                    if solid(world, next) {
                        p.age = p.life;
                    } else {
                        p.x = next;
                    }
                }
                FxKind::Dust => {
                    p.v.y -= 2.2 * dt;
                    p.v = p.v * (1.0 - 2.5 * dt).max(0.0) + wind * (0.6 * dt);
                    let next = p.x + p.v * dt;
                    if solid(world, next) {
                        p.v = V3::default();
                    } else {
                        p.x = next;
                    }
                }
                FxKind::Foam => {}
                FxKind::Spark => {
                    p.v.y -= g * dt;
                    p.v = p.v * (1.0 - 0.4 * dt);
                    let next = p.x + p.v * dt;
                    if solid(world, next) {
                        // Bounce off whichever side it came through.
                        for k in 0..3 {
                            let mut probe = p.x;
                            probe.set(k, next.get(k));
                            if solid(world, probe) {
                                let vk = p.v.get(k);
                                p.v.set(k, -vk * 0.35);
                            }
                        }
                        p.v = v3(p.v.x * 0.7, p.v.y, p.v.z * 0.7);
                    } else {
                        p.x = next;
                    }
                }
            }
            i += 1;
        }
        self.step_drops(dt, g, world);
        // The footprints fade; the ropes and capes nobody draws any more are forgotten.
        let life = 30.0;
        self.prints.retain_mut(|p| {
            p.age += dt;
            p.age < life
        });
        self.ropes.retain(|r| frame.saturating_sub(r.seen) < 30);
        self.capes.retain(|_, c| frame.saturating_sub(c.seen) < 60);
    }

    /// The blasts of this frame throw their blocks. Called with the pieces' physics.
    pub fn blow(&mut self, physics: &mut Physics, st: &Settings) {
        for (blast, blocks) in std::mem::take(&mut self.pending) {
            if blocks.is_empty() {
                continue;
            }
            // Big blasts make fewer pieces per block, so the scene stays light.
            let per = (st.pieces_per_block() / 2).clamp(2, 6 + 3 * usize::from(st.resolution % 3));
            let per = if blocks.len() > 120 { (per / 2).max(1) } else { per };
            for a in blocks {
                physics.spawn_blown(&a.boxes, a.pos, a.uv, a.tint, per, &blast, st.lifetime());
            }
        }
    }

    /// A fire block of `kind` (`scan_found`) at `p` for `dt` seconds: its smoke and embers.
    fn emit_fire(&mut self, p: [i32; 3], kind: u8, dt: f64, st: &Settings) {
        let base = v3(f64::from(p[0]), f64::from(p[1]), f64::from(p[2]));
        // Smoke a second, its size, life, colours and opacity; embers a second and their colour.
        let (rate, size, life, from, to, alpha, embers, ember) = match kind {
            1 => (6.0, 0.18, 5.0, [0.2, 0.19, 0.18], [0.55, 0.55, 0.56], 0.72, 2.5, [1.0, 0.62, 0.18]),
            2 => (5.0, 0.17, 4.5, [0.22, 0.25, 0.27], [0.55, 0.58, 0.6], 0.68, 2.0, [0.45, 0.9, 1.0]),
            3 => (14.0, 0.24, 10.0, [0.42, 0.42, 0.42], [0.8, 0.8, 0.81], 0.7, 1.2, [1.0, 0.62, 0.18]),
            4 => (13.0, 0.24, 10.0, [0.4, 0.43, 0.46], [0.76, 0.79, 0.82], 0.68, 1.0, [0.45, 0.9, 1.0]),
            _ => (0.15, 0.14, 3.5, [0.25, 0.22, 0.2], [0.55, 0.55, 0.55], 0.4, 0.3, [1.0, 0.55, 0.1]),
        };
        let y = match kind {
            5 => 1.0,
            3 | 4 => 0.55,
            _ => 0.35,
        };
        let campfire = kind == 3 || kind == 4;
        // How many this frame: the whole part, and the rest by chance.
        let mut count = |rate: f64| {
            let n = rate * dt;
            n.floor() as usize + usize::from(self.rng.next() < n.fract())
        };
        let embers = if st.embers { embers } else { 0.0 };
        let (smokes, sparks) = (count(rate * f64::from(st.smoke_amount)), count(embers));
        let alpha = (alpha * f64::from(st.smoke_opacity)).min(1.0);
        for _ in 0..smokes {
            let rng = &mut self.rng;
            let x = base + v3(rng.range(0.3, 0.7), y + rng.range(0.0, 0.2), rng.range(0.3, 0.7));
            let up = if campfire { rng.range(1.2, 1.9) } else { rng.range(0.9, 1.5) };
            let v = v3(rng.range(-0.15, 0.15), up, rng.range(-0.15, 0.15));
            let mut puff = Puff::new(FxKind::Smoke, x, v, size * rng.range(0.8, 1.2), life * rng.range(0.8, 1.2), from, to, rng);
            puff.alpha = alpha;
            self.puff(puff);
        }
        for _ in 0..sparks {
            let rng = &mut self.rng;
            let puff = if kind == 5 {
                // Lava pops: a fleck of it thrown up, falling back.
                let x = base + v3(rng.range(0.2, 0.8), 1.0, rng.range(0.2, 0.8));
                let v = v3(rng.range(-0.8, 0.8), rng.range(2.5, 4.5), rng.range(-0.8, 0.8));
                Puff::new(FxKind::Spark, x, v, rng.range(0.015, 0.025), rng.range(0.8, 1.4), [1.0, 0.75, 0.25], [0.8, 0.2, 0.02], rng)
            } else {
                let x = base + v3(rng.range(0.2, 0.8), y - 0.1 + rng.range(0.0, 0.3), rng.range(0.2, 0.8));
                let v = v3(rng.range(-0.3, 0.3), rng.range(0.8, 1.8), rng.range(-0.3, 0.3));
                Puff::new(FxKind::Ember, x, v, rng.range(0.018, 0.03), rng.range(1.0, 2.6), ember, [0.8, 0.15, 0.02], rng)
            };
            self.puff(puff);
        }
    }

    /// For each puff, which way the smoke around it pushes it (nothing for other kinds).
    fn smoke_spread(&self) -> Vec<V3> {
        let n = self.puffs.len();
        let mut push = vec![V3::default(); n];
        let smoke = |p: &Puff| p.kind == FxKind::Smoke && p.delay <= 0.0;
        let mut grid: HashMap<(i32, i32, i32), Vec<usize>> = HashMap::new();
        for (i, p) in self.puffs.iter().enumerate() {
            if smoke(p) {
                grid.entry((floor(p.x.x), floor(p.x.y), floor(p.x.z))).or_default().push(i);
            }
        }
        for (i, p) in self.puffs.iter().enumerate() {
            if !smoke(p) {
                continue;
            }
            let ri = (p.size * p.scale() * 0.5).clamp(0.1, 0.8);
            let c = (floor(p.x.x), floor(p.x.y), floor(p.x.z));
            for dx in -1..=1 {
                for dy in -1..=1 {
                    for dz in -1..=1 {
                        let Some(list) = grid.get(&(c.0 + dx, c.1 + dy, c.2 + dz)) else { continue };
                        for &j in list {
                            if j == i {
                                continue;
                            }
                            let q = &self.puffs[j];
                            let rj = (q.size * q.scale() * 0.5).clamp(0.1, 0.8);
                            let d = p.x - q.x;
                            let l = d.len();
                            let reach = ri + rj;
                            if l >= reach {
                                continue;
                            }
                            if l < 1e-6 {
                                let a = f64::from(p.seed % 628) * 0.01;
                                push[i] = push[i] + v3(a.cos(), 0.0, a.sin()) * 0.5;
                            } else {
                                push[i] = push[i] + d * ((reach - l) / reach / l);
                            }
                        }
                    }
                }
            }
        }
        push
    }

    pub fn emit(&self, camera: V3, white: [f32; 4], world: &dyn World, out: &mut Vec<Vertex>) {
        let lit = |p: V3| world.light(floor(p.x), floor(p.y), floor(p.z));
        let mut shapes: Vec<Shape> = Vec::new();
        for p in &self.puffs {
            if p.delay > 0.0 {
                continue;
            }
            let t = (p.age / p.life).clamp(0.0, 1.0);
            let radius = p.size * p.scale();
            let alpha = p.opacity();
            if radius <= 0.004 || alpha < 0.11 {
                continue;
            }
            let at = p.x - camera;
            let mut shape = Shape::new(at, radius, p.roll, p.seed, mix3(p.from, p.to, t), alpha, GLOW);
            match p.kind {
                FxKind::Smoke => {
                    shape.light = lit(p.x);
                    // Smoke trapped under a ceiling thickens and darkens as more gathers.
                    let dark = (p.held / 6.0).clamp(0.0, 0.55);
                    shape.rgb = shape.rgb.map(|c| c * (1.0 - dark));
                    shape.alpha = (shape.alpha * (1.0 + dark * 0.6)).min(1.0);
                    // Thicker in the middle of each puff, soft at its edge.
                    shape.core = 1.0;
                }
                FxKind::Flame => {
                    // Hot centre: the middle whiter than the rim.
                    shape.core = 1.0;
                    shape.center_rgb = Some(mix3([1.0, 0.96, 0.78], mix3(p.from, p.to, t), t.sqrt()));
                }
                FxKind::Spark => {
                    shape.stretch = Some((p.v, 1.0 + p.v.len() * 0.09));
                    shape.segments = 6;
                }
                FxKind::Ember => {
                    shape.segments = 6;
                    shape.center_rgb = Some([1.0, 0.95, 0.75]);
                }
                FxKind::Dust => {
                    shape.light = lit(p.x);
                    shape.segments = 8;
                }
                FxKind::Foam => {
                    shape.light = lit(p.x + v3(0.0, 0.2, 0.0));
                    shape.flat = true;
                    shape.segments = 10;
                }
            }
            shapes.push(shape);
        }
        for d in &self.drops {
            let t = (d.age / d.life).clamp(0.0, 1.0);
            let radius = d.size * ((1.0 - t) / 0.3).min(1.0);
            if radius <= 0.004 {
                continue;
            }
            let at = d.x - camera;
            let stretch = Some((d.v, 1.0 + d.v.len() * 0.06));
            let mut shape = if d.lava {
                let mut s = Shape::new(at, radius * 1.2, 0.0, 7, mix3([1.0, 0.62, 0.15], [0.75, 0.2, 0.02], t), 0.95, GLOW);
                s.center_rgb = Some([1.0, 0.85, 0.45]);
                s
            } else {
                let c = [0.62 + d.shade * 0.1, 0.76 + d.shade * 0.08, 0.98];
                let mut s = Shape::new(at, radius * 1.15, 0.0, 7, c, 0.62, lit(d.x));
                // A drop catches the light in its middle.
                s.center_rgb = Some([0.92, 0.96, 1.0]);
                s
            };
            shape.stretch = stretch;
            shape.segments = 6;
            shapes.push(shape);
        }
        // Prints lie on the snow under everything else: drawn first, in their own order.
        let mut ground: Vec<Shape> = Vec::new();
        for f in &self.prints {
            let t = (f.age / 30.0).clamp(0.0, 1.0);
            // The snow slowly fills the print in again.
            let fill = (1.0 - t).powf(1.5);
            if fill * 0.6 < 0.11 {
                continue;
            }
            let light = world.light(floor(f.x.x), floor(f.x.y + 0.1), floor(f.x.z));
            let (s, c) = f.yaw.sin_cos();
            let fwd = v3(-s, 0.0, c);
            let at = f.x - camera;
            // A boot pressed into snow: the far wall of the print catches the light (a bright rim
            // beyond the toe), the bottom lies in blue shade, darkest under the ball and heel.
            let mut rim = Shape::new(at + fwd * 0.014 + v3(0.0, -0.0005, 0.0), 0.33, 0.0, 3, [1.0, 1.0, 1.0], 0.55 * fill, light);
            rim.flat = true;
            rim.stretch = Some((fwd, 1.0));
            rim.outline = Some(&SOLE);
            rim.edge = 0.6;
            rim.center_rgb = Some([0.97, 0.98, 1.0]);
            ground.push(rim);
            let mut print = Shape::new(at, 0.32, 0.0, 3, [0.66, 0.73, 0.86], 0.6 * fill, light);
            print.flat = true;
            print.stretch = Some((fwd, 1.0));
            print.outline = Some(&SOLE);
            print.edge = 0.75;
            print.center_rgb = Some([0.47, 0.55, 0.72]);
            ground.push(print);
            for (along, radius) in [(0.09, 0.05), (-0.1, 0.04)] {
                let mut deep = Shape::new(at + fwd * along + v3(0.0, 0.0005, 0.0), radius, 0.0, 5, [0.5, 0.58, 0.75], 0.4 * fill, light);
                deep.flat = true;
                deep.stretch = Some((fwd, 1.3));
                deep.center_rgb = Some([0.42, 0.5, 0.68]);
                ground.push(deep);
            }
        }
        // Far ones first: each is blended over what is behind it.
        shapes.sort_by(|a, b| (b.at.dot(b.at)).partial_cmp(&a.at.dot(a.at)).unwrap_or(std::cmp::Ordering::Equal));
        for s in ground.iter().chain(&shapes) {
            s.draw(white, out);
        }
    }

    /// The player stands at `feet` (yaw in degrees), on snow whose top is `top`, or not on snow.
    pub fn footstep(&mut self, feet: V3, yaw: f64, top: Option<f64>) {
        let Some(top) = top else {
            self.last_step = None;
            return;
        };
        let here = v3(feet.x, top, feet.z);
        let Some(last) = self.last_step else {
            self.last_step = Some(here);
            return;
        };
        let d = v3(here.x - last.x, 0.0, here.z - last.z);
        if d.len() < 0.55 {
            return;
        }
        if d.len() > 3.0 {
            self.last_step = Some(here);
            return;
        }
        self.last_step = Some(here);
        self.left = !self.left;
        let r = yaw.to_radians();
        let (s, c) = r.sin_cos();
        let side = v3(c, 0.0, s) * if self.left { -0.13 } else { 0.13 };
        // The print itself is the snow pressed down (the renderer's snow); here only the powder.
        // A little powder kicked up by the step.
        let fwd = v3(-s, 0.0, c);
        for _ in 0..4 {
            let rng = &mut self.rng;
            let x = here + side + fwd * rng.range(0.0, 0.15) + v3(rng.range(-0.08, 0.08), 0.03, rng.range(-0.08, 0.08));
            let v = fwd * rng.range(0.3, 0.9) + v3(rng.range(-0.3, 0.3), rng.range(0.6, 1.3), rng.range(-0.3, 0.3));
            let puff = Puff::new(FxKind::Dust, x, v, rng.range(0.03, 0.055), rng.range(0.5, 0.9), [0.96, 0.97, 1.0], [0.92, 0.94, 0.98], rng);
            self.puff(puff);
        }
        if self.prints.len() > 240 {
            self.prints.remove(0);
        }
    }

    /// A rope from `a` to `b` this frame (an id-less match against last frame's ropes of the same
    /// `group`): sags under gravity, sways in the wind, lies on blocks. Returns its points.
    #[allow(clippy::too_many_arguments)]
    pub fn rope(&mut self, group: u8, a: V3, b: V3, slack: f64, dt: f64, g: f64, wind: V3, world: &dyn World) -> Vec<V3> {
        let frame = self.frame;
        let dist = (b - a).len();
        // A fixed number of segments, as vanilla's lead (24): the rope never has to be cut up
        // again as its ends move apart or together.
        let want = 24;
        // Last frame's rope with the nearest ends, whatever its number of points: the lead of a
        // mob that walks off keeps its shape instead of starting again from a straight line.
        let best = self
            .ropes
            .iter()
            .enumerate()
            .filter(|(_, r)| r.group == group && r.seen != frame + 1 && r.pts.len() >= 2)
            .map(|(i, r)| (i, (r.pts[0] - a).len() + (r.pts[r.pts.len() - 1] - b).len()))
            .filter(|(_, d)| *d < 2.5)
            .fold(None, |acc: Option<(usize, f64)>, x| if acc.is_none_or(|y| x.1 < y.1) { Some(x) } else { acc });
        let i = match best {
            Some((i, _)) => i,
            None => {
                let pts: Vec<V3> = (0..=want).map(|k| a + (b - a) * (k as f64 / want as f64)).collect();
                self.ropes.push(Rope { prev: pts.clone(), pts, seen: frame + 1, group });
                self.ropes.len() - 1
            }
        };
        let r = &mut self.ropes[i];
        r.seen = frame + 1;
        // Far more or fewer points than its length needs now: the same curve, resampled.
        let old = r.pts.len() - 1;
        if old * 2 < want || old > want * 2 {
            r.pts = resample(&r.pts, want);
            r.prev = resample(&r.prev, want);
        }
        let n = r.pts.len() - 1;
        let rest = dist * (1.0 + slack) / n as f64;
        let steps = 3;
        let h = dt.clamp(0.0, 0.05) / steps as f64;
        for _ in 0..steps {
            for k in 1..n {
                let p = r.pts[k];
                // Air drag damps the swing; a lead is heavy, so the wind barely moves it (a
                // fishing line, light, a little more).
                let vel = (p - r.prev[k]) * if group == 0 { 0.9 } else { 0.95 };
                r.prev[k] = p;
                let blow = if group == 0 { 0.03 } else { 0.12 };
                r.pts[k] = p + vel + (v3(0.0, -g, 0.0) + wind * blow) * (h * h);
            }
            r.pts[0] = a;
            r.pts[n] = b;
            for it in 0..12 {
                for k in 0..n {
                    let (p, q) = (r.pts[k], r.pts[k + 1]);
                    let d = q - p;
                    let l = d.len();
                    if l <= rest || l < 1e-9 {
                        continue;
                    }
                    let corr = d * ((l - rest) / l);
                    let (wa, wb) = (if k == 0 { 0.0 } else { 1.0 }, if k + 1 == n { 0.0 } else { 1.0 });
                    let sum = wa + wb;
                    if sum == 0.0 {
                        continue;
                    }
                    r.pts[k] = p + corr * (wa / sum);
                    r.pts[k + 1] = q - corr * (wb / sum);
                }
                // Out of the blocks, every other pass, so the length cannot pull it back in.
                if it % 2 == 1 {
                    for k in 1..n {
                        push_out(world, &mut r.pts[k], &mut r.prev[k]);
                    }
                }
            }
        }
        r.pts.clone()
    }
}

/// The polyline `pts` as `n` + 1 points evenly along it.
fn resample(pts: &[V3], n: usize) -> Vec<V3> {
    let lens: Vec<f64> = pts.windows(2).map(|w| (w[1] - w[0]).len()).collect();
    let total: f64 = lens.iter().sum();
    if total < 1e-9 || pts.len() < 2 {
        return vec![pts[0]; n + 1];
    }
    let mut out = Vec::with_capacity(n + 1);
    let (mut seg, mut done) = (0usize, 0.0);
    for k in 0..=n {
        let at = total * k as f64 / n as f64;
        while seg + 1 < lens.len() && done + lens[seg] < at {
            done += lens[seg];
            seg += 1;
        }
        let t = if lens[seg] > 1e-12 { ((at - done) / lens[seg]).clamp(0.0, 1.0) } else { 0.0 };
        out.push(pts[seg] + (pts[seg + 1] - pts[seg]) * t);
    }
    out
}

/// Moves point `p` out of the blocks it is in, and takes away its speed into them (`prev` holds
/// last step's place, so the speed is `p - prev`), with some friction along the surface.
fn push_out(world: &dyn World, p: &mut V3, prev: &mut V3) {
    for _ in 0..3 {
        let Some((n, depth)) = penetration(world, *p, *prev, None) else { return };
        *p = *p + n * depth;
        let v = *p - *prev;
        let vn = v.dot(n);
        let along = v - n * vn;
        let v = along * 0.6 + n * vn.max(0.0);
        *prev = *p - v;
    }
}

fn in_blast(b: &Blast, pos: [i32; 3]) -> bool {
    let c = v3(f64::from(pos[0]) + 0.5, f64::from(pos[1]) + 0.5, f64::from(pos[2]) + 0.5);
    (c - b.center).len() <= b.radius * 1.6 + 1.0
}

impl Physics {
    /// The player (a column of 0.35 blocks round and 1.8 high standing at `feet`, moving at `vel`)
    /// shoves the pieces and ragdolls it walks into: out of its way, carried along at its speed,
    /// tumbling.
    /// A hit from `eye` along `dir` (a unit vector): the first piece within `reach` is knocked
    /// away (its whole ragdoll goes along), `speed` blocks a second. True when one was hit.
    pub fn punch(&mut self, eye: V3, dir: V3, reach: f64, speed: f64) -> bool {
        let mut best: Option<(usize, usize, f64)> = None;
        for (oi, o) in self.objects.iter().enumerate() {
            for i in o.first..o.first + o.count {
                let b = &self.bodies[i];
                let r = b.radius().max(0.12);
                let to = b.x - eye;
                let t = to.dot(dir);
                if t < 0.0 || t > reach + r || (to - dir * t).len() > r {
                    continue;
                }
                if best.is_none_or(|x| t < x.2) {
                    best = Some((oi, i, t));
                }
            }
        }
        let Some((oi, i, t)) = best else {
            return false;
        };
        let o = &mut self.objects[oi];
        o.sleeping = false;
        o.still = 0.0;
        let push = (dir * 0.85 + v3(0.0, 0.4, 0.0)).norm() * speed;
        for j in o.first..o.first + o.count {
            let b = &mut self.bodies[j];
            if j == i {
                let r = eye + dir * t - b.x;
                let mass = 1.0 / b.inv_mass.max(1e-9);
                b.push(push * mass, r);
            } else {
                b.v = b.v + push * 0.35;
            }
        }
        true
    }

    pub fn shove(&mut self, feet: V3, vel: V3) {
        const RADIUS: f64 = 0.35;
        const HEIGHT: f64 = 1.8;
        let hv = v3(vel.x, 0.0, vel.z);
        for o in &mut self.objects {
            let mut hit = false;
            for b in &mut self.bodies[o.first..o.first + o.count] {
                let r = b.radius().min(0.6);
                if b.x.y + r < feet.y - 0.05 || b.x.y - r > feet.y + HEIGHT {
                    continue;
                }
                let d = v3(b.x.x - feet.x, 0.0, b.x.z - feet.z);
                let l = d.len();
                let reach = RADIUS + r * 0.8;
                if l >= reach {
                    continue;
                }
                // Standing (or landing) on it: it is pressed down a little, never shot away.
                if feet.y > b.x.y + b.half.y * 0.5 {
                    b.v.y = b.v.y.min(0.0) + vel.y.min(0.0) * 0.3;
                    b.v = b.v + hv * 0.15;
                    continue;
                }
                let n = if l > 1e-6 {
                    d * (1.0 / l)
                } else if hv.len() > 1e-6 {
                    hv.norm()
                } else {
                    v3(1.0, 0.0, 0.0)
                };
                // Out of the way, a little each frame (a big jump would push it into a block).
                b.x = b.x + n * (reach - l).min(0.06);
                // Carried at least as fast as the player walks into it, and a little more.
                let into = hv.dot(n).max(0.0).min(6.0);
                let target = into * 1.15 + 0.5;
                let vn = b.v.dot(n);
                if vn < target {
                    b.v = b.v + n * (target - vn);
                }
                b.v.y = b.v.y.max(0.35 * into.min(4.0));
                // Kicked over: it rolls away from the feet, as fast as the kick, no faster.
                let roll = n.cross(v3(0.0, 1.0, 0.0)) * (-(into * 2.5 + 0.5));
                if b.w.dot(roll) < roll.dot(roll) {
                    b.w = b.w + roll * 0.25;
                }
                hit = true;
            }
            if hit {
                o.sleeping = false;
                o.still = 0.0;
            }
        }
    }

    /// A block blown out of an explosion: a few pieces thrown away from its center.
    #[allow(clippy::too_many_arguments)]
    pub fn spawn_blown(&mut self, boxes: &[[f64; 6]], pos: [i32; 3], uv: [f32; 4], tint: u32, per: usize, blast: &Blast, life: f64) {
        let total: f64 = boxes.iter().map(|b| (b[3] - b[0]) * (b[4] - b[1]) * (b[5] - b[2])).sum();
        if total <= 1e-6 {
            return;
        }
        let o = v3(f64::from(pos[0]), f64::from(pos[1]), f64::from(pos[2]));
        let color = tint_color(tint);
        let mut bodies = Vec::new();
        for b in boxes {
            let vol = (b[3] - b[0]) * (b[4] - b[1]) * (b[5] - b[2]);
            let n = ((per as f64 * vol / total).round() as usize).max(1);
            for piece in split_box(*b, n, &mut self.rng) {
                let (c, h, verts) = box_mesh(piece, uv, color);
                let at = o + c;
                let mut body = Body::new(at, Quat::IDENTITY, h, verts);
                let d = at - blast.center;
                let dist = d.len().max(0.3);
                let dir = (d * (1.0 / dist) + self.rng.unit() * 0.35).norm();
                let power = (1.0 - dist / (blast.radius * 1.6 + 1.0)).clamp(0.15, 1.0);
                let speed = (4.0 + blast.radius * 2.2) * power * self.rng.range(0.6, 1.2);
                let up = self.rng.range(2.0, 5.0) * power + speed * 0.3;
                body.v = dir * speed.min(13.0) + v3(0.0, up, 0.0);
                body.w = self.rng.unit() * self.rng.range(3.0, 12.0);
                bodies.push(body);
            }
        }
        let life = life + self.rng.range(0.0, 2.0);
        self.add(bodies, Vec::new(), life, None);
    }

    /// Pushes the pieces (of objects from `from` on) near a blast away from it.
    pub fn blast(&mut self, center: V3, reach: f64, strength: f64, from: usize) {
        for oi in from..self.objects.len() {
            let o = &mut self.objects[oi];
            let mut hit = false;
            for b in &mut self.bodies[o.first..o.first + o.count] {
                let d = b.x - center;
                let l = d.len();
                if l >= reach {
                    continue;
                }
                let k = (1.0 - l / reach) * strength * 12.0;
                b.v = b.v + d.norm() * k + v3(0.0, k * 0.4, 0.0);
                b.w = b.w + self.rng.unit() * k;
                hit = true;
            }
            if hit {
                o.sleeping = false;
                o.still = 0.0;
            }
        }
    }
}

// --- What the hooks call ----------------------------------------------------------------------

/// Whether a part of the mod is on at all (the mod itself and the setting).
fn part_on(f: impl Fn(&Settings) -> bool) -> bool {
    ENABLED.load(Ordering::Relaxed) && f(&settings())
}

pub fn explosions_on() -> bool {
    part_on(|s| s.explosions)
}

/// Whether vanilla's own smoke over fire and campfires gives way to the mod's.
pub fn replaces_vanilla_smoke() -> bool {
    smoke_on() && settings().vanilla_smoke
}

pub fn smoke_on() -> bool {
    part_on(|s| s.smoke)
}

pub fn snow_on() -> bool {
    part_on(|s| s.snow)
}

/// Whether the mobs leave trails in the snow too, and how deep a trail is (0 to 1 of the most).
pub fn mob_trails_on() -> bool {
    part_on(|s| s.snow && s.mob_trails)
}

pub fn snow_depth() -> f32 {
    settings().snow_depth
}

/// Deep snow is pressed down deeper (the Snow page's Deep Snow).
pub fn snow_layers() -> bool {
    part_on(|s| s.snow && s.snow_layers)
}

/// A fishing line bows downstream in flowing water.
pub fn rod_current() -> bool {
    part_on(|s| s.ropes && s.rod_current)
}

pub fn ropes_on() -> bool {
    part_on(|s| s.ropes)
}

pub fn capes_on() -> bool {
    part_on(|s| s.capes)
}

/// Whether a cape is hidden while its wearer has a chest piece on.
pub fn hide_cape_with_armor() -> bool {
    part_on(|s| s.capes && s.hide_cape)
}

/// The explosion packet: a blast at `center` of `radius`.
pub fn explosion(center: [f64; 3], radius: f32) {
    if !explosions_on() {
        return;
    }
    with(|s| {
        let c = v3(center[0], center[1], center[2]);
        s.env.explode(c, f64::from(radius));
        // Pieces already lying around are thrown too.
        s.physics.blast(c, f64::from(radius) * 2.5, 0.8, 0);
    });
}

/// The explosion packet for a blast under water: the blast of `explosion` without fire or smoke
/// (it is wet), and a spout of water thrown up through the surface at height `surface`.
pub fn explosion_wet(center: [f64; 3], radius: f32, surface: f64) {
    if !explosions_on() {
        return;
    }
    let st = settings();
    with(|s| {
        let c = v3(center[0], center[1], center[2]);
        s.env.wet = true;
        s.env.explode(c, f64::from(radius));
        s.env.wet = false;
        s.physics.blast(c, f64::from(radius) * 2.5, 0.8, 0);
        if st.water_boom {
            s.env.water_boom(c, f64::from(radius), surface);
        }
    });
}

/// A block that just turned to air from the server: its outline (block space), sprite and tint.
pub fn block_aired(pos: [i32; 3], boxes: &[[f64; 6]], uv: [f32; 4], tint: u32) {
    if !explosions_on() || boxes.is_empty() {
        return;
    }
    with(|s| {
        let at = s.env.clock;
        s.env.aired(Aired { pos, boxes: boxes.to_vec(), uv, tint, at })
    });
}

/// Blows away this frame's blocks of explosions. Called from `frame` before the step.
fn blow_pending(s: &mut State, st: &Settings) {
    let State { env, physics, .. } = s;
    env.blow(physics, st);
}

/// Which layer of blocks around the camera the hook should look through for fire this frame:
/// `(y, x0, x1, z0, z1)`, all inclusive. A different layer each frame, 25 layers in turn.
pub fn scan_layer(camera: [f64; 3]) -> (i32, i32, i32, i32, i32) {
    with(|s| {
        s.env.scan = (s.env.scan + 1) % 25;
        let (x, y, z) = (floor(camera[0]), floor(camera[1]), floor(camera[2]));
        (y - 12 + s.env.scan, x - 20, x + 20, z - 20, z + 20)
    })
}

/// What the hook found on layer `y`: fire (1), soul fire (2), a lit campfire (3), a lit soul
/// campfire (4), open lava (5).
pub fn scan_found(y: i32, found: &[([i32; 3], u8)]) {
    with(|s| {
        s.env.emitters.retain(|p, _| p[1] != y);
        for &(p, k) in found.iter().take(400) {
            if s.env.emitters.len() >= 600 {
                break;
            }
            s.env.emitters.insert(p, k);
        }
    });
}

/// The player's feet this frame and the top of the snow they stand on (None: not on snow).
pub fn footstep(feet: [f64; 3], yaw: f32, top: Option<f64>) {
    if !snow_on() || !settings().snow_powder {
        return;
    }
    with(|s| s.env.footstep(v3(feet[0], feet[1], feet[2]), f64::from(yaw), top));
}

/// The points of a rope (lead: group 0, fishing line: group 1) between `a` and `b` this frame.
pub fn rope(group: u8, a: [f64; 3], b: [f64; 3], slack: f64, world: &dyn World) -> Vec<[f64; 3]> {
    let st = settings();
    with(|s| {
        let dt = s.last_dt;
        let wind = wind_at(s.env.clock, st.wind());
        let pts = s.env.rope(group, v3(a[0], a[1], a[2]), v3(b[0], b[1], b[2]), slack, dt, st.gravity(), wind, world);
        pts.iter().map(|p| [p.x, p.y, p.z]).collect()
    })
}

/// Leads: the lead geometry the game drew this frame (`drawn`, camera-relative) is replaced by
/// ropes. Each vanilla lead is 384 vertices with zero normals, the knots come after with real
/// normals and are kept as they are.
pub fn leads(drawn: &[Vertex], camera: [f64; 3], world: &dyn World, out: &mut Vec<Vertex>) {
    const PER: usize = 384;
    let cam = v3(camera[0], camera[1], camera[2]);
    let mut i = 0;
    while i < drawn.len() {
        if drawn[i].normal != [0; 4] || i + PER > drawn.len() || drawn[i..i + PER].iter().any(|v| v.normal != [0; 4]) {
            out.push(drawn[i]);
            i += 1;
            continue;
        }
        let lead = &drawn[i..i + PER];
        i += PER;
        let mid = |a: &Vertex, b: &Vertex| v3(f64::from(a.pos[0] + b.pos[0]) * 0.5, f64::from(a.pos[1] + b.pos[1]) * 0.5, f64::from(a.pos[2] + b.pos[2]) * 0.5);
        // Step 0's first corners and step 23's last ones (quad order a0 b0 b1 a1).
        let start = mid(&lead[0], &lead[1]) + cam;
        let end = mid(&lead[23 * 8 + 2], &lead[23 * 8 + 3]) + cam;
        let pts = rope(0, [start.x, start.y, start.z], [end.x, end.y, end.z], 0.12, world);
        let pts: Vec<V3> = pts.iter().map(|p| v3(p[0], p[1], p[2]) - cam).collect();
        let (c0, c1) = (lead[0].color, lead[8].color);
        let (light, uv) = (lead[0].light, lead[0].uv);
        tube(&pts, 0.025, [c0, c1], light, uv, out);
    }
}

/// A round tube (six sides, lit by its normals) of `half` thickness along `pts`, its segments in
/// alternating colours like vanilla's lead.
fn tube(pts: &[V3], half: f64, colors: [u32; 2], light: u32, uv: [f32; 2], out: &mut Vec<Vertex>) {
    const SIDES: usize = 6;
    let pack = |x: f64| (x.clamp(-1.0, 1.0) * 127.0) as i8;
    // A frame carried along the rope, so the sides never twist.
    let mut side = v3(1.0, 0.0, 0.0);
    let mut rings: Vec<Vec<(V3, V3)>> = Vec::with_capacity(pts.len());
    for k in 0..pts.len() {
        let a = pts[k.saturating_sub(1)];
        let b = pts[(k + 1).min(pts.len() - 1)];
        let d = (b - a).norm();
        let s = side - d * side.dot(d);
        side = if s.len() < 1e-6 { d.cross(v3(0.0, 1.0, 0.0)).norm() } else { s.norm() };
        let up = d.cross(side).norm();
        rings.push(
            (0..SIDES)
                .map(|i| {
                    let a = std::f64::consts::TAU * i as f64 / SIDES as f64;
                    let n = side * a.cos() + up * a.sin();
                    (pts[k] + n * half, n)
                })
                .collect(),
        );
    }
    for (k, w) in rings.windows(2).enumerate() {
        let color = colors[k % 2];
        for f in 0..SIDES {
            let g = (f + 1) % SIDES;
            for (p, n) in [w[0][f], w[0][g], w[1][g], w[1][f]] {
                out.push(Vertex {
                    pos: [p.x as f32, p.y as f32, p.z as f32],
                    uv,
                    layer: 0,
                    color,
                    light,
                    normal: [pack(n.x), pack(n.y), pack(n.z), -127],
                    glint_uv: [0.0, 0.0],
                });
            }
        }
    }
}

/// A cape as cloth. `rest` is the cape the game would draw hanging straight (camera-relative,
/// the last frame's camera), `pivot` where it hangs from (the neck). Draws the cloth into `out`
/// and returns true, or returns false when the game should draw its own (not a cape we know how
/// to read, or a second drawing of the same player this frame, as in the inventory).
pub fn cape(id: u32, rest: &[Vertex], pivot: [f32; 3], out: &mut Vec<Vertex>) -> bool {
    if !capes_on() || rest.len() < 24 || rest.len() % 4 != 0 {
        return false;
    }
    let st = settings();
    let d = pivot[0] * pivot[0] + pivot[1] * pivot[1] + pivot[2] * pivot[2];
    if d.sqrt() > st.cape_range {
        return false;
    }
    with(|s| {
        let cam = s.camera;
        let dt = s.last_dt;
        let frame = s.frame;
        let wind = wind_at(s.env.clock, st.wind());
        let g = st.gravity();
        let pos = |v: &Vertex| v3(f64::from(v.pos[0]), f64::from(v.pos[1]), f64::from(v.pos[2]));
        // The two biggest faces: the outside (away from the neck) and the inside.
        let mut quads: Vec<(usize, f64, V3)> = rest
            .chunks(4)
            .enumerate()
            .map(|(i, q)| {
                let (a, b, c) = (pos(&q[0]), pos(&q[1]), pos(&q[2]));
                let n = (b - a).cross(c - a);
                let center = (pos(&q[0]) + pos(&q[1]) + pos(&q[2]) + pos(&q[3])) * 0.25;
                (i, n.len(), center)
            })
            .collect();
        quads.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let piv = v3(f64::from(pivot[0]), f64::from(pivot[1]), f64::from(pivot[2]));
        let (qa, qb) = (quads[0], quads[1]);
        let (outer, inner) = if (qa.2 - piv).len() >= (qb.2 - piv).len() { (qa, qb) } else { (qb, qa) };
        let oq = &rest[outer.0 * 4..outer.0 * 4 + 4];
        let iq = &rest[inner.0 * 4..inner.0 * 4 + 4];
        // The outside's corners: the two nearest the neck are the top.
        let mut idx = [0usize, 1, 2, 3];
        idx.sort_by(|&a, &b| (pos(&oq[a]) - piv).len().partial_cmp(&(pos(&oq[b]) - piv).len()).unwrap_or(std::cmp::Ordering::Equal));
        // The two tops are equally far from the neck; take them in the face's own order, so the
        // cloth's left and right never swap from one frame to the next.
        let (t0, t1) = if (idx[0] + 1) % 4 == idx[1] { (idx[0], idx[1]) } else { (idx[1], idx[0]) };
        // Each top corner's bottom corner is the one next to it around the face.
        let next = |a: usize| if (a + 1) % 4 == t0 || (a + 1) % 4 == t1 { (a + 3) % 4 } else { (a + 1) % 4 };
        let (b0, b1) = (next(t0), next(t1));
        let corner = |k: usize| pos(&oq[k]) + cam;
        let (tl, tr, bl, br) = (corner(t0), corner(t1), corner(b0), corner(b1));
        let width = (tr - tl).len();
        let height = (bl - tl).len();
        if width < 0.05 || height < 0.05 {
            return false;
        }
        let into = (inner.2 - outer.2).norm();
        let grid_rest = |r: usize, c: usize| {
            let (u, v) = (c as f64 / (CAPE_W - 1) as f64, r as f64 / (CAPE_H - 1) as f64);
            let top = tl + (tr - tl) * u;
            let bottom = bl + (br - bl) * u;
            top + (bottom - top) * v
        };
        // A second drawing in the same frame somewhere else (the inventory's player): not ours.
        if let Some(c) = s.env.capes.get(&id)
            && c.seen == frame
            && (c.pts[0] - tl).len() > 1.5
        {
            return false;
        }
        let fresh = s.env.capes.get(&id).is_none_or(|c| (c.pts[0] - tl).len() > 3.0 || c.pts.len() != CAPE_W * CAPE_H);
        if fresh {
            let pts: Vec<V3> = (0..CAPE_H).flat_map(|r| (0..CAPE_W).map(move |c| (r, c))).map(|(r, c)| grid_rest(r, c)).collect();
            s.env.capes.insert(id, Cloth { prev: pts.clone(), pts, seen: 0, width, height });
        }
        let cl = s.env.capes.get_mut(&id).expect("cape just made");
        if cl.seen != frame {
            cl.seen = frame;
            cl.width = width;
            cl.height = height;
            let (dx, dy) = (width / (CAPE_W - 1) as f64, height / (CAPE_H - 1) as f64);
            let steps = 3;
            let h = dt.clamp(0.0, 0.05) / steps as f64;
            let at = |r: usize, c: usize| r * CAPE_W + c;
            let top_mid = (tl + tr) * 0.5;
            for _ in 0..steps {
                for r in 1..CAPE_H {
                    for c in 0..CAPE_W {
                        let k = at(r, c);
                        let p = cl.pts[k];
                        let vel = (p - cl.prev[k]) * 0.99;
                        cl.prev[k] = p;
                        // The air drags the cloth toward its own speed: running, the cape trails
                        // and lifts behind; standing in wind, it blows out. A ripple travels down
                        // it so it never hangs dead.
                        let speed = vel * (1.0 / h.max(1e-6));
                        let ripple = ((self_hash(id) + r as f64 * 0.9 + c as f64 * 0.35) - s.env.clock * 7.0).sin();
                        let air = wind * (0.45 + 0.15 * ripple) + v3(0.0, ripple * 0.08, 0.0);
                        let force = v3(0.0, -g * 0.5, 0.0) + (air - speed) * 2.2;
                        cl.pts[k] = p + vel + force * (h * h);
                    }
                }
                for c in 0..CAPE_W {
                    cl.pts[at(0, c)] = grid_rest(0, c);
                }
                for _ in 0..6 {
                    for r in 0..CAPE_H {
                        for c in 0..CAPE_W {
                            if c + 1 < CAPE_W {
                                link(&mut cl.pts, at(r, c), at(r, c + 1), dx, r == 0, r == 0);
                            }
                            if r + 1 < CAPE_H {
                                link(&mut cl.pts, at(r, c), at(r + 1, c), dy, r == 0, false);
                            }
                            if r + 2 < CAPE_H {
                                link(&mut cl.pts, at(r, c), at(r + 2, c), dy * 2.0, r == 0, false);
                            }
                            if r + 1 < CAPE_H && c + 1 < CAPE_W {
                                let diag = (dx * dx + dy * dy).sqrt();
                                link(&mut cl.pts, at(r, c), at(r + 1, c + 1), diag, r == 0, false);
                                link(&mut cl.pts, at(r, c + 1), at(r + 1, c), diag, r == 0, false);
                            }
                        }
                    }
                    // Not through the player's back.
                    for k in CAPE_W..cl.pts.len() {
                        let d = (cl.pts[k] - top_mid).dot(into);
                        if d > -0.01 {
                            cl.pts[k] = cl.pts[k] - into * (d + 0.01);
                        }
                    }
                }
            }
        }
        // Draw: the outside with the outer face's texture, the inside with the inner's.
        let uv_of = |q: &[Vertex], k: usize| [f64::from(q[k].uv[0]), f64::from(q[k].uv[1])];
        let near = |p: V3| {
            (0..4)
                .min_by(|&a, &b| (pos(&iq[a]) - p).len().partial_cmp(&(pos(&iq[b]) - p).len()).unwrap_or(std::cmp::Ordering::Equal))
                .unwrap_or(0)
        };
        let o_uv = [uv_of(oq, t0), uv_of(oq, t1), uv_of(oq, b0), uv_of(oq, b1)];
        let i_uv = [
            uv_of(iq, near(pos(&oq[t0]))),
            uv_of(iq, near(pos(&oq[t1]))),
            uv_of(iq, near(pos(&oq[b0]))),
            uv_of(iq, near(pos(&oq[b1]))),
        ];
        let bilerp = |uv: &[[f64; 2]; 4], r: usize, c: usize| -> [f32; 2] {
            let (u, v) = (c as f64 / (CAPE_W - 1) as f64, r as f64 / (CAPE_H - 1) as f64);
            let top = [uv[0][0] + (uv[1][0] - uv[0][0]) * u, uv[0][1] + (uv[1][1] - uv[0][1]) * u];
            let bot = [uv[2][0] + (uv[3][0] - uv[2][0]) * u, uv[2][1] + (uv[3][1] - uv[2][1]) * u];
            [(top[0] + (bot[0] - top[0]) * v) as f32, (top[1] + (bot[1] - top[1]) * v) as f32]
        };
        let base = rest[0];
        let pts = &cl.pts;
        let at = |r: usize, c: usize| pts[r * CAPE_W + c];
        // The cloth's normal at each point, pointing outward (away from the back).
        let normal = |r: usize, c: usize| {
            let du = at(r, (c + 1).min(CAPE_W - 1)) - at(r, c.saturating_sub(1));
            let dv = at((r + 1).min(CAPE_H - 1), c) - at(r.saturating_sub(1), c);
            let n = du.cross(dv).norm();
            if n.dot(into) > 0.0 { -n } else { n }
        };
        let pack = |x: f64| (x.clamp(-1.0, 1.0) * 127.0) as i8;
        let thick = 0.06;
        let mut push = |p: V3, n: V3, uv: [f32; 2]| {
            let p = p - cam;
            out.push(Vertex {
                pos: [p.x as f32, p.y as f32, p.z as f32],
                uv,
                layer: base.layer,
                color: base.color,
                light: base.light,
                normal: [pack(n.x), pack(n.y), pack(n.z), base.normal[3]],
                glint_uv: base.glint_uv,
            });
        };
        for r in 0..CAPE_H - 1 {
            for c in 0..CAPE_W - 1 {
                let cells = [(r, c), (r, c + 1), (r + 1, c + 1), (r + 1, c)];
                // Outward winding: check against the normal.
                let (p0, p1, p2) = (at(r, c), at(r, c + 1), at(r + 1, c + 1));
                let n0 = normal(r, c);
                let out_order = if (p1 - p0).cross(p2 - p0).dot(n0) >= 0.0 { [0, 1, 2, 3] } else { [3, 2, 1, 0] };
                for k in out_order {
                    let (rr, cc) = cells[k];
                    push(at(rr, cc), normal(rr, cc), bilerp(&o_uv, rr, cc));
                }
                let in_order = [out_order[3], out_order[2], out_order[1], out_order[0]];
                for k in in_order {
                    let (rr, cc) = cells[k];
                    let n = normal(rr, cc);
                    push(at(rr, cc) - n * thick, -n, bilerp(&i_uv, rr, cc));
                }
            }
        }
        true
    })
}

fn self_hash(id: u32) -> f64 {
    f64::from(id.wrapping_mul(2_654_435_761) % 1000) * 0.01
}

/// Keeps points `a` and `b` no further apart than `len` (cloth stretches little and folds freely).
fn link(pts: &mut [V3], a: usize, b: usize, len: f64, a_pinned: bool, b_pinned: bool) {
    let d = pts[b] - pts[a];
    let l = d.len();
    if l < 1e-9 {
        return;
    }
    let (wa, wb) = (if a_pinned { 0.0 } else { 1.0 }, if b_pinned { 0.0 } else { 1.0 });
    let sum = wa + wb;
    if sum == 0.0 {
        return;
    }
    // Pulled back hard when stretched, softly when squeezed.
    let k = if l > len { 1.0 } else { 0.15 };
    let corr = d * ((l - len) / l * k);
    pts[a] = pts[a] + corr * (wa / sum);
    pts[b] = pts[b] - corr * (wb / sum);
}


// --- Liquids (beta) -------------------------------------------------------------------------

/// A drop of water or lava: a tiny sphere that falls, flows over blocks and keeps a little apart
/// from the others (position-based fluid, much simplified).
#[derive(Clone, Copy, Debug)]
pub struct Drop {
    pub x: V3,
    pub v: V3,
    pub age: f64,
    pub life: f64,
    pub size: f64,
    pub lava: bool,
    shade: f64,
}

const MAX_DROPS: usize = 400;
/// How close two drops may come.
const DROP_GAP: f64 = 0.12;

impl Env {
    fn add_drop(&mut self, x: V3, v: V3, lava: bool, life: f64) {
        if self.drops.len() >= MAX_DROPS {
            self.drops.remove(0);
        }
        let size = self.rng.range(0.035, 0.06);
        let shade = self.rng.next();
        self.drops.push(Drop { x, v, age: 0.0, life, size, lava, shade });
    }

    /// Water or lava was poured into the block at `pos`: a blob of it spills out over the block
    /// and its neighbours before it settles into the block.
    pub fn pour(&mut self, pos: [i32; 3], lava: bool) {
        let c = v3(f64::from(pos[0]) + 0.5, f64::from(pos[1]) + 0.75, f64::from(pos[2]) + 0.5);
        let n = ((if lava { 40.0 } else { 70.0 }) * f64::from(settings().liquid_amount)).round() as usize;
        for _ in 0..n {
            let d = self.rng.unit();
            let x = c + v3(d.x * 0.3, d.y.abs() * 0.2, d.z * 0.3);
            let v = v3(d.x * self.rng.range(0.4, 1.5), self.rng.range(-0.5, 1.0), d.z * self.rng.range(0.4, 1.5));
            let life = self.rng.range(1.6, 3.2) * if lava { 1.4 } else { 1.0 };
            self.add_drop(x, v, lava, life);
        }
    }

    /// Something fell into water at `at` with downward `speed` (blocks a second): a splash.
    /// White froth on the water at `at`, `n` puffs over `reach` blocks.
    fn foam(&mut self, at: V3, n: usize, reach: f64, size: f64) {
        for _ in 0..n {
            let rng = &mut self.rng;
            let a = rng.range(0.0, std::f64::consts::TAU);
            let r = reach * rng.next().sqrt();
            let x = at + v3(a.cos() * r, 0.0, a.sin() * r);
            let puff = Puff::new(FxKind::Foam, x, V3::default(), size * rng.range(0.7, 1.3), rng.range(1.2, 2.6), [0.93, 0.96, 0.98], [0.85, 0.9, 0.95], rng);
            self.puff(puff);
        }
    }

    pub fn splash(&mut self, at: V3, speed: f64) {
        let froth = (6.0 + speed * 2.0).min(24.0) as usize;
        self.foam(v3(at.x, at.y + 0.01, at.z), froth, 0.7, 0.16);
        let n = (12.0 + speed * 18.0).min(80.0) as usize;
        for _ in 0..n {
            let a = self.rng.range(0.0, std::f64::consts::TAU);
            let r = self.rng.range(0.2, 0.55);
            let x = at + v3(a.cos() * r, 0.05, a.sin() * r);
            let up = self.rng.range(2.0, 3.5) * (0.5 + speed * 0.12).min(1.4);
            let v = v3(a.cos() * self.rng.range(0.8, 2.0), up, a.sin() * self.rng.range(0.8, 2.0));
            let life = self.rng.range(1.0, 1.8);
            self.add_drop(x, v, false, life);
        }
    }

    /// An explosion of `radius` at `center` under water whose surface is at height `surface`: a
    /// spout of water thrown up through it, and a ring of froth. The deeper the blast, the weaker
    /// it breaks through (none from deeper than about three radii).
    pub fn water_boom(&mut self, center: V3, radius: f64, surface: f64) {
        let r = radius.clamp(0.5, 12.0);
        let depth = (surface - center.y).max(0.0);
        let power = (1.0 - depth / (r * 3.0 + 1.0)).clamp(0.0, 1.0);
        if power <= 0.05 {
            return;
        }
        let n = ((24.0 + r * 22.0) * power).round().min(220.0) as usize;
        for _ in 0..n {
            let a = self.rng.range(0.0, std::f64::consts::TAU);
            let reach = self.rng.range(0.0, r * 0.5).max(0.05);
            let x = v3(center.x + a.cos() * reach, surface + 0.05, center.z + a.sin() * reach);
            // Most of it goes straight up in a column; the rest fans out.
            let side = self.rng.range(0.1, 1.6) * (0.4 + power);
            let up = self.rng.range(5.0, 8.5 + r * 2.0) * power.sqrt();
            let v = v3(a.cos() * side, up, a.sin() * side);
            let life = self.rng.range(1.4, 2.6);
            self.add_drop(x, v, false, life);
        }
        let froth = ((10.0 + r * 6.0) * power).round() as usize;
        self.foam(v3(center.x, surface + 0.02, center.z), froth, r * 1.2 + 0.4, 0.22);
    }

    /// Dust over the blocks of a collapse: two puffs for a block (at most 80 blocks' worth),
    /// rising and spreading as they thin.
    pub fn collapse_dust(&mut self, blocks: &[Falling]) {
        let n = blocks.len().min(80);
        if n == 0 {
            return;
        }
        let stride = (blocks.len() / n).max(1);
        for b in blocks.iter().step_by(stride).take(n) {
            let at = v3(f64::from(b.pos[0]) + 0.5, f64::from(b.pos[1]) + 0.5, f64::from(b.pos[2]) + 0.5);
            for _ in 0..2 {
                let rng = &mut self.rng;
                let d = rng.unit();
                let g = rng.range(0.5, 0.65);
                let puff = Puff::new(FxKind::Dust, at + d * 0.4, v3(d.x * 0.8, rng.range(0.2, 0.9), d.z * 0.8), rng.range(0.25, 0.45), rng.range(1.4, 2.6), [g, g * 0.95, g * 0.88], [0.45, 0.43, 0.4], rng);
                self.puff(puff);
            }
        }
    }

    /// The player's feet this frame: in water or not, and how fast they move down.
    pub fn player_in_water(&mut self, feet: V3, in_water: bool, down: f64) {
        if in_water && !self.was_in_water && down > 3.0 {
            self.splash(v3(feet.x, feet.y.floor() + 0.9, feet.z), down);
        }
        self.was_in_water = in_water;
    }

    fn step_drops(&mut self, dt: f64, g: f64, world: &dyn World) {
        if self.drops.is_empty() || dt <= 0.0 {
            return;
        }
        let steps = 2;
        let h = dt.min(0.05) / steps as f64;
        for _ in 0..steps {
            let mut prev: Vec<V3> = Vec::with_capacity(self.drops.len());
            for d in &mut self.drops {
                prev.push(d.x);
                d.v.y -= g * h;
                d.x = d.x + d.v * h;
            }
            // Keep apart: a grid of cells one gap wide, each drop against its neighbours.
            let cell = |p: V3| ((p.x / DROP_GAP).floor() as i64, (p.y / DROP_GAP).floor() as i64, (p.z / DROP_GAP).floor() as i64);
            let mut grid: HashMap<(i64, i64, i64), Vec<usize>> = HashMap::new();
            for (i, d) in self.drops.iter().enumerate() {
                grid.entry(cell(d.x)).or_default().push(i);
            }
            let mut push = vec![V3::default(); self.drops.len()];
            for (i, d) in self.drops.iter().enumerate() {
                let (cx, cy, cz) = cell(d.x);
                for dx in -1..=1 {
                    for dy in -1..=1 {
                        for dz in -1..=1 {
                            let Some(list) = grid.get(&(cx + dx, cy + dy, cz + dz)) else { continue };
                            for &j in list {
                                if j <= i {
                                    continue;
                                }
                                let e = self.drops[j].x - d.x;
                                let l = e.len();
                                if l >= DROP_GAP || l < 1e-9 {
                                    continue;
                                }
                                let corr = e * ((DROP_GAP - l) / l * 0.5 * 0.5);
                                push[i] = push[i] - corr;
                                push[j] = push[j] + corr;
                            }
                        }
                    }
                }
            }
            for (k, d) in self.drops.iter_mut().enumerate() {
                d.x = d.x + push[k];
                let mut p = prev[k];
                let before = d.x;
                push_out(world, &mut d.x, &mut p);
                let touched = (d.x - before).len() > 1e-9;
                d.v = (d.x - prev[k]) * (1.0 / h);
                if touched {
                    // Water runs on along the ground; it barely bounces.
                    d.v = v3(d.v.x * 0.94, d.v.y.max(0.0) * 0.1, d.v.z * 0.94);
                }
            }
        }
        let mut landed: Vec<V3> = Vec::new();
        let mut i = 0;
        while i < self.drops.len() {
            let d = &mut self.drops[i];
            d.age += dt;
            // A drop that reaches standing water joins it (after its first moment: the block it
            // was poured into is water itself).
            let joined = d.age > 0.5 && !d.lava && matches!(world.cell(floor(d.x.x), floor(d.x.y), floor(d.x.z)), Cell::Fluid);
            if d.age >= d.life || joined || d.x.y < -100.0 {
                if joined && self.puffs.len() < MAX_PUFFS - 50 {
                    landed.push(d.x);
                }
                self.drops.swap_remove(i);
            } else {
                i += 1;
            }
        }
        // Where drops fell back into the water, a fleck of froth.
        for x in landed.into_iter().take(12) {
            if self.rng.next() < 0.35 {
                let top = x.y.floor() + 0.9;
                self.foam(v3(x.x, top, x.z), 1, 0.05, 0.08);
            }
        }
    }
}

pub fn liquids_on() -> bool {
    part_on(|s| s.liquids)
}

/// Water (or lava) was placed at `pos`.
pub fn pour(pos: [i32; 3], lava: bool) {
    if !liquids_on() {
        return;
    }
    with(|s| s.env.pour(pos, lava));
}

/// The player's feet: in water or not, and their speed down (blocks a second).
pub fn player_water(feet: [f64; 3], in_water: bool, down: f64) {
    if !liquids_on() || !settings().splashes {
        return;
    }
    with(|s| s.env.player_in_water(v3(feet[0], feet[1], feet[2]), in_water, down));
}

// ---------------------------------------------------------------------------------------------
// Quick setup: the live previews on its pages
// ---------------------------------------------------------------------------------------------
//
// Each panel of the quick setup shows its choice happening, with this module's own solver: a pig
// that dies (falling apart, as a ragdoll, in pieces), a stone block that breaks (whole, as small
// bits, as lumps, as shards), rain splashing on a pond, a pillar collapsing. A scene steps on its
// own clock and starts over every few seconds; `setup_quads` sees it through a camera of its own
// and hands back the quads to draw (screen position, depth, sprite and its part, colour).

/// What a preview shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scene {
    /// Mob deaths (`Settings::deaths`: 1 ragdoll, 2 blocky, 3 fractured).
    Mob(u8),
    /// A broken block (`Settings::block_style`).
    Block(u8),
    Weather,
    Collapse,
    /// The menu's backgrounds: open water with a boat, a snowfield with a trail, a burning tree,
    /// woods in the wind.
    Ocean,
    Snow,
    Fire,
    Trees,
}

/// The sprites a preview draws with; the screen finds them in the HUD atlas.
pub const SETUP_SPRITES: [&str; 12] = [
    "",
    "block/grass_block_top",
    "block/stone",
    "block/dirt",
    "block/oak_log",
    "block/water_still",
    "block/stone_bricks",
    "entity/pig/pig_temperate",
    "block/snow",
    "block/oak_leaves",
    "block/oak_planks",
    "block/fire_0",
];
const SP_FLAT: u8 = 0;
const SP_GRASS: u8 = 1;
const SP_STONE: u8 = 2;
const SP_DIRT: u8 = 3;
const SP_LOG: u8 = 4;
const SP_WATER: u8 = 5;
const SP_BRICKS: u8 = 6;
const SP_PIG: u8 = 7;
const SP_SNOW: u8 = 8;
const SP_LEAVES: u8 = 9;
const SP_PLANKS: u8 = 10;
const SETUP_FIRE: u8 = 11;

/// One quad to draw: its corners on the screen (x, y in GUI pixels, and depth: larger is nearer),
/// the sprite (`SETUP_SPRITES`, 0 a plain colour) and where in it each corner is (0..1), and its
/// colour (rgba, 0..1).
#[derive(Clone, Copy, Debug)]
pub struct SetupQuad {
    pub p: [[f32; 3]; 4],
    pub sprite: u8,
    pub uv: [[f32; 2]; 4],
    pub color: [f32; 4],
}

/// What each body of a scene looks like: its sprite, its box in block space (for which part of
/// the sprite each face shows), and its colour.
#[derive(Clone, Copy, Debug)]
struct Look {
    sprite: u8,
    tex: [f64; 6],
    rgb: [f64; 3],
    /// Each face's own part of the sprite (u0, v0, u1, v1, in 0..1), in `box_faces` order, for
    /// a mob's skin.
    faces: Option<[[f64; 4]; 6]>,
}

pub struct SetupScene {
    pub scene: Scene,
    /// How much closer the camera looks (the menu's full-screen pictures).
    pub zoom: f64,
    physics: Physics,
    looks: Vec<Look>,
    /// Seconds since it started.
    pub t: f64,
    broken: bool,
    rng: Rng,
    /// Rain: falling drops, and the spray of the ones that landed (position, velocity, age).
    rain: Vec<(V3, V3)>,
    spray: Vec<(V3, V3, f64)>,
}

/// The ground of a preview: grass at y 0 (and the pond's floor a block down).
struct SetupGround;

impl World for SetupGround {
    fn cell(&self, _: i32, y: i32, _: i32) -> Cell {
        if y < 0 { Cell::Full } else { Cell::Air }
    }
    fn light(&self, _: i32, _: i32, _: i32) -> u32 {
        15 * 16
    }
}

/// A rectangle of the 64 by 64 pig skin, as fractions.
const fn skin(u0: f64, v0: f64, u1: f64, v1: f64) -> [f64; 4] {
    [u0 / 64.0, v0 / 64.0, u1 / 64.0, v1 / 64.0]
}
/// A standing pig, as boxes (centre, half size, each face's part of the pig skin in
/// `box_faces` order: +x, -x, +y, -y, +z, -z) in block space, its feet at y 0, facing +x.
fn pig_parts() -> Vec<(V3, V3, [[f64; 4]; 6])> {
    let p = 1.0 / 16.0;
    let head = [skin(8.0, 8.0, 16.0, 16.0), skin(24.0, 8.0, 32.0, 16.0), skin(8.0, 0.0, 16.0, 8.0), skin(16.0, 0.0, 24.0, 8.0), skin(0.0, 8.0, 8.0, 16.0), skin(16.0, 8.0, 24.0, 16.0)];
    let body = [skin(36.0, 8.0, 46.0, 16.0), skin(46.0, 8.0, 56.0, 16.0), skin(36.0, 16.0, 46.0, 32.0), skin(54.0, 16.0, 64.0, 32.0), skin(28.0, 16.0, 36.0, 32.0), skin(46.0, 16.0, 54.0, 32.0)];
    let snout = [skin(17.0, 17.0, 21.0, 20.0); 6];
    let leg = [skin(4.0, 20.0, 8.0, 26.0), skin(12.0, 20.0, 16.0, 26.0), skin(4.0, 16.0, 8.0, 20.0), skin(8.0, 16.0, 12.0, 20.0), skin(0.0, 20.0, 4.0, 26.0), skin(8.0, 20.0, 12.0, 26.0)];
    vec![
        (v3(0.0, 9.0 * p, 0.0), v3(8.0 * p, 4.0 * p, 5.0 * p), body),
        (v3(10.0 * p, 12.0 * p, 0.0), v3(4.0 * p, 4.0 * p, 4.0 * p), head),
        (v3(14.5 * p, 10.5 * p, 0.0), v3(0.5 * p, 1.5 * p, 2.0 * p), snout),
        (v3(5.0 * p, 3.0 * p, 3.0 * p), v3(2.0 * p, 3.0 * p, 2.0 * p), leg),
        (v3(5.0 * p, 3.0 * p, -3.0 * p), v3(2.0 * p, 3.0 * p, 2.0 * p), leg),
        (v3(-5.0 * p, 3.0 * p, 3.0 * p), v3(2.0 * p, 3.0 * p, 2.0 * p), leg),
        (v3(-5.0 * p, 3.0 * p, -3.0 * p), v3(2.0 * p, 3.0 * p, 2.0 * p), leg),
    ]
}

/// The part of the cliff that collapses: blocks (position, sprite), stone under dirt.
fn pillar() -> Vec<([i32; 3], u8)> {
    let mut out = Vec::new();
    for y in 0..9 {
        for x in 0..3 {
            for z in 0..2 {
                out.push(([x, y, z], if y >= 7 { SP_DIRT } else { SP_STONE }));
            }
        }
    }
    out
}

/// The trees around the pond in the weather preview: trunk blocks and leaf blocks.
fn weather_trees() -> Vec<(V3, V3, u8, [f64; 3])> {
    let mut out = Vec::new();
    for (tx, tz, h) in [(-4.5, 7.5, 5), (3.5, 9.0, 6), (-8.0, 12.0, 5), (6.5, 5.5, 4), (0.5, 14.0, 6)] {
        for y in 0..h {
            out.push((v3(tx, f64::from(y) + 0.5, tz), v3(0.5, 0.5, 0.5), SP_LOG, [1.0, 1.0, 1.0]));
        }
        let top = f64::from(h);
        out.push((v3(tx, top + 0.2, tz), v3(1.8, 0.9, 1.8), SP_FLAT, [0.16, 0.34, 0.14]));
        out.push((v3(tx, top + 1.2, tz), v3(1.1, 0.6, 1.1), SP_FLAT, [0.18, 0.38, 0.16]));
    }
    out
}

impl SetupScene {
    pub fn new(scene: Scene, seed: u64) -> SetupScene {
        SetupScene {
            scene,
            zoom: 1.0,
            physics: Physics::default(),
            looks: Vec::new(),
            t: 0.0,
            broken: false,
            rng: Rng::new(seed),
            rain: Vec::new(),
            spray: Vec::new(),
        }
    }

    /// How long a scene runs before it starts over.
    fn length(&self) -> f64 {
        match self.scene {
            Scene::Weather | Scene::Ocean | Scene::Snow | Scene::Fire | Scene::Trees => f64::INFINITY,
            Scene::Collapse => 7.0,
            _ => 5.0,
        }
    }

    /// When the thing breaks (dies, collapses).
    fn moment(&self) -> f64 {
        match self.scene {
            Scene::Collapse => 1.2,
            _ => 0.8,
        }
    }

    fn add(&mut self, bodies: Vec<Body>, looks: Vec<Look>, joints: Vec<Joint>) {
        self.looks.extend(looks);
        self.physics.add(bodies, joints, 1.0e6, None);
    }

    /// Breaks the thing: the bodies of the scene's choice, with the game's own pieces and pushes.
    fn break_now(&mut self) {
        self.broken = true;
        match self.scene {
            Scene::Mob(mode) => {
                let parts = pig_parts();
                let mut bodies = Vec::new();
                let mut looks = Vec::new();
                let mid = v3(0.0, 0.5, 0.0);
                // Knocked away from the camera, a little up.
                let base = v3(0.3, 1.4, 1.1);
                for (c, h, rgb) in &parts {
                    if mode == 3 {
                        let size = *h * 2.0;
                        let n = ((size.x * size.y * size.z * 90.0).round() as usize).clamp(2, 7);
                        for unit in split_box([0.0, 0.0, 0.0, 1.0, 1.0, 1.0], n, &mut self.rng) {
                            let uc = v3((unit[0] + unit[3]) * 0.5, (unit[1] + unit[4]) * 0.5, (unit[2] + unit[5]) * 0.5);
                            let local = v3((uc.x - 0.5) * size.x, (uc.y - 0.5) * size.y, (uc.z - 0.5) * size.z);
                            let half = v3((unit[3] - unit[0]) * 0.5 * size.x, (unit[4] - unit[1]) * 0.5 * size.y, (unit[5] - unit[2]) * 0.5 * size.z);
                            let mut b = Body::new(*c + local, Quat::IDENTITY, half, Vec::new());
                            let out = (b.x - mid).norm();
                            let r = self.rng.unit();
                            b.v = base * 0.5 + out * 1.3 + r * 0.6 + v3(0.0, 0.8, 0.0);
                            b.w = self.rng.unit() * self.rng.range(2.0, 8.0);
                            bodies.push(b);
                            looks.push(Look { sprite: SP_PIG, tex: [0.0; 6], rgb: [1.0; 3], faces: Some(*rgb) });
                        }
                    } else {
                        let mut b = Body::new(*c, Quat::IDENTITY, *h, Vec::new());
                        if mode == 2 {
                            let out = (b.x - mid).norm();
                            let r = self.rng.unit();
                            b.v = base * 0.5 + out * 1.4 + r * 0.6 + v3(0.0, 0.8, 0.0);
                            b.w = self.rng.unit() * self.rng.range(2.0, 6.0);
                        } else {
                            // Knocked over onto its side: the whole pig already tipping about
                            // the feet on that side, turning on as one.
                            let pivot = v3(0.0, 0.0, 0.32);
                            let tip = Quat::axis_angle(v3(1.0, 0.0, 0.0), 0.45);
                            b.x = pivot + tip.rotate(b.x - pivot);
                            b.q = tip;
                            let w = v3(3.4, 0.0, 0.0);
                            b.v = w.cross(b.x - pivot) + v3(0.2, 0.3, 0.3);
                            b.w = w;
                        }
                        bodies.push(b);
                        looks.push(Look { sprite: SP_PIG, tex: [0.0; 6], rgb: [1.0; 3], faces: Some(*rgb) });
                    }
                }
                let mut joints = Vec::new();
                if mode == 1 {
                    // The head at the neck, each leg at its hip.
                    let p = 1.0 / 16.0;
                    let pivot = v3(0.0, 0.0, 0.32);
                    let tip = Quat::axis_angle(v3(1.0, 0.0, 0.0), 0.45);
                    let mut join = |a: usize, b: usize, at: V3| {
                        // In each part's own frame (the parts are turned by `tip`).
                        let at = pivot + tip.rotate(at - pivot);
                        let ra = bodies[a].q.conj().rotate(at - bodies[a].x);
                        let rb = bodies[b].q.conj().rotate(at - bodies[b].x);
                        joints.push(Joint { a, b, ra, rb });
                    };
                    join(0, 1, v3(7.0 * p, 11.0 * p, 0.0));
                    join(1, 2, v3(14.0 * p, 10.5 * p, 0.0));
                    for leg in 3..7 {
                        let c = parts[leg].0;
                        join(0, leg, v3(c.x, 6.0 * p, c.z));
                    }
                }
                self.add(bodies, looks, joints);
            }
            Scene::Block(style) => {
                let b = [0.0, 0.0, 0.0, 1.0, 1.0, 1.0];
                let mut bodies = Vec::new();
                let mut looks = Vec::new();
                let o = v3(-0.5, 0.0, -0.5);
                for (piece, q, v, w) in block_pieces(style, b, 1.0, 8, &mut self.rng) {
                    let c = v3((piece[0] + piece[3]) * 0.5, (piece[1] + piece[4]) * 0.5, (piece[2] + piece[5]) * 0.5);
                    let h = v3((piece[3] - piece[0]) * 0.5, (piece[4] - piece[1]) * 0.5, (piece[5] - piece[2]) * 0.5);
                    let mut body = Body::new(o + c, q, h, Vec::new());
                    body.v = v;
                    body.w = w;
                    bodies.push(body);
                    looks.push(Look { sprite: SP_STONE, tex: piece, rgb: [1.0, 1.0, 1.0], faces: None });
                }
                self.add(bodies, looks, Vec::new());
            }
            Scene::Collapse => {
                let mut bodies = Vec::new();
                let mut looks = Vec::new();
                let foot = v3(1.5, 0.0, 1.0);
                let push = v3(0.8, 0.0, -0.6).norm();
                let side = v3(-push.z, 0.0, push.x);
                for (p, sprite) in pillar() {
                    let o = v3(f64::from(p[0]), f64::from(p[1]), f64::from(p[2]));
                    let up = (o.y + 0.5 - foot.y).max(0.0);
                    // The bottom stays; the rest breaks off in lumps as it goes over.
                    if p[1] == 0 {
                        continue;
                    }
                    for (piece, q, _, _) in block_pieces(3, [0.0, 0.0, 0.0, 1.0, 1.0, 1.0], 1.0, 3, &mut self.rng) {
                        let c = v3((piece[0] + piece[3]) * 0.5, (piece[1] + piece[4]) * 0.5, (piece[2] + piece[5]) * 0.5);
                        let h = v3((piece[3] - piece[0]) * 0.5, (piece[4] - piece[1]) * 0.5, (piece[5] - piece[2]) * 0.5);
                        let mut body = Body::new(o + c, q, h, Vec::new());
                        body.v = push * (0.55 * up) + v3(self.rng.range(-0.3, 0.3), self.rng.range(-0.1, 0.4), self.rng.range(-0.3, 0.3));
                        body.w = side * (-0.6) + self.rng.unit() * self.rng.range(0.3, 1.5);
                        bodies.push(body);
                        looks.push(Look { sprite, tex: piece, rgb: [1.0, 1.0, 1.0], faces: None });
                    }
                }
                self.add(bodies, looks, Vec::new());
            }
            Scene::Weather | Scene::Ocean | Scene::Snow | Scene::Fire | Scene::Trees => {}
        }
    }

    /// Moves the scene on by `dt` seconds (and starts it over when its time is up).
    pub fn step(&mut self, dt: f64) {
        let dt = dt.clamp(0.0, 0.05);
        self.t += dt;
        if self.t > self.length() {
            let (scene, zoom) = (self.scene, self.zoom);
            let seed = self.rng.next().to_bits();
            *self = SetupScene::new(scene, seed);
            self.zoom = zoom;
            return;
        }
        if !self.broken && self.t >= self.moment() {
            self.break_now();
        }
        if self.broken {
            self.physics.step(dt, EARTH, &SetupGround);
        }
        if self.scene == Scene::Fire {
            // Smoke from the burning tree: puffs that rise, drift and spread (the spray holds
            // them: position, velocity, age).
            for _ in 0..((dt * 14.0).round().max(1.0) as usize) {
                let at = v3(2.5 + self.rng.range(-1.2, 1.2), self.rng.range(1.0, 4.5), 4.0 + self.rng.range(-1.2, 1.2));
                self.spray.push((at, v3(self.rng.range(0.2, 0.6), self.rng.range(1.0, 1.8), self.rng.range(-0.2, 0.2)), 0.0));
            }
            self.spray.retain_mut(|(x, v, age)| {
                *age += dt;
                *x = *x + *v * dt;
                *age < 5.0
            });
        }
        if self.scene == Scene::Weather {
            // Rain over the pond and the grass; each drop that lands splashes.
            for _ in 0..((dt * 520.0).round() as usize) {
                let x = self.rng.range(-9.0, 9.0);
                let z = self.rng.range(-2.0, 14.0);
                let at = v3(x + 2.0, self.rng.range(7.0, 10.0), z);
                self.rain.push((at, v3(-3.0, -18.0, 0.0)));
            }
            let mut landed = Vec::new();
            self.rain.retain_mut(|(x, v)| {
                *x = *x + *v * dt;
                if x.y <= 0.02 {
                    landed.push(v3(x.x, 0.02, x.z));
                    false
                } else {
                    true
                }
            });
            for at in landed {
                for _ in 0..3 {
                    let r = self.rng.unit();
                    let v = v3(r.x * 0.9, self.rng.range(1.2, 2.4), r.z * 0.9);
                    self.spray.push((at, v, 0.0));
                }
            }
            self.spray.retain_mut(|(x, v, age)| {
                *age += dt;
                v.y -= EARTH * 0.6 * dt;
                *x = *x + *v * dt;
                *age < 0.32 && x.y > 0.0
            });
        }
    }

    /// The scene through its camera, for a panel `w` by `h` GUI pixels whose middle is at
    /// (`cx`, `cy`). `focus` lights the panel up (the pointer is on it).
    pub fn quads(&self, cx: f32, cy: f32, w: f32, h: f32) -> Vec<SetupQuad> {
        let (eye, target, focal) = match self.scene {
            Scene::Mob(_) | Scene::Block(_) => (v3(0.0, 4.6, -6.0), v3(0.0, 0.3, 0.6), h as f64 * 1.1),
            Scene::Weather => (v3(0.0, 2.4, -5.5), v3(0.0, 1.2, 6.0), w as f64 * 0.55),
            Scene::Collapse => (v3(-3.0, 1.2, -10.0), v3(1.0, 3.8, 1.0), w as f64 * 0.46),
            Scene::Ocean => (v3(0.0, 1.8, -6.0), v3(-2.0, 0.7, 10.0), w as f64 * 0.55),
            Scene::Snow => (v3(0.0, 1.9, -5.0), v3(0.5, 0.2, 8.0), w as f64 * 0.55),
            Scene::Fire => (v3(-3.0, 2.4, -7.0), v3(2.0, 3.0, 4.0), w as f64 * 0.5),
            Scene::Trees => (v3(0.0, 2.2, -6.0), v3(0.5, 2.6, 8.0), w as f64 * 0.5),
        };
        // The menu's wide pictures are framed for the whole screen already; only the close ones
        // come closer.
        let zoom = if matches!(self.scene, Scene::Mob(_) | Scene::Block(_)) { self.zoom } else { 1.0 };
        let cam = SetupCamera::new(eye, target, focal * zoom, cx, cy);
        let mut out = Vec::new();
        let fog = match self.scene {
            Scene::Weather => [0.32, 0.38, 0.42],
            Scene::Collapse => [0.80, 0.84, 0.87],
            Scene::Ocean => [0.80, 0.85, 0.90],
            Scene::Snow => [0.80, 0.82, 0.90],
            Scene::Fire | Scene::Trees => [0.68, 0.78, 0.88],
            _ => [0.50, 0.62, 0.72],
        };
        // The grass: tiles out to the distance, fading into the haze.
        let (zx, zz) = match self.scene {
            Scene::Collapse => ((-14, 16), (-8, 26)),
            Scene::Weather => ((-14, 14), (-3, 26)),
            Scene::Ocean => ((-30, 30), (-4, 48)),
            Scene::Snow | Scene::Fire | Scene::Trees => ((-16, 16), (-3, 30)),
            _ => ((-8, 8), (-5, 22)),
        };
        let grass = match self.scene {
            Scene::Weather => [0.30, 0.44, 0.24],
            _ => [0.45, 0.66, 0.30],
        };
        // Water's surface: a slow swell.
        let t = self.t;
        let sea = |x: f64, z: f64| 0.12 * (x * 0.55 + t * 1.1).sin() + 0.08 * (z * 0.7 - t * 0.9 + x * 0.2).sin();
        // A trail through the snow, as the game presses it (an eighth of a block, a lip beside).
        let trail = |x: f64, z: f64| -> f64 {
            let path = 1.0 + 1.6 * (z * 0.35).sin();
            let shape = [path, z, 0.0, 1.0];
            let (dx, dz) = (x - shape[0], z - shape[1]);
            let q = (dx * dx + dz * dz).sqrt() / 0.45;
            if q < 0.3 {
                -0.125
            } else if q < 1.0 {
                let k = (q - 0.3) / 0.7;
                -0.125 * (1.0 - k * k * (3.0 - 2.0 * k))
            } else if q < 1.6 {
                0.125 * 0.18 * (std::f64::consts::PI * (q - 1.0) / 0.6).sin()
            } else {
                0.0
            }
        };
        for x in zx.0..zx.1 {
            for z in zz.0..zz.1 {
                if self.scene == Scene::Snow && (-3..5).contains(&x) && (-2..14).contains(&z) {
                    // Near the trail: a fine grid, the snow's height at each point.
                    let n = 4;
                    let step = 1.0 / f64::from(n);
                    for i in 0..n {
                        for j in 0..n {
                            let (x0, z0) = (f64::from(x) + f64::from(i) * step, f64::from(z) + f64::from(j) * step);
                            let pts = [(x0, z0), (x0, z0 + step), (x0 + step, z0 + step), (x0 + step, z0)];
                            let c = pts.map(|(px, pz)| v3(px, trail(px, pz), pz));
                            let nrm = (c[1] - c[0]).cross(c[2] - c[0]).norm();
                            let k = 0.5 + 0.5 * nrm.dot(v3(-0.6, 0.6, -0.5).norm()).max(0.0);
                            let haze = setup_haze(((c[0] + c[2]) * 0.5 - eye).len());
                            let rgb = mix3([0.95 * k, 0.96 * k, 1.0 * k], fog, haze);
                            let f = |a: f64| a as f32;
                            let uv = [[f(i as f64 * step), f(j as f64 * step)], [f(i as f64 * step), f((j + 1) as f64 * step)], [f((i + 1) as f64 * step), f((j + 1) as f64 * step)], [f((i + 1) as f64 * step), f(j as f64 * step)]];
                            cam.push(&mut out, c, SP_SNOW, uv, [rgb[0] as f32, rgb[1] as f32, rgb[2] as f32, 1.0]);
                        }
                    }
                    continue;
                }
                let pond = matches!(self.scene, Scene::Weather) && !matches!(self.scene, Scene::Snow) && (-5..5).contains(&x) && (1..9).contains(&z) && !(x == -5 && z == 8);
                let (sprite, rgb) = match self.scene {
                    Scene::Ocean => (SP_WATER, [0.30, 0.46, 0.72]),
                    Scene::Snow => (SP_SNOW, [0.86, 0.87, 0.93]),
                    _ if pond => (SP_WATER, [0.25, 0.36, 0.55]),
                    _ => (SP_GRASS, grass),
                };
                let lift = |px: f64, pz: f64| if self.scene == Scene::Ocean { sea(px, pz) } else { 0.0 };
                let (fx, fz) = (f64::from(x), f64::from(z));
                let c = [v3(fx, lift(fx, fz), fz), v3(fx, lift(fx, fz + 1.0), fz + 1.0), v3(fx + 1.0, lift(fx + 1.0, fz + 1.0), fz + 1.0), v3(fx + 1.0, lift(fx + 1.0, fz), fz)];
                let mid = (c[0] + c[2]) * 0.5;
                let haze = setup_haze((mid - eye).len());
                let rgb = mix3(rgb, fog, haze);
                cam.push(&mut out, c, sprite, [[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]], [rgb[0] as f32, rgb[1] as f32, rgb[2] as f32, 1.0]);
            }
        }
        let sun = v3(-0.45, 0.8, -0.5).norm();
        let shade = |n: V3| 0.58 + 0.42 * n.dot(sun).max(0.0);
        // What stands before it breaks.
        let mut boxes: Vec<(V3, Quat, V3, Look)> = Vec::new();
        if !self.broken {
            match self.scene {
                Scene::Mob(_) => {
                    for (c, h, rgb) in pig_parts() {
                        boxes.push((c, Quat::IDENTITY, h, Look { sprite: SP_PIG, tex: [0.0; 6], rgb: [1.0; 3], faces: Some(rgb) }));
                    }
                }
                Scene::Block(_) => boxes.push((v3(0.0, 0.5, 0.0), Quat::IDENTITY, v3(0.5, 0.5, 0.5), Look { sprite: SP_STONE, tex: [0.0, 0.0, 0.0, 1.0, 1.0, 1.0], rgb: [1.0; 3], faces: None })),
                Scene::Collapse => {}
                _ => {}
            }
        }
        if self.scene == Scene::Collapse {
            for (p, sprite) in pillar() {
                if !self.broken || p[1] == 0 {
                    let c = v3(f64::from(p[0]) + 0.5, f64::from(p[1]) + 0.5, f64::from(p[2]) + 0.5);
                    boxes.push((c, Quat::IDENTITY, v3(0.5, 0.5, 0.5), Look { sprite, tex: [0.0, 0.0, 0.0, 1.0, 1.0, 1.0], rgb: [1.0; 3], faces: None }));
                }
            }
            // The rock face it breaks from.
            for y in 0..10 {
                for x in -7..0 {
                    for z in 0..3 {
                        if z < 2 && x < -4 + y / 3 {
                            continue;
                        }
                        let sprite = if y >= 8 { SP_DIRT } else { SP_STONE };
                        boxes.push((v3(f64::from(x) + 0.5, f64::from(y) + 0.5, f64::from(z) + 0.5), Quat::IDENTITY, v3(0.5, 0.5, 0.5), Look { sprite, tex: [0.0, 0.0, 0.0, 1.0, 1.0, 1.0], rgb: [0.85; 3], faces: None }));
                    }
                }
            }
        }
        if self.scene == Scene::Weather {
            for (c, h, sprite, rgb) in weather_trees() {
                boxes.push((c, Quat::IDENTITY, h, Look { sprite, tex: [0.0, 0.0, 0.0, 1.0, 1.0, 1.0], rgb, faces: None }));
            }
        }
        let plain = |sprite: u8, rgb: [f64; 3]| Look { sprite, tex: [0.0, 0.0, 0.0, 1.0, 1.0, 1.0], rgb, faces: None };
        match self.scene {
            Scene::Ocean => {
                // A boat on the swell, rising, falling and tilting with it.
                let (bx, bz) = (-3.0, 9.0);
                let y = sea(bx, bz);
                let tilt_x = (sea(bx + 0.7, bz) - sea(bx - 0.7, bz)) / 1.4;
                let tilt_z = (sea(bx, bz + 0.7) - sea(bx, bz - 0.7)) / 1.4;
                let q = Quat::axis_angle(v3(0.0, 0.0, 1.0), tilt_x).mul(Quat::axis_angle(v3(1.0, 0.0, 0.0), -tilt_z));
                let o = v3(bx, y + 0.1, bz);
                let wood = plain(SP_PLANKS, [1.0; 3]);
                for (c, h) in [
                    (v3(0.0, 0.0, 0.0), v3(0.75, 0.06, 0.45)),
                    (v3(0.0, 0.22, 0.45), v3(0.75, 0.2, 0.06)),
                    (v3(0.0, 0.22, -0.45), v3(0.75, 0.2, 0.06)),
                    (v3(0.78, 0.22, 0.0), v3(0.06, 0.2, 0.45)),
                    (v3(-0.78, 0.22, 0.0), v3(0.06, 0.2, 0.45)),
                ] {
                    boxes.push((o + q.rotate(c), q, h, wood));
                }
            }
            Scene::Fire | Scene::Trees => {
                // Trees: trunks, and leaves that sway in the wind.
                let spots: &[(f64, f64, i32)] = if self.scene == Scene::Fire { &[(2.5, 4.0, 4), (-5.0, 12.0, 5), (7.0, 14.0, 5)] } else { &[(-3.0, 6.0, 5), (3.0, 9.0, 6), (-7.0, 13.0, 5), (6.5, 4.0, 4), (0.5, 16.0, 6)] };
                for &(tx, tz, ht) in spots {
                    for y in 0..ht {
                        boxes.push((v3(tx, f64::from(y) + 0.5, tz), Quat::IDENTITY, v3(0.5, 0.5, 0.5), plain(SP_LOG, [1.0; 3])));
                    }
                    let sway = 0.08 * (t * 1.3 + tx).sin();
                    let leaves = plain(SP_LEAVES, [0.36, 0.62, 0.24]);
                    for (dx, dy, dz) in [(0, 0, 0), (1, 0, 0), (-1, 0, 0), (0, 0, 1), (0, 0, -1), (1, 0, 1), (-1, 0, -1), (1, 0, -1), (-1, 0, 1), (0, 1, 0), (1, 1, 0), (-1, 1, 0), (0, 1, 1), (0, 1, -1), (0, 2, 0)] {
                        let c = v3(tx + f64::from(dx) + sway * f64::from(dy + 1), f64::from(ht) - 0.5 + f64::from(dy), tz + f64::from(dz));
                        boxes.push((c, Quat::IDENTITY, v3(0.5, 0.5, 0.5), leaves));
                    }
                }
            }
            _ => {}
        }
        for (i, b) in self.physics.bodies.iter().enumerate() {
            if let Some(look) = self.looks.get(i) {
                boxes.push((b.x, b.q, b.half, *look));
            }
        }
        for (c, q, h, look) in &boxes {
            // A soft shadow on the grass under it.
            if c.y - h.y < 2.5 && self.scene != Scene::Weather {
                let k = (1.0 - (c.y - h.y) / 2.5).clamp(0.0, 1.0) * 0.12;
                // Two squares turned against each other, twice: a soft round blot.
                for (grow, a) in [(1.35, 0.0), (1.35, 0.785), (0.95, 0.0), (0.95, 0.785)] {
                    let r = (h.x.max(h.z) * grow + 0.04) * std::f64::consts::SQRT_2;
                    let s = [0.0, 1.0, 2.0, 3.0].map(|i: f64| {
                        let t = a + std::f64::consts::FRAC_PI_4 + i * std::f64::consts::FRAC_PI_2;
                        v3(c.x + r * t.cos(), 0.004, c.z + r * t.sin())
                    });
                    cam.push(&mut out, [s[0], s[3], s[2], s[1]], SP_FLAT, [[0.0; 2]; 4], [0.0, 0.0, 0.0, k as f32]);
                }
            }
            let haze = setup_haze((*c - eye).len());
            for (fi, (n, corners)) in box_faces(*h).into_iter().enumerate() {
                let n_world = q.rotate(n);
                let world: [V3; 4] = corners.map(|l| *c + q.rotate(l));
                // Where on the sprite: the face's own part of the block it came from, or its own
                // part of a mob's skin.
                let tex = match look.faces {
                    Some(_) => [0.0, 0.0, 0.0, 1.0, 1.0, 1.0],
                    None => look.tex,
                };
                let uv = corners.map(|l| {
                    let f = |k: usize, v: f64| -> f64 {
                        let (lo, hi) = (tex[k], tex[k + 3]);
                        let half = [h.x, h.y, h.z][k].max(1e-6);
                        lo + (hi - lo) * ((v / half) * 0.5 + 0.5)
                    };
                    let (u, v) = if n.x != 0.0 {
                        (f(2, l.z), 1.0 - f(1, l.y))
                    } else if n.y != 0.0 {
                        (f(0, l.x), f(2, l.z))
                    } else {
                        (f(0, l.x), 1.0 - f(1, l.y))
                    };
                    let (u, v) = (u.clamp(0.0, 1.0), v.clamp(0.0, 1.0));
                    match look.faces {
                        Some(r) => {
                            let r = r[fi];
                            [(r[0] + (r[2] - r[0]) * u) as f32, (r[1] + (r[3] - r[1]) * v) as f32]
                        }
                        None => [u as f32, v as f32],
                    }
                });
                let k = shade(n_world);
                let rgb = mix3([look.rgb[0] * k, look.rgb[1] * k, look.rgb[2] * k], fog, haze);
                cam.push(&mut out, world, look.sprite, uv, [rgb[0] as f32, rgb[1] as f32, rgb[2] as f32, 1.0]);
            }
        }
        // Rain: thin streaks; the spray: little white-blue drops.
        for (x, v) in &self.rain {
            let a = *x;
            let b = *x - *v * 0.035;
            if let (Some(pa), Some(pb)) = (cam.project(a), cam.project(b)) {
                let width = 0.6f32;
                out.push(SetupQuad {
                    p: [[pa[0] - width, pa[1], pa[2]], [pb[0] - width, pb[1], pb[2]], [pb[0] + width, pb[1], pb[2]], [pa[0] + width, pa[1], pa[2]]],
                    sprite: SP_FLAT,
                    uv: [[0.0; 2]; 4],
                    color: [0.42, 0.52, 0.85, 0.55],
                });
            }
        }
        if self.scene == Scene::Fire {
            // Flames at the foot of the tree, then the smoke over everything.
            for (fx, fz) in [(2.0, 3.6), (3.0, 4.4), (2.6, 3.2)] {
                if let Some(p) = cam.project(v3(fx, 0.5, fz)) {
                    let r = (cam.focal / f64::from(-p[2]).max(0.3) * 0.5) as f32;
                    out.push(SetupQuad { p: [[p[0] - r, p[1] - r, p[2]], [p[0] - r, p[1] + r, p[2]], [p[0] + r, p[1] + r, p[2]], [p[0] + r, p[1] - r, p[2]]], sprite: SETUP_FIRE, uv: [[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]], color: [1.0, 1.0, 1.0, 0.98] });
                }
            }
            for (x, _, age) in &self.spray {
                if let Some(p) = cam.project(*x) {
                    let size = 0.5 + age * 0.55;
                    let r = (cam.focal / f64::from(-p[2]).max(0.3) * size) as f32;
                    let a = (0.28 * (1.0 - age / 5.0)).clamp(0.0, 0.28) as f32;
                    let g = (0.22 + age * 0.04) as f32;
                    out.push(SetupQuad { p: [[p[0] - r, p[1] - r, p[2]], [p[0] - r, p[1] + r, p[2]], [p[0] + r, p[1] + r, p[2]], [p[0] + r, p[1] - r, p[2]]], sprite: SP_FLAT, uv: [[0.0; 2]; 4], color: [g, g * 0.95, g * 0.9, a] });
                }
            }
            return out;
        }
        for (x, _, age) in &self.spray {
            if let Some(p) = cam.project(*x) {
                let r = (1.4 * (1.0 - age / 0.32)) as f32 + 0.5;
                out.push(SetupQuad {
                    p: [[p[0] - r, p[1] - r, p[2]], [p[0] - r, p[1] + r, p[2]], [p[0] + r, p[1] + r, p[2]], [p[0] + r, p[1] - r, p[2]]],
                    sprite: SP_FLAT,
                    uv: [[0.0; 2]; 4],
                    color: [0.55, 0.62, 0.95, 0.85],
                });
            }
        }
        out
    }
}

/// How much of the haze is between the eye and a point `d` blocks away.
fn setup_haze(d: f64) -> f64 {
    ((d - 9.0) / 22.0).clamp(0.0, 0.85)
}

/// A box's six faces (outward normal and four corners, counter-clockwise from outside), from
/// its half size.
fn box_faces(h: V3) -> [(V3, [V3; 4]); 6] {
    let (x, y, z) = (h.x, h.y, h.z);
    [
        (v3(1.0, 0.0, 0.0), [v3(x, -y, -z), v3(x, y, -z), v3(x, y, z), v3(x, -y, z)]),
        (v3(-1.0, 0.0, 0.0), [v3(-x, -y, z), v3(-x, y, z), v3(-x, y, -z), v3(-x, -y, -z)]),
        (v3(0.0, 1.0, 0.0), [v3(-x, y, -z), v3(-x, y, z), v3(x, y, z), v3(x, y, -z)]),
        (v3(0.0, -1.0, 0.0), [v3(-x, -y, z), v3(-x, -y, -z), v3(x, -y, -z), v3(x, -y, z)]),
        (v3(0.0, 0.0, 1.0), [v3(x, -y, z), v3(x, y, z), v3(-x, y, z), v3(-x, -y, z)]),
        (v3(0.0, 0.0, -1.0), [v3(-x, -y, -z), v3(-x, y, -z), v3(x, y, -z), v3(x, -y, -z)]),
    ]
}

/// A camera at `eye` looking at `target`, `focal` GUI pixels per unit at one block's distance,
/// the middle of its picture at (`cx`, `cy`).
struct SetupCamera {
    eye: V3,
    right: V3,
    up: V3,
    fwd: V3,
    focal: f64,
    cx: f32,
    cy: f32,
}

impl SetupCamera {
    fn new(eye: V3, target: V3, focal: f64, cx: f32, cy: f32) -> SetupCamera {
        let fwd = (target - eye).norm();
        let right = v3(0.0, 1.0, 0.0).cross(fwd).norm();
        let up = fwd.cross(right);
        SetupCamera { eye, right, up, fwd, focal, cx, cy }
    }

    /// Screen x, y and depth (larger: nearer) of a world point, None behind the camera.
    fn project(&self, p: V3) -> Option<[f32; 3]> {
        let d = p - self.eye;
        let z = d.dot(self.fwd);
        if z < 0.3 {
            return None;
        }
        let x = d.dot(self.right) / z * self.focal;
        let y = d.dot(self.up) / z * self.focal;
        Some([self.cx - x as f32, self.cy - y as f32, (-z) as f32])
    }

    /// A quad of the world, if it is in front of the camera and faces it.
    fn push(&self, out: &mut Vec<SetupQuad>, c: [V3; 4], sprite: u8, uv: [[f32; 2]; 4], color: [f32; 4]) {
        let n = (c[1] - c[0]).cross(c[2] - c[0]);
        if n.dot(self.eye - c[0]) <= 0.0 && color[3] >= 1.0 {
            return;
        }
        let mut p = [[0.0f32; 3]; 4];
        for k in 0..4 {
            let Some(q) = self.project(c[k]) else {
                return;
            };
            p[k] = q;
        }
        out.push(SetupQuad { p, sprite, uv, color });
    }
}

/// The quick setup's pages: (title, choices: (name, the scene it shows)). The last page has none:
/// it says the setup is done.
pub const SETUP_PAGES: usize = 4;

pub fn setup_page(page: usize) -> (&'static str, Vec<(&'static str, Scene)>) {
    match page {
        0 => ("Mobs", vec![("Blocky", Scene::Mob(2)), ("Ragdoll", Scene::Mob(1)), ("Fractured", Scene::Mob(3))]),
        1 => ("Blocks", vec![("Blocky", Scene::Block(0)), ("Particles", Scene::Block(1)), ("Crumbled Voxel", Scene::Block(2)), ("Fractured", Scene::Block(3))]),
        2 => ("Weather", vec![("Particles", Scene::Weather)]),
        3 => ("Collapsing Structures", vec![("On", Scene::Collapse)]),
        _ => ("", Vec::new()),
    }
}

/// The choice `pick` on `page` (None: Off), into the settings.
pub fn setup_choose(s: &mut Settings, page: usize, pick: Option<usize>) {
    match page {
        0 => s.deaths = pick.map_or(0, |i| [2, 1, 3][i.min(2)]),
        1 => match pick {
            Some(i) => {
                s.blocks = true;
                s.block_style = (i as u8).min(3);
            }
            None => s.blocks = false,
        },
        2 => s.weather = pick.is_some(),
        3 => s.collapse = pick.is_some(),
        _ => {}
    }
}

/// The quick setup's own state: the page it is on and its previews.
pub struct Setup {
    pub page: usize,
    pub scenes: Vec<SetupScene>,
    clock: Option<std::time::Instant>,
}

static SETUP: Mutex<Option<Setup>> = Mutex::new(None);

/// Whether the quick setup shows (the settings are read and it was neither done nor skipped).
pub fn setup_visible() -> bool {
    let g = SETTINGS.lock().unwrap_or_else(|e| e.into_inner());
    matches!(g.as_ref(), Some((s, Some(_))) if !s.setup)
}

/// Runs `f` on the quick setup, its previews moved on to now.
pub fn with_setup<R>(f: impl FnOnce(&mut Setup) -> R) -> R {
    let mut g = SETUP.lock().unwrap_or_else(|e| e.into_inner());
    let s = g.get_or_insert_with(|| Setup { page: 0, scenes: Vec::new(), clock: None });
    let now = std::time::Instant::now();
    let dt = s.clock.map_or(0.0, |c| now.duration_since(c).as_secs_f64());
    s.clock = Some(now);
    let want: Vec<Scene> = setup_page(s.page).1.iter().map(|c| c.1).collect();
    if s.scenes.iter().map(|c| c.scene).collect::<Vec<_>>() != want {
        s.scenes = want.iter().enumerate().map(|(i, sc)| SetupScene::new(*sc, 17 + i as u64 * 31)).collect();
    }
    for sc in &mut s.scenes {
        sc.step(dt);
    }
    f(s)
}

/// The quick setup: `pick` on its page (None: Off). Moves to the next page.
pub fn setup_pick(pick: Option<usize>) {
    with_setup(|s| {
        let page = s.page;
        update_settings(|st| setup_choose(st, page, pick));
        s.page = (s.page + 1).min(SETUP_PAGES);
    });
}

/// Ends the quick setup (Skip Setup, or Continue on its last page).
pub fn setup_finish() {
    update_settings(|st| st.setup = true);
    with_setup(|s| s.page = 0);
}

/// One thing the quick setup draws, in GUI pixels; colours are rgba bytes.
#[derive(Clone, Debug, PartialEq)]
pub enum SetupUi {
    Rect { x: f32, y: f32, w: f32, h: f32, color: [u8; 4] },
    Gradient { x: f32, y: f32, w: f32, h: f32, top: [u8; 4], bottom: [u8; 4] },
    /// Text centred on `x` (its top at `y`).
    Text { text: String, x: f32, y: f32, color: [u8; 4] },
    /// Preview `index` of the page, filling the rectangle (clipped to it).
    Scene { index: usize, x: f32, y: f32, w: f32, h: f32 },
    /// Text `scale` times the font's size, from `x` (`left`) or centred on it.
    Label { text: String, x: f32, y: f32, color: [u8; 4], scale: f32, left: bool },
    /// A sprite (`SETUP_SPRITES`) repeated over the rectangle, `size` pixels a tile, tinted.
    Tiles { x: f32, y: f32, w: f32, h: f32, sprite: u8, size: f32, color: [u8; 4] },
    /// What comes up to the next `Unclip` shows only inside the rectangle.
    Clip { x: f32, y: f32, w: f32, h: f32 },
    Unclip,
}

/// More things the quick setup and the Physics Settings screen draw, in GUI pixels.
impl SetupUi {
    fn label(text: impl Into<String>, x: f32, y: f32, color: [u8; 4], scale: f32, left: bool) -> SetupUi {
        SetupUi::Label { text: text.into(), x, y, color, scale, left }
    }
}

/// Where a press on the quick setup lands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SetupHit {
    None,
    Off,
    Skip,
    Pick(usize),
    Continue,
}

const UI_YELLOW: [u8; 4] = [255, 204, 64, 255];
const UI_WHITE: [u8; 4] = [255, 255, 255, 255];

struct SetupBox {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

impl SetupBox {
    fn has(&self, (x, y): (f32, f32)) -> bool {
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }
}

/// The quick setup's layout on a `w` by `h` screen.
fn setup_boxes(page: usize, w: f32, h: f32) -> (SetupBox, SetupBox, Vec<SetupBox>, SetupBox) {
    let off = SetupBox { x: (w * 0.016).round(), y: (h * 0.045).round(), w: (w * 0.159).round(), h: (h * 0.069).round() };
    let skip_w = (w * 0.165).round();
    let skip = SetupBox { x: w - (w * 0.016).round() - skip_w, y: off.y, w: skip_w, h: off.h };
    let n = setup_page(page).1.len().max(1);
    let col = w / n as f32;
    let top = (h * 0.15).round();
    let bottom = (h * 0.832).round();
    let panels = (0..n).map(|i| SetupBox { x: (col * i as f32).round(), y: top, w: (col * (i + 1) as f32).round() - (col * i as f32).round(), h: bottom - top }).collect();
    let cont = SetupBox { x: (w * 0.391).round(), y: (h * 0.679).round(), w: (w * 0.218).round(), h: (h * 0.079).round() };
    (off, skip, panels, cont)
}

fn setup_button(out: &mut Vec<SetupUi>, b: &SetupBox, text: &str, hover: bool) {
    out.push(SetupUi::Rect { x: b.x, y: b.y, w: b.w, h: b.h, color: UI_WHITE });
    let fill = if hover { [122, 140, 162, 255] } else { [72, 84, 98, 255] };
    out.push(SetupUi::Rect { x: b.x + 1.0, y: b.y + 1.0, w: b.w - 2.0, h: b.h - 2.0, color: fill });
    out.push(SetupUi::Text { text: text.to_string(), x: b.x + b.w * 0.5, y: b.y + (b.h - 8.0) * 0.5, color: UI_WHITE });
}

/// Everything the quick setup draws on page `page`, on a `w` by `h` screen with the pointer at
/// `cursor`.
pub fn setup_ui(page: usize, w: f32, h: f32, cursor: (f32, f32)) -> Vec<SetupUi> {
    let mut out = Vec::new();
    let (off, skip, panels, cont) = setup_boxes(page, w, h);
    if page >= SETUP_PAGES {
        // Done: the summary.
        out.push(SetupUi::Gradient { x: 0.0, y: 0.0, w, h, top: [22, 26, 40, 255], bottom: [46, 38, 54, 255] });
        let (x0, y0, x1, y1) = ((w * 0.133).round(), (h * 0.203).round(), (w * 0.867).round(), (h * 0.815).round());
        out.push(SetupUi::Rect { x: x0 + 3.0, y: y0 + 3.0, w: x1 - x0, h: y1 - y0, color: [0, 0, 0, 90] });
        out.push(SetupUi::Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0, color: UI_YELLOW });
        out.push(SetupUi::Rect { x: x0 + 1.0, y: y0 + 1.0, w: x1 - x0 - 2.0, h: y1 - y0 - 2.0, color: [42, 42, 36, 255] });
        // A bar chart: the settings, set.
        let cx = (w * 0.5).round();
        let base = (h * 0.31).round();
        for (dx, bh) in [(-14.0, 10.0), (-3.0, 17.0), (8.0, 13.0)] {
            out.push(SetupUi::Rect { x: cx + dx, y: base - bh, w: 6.0, h: bh, color: UI_YELLOW });
        }
        out.push(SetupUi::Text { text: "Quick setup complete".into(), x: cx, y: (h * 0.355).round(), color: UI_WHITE });
        out.push(SetupUi::Text { text: "You can find more detailed settings under".into(), x: cx, y: (h * 0.455).round(), color: [224, 224, 224, 255] });
        out.push(SetupUi::Text { text: "Options -> Mods -> Horizon Physics".into(), x: cx, y: (h * 0.508).round(), color: UI_YELLOW });
        out.push(SetupUi::Text { text: "You can change these choices there at any time.".into(), x: cx, y: (h * 0.566).round(), color: [224, 224, 224, 255] });
        setup_button(&mut out, &cont, "Continue", cont.has(cursor));
        return out;
    }
    let (name, choices) = setup_page(page);
    let top = panels[0].y;
    let bottom = panels[0].y + panels[0].h;
    let hovered = panels.iter().position(|p| p.has(cursor));
    // The previews fill the whole height; the bars lie over them.
    for (i, p) in panels.iter().enumerate() {
        let (top_sky, low_sky) = match choices.get(i).map(|c| c.1) {
            Some(Scene::Weather) => ([62, 72, 84, 255], [82, 96, 106, 255]),
            Some(Scene::Collapse) => ([112, 150, 205, 255], [205, 214, 222, 255]),
            _ => ([120, 160, 120, 255], [128, 158, 184, 255]),
        };
        out.push(SetupUi::Gradient { x: p.x, y: 0.0, w: p.w, h, top: top_sky, bottom: low_sky });
        out.push(SetupUi::Scene { index: i, x: p.x, y: 0.0, w: p.w, h });
        if hovered != Some(i) {
            out.push(SetupUi::Rect { x: p.x, y: 0.0, w: p.w, h, color: [0, 0, 0, 72] });
        }
    }
    out.push(SetupUi::Rect { x: 0.0, y: 0.0, w, h: top, color: [8, 10, 14, 150] });
    out.push(SetupUi::Rect { x: 0.0, y: bottom, w, h: h - bottom, color: [24, 27, 18, 236] });
    // Thin lines between the previews; the one under the pointer framed in white.
    for (i, p) in panels.iter().enumerate().skip(1) {
        let _ = i;
        out.push(SetupUi::Rect { x: p.x, y: top, w: 1.0, h: bottom - top, color: [190, 220, 150, 60] });
    }
    if let Some(i) = hovered
        && panels.len() > 1
    {
        let p = &panels[i];
        if i > 0 {
            out.push(SetupUi::Rect { x: p.x - 1.0, y: 0.0, w: 2.0, h, color: UI_WHITE });
        }
        if i + 1 < panels.len() {
            out.push(SetupUi::Rect { x: p.x + p.w - 1.0, y: 0.0, w: 2.0, h, color: UI_WHITE });
        }
    }
    out.push(SetupUi::Text { text: "Horizon Physics Quick Setup".into(), x: (w * 0.5).round(), y: (h * 0.049).round(), color: UI_WHITE });
    out.push(SetupUi::Text { text: name.into(), x: (w * 0.5).round(), y: (h * 0.098).round(), color: UI_WHITE });
    setup_button(&mut out, &off, "Off", off.has(cursor));
    setup_button(&mut out, &skip, "Skip Setup", skip.has(cursor));
    for (p, (label, _)) in panels.iter().zip(&choices) {
        let cx = (p.x + p.w * 0.5).round();
        out.push(SetupUi::Text { text: (*label).into(), x: cx, y: (h * 0.878).round(), color: UI_WHITE });
        out.push(SetupUi::Text { text: "FPS Impact: Low".into(), x: cx, y: (h * 0.93).round(), color: UI_YELLOW });
    }
    if choices.len() == 1 {
        out.push(SetupUi::Text { text: "Click anywhere to select".into(), x: (w * 0.5).round(), y: (h * 0.777).round(), color: UI_YELLOW });
    }
    out
}

/// What a press at (`x`, `y`) on page `page` of a `w` by `h` screen hits.
pub fn setup_hit(page: usize, w: f32, h: f32, x: f32, y: f32) -> SetupHit {
    let (off, skip, panels, cont) = setup_boxes(page, w, h);
    if page >= SETUP_PAGES {
        return if cont.has((x, y)) { SetupHit::Continue } else { SetupHit::None };
    }
    if off.has((x, y)) {
        return SetupHit::Off;
    }
    if skip.has((x, y)) {
        return SetupHit::Skip;
    }
    match panels.iter().position(|p| p.has((x, y))) {
        Some(i) => SetupHit::Pick(i),
        None => SetupHit::None,
    }
}

/// A press on the quick setup. Returns whether it hit anything (for the click's sound).
pub fn setup_click(w: f32, h: f32, x: f32, y: f32) -> bool {
    let page = with_setup(|s| s.page);
    match setup_hit(page, w, h, x, y) {
        SetupHit::Off => setup_pick(None),
        SetupHit::Pick(i) => setup_pick(Some(i)),
        SetupHit::Skip | SetupHit::Continue => setup_finish(),
        SetupHit::None => return false,
    }
    true
}


// ---------------------------------------------------------------------------------------------
// Physics Settings: the screen behind Options' "Physics Settings..." button
// ---------------------------------------------------------------------------------------------
//
// A column of categories on the left over a live picture of the one under the pointer, its
// description in a box at the bottom; Quick Setup and Back on the right. A category opens its
// settings page: rows of switches and sliders, then Reset, a page button and Done.

/// How wide a text is in the game's font, in GUI pixels (its glyphs' advances).
pub fn ui_text_width(text: &str) -> f32 {
    text.chars()
        .map(|c| match c {
            ' ' => 4.0,
            'i' | '!' | '.' | ',' | ':' | ';' | '|' | '\'' => 2.0,
            'l' | '`' => 3.0,
            't' | 'I' | '[' | ']' => 4.0,
            'f' | 'k' | '<' | '>' | '(' | ')' | '{' | '}' | '*' | '"' => 5.0,
            _ => 6.0,
        })
        .sum()
}

/// `text` broken into lines of at most `width` GUI pixels.
pub fn ui_wrap(text: &str, width: f32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split(' ') {
        let try_line = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
        if ui_text_width(&try_line) > width && !line.is_empty() {
            lines.push(std::mem::take(&mut line));
            line = word.to_string();
        } else {
            line = try_line;
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

/// A category: its name, what the box says about it, its picture, and its colour: 0 it has
/// settings (green), 1 not in the mod yet (red), 2 the credits (yellow).
pub struct Category {
    pub name: &'static str,
    pub title: &'static str,
    pub about: &'static str,
    pub scene: Scene,
    pub kind: u8,
}

pub const CATEGORIES: [Category; 15] = [
    Category { name: "GENERAL", title: "General Settings", about: "How many physics objects there can be at once, how fast the physics runs, and which of the game's own particles become physics pieces.", scene: Scene::Block(2), kind: 0 },
    Category { name: "MOBS", title: "Mob Settings", about: "Change what happens when mobs die. They fall as ragdolls, come apart into their blocky parts, or break into pieces of their own skin!", scene: Scene::Mob(1), kind: 0 },
    Category { name: "BLOCKS", title: "Block Settings", about: "Change how blocks crumble when they are broken: small bits, the whole block, lumps of its pixels or sharp shards. Your choice!", scene: Scene::Block(3), kind: 0 },
    Category { name: "OCEAN", title: "Ocean Settings", about: "Water that knows how deep and open it is: the open sea rolls in a swell that boats ride, rivers and lakes lap, puddles stay still.", scene: Scene::Ocean, kind: 0 },
    Category { name: "DYNAMIC BLOCKS", title: "Dynamic Block Settings", about: "Leaves, grass, flowers, crops and vines sway in the wind, plants bend as you walk through them, and doors swing open and shut, wobbling before they settle.", scene: Scene::Trees, kind: 0 },
    Category { name: "CLOTH", title: "Cloth Settings", about: "Capes and banners are cloth: capes trail behind you and lift when you run, banners blow out and ripple in the wind. Leads and fishing lines sag and swing.", scene: Scene::Trees, kind: 0 },
    Category { name: "PROJECTILES", title: "Projectile Settings", about: "Snowballs and eggs shatter into little pieces where they hit, and explosions blow blocks apart into pieces that fly.", scene: Scene::Block(1), kind: 0 },
    Category { name: "LIQUIDS", title: "Liquid Settings", about: "BETA! Water and lava you pour splash out as droplets that run over the blocks before they settle, and a fall into water splashes.", scene: Scene::Weather, kind: 0 },
    Category { name: "SNOW", title: "Snow Settings", about: "Snow layers give way where you and the mobs walk and leave a trail behind, and every step kicks up a little powder.", scene: Scene::Snow, kind: 0 },
    Category { name: "SOUNDS", title: "Sound Settings", about: "Hear the pieces land! Blocks clatter and ragdolls thump as they hit the ground, and the wind howls when it blows hard.", scene: Scene::Collapse, kind: 0 },
    Category { name: "ANIMATIONS", title: "Despawn Animation Settings", about: "Choose how pieces and dead mobs go when their time is up: they shrink away, sink into the ground, bounce, or go up in a puff. Or make your own!", scene: Scene::Block(1), kind: 0 },
    Category { name: "FIRE & SMOKE", title: "Fire & Smoke Settings", about: "Don't get stuck in a building without a chimney! Fire, campfires and lava give off soft smoke that rises, drifts and gathers under ceilings.", scene: Scene::Fire, kind: 0 },
    Category { name: "WEATHER", title: "Weather Settings", about: "Rain splashes up off the ground, snow drifts down, sand blows off the deserts, and the wind gets stronger with the weather and blows everything around.", scene: Scene::Weather, kind: 0 },
    Category { name: "COLLAPSE", title: "Collapse Settings", about: "CAREFUL! Break what holds a structure up and everything above it falls: trees fall over, bridges and towers come down.", scene: Scene::Collapse, kind: 0 },
    Category { name: "CREDITS", title: "Credits", about: "Horizon Physics by tariksabanuz. Inspired by PhysicsMod by Haubna; none of its code or pictures are used. The textures are your own game's.", scene: Scene::Mob(3), kind: 2 },
];

/// A setting on a page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    MaxObjects, McBreak, SimSpeed, Items, ItemSpin, Pvp, ItemBreak, Crack, Sprint, Eat, BlockParticles, ParticleLife, ParticleVar, Deaths, MobLife, MobVar, Anim, JointForce, JointBlood, RagdollLimit, BlockPhysics, Activation, BlockLife, ObjScale, BlockVar, CapeOn, CapeRange, HideCape, Ropes, ImpactVolume, WindVolume, WindForces, Weather, RainAmount, ThunderAmount, WindClear, WindRain, WindThunder, OpRain, OpSnow, OpDust, Collapse, CollapseDrops, CollapseTicks, CollapseLimit, Waves, WaveHeight, Boats, Foam, SwayLeaves, Plants, SwayVines, SwayWater, SwayStrength, PlantPush, WaveSpeed, Punch, Banners, Doors, Projectiles, Explosions, Liquids, LiquidAmount, Splashes, Snow, MobTrails, SnowDepth, SnowPowder, Smoke, Embers, SmokeAmount, SmokeOpacity, VanillaSmoke, AnimMotion, AnimParticle, AnimDuration, AnimAmount, Resolution, RagdollFix, WaterBoom, RodCurrent, SnowLayers, CollapseDust,
}

/// A slider's range and step.
fn slider(k: Key) -> Option<(f32, f32, f32)> {
    Some(match k {
        Key::MaxObjects => (100.0, 10000.0, 100.0),
        Key::SimSpeed => (0.1, 2.0, 0.05),
        Key::ItemSpin => (0.0, 3.0, 0.05),
        Key::ParticleLife => (0.0, 1.0, 0.01),
        Key::ParticleVar => (0.0, 10.0, 0.1),
        Key::MobLife => (0.0, 30.0, 0.1),
        Key::MobVar => (0.0, 10.0, 0.1),
        Key::JointForce => (0.0, 10.0, 0.05),
        Key::JointBlood => (0.0, 5.0, 0.05),
        Key::RagdollLimit => (1.0, 64.0, 1.0),
        Key::Activation => (8.0, 256.0, 1.0),
        Key::BlockLife => (0.0, 30.0, 0.1),
        Key::ObjScale => (0.25, 3.0, 0.05),
        Key::BlockVar => (0.0, 10.0, 0.1),
        Key::CapeRange => (4.0, 128.0, 1.0),
        Key::ImpactVolume => (0.0, 2.0, 0.05),
        Key::WindVolume => (0.0, 2.0, 0.05),
        Key::RainAmount => (0.0, 20.0, 1.0),
        Key::ThunderAmount => (0.0, 20.0, 1.0),
        Key::WindClear => (0.0, 3.0, 0.05),
        Key::WindRain => (0.0, 3.0, 0.05),
        Key::WindThunder => (0.0, 3.0, 0.05),
        Key::OpRain => (0.0, 1.0, 0.05),
        Key::OpSnow => (0.0, 1.0, 0.05),
        Key::OpDust => (0.0, 1.0, 0.05),
        Key::CollapseTicks => (0.0, 100.0, 1.0),
        Key::CollapseLimit => (10.0, 1000.0, 10.0),
        Key::WaveHeight => (0.0, 6.0, 0.05),
        Key::SwayStrength => (0.0, 3.0, 0.05),
        Key::WaveSpeed => (0.25, 6.0, 0.05),
        Key::LiquidAmount => (0.0, 3.0, 0.05),
        Key::SnowDepth => (0.1, 1.0, 0.05),
        Key::SmokeAmount => (0.0, 3.0, 0.05),
        Key::SmokeOpacity => (0.1, 2.0, 0.05),
        Key::AnimDuration => (0.05, 3.0, 0.05),
        Key::AnimAmount => (0.0, 32.0, 1.0),
        _ => return None,
    })
}

impl Key {
    pub fn label(self, s: &Settings) -> String {
        match self {
            Key::MaxObjects => format!("Physics Max Objects: {}", s.max_objects),
            Key::McBreak => format!("MC Break Particles: {}", on_upper(s.mc_break)),
            Key::SimSpeed => format!("Simulation Speed: {}", decimal(s.sim_speed)),
            Key::Items => format!("Dropped Item Physics: {}", on_upper(s.items)),
            Key::ItemSpin => format!("Dropped Item Rotation Speed: {}", decimal(s.item_spin)),
            Key::Pvp => format!("PVP Server Compatibility: {}", on_upper(s.pvp)),
            Key::ItemBreak => format!("Item Break Physics: {}", on_upper(s.item_break)),
            Key::Crack => format!("Crack Particle Physics: {}", on_upper(s.crack)),
            Key::Sprint => format!("Sprint Particle Physics: {}", on_upper(s.sprint)),
            Key::Eat => format!("Eat Particle Physics: {}", on_upper(s.eat)),
            Key::BlockParticles => format!("Block Particle Physics: {}", on_upper(s.block_particles)),
            Key::ParticleLife => format!("Lifetime (Particles): {}", decimal(s.particle_life)),
            Key::ParticleVar => format!("Lifetime Variance: {}", decimal(s.particle_var)),
            Key::MobLife => format!("Lifetime (Mobs): {}", decimal(s.mob_life)),
            Key::MobVar => format!("Lifetime Variance: {}", decimal(s.mob_var)),
            Key::JointForce => format!("Ragdoll Joint Break Force: {}", decimal(s.joint_force)),
            Key::JointBlood => format!("Ragdoll Joint Break Blood: {}", decimal(s.joint_blood)),
            Key::RagdollLimit => format!("Ragdoll Limit: {}", s.ragdoll_limit),
            Key::Activation => format!("Activation Distance: {}", decimal(s.activation)),
            Key::BlockLife => format!("Lifetime (Blocks): {}", decimal(s.block_life)),
            Key::ObjScale => format!("Physics Objects Scale: {}", decimal(s.obj_scale)),
            Key::BlockVar => format!("Lifetime Variance: {}", decimal(s.block_var)),
            Key::CapeOn => format!("Entity Cloth Physics: {}", on_upper(s.capes)),
            Key::CapeRange => format!("Entity Cloth Range: {}", decimal(s.cape_range)),
            Key::HideCape => format!("Hide Cloth when Armor equipped: {}", on_upper(s.hide_cape)),
            Key::Ropes => format!("Lead and Fishing Line Physics: {}", on_upper(s.ropes)),
            Key::ImpactVolume => format!("Physics Impact Volume: {}", decimal(s.impact_volume)),
            Key::WindVolume => format!("Wind Volume: {}", decimal(s.wind_volume)),
            Key::WindForces => format!("Wind Forces: {}", on_upper(s.wind_forces)),
            Key::Weather => format!("Weather Particles: {}", on_upper(s.weather)),
            Key::RainAmount => format!("Rain Weather Particle Amount: {}", s.rain_amount),
            Key::ThunderAmount => format!("Thunder Weather Particle Amount: {}", s.thunder_amount),
            Key::WindClear => format!("Clear Weather: {}", decimal(s.wind_clear)),
            Key::WindRain => format!("Rainy Weather: {}", decimal(s.wind_rain)),
            Key::WindThunder => format!("Thunder Weather: {}", decimal(s.wind_thunder)),
            Key::OpRain => format!("Rain: {}", decimal(s.op_rain)),
            Key::OpSnow => format!("Snow: {}", decimal(s.op_snow)),
            Key::OpDust => format!("Dust: {}", decimal(s.op_dust)),
            Key::Collapse => format!("Collapse Blocks: {}", on_upper(s.collapse)),
            Key::CollapseDrops => format!("Drop collapsed items: {}", on_upper(s.collapse_drops)),
            Key::CollapseTicks => format!("Collapse Speed (Ticks): {}", s.collapse_ticks),
            Key::CollapseLimit => format!("Collapse Limit (Blocks): {}", s.collapse_limit),
            Key::Waves => format!("Ocean Waves: {}", on_upper(s.waves)),
            Key::WaveHeight => format!("Wave Height: {}", decimal(s.wave_height)),
            Key::Boats => format!("Boat Physics: {}", on_upper(s.boats)),
            Key::Foam => format!("Ocean Foam: {}", on_upper(s.foam)),
            Key::SwayLeaves => format!("Swaying Leaves: {}", on_upper(s.sway_leaves)),
            Key::Plants => format!("Swaying Plants: {}", on_upper(s.plants)),
            Key::SwayVines => format!("Swaying Vines: {}", on_upper(s.sway_vines)),
            Key::SwayWater => format!("Swaying Water Plants: {}", on_upper(s.sway_water)),
            Key::SwayStrength => format!("Sway Strength: {}", decimal(s.sway_strength)),
            Key::PlantPush => format!("Walking Bends Plants: {}", on_upper(s.plant_push)),
            Key::WaveSpeed => format!("Wave Speed: {}", decimal(s.wave_speed)),
            Key::Punch => format!("Interactive Ragdolls: {}", on_upper(s.punch)),
            Key::Banners => format!("Banner Physics: {}", on_upper(s.banners)),
            Key::Doors => format!("Swinging Doors: {}", on_upper(s.doors)),
            Key::Projectiles => format!("Projectile Physics: {}", on_upper(s.projectiles)),
            Key::Explosions => format!("Explosion Physics: {}", on_upper(s.explosions)),
            Key::Liquids => format!("Liquid Physics (Beta): {}", on_upper(s.liquids)),
            Key::LiquidAmount => format!("Droplet Amount: {}", decimal(s.liquid_amount)),
            Key::Splashes => format!("Splash Physics: {}", on_upper(s.splashes)),
            Key::Snow => format!("Snow Physics: {}", on_upper(s.snow)),
            Key::MobTrails => format!("Mob Trails: {}", on_upper(s.mob_trails)),
            Key::SnowDepth => format!("Trail Depth: {}", decimal(s.snow_depth)),
            Key::SnowPowder => format!("Snow Powder: {}", on_upper(s.snow_powder)),
            Key::Smoke => format!("Smoke Physics: {}", on_upper(s.smoke)),
            Key::Embers => format!("Embers: {}", on_upper(s.embers)),
            Key::SmokeAmount => format!("Smoke Amount: {}", decimal(s.smoke_amount)),
            Key::SmokeOpacity => format!("Smoke Opacity: {}", decimal(s.smoke_opacity)),
            Key::VanillaSmoke => format!("Replace Vanilla Smoke: {}", on_upper(s.vanilla_smoke)),
            Key::AnimMotion => format!("Motion: {}", ANIM_MOTIONS[s.anims[anim_edit()].motion as usize % ANIM_MOTIONS.len()]),
            Key::AnimParticle => format!("Particles: {}", ANIM_PARTICLES[s.anims[anim_edit()].particle as usize % ANIM_PARTICLES.len()]),
            Key::AnimDuration => format!("Duration: {}", decimal(s.anims[anim_edit()].duration)),
            Key::AnimAmount => format!("Particle Amount: {}", s.anims[anim_edit()].amount),
            Key::Resolution => format!("Physics Resolution: {}", ["Normal", "High", "Ultra"][s.resolution as usize % 3]),
            Key::RagdollFix => format!("Big & Tiny Mob Ragdoll Fix: {}", on_upper(s.ragdoll_fix)),
            Key::WaterBoom => format!("Underwater Explosion Spout: {}", on_upper(s.water_boom)),
            Key::RodCurrent => format!("Fishing Line Follows Current: {}", on_upper(s.rod_current)),
            Key::SnowLayers => format!("Deep Snow Layers: {}", on_upper(s.snow_layers)),
            Key::CollapseDust => format!("Collapse Dust & Debris: {}", on_upper(s.collapse_dust)),
            Key::Deaths => format!("Mob Physics: {}", DEATHS[s.deaths as usize % 4]),
            Key::Anim => format!("Animation: {}", anim_name(s, s.anim as usize)),
            Key::BlockPhysics => format!("Block Physics: {}", if s.blocks { ["Blocky", "Particles", "Crumbled", "Fractured"][s.block_style as usize % 4] } else { "Off" }),
        }
    }

    /// A slider's value now, from 0 to 1.
    fn fraction(self, s: &Settings) -> f32 {
        let v = match self {
            Key::MaxObjects => s.max_objects as f32,
            Key::SimSpeed => s.sim_speed as f32,
            Key::ItemSpin => s.item_spin as f32,
            Key::ParticleLife => s.particle_life as f32,
            Key::ParticleVar => s.particle_var as f32,
            Key::MobLife => s.mob_life as f32,
            Key::MobVar => s.mob_var as f32,
            Key::JointForce => s.joint_force as f32,
            Key::JointBlood => s.joint_blood as f32,
            Key::RagdollLimit => s.ragdoll_limit as f32,
            Key::Activation => s.activation as f32,
            Key::BlockLife => s.block_life as f32,
            Key::ObjScale => s.obj_scale as f32,
            Key::BlockVar => s.block_var as f32,
            Key::CapeRange => s.cape_range as f32,
            Key::ImpactVolume => s.impact_volume as f32,
            Key::WindVolume => s.wind_volume as f32,
            Key::RainAmount => s.rain_amount as f32,
            Key::ThunderAmount => s.thunder_amount as f32,
            Key::WindClear => s.wind_clear as f32,
            Key::WindRain => s.wind_rain as f32,
            Key::WindThunder => s.wind_thunder as f32,
            Key::OpRain => s.op_rain as f32,
            Key::OpSnow => s.op_snow as f32,
            Key::OpDust => s.op_dust as f32,
            Key::CollapseTicks => s.collapse_ticks as f32,
            Key::CollapseLimit => s.collapse_limit as f32,
            Key::WaveHeight => s.wave_height as f32,
            Key::SwayStrength => s.sway_strength as f32,
            Key::WaveSpeed => s.wave_speed as f32,
            Key::LiquidAmount => s.liquid_amount as f32,
            Key::SnowDepth => s.snow_depth as f32,
            Key::SmokeAmount => s.smoke_amount as f32,
            Key::SmokeOpacity => s.smoke_opacity as f32,
            Key::AnimDuration => s.anims[anim_edit()].duration,
            Key::AnimAmount => f32::from(s.anims[anim_edit()].amount),
            _ => return 0.0,
        };
        let (lo, hi, _) = slider(self).unwrap_or((0.0, 1.0, 1.0));
        ((v - lo) / (hi - lo)).clamp(0.0, 1.0)
    }

    /// Sets a slider to `t` of its way (0 to 1), on its steps.
    pub fn slide(self, s: &mut Settings, t: f32) {
        let Some((lo, hi, step)) = slider(self) else {
            return;
        };
        let v = (lo + (hi - lo) * t.clamp(0.0, 1.0)) / step;
        let v = (v.round() * step).clamp(lo, hi);
        match self {
            Key::MaxObjects => s.max_objects = v.round() as u32,
            Key::SimSpeed => s.sim_speed = v,
            Key::ItemSpin => s.item_spin = v,
            Key::ParticleLife => s.particle_life = v,
            Key::ParticleVar => s.particle_var = v,
            Key::MobLife => s.mob_life = v,
            Key::MobVar => s.mob_var = v,
            Key::JointForce => s.joint_force = v,
            Key::JointBlood => s.joint_blood = v,
            Key::RagdollLimit => s.ragdoll_limit = v.round() as u32,
            Key::Activation => s.activation = v,
            Key::BlockLife => s.block_life = v,
            Key::ObjScale => s.obj_scale = v,
            Key::BlockVar => s.block_var = v,
            Key::CapeRange => s.cape_range = v,
            Key::ImpactVolume => s.impact_volume = v,
            Key::WindVolume => s.wind_volume = v,
            Key::RainAmount => s.rain_amount = v.round() as u32,
            Key::ThunderAmount => s.thunder_amount = v.round() as u32,
            Key::WindClear => s.wind_clear = v,
            Key::WindRain => s.wind_rain = v,
            Key::WindThunder => s.wind_thunder = v,
            Key::OpRain => s.op_rain = v,
            Key::OpSnow => s.op_snow = v,
            Key::OpDust => s.op_dust = v,
            Key::CollapseTicks => s.collapse_ticks = v.round() as u32,
            Key::CollapseLimit => s.collapse_limit = v.round() as u32,
            Key::WaveHeight => s.wave_height = v,
            Key::SwayStrength => s.sway_strength = v,
            Key::WaveSpeed => s.wave_speed = v,
            Key::LiquidAmount => s.liquid_amount = v,
            Key::SnowDepth => s.snow_depth = v,
            Key::SmokeAmount => s.smoke_amount = v,
            Key::SmokeOpacity => s.smoke_opacity = v,
            Key::AnimDuration => s.anims[anim_edit()].duration = v,
            Key::AnimAmount => s.anims[anim_edit()].amount = v.round() as u8,
            _ => {}
        }
    }

    /// A switch or a choice: the next value.
    pub fn press(self, s: &mut Settings) {
        match self {
            Key::McBreak => s.mc_break = !s.mc_break,
            Key::Items => s.items = !s.items,
            Key::Pvp => s.pvp = !s.pvp,
            Key::ItemBreak => s.item_break = !s.item_break,
            Key::Crack => s.crack = !s.crack,
            Key::Sprint => s.sprint = !s.sprint,
            Key::Eat => s.eat = !s.eat,
            Key::BlockParticles => s.block_particles = !s.block_particles,
            Key::CapeOn => s.capes = !s.capes,
            Key::HideCape => s.hide_cape = !s.hide_cape,
            Key::Ropes => s.ropes = !s.ropes,
            Key::WindForces => s.wind_forces = !s.wind_forces,
            Key::Weather => s.weather = !s.weather,
            Key::Collapse => s.collapse = !s.collapse,
            Key::CollapseDrops => s.collapse_drops = !s.collapse_drops,
            Key::Waves => s.waves = !s.waves,
            Key::Boats => s.boats = !s.boats,
            Key::Foam => s.foam = !s.foam,
            Key::SwayLeaves => s.sway_leaves = !s.sway_leaves,
            Key::Plants => s.plants = !s.plants,
            Key::SwayVines => s.sway_vines = !s.sway_vines,
            Key::SwayWater => s.sway_water = !s.sway_water,
            Key::PlantPush => s.plant_push = !s.plant_push,
            Key::Punch => s.punch = !s.punch,
            Key::Banners => s.banners = !s.banners,
            Key::Doors => s.doors = !s.doors,
            Key::Projectiles => s.projectiles = !s.projectiles,
            Key::Explosions => s.explosions = !s.explosions,
            Key::Liquids => s.liquids = !s.liquids,
            Key::Splashes => s.splashes = !s.splashes,
            Key::Snow => s.snow = !s.snow,
            Key::MobTrails => s.mob_trails = !s.mob_trails,
            Key::SnowPowder => s.snow_powder = !s.snow_powder,
            Key::Smoke => s.smoke = !s.smoke,
            Key::Embers => s.embers = !s.embers,
            Key::VanillaSmoke => s.vanilla_smoke = !s.vanilla_smoke,
            Key::Resolution => s.resolution = (s.resolution + 1) % 3,
            Key::RagdollFix => s.ragdoll_fix = !s.ragdoll_fix,
            Key::WaterBoom => s.water_boom = !s.water_boom,
            Key::RodCurrent => s.rod_current = !s.rod_current,
            Key::SnowLayers => s.snow_layers = !s.snow_layers,
            Key::CollapseDust => s.collapse_dust = !s.collapse_dust,
            Key::Deaths => s.deaths = (s.deaths + 1) % 4,
            Key::AnimMotion => { let a = &mut s.anims[anim_edit()]; a.motion = (a.motion + 1) % ANIM_MOTIONS.len() as u8; }
            Key::AnimParticle => { let a = &mut s.anims[anim_edit()]; a.particle = (a.particle + 1) % ANIM_PARTICLES.len() as u8; }
            Key::Anim => s.anim = next_anim(s, s.anim),
            Key::BlockPhysics => {
                if !s.blocks {
                    s.blocks = true;
                    s.block_style = 0;
                } else if s.block_style >= 3 {
                    s.blocks = false;
                } else {
                    s.block_style += 1;
                }
            }
            _ => {}
        }
    }

    /// Back to how the mod comes.
    pub fn reset(self, s: &mut Settings) {
        let d = Settings::default();
        match self {
            Key::MaxObjects => s.max_objects = d.max_objects,
            Key::McBreak => s.mc_break = d.mc_break,
            Key::SimSpeed => s.sim_speed = d.sim_speed,
            Key::Items => s.items = d.items,
            Key::ItemSpin => s.item_spin = d.item_spin,
            Key::Pvp => s.pvp = d.pvp,
            Key::ItemBreak => s.item_break = d.item_break,
            Key::Crack => s.crack = d.crack,
            Key::Sprint => s.sprint = d.sprint,
            Key::Eat => s.eat = d.eat,
            Key::BlockParticles => s.block_particles = d.block_particles,
            Key::ParticleLife => s.particle_life = d.particle_life,
            Key::ParticleVar => s.particle_var = d.particle_var,
            Key::Deaths => s.deaths = d.deaths,
            Key::MobLife => s.mob_life = d.mob_life,
            Key::MobVar => s.mob_var = d.mob_var,
            Key::Anim => s.anim = d.anim,
            Key::JointForce => s.joint_force = d.joint_force,
            Key::JointBlood => s.joint_blood = d.joint_blood,
            Key::RagdollLimit => s.ragdoll_limit = d.ragdoll_limit,
            Key::BlockPhysics => { s.blocks = d.blocks; s.block_style = d.block_style; }
            Key::Activation => s.activation = d.activation,
            Key::BlockLife => s.block_life = d.block_life,
            Key::ObjScale => s.obj_scale = d.obj_scale,
            Key::BlockVar => s.block_var = d.block_var,
            Key::CapeOn => s.capes = d.capes,
            Key::CapeRange => s.cape_range = d.cape_range,
            Key::HideCape => s.hide_cape = d.hide_cape,
            Key::Ropes => s.ropes = d.ropes,
            Key::ImpactVolume => s.impact_volume = d.impact_volume,
            Key::WindVolume => s.wind_volume = d.wind_volume,
            Key::WindForces => s.wind_forces = d.wind_forces,
            Key::Weather => s.weather = d.weather,
            Key::RainAmount => s.rain_amount = d.rain_amount,
            Key::ThunderAmount => s.thunder_amount = d.thunder_amount,
            Key::WindClear => s.wind_clear = d.wind_clear,
            Key::WindRain => s.wind_rain = d.wind_rain,
            Key::WindThunder => s.wind_thunder = d.wind_thunder,
            Key::OpRain => s.op_rain = d.op_rain,
            Key::OpSnow => s.op_snow = d.op_snow,
            Key::OpDust => s.op_dust = d.op_dust,
            Key::Collapse => s.collapse = d.collapse,
            Key::CollapseDrops => s.collapse_drops = d.collapse_drops,
            Key::CollapseTicks => s.collapse_ticks = d.collapse_ticks,
            Key::CollapseLimit => s.collapse_limit = d.collapse_limit,
            Key::Waves => s.waves = d.waves,
            Key::WaveHeight => s.wave_height = d.wave_height,
            Key::Boats => s.boats = d.boats,
            Key::Foam => s.foam = d.foam,
            Key::SwayLeaves => s.sway_leaves = d.sway_leaves,
            Key::Plants => s.plants = d.plants,
            Key::SwayVines => s.sway_vines = d.sway_vines,
            Key::SwayWater => s.sway_water = d.sway_water,
            Key::SwayStrength => s.sway_strength = d.sway_strength,
            Key::PlantPush => s.plant_push = d.plant_push,
            Key::WaveSpeed => s.wave_speed = d.wave_speed,
            Key::Punch => s.punch = d.punch,
            Key::Banners => s.banners = d.banners,
            Key::Doors => s.doors = d.doors,
            Key::Projectiles => s.projectiles = d.projectiles,
            Key::Explosions => s.explosions = d.explosions,
            Key::Liquids => s.liquids = d.liquids,
            Key::LiquidAmount => s.liquid_amount = d.liquid_amount,
            Key::Splashes => s.splashes = d.splashes,
            Key::Snow => s.snow = d.snow,
            Key::MobTrails => s.mob_trails = d.mob_trails,
            Key::SnowDepth => s.snow_depth = d.snow_depth,
            Key::SnowPowder => s.snow_powder = d.snow_powder,
            Key::Smoke => s.smoke = d.smoke,
            Key::Embers => s.embers = d.embers,
            Key::SmokeAmount => s.smoke_amount = d.smoke_amount,
            Key::SmokeOpacity => s.smoke_opacity = d.smoke_opacity,
            Key::VanillaSmoke => s.vanilla_smoke = d.vanilla_smoke,
            Key::Resolution => s.resolution = d.resolution,
            Key::RagdollFix => s.ragdoll_fix = d.ragdoll_fix,
            Key::WaterBoom => s.water_boom = d.water_boom,
            Key::RodCurrent => s.rod_current = d.rod_current,
            Key::SnowLayers => s.snow_layers = d.snow_layers,
            Key::CollapseDust => s.collapse_dust = d.collapse_dust,
            Key::AnimMotion | Key::AnimParticle | Key::AnimDuration | Key::AnimAmount => { let e = anim_edit(); s.anims[e] = anim_default(e); }
        }
    }
}

/// A line of a settings page: one setting wide or two side by side, a heading over the settings
/// below it, or a note in yellow.
#[derive(Clone, Debug, PartialEq)]
pub enum Row {
    Keys(Vec<Key>),
    Header(&'static str),
    Note(&'static str),
}

/// What a category's settings page holds, top to bottom.
pub fn page_rows(cat: usize) -> Vec<Row> {
    use Key::*;
    use Row::{Header, Keys, Note};
    match cat {
        0 => vec![Keys(vec![MaxObjects, McBreak]), Keys(vec![SimSpeed]), Keys(vec![Items]), Keys(vec![ItemSpin]), Keys(vec![Pvp]), Keys(vec![ItemBreak]), Keys(vec![Crack, Sprint]), Keys(vec![Eat, BlockParticles]), Keys(vec![ParticleLife, ParticleVar])],
        1 => vec![Keys(vec![Deaths, MobLife]), Keys(vec![MobVar, Anim]), Keys(vec![JointForce]), Keys(vec![JointBlood]), Keys(vec![RagdollLimit]), Keys(vec![Punch]), Keys(vec![RagdollFix])],
        2 => vec![Keys(vec![BlockPhysics, Activation]), Keys(vec![BlockLife, ObjScale]), Keys(vec![BlockVar, Anim]), Keys(vec![Resolution])],
        3 => vec![Keys(vec![Waves, Boats]), Keys(vec![WaveHeight]), Keys(vec![WaveSpeed]), Keys(vec![Foam]), Note("Waves need a Horizon shader pack or the mod's own look.")],
        4 => vec![Keys(vec![SwayLeaves, Plants]), Keys(vec![SwayVines]), Keys(vec![SwayWater]), Keys(vec![SwayStrength]), Keys(vec![PlantPush, Doors]), Note("The wind's strength per weather is on the Weather page.")],
        5 => vec![Keys(vec![CapeOn, CapeRange]), Keys(vec![HideCape]), Keys(vec![Banners]), Note("Leads and fishing lines hang and swing as ropes:"), Keys(vec![Ropes]), Keys(vec![RodCurrent])],
        6 => vec![Keys(vec![Projectiles]), Keys(vec![Explosions]), Keys(vec![WaterBoom])],
        7 => vec![Keys(vec![Liquids]), Keys(vec![LiquidAmount]), Keys(vec![Splashes])],
        8 => vec![Keys(vec![Snow, MobTrails]), Keys(vec![SnowDepth]), Keys(vec![SnowPowder]), Keys(vec![SnowLayers])],
        9 => vec![Keys(vec![ImpactVolume, WindVolume])],
        10 => vec![Keys(vec![Anim])],
        11 => vec![Keys(vec![Smoke, Embers]), Keys(vec![SmokeAmount]), Keys(vec![SmokeOpacity]), Keys(vec![VanillaSmoke])],
        12 => vec![
            Keys(vec![WindForces, Weather]),
            Keys(vec![RainAmount]),
            Keys(vec![ThunderAmount]),
            Header("Wind Strength"),
            Keys(vec![WindClear]),
            Keys(vec![WindRain]),
            Keys(vec![WindThunder]),
            Header("Particle Opacities"),
            Keys(vec![OpRain]),
            Keys(vec![OpSnow]),
            Keys(vec![OpDust]),
        ],
        13 => vec![Keys(vec![Collapse, CollapseDrops]), Keys(vec![CollapseTicks]), Keys(vec![CollapseLimit]), Keys(vec![CollapseDust])],
        _ => Vec::new(),
    }
}

/// A category's settings, row by row (the headings and notes left out).
pub fn category_rows(cat: usize) -> Vec<Vec<Key>> {
    page_rows(cat)
        .into_iter()
        .filter_map(|r| match r {
            Row::Keys(k) => Some(k),
            _ => None,
        })
        .collect()
}

/// The despawn animation list's category.
pub const ANIM_PAGE: usize = 10;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuView {
    Main,
    Page(usize),
    Setup,
    /// The animation editor, on a slot.
    Anim(usize),
}

pub struct Menu {
    pub open: bool,
    pub view: MenuView,
    pub scroll: f32,
    /// How far a page is scrolled down.
    pub page_scroll: f32,
    /// The slider being dragged (row, column).
    drag: Option<(usize, usize)>,
    /// The category whose picture shows, and the picture.
    pub shown: usize,
    pub picture: SetupScene,
    clock: Option<std::time::Instant>,
    /// How lit up each button is (0 to 1, easing towards the pointer), by button.
    glow: HashMap<u32, f32>,
    pub ui_clock: Option<std::time::Instant>,
    /// Seconds since the view, and since the category shown, changed (things slide in).
    age: f32,
    about_age: f32,
    /// No easing: every frame shows where things end (the tests).
    pub instant: bool,
}

static MENU: Mutex<Option<Menu>> = Mutex::new(None);

impl Menu {
    fn new() -> Menu {
        let mut picture = SetupScene::new(CATEGORIES[0].scene, 3);
        picture.zoom = MENU_ZOOM;
        Menu {
            open: false,
            view: MenuView::Main,
            scroll: 0.0,
            page_scroll: 0.0,
            drag: None,
            shown: 0,
            picture,
            clock: None,
            glow: HashMap::new(),
            ui_clock: None,
            age: 0.0,
            about_age: 0.0,
            instant: false,
        }
    }

    #[doc(hidden)]
    pub fn for_test() -> Menu {
        let mut m = Menu::new();
        m.open = true;
        m.instant = true;
        m.age = 10.0;
        m.about_age = 10.0;
        m
    }

    /// Goes to `view`, at the top of it.
    fn go(&mut self, view: MenuView) {
        self.view = view;
        self.page_scroll = 0.0;
        self.drag = None;
        self.age = 0.0;
    }

    /// Seconds since the last frame drawn (none in the tests).
    fn frame_dt(&mut self) -> f32 {
        let now = std::time::Instant::now();
        let dt = self.ui_clock.map_or(0.0, |c| now.duration_since(c).as_secs_f32()).min(0.1);
        self.ui_clock = Some(now);
        if self.instant { 10.0 } else { dt }
    }

    /// Button `id` lit up as far as it got towards `on` (eased, 0 to 1).
    fn lit(&mut self, id: u32, on: bool, dt: f32) -> f32 {
        let g = self.glow.entry(id).or_insert(0.0);
        let target = if on { 1.0 } else { 0.0 };
        *g += (target - *g) * (dt * 14.0).min(1.0);
        if (*g - target).abs() < 0.002 {
            *g = target;
        }
        ease(*g)
    }
}

/// Smooth in and out, 0 to 1.
fn ease(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Between colours `a` and `b`, `t` of the way.
fn mix_rgba(a: [u8; 4], b: [u8; 4], t: f32) -> [u8; 4] {
    std::array::from_fn(|k| (f32::from(a[k]) + (f32::from(b[k]) - f32::from(a[k])) * t.clamp(0.0, 1.0)).round() as u8)
}

/// Runs `f` on the Physics Settings screen, its picture moved on to now.
pub fn with_menu<R>(f: impl FnOnce(&mut Menu) -> R) -> R {
    let mut g = MENU.lock().unwrap_or_else(|e| e.into_inner());
    let m = g.get_or_insert_with(Menu::new);
    if m.picture.zoom != MENU_ZOOM {
        m.picture.zoom = MENU_ZOOM;
    }
    let now = std::time::Instant::now();
    let dt = m.clock.map_or(0.0, |c| now.duration_since(c).as_secs_f64());
    m.clock = Some(now);
    m.picture.step(dt);
    f(m)
}

pub fn menu_is_open() -> bool {
    MENU.lock().map(|g| g.as_ref().is_some_and(|m| m.open)).unwrap_or(false)
}

pub fn menu_open() {
    with_menu(|m| {
        m.open = true;
        m.go(MenuView::Main);
        m.glow.clear();
    });
}

pub fn menu_close() {
    with_menu(|m| {
        m.open = false;
        m.drag = None;
    });
}

/// The view the screen shows (the quick setup draws itself).
pub fn menu_view() -> MenuView {
    with_menu(|m| m.view)
}

/// The menu's pictures: closer than the quick setup's panels.
const MENU_ZOOM: f64 = 1.9;
const TAB_W: f32 = 120.0;
const TAB_H: f32 = 24.0;
const TAB_STEP: f32 = 30.0;
const TAB_TOP: f32 = 25.0;

fn tab_box(i: usize, scroll: f32, hover: bool) -> SetupBox {
    SetupBox { x: 0.0, y: TAB_TOP + TAB_STEP * i as f32 - scroll, w: if hover { TAB_W + 14.0 } else { TAB_W }, h: TAB_H }
}

fn menu_buttons(w: f32) -> (SetupBox, SetupBox) {
    (SetupBox { x: w - 90.0, y: TAB_TOP, w: 90.0, h: TAB_H }, SetupBox { x: w - 45.0, y: TAB_TOP + TAB_STEP, w: 45.0, h: TAB_H })
}

fn max_scroll(h: f32) -> f32 {
    (TAB_TOP + TAB_STEP * CATEGORIES.len() as f32 + 8.0 - h).max(0.0)
}

const BAR_GREEN: [u8; 4] = [110, 232, 40, 255];
const BAR_RED: [u8; 4] = [240, 84, 60, 255];
const BAR_YELLOW: [u8; 4] = [250, 214, 30, 255];
const BAR_LIGHT: [u8; 4] = [214, 214, 214, 255];
const NOTE_YELLOW: [u8; 4] = [255, 255, 85, 255];
const GREY_TEXT: [u8; 4] = [130, 130, 130, 255];

/// A tab of the menu: its fill, the bar on its `left` or right edge, its name.
fn menu_tab(out: &mut Vec<SetupUi>, b: &SetupBox, name: &str, bar: [u8; 4], left: bool, lit: f32) {
    let fill = mix_rgba([26, 30, 40, 185], [70, 74, 86, 205], lit);
    out.push(SetupUi::Rect { x: b.x, y: b.y, w: b.w, h: b.h, color: fill });
    // The bar on its edge widens a little as it lights up.
    let bw = 3.0 + lit;
    let bx = if left { b.x } else { b.x + b.w - bw };
    out.push(SetupUi::Rect { x: bx, y: b.y, w: bw, h: b.h, color: bar });
    let scale = 1.35;
    let tx = if left { b.x + b.w - 7.0 - ui_text_width(name) * scale - 2.0 * lit } else { b.x + 7.0 + 2.0 * lit };
    out.push(SetupUi::label(name, tx, b.y + (b.h - 8.0 * scale) * 0.5 + 1.0, UI_WHITE, scale, true));
}

/// The top of a page's scrolling part, and its bottom on a screen `h` high.
const PAGE_TOP: f32 = 40.0;

fn page_bottom(h: f32) -> f32 {
    h - 33.0
}

/// One thing on a page, laid out.
#[derive(Clone, Debug)]
enum PageItem {
    /// A setting, and where it is (row, column).
    Key(Key, (usize, usize)),
    Header(&'static str),
    Note(Vec<String>),
    /// A despawn animation of the list (its slot).
    Anim(usize),
}

/// What `view` shows, laid out (before scrolling): each thing's box. Returns the boxes and the
/// height of it all.
fn page_items(view: MenuView, s: &Settings, w: f32) -> (Vec<(SetupBox, PageItem)>, f32) {
    let cx = (w * 0.5).round();
    let mut out = Vec::new();
    let mut y = PAGE_TOP + 4.0;
    let rows = match view {
        MenuView::Page(ANIM_PAGE) => {
            for slot in anim_slots(s) {
                out.push((SetupBox { x: cx - 110.0, y, w: 220.0, h: 20.0 }, PageItem::Anim(slot)));
                y += 24.0;
            }
            return (out, y - PAGE_TOP);
        }
        MenuView::Page(cat) => page_rows(cat),
        MenuView::Anim(_) => {
            use Key::*;
            vec![Row::Keys(vec![AnimMotion]), Row::Keys(vec![AnimParticle]), Row::Keys(vec![AnimDuration]), Row::Keys(vec![AnimAmount])]
        }
        _ => Vec::new(),
    };
    for (r, row) in rows.into_iter().enumerate() {
        match row {
            Row::Keys(keys) => {
                if keys.len() == 1 {
                    out.push((SetupBox { x: cx - 155.0, y, w: 310.0, h: 20.0 }, PageItem::Key(keys[0], (r, 0))));
                } else {
                    for (c, k) in keys.iter().enumerate() {
                        out.push((SetupBox { x: cx - 155.0 + 160.0 * c as f32, y, w: 150.0, h: 20.0 }, PageItem::Key(*k, (r, c))));
                    }
                }
                y += 25.0;
            }
            Row::Header(t) => {
                out.push((SetupBox { x: cx - 155.0, y: y + 2.0, w: 310.0, h: 14.0 }, PageItem::Header(t)));
                y += 20.0;
            }
            Row::Note(t) => {
                let lines = ui_wrap(t, 300.0);
                let hh = 10.0 * lines.len() as f32 + 4.0;
                out.push((SetupBox { x: cx - 155.0, y, w: 310.0, h: hh }, PageItem::Note(lines)));
                y += hh + 4.0;
            }
        }
    }
    (out, y - PAGE_TOP)
}

/// How far a page can scroll.
fn page_max_scroll(view: MenuView, s: &Settings, w: f32, h: f32) -> f32 {
    let (_, content) = page_items(view, s, w);
    (content + 4.0 - (page_bottom(h) - PAGE_TOP)).max(0.0)
}

/// The things of a page where they are now (scrolled), with only those that show.
fn page_boxes(view: MenuView, s: &Settings, w: f32, h: f32, scroll: f32) -> Vec<(SetupBox, PageItem)> {
    page_items(view, s, w)
        .0
        .into_iter()
        .map(|(mut b, it)| {
            b.y -= scroll;
            (b, it)
        })
        .filter(|(b, _)| b.y + b.h > PAGE_TOP && b.y < page_bottom(h))
        .collect()
}

/// A footer button's job.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Foot {
    Reset,
    Gravity,
    /// More choices (not in Horizon Physics: shown greyed).
    Customize,
    Create,
    Replay,
    Done,
}

/// The buttons under a page.
fn page_footer(view: MenuView, w: f32, h: f32) -> Vec<(SetupBox, Foot)> {
    let cx = (w * 0.5).round();
    let y = h - 28.0;
    let three = |middle: Foot| vec![
        (SetupBox { x: cx - 130.0, y, w: 80.0, h: 20.0 }, Foot::Reset),
        (SetupBox { x: cx - 40.0, y, w: 80.0, h: 20.0 }, middle),
        (SetupBox { x: cx + 50.0, y, w: 80.0, h: 20.0 }, Foot::Done),
    ];
    let two = vec![(SetupBox { x: cx - 80.0, y, w: 75.0, h: 20.0 }, Foot::Reset), (SetupBox { x: cx + 5.0, y, w: 75.0, h: 20.0 }, Foot::Done)];
    match view {
        MenuView::Page(0) => three(Foot::Gravity),
        MenuView::Page(1) | MenuView::Page(2) => three(Foot::Customize),
        MenuView::Page(ANIM_PAGE) => {
            let mut f = vec![
                (SetupBox { x: cx - 155.0, y, w: 70.0, h: 20.0 }, Foot::Reset),
                (SetupBox { x: cx - 75.0, y, w: 150.0, h: 20.0 }, Foot::Create),
                (SetupBox { x: cx + 85.0, y, w: 70.0, h: 20.0 }, Foot::Done),
            ];
            f.push((SetupBox { x: 6.0, y, w: 60.0, h: 20.0 }, Foot::Replay));
            f
        }
        _ => two,
    }
}

fn foot_text(f: Foot, s: &Settings) -> String {
    match f {
        Foot::Reset => "RESET".into(),
        Foot::Gravity => format!("Gravity: {}", GRAVITY[s.gravity as usize % 5].0),
        Foot::Customize => "Customize".into(),
        Foot::Create => "Create Animation".into(),
        Foot::Replay => "REPLAY".into(),
        Foot::Done => "Done".into(),
    }
}

/// Whether a footer button does anything now.
fn foot_active(f: Foot, s: &Settings) -> bool {
    match f {
        Foot::Customize => false,
        Foot::Create => free_anim_slot(s).is_some(),
        _ => true,
    }
}

/// A slot for a new animation: the first free one after the mod's own, else any free one.
fn free_anim_slot(s: &Settings) -> Option<usize> {
    let free = |i: &usize| s.anim_used & (1 << i) == 0;
    (BUILTIN_ANIMS.len()..ANIM_SLOTS).find(free).or_else(|| (0..BUILTIN_ANIMS.len()).find(free))
}

/// A page's widget: dark, a light line under it, its text; a slider has its handle.
fn page_widget(out: &mut Vec<SetupUi>, b: &SetupBox, text: &str, lit: f32, handle: Option<f32>, active: bool) {
    let lit = if active { lit } else { 0.0 };
    let fill = mix_rgba([6, 6, 8, 205], [40, 40, 44, 215], lit);
    out.push(SetupUi::Rect { x: b.x, y: b.y, w: b.w, h: b.h, color: fill });
    // The line under it lights up and grows from the middle out.
    out.push(SetupUi::Rect { x: b.x, y: b.y + b.h - 1.0, w: b.w, h: 1.0, color: [200, 200, 200, 255] });
    if lit > 0.0 {
        let half = b.w * 0.5 * lit;
        out.push(SetupUi::Rect { x: b.x + b.w * 0.5 - half, y: b.y + b.h - 1.0, w: half * 2.0, h: 1.0, color: [255, 255, 255, 255] });
    }
    if let Some(t) = handle {
        let hx = b.x + t * (b.w - 8.0);
        out.push(SetupUi::Rect { x: hx, y: b.y, w: 8.0, h: b.h, color: mix_rgba([170, 170, 170, 255], [235, 235, 235, 255], lit) });
        out.push(SetupUi::Rect { x: hx + 1.0, y: b.y + 1.0, w: 6.0, h: b.h - 2.0, color: mix_rgba([92, 92, 92, 255], [120, 120, 124, 255], lit) });
    }
    let color = if active { UI_WHITE } else { GREY_TEXT };
    out.push(SetupUi::Text { text: text.to_string(), x: b.x + b.w * 0.5, y: b.y + (b.h - 8.0) * 0.5, color });
}

/// The pencil and the cross beside an animation of the list.
fn anim_buttons(b: &SetupBox) -> (SetupBox, SetupBox) {
    (SetupBox { x: b.x + b.w + 4.0, y: b.y, w: 20.0, h: 20.0 }, SetupBox { x: b.x + b.w + 28.0, y: b.y, w: 20.0, h: 20.0 })
}

/// Pixels of an icon, 2 GUI pixels each, on a 7 by 7 grid from (`x`, `y`).
fn icon(out: &mut Vec<SetupUi>, x: f32, y: f32, cells: &[(u8, u8)], color: [u8; 4]) {
    for &(cx, cy) in cells {
        out.push(SetupUi::Rect { x: x + 2.0 * f32::from(cx), y: y + 2.0 * f32::from(cy), w: 2.0, h: 2.0, color });
    }
}

const PENCIL: [(u8, u8); 11] = [(5, 0), (6, 1), (4, 1), (5, 2), (3, 2), (4, 3), (2, 3), (3, 4), (1, 4), (2, 5), (0, 6)];
const CROSS: [(u8, u8); 13] = [(0, 0), (1, 1), (2, 2), (3, 3), (4, 4), (5, 5), (6, 6), (6, 0), (5, 1), (4, 2), (2, 4), (1, 5), (0, 6)];

/// What the Physics Settings screen draws (Main and pages; the quick setup draws itself).
pub fn menu_ui(m: &mut Menu, w: f32, h: f32, cursor: (f32, f32)) -> Vec<SetupUi> {
    let mut out = Vec::new();
    let st = settings();
    let dt = m.frame_dt();
    m.age += dt;
    m.about_age += dt;
    match m.view {
        MenuView::Main | MenuView::Setup => {
            let hovered = (0..CATEGORIES.len()).find(|&i| tab_box(i, m.scroll, false).has(cursor) || tab_box(i, m.scroll, true).has(cursor) && m.shown == i);
            if let Some(i) = hovered
                && i != m.shown
            {
                m.shown = i;
                m.picture = SetupScene::new(CATEGORIES[i].scene, 11 + i as u64);
                m.picture.zoom = MENU_ZOOM;
                m.about_age = 0.0;
            }
            let cat = &CATEGORIES[m.shown];
            let (top, low) = match cat.scene {
                Scene::Weather => ([62, 72, 84, 255], [82, 96, 106, 255]),
                Scene::Snow => ([168, 186, 222, 255], [226, 226, 238, 255]),
                Scene::Ocean => ([150, 180, 220, 255], [222, 230, 238, 255]),
                _ => ([110, 158, 220, 255], [205, 218, 232, 255]),
            };
            out.push(SetupUi::Gradient { x: 0.0, y: 0.0, w, h, top, bottom: low });
            out.push(SetupUi::Scene { index: 0, x: 0.0, y: 0.0, w, h });
            for (i, c) in CATEGORIES.iter().enumerate() {
                let lit = m.lit(1000 + i as u32, hovered == Some(i), dt);
                let mut b = tab_box(i, m.scroll, false);
                // Out it slides as it lights up; on opening, the tabs come in one after another.
                b.w += 14.0 * lit;
                b.x -= (1.0 - ease((m.age - i as f32 * 0.025) / 0.3)) * (TAB_W + 20.0);
                if b.y + b.h < 0.0 || b.y > h {
                    continue;
                }
                let bar = match c.kind {
                    2 => BAR_YELLOW,
                    1 => BAR_RED,
                    _ => mix_rgba(BAR_LIGHT, BAR_GREEN, lit),
                };
                menu_tab(&mut out, &b, c.name, bar, false, lit);
            }
            // The list goes on below: a small arrow says so.
            if m.scroll < max_scroll(h) {
                out.push(SetupUi::label("v", 60.0, h - 12.0, UI_WHITE, 1.4, false));
            }
            let (mut quick, mut back) = menu_buttons(w);
            let in_from_right = (1.0 - ease(m.age / 0.3)) * 100.0;
            let (lq, lb) = (m.lit(2000, quick.has(cursor), dt), m.lit(2001, back.has(cursor), dt));
            for (b, l) in [(&mut quick, lq), (&mut back, lb)] {
                b.x += in_from_right - 10.0 * l;
                b.w += 10.0 * l;
            }
            menu_tab(&mut out, &quick, "QUICK SETUP", mix_rgba(BAR_LIGHT, BAR_GREEN, lq), true, lq);
            menu_tab(&mut out, &back, "BACK", mix_rgba(BAR_LIGHT, BAR_GREEN, lb), true, lb);
            // What the category is: a box at the bottom with its colour under it.
            let (x0, x1) = ((w * 0.236).round(), (w * 0.973).round());
            let lines = ui_wrap(cat.about, x1 - x0 - 20.0);
            // A new category's box rises a little into place and its text fades in.
            let fade = ease(m.about_age / 0.22);
            let bottom = h - 21.0 + (1.0 - fade) * 6.0;
            let top_y = bottom - 16.0 - 10.0 * lines.len() as f32;
            out.push(SetupUi::Rect { x: x0, y: top_y, w: x1 - x0, h: bottom - top_y, color: [24, 26, 32, 218] });
            let line_color = match cat.kind {
                2 => BAR_YELLOW,
                1 => BAR_RED,
                _ => BAR_GREEN,
            };
            out.push(SetupUi::Rect { x: x0, y: bottom, w: x1 - x0, h: 3.0, color: line_color });
            for (k, line) in lines.iter().enumerate() {
                let mut color = UI_WHITE;
                color[3] = (255.0 * fade).round().max(8.0) as u8;
                out.push(SetupUi::label(line.clone(), x0 + 10.0, top_y + 9.0 + 10.0 * k as f32, color, 1.0, true));
            }
        }
        view @ (MenuView::Page(_) | MenuView::Anim(_)) => {
            let max = page_max_scroll(view, &st, w, h);
            m.page_scroll = m.page_scroll.clamp(0.0, max);
            out.push(SetupUi::Tiles { x: 0.0, y: 0.0, w, h, sprite: SP_BRICKS, size: 32.0, color: [74, 74, 74, 255] });
            out.push(SetupUi::Rect { x: 0.0, y: 0.0, w, h: PAGE_TOP, color: [0, 0, 0, 70] });
            out.push(SetupUi::Rect { x: 0.0, y: page_bottom(h), w, h: h - page_bottom(h), color: [0, 0, 0, 70] });
            let title = match view {
                MenuView::Anim(slot) => format!("Edit Animation: {}", anim_name(&st, slot)),
                MenuView::Page(cat) => CATEGORIES[cat].title.to_string(),
                _ => String::new(),
            };
            out.push(SetupUi::Text { text: title, x: (w * 0.5).round(), y: 22.0, color: UI_WHITE });
            let inside = cursor.1 >= PAGE_TOP && cursor.1 < page_bottom(h);
            out.push(SetupUi::Clip { x: 0.0, y: PAGE_TOP, w, h: page_bottom(h) - PAGE_TOP });
            // On opening, the rows rise into place one after another.
            let rise = |m: &Menu, k: usize| (1.0 - ease((m.age - k as f32 * 0.02) / 0.25)) * 12.0;
            for (k, (mut b, item)) in page_boxes(view, &st, w, h, m.page_scroll).into_iter().enumerate() {
                let hovered = inside && b.has(cursor);
                b.y += rise(m, k);
                match item {
                    PageItem::Key(key, _) => {
                        let handle = slider(key).map(|_| key.fraction(&st));
                        let lit = m.lit(3000 + k as u32, hovered, dt);
                        page_widget(&mut out, &b, &key.label(&st), lit, handle, true);
                    }
                    PageItem::Header(t) => {
                        out.push(SetupUi::Text { text: t.to_string(), x: b.x + b.w * 0.5, y: b.y + 3.0, color: UI_WHITE });
                    }
                    PageItem::Note(lines) => {
                        for (i, line) in lines.iter().enumerate() {
                            out.push(SetupUi::Text { text: line.clone(), x: b.x + b.w * 0.5, y: b.y + 2.0 + 10.0 * i as f32, color: NOTE_YELLOW });
                        }
                    }
                    PageItem::Anim(slot) => {
                        let chosen = st.anim as usize == slot;
                        let lit = m.lit(5000 + slot as u32 * 3, hovered, dt);
                        let pick = m.lit(5001 + slot as u32 * 3, chosen, dt);
                        // The chosen one's white frame fades in round it.
                        if pick > 0.0 {
                            out.push(SetupUi::Rect { x: b.x - 1.0, y: b.y - 1.0, w: b.w + 2.0, h: b.h + 2.0, color: mix_rgba([255, 255, 255, 0], UI_WHITE, pick) });
                        }
                        let fill = mix_rgba(mix_rgba([0, 0, 0, 120], [40, 40, 44, 200], lit), [0, 0, 0, 255], pick);
                        out.push(SetupUi::Rect { x: b.x, y: b.y, w: b.w, h: b.h, color: fill });
                        out.push(SetupUi::Text { text: anim_name(&st, slot), x: b.x + b.w * 0.5, y: b.y + 6.0, color: UI_WHITE });
                        let (edit, remove) = anim_buttons(&b);
                        for (n, (bb, cells, color)) in [(edit, &PENCIL[..], [235, 235, 235, 255]), (remove, &CROSS[..], [230, 60, 50, 255])].into_iter().enumerate() {
                            let l = m.lit(6000 + slot as u32 * 2 + n as u32, inside && bb.has(cursor), dt);
                            out.push(SetupUi::Rect { x: bb.x, y: bb.y, w: bb.w, h: bb.h, color: mix_rgba([20, 20, 24, 200], [70, 70, 76, 220], l) });
                            out.push(SetupUi::Rect { x: bb.x, y: bb.y + bb.h - 1.0, w: bb.w, h: 1.0, color: mix_rgba([200, 200, 200, 255], UI_WHITE, l) });
                            icon(&mut out, bb.x + 3.0, bb.y + 3.0 - l, cells, color);
                        }
                    }
                }
            }
            out.push(SetupUi::Unclip);
            // The page goes on below: a scroll bar on its right.
            if max > 0.0 {
                let cx = (w * 0.5).round();
                let (x, top, len) = (cx + 162.0, PAGE_TOP, page_bottom(h) - PAGE_TOP);
                let thumb = (len * len / (len + max)).max(16.0);
                let ty = top + (len - thumb) * (m.page_scroll / max);
                out.push(SetupUi::Rect { x, y: top, w: 6.0, h: len, color: [0, 0, 0, 255] });
                out.push(SetupUi::Rect { x, y: ty, w: 6.0, h: thumb, color: [128, 128, 128, 255] });
                out.push(SetupUi::Rect { x, y: ty, w: 5.0, h: thumb - 1.0, color: [192, 192, 192, 255] });
            }
            for (j, (b, f)) in page_footer(view, w, h).into_iter().enumerate() {
                let lit = m.lit(4000 + j as u32, b.has(cursor), dt);
                page_widget(&mut out, &b, &foot_text(f, &st), lit, None, foot_active(f, &st));
            }
        }
    }
    out
}

/// What a press (or release) on the Physics Settings screen did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuOut {
    None,
    Click,
    /// Back to the Options screen.
    Close,
}

/// A press (`pressed`) or release at (`x`, `y`) on the screen, `w` by `h`.
pub fn menu_mouse(w: f32, h: f32, x: f32, y: f32, pressed: bool) -> MenuOut {
    if menu_view() == MenuView::Setup {
        if !pressed {
            return MenuOut::None;
        }
        let hit = setup_click(w, h, x, y);
        if settings().setup {
            with_menu(|m| m.view = MenuView::Main);
        }
        return if hit { MenuOut::Click } else { MenuOut::None };
    }
    with_menu(|m| {
        if !pressed {
            m.drag = None;
            return MenuOut::None;
        }
        match m.view {
            MenuView::Main | MenuView::Setup => {
                let (quick, back) = menu_buttons(w);
                if back.has((x, y)) {
                    m.open = false;
                    return MenuOut::Close;
                }
                if quick.has((x, y)) {
                    m.view = MenuView::Setup;
                    update_settings(|s| s.setup = false);
                    with_setup(|s| s.page = 0);
                    return MenuOut::Click;
                }
                for i in 0..CATEGORIES.len() {
                    if tab_box(i, m.scroll, m.shown == i).has((x, y)) {
                        if !page_rows(i).is_empty() {
                            m.go(MenuView::Page(i));
                            return MenuOut::Click;
                        }
                        return MenuOut::None;
                    }
                }
                MenuOut::None
            }
            view @ (MenuView::Page(_) | MenuView::Anim(_)) => page_mouse(m, view, w, h, x, y),
        }
    })
}

/// A press on a settings page or the animation editor.
fn page_mouse(m: &mut Menu, view: MenuView, w: f32, h: f32, x: f32, y: f32) -> MenuOut {
    let st = settings();
    for (b, f) in page_footer(view, w, h) {
        if !b.has((x, y)) {
            continue;
        }
        if !foot_active(f, &st) {
            return MenuOut::None;
        }
        match f {
            Foot::Reset => update_settings(|s| match view {
                MenuView::Page(ANIM_PAGE) => {
                    let d = Settings::default();
                    s.anims = d.anims;
                    s.anim_used = d.anim_used;
                    s.anim = d.anim;
                }
                MenuView::Anim(slot) => s.anims[slot] = anim_default(slot),
                MenuView::Page(cat) => {
                    for row in category_rows(cat) {
                        for k in row {
                            k.reset(s);
                        }
                    }
                }
                _ => {}
            }),
            Foot::Gravity => update_settings(|s| s.gravity = (s.gravity + 1) % 5),
            Foot::Customize => {}
            Foot::Create => {
                if let Some(slot) = free_anim_slot(&st) {
                    update_settings(|s| {
                        s.anim_used |= 1 << slot;
                        s.anims[slot] = anim_default(slot);
                    });
                    set_anim_edit(slot);
                    m.go(MenuView::Anim(slot));
                }
            }
            Foot::Replay => replay_anim(),
            Foot::Done => match view {
                MenuView::Anim(_) => m.go(MenuView::Page(ANIM_PAGE)),
                _ => m.go(MenuView::Main),
            },
        }
        return MenuOut::Click;
    }
    if !(PAGE_TOP..page_bottom(h)).contains(&y) {
        return MenuOut::None;
    }
    for (b, item) in page_boxes(view, &st, w, h, m.page_scroll) {
        match item {
            PageItem::Key(k, at) if b.has((x, y)) => {
                if slider(k).is_some() {
                    let t = (x - b.x - 4.0) / (b.w - 8.0);
                    update_settings(|s| k.slide(s, t));
                    m.drag = Some(at);
                    return MenuOut::None;
                }
                update_settings(|s| k.press(s));
                return MenuOut::Click;
            }
            PageItem::Anim(slot) => {
                let (edit, remove) = anim_buttons(&b);
                if edit.has((x, y)) {
                    set_anim_edit(slot);
                    m.go(MenuView::Anim(slot));
                    return MenuOut::Click;
                }
                if remove.has((x, y)) {
                    // The last one stays: something has to be used.
                    if anim_slots(&st).len() > 1 {
                        update_settings(|s| {
                            s.anim_used &= !(1 << slot);
                            if s.anim as usize == slot {
                                s.anim = anim_slots(s).first().copied().unwrap_or(0) as u8;
                            }
                        });
                    }
                    return MenuOut::Click;
                }
                if b.has((x, y)) {
                    update_settings(|s| s.anim = slot as u8);
                    return MenuOut::Click;
                }
            }
            _ => {}
        }
    }
    MenuOut::None
}

/// The pointer moved with the button down: the slider being dragged follows it.
pub fn menu_drag(w: f32, x: f32) {
    with_menu(|m| {
        let Some(at) = m.drag else {
            return;
        };
        if !matches!(m.view, MenuView::Page(_) | MenuView::Anim(_)) {
            return;
        }
        let st = settings();
        for (b, item) in page_items(m.view, &st, w).0 {
            if let PageItem::Key(k, here) = item
                && here == at
            {
                let t = (x - b.x - 4.0) / (b.w - 8.0);
                update_settings(|s| k.slide(s, t));
                return;
            }
        }
    });
}

/// The wheel: the category list scrolls, or the page.
pub fn menu_scroll(h: f32, amount: f32) {
    with_menu(|m| match m.view {
        MenuView::Main => m.scroll = (m.scroll - amount * 20.0).clamp(0.0, max_scroll(h)),
        MenuView::Page(_) | MenuView::Anim(_) => m.page_scroll = (m.page_scroll - amount * 20.0).max(0.0),
        MenuView::Setup => {}
    });
}

/// Escape: a page or the quick setup goes back to the menu (the editor to its list), the menu
/// to Options. Returns true when the screen closed.
pub fn menu_escape() -> bool {
    with_menu(|m| match m.view {
        MenuView::Main => {
            m.open = false;
            true
        }
        MenuView::Anim(_) => {
            m.go(MenuView::Page(ANIM_PAGE));
            false
        }
        _ => {
            m.go(MenuView::Main);
            false
        }
    })
}
