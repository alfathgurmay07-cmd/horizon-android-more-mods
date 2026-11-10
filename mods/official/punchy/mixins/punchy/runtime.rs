// Horizon glue. This is an integration preview, not complete Punchy renderer parity.
use super::*;
use glam::{EulerRot, Mat4, Quat, Vec3};
use punchy_engine::player::Playback;
use punchy_engine::{Clip, Pose, actions as action, phases, resources};
use std::{cell::RefCell, collections::{BTreeMap, VecDeque}, fs, path::Path, sync::Arc, time::Instant};

pub(super) const ID: &str = "punchy";
pub(super) const TITLE: &str = "Punchy";

pub(super) fn install_builtin_resource_pack(resources_root: &Path) -> std::io::Result<()> {
    let path = resources_root.join("assets/minecraft/resourcepacks/punchy.zip");
    let bytes = punchy_engine::resources::builtin_resource_pack();
    if fs::read(&path).ok().as_deref() != Some(bytes) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, bytes)?;
    }
    Ok(())
}

thread_local! { static STATE: RefCell<State> = RefCell::new(State::default()); }
#[derive(Default)]
struct State {
    attempted: bool,
    source: Option<resources::Resources>,
    clips: BTreeMap<String, Arc<Clip>>,
    machine: action::ActionMachine,
    consume: phases::ConsumeController,
    mining: phases::MiningController,
    tick: i64,
    playback_id: u64,
    stack_keys: Vec<ItemStack>,
    forced_stack: Option<(usize, ItemStack)>,
    consume_hand: action::Hand,
    active: Option<Playback>,
    hand_motion: punchy_engine::hand_motion::HandMotion,
    pendulum_states: [punchy_engine::pendulum::State; 2],
    pendulum_items: [String; 2],
    pendulum_last_frame: Option<Instant>,
    pendulum_camera: Option<(f32, f32)>,
    pendulum_offsets: [f32; 2],
    pendulum_action_values: [f32; 2],
    pendulum_action_velocities: [f32; 2],
    pendulum_action_kicks: [f32; 2],
    idle_layers: [IdleLayer; 2],
    motion_blocked: bool,
    previous_swings: u32,
    use_down: bool,
    successful_use: VecDeque<action::Hand>,
    item_kinds: [Option<String>; 2],
    mining_attempt: bool,
    offhand_item: bool,
    inspect: bool,
    last_items: [String; 2],
    error: Option<String>,
    main_left: bool,
    render_alpha: f32,
    enabled: bool,
    tuning: Option<punchy_engine::tuning::Tuning>,
    status: String,
    render_count: u64,
    resource_source: String,
    editor: HandEditor,
    last_slider: Option<String>,
}
#[derive(Clone, Debug, Default)]
struct IdleLayer {
    item: String,
    clip: Option<String>,
    time: f32,
    previous_time: f32,
    weight: f32,
    previous_weight: f32,
}
#[derive(Clone, Debug)]
struct HandEditor {
    selected_hand: usize,
    selected_target: String,
    selected_scope: String,
    selected_selector: String,
    items: serde_json::Value,
    rules: Vec<serde_json::Value>,
    groups: serde_json::Value,
    profiles: Vec<serde_json::Value>,
    profile_index: usize,
    undo: Vec<serde_json::Value>,
    redo: Vec<serde_json::Value>,
}
impl Default for HandEditor {
    fn default() -> Self {
        Self {
            selected_hand: 0,
            selected_target: "item".into(),
            selected_scope: "item".into(),
            selected_selector: String::new(),
            items: serde_json::json!({}),
            rules: Vec::new(),
            groups: serde_json::json!({}),
            profiles: Vec::new(),
            profile_index: 0,
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }
}
impl HandEditor {
    fn parse(raw: Option<String>) -> Self {
        let Some(raw) = raw else { return Self::default() };
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw) else { return Self::default() };
        Self {
            selected_hand: value.get("selected_hand").and_then(|v| v.as_u64()).unwrap_or(0).min(1) as usize,
            selected_target: value.get("selected_target").and_then(|v| v.as_str()).filter(|v| matches!(*v, "item" | "arm" | "line")).unwrap_or("item").to_owned(),
            selected_scope: value.get("selected_scope").and_then(|v| v.as_str()).filter(|v| matches!(*v, "item" | "tag" | "group" | "all")).unwrap_or("item").to_owned(),
            selected_selector: value.get("selected_selector").and_then(|v| v.as_str()).unwrap_or_default().to_owned(),
            items: value.get("items").cloned().filter(serde_json::Value::is_object).unwrap_or_else(|| serde_json::json!({})),
            rules: value.get("rules").and_then(|v| v.as_array()).cloned().unwrap_or_default(),
            groups: value.get("groups").cloned().filter(serde_json::Value::is_object).unwrap_or_else(|| serde_json::json!({})),
            profiles: value.get("profiles").and_then(|v| v.as_array()).cloned().unwrap_or_default(),
            profile_index: value.get("profile_index").and_then(|v| v.as_u64()).unwrap_or(0) as usize,
            undo: value.get("undo").and_then(|v| v.as_array()).cloned().unwrap_or_default(),
            redo: value.get("redo").and_then(|v| v.as_array()).cloned().unwrap_or_default(),
        }
    }
    fn snapshot(&self) -> serde_json::Value {
        serde_json::json!({"items":self.items,"rules":self.rules,"groups":self.groups})
    }
    fn restore(&mut self, snapshot: &serde_json::Value) {
        self.items = snapshot.get("items").cloned().filter(serde_json::Value::is_object).unwrap_or_else(|| serde_json::json!({}));
        self.rules = snapshot.get("rules").and_then(|v| v.as_array()).cloned().unwrap_or_default();
        self.groups = snapshot.get("groups").cloned().filter(serde_json::Value::is_object).unwrap_or_else(|| serde_json::json!({}));
    }
    fn remember(&mut self) {
        if self.undo.len() >= 24 { self.undo.remove(0); }
        let snapshot = self.snapshot();
        self.undo.push(snapshot);
        self.redo.clear();
    }
    fn save(&self, game: &Game<'_>) {
        let value = serde_json::json!({
            "selected_hand": self.selected_hand,
            "selected_target": self.selected_target,
            "selected_scope": self.selected_scope,
            "selected_selector": self.selected_selector,
            "items": self.items,
            "rules": self.rules,
            "groups": self.groups,
            "profiles": self.profiles,
            "profile_index": self.profile_index,
            "undo": self.undo,
            "redo": self.redo,
        });
        if let Ok(raw) = serde_json::to_string_pretty(&value) {
            game.native_set_setting(ID, "hand_editor.json", &raw);
        }
    }
    fn transform_mut(&mut self, item: &str, left: bool, target: &str, scope: &str, selector: &str) -> &mut serde_json::Value {
        if scope == "item" {
            let side = if left { "left" } else { "right" };
            let root = self.items.as_object_mut().expect("editor items are an object");
            let entry = root.entry(item.to_owned()).or_insert_with(|| serde_json::json!({}));
            let side = entry.as_object_mut().expect("editor item is an object").entry(side).or_insert_with(|| serde_json::json!({}));
            return side.as_object_mut().expect("editor side is an object").entry(target.to_owned()).or_insert_with(|| serde_json::json!({}));
        }
        let side = if left { "left" } else { "right" };
        if let Some(index) = self.rules.iter().position(|rule| {
            rule.get("scope").and_then(|v|v.as_str()) == Some(scope)
                && rule.get("selector").and_then(|v|v.as_str()).unwrap_or_default() == selector
                && rule.get("side").and_then(|v|v.as_str()) == Some(side)
                && rule.get("target").and_then(|v|v.as_str()) == Some(target)
        }) {
            return self.rules[index].get_mut("transform").expect("editor rule has a transform");
        }
        self.rules.push(serde_json::json!({"scope":scope,"selector":selector,"side":side,"target":target,"transform":{}}));
        self.rules.last_mut().unwrap().get_mut("transform").unwrap()
    }
}

fn held_item_id(game: &Game<'_>, hand: usize) -> String {
    let stack = if hand == 0 { game.held() } else { game.offhand.as_ref() };
    stack.map_or_else(|| "minecraft:air".to_owned(), |v| v.item.to_string())
}
fn editor_left(game: &Game<'_>, hand: usize) -> bool {
    game.settings.main_hand_left != (hand == 1)
}
fn held_tags(game: &Game<'_>, item: &str) -> Vec<String> {
    game.items().item_tags.iter()
        .filter(|(_, members)| members.contains(item))
        .map(|(tag, _)| tag.clone())
        .collect()
}
fn custom_group_names(state: &State) -> Vec<String> {
    let mut names = Vec::new();
    if let Some(groups) = state.editor.groups.as_object() {
        names.extend(groups.keys().cloned());
    }
    if let Some(groups) = state.tuning.as_ref().and_then(|t| t.document.get("customGroups")).and_then(serde_json::Value::as_array) {
        names.extend(groups.iter().filter_map(|g| g.get("name").and_then(|v|v.as_str()).map(str::to_owned)));
    }
    names.sort();
    names.dedup();
    names
}
fn selector_choices(game: &Game<'_>, state: &State, item: &str) -> Vec<String> {
    match state.editor.selected_scope.as_str() {
        "tag" => held_tags(game, item),
        "group" => custom_group_names(state),
        "all" => vec!["*".into()],
        _ => vec![item.to_owned()],
    }
}
fn selector_members(state: &State, name: &str) -> Vec<String> {
    let mut members = state.editor.groups.get(name).and_then(serde_json::Value::as_array)
        .map(|v|v.iter().filter_map(|x|x.as_str().map(str::to_owned)).collect::<Vec<_>>()).unwrap_or_default();
    if members.is_empty() {
        if let Some(groups) = state.tuning.as_ref().and_then(|t|t.document.get("customGroups")).and_then(serde_json::Value::as_array) {
            if let Some(group) = groups.iter().find(|g|g.get("name").and_then(|v|v.as_str()) == Some(name)) {
                members = group.get("items").and_then(serde_json::Value::as_array)
                    .map(|v|v.iter().filter_map(|x|x.as_str().map(str::to_owned)).collect()).unwrap_or_default();
            }
        }
    }
    members
}
fn selector_matches(selector: &str, item: &str, tags: &[String]) -> bool {
    if let Some(tag) = selector.strip_prefix('#') {
        tags.iter().any(|t| t.as_str() == tag)
    } else if selector.starts_with("minecraft:") || selector.contains(':') {
        selector == item
    } else {
        false
    }
}
fn group_matches(state: &State, selector: &str, item: &str, tags: &[String]) -> bool {
    selector_members(state, selector).iter().any(|member| selector_matches(member, item, tags))
}
fn transform_component(transform: &punchy_engine::tuning::Transform, key: &str) -> f32 {
    match key {
        "scale" => transform.scale,
        "offX" => transform.offset[0], "offY" => transform.offset[1], "offZ" => transform.offset[2],
        "rotX" => transform.rotation[0], "rotY" => transform.rotation[1], "rotZ" => transform.rotation[2],
        "pivotX" => transform.pivot[0], "pivotY" => transform.pivot[1], "pivotZ" => transform.pivot[2],
        _ => 0.0,
    }
}
fn set_transform_component(transform: &mut punchy_engine::tuning::Transform, key: &str, value: f32) {
    match key {
        "scale" => transform.scale = value,
        "offX" => transform.offset[0] = value, "offY" => transform.offset[1] = value, "offZ" => transform.offset[2] = value,
        "rotX" => transform.rotation[0] = value, "rotY" => transform.rotation[1] = value, "rotZ" => transform.rotation[2] = value,
        "pivotX" => transform.pivot[0] = value, "pivotY" => transform.pivot[1] = value, "pivotZ" => transform.pivot[2] = value,
        _ => {}
    }
}
fn transform_value(transform: &punchy_engine::tuning::Transform) -> serde_json::Value {
    serde_json::json!({"scale":transform.scale,"offX":transform.offset[0],"offY":transform.offset[1],"offZ":transform.offset[2],
        "rotX":transform.rotation[0],"rotY":transform.rotation[1],"rotZ":transform.rotation[2],
        "pivotX":transform.pivot[0],"pivotY":transform.pivot[1],"pivotZ":transform.pivot[2]})
}
fn apply_transform_json(base: &mut punchy_engine::tuning::Transform, values: &serde_json::Value) {
    if let Some(values) = values.as_object() {
        for (key, value) in values {
            if let Some(number) = value.as_f64().filter(|n| n.is_finite()) {
                set_transform_component(base, key, number as f32);
            }
        }
    }
}
fn editor_base_transform(state: &State, item: &str, left: bool, arm: bool, tags: &[String]) -> punchy_engine::tuning::Transform {
    let refs = tags.iter().map(String::as_str).collect::<Vec<_>>();
    let context = punchy_engine::tuning::ItemContext { id: item, tags: &refs, ..Default::default() };
    state.tuning.as_ref().and_then(|t|t.transform_for(&context,left,arm)).unwrap_or_default()
}
fn editor_transform(
    state: &State, item: &str, left: bool, target: &str, tags: &[String],
) -> punchy_engine::tuning::Transform {
    let arm = target == "arm";
    let mut base = if target == "line" { punchy_engine::tuning::Transform::default() }
        else { editor_base_transform(state,item,left,arm,tags) };
    let side = if left { "left" } else { "right" };
    for scope in ["all", "group", "tag", "item"] {
        if scope == "item" {
            if let Some(values) = state.editor.items.get(item).and_then(|v|v.get(side)).and_then(|v|v.get(target)) {
                apply_transform_json(&mut base, values);
            }
            continue;
        }
        for rule in &state.editor.rules {
            if rule.get("scope").and_then(|v|v.as_str()) != Some(scope)
                || rule.get("side").and_then(|v|v.as_str()) != Some(side)
                || rule.get("target").and_then(|v|v.as_str()) != Some(target) { continue; }
            let selector = rule.get("selector").and_then(|v|v.as_str()).unwrap_or_default();
            let matches = match scope {
                "all" => true,
                "tag" => tags.iter().any(|tag| tag == selector),
                "group" => group_matches(state,selector,item,tags),
                _ => false,
            };
            if matches { apply_transform_json(&mut base, rule.get("transform").unwrap_or(&serde_json::Value::Null)); }
        }
    }
    base
}
fn editor_scope_transform(
    state: &State,
    item: &str,
    left: bool,
    target: &str,
    scope: &str,
    selector: &str,
) -> punchy_engine::tuning::Transform {
    let mut transform = punchy_engine::tuning::Transform::default();
    if scope == "item" {
        let side = if left { "left" } else { "right" };
        if let Some(values) = state.editor.items.get(item).and_then(|v| v.get(side)).and_then(|v| v.get(target)) {
            apply_transform_json(&mut transform, values);
        }
    } else if let Some(rule) = state.editor.rules.iter().find(|rule| {
        rule.get("scope").and_then(|v| v.as_str()) == Some(scope)
            && rule.get("selector").and_then(|v| v.as_str()).unwrap_or_default() == selector
            && rule.get("side").and_then(|v| v.as_str()) == Some(if left { "left" } else { "right" })
            && rule.get("target").and_then(|v| v.as_str()) == Some(target)
    }) {
        apply_transform_json(&mut transform, rule.get("transform").unwrap_or(&serde_json::Value::Null));
    }
    transform
}

fn editor_has_line(state: &State, left: bool) -> bool {
    let side = if left { "left" } else { "right" };
    state.editor.rules.iter().any(|rule| rule.get("target").and_then(|v|v.as_str()) == Some("line")
        && rule.get("side").and_then(|v|v.as_str()) == Some(side))
}

fn inspect_clip_aliases(item: &str, kind: Option<&str>, left: bool) -> Vec<String> {
    let side = if left { "left" } else { "right" };
    let other = if left { "right" } else { "left" };
    let kind = kind.unwrap_or_else(|| punchy_engine::tuning::vanilla_kind(item));
    if kind.eq_ignore_ascii_case("PICKAXE") {
        vec![
            format!("new_inspect_pickaxe_{side}"),
            format!("new_inspect_{side}"),
            format!("new_inspect_pickaxe_{other}"),
            format!("new_inspect_{other}"),
        ]
    } else {
        vec![format!("new_inspect_{side}"), format!("new_inspect_{other}")]
    }
}

struct Catalog<'a> {
    state: &'a State,
}
impl action::ClipCatalog for Catalog<'_> {
    fn resolve(&self, name: &str, hand: action::Hand) -> Option<action::ClipMeta> {
        let left = self.state.main_left != (hand == action::Hand::Off);
        let side = if left { "left" } else { "right" };
        let handed = format!("{name}_{side}");
        let aliases: Vec<String> = match name {
            "use_drink" => ["use_drink2", "drink_loop", "drink_loop2"].into_iter().map(str::to_owned).collect(),
            "use_shield" => ["use_shield"].into_iter().map(str::to_owned).collect(),
            "mining" => ["mining_foward", "mining_forward", "mining_sideways", "mining_digging"].into_iter().map(str::to_owned).collect(),
            "inspect" => {
                let index = hand.index();
                let item = self.state.last_items.get(index).map(String::as_str).unwrap_or("");
                let kind = self.state.item_kinds.get(index).and_then(Option::as_deref);
                inspect_clip_aliases(item, kind, left)
            }
            _ => Vec::new(),
        };
        let mapped = tuning_animation(self.state, hand, name);
        let custom = mapped
            .as_ref()
            .and_then(|m| m.name.as_deref())
            .and_then(|n| {
                punchy_engine::tuning::handed_candidates(n, left)
                    .into_iter()
                    .find_map(|n| self.state.clips.get(&n))
            });
        let bow_shot_alias = match name {
            "use_bow_shot" => Some(if left {
                "use_bow_shoot_left"
            } else {
                "use_bow_shoot_right"
            }),
            _ => None,
        };
        let clip = custom
            .or_else(|| self.state.clips.get(&handed))
            .or_else(|| self.state.clips.get(name))
            .or_else(|| bow_shot_alias.and_then(|n| self.state.clips.get(n)))
            .or_else(|| aliases.iter().find_map(|n| self.state.clips.get(n)))?;
        Some(action::ClipMeta {
            name: clip.name.clone(),
            length: clip.length,
            hold_on_last_frame: clip.hold_on_last_frame,
            dual_handed: punchy_engine::policy::dual_handed(
                clip,
                0.,
                self.state.offhand_item,
                self.state.offhand_item,
            ),
        })
    }
}
fn load(game: &Game<'_>, state: &mut State) {
    if state.attempted { return; }
    state.attempted = true;
    state.editor = HandEditor::parse(game.native_setting(ID, "hand_editor.json"));
    let pack_priorities = game.native_data("data/horizon_internal/punchy_resource_priorities.json")
        .and_then(|bytes| serde_json::from_slice::<BTreeMap<String, i64>>(&bytes).ok())
        .unwrap_or_default();
    let stack_manifest = game.native_data("data/horizon_internal/punchy_resource_stacks.json")
        .and_then(|bytes| serde_json::from_slice::<BTreeMap<String, Vec<(i64, String)>>>(&bytes).ok())
        .unwrap_or_default();
    let stack_files: BTreeMap<String, Vec<u8>> = game.native_data_tree("data/horizon_internal/punchy_resource_stack")
        .into_iter()
        .collect();
    let mut animation_files: Vec<(String, Vec<u8>, i64)> = game.native_data_tree("assets/punchy/animations")
        .into_iter()
        .chain(game.native_data_tree("assets/minecraft/punchy/animations"))
        .map(|(path, bytes)| {
            let priority = pack_priorities.get(&path).copied().unwrap_or(-1);
            (path, bytes, priority)
        })
        .collect();
    let mut compat_files: Vec<(String, Vec<u8>, i64)> = game.native_data_tree("assets/minecraft/punchy/compat")
        .into_iter()
        .chain(game.native_data_tree("assets/punchy/compat"))
        .map(|(path, bytes)| {
            let priority = pack_priorities.get(&path).copied().unwrap_or(-1);
            (path, bytes, priority)
        })
        .collect();
    for (asset_path, layers) in &stack_manifest {
        let is_animation = asset_path.starts_with("assets/punchy/animations/")
            || asset_path.starts_with("assets/minecraft/punchy/animations/");
        let is_compat = asset_path.starts_with("assets/minecraft/punchy/compat/")
            || asset_path.starts_with("assets/punchy/compat/");
        if !is_animation && !is_compat { continue; }
        let visible_priority = pack_priorities.get(asset_path).copied();
        for (priority, stored_path) in layers {
            if visible_priority == Some(*priority) { continue; }
            let Some(bytes) = stack_files.get(stored_path) else { continue };
            if is_animation {
                animation_files.push((asset_path.clone(), bytes.clone(), *priority));
            } else {
                compat_files.push((asset_path.clone(), bytes.clone(), *priority));
            }
        }
    }
    let geometry = game.native_data(resources::PACK_GEOMETRY);
    let default_tuning = game.native_data("assets/punchy/default_tuning.json");
    let result = resources::from_active_pack_tree(
        &animation_files,
        geometry.as_deref(),
        default_tuning.as_deref(),
        &compat_files,
    ).and_then(|(source, has_pack_data)| {
        let clips = punchy_engine::load_clips(&source.animations)?;
        Ok((source, clips, has_pack_data))
    });
    match result {
        Ok((source, clips, has_pack_data)) => {
            state.status = format!("{} clips loaded{}", clips.len(), if has_pack_data { " · resource pack" } else { " · bundled" });
            state.resource_source = if has_pack_data { "resource_pack" } else { "embedded" }.into();
            state.clips = clips.into_iter().map(|(key, clip)| (key, Arc::new(clip))).collect();
            state.tuning = punchy_engine::tuning::Tuning::parse(&source.tuning).ok();
            state.source = Some(source);
            state.error = None;
        }
        Err(error) => {
            state.status = "Punchy resource-pack data invalid; using bundled resources".into();
            state.resource_source = "error".into();
            state.error = Some(error);
            if let Ok(source) = resources::embedded() {
                if let Ok(clips) = punchy_engine::load_clips(&source.animations) {
                    state.clips = clips.into_iter().map(|(key, clip)| (key, Arc::new(clip))).collect();
                    state.tuning = punchy_engine::tuning::Tuning::parse(&source.tuning).ok();
                    state.source = Some(source);
                }
            }
        }
    }
}

pub(super) fn resources_reloaded() {
    STATE.with(|cell| {
        let mut state = cell.borrow_mut();
        state.attempted = false;
        state.source = None;
        state.clips.clear();
        state.tuning = None;
        state.active = None;
        state.hand_motion = Default::default();
        state.pendulum_states = Default::default();
        state.pendulum_items = Default::default();
        state.pendulum_last_frame = None;
        state.pendulum_camera = None;
        state.idle_layers = Default::default();
        state.motion_blocked = false;
        state.machine = Default::default();
        state.consume = Default::default();
        state.mining = Default::default();
        state.resource_source.clear();
    });
}

fn enabled(game: &Game<'_>) -> bool {
    game.native_on(ID)
}
fn global_speed(game: &Game<'_>) -> f32 {
    game.native_setting(ID, "speed")
        .and_then(|s| s.parse::<f32>().ok())
        .filter(|s| s.is_finite())
        .unwrap_or(5.5)
        .clamp(0.5, 5.5)
}
fn attack_held(
    mouse_down: bool,
    mapped_pad_down: bool,
    touch_trigger: f32,
    threshold: f32,
) -> bool {
    mouse_down || mapped_pad_down || touch_trigger >= threshold
}
fn hand(h: Hand) -> action::Hand {
    if h == Hand::Main {
        action::Hand::Main
    } else {
        action::Hand::Off
    }
}
fn tuning_animation(
    state: &State,
    hand: action::Hand,
    action_name: &str,
) -> Option<punchy_engine::tuning::Animation> {
    let index = hand.index();
    let item = &state.last_items[index];
    let context = punchy_engine::tuning::ItemContext {
        id: item,
        kind: state.item_kinds[index].as_deref(),
        ..Default::default()
    };
    state.tuning.as_ref()?.animation_for(
        &context,
        action_name,
        state.main_left != (hand == action::Hand::Off),
    )
}

fn action_owns_arm(state: &State, left: bool) -> f32 {
    let Some(playback) = &state.active else { return 0.0 };
    let owner_left = state.main_left != (playback.hand == action::Hand::Off);
    if left != owner_left
        && !punchy_engine::policy::dual_handed(&playback.clip, playback.time, state.offhand_item, state.offhand_item)
    {
        return 0.0;
    }
    playback.previous_weight + (playback.weight - playback.previous_weight) * state.render_alpha
}

fn resolve_idle_clip(state: &State, left: bool) -> Option<String> {
    let hand = if left == state.main_left { action::Hand::Main } else { action::Hand::Off };
    let index = hand.index();
    let item = state.last_items.get(index)?;
    if item.is_empty() || item == "minecraft:air" { return None; }
    let mapped = tuning_animation(state, hand, "idle");
    let mut requested = Vec::new();
    if let Some(animation) = mapped {
        if let Some(name) = animation.name { requested.push(name); }
        requested.extend(animation.variations);
    }
    requested.extend(["idle".to_owned(), "punchy_idle".to_owned()]);
    requested.into_iter().flat_map(|name| {
        punchy_engine::tuning::handed_candidates(&name, left)
    }).find(|name| state.clips.contains_key(name))
}

fn update_idle_layers(state: &mut State, dt: f32, speed: f32, blocked: bool) {
    let names = [resolve_idle_clip(state, false), resolve_idle_clip(state, true)];
    for (index, name) in names.into_iter().enumerate() {
        let left = index == 1;
        let hand = if left == state.main_left { action::Hand::Main } else { action::Hand::Off };
        let item = state.last_items[hand.index()].clone();
        let owner = action_owns_arm(state, left);
        let layer = &mut state.idle_layers[index];
        if layer.item != item || layer.clip != name {
            layer.item = item;
            layer.clip = name;
            layer.time = 0.0;
            layer.previous_time = 0.0;
            layer.weight = 0.0;
            layer.previous_weight = 0.0;
        }
        layer.previous_time = layer.time;
        layer.previous_weight = layer.weight;
        let target = if layer.clip.is_some() && !blocked && owner < 0.02 { 1.0 } else { 0.0 };
        let duration = if target > layer.weight { 0.27 } else { 0.20 };
        layer.weight += (target - layer.weight) * (dt / duration).clamp(0.0, 1.0);
        if layer.weight > 0.001 {
            layer.time += dt * speed.max(0.1);
        }
    }
}

fn sampled_bone_delta(state: &State, clip: &Clip, bone: &str, time: f32, weight: f32) -> Mat4 {
    let Some(base) = state.source.as_ref().and_then(|resources| resources.bones.get(bone)) else {
        return Mat4::IDENTITY;
    };
    let local_time = clip.local_time(time);
    let sample = Pose::default().blend(clip.sample(bone, local_time, local_time), weight.clamp(0.0, 1.0));
    let mut rest = base.matrix();
    rest.w_axis = (base.position / 16.0).extend(1.0);
    let posed = if clip.effect_active("use_keyframes_v2", local_time) {
        let rotation = base.rotation + sample.euler;
        Mat4::from_scale_rotation_translation(
            sample.scale,
            Quat::from_euler(EulerRot::ZYX, rotation.z.to_radians(), rotation.y.to_radians(), rotation.x.to_radians()),
            (base.position + sample.position) / 16.0,
        )
    } else {
        rest * sample.matrix()
    };
    posed * rest.inverse()
}

fn clip_arm_delta(state: &State, clip: &Clip, left: bool, time: f32, weight: f32) -> Mat4 {
    let side = if left { "left" } else { "right" };
    let flip = Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0));
    flip * sampled_bone_delta(state, clip, "camera", time, weight)
        * sampled_bone_delta(state, clip, "body", time, weight)
        * sampled_bone_delta(state, clip, &format!("{side}_arm"), time, weight)
        * flip
}

fn locomotion_clip(sample: punchy_engine::hand_motion::Sample) -> (&'static [&'static str], f32, f32) {
    use punchy_engine::hand_motion::Mode;
    match sample.mode {
        Mode::WalkForward => (&["punchy_moves_walk_forward", "punchy_moves_walk"], sample.phase, sample.walk_weight),
        Mode::WalkBackward => (&["punchy_moves_walk_backward", "punchy_moves_walk"], sample.phase, sample.walk_weight),
        Mode::WalkLeft => (&["punchy_moves_walk_left", "punchy_moves_walk"], sample.phase, sample.walk_weight),
        Mode::WalkRight => (&["punchy_moves_walk_right", "punchy_moves_walk"], sample.phase, sample.walk_weight),
        Mode::SprintForward => (&["punchy_moves_sprint_forward", "punchy_moves_sprint"], sample.phase, sample.sprint_weight),
        Mode::SprintLeft => (&["punchy_moves_sprint_left", "punchy_moves_sprint"], sample.phase, sample.sprint_weight),
        Mode::SprintRight => (&["punchy_moves_sprint_right", "punchy_moves_sprint"], sample.phase, sample.sprint_weight),
        Mode::Jump => (&["punchy_moves_jump"], sample.air_time, sample.air_weight),
        Mode::Fall => (&["punchy_moves_fall"], sample.air_time, sample.fall_weight.max(0.45)),
        Mode::Land => (&["punchy_moves_land"], sample.phase, sample.air_weight.max(0.45)),
        Mode::Air => (&[], sample.phase, 0.0),
        Mode::Climb => (&["climb2", "climb"], sample.phase, sample.special_weight),
        Mode::Crawl => (&["crawl"], sample.phase, sample.special_weight),
        Mode::Ride => (&["newhorse_riding_loop"], sample.phase, sample.special_weight),
        Mode::RideIdle => (&["newhorse_riding_idle"], sample.phase, sample.special_weight),
        Mode::Rowing => (&[], sample.phase, sample.special_weight),
        Mode::Flight => (&["punchy_moves_fall_flying", "punchy_moves_fall"], sample.air_time, sample.flight_weight),
        Mode::Swim => (&["punchy_moves_swim", "swiming", "swiming_above"], sample.phase, 0.45),
        Mode::Idle => (&["punchy_moves_idle"], sample.phase, sample.idle_weight * 0.82),
    }
}

fn locomotion_delta(state: &State, left: bool, alpha: f32, action_weight: f32) -> (Mat4, f32) {
    let sample = state.hand_motion.sample(alpha);
    let (candidates, time, weight) = locomotion_clip(sample);
    if weight <= 0.001 { return (Mat4::IDENTITY, 0.0); }
    let Some(clip) = candidates.iter().find_map(|name| state.clips.get(*name)) else {
        return (Mat4::IDENTITY, 0.0);
    };
    let side = if left { "left_arm" } else { "right_arm" };
    if !clip.tracks.contains_key(side) && !clip.tracks.contains_key("camera") && !clip.tracks.contains_key("body") {
        return (Mat4::IDENTITY, 0.0);
    }
    let effective_weight = weight * (1.0 - action_weight.clamp(0.0, 1.0) * 0.92);
    (clip_arm_delta(state, clip, left, time, effective_weight), effective_weight)
}

fn idle_delta(state: &State, left: bool, alpha: f32, action_weight: f32) -> Mat4 {
    let layer = &state.idle_layers[usize::from(left)];
    let Some(name) = layer.clip.as_deref() else { return Mat4::IDENTITY };
    let Some(clip) = state.clips.get(name) else { return Mat4::IDENTITY };
    let t = alpha.clamp(0.0, 1.0);
    let time = layer.previous_time + (layer.time - layer.previous_time) * t;
    let weight = (layer.previous_weight + (layer.weight - layer.previous_weight) * t)
        * (1.0 - action_weight.clamp(0.0, 1.0));
    clip_arm_delta(state, clip, left, time, weight)
}

fn motion_delta(state: &State, left: bool, alpha: f32) -> Mat4 {
    let action_weight = action_owns_arm(state, left);
    let (locomotion, coverage) = locomotion_delta(state, left, alpha, action_weight);
    state.hand_motion.matrix_with_influence(left, alpha, action_weight, 1.0 - coverage.clamp(0.0, 1.0) * 0.92)
        * locomotion
        * idle_delta(state, left, alpha, action_weight)
}

pub(super) fn tick(game: &mut Game<'_>) {
    if !game.native_window() {
        return;
    }
    if !enabled(game) || game.player.is_none() {
        STATE.with(|s| {
            let mut s = s.borrow_mut();
            s.active = None;
            s.hand_motion = Default::default();
            s.pendulum_states = Default::default();
            s.pendulum_items = Default::default();
            s.pendulum_last_frame = None;
            s.pendulum_camera = None;
            s.pendulum_offsets = [0.0; 2];
            s.pendulum_action_values = [0.0; 2];
            s.pendulum_action_velocities = [0.0; 2];
            s.pendulum_action_kicks = [0.0; 2];
            s.idle_layers = Default::default();
            s.motion_blocked = false;
            s.machine = Default::default();
            s.consume = Default::default();
            s.mining = Default::default();
            s.forced_stack = None;
            s.successful_use.clear();
            s.mining_attempt = false;
            s.previous_swings = game.swings_started;
        });
        return;
    }
    let using = game.used_hand();
    let items = [game.held().cloned(), game.offhand.clone()];
    let key_use_down = game.native_mapping("key.use").is_some_and(|m| m.0);
    let use_down = key_use_down || using.is_some();
    let attack_down = attack_held(
        game.native_mapping("key.attack").is_some_and(|m| m.0),
        game.pad.holds_action(&game.settings.controller, "key.attack"),
        game.pad.state.triggers[1],
        game.settings.controller.trigger_threshold,
    );
    let speed = global_speed(game);
    STATE.with(|cell| {
        let mut s=cell.borrow_mut();load(game, &mut s);
        if s.source.is_none() {return;}
        s.main_left=game.settings.main_hand_left;
        s.offhand_item=items[1].is_some();
        tick_pendulum_feedback(&mut s, game);
        if let Some(player) = game.player.as_ref() {
            let velocity = player.velocity;
            s.hand_motion.tick(punchy_engine::hand_motion::Input {
                dt: 0.05,
                forward: player.applied_input[1],
                strafe: player.applied_input[0],
                horizontal_speed: ((velocity.x * velocity.x + velocity.z * velocity.z) as f32).sqrt(),
                velocity_x: velocity.x as f32,
                velocity_z: velocity.z as f32,
                vertical_speed: velocity.y as f32,
                fall_distance: player.fall_distance as f32,
                yaw: game.yaw,
                pitch: game.pitch,
                grounded: player.ground.on_ground,
                sprinting: player.sprinting,
                sneaking: player.shift_key_down || player.local_crouching,
                swimming: player.swimming,
                fall_flying: player.fall_flying,
                flying: player.flying,
                spectator: player.spectator,
                passenger: player.is_passenger,
                sleeping: player.sleeping,
                climbing: player.last_climbable.is_some() && (player.jumping || player.shift_key_down || player.applied_input[1].abs() > 0.05),
                crawling: player.pose == horizon_client::pose::Pose::Swimming && !player.swimming,
                riding_horse: game.riding.and_then(|id| game.mobs.iter().find(|mob| mob.id == id)).is_some_and(|mob| horizon_client::mobs::equine::is_equine(mob.species())),
                rowing: game.riding.and_then(|id| game.boats.iter().find(|boat| boat.id == id)).is_some_and(|boat| boat.paddling.iter().any(|paddling| *paddling)),
            });
        }
        let mut input=action::Input { animation_speed:speed, use_down,
            mining:game.mining.is_some(),
            entity_target:game.pick_entity_hit().flatten().is_some(),
            block_target:game.pick().is_some(),
            inspect:std::mem::take(&mut s.inspect),
            ..Default::default() };
        let changed=game.swings_started!=s.previous_swings;
        s.previous_swings=game.swings_started;
        let mining_attempt=std::mem::take(&mut s.mining_attempt);
        input.attack=(changed && !input.mining && !mining_attempt).then_some(hand(game.swing.attack_hand()));
        input.mining_trigger=mining_attempt || changed && input.mining;
        let use_hand=hand(using.unwrap_or(Hand::Main));
        let was_use_down=s.use_down;
        s.use_down=use_down;
        for (i,stack) in items.iter().enumerate() {
            input.hands[i].kind=stack.as_ref().map_or(action::ItemKind::Empty,|v|action::ItemKind::from_id(&v.item));
            input.hands[i].crossbow_charged=stack.as_ref().and_then(|v|v.data()).is_some_and(|d|!d.charged.is_empty());
            let id=stack.as_ref().map_or("",|v|v.item.as_ref()).to_owned();
            s.item_kinds[i] = stack.as_ref().and_then(|stack| {
                game.items()
                    .get(&stack.item)
                    .filter(|info| info.block.is_some())
                    .map(|_| "FULL_BLOCK".to_owned())
            });
            if id!=s.last_items[i] {
                input.hotbar_changed|=i==0;
                input.equip=Some(action::EquipEvent {hand:if i==0 {action::Hand::Main}else{action::Hand::Off},entering:!id.is_empty()});
                s.last_items[i]=id;
            }
        }
        s.motion_blocked = input.mining || use_down || game.player.as_ref().is_some_and(|player| player.swimming || player.fall_flying || player.flying || player.sleeping || player.is_passenger || player.spectator || player.last_climbable.is_some() || player.pose == horizon_client::pose::Pose::Swimming && !player.swimming);
        let motion_blocked = s.motion_blocked;
        update_idle_layers(&mut s, 0.05, speed, motion_blocked);
        let input_driven=tuning_animation(&s,use_hand,"useItem").is_some_and(|e|e.trigger_on_input);
        input.use_trigger=s.successful_use.pop_front().or_else(|| (use_down && !was_use_down && input_driven).then_some(use_hand));
        if let Some(h)=using {
            let i=usize::from(h==Hand::Off);
            if let Some(stack)=items[i].as_ref() {
                let owner=game.player_owner(Some(stack));
                let kinetic=horizon_client::spear::Kinetic::of(&stack.item);
                let mut kind=if kinetic.is_some() {
                    action::ItemKind::Spear
                } else {
                    input.hands[i].kind
                };
                if kind==action::ItemKind::Other {
                    let info=game.items().get(&stack.item);
                    let mut animation=info.and_then(|info| info.consumable.as_ref())
                        .map_or(horizon_world::consumable::UseAnimation::None,|consumable| consumable.animation);
                    game.native_use_animation(stack,&mut animation);
                    kind=match animation {
                        horizon_world::consumable::UseAnimation::Eat=>action::ItemKind::Eat,
                        horizon_world::consumable::UseAnimation::Drink=>action::ItemKind::Drink,
                        _ if info.is_some_and(|info| info.food.is_some())=>action::ItemKind::Eat,
                        _=>kind
                    };
                }
                let elapsed_ticks=owner.use_duration-owner.use_item_remaining_ticks-1;
                let (spear_stage,spear_maxed)=kinetic.map_or((1,false),|k| action::spear_charge_stage(
                    elapsed_ticks,k.delay_ticks,k.dismount.max_duration_ticks,
                    k.knockback.max_duration_ticks,k.damage.max_duration_ticks));
                input.using=Some(action::UseState {hand:hand(h),kind,
                    elapsed_ticks,
                    charge_duration_ticks:owner.crossbow_charge_duration as i32,
                    spear_stage,spear_maxed,
                    riptide:stack.enchantment_level("minecraft:riptide")>0,
                    loyalty:stack.enchantment_level("minecraft:loyalty")>0,
                    ..Default::default()});
            }
        }
        if s.tuning.is_some() {
            let h=input.attack.unwrap_or(action::Hand::Main);
            if let Some(entry)=tuning_animation(&s,h,"attack") {
                input.attack_options.custom_entry_present=true;
                input.attack_options.combo=if entry.variations.is_empty(){entry.name.into_iter().collect()}else{entry.variations};
                input.attack_options.critical_clip=entry.critical;
                input.attack_options.trigger_on_input=entry.trigger_on_input;
                input.attack_options.skip_blend_in=entry.skip_blend_in;
            }
        }
        input.has_bow_projectile_or_creative=game.creative || game.projectile_slot("minecraft:bow").is_some();
        input.bow_shot_configured=s.tuning.as_ref().is_some_and(|t|s.last_items.iter().enumerate().any(|(i,id)|t.animation(id,"use_bow_shot",s.main_left!=(i==1)).is_some()));
        if let Some(p)=&s.active {
            input.playback_active=!p.stopping;
            input.playback_clip=Some(p.clip.name.clone());input.playback_hand=Some(p.hand);
        }

        let phase_request = phase_tick(&mut s, &mut input, &items, speed, attack_down);

        // Split ownership to let the catalog borrow clip data during arbitration.
        let mut machine=std::mem::take(&mut s.machine);
        let intent=machine.tick(&input,&Catalog{state:&s});s.machine=machine;
        let core_won = intent.is_some();
        if let Some(intent)=intent {
            match intent.operation {
                action::Operation::Play(p) => {
                    if let Some(clip)=s.clips.get(&p.clip.name).cloned() {

                        s.status=format!("Playing {}",clip.name);
                        s.playback_id=s.playback_id.wrapping_add(1);
                        let from=s.active.take().map(Box::new);
                        let play_hand = p.hand;
                        s.active=Some(Playback{from,out_weight:1.,previous_out_weight:1.,id:s.playback_id,expression:p.start_time,previous_expression:p.start_time,return_fade:0.28,skip_tick:false,clip,hand:play_hand,previous:p.start_time,time:p.start_time,
                            multiplier:p.speed_multiplier,looping:p.looping,released:false,hold:p.hold_on_last_frame,
                            fade:p.blend_seconds.unwrap_or(0.2).max(0.001),stopping:false,
                            weight:if p.skip_blend_in {1.0}else{0.0},previous_weight:if p.skip_blend_in {1.}else{0.}});
                        trigger_pendulum_action(&mut s, play_hand);
                    }
                }
                action::Operation::Stop{hand,mode} => {
                    if let Some(p)=s.active.as_mut().filter(|p|p.hand==hand) {
                        p.looping=false;p.released=true;p.hold=false;
                        if let action::StopMode::BlendOut(seconds)=mode {p.stopping=true;p.return_fade=seconds.max(0.001);}
                    }
                }
            }
        }
        if !core_won {
            if let Some((consume, request)) = phase_request { apply_phase(&mut s, consume, &request, speed); }
        }
        if let Some(p)=s.active.as_mut() {
            p.advance(speed);
            if p.stopping && p.out_weight<=0. {s.active=None;}
        }
        if let Some(path)=std::env::var_os("PUNCHY_DIAGNOSTICS") {
            let value=serde_json::json!({"loaded":s.source.is_some(),"resource_source":s.resource_source,"clips":s.clips.len(),"error":s.error,
                "active":s.active.as_ref().map(|p|serde_json::json!({"name":p.clip.name,"time":p.time,"weight":p.weight,"speed":punchy_engine::playback::rate(&p.clip,p.time,speed,p.multiplier,1.)})),
                "swings":s.previous_swings,"mining":input.mining,"consume_phase":format!("{:?}",s.consume.phase()),"mining_phase":format!("{:?}",s.mining.phase())});
            if let Ok(bytes)=serde_json::to_vec_pretty(&value) {let _=std::fs::write(path,bytes);}
        }
    });
}

pub(super) fn key(game: &mut Game<'_>, key: &str, pressed: bool) {
    if !pressed || !game.native_playing() || !enabled(game) {
        return;
    }
    if key == game.native_mod_key(ID, "inspect", "key.keyboard.f6") {
        STATE.with(|s| s.borrow_mut().inspect = true);
    }
    if key == game.native_mod_key(ID, "reload", "key.keyboard.f7") {
        STATE.with(|s| *s.borrow_mut() = State::default());
    }
}
pub(super) fn settings(game: &Game<'_>, buttons: &mut Vec<String>) {
    buttons.push(format!("Animation speed: {:.1}", global_speed(game)));
    buttons.push("Reload animations and resource packs".into());
    buttons.push(format!("Hand poses: {}", if game.native_setting(ID,"hand_poses").as_deref()==Some("horizon") {"Horizon"} else {"Punchy"}));
    STATE.with(|cell| {
        let mut state = cell.borrow_mut();
        load(game, &mut state);
        state.editor = HandEditor::parse(game.native_setting(ID,"hand_editor.json"));
        let hand = state.editor.selected_hand;
        let item = held_item_id(game, hand);
        let left = editor_left(game, hand);
        let choices = selector_choices(game, &state, &item);
        if !choices.iter().any(|choice| choice == &state.editor.selected_selector) {
            state.editor.selected_selector = choices.first().cloned().unwrap_or_default();
        }
        if state.editor.selected_target == "line" && item != "minecraft:fishing_rod" {
            state.editor.selected_target = "item".into();
            state.editor.selected_scope = "item".into();
            state.editor.selected_selector = item.clone();
        }
        if state.editor.selected_target == "line" && state.editor.selected_scope != "item" {
            state.editor.selected_scope = "item".into();
            state.editor.selected_selector = item.clone();
        }
        let target = state.editor.selected_target.clone();
        let scope = state.editor.selected_scope.clone();
        let selector = if scope == "item" { item.as_str() } else { state.editor.selected_selector.as_str() };
        let transform = editor_scope_transform(&state, &item, left, &target, &scope, selector);
        buttons.push(format!("Hand · {}", if hand == 0 { "Main hand" } else { "Off hand" }));
        buttons.push(format!("Edit · {}", match target.as_str() { "arm" => "Arm", "line" => "Line", _ => "Item" }));
        buttons.push(format!("Applies to · {}", match state.editor.selected_scope.as_str() { "tag" => "Tag", "group" => "Group", "all" => "All", _ => "Item" }));
        buttons.push("Previous selector".into());
        buttons.push("Next selector".into());
        buttons.push(format!("Offset X − · {:.3}", transform.offset[0]));
        buttons.push(format!("Offset X + · {:.3}", transform.offset[0]));
        buttons.push(format!("Offset Y − · {:.3}", transform.offset[1]));
        buttons.push(format!("Offset Y + · {:.3}", transform.offset[1]));
        buttons.push(format!("Offset Z − · {:.3}", transform.offset[2]));
        buttons.push(format!("Offset Z + · {:.3}", transform.offset[2]));
        buttons.push(format!("Rotation X − · {:.1}°", transform.rotation[0]));
        buttons.push(format!("Rotation X + · {:.1}°", transform.rotation[0]));
        buttons.push(format!("Rotation Y − · {:.1}°", transform.rotation[1]));
        buttons.push(format!("Rotation Y + · {:.1}°", transform.rotation[1]));
        buttons.push(format!("Rotation Z − · {:.1}°", transform.rotation[2]));
        buttons.push(format!("Rotation Z + · {:.1}°", transform.rotation[2]));
        buttons.push(format!("Scale − · {:.2}", transform.scale));
        buttons.push(format!("Scale + · {:.2}", transform.scale));
        buttons.push(format!("Pivot X − · {:.3}", transform.pivot[0]));
        buttons.push(format!("Pivot X + · {:.3}", transform.pivot[0]));
        buttons.push(format!("Pivot Y − · {:.3}", transform.pivot[1]));
        buttons.push(format!("Pivot Y + · {:.3}", transform.pivot[1]));
        buttons.push(format!("Pivot Z − · {:.3}", transform.pivot[2]));
        buttons.push(format!("Pivot Z + · {:.3}", transform.pivot[2]));
        buttons.push("Mirror selected pose to other hand".into());
        buttons.push("Reset selected pose".into());
        let profile_name = state.editor.profiles.get(state.editor.profile_index).and_then(|v|v.get("name")).and_then(|v|v.as_str()).unwrap_or("none");
        buttons.push(format!("Save new Hand Editor profile · current: {profile_name}"));
        buttons.push("Load next Hand Editor profile".into());
        buttons.push("Delete selected Hand Editor profile".into());
        buttons.push(format!("Undo · {} available", state.editor.undo.len()));
        buttons.push(format!("Redo · {} available", state.editor.redo.len()));
        buttons.push("Create group / add held item to selected group".into());
        buttons.push("Delete selected custom group".into());
        buttons.push("Export Hand Editor data to hand_editor_export.json".into());
        buttons.push("Import Hand Editor data from hand_editor_import.json".into());
        buttons.push(format!("Resource source: {}", state.resource_source));
        buttons.push(state.error.clone().unwrap_or_else(||state.status.clone()));
        buttons.push(format!("Held item: {item}"));
        buttons.push(format!("Selected {scope}: {}", if selector.is_empty() { "none" } else { selector }));
        buttons.push(format!("Profile: {profile_name}"));
        if scope == "tag" || scope == "group" {
            for choice in choices {
                let marker = if choice == state.editor.selected_selector { "●" } else { "○" };
                buttons.push(format!("{marker} {choice}"));
            }
        }
    });
}

pub(super) fn click(game: &Game<'_>, button: usize) {
    match button {
        0 => {
            let speed = global_speed(game);
            game.native_set_setting(ID,"speed",&if speed >= 5.5 {"0.5".into()} else {format!("{:.1}",speed+0.5)});
        }
        1 => resources_reloaded(),
        2 => game.native_set_setting(ID,"hand_poses",if game.native_setting(ID,"hand_poses").as_deref()==Some("horizon"){"punchy"}else{"horizon"}),
        _ => STATE.with(|cell| {
            let mut state = cell.borrow_mut();
            load(game,&mut state);
            state.editor = HandEditor::parse(game.native_setting(ID,"hand_editor.json"));
            state.last_slider = None;
            let hand = state.editor.selected_hand;
            let item = held_item_id(game,hand);
            let left = editor_left(game,hand);
            let mut choices = selector_choices(game,&state,&item);
            if !choices.iter().any(|choice|choice==&state.editor.selected_selector) {
                state.editor.selected_selector = choices.first().cloned().unwrap_or_default();
            }
            match button {
                3 => { state.editor.selected_hand = 1-state.editor.selected_hand; }
                4 => {
                    state.editor.remember();
                    let next = match state.editor.selected_target.as_str() {
                        "item" => "arm",
                        "arm" if item == "minecraft:fishing_rod" => "line",
                        _ => "item",
                    };
                    state.editor.selected_target = next.into();
                    if next == "line" {
                        state.editor.selected_scope = "item".into();
                        state.editor.selected_selector = item.clone();
                    }
                }
                5 => {
                    if state.editor.selected_target == "line" { return; }
                    state.editor.remember();
                    state.editor.selected_scope = match state.editor.selected_scope.as_str() {
                        "item" => "tag", "tag" => "group", "group" => "all", _ => "item",
                    }.into();
                    choices = selector_choices(game,&state,&item);
                    state.editor.selected_selector = choices.first().cloned().unwrap_or_default();
                }
                6 | 7 => {
                    if !choices.is_empty() {
                        let current = choices.iter().position(|v|v==&state.editor.selected_selector).unwrap_or(0);
                        let next = if button==6 { (current+choices.len()-1)%choices.len() } else { (current+1)%choices.len() };
                        state.editor.selected_selector = choices[next].clone();
                    }
                }
                8..=27 => {
                    let target = state.editor.selected_target.clone();
                    let scope = state.editor.selected_scope.clone();
                    let selector = if scope=="item" { item.clone() } else { state.editor.selected_selector.clone() };
                    if scope=="tag" && selector.is_empty() { return; }
                    if scope=="group" && selector.is_empty() { return; }
                    state.editor.remember();
                    let transform = editor_scope_transform(&state, &item, left, &target, &scope, &selector);
                    let (key,delta) = match button {
                        8 => ("offX",-0.005), 9 => ("offX",0.005),
                        10 => ("offY",-0.005), 11 => ("offY",0.005),
                        12 => ("offZ",-0.005), 13 => ("offZ",0.005),
                        14 => ("rotX",-0.5), 15 => ("rotX",0.5),
                        16 => ("rotY",-0.5), 17 => ("rotY",0.5),
                        18 => ("rotZ",-0.5), 19 => ("rotZ",0.5),
                        20 => ("scale",-0.01), 21 => ("scale",0.01),
                        22 => ("pivotX",-0.005), 23 => ("pivotX",0.005),
                        24 => ("pivotY",-0.005), 25 => ("pivotY",0.005),
                        26 => ("pivotZ",-0.005), _ => ("pivotZ",0.005),
                    };
                    let (minimum,maximum) = if target=="line" {
                        match key { "scale"=>(0.01,10.0), "rotX"|"rotY"|"rotZ"=>(-720.0,720.0), _=>(-32.0,32.0) }
                    } else {
                        match key { "scale"=>(0.1,3.0), "rotX"|"rotY"|"rotZ"=>(-180.0,180.0), "pivotX"|"pivotY"|"pivotZ"=>(-1.0,1.0), _=>(-2.0,2.0) }
                    };
                    let next=(transform_component(&transform,key)+delta).clamp(minimum,maximum);
                    let values=state.editor.transform_mut(&item,left,&target,&scope,&selector);
                    values.as_object_mut().expect("editor transform object").insert(key.into(),serde_json::json!(next));
                }
                28 => {
                    let target=state.editor.selected_target.clone();
                    let scope=state.editor.selected_scope.clone();
                    let selector=if scope=="item" {item.clone()} else {state.editor.selected_selector.clone()};
                    state.editor.remember();
                    let mut mirror=editor_scope_transform(&state,&item,left,&target,&scope,&selector);
                    mirror.offset[0] = -mirror.offset[0];
                    mirror.rotation[1] = -mirror.rotation[1];
                    mirror.rotation[2] = -mirror.rotation[2];
                    mirror.pivot[0] = -mirror.pivot[0];
                    *state.editor.transform_mut(&item,!left,&target,&scope,&selector)=transform_value(&mirror);
                }
                29 => {
                    let target=state.editor.selected_target.clone();
                    let scope=state.editor.selected_scope.clone();
                    let selector=if scope=="item" {item.clone()} else {state.editor.selected_selector.clone()};
                    state.editor.remember();
                    if scope=="item" {
                        let side=if left{"left"}else{"right"};
                        if let Some(entry)=state.editor.items.get_mut(&item).and_then(serde_json::Value::as_object_mut) {
                            if let Some(side_entry)=entry.get_mut(side).and_then(serde_json::Value::as_object_mut) {
                                side_entry.remove(&target);
                            }
                        }
                    } else {
                        state.editor.rules.retain(|rule| !(rule.get("scope").and_then(|v|v.as_str())==Some(scope.as_str())
                            && rule.get("selector").and_then(|v|v.as_str())==Some(selector.as_str())
                            && rule.get("side").and_then(|v|v.as_str())==Some(if left{"left"}else{"right"})
                            && rule.get("target").and_then(|v|v.as_str())==Some(target.as_str())));
                    }
                }
                30 => {
                    state.editor.remember();
                    let number=state.editor.profiles.len()+1;
                    let payload=state.editor.snapshot();
                    state.editor.profiles.push(serde_json::json!({"name":format!("Android Profile {number}"),"payload":payload}));
                    state.editor.profile_index=state.editor.profiles.len()-1;
                }
                31 => {
                    if !state.editor.profiles.is_empty() {
                        state.editor.remember();
                        state.editor.profile_index=(state.editor.profile_index+1)%state.editor.profiles.len();
                        let payload=state.editor.profiles[state.editor.profile_index].get("payload").cloned().unwrap_or(serde_json::Value::Null);
                        state.editor.restore(&payload);
                    }
                }
                32 => {
                    if !state.editor.profiles.is_empty() {
                        let index=state.editor.profile_index.min(state.editor.profiles.len()-1);
                        state.editor.profiles.remove(index);
                        state.editor.profile_index=state.editor.profile_index.min(state.editor.profiles.len().saturating_sub(1));
                    }
                }
                33 => {
                    if let Some(previous)=state.editor.undo.pop() {
                        let current=state.editor.snapshot();
                        if state.editor.redo.len()>=24 {state.editor.redo.remove(0);}
                        state.editor.redo.push(current);
                        state.editor.restore(&previous);
                    }
                }
                34 => {
                    if let Some(next)=state.editor.redo.pop() {
                        let current=state.editor.snapshot();
                        if state.editor.undo.len()>=24 {state.editor.undo.remove(0);}
                        state.editor.undo.push(current);
                        state.editor.restore(&next);
                    }
                }
                35 => {
                    state.editor.remember();
                    let mut selector=state.editor.selected_selector.clone();
                    if state.editor.selected_scope!="group" || selector.is_empty() {
                        let count=state.editor.groups.as_object().map_or(0,|v|v.len());
                        selector=format!("android_group_{}",count+1);
                    }
                    let initial=selector_members(&state,&selector);
                    let groups=state.editor.groups.as_object_mut().expect("custom groups object");
                    let members=groups.entry(selector.clone()).or_insert_with(||serde_json::json!(initial)).as_array_mut().expect("group members array");
                    if !members.iter().any(|v|v.as_str()==Some(item.as_str())) {members.push(serde_json::json!(item));}
                    state.editor.selected_scope="group".into();
                    state.editor.selected_selector=selector;
                }
                36 => {
                    if state.editor.selected_scope=="group" {
                        state.editor.remember();
                        let selected_group = state.editor.selected_selector.clone();
                        state.editor.groups.as_object_mut().expect("custom groups object").remove(&selected_group);
                        state.editor.rules.retain(|rule| !(rule.get("scope").and_then(|v|v.as_str())==Some("group") && rule.get("selector").and_then(|v|v.as_str())==Some(selected_group.as_str())));
                        state.editor.selected_selector.clear();
                    }
                }
                37 => {
                    let payload = state.editor.snapshot();
                    match serde_json::to_string_pretty(&payload) {
                        Ok(raw) => {
                            game.native_set_setting(ID, "hand_editor_export.json", &raw);
                            state.status = "Hand Editor data exported to hand_editor_export.json".into();
                        }
                        Err(error) => state.status = format!("Hand Editor export failed: {error}"),
                    }
                }
                38 => {
                    match game.native_setting(ID, "hand_editor_import.json") {
                        Some(raw) => match serde_json::from_str::<serde_json::Value>(&raw) {
                            Ok(payload) if payload.get("items").is_some() || payload.get("rules").is_some() || payload.get("groups").is_some() => {
                                state.editor.remember();
                                state.editor.restore(&payload);
                                let number = state.editor.profiles.len() + 1;
                                state.editor.profiles.push(serde_json::json!({"name":format!("Imported Profile {number}"),"payload":payload}));
                                state.editor.profile_index = state.editor.profiles.len() - 1;
                                state.status = "Imported Hand Editor data".into();
                            }
                            Ok(_) => state.status = "Invalid Hand Editor import: missing items, rules or groups".into(),
                            Err(error) => state.status = format!("Invalid Hand Editor import JSON: {error}"),
                        },
                        None => state.status = "Place hand_editor_import.json in the Punchy data folder first".into(),
                    }
                }
                44.. => {
                    if let Some(choice) = choices.get(button - 44) {
                        state.editor.selected_selector = choice.clone();
                    }
                }
                _ => {}
            }
            state.editor.save(game);
        }),
    }
}

#[derive(Clone, Copy)]
struct EditorField {
    key: &'static str,
    label: &'static str,
    section: &'static str,
    minimum: f32,
    maximum: f32,
    step: f32,
    precision: usize,
}

fn editor_fields(target: &str, scope: &str) -> Vec<EditorField> {
    let position = [
        EditorField { key: "offX", label: "Offset X", section: "Position", minimum: -2.0, maximum: 2.0, step: 0.005, precision: 3 },
        EditorField { key: "offY", label: "Offset Y", section: "Position", minimum: -2.0, maximum: 2.0, step: 0.005, precision: 3 },
        EditorField { key: "offZ", label: "Offset Z", section: "Position", minimum: -2.0, maximum: 2.0, step: 0.005, precision: 3 },
    ];
    let rotation = [
        EditorField { key: "rotX", label: "Rotation X", section: "Rotation", minimum: -180.0, maximum: 180.0, step: 0.5, precision: 1 },
        EditorField { key: "rotY", label: "Rotation Y", section: "Rotation", minimum: -180.0, maximum: 180.0, step: 0.5, precision: 1 },
        EditorField { key: "rotZ", label: "Rotation Z", section: "Rotation", minimum: -180.0, maximum: 180.0, step: 0.5, precision: 1 },
    ];
    let scale = EditorField { key: "scale", label: "Scale", section: "Size", minimum: 0.1, maximum: 3.0, step: 0.01, precision: 2 };
    let pivot = [
        EditorField { key: "pivotX", label: "Pivot X", section: "Pivot", minimum: -1.0, maximum: 1.0, step: 0.005, precision: 3 },
        EditorField { key: "pivotY", label: "Pivot Y", section: "Pivot", minimum: -1.0, maximum: 1.0, step: 0.005, precision: 3 },
        EditorField { key: "pivotZ", label: "Pivot Z", section: "Pivot", minimum: -1.0, maximum: 1.0, step: 0.005, precision: 3 },
    ];
    if target == "line" {
        let position = position.map(|field| EditorField { minimum: -32.0, maximum: 32.0, step: 0.05, precision: 2, ..field });
        let rotation = rotation.map(|field| EditorField { minimum: -720.0, maximum: 720.0, step: 1.0, precision: 0, ..field });
        let scale = EditorField { minimum: 0.01, maximum: 10.0, step: 0.01, precision: 2, ..scale };
        return position.into_iter().chain(rotation).chain([scale]).collect();
    }
    if target == "arm" {
        let mut fields = position.into_iter().collect::<Vec<_>>();
        if scope == "all" {
            fields.extend(rotation);
        }
        fields.push(scale);
        return fields;
    }
    position.into_iter().chain(rotation).chain([scale]).chain(pivot).collect()
}

pub(super) fn sliders(game: &Game<'_>, sliders: &mut Vec<horizon_client_render::mod_list::NativeSlider>) {
    STATE.with(|cell| {
        let mut state = cell.borrow_mut();
        load(game, &mut state);
        state.editor = HandEditor::parse(game.native_setting(ID, "hand_editor.json"));
        let hand = state.editor.selected_hand;
        let item = held_item_id(game, hand);
        let target = state.editor.selected_target.clone();
        let scope = state.editor.selected_scope.clone();
        if target == "line" && (item != "minecraft:fishing_rod" || scope != "item") {
            return;
        }
        let selector = if scope == "item" { item.clone() } else { state.editor.selected_selector.clone() };
        if (scope == "tag" || scope == "group") && selector.is_empty() {
            return;
        }
        let left = editor_left(game, hand);
        let transform = editor_scope_transform(&state, &item, left, &target, &scope, &selector);
        for field in editor_fields(&target, &scope) {
            let value = transform_component(&transform, field.key).clamp(field.minimum, field.maximum);
            let normalized = (value - field.minimum) / (field.maximum - field.minimum);
            sliders.push(horizon_client_render::mod_list::NativeSlider {
                section: field.section.into(),
                label: format!("{}  {}", field.label, format!("{:.*}", field.precision, value)),
                value: normalized.clamp(0.0, 1.0),
            });
        }
    });
}

pub(super) fn slider(game: &Game<'_>, slider: usize, normalized: f32, finished: bool) {
    STATE.with(|cell| {
        let mut state = cell.borrow_mut();
        load(game, &mut state);
        state.editor = HandEditor::parse(game.native_setting(ID, "hand_editor.json"));
        let hand = state.editor.selected_hand;
        let item = held_item_id(game, hand);
        let left = editor_left(game, hand);
        let target = state.editor.selected_target.clone();
        let scope = state.editor.selected_scope.clone();
        if target == "line" && (item != "minecraft:fishing_rod" || scope != "item") {
            if finished { state.last_slider = None; }
            return;
        }
        let fields = editor_fields(&target, &scope);
        let Some(field) = fields.get(slider).copied() else {
            if finished { state.last_slider = None; }
            return;
        };
        let selector = if scope == "item" { item.clone() } else { state.editor.selected_selector.clone() };
        if (scope == "tag" || scope == "group") && selector.is_empty() {
            if finished { state.last_slider = None; }
            return;
        }
        let current_transform = editor_scope_transform(&state, &item, left, &target, &scope, &selector);
        let current = transform_component(&current_transform, field.key);
        let raw = field.minimum + normalized.clamp(0.0, 1.0) * (field.maximum - field.minimum);
        let next = (field.minimum + ((raw - field.minimum) / field.step).round() * field.step)
            .clamp(field.minimum, field.maximum);
        if (next - current).abs() > 0.00001 {
            let context = format!("{item}|{hand}|{target}|{scope}|{selector}|{}", field.key);
            if state.last_slider.as_deref() != Some(context.as_str()) {
                state.editor.remember();
                state.last_slider = Some(context);
            }
            let values = state.editor.transform_mut(&item, left, &target, &scope, &selector);
            values.as_object_mut().expect("editor transform is an object").insert(field.key.into(), serde_json::json!(next));
            state.editor.save(game);
        }
        if finished {
            state.last_slider = None;
        }
    });
}

pub(super) fn fishing_line_transform(game: &Game<'_>, left: bool) -> Option<punchy_engine::tuning::Transform> {
    if !enabled(game) { return None; }
    STATE.with(|cell| {
        let mut state=cell.borrow_mut();
        load(game,&mut state);
        let tags=held_tags(game,"minecraft:fishing_rod");
        let transform=editor_transform(&state,"minecraft:fishing_rod",left,"line",&tags);
        let side=if left{"left"}else{"right"};
        let has_rule=state.editor.rules.iter().any(|rule| rule.get("target").and_then(|v|v.as_str())==Some("line")
            && rule.get("side").and_then(|v|v.as_str())==Some(side));
        let has_item=state.editor.items.get("minecraft:fishing_rod").and_then(|v|v.get(side)).and_then(|v|v.get("line")).is_some();
        (has_rule || has_item).then_some(transform)
    })
}

// Authored skeletal delta calibrated against the same bone's rest transform.
// This preserves Horizon's item renderer and equipment coordinates; the full
// Punchy item mesh/physics/attachment renderer is a separate unfinished port.
fn bone_delta(s: &State, name: &str, alpha: f32) -> Mat4 {
    let Some(p) = &s.active else {
        return Mat4::IDENTITY;
    };
    let Some(base) = s.source.as_ref().and_then(|r| r.bones.get(name)) else {
        return Mat4::IDENTITY;
    };
    let time = if p.time < p.previous {
        p.time
    } else {
        p.previous + (p.time - p.previous) * alpha
    };
    let expression = p.previous_expression + (p.expression - p.previous_expression) * alpha;
    let sample = p.clip.sample(name, time, expression);
    let weight = p.previous_weight + (p.weight - p.previous_weight) * alpha;
    let sample = Pose::default().blend(sample, weight);
    let mut rest = base.matrix();
    rest.w_axis = (base.position / 16.).extend(1.);
    let posed = if p.clip.effect_active("use_keyframes_v2", time) {
        let rot = base.rotation + sample.euler;
        Mat4::from_scale_rotation_translation(
            sample.scale,
            Quat::from_euler(
                EulerRot::ZYX,
                rot.z.to_radians(),
                rot.y.to_radians(),
                rot.x.to_radians(),
            ),
            (base.position + sample.position) / 16.,
        )
    } else {
        rest * sample.matrix()
    };
    posed * rest.inverse()
}
fn hand_delta(s: &State, left: bool, alpha: f32) -> Mat4 {
    let Some(p) = &s.active else {
        return Mat4::IDENTITY;
    };
    let active_left = s.main_left != (p.hand == action::Hand::Off);
    if left != active_left
        && !punchy_engine::policy::dual_handed(&p.clip, p.time, s.offhand_item, s.offhand_item)
    {
        return Mat4::IDENTITY;
    }
    let side = if left { "left" } else { "right" };
    // Minecraft's authored model axes use X/Y opposite to first-person view.
    let flip = Mat4::from_scale(Vec3::new(-1., -1., 1.));
    flip * bone_delta(s, "camera", alpha)
        * bone_delta(s, "body", alpha)
        * bone_delta(s, &format!("{side}_arm"), alpha)
        * flip
}
pub(super) fn arms(game: &mut Game<'_>, alpha: f64, out: &mut Vec<Vertex>) {
    if !enabled(game)
        || game.spectator()
        || game.ui.hide_gui
        || game.sleeping.is_some()
        || !game.camera_type.first_person()
        || game.own_invisible()
        || game
            .player
            .as_ref()
            .is_none_or(|p| p.health.dead_or_dying())
        || game.screen == Screen::Dead
    {
        return;
    }
    let items = rendered_items(game);
    if items.iter().flatten().any(|s| maps::map_id(s).is_some()) {
        return;
    }
    let shown = game.hands_to_render();
    let a = alpha as f32;
    let eye = game.eye_position().unwrap_or_default();
    let light = game.world.light(
        eye[0].floor() as i32,
        eye[1].floor() as i32,
        eye[2].floor() as i32,
    );
    let packed = (u32::from(light >> 4) * 16) | ((u32::from(light & 15) * 16) << 8);
    let world = game.sway.to_world(a, &game.view);
    STATE.with(|cell| {
        let s = cell.borrow();
        if s.source.is_none() {
            return;
        }
        for (i, item) in items.iter().enumerate() {
            let dual = s.active.as_ref().is_some_and(|p| {
                punchy_engine::policy::dual_handed(&p.clip, p.time, s.offhand_item, s.offhand_item)
            });
            let hidden = i == 1
                && s.active.as_ref().is_some_and(|p| {
                    punchy_engine::policy::hide_empty_offhand(&p.clip, p.time, s.offhand_item)
                });
            if hidden {
                continue;
            }
            if (!shown[i] && !dual) || (item.is_none() && (i == 0 || !dual)) {
                continue;
            }
            let left = game.settings.main_hand_left != (i == 1);
            let m = world * arm_pose(left, 0., 0.);
            game.first_person_arm(left, m, packed, out);
        }
    });
}

fn pendulum_frame_input(state: &mut State, game: &Game<'_>, _alpha: f32) -> punchy_engine::pendulum::Input {
    let now = Instant::now();
    let dt = state.pendulum_last_frame.replace(now)
        .map_or(1.0 / 60.0, |previous| now.duration_since(previous).as_secs_f32())
        .clamp(0.001, 1.0 / 30.0);
    let yaw = game.yaw;
    let pitch = game.pitch;
    let (yaw_delta, pitch_delta) = state.pendulum_camera.map_or((0.0, 0.0), |(last_yaw, last_pitch)| {
        (((yaw - last_yaw + 180.0).rem_euclid(360.0) - 180.0), pitch - last_pitch)
    });
    state.pendulum_camera = Some((yaw, pitch));
    let Some(player) = game.player.as_ref() else {
        return punchy_engine::pendulum::Input { dt, yaw, yaw_delta, pitch, pitch_delta, ..Default::default() };
    };
    let velocity = player.velocity;
    let (walk_speed, walk_pos) = game.player_model.as_ref().map_or((0.0, 0.0), |model| {
        (model.limb_speed, model.limb_position)
    });
    punchy_engine::pendulum::Input {
        dt,
        yaw,
        yaw_delta,
        pitch,
        pitch_delta,
        walk_speed,
        walk_pos,
        vel_y: velocity.y as f32,
        on_ground: player.ground.on_ground,
        offset_x: state.pendulum_offsets[0],
        offset_z: state.pendulum_offsets[1],
        ..Default::default()
    }
}

fn trigger_pendulum_action(state: &mut State, hand: action::Hand) {
    let left = state.main_left != (hand == action::Hand::Off);
    let index = usize::from(left);
    state.pendulum_action_velocities[index] += 10.0;
    state.pendulum_action_kicks[index] += 1.0;
}

fn tick_pendulum_feedback(state: &mut State, game: &Game<'_>) {
    let dt = 0.05_f32;
    let boat_passenger = game.riding.is_some_and(|id| game.boats.iter().any(|boat| boat.id == id));
    let (target_x, target_z) = game.player.as_ref().map_or((0.0, 0.0), |player| {
        if boat_passenger {
            (0.0, 0.0)
        } else {
            (player.applied_input[0].clamp(-1.0, 1.0) * 1.5,
             -player.applied_input[1].clamp(-1.0, 1.0) * 1.6)
        }
    });
    let blend = 1.0 - 0.7_f32.powf(dt * 20.0);
    state.pendulum_offsets[0] += (target_x - state.pendulum_offsets[0]) * blend;
    state.pendulum_offsets[1] += (target_z - state.pendulum_offsets[1]) * blend;
    for index in 0..2 {
        let acceleration = -60.0 * state.pendulum_action_values[index]
            - 15.5 * state.pendulum_action_velocities[index];
        state.pendulum_action_velocities[index] += acceleration * dt;
        state.pendulum_action_values[index] += state.pendulum_action_velocities[index] * dt;
        if state.pendulum_action_values[index].abs() < 0.0001
            && state.pendulum_action_velocities[index].abs() < 0.0001
        {
            state.pendulum_action_values[index] = 0.0;
            state.pendulum_action_velocities[index] = 0.0;
        }
    }
}

pub(super) fn begin(game: &mut Game<'_>, alpha: f64) {
    STATE.with(|cell| {
        let mut s = cell.borrow_mut();
        s.enabled = enabled(game) && game.native_window();
        s.render_alpha = (alpha as f32).clamp(0., 1.);
        s.main_left = game.settings.main_hand_left;
        let deltas = [
            hand_delta(&s, false, s.render_alpha),
            hand_delta(&s, true, s.render_alpha),
        ];
        let motion = if s.enabled && game.native_setting(ID, "hand_poses").as_deref() != Some("horizon") {
            [motion_delta(&s, false, s.render_alpha), motion_delta(&s, true, s.render_alpha)]
        } else {
            [Mat4::IDENTITY; 2]
        };
        let active = if let Some(p) = &s.active {
            let left = s.main_left != (p.hand == action::Hand::Off);
            let dual = punchy_engine::policy::dual_handed(&p.clip, p.time, s.offhand_item, s.offhand_item);
            [!left || dual, left || dual]
        } else {
            [false, false]
        };
        let mut authored = [None, None];
        let render_alpha = s.render_alpha;
        let mut pendulum_input = pendulum_frame_input(&mut s, game, render_alpha);
        if s.enabled && game.native_setting(ID, "hand_poses").as_deref() != Some("horizon") {
            let source_bones = s.source.as_ref().map(|source| source.bones.clone());
            let physics_profiles: Option<[punchy_engine::tuning::PhysicsProfile; 2]> =
                s.tuning.as_ref().map(|tuning| {
                    std::array::from_fn(|i| {
                        let left = i != 0;
                        let hand = usize::from(s.main_left != left);
                        let context = punchy_engine::tuning::ItemContext {
                            id: &s.last_items[hand],
                            kind: s.item_kinds[hand].as_deref(),
                            ..Default::default()
                        };
                        tuning.physics_for(&context, left)
                    })
            });
            if let (Some(source_bones), Some(physics_profiles)) =
                (source_bones.as_ref(), physics_profiles.as_ref())
            {
                if let Ok(skeleton) = punchy_engine::skeleton::Skeleton::new(source_bones)
                {
                    for left in [false, true] {
                        let i = usize::from(left);
                        let h = usize::from(s.main_left != left);
                        let id = &s.last_items[h];
                        let owns = s.active.as_ref().is_some_and(|p| {
                            (s.main_left != (p.hand == action::Hand::Off)) == left
                                || punchy_engine::policy::dual_handed(&p.clip, p.time, s.offhand_item, s.offhand_item)
                        });
                        let animation = s.active.as_ref().map(|p| {
                            let t = if p.time < p.previous {
                                p.time
                            } else {
                                p.previous + (p.time - p.previous) * s.render_alpha
                            };
                            (&*p.clip, t, p.previous_expression+(p.expression-p.previous_expression)*s.render_alpha)
                        });
                        let weight = s.active.as_ref().map_or(1., |p| {
                            p.previous_weight + (p.weight - p.previous_weight) * s.render_alpha
                        });
                        let tags = held_tags(game, id);
                        let arm = editor_transform(&s, id, left, "arm", &tags);
                        let item = editor_transform(&s, id, left, "item", &tags);
                        let suppress_tuning = animation.is_some_and(|(c, t, _)| {
                            c.effect_active("disable_arm_tuning", t)
                                || (c.effect_active("use_keyframes_v2", t)
                                    && !c.effect_active("keep_arm_tuning", t))
                        });
                        let factor = if owns && suppress_tuning { 1. - weight } else { 1. };
                        let frame=s.active.as_ref().map_or_else(
                            ||punchy_engine::attachments::sample_frame(&skeleton,left,None,1.,true),
                            |p|p.frame(&skeleton,left,s.main_left,s.offhand_item,s.render_alpha));
                        let physics = physics_profiles[i];
                        let item_id = id.clone();
                        if s.pendulum_items[i] != item_id {
                            s.pendulum_states[i].reset();
                            s.pendulum_items[i] = item_id;
                        }
                        let pendulum_disabled = animation.is_some_and(|(clip, time, _)| {
                            owns && (clip.effect_active("use_keyframes_v2", time)
                                || clip.effect_active("disable_pendulum", time)
                                || clip.effect_active("disable_pendulum_physics", time))
                        });
                        let pendulum_influence = if animation.is_some_and(|(clip, time, _)| {
                            owns && (clip.effect_active("disable_pendulum", time)
                                || clip.effect_active("disable_pendulum_physics", time))
                        }) { 1.0 - weight } else { 1.0 };
                        pendulum_input.action_value = s.pendulum_action_values[i];
                        pendulum_input.action_kick = s.pendulum_action_kicks[i];
                        if physics.mode == punchy_engine::pendulum::Mode::None {
                            s.pendulum_states[i].reset();
                        } else {
                            let consumed = s.pendulum_states[i].update(pendulum_input, physics.settings, !pendulum_disabled);
                            if consumed {
                                s.pendulum_action_kicks[i] = 0.0;
                            }
                        }
                        let mut pose = punchy_engine::attachments::attach_with_pendulum(
                            frame,
                            left,
                            &arm,
                            &item,
                            factor,
                            if owns { 1. - weight } else { 1. },
                            id.ends_with(":chest")
                                || id.ends_with(":piston")
                                || id.ends_with(":sticky_piston"),
                            (physics.mode != punchy_engine::pendulum::Mode::None).then_some((
                                &mut s.pendulum_states[i],
                                physics.mode,
                                physics.settings,
                                pendulum_influence,
                                render_alpha,
                            )),
                        );
                        let slim = game.player_model.as_ref().is_some_and(|p| p.slim);
                        if let Some(model) = game.res.player_arm(slim, left).as_ref() {
                            let base = Mat4::from_translation(
                                Vec3::new(model.pose[0], model.pose[1], model.pose[2]) / 16.,
                            ) * Mat4::from_rotation_z(if left { -0.1 } else { 0.1 });
                            pose.arm *= base.inverse();
                            authored[i] = Some(pose);
                        }
                    }
                }
            }
        }
        s.render_count=s.render_count.saturating_add(1);
        if let Some(path)=std::env::var_os("PUNCHY_RENDER_DIAGNOSTICS") {
            let value=serde_json::json!({"frames":s.render_count,"authored_poses":authored.iter().flatten().count(),"active":active,
                "arm_matrices":authored.map(|p|p.map(|p|p.arm.to_cols_array())),"item_matrices":authored.map(|p|p.map(|p|p.item.to_cols_array()))});
            if let Ok(bytes)=serde_json::to_vec_pretty(&value){let _=std::fs::write(path,bytes);}
        }
        punchy_engine::horizon_pose::set_frame(punchy_engine::horizon_pose::Frame {
            authored,
            enabled: s.enabled,
            deltas,
            motion,
            active,
        });
    });
}

pub(super) fn ready(game: &Game<'_>) -> bool {
    enabled(game) && STATE.with(|cell| cell.borrow().source.is_some())
}
pub(super) fn item_pose(
    left: bool,
    height: f32,
    attack: f32,
    pose: horizon_client::first_person::ItemPose,
) -> Mat4 {
    if let Some(pose) = punchy_engine::horizon_pose::authored(left) {
        return punchy_engine::horizon_pose::motion(left) * pose.item;
    }
    let swing = if punchy_engine::horizon_pose::suppress_swing(left) {
        0.
    } else {
        attack
    };
    punchy_engine::horizon_pose::transform(
        left,
        horizon_client::first_person::item_pose(left, height, swing, pose),
    )
}
pub(super) fn arm_pose(left: bool, height: f32, attack: f32) -> Mat4 {
    if let Some(pose) = punchy_engine::horizon_pose::authored(left) {
        return punchy_engine::horizon_pose::motion(left) * pose.arm;
    }
    let swing = if punchy_engine::horizon_pose::suppress_swing(left) {
        0.
    } else {
        attack
    };
    punchy_engine::horizon_pose::transform(
        left,
        horizon_client::first_person::player_arm_pose(left, height, swing),
    )
}

pub(super) fn authored(left: bool) -> bool {
    punchy_engine::horizon_pose::authored(left).is_some()
}

// Called only after a successful client-side interaction, including offhand.
pub(super) fn used(game: &Game<'_>, used_hand: Hand, success: bool) {
    if success && game.native_window() && enabled(game) {
        STATE.with(|cell| {
            let mut state = cell.borrow_mut();
            if state.successful_use.len() >= 32 {
                state.successful_use.pop_front();
            }
            state.successful_use.push_back(hand(used_hand));
        });
    }
}

fn phase_hand(hand: action::Hand) -> phases::Hand {
    if hand == action::Hand::Main {
        phases::Hand::Main
    } else {
        phases::Hand::Off
    }
}
fn action_hand(hand: phases::Hand) -> action::Hand {
    if hand == phases::Hand::Main {
        action::Hand::Main
    } else {
        action::Hand::Off
    }
}
fn phase_playback(s: &State) -> phases::Playback {
    s.active
        .as_ref()
        .map_or_else(Default::default, |p| phases::Playback {
            clip: Some(p.clip.name.clone()),
            id: p.id,
            playing: !p.stopping,
            blending: p.stopping || p.weight < 1.,
            time: p.time,
            expression_time: p.expression,
            ..Default::default()
        })
}
fn phase_selection(
    s: &State,
    kind: &str,
    hand: action::Hand,
    speed: f32,
) -> Option<phases::Selection> {
    let item = &s.last_items[hand.index()];
    let entry = tuning_animation(s, hand, kind)?;
    let resolve = |name: &str| {
        let (_, clip) = entry.resolve(name, &s.clips)?;
        Some(phases::ClipSpec {
            name: clip.name.clone(),
            length: clip.length,
            speed_multiplier: punchy_engine::playback::manager_multiplier(&clip.name, speed, false),
            hold: clip.hold_on_last_frame,
        })
    };
    use std::hash::{Hash, Hasher};
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    item.hash(&mut hash);
    kind.hash(&mut hash);
    entry.raw.to_string().hash(&mut hash);
    Some(phases::Selection {
        entry_id: hash.finish(),
        start: resolve(entry.start.as_deref()?)?,
        looping: resolve(entry.looping.as_deref()?)?,
        end: resolve(entry.end.as_deref()?)?,
        skip_blend_in: entry.skip_blend_in,
        skip_blend_out: entry.skip_blend_out,
        stop_immediate: entry.stop_immediate,
    })
}
fn stack_token(s: &mut State, stack: Option<&ItemStack>, mining: bool) -> u64 {
    let Some(stack) = stack else {
        return 0;
    };
    let mut key = stack.clone();
    key.count = 1;
    if mining {
        key.damage = 0;
        if let Some(extra) = key.extra.as_mut() {
            extra.enchantments.clear();
            extra.other.retain(|(id, _)| id.starts_with("minecraft:"));
        }
    }
    if let Some(index) = s.stack_keys.iter().position(|k| *k == key) {
        return index as u64 + 1;
    }
    s.stack_keys.push(key);
    s.stack_keys.len() as u64
}
fn phase_effects(s: &mut State, effects: Vec<phases::Effect>) {
    for effect in effects {
        match effect {
            phases::Effect::ClearUseTracker => s.successful_use.clear(),
            phases::Effect::SetForcedStack { hand, token } => {
                s.forced_stack = token
                    .checked_sub(1)
                    .and_then(|i| s.stack_keys.get(i as usize))
                    .cloned()
                    .map(|v| (action_hand(hand).index(), v));
            }
            phases::Effect::ClearForcedStack => s.forced_stack = None,
            phases::Effect::ResetPoseImmediate => s.active = None,
            phases::Effect::SetLooping(value) => {
                if let Some(p) = &mut s.active {
                    p.looping = value;
                    p.released = !value;
                }
            }
            phases::Effect::SetSpeedMultiplier(value) => {
                if let Some(p) = &mut s.active {
                    p.multiplier = value;
                }
            }
            phases::Effect::SetBlendOutDurationNext(value) => {
                if let Some(p) = &mut s.active {
                    p.return_fade = value;
                }
            }
            // Source-hand and rendered-sync notifications are represented by the
            // playback hand and the forced stack at this renderer boundary.
            _ => {}
        }
    }
}
fn phase_tick(
    s: &mut State,
    input: &mut action::Input,
    items: &[Option<ItemStack>; 2],
    speed: f32,
    attack_down: bool,
) -> Option<(bool, phases::Request)> {
    s.tick += 1;
    let playback = phase_playback(s);
    let mining_selection =
        phase_selection(s, "mining", action::Hand::Main, speed);
    let mining_stack = stack_token(s, items[0].as_ref(), true);
    let mining = s.mining.tick(&phases::MiningInput {
        tick: s.tick,
        raw_mining: input.mining,
        triggered: input.mining_trigger,
        attack_down,
        aiming_trackable_block: input.block_target,
        stack: mining_stack,
        selection: mining_selection.clone(),
        playback: playback.clone(),
        ..Default::default()
    });
    let using = input.using;
    if let Some(u) = using {
        s.consume_hand = u.hand;
    }
    let h = s.consume_hand.index();
    let kind = using.map(|u| u.kind);
    let consume_kind = match kind {
        Some(action::ItemKind::Eat) => Some("eating"),
        Some(action::ItemKind::Drink) => Some("drinking"),
        _ => None,
    };
    let selection = consume_kind
        .and_then(|kind| phase_selection(s, kind, s.consume_hand, speed));
    let tokens = [
        stack_token(s, items[0].as_ref(), false),
        stack_token(s, items[1].as_ref(), false),
    ];
    let consume = s.consume.tick(&phases::ConsumeInput {
        use_down: input.use_down,
        using_item: using.is_some(),
        using_consume: consume_kind.is_some(),
        hand: phase_hand(s.consume_hand),
        using_stack: tokens[h],
        current_hand_stack: tokens[h],
        opposite_hand_stack: tokens[1 - h],
        animation_speed: speed,
        selection: selection.clone().map(phases::ConsumeSelection::Phases),
        playback: playback.clone(),
        ..Default::default()
    });
    if selection.is_some() {
        input.using = None;
        input.use_trigger = None;
    }
    if mining_selection.is_some() {
        input.mining_trigger = false;
        input.mining = false;
    }
    if s.consume.equip_blocked() {
        input.equip = None;
    }
    phase_effects(s, mining.effects);
    phase_effects(s, consume.effects);
    let request = consume
        .request
        .filter(|r| r.may_apply(&playback))
        .map(|r| (true, r))
        .or_else(|| {
            mining
                .request
                .filter(|r| r.may_apply(&playback))
                .map(|r| (false, r))
        });
    input.external_priority = request.as_ref().map(|(_, r)| r.priority);
    request
}
fn apply_phase(s: &mut State, consume: bool, request: &phases::Request, speed: f32) {
    let mut consume_tick = false;
    let play = match &request.command {
        phases::Command::Play(p) => Some(p.clone()),
        phases::Command::Handoff {
            clip,
            hand,
            speed_multiplier,
            tick_seconds,
            blend_seconds,
            fallback,
        } => {
            let playback = phase_playback(s);
            let Some(target) = s.clips.get(&clip.name) else {
                return;
            };
            let source_length = s.active.as_ref().map_or(0., |p| p.clip.length);
            let source_delta = s.active.as_ref().map_or(0., |p| {
                tick_seconds
                    * punchy_engine::playback::rate(&p.clip, p.time, speed, p.multiplier, 1.)
            });
            match phases::exact_handoff(
                &playback,
                source_length,
                source_delta,
                clip.length,
                *tick_seconds,
                *blend_seconds,
                |seconds| {
                    seconds
                        * punchy_engine::playback::rate(target, 0., speed, *speed_multiplier, 1.)
                },
            ) {
                phases::HandoffDecision::Wait => return,
                phases::HandoffDecision::Fallback => Some(fallback.clone()),
                phases::HandoffDecision::Start {
                    time,
                    expression_time,
                    consumes_tick,
                } => {
                    consume_tick = consumes_tick;
                    let mut p = fallback.clone();
                    p.clip = clip.clone();
                    p.hand = *hand;
                    p.looping = true;
                    p.hold = false;
                    p.speed_multiplier = *speed_multiplier;
                    p.blend_seconds = *blend_seconds;
                    p.start_time = time;
                    p.expression_time = expression_time;
                    p.stop_after_current = false;
                    Some(p)
                }
            }
        }
        phases::Command::FinishCurrent { release_hold } => {
            if let Some(p) = &mut s.active {
                p.looping = false;
                p.released = true;
                if *release_hold {
                    p.hold = false;
                }
            }
            None
        }
        phases::Command::StopImmediate { blend_seconds } => {
            if let Some(p) = &mut s.active {
                p.stopping = true;
                p.return_fade = blend_seconds.max(0.001);
            }
            None
        }
    };
    if let Some(play) = play {
        let Some(clip) = s.clips.get(&play.clip.name).cloned() else {
            return;
        };
        s.playback_id = s.playback_id.wrapping_add(1);
        let from = s.active.take().map(Box::new);
        let play_hand = action_hand(play.hand);
        s.active = Some(Playback {
            from,
            out_weight: 1.,
            previous_out_weight: 1.,
            id: s.playback_id,
            clip,
            hand: play_hand,
            previous: play.start_time,
            time: play.start_time,
            expression: play.expression_time,
            previous_expression: play.expression_time,
            multiplier: play.speed_multiplier,
            looping: play.looping,
            released: play.stop_after_current,
            hold: play.hold,
            fade: play.blend_seconds.max(0.001),
            return_fade: play.return_blend_seconds.unwrap_or(0.28),
            skip_tick: consume_tick,
            stopping: false,
            weight: if play.blend_seconds <= 0. { 1. } else { 0. },
            previous_weight: if play.blend_seconds <= 0. { 1. } else { 0. },
        });
        trigger_pendulum_action(s, play_hand);
        let playback = phase_playback(s);
        if consume {
            s.consume.commit(request, &playback);
        } else {
            s.mining.commit(request, &playback, s.tick);
        }
    }
}

pub(super) fn rendered_items(game: &Game<'_>) -> [Option<ItemStack>; 2] {
    let mut items = game.visible_hand_items();
    STATE.with(|s| {
        if let Some((hand, stack)) = &s.borrow().forced_stack {
            items[*hand] = Some(stack.clone());
        }
    });
    items
}

// Capture the target before creative prediction removes the block. Source
// records start attempts too; relying only on Game.mining loses instant breaks.
pub(super) fn mining_attempt(game: &Game<'_>) {
    if game.native_window() && enabled(game) && !game.spectator() {
        STATE.with(|s| s.borrow_mut().mining_attempt = true);
    }
}

#[cfg(test)]
mod tests {
    use super::{attack_held, inspect_clip_aliases};

    #[test]
    fn touch_attack_trigger_keeps_mining_held() {
        assert!(attack_held(false, false, 1.0, 0.4));
        assert!(!attack_held(false, false, 0.0, 0.4));
    }

    #[test]
    fn inspect_pickaxe_aliases_follow_the_selected_hand() {
        assert_eq!(
            inspect_clip_aliases("minecraft:diamond_pickaxe", None, false),
            vec!["new_inspect_pickaxe_right".to_owned(), "new_inspect_right".to_owned(), "new_inspect_pickaxe_left".to_owned(), "new_inspect_left".to_owned()]
        );
        assert_eq!(
            inspect_clip_aliases("minecraft:diamond_pickaxe", None, true),
            vec!["new_inspect_pickaxe_left".to_owned(), "new_inspect_left".to_owned(), "new_inspect_pickaxe_right".to_owned(), "new_inspect_right".to_owned()]
        );
    }

    #[test]
    fn inspect_non_pickaxe_keeps_the_generic_hand_specific_clips() {
        assert_eq!(
            inspect_clip_aliases("minecraft:diamond_sword", None, false),
            vec!["new_inspect_right".to_owned(), "new_inspect_left".to_owned()]
        );
    }

    #[test]
    fn mouse_and_mapped_controller_attack_holds_remain_supported() {
        assert!(attack_held(true, false, 0.0, 0.4));
        assert!(attack_held(false, true, 0.0, 0.4));
    }
}
