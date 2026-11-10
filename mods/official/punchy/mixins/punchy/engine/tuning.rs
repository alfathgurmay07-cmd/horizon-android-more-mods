//! Static subset of Punchy 3.1b's tuning resolver.
//!
//! Evidence: ToolTuning.getCustomAnimationInternal/mergeAnimationEntries,
//! CustomAnimationResolver.findEntries/normalizeType and
//! PunchyAnimationManager.resolveHandedClip in the local decompiled artifact.
//! World predicates, Java class/tag/component-based classification and mix-pack
//! overlay ordering require caller context; they are not inferred from an ID.
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Default)]
pub struct Tuning {
    groups: BTreeMap<String, Value>,
    custom_groups: BTreeMap<String, Vec<String>>,
    specific: BTreeMap<String, Value>,
    patterns: Vec<(String, String, Value)>,
    defaults: Vec<Value>,
    empty: Value,
    /// Original document, including currently unimplemented physics/proxy data.
    pub document: Value,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Transform {
    pub scale: f32,
    pub offset: [f32; 3],
    pub rotation: [f32; 3],
    pub pivot: [f32; 3],
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            scale: 1.0,
            offset: [0.0; 3],
            rotation: [0.0; 3],
            pivot: [0.0; 3],
        }
    }
}

impl Transform {
    fn parse(value: &Value) -> Self {
        let number = |key: &str, fallback| {
            value
                .get(key)
                .and_then(Value::as_f64)
                .filter(|x| x.is_finite())
                .map_or(fallback, |x| x as f32)
        };
        Self {
            scale: number("scale", 1.0),
            offset: [
                number("offX", 0.0),
                number("offY", 0.0),
                number("offZ", 0.0),
            ],
            rotation: [
                number("rotX", 0.0),
                number("rotY", 0.0),
                number("rotZ", 0.0),
            ],
            pivot: [
                number("pivotX", 0.0),
                number("pivotY", 0.0),
                number("pivotZ", 0.0),
            ],
        }
    }
}

/// The classifier's evidence comes from Horizon's registry, rather than guesses
/// for custom items. `kind` accepts Punchy's upper-case ToolKind names.
#[derive(Clone, Debug, Default)]
pub struct ItemContext<'a> {
    pub id: &'a str,
    pub kind: Option<&'a str>,
    pub tags: &'a [&'a str],
    pub translation_key: Option<&'a str>,
    /// Optional original state predicates. Empty context excludes conditional
    /// entries instead of silently making them unconditional.
    pub predicates: Option<&'a Value>,
}

#[derive(Clone, Debug)]
pub struct Animation {
    pub action: String,
    pub name: Option<String>,
    pub variations: Vec<String>,
    pub critical: Option<String>,
    pub start: Option<String>,
    pub looping: Option<String>,
    pub end: Option<String>,
    pub charge: Option<String>,
    pub release: Option<String>,
    pub abort: Option<String>,
    pub throw: Option<String>,
    pub pull: Option<String>,
    pub stages: [Option<String>; 3],
    pub skip_blend_in: bool,
    pub skip_blend_out: bool,
    pub stop_immediate: bool,
    pub trigger_on_input: bool,
    pub loop_speed_max: Option<f32>,
    pub loop_speed_ramp: Option<f32>,
    pub left: bool,
    pub raw: Value,
}

impl Animation {
    fn parse(entry: &Value, left: bool) -> Self {
        let field = |keys: &[&str]| keys.iter().find_map(|k| text(entry.get(*k)));
        let name = field(&["animation", "name", "var_1"]);
        let variations = (1..=3)
            .filter_map(|i| text(entry.get(format!("var_{i}"))))
            .collect();
        Self {
            action: field(&["type"]).unwrap_or_default(),
            name,
            variations,
            critical: field(&["critical_hit"]),
            start: field(&["start"]),
            looping: field(&["looping"]),
            end: field(&["end"]),
            charge: field(&["charge"]),
            release: field(&["release"]),
            abort: field(&["abbort", "abort"]),
            throw: field(&["throw", "throw_animation", "cast", "start_cast"]),
            pull: field(&["pull", "pull_animation", "reel", "reel_in"]),
            stages: [
                field(&["stage_1", "stage1"]),
                field(&["stage_2", "stage2"]),
                field(&["stage_3", "stage3"]),
            ],
            skip_blend_in: flag(entry, &["skip_blend_in"]),
            skip_blend_out: flag(entry, &["skip_blend_out"]),
            stop_immediate: flag(entry, &["stop_immediate"]),
            trigger_on_input: flag(entry, &["trigger_on_input"]),
            loop_speed_max: entry
                .get("loop_speed_max")
                .and_then(Value::as_f64)
                .map(|v| v as f32),
            loop_speed_ramp: entry
                .get("loop_speed_ramp")
                .and_then(Value::as_f64)
                .map(|v| v as f32),
            left,
            raw: entry.clone(),
        }
    }

    /// Original handed-clip fallback: requested side, unsuffixed base, opposite
    /// side. Explicit `_left`/`_right` names are never rewritten.
    pub fn candidates(&self, name: &str) -> Vec<String> {
        handed_candidates(name, self.left)
    }

    pub fn resolve<'a, T>(
        &self,
        name: &str,
        clips: &'a BTreeMap<String, T>,
    ) -> Option<(&'a str, &'a T)> {
        handed_candidates(name, self.left).iter().find_map(|n| {
            clips
                .get_key_value(n)
                .map(|(name, clip)| (name.as_str(), clip))
        })
    }

    /// Combo numbering is zero-based at this Rust interface.
    pub fn attack_name(&self, combo: usize, critical: bool) -> Option<&str> {
        if critical && let Some(name) = self.critical.as_deref() {
            return Some(name);
        }
        if !self.variations.is_empty() {
            Some(&self.variations[combo % self.variations.len()])
        } else {
            self.name.as_deref()
        }
    }
}

pub fn handed_candidates(name: &str, left: bool) -> Vec<String> {
    let name = name.trim();
    if name.is_empty() {
        return vec![];
    }
    if name.ends_with("_left") || name.ends_with("_right") {
        return vec![name.into()];
    }
    let (side, other) = if left {
        ("left", "right")
    } else {
        ("right", "left")
    };
    vec![
        format!("{name}_{side}"),
        name.into(),
        format!("{name}_{other}"),
    ]
}

impl Tuning {
    pub fn parse(value: &Value) -> Result<Self, String> {
        if !value.is_object() {
            return Err("Punchy tuning root must be an object".into());
        }
        let mut out = Self {
            document: value.clone(),
            ..Self::default()
        };
        if let Some(groups) = value.get("customGroups").and_then(Value::as_array) {
            for group in groups {
                if let Some(name) = text(group.get("name")) {
                    out.custom_groups.insert(name, strings(group.get("items")));
                }
            }
        }
        if let Some(groups) = value.get("item").and_then(Value::as_object) {
            for (name, group) in groups {
                out.groups.insert(name.clone(), group.clone());
                let pattern = text(
                    group
                        .get("pattern_matching")
                        .or_else(|| group.get("patternMatching")),
                )
                .or_else(|| name.strip_prefix("related:").map(str::to_owned));
                if let Some(pattern) = pattern {
                    out.patterns
                        .push((pattern.to_lowercase(), name.clone(), group.clone()));
                }
            }
        }
        // Longer selectors win; explicit itemSpecific overrides all patterns.
        out.patterns.sort_by(|a, b| {
            b.0.len()
                .cmp(&a.0.len())
                .then_with(|| a.1.to_lowercase().cmp(&b.1.to_lowercase()))
        });
        if let Some(specific) = value.get("itemSpecific").and_then(Value::as_object) {
            for (id, entry) in specific {
                out.specific.insert(normalize_id(id), entry.clone());
            }
        }
        out.empty = value.get("emptyHand").cloned().unwrap_or(Value::Null);
        out.defaults = array(
            value
                .get("default_animations")
                .or_else(|| value.get("defaultAnimations")),
        );
        Ok(out)
    }

    /// Convenient ID-only path for vanilla tools. For blocks, tags, consumable
    /// components or custom items use `animation_for` with registry `kind`.
    pub fn animation(&self, item_id: &str, action: &str, left: bool) -> Option<Animation> {
        self.animation_for(
            &ItemContext {
                id: item_id,
                ..ItemContext::default()
            },
            action,
            left,
        )
    }

    pub fn animation_for(
        &self,
        item: &ItemContext<'_>,
        action: &str,
        left: bool,
    ) -> Option<Animation> {
        let target = normalize_type(action);
        let entries = self.entries(item);
        let mut matches: Vec<_> = entries
            .iter()
            .filter(|entry| {
                text(entry.get("type")).is_some_and(|t| normalize_type(&t) == target)
                    && self.matches_items(item, entry.get("items"))
                    && matches_predicates(entry, item.predicates)
            })
            .collect();
        // Java's stable sort uses descending trigger specificity.
        matches.sort_by_key(|entry| std::cmp::Reverse(specificity(entry)));
        matches.first().map(|entry| Animation::parse(entry, left))
    }

    /// All matching triggerable entries must remain distinct by key/id.
    pub fn animations_for(
        &self,
        item: &ItemContext<'_>,
        action: &str,
        left: bool,
    ) -> Vec<Animation> {
        let target = normalize_type(action);
        self.entries(item)
            .into_iter()
            .filter(|entry| {
                text(entry.get("type")).is_some_and(|t| normalize_type(&t) == target)
                    && self.matches_items(item, entry.get("items"))
                    && matches_predicates(entry, item.predicates)
            })
            .map(|e| Animation::parse(&e, left))
            .collect()
    }

    pub fn item_transform(&self, id: &str, left: bool) -> Option<Transform> {
        self.transform_for(
            &ItemContext {
                id,
                ..ItemContext::default()
            },
            left,
            false,
        )
    }

    pub fn arm_transform(&self, id: &str, left: bool) -> Option<Transform> {
        self.transform_for(
            &ItemContext {
                id,
                ..ItemContext::default()
            },
            left,
            true,
        )
    }

    pub fn transform_for(
        &self,
        item: &ItemContext<'_>,
        left: bool,
        arm: bool,
    ) -> Option<Transform> {
        let side = if left { "left" } else { "right" };
        let key = if arm { "armTransforms" } else { "transforms" };
        let specific = self.specific_entry(item);
        let group = self.group_entry(item);
        // Explicit per-item side values inherit missing values from its group.
        let base = group.and_then(|g| g.get(key)).and_then(|s| s.get(side));
        let over = specific.and_then(|g| g.get(key)).and_then(|s| s.get(side));
        let merged = match (base, over) {
            (Some(a), Some(b)) => merge_objects(a, b),
            (Some(v), None) | (None, Some(v)) => v.clone(),
            (None, None) => return None,
        };
        Some(Transform::parse(&merged))
    }

    pub fn runtime_flag(&self, item: &ItemContext<'_>, key: &str) -> Option<bool> {
        self.specific_entry(item)
            .and_then(|v| v.get(key))
            .and_then(Value::as_bool)
            .or_else(|| {
                self.group_entry(item)
                    .and_then(|v| v.get(key))
                    .and_then(Value::as_bool)
            })
    }

    pub fn kind(&self, item: &ItemContext<'_>) -> String {
        self.specific_entry(item)
            .and_then(|v| text(v.get("kind")))
            .or_else(|| item.kind.map(str::to_owned))
            .unwrap_or_else(|| vanilla_kind(item.id).into())
            .to_uppercase()
    }

    fn specific_entry(&self, item: &ItemContext<'_>) -> Option<&Value> {
        let id = normalize_id(item.id);
        if let Some(v) = self.specific.get(&id) {
            return Some(v);
        }
        for (pattern, _, entry) in &self.patterns {
            if id.contains(pattern)
                || item
                    .translation_key
                    .is_some_and(|key| key.to_lowercase().contains(pattern))
            {
                return Some(entry);
            }
        }
        for (name, selectors) in &self.custom_groups {
            if selectors.iter().any(|s| self.selector_matches(item, s))
                && let Some(v) = self.groups.get(name)
            {
                return Some(v);
            }
        }
        None
    }

    fn group_entry(&self, item: &ItemContext<'_>) -> Option<&Value> {
        if is_empty(item.id) {
            return Some(&self.empty);
        }
        self.groups.get(&self.kind(item))
    }

    fn entries(&self, item: &ItemContext<'_>) -> Vec<Value> {
        let specific = self
            .specific_entry(item)
            .and_then(|v| v.get("customAnimation"));
        let primary = specific.or_else(|| {
            self.group_entry(item)
                .and_then(|v| v.get("customAnimation"))
        });
        let mut entries = array(primary);
        if entries.is_empty() {
            return self.defaults.clone();
        }
        let mut keys = BTreeSet::new();
        let mut merged = Vec::new();
        // Java LinkedHashMap: last repeated primary key replaces value without
        // moving insertion position; defaults only fill missing merge keys.
        for entry in entries.drain(..) {
            if let Some(key) = merge_key(&entry) {
                if keys.insert(key.clone()) {
                    merged.push(entry);
                } else if let Some(old) = merged
                    .iter_mut()
                    .find(|old| merge_key(old).as_deref() == Some(&key))
                {
                    *old = entry;
                }
            }
        }
        for entry in &self.defaults {
            if let Some(key) = merge_key(entry)
                && keys.insert(key)
            {
                merged.push(entry.clone());
            }
        }
        merged
    }

    fn matches_items(&self, item: &ItemContext<'_>, value: Option<&Value>) -> bool {
        let items = strings(value);
        items.is_empty() || items.iter().any(|s| self.selector_matches(item, s))
    }

    fn selector_matches(&self, item: &ItemContext<'_>, selector: &str) -> bool {
        let selector = selector.trim();
        if let Some(tag) = selector.strip_prefix('#') {
            let tag = normalize_id(tag);
            return item
                .tags
                .iter()
                .any(|t| normalize_id(t.trim_start_matches('#')) == tag);
        }
        // REGISTRY_CUSTOM_GROUPS entries may themselves be referenced by name.
        if let Some(selectors) = self.custom_groups.get(selector).or_else(|| {
            self.custom_groups
                .iter()
                .find(|(name, _)| name.eq_ignore_ascii_case(selector))
                .map(|(_, v)| v)
        }) {
            return selectors
                .iter()
                .filter(|s| s.as_str() != selector)
                .any(|s| {
                    if let Some(tag) = s.strip_prefix('#') {
                        item.tags
                            .iter()
                            .any(|t| normalize_id(t.trim_start_matches('#')) == normalize_id(tag))
                    } else {
                        normalize_id(s) == normalize_id(item.id)
                    }
                });
        }
        normalize_id(selector) == normalize_id(item.id)
    }
}

fn array(value: Option<&Value>) -> Vec<Value> {
    value.and_then(Value::as_array).cloned().unwrap_or_default()
}
fn strings(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(|v| text(Some(v))).collect())
        .unwrap_or_default()
}
fn text(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}
fn flag(value: &Value, keys: &[&str]) -> bool {
    keys.iter()
        .find_map(|k| value.get(*k).and_then(Value::as_bool))
        .unwrap_or(false)
}
fn normalize_id(id: &str) -> String {
    let id = id.trim().to_lowercase();
    if id.contains(':') || id.is_empty() {
        id
    } else {
        format!("minecraft:{id}")
    }
}
fn is_empty(id: &str) -> bool {
    id.trim().is_empty() || normalize_id(id) == "minecraft:air"
}
pub fn normalize_type(s: &str) -> String {
    let normalized: String = s
        .trim()
        .to_lowercase()
        .chars()
        .filter(|c| !matches!(c, '_' | '-' | ' '))
        .collect();
    if normalized == "miningv2" {
        "mining".into()
    } else {
        normalized
    }
}
fn merge_key(entry: &Value) -> Option<String> {
    let ty = text(entry.get("type"))?;
    // Unlike findEntry, animationMergeKey does not alias mining_v2 to mining.
    let ty: String = ty
        .to_lowercase()
        .chars()
        .filter(|c| !matches!(c, '_' | '-' | ' '))
        .collect();
    let key = text(entry.get("key")).unwrap_or_default().to_lowercase();
    let mut key = format!("{ty}|{key}");
    if ty == "interaction" {
        for name in [
            "items",
            "blockState",
            "in_water",
            "inside_water",
            "block_detection",
            "weather_detection",
            "time_detection",
            "biome_detection",
        ] {
            key.push('|');
            key.push_str(&entry.get(name).unwrap_or(&Value::Null).to_string());
        }
    }
    Some(key)
}
fn specificity(entry: &Value) -> usize {
    let items = if strings(entry.get("items")).is_empty() {
        0
    } else {
        100
    };
    items
        + [
            "in_water",
            "inside_water",
            "block_detection",
            "weather_detection",
            "time_detection",
            "biome_detection",
        ]
        .iter()
        .filter(|k| entry.get(**k).is_some_and(|v| !v.is_null()))
        .count()
}
fn matches_predicates(entry: &Value, context: Option<&Value>) -> bool {
    for key in [
        "blockState",
        "in_water",
        "inside_water",
        "block_detection",
        "weather_detection",
        "time_detection",
        "biome_detection",
    ] {
        if let Some(expected) = entry.get(key).filter(|v| !v.is_null())
            && context.and_then(|ctx| ctx.get(key)) != Some(expected)
        {
            return false;
        }
    }
    true
}
fn merge_objects(base: &Value, over: &Value) -> Value {
    let mut out = base.clone();
    if let (Some(a), Some(b)) = (out.as_object_mut(), over.as_object()) {
        for (key, value) in b {
            a.insert(key.clone(), value.clone());
        }
    } else {
        return over.clone();
    }
    out
}

/// Only names whose vanilla category is unambiguous from an item ID. Registry
/// tags, BlockItem collision shapes and consumable components are caller input.
pub fn vanilla_kind(id: &str) -> &'static str {
    let id = normalize_id(id);
    if is_empty(&id) {
        return "EMPTY_HAND";
    }
    if !id.starts_with("minecraft:") {
        return "INGREDIENT";
    }
    let name = id.trim_start_matches("minecraft:");
    if name.ends_with("_pickaxe") {
        "PICKAXE"
    } else if name.ends_with("_sword") {
        "SWORD"
    } else if name.ends_with("_shovel") {
        "SHOVEL"
    } else if name.ends_with("_axe") {
        "AXE"
    } else if name.ends_with("_hoe") {
        "HOE"
    } else if name == "spear" || name.ends_with("_spear") {
        "SPEAR"
    } else if matches!(
        name,
        "bow" | "crossbow" | "shield" | "trident" | "spyglass" | "mace" | "arrow"
    ) {
        match name {
            "bow" => "BOW",
            "crossbow" => "CROSSBOW",
            "shield" => "SHIELD",
            "trident" => "TRIDENT",
            "spyglass" => "SPYGLASS",
            "mace" => "MACE",
            _ => "ARROW",
        }
    } else if name == "flint_and_steel" {
        "FLINT"
    } else if matches!(
        name,
        "fishing_rod" | "carrot_on_a_stick" | "warped_fungus_on_a_stick"
    ) {
        "FISHING_ROD"
    } else if matches!(
        name,
        "axolotl_bucket"
            | "tropical_fish_bucket"
            | "pufferfish_bucket"
            | "cod_bucket"
            | "salmon_bucket"
            | "tadpole_bucket"
    ) {
        "CREATURE_BUCKET"
    } else if name.contains("bucket") {
        "BUCKET"
    } else if name.ends_with("minecart") {
        "MINECART"
    } else if name.ends_with("_boat") || name.ends_with("_raft") {
        "BOAT"
    } else if matches!(name, "lantern" | "soul_lantern") || name.contains("copper_lantern") {
        "LANTERN"
    } else {
        "INGREDIENT"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn merges_primary_before_defaults_with_java_key_rules() {
        let t=Tuning::parse(&json!({"default_animations":[{"type":"attack","var_1":"punch"},{"type":"mining_v2","start":"punch_start"},{"type":"hand_in","var_1":"equip"}],"item":{"SWORD":{"customAnimation":[{"type":"attack","var_1":"a","var_2":"b","critical_hit":"crit"},{"type":"mining","name":"sword_mine"}]}}})).unwrap();
        let a = t
            .animation("minecraft:diamond_sword", "ATT_ACK", false)
            .unwrap();
        assert_eq!(a.attack_name(1, false), Some("b"));
        assert_eq!(a.attack_name(4, true), Some("crit"));
        assert_eq!(
            t.animation("diamond_sword", "mining-v2", false)
                .unwrap()
                .name
                .as_deref(),
            Some("sword_mine")
        );
        assert_eq!(
            t.animation("diamond_sword", "hand_in", false)
                .unwrap()
                .name
                .as_deref(),
            Some("equip")
        );
    }
    #[test]
    fn exact_item_overrides_patterns_and_explicit_empty_animation_discards_group() {
        let t=Tuning::parse(&json!({"default_animations":[{"type":"attack","name":"default"}],"item":{"SWORD":{"pattern_matching":"sword","customAnimation":[{"type":"inspect","name":"sword"}]}},"itemSpecific":{"minecraft:diamond_sword":{"customAnimation":[]}}})).unwrap();
        assert!(t.animation("diamond_sword", "inspect", false).is_none());
        assert_eq!(
            t.animation("iron_sword", "inspect", false)
                .unwrap()
                .name
                .as_deref(),
            Some("sword")
        );
        assert_eq!(
            t.animation("diamond_sword", "attack", false)
                .unwrap()
                .name
                .as_deref(),
            Some("default")
        );
    }
    #[test]
    fn conditional_entries_need_evidence_and_custom_group_resolves() {
        let t=Tuning::parse(&json!({"customGroups":[{"name":"HORNS","items":["minecraft:goat_horn"]}],"item":{"HORNS":{"customAnimation":[{"type":"useItem","key":"wet","name":"wet","in_water":true},{"type":"useitem","name":"horn"}]}}})).unwrap();
        assert_eq!(
            t.animation("goat_horn", "use_item", false)
                .unwrap()
                .name
                .as_deref(),
            Some("horn")
        );
        let predicates = json!({"in_water":true});
        assert_eq!(
            t.animation_for(
                &ItemContext {
                    id: "goat_horn",
                    predicates: Some(&predicates),
                    ..Default::default()
                },
                "useItem",
                false
            )
            .unwrap()
            .name
            .as_deref(),
            Some("wet")
        );
    }
    #[test]
    fn handed_resolution_uses_requested_base_then_opposite() {
        let mut clips =
            BTreeMap::from([("inspect_right".to_owned(), 1), ("inspect".to_owned(), 2)]);
        let a = Animation::parse(&json!({"type":"inspect","name":"inspect"}), true);
        assert_eq!(a.resolve("inspect", &clips).map(|v| v.0), Some("inspect"));
        clips.remove("inspect");
        assert_eq!(
            a.resolve("inspect", &clips).map(|v| v.0),
            Some("inspect_right")
        );
        assert_eq!(a.candidates("inspect_right"), ["inspect_right"]);
    }
    #[test]
    fn bundled_special_use_tuning_resolves_bow_trident_spear_and_consumables() {
        let t = Tuning::parse(
            &serde_json::from_str(include_str!("data/tuning.json")).unwrap(),
        )
        .unwrap();

        let bow = t.animation("minecraft:bow", "use_bow", false).unwrap();
        assert_eq!(bow.name.as_deref(), Some("use_bow"));
        let bow_shot = t.animation("minecraft:bow", "use_bow_shot", false).unwrap();
        assert_eq!(bow_shot.name.as_deref(), Some("use_bow_shoot"));

        let trident = t.animation("minecraft:trident", "use_trident", false).unwrap();
        assert_eq!(trident.name.as_deref(), Some("use_trident"));
        assert_eq!(
            t.animation("minecraft:trident", "use_trident_throw_riptide", false)
                .unwrap()
                .name
                .as_deref(),
            Some("use_trident_throw_riptide")
        );

        let spear = t.animation("minecraft:wooden_spear", "spear_charge", false).unwrap();
        assert_eq!(spear.stages[0].as_deref(), Some("use_spear_pose_1"));
        assert_eq!(spear.stages[1].as_deref(), Some("use_spear_pose_2"));
        assert_eq!(spear.stages[2].as_deref(), Some("use_spear_pose_3"));
        assert_eq!(vanilla_kind("minecraft:spear"), "SPEAR");

        let eating = t.animation("minecraft:golden_apple", "eating", false).unwrap();
        assert_eq!(eating.start.as_deref(), Some("eat_start"));
        assert_eq!(eating.looping.as_deref(), Some("eat_loop"));
        assert_eq!(eating.end.as_deref(), Some("eat_end"));
    }

    #[test]
    fn registry_kind_covers_full_blocks_without_guessing_custom_names() {
        let t=Tuning::parse(&json!({"item":{"FULL_BLOCK":{"customAnimation":[{"type":"attack","var_1":"interact"}]}}})).unwrap();
        let item = ItemContext {
            id: "minecraft:stone",
            kind: Some("FULL_BLOCK"),
            ..Default::default()
        };
        assert_eq!(
            t.animation_for(&item, "attack", false)
                .unwrap()
                .name
                .as_deref(),
            Some("interact")
        );
        assert!(t.animation("example:fake_sword", "attack", false).is_none());
    }
}
