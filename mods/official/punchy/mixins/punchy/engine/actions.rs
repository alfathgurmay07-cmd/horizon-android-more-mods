//! Punchy 3.1b action scheduling, reconstructed from the local 26.3 JAR.
//!
//! Exact isolated rules: intent arbitration, attack clock/combo, bow power gate,
//! spear stage boundaries, and the ordinary shield/bow/charge action selection.
//! `ActionMachine` is a core integration adapter, not a claim that all 18,886
//! lines of Punchy's Java state machines have been ported. It intentionally does
//! not implement custom-entry predicates, consume/mining v2 phase handoffs,
//! detached equip-arm snapshots, loyalty return tracking, or compatibility mods.
//! The caller resolves names and item tags and supplies actual gameplay events;
//! the adapter never infers a successful attack/use from a held mouse button.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Hand {
    #[default]
    Main,
    Off,
}

impl Hand {
    pub const fn index(self) -> usize {
        match self {
            Self::Main => 0,
            Self::Off => 1,
        }
    }
}

/// Source priorities and equal-priority replacement are literal Java behavior.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum Source {
    ActionVariation = 10,
    Equip = 20,
    Mining = 60,
    Attack = 70,
    SustainedUse = 80,
    TwoHandedUse = 90,
    SafetyStop = 100,
}

#[derive(Clone, Debug)]
pub struct Coordinator<T> {
    pending: Option<(Source, T)>,
}

impl<T> Default for Coordinator<T> {
    fn default() -> Self {
        Self { pending: None }
    }
}

impl<T> Coordinator<T> {
    pub fn request(&mut self, source: Source, operation: T) {
        if self.pending.as_ref().is_none_or(|(old, _)| source >= *old) {
            self.pending = Some((source, operation));
        }
    }
    pub fn discard_for_equip_transition(&mut self) {
        self.pending = None;
    }
    pub fn flush(&mut self) -> Option<(Source, T)> {
        self.pending.take()
    }
}

#[derive(Clone, Debug)]
pub struct AttackClock {
    next_visual_tick: f64,
    last_trigger_tick: i64,
    last_combo_tick: i64,
    combo_index: usize,
    last_combo_size: usize,
    throttled_pending: bool,
}

impl Default for AttackClock {
    fn default() -> Self {
        Self {
            next_visual_tick: -1.0,
            last_trigger_tick: -1,
            last_combo_tick: -1,
            combo_index: 0,
            last_combo_size: 0,
            throttled_pending: false,
        }
    }
}

impl AttackClock {
    pub fn cooling_down(&self, now: i64, input_driven_combo: bool) -> bool {
        !input_driven_combo && self.next_visual_tick >= 0.0 && (now as f64) < self.next_visual_tick
    }
    pub fn defer(&mut self) {
        self.throttled_pending = true;
    }
    pub fn visual_speed(&self, input_driven_combo: bool) -> f32 {
        if self.throttled_pending && !input_driven_combo {
            1.12
        } else {
            1.0
        }
    }
    /// Call only when the ATTACK request won arbitration and playback started.
    pub fn record_playback(&mut self, now: i64, input_driven_combo: bool) {
        self.last_trigger_tick = now;
        if input_driven_combo {
            self.next_visual_tick = -1.0;
        } else {
            let interval = 5.714285714285714_f64;
            if self.next_visual_tick < 0.0 || now as f64 >= self.next_visual_tick + interval {
                self.next_visual_tick = now as f64 + interval;
            } else {
                self.next_visual_tick += interval;
            }
        }
        self.throttled_pending = false;
    }
    pub fn skip_fast_blend(&self, now: i64) -> bool {
        self.last_trigger_tick > 0 && now - self.last_trigger_tick <= 4
    }
    /// Java deliberately leaves the previous multi-entry combo state untouched
    /// when a zero/single-entry list is selected.
    pub fn next_combo_index(&mut self, now: i64, total: usize) -> usize {
        if total <= 1 {
            return 0;
        }
        if self.last_combo_tick < 0
            || now - self.last_combo_tick > 30
            || total != self.last_combo_size
        {
            self.combo_index = 0;
        }
        if self.combo_index >= total {
            self.combo_index = 0;
        }
        let current = self.combo_index;
        self.combo_index = (self.combo_index + 1) % total;
        self.last_combo_tick = now;
        self.last_combo_size = total;
        current
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ItemKind {
    #[default]
    Empty,
    Other,
    Sword,
    Pickaxe,
    Hoe,
    Axe,
    Shovel,
    Mace,
    Bow,
    Crossbow,
    Shield,
    Trident,
    Spear,
    Eat,
    Drink,
}

impl ItemKind {
    /// Convenience for vanilla IDs. The integration must override this using
    /// actual item tags/components for modded tools and consumables.
    pub fn from_id(id: &str) -> Self {
        let id = id.strip_prefix("minecraft:").unwrap_or(id);
        if id.is_empty() || id == "air" {
            return Self::Empty;
        }
        if id.ends_with("_pickaxe") {
            return Self::Pickaxe;
        }
        if id.ends_with("_shovel") {
            return Self::Shovel;
        }
        if id.ends_with("_sword") {
            return Self::Sword;
        }
        if id.ends_with("_hoe") {
            return Self::Hoe;
        }
        if id.ends_with("_axe") {
            return Self::Axe;
        }
        if id == "spear" || id.ends_with("_spear") {
            return Self::Spear;
        }
        match id {
            "mace" => Self::Mace,
            "bow" => Self::Bow,
            "crossbow" => Self::Crossbow,
            "shield" => Self::Shield,
            "trident" => Self::Trident,
            _ => Self::Other,
        }
    }
    fn entity_only_attack(self) -> bool {
        matches!(self, Self::Pickaxe | Self::Hoe | Self::Shovel)
    }
}

#[derive(Clone, Debug, Default)]
pub struct HandState {
    pub kind: ItemKind,
    pub blacklisted: bool,
    pub crossbow_charged: bool,
}

#[derive(Clone, Debug)]
pub struct ClipMeta {
    pub name: String,
    pub length: f32,
    pub hold_on_last_frame: bool,
    pub dual_handed: bool,
}

/// Resolve preferred-pack/custom names/handed suffixes outside the state machine.
pub trait ClipCatalog {
    fn resolve(&self, name: &str, hand: Hand) -> Option<ClipMeta>;
}

#[derive(Clone, Debug, Default)]
pub struct AttackOptions {
    pub combo: Vec<String>,
    pub critical_clip: Option<String>,
    pub critical_hit: bool,
    pub trigger_on_input: bool,
    pub mining_animation_disabled: bool,
    pub custom_entry_present: bool,
    pub skip_blend_in: bool,
}

impl AttackOptions {
    fn input_driven(&self) -> bool {
        self.trigger_on_input && !self.combo.is_empty()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct UseState {
    pub hand: Hand,
    pub kind: ItemKind,
    pub elapsed_ticks: i32,
    pub charge_duration_ticks: i32,
    pub spear_stage: u8,
    pub spear_maxed: bool,
    pub riptide: bool,
    pub loyalty: bool,
}

impl Default for UseState {
    fn default() -> Self {
        Self {
            hand: Hand::Main,
            kind: ItemKind::Other,
            elapsed_ticks: 0,
            charge_duration_ticks: 0,
            spear_stage: 1,
            spear_maxed: false,
            riptide: false,
            loyalty: false,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct EquipEvent {
    pub hand: Hand,
    pub entering: bool,
}

#[derive(Clone, Debug)]
pub struct Input {
    pub player_present: bool,
    /// An independently resolved phased action participates in the same arbitration.
    pub external_priority: Option<u8>,
    pub hands: [HandState; 2],
    pub attack: Option<Hand>,
    pub attack_options: AttackOptions,
    pub entity_target: bool,
    pub block_target: bool,
    pub mining: bool,
    pub mining_trigger: bool,
    pub use_trigger: Option<Hand>,
    pub use_down: bool,
    pub using: Option<UseState>,
    pub inspect: bool,
    pub equip: Option<EquipEvent>,
    pub equip_in_progress: bool,
    /// Adapter has successfully cancelled the conflicting equip transition.
    pub may_abort_equip: bool,
    pub equip_transition_recent: bool,
    pub hotbar_changed: bool,
    pub has_bow_projectile_or_creative: bool,
    /// Whether tuning has an explicit use_bow_shot entry. The Java release
    /// path requires an entry even if a same-named default clip exists.
    pub bow_shot_configured: bool,
    pub playback_active: bool,
    pub playback_clip: Option<String>,
    pub playback_hand: Option<Hand>,
    /// Global playback rate in the original PoseHandler; default 5.5.
    pub animation_speed: f32,
}

impl Default for Input {
    fn default() -> Self {
        Self {
            player_present: true,
            external_priority: None,
            hands: Default::default(),
            attack: None,
            attack_options: Default::default(),
            entity_target: false,
            block_target: false,
            mining: false,
            mining_trigger: false,
            use_trigger: None,
            use_down: false,
            using: None,
            inspect: false,
            equip: None,
            equip_in_progress: false,
            may_abort_equip: true,
            equip_transition_recent: false,
            hotbar_changed: false,
            has_bow_projectile_or_creative: false,
            bow_shot_configured: false,
            playback_active: false,
            playback_clip: None,
            playback_hand: None,
            animation_speed: 5.5,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionKind {
    Mining,
    Interact,
    Attack,
    Inspect,
    Equip,
    Bow,
    Crossbow,
    Shield,
    Trident,
    Spear,
    Eat,
    Drink,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StopMode {
    FinishCurrent,
    BlendOut(f32),
}

#[derive(Clone, Debug)]
pub struct Play {
    pub kind: ActionKind,
    pub clip: ClipMeta,
    pub hand: Hand,
    pub looping: bool,
    pub hold_on_last_frame: bool,
    /// Multiplied by the global animation speed by the animation engine.
    pub speed_multiplier: f32,
    pub skip_blend_in: bool,
    pub blend_seconds: Option<f32>,
    pub start_time: f32,
}

#[derive(Clone, Debug)]
pub enum Operation {
    Play(Play),
    Stop { hand: Hand, mode: StopMode },
}

#[derive(Clone, Debug)]
pub struct Intent {
    pub source: Source,
    pub operation: Operation,
}

/// BowItem.getPowerForTime's 0.1 minimum gate, as used by BowStateMachine.
pub fn bow_shot_allowed(draw_ticks: i32, projectile_or_creative: bool) -> bool {
    let f = draw_ticks as f32 / 20.0;
    ((f * f + f * 2.0) / 3.0).min(1.0) >= 0.1 && projectile_or_creative
}

/// PunchyAnimationManager.clipSpeedMultiplier, for normalized clip names.
pub fn clip_speed_multiplier(name: &str, animation_speed: f32) -> f32 {
    let name = name.strip_prefix("animation.").unwrap_or(name);
    match name {
        "use_bow"
        | "use_shield"
        | "use_crossbow"
        | "use_crossbow_shot"
        | "use_trident"
        | "use_trident_throw"
        | "use_trident_throw_loyalty"
        | "use_trident_throw_riptide"
        | "attack_mace"
        | "dry_out"
        | "hand_in"
        | "hand_out" => 1.0 / animation_speed.max(0.01),
        "interact" => 5.4 / animation_speed.max(0.01),
        _ => 1.0,
    }
}

/// Return (stage, maximum exceeded). The elapsed input is Java's useDuration -
/// (remainingTicks + 1), and the maximum comparison is strictly greater-than.
pub fn spear_charge_stage(
    elapsed: i32,
    delay: i32,
    dismount: i32,
    knockback: i32,
    damage: i32,
) -> (u8, bool) {
    let raw3 = damage.saturating_sub(delay);
    let has_limit = raw3 > 0;
    let stage3 = if raw3 <= 0 { i32::MAX } else { raw3 };
    let raw2 = knockback.saturating_sub(delay);
    let stage2 = if raw2 <= 0 { stage3 } else { raw2 };
    let raw1 = dismount.saturating_sub(delay);
    let stage1 = if raw1 <= 0 { stage2 } else { raw1 };
    let stage = if elapsed < stage1 {
        1
    } else if elapsed < stage2 {
        2
    } else {
        3
    };
    (stage, has_limit && elapsed > stage3)
}

#[derive(Default)]
pub struct ActionMachine {
    tick: i64,
    attack_clock: AttackClock,
    pending_attack: Option<Hand>,
    pending_inspect: bool,
    last_use: Option<UseState>,
    last_use_down: bool,
    last_shield_hand: Option<Hand>,
    last_crossbow_charged: [bool; 2],
    predicted_crossbow_shot: [bool; 2],
    crossbow_suppressed: bool,
    last_mining_trigger: i64,
    last_mining_clip: Option<String>,
    tool_fallback_expire: Option<i64>,
}

impl ActionMachine {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn tick<C: ClipCatalog>(&mut self, input: &Input, clips: &C) -> Option<Intent> {
        if !input.player_present {
            self.reset();
            return input.playback_active.then(|| Intent {
                source: Source::SafetyStop,
                operation: Operation::Stop {
                    hand: input.playback_hand.unwrap_or_default(),
                    mode: StopMode::BlendOut(0.0),
                },
            });
        }
        self.tick += 1;
        let mut pending = Coordinator::default();
        if let Some(hand) = input.attack {
            self.pending_attack = Some(hand);
        }
        self.pending_inspect |= input.inspect;

        // Call order follows PunchyClient: mining, consume/use, weapons,
        // attack, inspect, equip. Later equal-priority requests win.
        if !input.hands[0].blacklisted
            && (input.mining_trigger || input.mining && !input.playback_active)
        {
            if let Some(clip) = mining_clip(clips, input.hands[0].kind, Hand::Main, true) {
                let mut play = play(ActionKind::Mining, clip, Hand::Main);
                play.skip_blend_in =
                    self.last_mining_trigger > 0 && self.tick - self.last_mining_trigger <= 4;
                offer(&mut pending, Source::Mining, play);
            }
        } else if !input.mining
            && self.last_mining_clip.as_deref() == input.playback_clip.as_deref()
            && self.last_mining_clip.is_some()
            && self.tick
                >= self
                    .tool_fallback_expire
                    .unwrap_or(self.last_mining_trigger + 2)
        {
            pending.request(
                Source::Mining,
                Operation::Stop {
                    hand: Hand::Main,
                    mode: StopMode::FinishCurrent,
                },
            );
            self.last_mining_clip = None;
            self.tool_fallback_expire = None;
        }

        let using = input.using.filter(|u| {
            !input.hands[u.hand.index()].blacklisted
                && (u.kind == ItemKind::Shield || input.use_down)
        });
        let can_act = !input.equip_in_progress || input.may_abort_equip;
        if can_act {
            if let Some(u) = using {
                self.use_action(input, u, clips, &mut pending);
            } else if let Some(old) = self.last_use {
                self.release_use(input, old, clips, &mut pending);
            }
            if let Some(hand) = input.use_trigger
                && !input.hands[hand.index()].blacklisted
                && using.is_none()
                && let Some(clip) = first(
                    clips,
                    &[
                        "use_item",
                        "useitem",
                        "interaction",
                        "interact",
                        "mining_use_foward",
                        "mining_use_digging",
                    ],
                    hand,
                )
            {
                offer(
                    &mut pending,
                    Source::SustainedUse,
                    play(ActionKind::Interact, clip, hand),
                );
            }
            // Shield visuals persist when right click remains down and the
            // player stops reporting active use for one or more ticks.
            if using.is_none()
                && input.use_down
                && let Some(hand) = self.last_shield_hand
                && input.hands[hand.index()].kind == ItemKind::Shield
                && !input.hands[hand.index()].blacklisted
                && let Some(clip) = clips.resolve("use_shield", hand)
                && needs_play(input, &clip, hand)
            {
                let mut p = play(ActionKind::Shield, clip, hand);
                p.looping = !p.hold_on_last_frame;
                p.skip_blend_in = true;
                offer(&mut pending, Source::SustainedUse, p);
            }
            self.crossbow_shots(input, clips, &mut pending);
            self.attack(input, clips, &mut pending);
        }

        if self.pending_inspect && !input.equip_in_progress {
            self.pending_inspect = false;
            for hand in [Hand::Main, Hand::Off] {
                if !input.hands[hand.index()].blacklisted
                    && let Some(clip) = clips.resolve("inspect", hand)
                {
                    offer(
                        &mut pending,
                        Source::SustainedUse,
                        play(ActionKind::Inspect, clip, hand),
                    );
                    break;
                }
            }
        }
        if let Some(equip) = input.equip
            && let Some(clip) = clips.resolve(
                if equip.entering {
                    "hand_in"
                } else {
                    "hand_out"
                },
                equip.hand,
            )
        {
            offer(
                &mut pending,
                Source::Equip,
                play(ActionKind::Equip, clip, equip.hand),
            );
        }
        if can_act {
            self.last_use = using.or_else(|| {
                self.last_shield_hand
                    .filter(|hand| {
                        input.use_down
                            && !input.hands[hand.index()].blacklisted
                            && input.hands[hand.index()].kind == ItemKind::Shield
                    })
                    .map(|hand| UseState {
                        hand,
                        kind: ItemKind::Shield,
                        ..Default::default()
                    })
            });
        }
        self.last_use_down = input.use_down;
        if !input.use_down {
            self.last_shield_hand = None;
        }
        for hand in [Hand::Main, Hand::Off] {
            self.last_crossbow_charged[hand.index()] = input.hands[hand.index()].crossbow_charged;
        }
        let (source, mut operation) = pending.flush()?;
        if input.external_priority.is_some_and(|p| p >= source as u8) {
            return None;
        }
        if let Operation::Play(p) = &mut operation {
            // Bow draw, crossbow charge/shot and spear set their own real-time
            // rates at the call site. Other actions use the clip-name policy.
            if !matches!(
                p.kind,
                ActionKind::Bow | ActionKind::Crossbow | ActionKind::Spear
            ) {
                p.speed_multiplier *= if matches!(p.kind, ActionKind::Eat | ActionKind::Drink)
                    && p.clip.name != "use_eat"
                    && p.clip.name != "use_drink"
                {
                    1.0 / input.animation_speed.max(0.01)
                } else {
                    clip_speed_multiplier(&p.clip.name, input.animation_speed)
                };
            }
            match p.kind {
                ActionKind::Attack => self
                    .attack_clock
                    .record_playback(self.tick, input.attack_options.input_driven()),
                ActionKind::Mining => {
                    self.last_mining_trigger = self.tick;
                    self.last_mining_clip = Some(p.clip.name.clone());
                    self.tool_fallback_expire = if input.mining || input.mining_trigger {
                        None
                    } else {
                        Some(self.tick + 4)
                    };
                }
                _ => {}
            }
        }
        Some(Intent { source, operation })
    }

    fn attack<C: ClipCatalog>(
        &mut self,
        input: &Input,
        clips: &C,
        pending: &mut Coordinator<Operation>,
    ) {
        if input
            .external_priority
            .is_some_and(|p| p >= Source::Attack as u8)
        {
            return;
        }
        let Some(hand) = self.pending_attack else {
            return;
        };
        let options = &input.attack_options;
        if self
            .attack_clock
            .cooling_down(self.tick, options.input_driven())
        {
            self.attack_clock.defer();
            return;
        }
        self.pending_attack = None;
        if input.hands[hand.index()].blacklisted {
            return;
        }
        let kind = input.hands[hand.index()].kind;
        if kind.entity_only_attack()
            && !input.entity_target
            && !options.custom_entry_present
            && options.combo.is_empty()
            && !options.mining_animation_disabled
        {
            if !input.mining
                && !input.block_target
                && let Some(clip) = mining_clip(clips, kind, hand, false)
            {
                let mut p = play(ActionKind::Mining, clip, hand);
                p.skip_blend_in = self.attack_clock.skip_fast_blend(self.tick);
                offer(pending, Source::Mining, p);
            }
            return;
        }
        let mut clip = if options.critical_hit {
            options
                .critical_clip
                .as_ref()
                .and_then(|name| clips.resolve(name, hand))
        } else {
            None
        };
        if clip.is_none() && !options.combo.is_empty() {
            let index = self
                .attack_clock
                .next_combo_index(self.tick, options.combo.len());
            clip = clips.resolve(&options.combo[index], hand);
        }
        if clip.is_none() {
            clip = clips.resolve("attack", hand);
        }
        if let Some(base) = &clip
            && (base.name == "attack" || base.name == "animation.attack")
        {
            let variant = match kind {
                ItemKind::Mace => Some("attack_mace"),
                ItemKind::Trident => Some("attack_trident"),
                _ => None,
            };
            if let Some(name) = variant {
                clip = clips.resolve(name, hand).or(clip);
            }
        }
        if let Some(clip) = clip {
            let mut p = play(ActionKind::Attack, clip, hand);
            p.speed_multiplier = self.attack_clock.visual_speed(options.input_driven());
            p.skip_blend_in = options.skip_blend_in || input.equip_transition_recent;
            offer(pending, Source::Attack, p);
        }
    }

    fn use_action<C: ClipCatalog>(
        &mut self,
        input: &Input,
        u: UseState,
        clips: &C,
        pending: &mut Coordinator<Operation>,
    ) {
        let (name, kind, source) = match u.kind {
            ItemKind::Bow => ("use_bow", ActionKind::Bow, Source::TwoHandedUse),
            ItemKind::Crossbow => {
                if !input.use_down {
                    self.crossbow_suppressed = false;
                }
                if u.charge_duration_ticks > 0 && u.elapsed_ticks >= u.charge_duration_ticks {
                    self.crossbow_suppressed = true;
                    if input.playback_active {
                        pending.request(
                            Source::TwoHandedUse,
                            Operation::Stop {
                                hand: u.hand,
                                mode: StopMode::FinishCurrent,
                            },
                        );
                    }
                }
                if input.hands[u.hand.index()].crossbow_charged || self.crossbow_suppressed {
                    return;
                }
                ("use_crossbow", ActionKind::Crossbow, Source::TwoHandedUse)
            }
            ItemKind::Shield => {
                self.last_shield_hand = Some(u.hand);
                ("use_shield", ActionKind::Shield, Source::SustainedUse)
            }
            ItemKind::Trident => ("use_trident", ActionKind::Trident, Source::TwoHandedUse),
            ItemKind::Spear => {
                if u.spear_maxed || u.spear_stage == 0 {
                    return;
                }
                let name = match u.spear_stage {
                    1 => "use_spear_pose_1",
                    2 => "use_spear_pose_2",
                    _ => "use_spear_pose_3",
                };
                (name, ActionKind::Spear, Source::TwoHandedUse)
            }
            ItemKind::Eat => ("use_eat", ActionKind::Eat, Source::SustainedUse),
            ItemKind::Drink => ("use_drink", ActionKind::Drink, Source::SustainedUse),
            _ => ("use_item", ActionKind::Interact, Source::SustainedUse),
        };
        let clip = clips.resolve(name, u.hand).or_else(|| {
            (kind == ActionKind::Interact)
                .then(|| clips.resolve("interact", u.hand))
                .flatten()
        });
        if let Some(clip) = clip
            && needs_play(input, &clip, u.hand)
        {
            let mut p = play(kind, clip, u.hand);
            p.looping = match kind {
                ActionKind::Spear | ActionKind::Eat | ActionKind::Drink => false,
                ActionKind::Crossbow => {
                    if u.charge_duration_ticks > 0 {
                        p.clip.length < u.charge_duration_ticks as f32 / 20.0
                    } else {
                        !p.hold_on_last_frame
                    }
                }
                _ => !p.hold_on_last_frame,
            };
            if matches!(
                kind,
                ActionKind::Bow | ActionKind::Crossbow | ActionKind::Spear
            ) {
                p.speed_multiplier = 1.0 / input.animation_speed.max(0.01);
            }
            if matches!(kind, ActionKind::Bow | ActionKind::Crossbow) {
                p.blend_seconds = Some(0.25);
            }
            p.skip_blend_in = matches!(kind, ActionKind::Shield | ActionKind::Trident);
            offer(pending, source, p);
        }
    }

    fn release_use<C: ClipCatalog>(
        &mut self,
        input: &Input,
        old: UseState,
        clips: &C,
        pending: &mut Coordinator<Operation>,
    ) {
        let hand = old.hand;
        match old.kind {
            ItemKind::Bow => {
                if input.bow_shot_configured
                    && bow_shot_allowed(old.elapsed_ticks, input.has_bow_projectile_or_creative)
                    && let Some(clip) = clips.resolve("use_bow_shot", hand)
                {
                    let mut p = play(ActionKind::Bow, clip, hand);
                    p.speed_multiplier = clip_speed_multiplier(&p.clip.name, input.animation_speed);
                    p.blend_seconds = Some(0.05);
                    offer(pending, Source::TwoHandedUse, p);
                    return;
                }
                pending.request(
                    Source::TwoHandedUse,
                    Operation::Stop {
                        hand,
                        mode: StopMode::BlendOut(0.25),
                    },
                );
            }
            ItemKind::Crossbow => {
                let charged =
                    input.hands[hand.index()].crossbow_charged || self.crossbow_suppressed;
                pending.request(
                    Source::TwoHandedUse,
                    Operation::Stop {
                        hand,
                        mode: if charged {
                            StopMode::FinishCurrent
                        } else {
                            StopMode::BlendOut(0.25)
                        },
                    },
                );
                if !input.use_down {
                    self.crossbow_suppressed = false;
                }
            }
            ItemKind::Trident => {
                if !input.use_down && !input.hotbar_changed && old.elapsed_ticks >= 10 {
                    let special = if old.riptide {
                        Some("use_trident_throw_riptide")
                    } else if old.loyalty {
                        Some("use_trident_throw_loyalty")
                    } else {
                        None
                    };
                    let clip = special
                        .and_then(|name| clips.resolve(name, hand))
                        .or_else(|| clips.resolve("use_trident_throw", hand));
                    if let Some(clip) = clip {
                        let mut p = play(ActionKind::Trident, clip, hand);
                        p.skip_blend_in = true;
                        offer(pending, Source::TwoHandedUse, p);
                        return;
                    }
                }
                if input.playback_hand == Some(hand) {
                    pending.request(
                        Source::TwoHandedUse,
                        Operation::Stop {
                            hand,
                            mode: StopMode::BlendOut(0.25),
                        },
                    );
                }
            }
            ItemKind::Shield
                if input.use_down && input.hands[hand.index()].kind == ItemKind::Shield => {}
            _ => {
                if input.playback_hand == Some(hand) {
                    pending.request(
                        Source::SustainedUse,
                        Operation::Stop {
                            hand,
                            mode: StopMode::BlendOut(0.25),
                        },
                    );
                }
            }
        }
    }

    fn crossbow_shots<C: ClipCatalog>(
        &mut self,
        input: &Input,
        clips: &C,
        pending: &mut Coordinator<Operation>,
    ) {
        let click = input.use_down && !self.last_use_down;
        for hand in [Hand::Main, Hand::Off] {
            let i = hand.index();
            let h = &input.hands[i];
            if h.kind != ItemKind::Crossbow || h.blacklisted {
                self.predicted_crossbow_shot[i] = false;
                continue;
            }
            let actual = self.last_crossbow_charged[i] && !h.crossbow_charged;
            let predicted = click
                && h.crossbow_charged
                && (hand == Hand::Main || !input.hands[0].crossbow_charged);
            let start = predicted && !self.predicted_crossbow_shot[i]
                || actual
                    && !self.predicted_crossbow_shot[i]
                    && (input.use_down || self.last_use_down);
            if predicted {
                self.predicted_crossbow_shot[i] = true;
            }
            if !h.crossbow_charged {
                self.predicted_crossbow_shot[i] = false;
            }
            if start && let Some(clip) = clips.resolve("use_crossbow_shot", hand) {
                let mut p = play(ActionKind::Crossbow, clip, hand);
                p.blend_seconds = Some(0.05);
                p.speed_multiplier = 1.0 / input.animation_speed.max(0.01);
                offer(pending, Source::TwoHandedUse, p);
            }
        }
    }
}

fn play(kind: ActionKind, clip: ClipMeta, hand: Hand) -> Play {
    Play {
        hold_on_last_frame: clip.hold_on_last_frame,
        clip,
        kind,
        hand,
        looping: false,
        speed_multiplier: 1.0,
        skip_blend_in: false,
        blend_seconds: None,
        start_time: 0.0,
    }
}

fn offer(pending: &mut Coordinator<Operation>, source: Source, p: Play) {
    pending.request(source, Operation::Play(p));
}

fn needs_play(input: &Input, clip: &ClipMeta, hand: Hand) -> bool {
    !input.playback_active
        || input.playback_clip.as_deref() != Some(clip.name.as_str())
        || input.playback_hand != Some(hand)
}

fn first<C: ClipCatalog>(clips: &C, names: &[&str], hand: Hand) -> Option<ClipMeta> {
    names.iter().find_map(|name| clips.resolve(name, hand))
}

fn mining_clip<C: ClipCatalog>(
    clips: &C,
    kind: ItemKind,
    hand: Hand,
    normal: bool,
) -> Option<ClipMeta> {
    // Punchy's shipped alias is intentionally spelled "foward".
    // The ordinary mining path starts at punch before considering item kind;
    // unlike the air-swing fallback, it retains that clip for pickaxes/hoes/
    // shovels when a pack supplies it. Custom mining entries resolve upstream.
    if normal && kind != ItemKind::Axe {
        let punch = clips.resolve("punch", hand);
        if kind == ItemKind::Trident {
            return clips.resolve("attack_trident", hand).or(punch);
        }
        if punch.is_some() {
            return punch;
        }
    }
    match kind {
        ItemKind::Axe => first(
            clips,
            &[
                "mining_sideways",
                "mining",
                "mining_foward",
                "mining_forward",
                "mining_digging",
            ],
            hand,
        ),
        ItemKind::Pickaxe | ItemKind::Hoe => first(
            clips,
            &[
                "mining",
                "mining_foward",
                "mining_forward",
                "mining_sideways",
                "mining_digging",
            ],
            hand,
        ),
        ItemKind::Shovel => first(
            clips,
            &[
                "mining",
                "mining_foward",
                "mining_forward",
                "mining_sideways",
                "mining_digging",
            ],
            hand,
        ),
        ItemKind::Trident if normal => first(clips, &["attack_trident", "punch"], hand),
        _ if normal => clips.resolve("punch", hand),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Catalog(&'static [&'static str]);
    impl ClipCatalog for Catalog {
        fn resolve(&self, name: &str, _: Hand) -> Option<ClipMeta> {
            self.0.contains(&name).then(|| ClipMeta {
                name: name.into(),
                length: 1.0,
                hold_on_last_frame: false,
                dual_handed: false,
            })
        }
    }
    fn selected(intent: Intent) -> Play {
        match intent.operation {
            Operation::Play(p) => p,
            _ => panic!("expected play"),
        }
    }

    #[test]
    fn arbitration_uses_last_equal_priority_and_flush_clears() {
        let mut c = Coordinator::default();
        c.request(Source::SustainedUse, "inspect");
        c.request(Source::Attack, "attack");
        c.request(Source::SustainedUse, "consume");
        c.request(Source::Equip, "equip");
        assert_eq!(c.flush(), Some((Source::SustainedUse, "consume")));
        assert_eq!(c.flush(), None);
        c.request(Source::Attack, "attack");
        c.discard_for_equip_transition();
        assert_eq!(c.flush(), None);
    }

    #[test]
    fn held_attack_scheduler_keeps_fractional_cadence() {
        let mut c = AttackClock::default();
        let mut plays = Vec::new();
        for tick in 1..=41 {
            if c.cooling_down(tick, false) {
                c.defer();
            } else {
                plays.push(tick);
                c.record_playback(tick, false);
            }
        }
        assert_eq!(plays, [1, 7, 13, 19, 24, 30, 36, 41]);
        assert!(!c.cooling_down(41, true));
    }

    #[test]
    fn combo_boundary_is_strictly_more_than_thirty_ticks() {
        let mut c = AttackClock::default();
        assert_eq!(c.next_combo_index(1, 3), 0);
        assert_eq!(c.next_combo_index(31, 3), 1);
        assert_eq!(c.next_combo_index(62, 3), 0);
        assert_eq!(c.next_combo_index(63, 2), 0);
        assert_eq!(c.next_combo_index(64, 1), 0);
        assert_eq!(c.next_combo_index(65, 2), 1);
    }

    #[test]
    fn spear_item_ids_are_classified_as_spear() {
        for id in [
            "minecraft:spear",
            "minecraft:wooden_spear",
            "minecraft:stone_spear",
            "minecraft:copper_spear",
            "minecraft:iron_spear",
            "minecraft:golden_spear",
            "minecraft:diamond_spear",
            "minecraft:netherite_spear",
        ] {
            assert_eq!(ItemKind::from_id(id), ItemKind::Spear, "{id}");
        }
    }

    #[test]
    fn bow_requires_three_ticks_and_projectile() {
        assert!(!bow_shot_allowed(2, true));
        assert!(bow_shot_allowed(3, true));
        assert!(!bow_shot_allowed(20, false));
    }

    #[test]
    fn spear_stage_edges_and_limit_match_java() {
        assert_eq!(spear_charge_stage(9, 5, 15, 25, 35), (1, false));
        assert_eq!(spear_charge_stage(10, 5, 15, 25, 35), (2, false));
        assert_eq!(spear_charge_stage(20, 5, 15, 25, 35), (3, false));
        assert_eq!(spear_charge_stage(30, 5, 15, 25, 35), (3, false));
        assert_eq!(spear_charge_stage(31, 5, 15, 25, 35), (3, true));
        assert_eq!(spear_charge_stage(1000, 5, 0, 0, 0), (1, false));
    }

    #[test]
    fn generic_held_use_uses_interaction_clip_when_item_clip_is_missing() {
        let mut machine = ActionMachine::default();
        let input = Input {
            use_down: true,
            using: Some(UseState {
                kind: ItemKind::Other,
                hand: Hand::Off,
                ..Default::default()
            }),
            ..Default::default()
        };
        let play = selected(machine.tick(&input, &Catalog(&["interact"])).unwrap());
        assert_eq!(play.kind, ActionKind::Interact);
        assert_eq!(play.hand, Hand::Off);
        assert_eq!(play.clip.name, "interact");
        assert!(play.looping);
    }

    #[test]
    fn special_use_items_select_their_dedicated_animation_clips() {
        let cases = [
            (ItemKind::Bow, "use_bow", ActionKind::Bow, 1),
            (ItemKind::Trident, "use_trident", ActionKind::Trident, 1),
            (ItemKind::Spear, "use_spear_pose_2", ActionKind::Spear, 2),
            (ItemKind::Eat, "use_eat", ActionKind::Eat, 1),
            (ItemKind::Drink, "use_drink", ActionKind::Drink, 1),
        ];
        for (kind, clip, expected_kind, spear_stage) in cases {
            let mut machine = ActionMachine::default();
            let input = Input {
                use_down: true,
                using: Some(UseState {
                    kind,
                    spear_stage,
                    ..Default::default()
                }),
                ..Default::default()
            };
            let play = selected(machine.tick(&input, &Catalog(&[clip])).unwrap());
            assert_eq!(play.kind, expected_kind, "{clip}");
            assert_eq!(play.clip.name, clip, "{clip}");
        }
    }

    #[test]
    fn bow_release_selects_the_bow_shoot_clip_after_a_valid_draw() {
        let mut machine = ActionMachine::default();
        let catalog = Catalog(&["use_bow", "use_bow_shot"]);
        let mut input = Input {
            use_down: true,
            using: Some(UseState {
                kind: ItemKind::Bow,
                elapsed_ticks: 20,
                ..Default::default()
            }),
            has_bow_projectile_or_creative: true,
            bow_shot_configured: true,
            ..Default::default()
        };
        assert_eq!(selected(machine.tick(&input, &catalog).unwrap()).clip.name, "use_bow");
        input.use_down = false;
        input.using = None;
        let play = selected(machine.tick(&input, &catalog).unwrap());
        assert_eq!(play.kind, ActionKind::Bow);
        assert_eq!(play.clip.name, "use_bow_shot");
    }

    #[test]
    fn trident_release_uses_riptide_or_loyalty_specific_throw_clip() {
        for (riptide, loyalty, expected) in [
            (true, false, "use_trident_throw_riptide"),
            (false, true, "use_trident_throw_loyalty"),
            (false, false, "use_trident_throw"),
        ] {
            let mut machine = ActionMachine::default();
            let catalog = Catalog(&[
                "use_trident",
                "use_trident_throw",
                "use_trident_throw_riptide",
                "use_trident_throw_loyalty",
            ]);
            let mut input = Input {
                use_down: true,
                using: Some(UseState {
                    kind: ItemKind::Trident,
                    elapsed_ticks: 12,
                    riptide,
                    loyalty,
                    ..Default::default()
                }),
                ..Default::default()
            };
            assert_eq!(selected(machine.tick(&input, &catalog).unwrap()).clip.name, "use_trident");
            input.use_down = false;
            input.using = None;
            let play = selected(machine.tick(&input, &catalog).unwrap());
            assert_eq!(play.kind, ActionKind::Trident);
            assert_eq!(play.clip.name, expected);
        }
    }

    #[test]
    fn successful_interaction_event_uses_interact_clip_for_offhand() {
        let mut machine = ActionMachine::default();
        let input = Input {
            use_trigger: Some(Hand::Off),
            ..Default::default()
        };
        let play = selected(machine.tick(&input, &Catalog(&["interact"])).unwrap());
        assert_eq!(play.kind, ActionKind::Interact);
        assert_eq!(play.hand, Hand::Off);
        assert_eq!(play.clip.name, "interact");
        assert!(!play.looping);
    }

    #[test]
    fn phased_use_reserves_priority_without_spending_queued_attack() {
        let mut machine = ActionMachine::default();
        let mut input = Input {
            attack: Some(Hand::Main),
            external_priority: Some(80),
            ..Default::default()
        };
        let clips = Catalog(&["attack"]);
        assert!(machine.tick(&input, &clips).is_none());
        input.attack = None;
        input.external_priority = None;
        assert_eq!(
            selected(machine.tick(&input, &clips).unwrap()).kind,
            ActionKind::Attack
        );
    }
    #[test]
    fn blacklisted_attack_does_not_advance_clock() {
        let mut machine = ActionMachine::default();
        let mut i = Input {
            attack: Some(Hand::Main),
            ..Default::default()
        };
        i.hands[0].blacklisted = true;
        assert!(machine.tick(&i, &Catalog(&["attack"])).is_none());
        i.hands[0].blacklisted = false;
        assert_eq!(
            selected(machine.tick(&i, &Catalog(&["attack"])).unwrap()).kind,
            ActionKind::Attack
        );
    }

    #[test]
    fn two_handed_use_beats_attack_and_attack_clock_is_not_committed() {
        let mut machine = ActionMachine::default();
        let mut i = Input {
            attack: Some(Hand::Main),
            use_down: true,
            using: Some(UseState {
                kind: ItemKind::Bow,
                ..Default::default()
            }),
            ..Default::default()
        };
        let c = Catalog(&["attack", "use_bow"]);
        assert_eq!(
            selected(machine.tick(&i, &c).unwrap()).kind,
            ActionKind::Bow
        );
        i.using = None;
        i.use_down = false;
        machine.tick(&i, &c); // release owns the second tick
        assert_eq!(machine.attack_clock.next_visual_tick, -1.0);
        i.attack = Some(Hand::Main);
        assert_eq!(
            selected(machine.tick(&i, &c).unwrap()).kind,
            ActionKind::Attack
        );
    }

    #[test]
    fn trident_release_does_not_throw_after_hotbar_swap() {
        let mut machine = ActionMachine::default();
        let c = Catalog(&["use_trident", "use_trident_throw"]);
        let mut i = Input {
            use_down: true,
            using: Some(UseState {
                kind: ItemKind::Trident,
                elapsed_ticks: 12,
                ..Default::default()
            }),
            ..Default::default()
        };
        assert_eq!(
            selected(machine.tick(&i, &c).unwrap()).clip.name,
            "use_trident"
        );
        i.using = None;
        i.use_down = false;
        i.hotbar_changed = true;
        i.playback_hand = Some(Hand::Main);
        assert!(matches!(
            machine.tick(&i, &c).unwrap().operation,
            Operation::Stop { .. }
        ));
    }

    #[test]
    fn inspect_tries_main_then_off_and_waits_for_equip() {
        let c = Catalog(&["inspect"]);
        let mut m = ActionMachine::default();
        let mut i = Input {
            inspect: true,
            equip_in_progress: true,
            ..Default::default()
        };
        assert!(m.tick(&i, &c).is_none());
        i.equip_in_progress = false;
        i.inspect = false; // tracker event remains pending across equip
        i.hands[0].blacklisted = true;
        assert_eq!(selected(m.tick(&i, &c).unwrap()).hand, Hand::Off);
    }

    #[test]
    fn queued_attacks_wait_for_rate_gate_and_get_source_speed_boost() {
        let c = Catalog(&["attack"]);
        let mut m = ActionMachine::default();
        let mut i = Input {
            attack: Some(Hand::Main),
            ..Default::default()
        };
        assert_eq!(selected(m.tick(&i, &c).unwrap()).speed_multiplier, 1.0);
        assert!(m.tick(&i, &c).is_none());
        i.attack = None;
        for _ in 3..=6 {
            assert!(m.tick(&i, &c).is_none());
        }
        assert_eq!(selected(m.tick(&i, &c).unwrap()).speed_multiplier, 1.12);
    }

    #[test]
    fn main_charged_crossbow_wins_prediction_and_uncharge_does_not_repeat() {
        let c = Catalog(&["use_crossbow_shot"]);
        let mut m = ActionMachine::default();
        let mut i = Input {
            use_down: true,
            ..Default::default()
        };
        for h in &mut i.hands {
            h.kind = ItemKind::Crossbow;
            h.crossbow_charged = true;
        }
        let shot = selected(m.tick(&i, &c).unwrap());
        assert_eq!(shot.hand, Hand::Main);
        assert_eq!(shot.blend_seconds, Some(0.05));
        assert!((shot.speed_multiplier - 1.0 / 5.5).abs() < 1e-6);
        i.hands[0].crossbow_charged = false;
        assert!(m.tick(&i, &c).is_none());
        i.use_down = false;
        assert!(m.tick(&i, &c).is_none());
        i.use_down = true;
        assert_eq!(selected(m.tick(&i, &c).unwrap()).hand, Hand::Off);
    }

    #[test]
    fn shield_reuses_owned_clip_and_keeps_visual_hold_until_key_release() {
        let c = Catalog(&["use_shield"]);
        let mut m = ActionMachine::default();
        let mut i = Input {
            use_down: true,
            using: Some(UseState {
                kind: ItemKind::Shield,
                hand: Hand::Off,
                ..Default::default()
            }),
            ..Default::default()
        };
        i.hands[1].kind = ItemKind::Shield;
        let p = selected(m.tick(&i, &c).unwrap());
        assert!(p.looping);
        i.playback_active = true;
        i.playback_hand = Some(Hand::Off);
        i.playback_clip = Some("use_shield".into());
        i.using = None;
        assert!(m.tick(&i, &c).is_none());
        i.use_down = false;
        // The active gameplay use was already absent on the previous tick:
        // persistent visual ownership still needs to be explicitly stopped.
        assert!(matches!(
            m.tick(&i, &c).map(|v| v.operation),
            Some(Operation::Stop {
                hand: Hand::Off,
                ..
            })
        ));
    }
}
