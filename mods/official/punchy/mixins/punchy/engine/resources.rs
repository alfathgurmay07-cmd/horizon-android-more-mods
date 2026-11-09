use glam::{EulerRot, Mat4, Quat, Vec3};
use serde_json::Value;
use std::{collections::BTreeMap, fs::File, io::{Cursor, Read}, path::Path};

const MAX_MEMBER: u64 = 32 * 1024 * 1024;
pub const ANIMATIONS: &str =
    "resourcepacks/punchy/assets/minecraft/punchy/animations/player.animation.json";
pub const GEOMETRY: &str = "assets/punchy/geo/player.geo.json";
pub const PACK_ANIMATIONS: &str = "assets/minecraft/punchy/animations/player.animation.json";
pub const PACK_THIRD_PERSON_ANIMATIONS: &str = "assets/punchy/animations/player_3_p.animation.json";
pub const PACK_THIRD_PERSON_ANIMATIONS_FALLBACK: &str = "assets/minecraft/punchy/animations/player_3_p.animation.json";
pub const PACK_GEOMETRY: &str = "assets/punchy/geo/player.geo.json";
pub const PACK_TUNING: &str = "assets/minecraft/punchy/compat/tuning_template.json";

#[derive(Clone, Debug)]
pub struct Bone {
    pub name: String,
    pub parent: Option<String>,
    pub position: Vec3,
    pub rotation: Vec3,
}
impl Bone {
    /// SimpleBedrockGeo's ModelPart base transform, in model pixels.
    pub fn matrix(&self) -> Mat4 {
        Mat4::from_translation(self.position)
            * Mat4::from_quat(Quat::from_euler(
                EulerRot::ZYX,
                self.rotation.z.to_radians(),
                self.rotation.y.to_radians(),
                self.rotation.x.to_radians(),
            ))
    }
}
pub struct Resources {
    pub animations: Vec<u8>,
    pub bones: BTreeMap<String, Bone>,
    pub tuning: Value,
}
const BUILTIN_RESOURCE_PACK: &[u8] = include_bytes!("data/punchy_builtin.zip");

pub fn builtin_resource_pack() -> &'static [u8] {
    BUILTIN_RESOURCE_PACK
}

fn builtin_member(name: &str) -> Result<Vec<u8>, String> {
    let mut archive = zip::ZipArchive::new(Cursor::new(BUILTIN_RESOURCE_PACK))
        .map_err(|error| error.to_string())?;
    let mut entry = archive.by_name(name).map_err(|error| format!("{name}: {error}"))?;
    if entry.size() > MAX_MEMBER {
        return Err(format!("{name} exceeds 32 MiB"));
    }
    let mut bytes = Vec::new();
    entry
        .by_ref()
        .take(MAX_MEMBER + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.len() as u64 > MAX_MEMBER {
        return Err(format!("{name} exceeds 32 MiB"));
    }
    Ok(bytes)
}

pub fn embedded() -> Result<Resources, String> {
    let animations = builtin_member(PACK_ANIMATIONS)?;
    let geometry = builtin_member(PACK_GEOMETRY)?;
    let tuning = builtin_member(PACK_TUNING)?;
    Ok(Resources {
        animations,
        bones: parse_bones(&geometry)?,
        tuning: serde_json::from_slice(&tuning).map_err(|e| e.to_string())?,
    })
}

pub fn from_active_pack_tree(
    animation_files: &[(String, Vec<u8>, i64)],
    geometry: Option<&[u8]>,
    default_tuning: Option<&[u8]>,
    compat_files: &[(String, Vec<u8>, i64)],
) -> Result<(Resources, bool), String> {
    let mut result = embedded()?;
    let mut has_pack_data = false;
    let mut animation_doc: Value = serde_json::from_slice(&result.animations)
        .map_err(|e| e.to_string())?;
    let mut ordered_animations: Vec<_> = animation_files.iter().collect();
    ordered_animations.sort_by(|a, b| {
        b.2.cmp(&a.2)
            .then_with(|| animation_namespace_rank(&a.0).cmp(&animation_namespace_rank(&b.0)))
            .then_with(|| a.0.cmp(&b.0))
    });
    let mut external_clips = BTreeMap::<String, Value>::new();
    for (path, bytes, _) in ordered_animations {
        if !is_punchy_animation_path(path) || bytes.is_empty() {
            continue;
        }
        let Ok(document) = serde_json::from_slice::<Value>(bytes) else { continue };
        let Some(clips) = document.get("animations").and_then(Value::as_object) else {
            continue;
        };
        for (name, clip) in clips {
            if !name.trim().is_empty() && clip.is_object() {
                external_clips.entry(name.clone()).or_insert_with(|| clip.clone());
            }
        }
    }
    if !external_clips.is_empty() {
        let destination = animation_doc
            .get_mut("animations")
            .and_then(Value::as_object_mut)
            .ok_or("Bundled Punchy animations are missing their animations object")?;
        for (name, clip) in external_clips {
            destination.insert(name, clip);
            has_pack_data = true;
        }
    }
    result.animations = serde_json::to_vec(&animation_doc).map_err(|e| e.to_string())?;

    if let Some(bytes) = geometry.filter(|b| !b.is_empty()) {
        result.bones = parse_bones(bytes).map_err(|e| format!("Punchy geometry: {e}"))?;
        has_pack_data = true;
    }
    if let Some(bytes) = default_tuning.filter(|b| !b.is_empty()) {
        if let Ok(incoming) = serde_json::from_slice::<Value>(bytes) {
            if incoming.is_object() {
                result.tuning = incoming;
                has_pack_data = true;
            }
        }
    }

    let mut ordered_compat: Vec<_> = compat_files.iter().collect();
    ordered_compat.sort_by(|a, b| {
        b.2.cmp(&a.2)
            .then_with(|| compat_namespace_rank(&a.0).cmp(&compat_namespace_rank(&b.0)))
            .then_with(|| a.0.cmp(&b.0))
    });
    let has_external_default_tuning = ordered_compat.iter().any(|entry| {
        let path = &entry.0;
        let bytes = &entry.1;
        let priority = entry.2;
        is_punchy_compat_path(path)
            && compat_profile(path).as_deref() == Some("default_tuning")
            && priority >= 0
            && !bytes.is_empty()
    });
    let mut compat_overlay = Value::Null;
    for (path, bytes, priority) in ordered_compat {
        if !is_punchy_compat_path(path) || bytes.is_empty() {
            continue;
        }
        let Some(profile) = compat_profile(path) else { continue };
        if has_external_default_tuning && profile == "tuning_template" && *priority < 0 {
            continue;
        }
        let Ok(incoming) = serde_json::from_slice::<Value>(bytes) else { continue };
        if !incoming.is_object() {
            continue;
        }
        merge_tuning_prefer_base(&mut compat_overlay, &incoming, None);
        has_pack_data = true;
    }
    if !compat_overlay.is_null() {
        merge_tuning_override(&mut result.tuning, &compat_overlay, None);
    }

    Ok((result, has_pack_data))
}

fn is_punchy_animation_path(path: &str) -> bool {
    path.starts_with("assets/punchy/animations/")
        || path.starts_with("assets/minecraft/punchy/animations/")
}

fn animation_namespace_rank(path: &str) -> u8 {
    if path.starts_with("assets/punchy/animations/") { 0 } else { 1 }
}

fn is_punchy_compat_path(path: &str) -> bool {
    path.starts_with("assets/minecraft/punchy/compat/")
        || path.starts_with("assets/punchy/compat/")
}

fn compat_namespace_rank(path: &str) -> u8 {
    if path.starts_with("assets/minecraft/punchy/compat/") { 0 } else { 1 }
}

fn compat_profile(path: &str) -> Option<String> {
    let prefix = if let Some(rest) = path.strip_prefix("assets/minecraft/punchy/compat/") {
        rest
    } else {
        path.strip_prefix("assets/punchy/compat/")?
    };
    let name = prefix.strip_suffix(".json")?;
    if name.trim().is_empty() { None } else { Some(name.to_lowercase()) }
}

fn animation_family(value: &Value) -> String {
    let raw = value.get("type").and_then(Value::as_str).unwrap_or("");
    let compact: String = raw.trim().to_lowercase().chars()
        .filter(|c| *c != '_' && *c != '-' && !c.is_whitespace()).collect();
    if compact == "mining" || compact == "miningv2" { "mining".into() } else { compact }
}

fn custom_animation_key(value: &Value) -> String {
    let family = animation_family(value);
    if family == "interaction" {
        let selectors = serde_json::json!([
            value.get("items"), value.get("blockState"), value.get("in_water"),
            value.get("inside_water"), value.get("block_detection"),
            value.get("weather_detection"), value.get("time_detection"),
            value.get("biome_detection")
        ]);
        let key = value.get("key").and_then(Value::as_str).unwrap_or("").trim().to_lowercase();
        format!("{family}|{key}|{selectors}")
    } else if family.is_empty() { "__untyped__".into() } else { family }
}

fn merge_animation_list_override(base: &mut Value, overlay: &Value) {
    let Some(incoming) = overlay.as_array() else { *base = overlay.clone(); return };
    let Some(existing) = base.as_array() else { *base = overlay.clone(); return };
    let mut groups = Vec::<(String, Vec<Value>)>::new();
    for entry in incoming {
        let key = custom_animation_key(entry);
        if let Some((_, values)) = groups.iter_mut().find(|(k, _)| *k == key) {
            values.push(entry.clone());
        } else {
            groups.push((key, vec![entry.clone()]));
        }
    }
    let mut result = Vec::new();
    let mut applied = BTreeMap::<String, bool>::new();
    for entry in existing {
        let key = custom_animation_key(entry);
        if let Some((_, replacements)) = groups.iter().find(|(k, _)| *k == key) {
            if applied.insert(key, true).is_none() {
                result.extend(replacements.iter().cloned());
            }
        } else {
            result.push(entry.clone());
        }
    }
    for (key, replacements) in groups {
        if applied.insert(key, true).is_none() {
            result.extend(replacements);
        }
    }
    *base = Value::Array(result);
}

fn merge_animation_list_prefer_base(base: &mut Value, overlay: &Value) {
    let Some(incoming) = overlay.as_array() else { return };
    let Some(existing) = base.as_array_mut() else {
        if base.is_null() { *base = overlay.clone(); }
        return;
    };
    let mut keys: BTreeMap<String, bool> = existing.iter().map(|v| (custom_animation_key(v), true)).collect();
    for entry in incoming {
        let key = custom_animation_key(entry);
        if keys.insert(key, true).is_none() { existing.push(entry.clone()); }
    }
}

fn merge_tuning_prefer_base(base: &mut Value, overlay: &Value, key: Option<&str>) {
    if base.is_null() { *base = overlay.clone(); return; }
    match (base, overlay) {
        (Value::Object(base), Value::Object(overlay)) => {
            for (name, value) in overlay {
                match base.get_mut(name) {
                    Some(existing) => merge_tuning_prefer_base(existing, value, Some(name)),
                    None => { base.insert(name.clone(), value.clone()); }
                }
            }
        }
        (Value::Array(base), Value::Array(overlay)) => {
            match key {
                Some("customAnimation" | "customItemAnimations" | "default_animations" | "defaultAnimations") => {
                    let mut value = Value::Array(std::mem::take(base));
                    merge_animation_list_prefer_base(&mut value, &Value::Array(overlay.clone()));
                    *base = value.as_array().cloned().unwrap_or_default();
                }
                Some("customGroups") => {
                    for entry in overlay {
                        let name = entry.get("name").and_then(Value::as_str).unwrap_or("").to_lowercase();
                        let exists = !name.is_empty() && base.iter().any(|v| v.get("name").and_then(Value::as_str).is_some_and(|n| n.eq_ignore_ascii_case(&name)));
                        if !exists { base.push(entry.clone()); }
                    }
                }
                _ => {}
            }
        }
        _ => {}
    }
}

/// Apply the already-prioritized compat overlay to the base tuning document.
fn merge_tuning_override(base: &mut Value, overlay: &Value, key: Option<&str>) {
    match (base, overlay) {
        (Value::Object(base), Value::Object(overlay)) => {
            for (name, value) in overlay {
                match base.get_mut(name) {
                    Some(existing) => merge_tuning_override(existing, value, Some(name)),
                    None => { base.insert(name.clone(), value.clone()); }
                }
            }
        }
        (Value::Array(base), Value::Array(overlay)) => match key {
            Some("customAnimation" | "customItemAnimations" | "default_animations" | "defaultAnimations") => {
                let mut value = Value::Array(std::mem::take(base));
                merge_animation_list_override(&mut value, &Value::Array(overlay.clone()));
                *base = value.as_array().cloned().unwrap_or_default();
            }
            Some("customGroups") => base.extend(overlay.iter().cloned()),
            _ => *base = overlay.clone(),
        },
        (base, overlay) => *base = overlay.clone(),
    }
}

pub fn from_active_pack(
    first_person: Option<&[u8]>,
    third_person: Option<&[u8]>,
    third_person_fallback: Option<&[u8]>,
    geometry: Option<&[u8]>,
    tuning: Option<&[u8]>,
) -> Result<(Resources, bool), String> {
    let mut result = embedded()?;
    let mut animation_doc: Value = serde_json::from_slice(&result.animations).map_err(|e| e.to_string())?;
    let mut has_pack_data = false;
    for (label, bytes) in [
        ("third-person animations", third_person.or(third_person_fallback)),
        ("first-person animations", first_person),
    ] {
        let Some(bytes) = bytes.filter(|b| !b.is_empty()) else { continue };
        let incoming: Value = serde_json::from_slice(bytes).map_err(|e| format!("Punchy {label}: {e}"))?;
        let incoming = incoming.get("animations").and_then(Value::as_object)
            .ok_or_else(|| format!("Punchy {label}: missing animations object"))?;
        let destination = animation_doc.get_mut("animations").and_then(Value::as_object_mut)
            .ok_or("Bundled Punchy animations are missing their animations object")?;
        for (name, clip) in incoming {
            match destination.get_mut(name) {
                Some(existing) => merge_json(existing, clip),
                None => { destination.insert(name.clone(), clip.clone()); }
            }
        }
        has_pack_data = true;
    }
    result.animations = serde_json::to_vec(&animation_doc).map_err(|e| e.to_string())?;
    if let Some(bytes) = geometry.filter(|b| !b.is_empty()) {
        result.bones = parse_bones(bytes).map_err(|e| format!("Punchy geometry: {e}"))?;
        has_pack_data = true;
    }
    if let Some(bytes) = tuning.filter(|b| !b.is_empty()) {
        let incoming: Value = serde_json::from_slice(bytes).map_err(|e| format!("Punchy tuning: {e}"))?;
        merge_json(&mut result.tuning, &incoming);
        has_pack_data = true;
    }
    Ok((result, has_pack_data))
}

fn merge_json(destination: &mut Value, incoming: &Value) {
    match (destination, incoming) {
        (Value::Object(destination), Value::Object(incoming)) => {
            for (key, value) in incoming {
                match destination.get_mut(key) {
                    Some(existing) => merge_json(existing, value),
                    None => { destination.insert(key.clone(), value.clone()); }
                }
            }
        }
        (destination, incoming) => *destination = incoming.clone(),
    }
}

pub fn member(path: &Path, name: &str) -> Result<Vec<u8>, String> {
    let file = File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    let mut entry = zip.by_name(name).map_err(|e| format!("{name}: {e}"))?;
    if entry.size() > MAX_MEMBER {
        return Err(format!("{name} exceeds 32 MiB"));
    }
    let mut bytes = Vec::new();
    entry
        .by_ref()
        .take(MAX_MEMBER + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_MEMBER {
        return Err(format!("{name} exceeds 32 MiB"));
    }
    Ok(bytes)
}
pub fn load(path: &Path) -> Result<Resources, String> {
    let manifest: Value =
        serde_json::from_slice(&member(path, "fabric.mod.json")?).map_err(|e| e.to_string())?;
    if manifest["id"] != "punchy" || manifest["version"] != "3.1b" {
        return Err("Expected the analyzed Punchy 3.1b JAR".into());
    }
    let bones = parse_bones(&member(path, GEOMETRY)?)?;
    let tuning = serde_json::from_slice(&member(
        path,
        "resourcepacks/punchy/assets/minecraft/punchy/compat/tuning_template.json",
    )?)
    .map_err(|e| e.to_string())?;
    Ok(Resources {
        animations: member(path, ANIMATIONS)?,
        bones,
        tuning,
    })
}
fn vec3(v: Option<&Value>) -> Result<Vec3, String> {
    let Some(v) = v else { return Ok(Vec3::ZERO) };
    let a = v.as_array().ok_or("Expected vector array")?;
    if a.len() != 3 {
        return Err("Expected three vector components".into());
    }
    let mut r = [0.; 3];
    for (i, x) in a.iter().enumerate() {
        let n = x.as_f64().ok_or("Expected numeric component")? as f32;
        if !n.is_finite() {
            return Err("Non-finite bone component".into());
        }
        r[i] = n;
    }
    Ok(Vec3::from_array(r))
}
pub fn parse_bones(bytes: &[u8]) -> Result<BTreeMap<String, Bone>, String> {
    let json: Value = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    let bones = json["minecraft:geometry"][0]["bones"]
        .as_array()
        .ok_or("Missing geometry bones")?;
    let mut pivots = BTreeMap::new();
    for b in bones {
        let name = b["name"].as_str().ok_or("Missing bone name")?;
        if pivots.insert(name, vec3(b.get("pivot"))?).is_some() {
            return Err(format!("Duplicate bone {name}"));
        }
    }
    let mut result = BTreeMap::new();
    for b in bones {
        let name = b["name"].as_str().unwrap();
        let pivot = pivots[name];
        let parent = b.get("parent").and_then(Value::as_str);
        let position = if let Some(parent) = parent {
            let p = pivots
                .get(parent)
                .ok_or_else(|| format!("Missing parent {parent}"))?;
            // The double Z inversion is present in the shipped loader.
            Vec3::new(pivot.x - p.x, p.y - pivot.y, pivot.z - p.z)
        } else {
            Vec3::new(pivot.x, 28. - pivot.y, -pivot.z)
        };
        result.insert(
            name.to_owned(),
            Bone {
                name: name.to_owned(),
                parent: parent.map(str::to_owned),
                position,
                rotation: vec3(b.get("rotation"))?,
            },
        );
    }
    for name in result.keys() {
        let mut current = Some(name.as_str());
        let mut visited = std::collections::BTreeSet::new();
        while let Some(n) = current {
            if !visited.insert(n) {
                return Err(format!("Cyclic bone parent at {n}"));
            }
            current = result[n].parent.as_deref();
        }
    }
    Ok(result)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn embedded_resources_contain_full_analyzed_default_animation_set() {
        let r = embedded().unwrap();
        let clips = crate::load_clips(&r.animations).unwrap();
        assert_eq!(clips.len(), 223);
        assert_eq!(r.bones.len(), 17);
        let tuning = crate::tuning::Tuning::parse(&r.tuning).unwrap();
        let sword = tuning
            .animation("minecraft:diamond_sword", "attack", false)
            .unwrap();
        assert_eq!(sword.variations, vec!["attack_1", "attack_2", "attack_3"]);
        assert!(clips["use_shield"].hold_on_last_frame);
    }

    #[test]
    fn resource_pack_overrides_named_clips_and_keeps_bundled_defaults() {
        let overlay = br#"{"animations":{"custom_walk":{"animation_length":2,"bones":{"right_arm":{"rotation":[1,2,3]}}}}}"#;
        let (loaded, used) = from_active_pack(Some(overlay), None, None, None, None).unwrap();
        assert!(used);
        let clips = crate::load_clips(&loaded.animations).unwrap();
        assert!(clips.contains_key("custom_walk"));
        assert!(clips.contains_key("attack_1"));
    }

    #[test]
    fn resource_pack_tree_loads_all_namespaces_and_merges_compat_profiles() {
        let animation_files = vec![
            ("assets/minecraft/punchy/animations/nested/second.json".into(), br#"{"animations":{"tree_collision":{"animation_length":2,"bones":{"right_arm":{"rotation":[2,0,0]}}},"tree_second":{"animation_length":1}}}"#.to_vec(), 5),
            ("assets/punchy/animations/first.json".into(), br#"{"animations":{"tree_collision":{"animation_length":1,"bones":{"right_arm":{"rotation":[1,0,0]}}},"tree_first":{"animation_length":1}}}"#.to_vec(), 1),
            ("assets/minecraft/punchy/animations/shared.json".into(), br#"{"animations":{"tree_same_path":{"animation_length":3}}}"#.to_vec(), 8),
            ("assets/minecraft/punchy/animations/shared.json".into(), br#"{"animations":{"tree_same_path":{"animation_length":1}}}"#.to_vec(), 2),
        ];
        let compat_files = vec![
            ("assets/minecraft/punchy/compat/first_profile.json".into(), br#"{"resource_pack_merge_test":{"priority":1,"first":true},"customGroups":[{"name":"Example","value":1}] }"#.to_vec(), 1),
            ("assets/punchy/compat/nested/second_profile.json".into(), br#"{"resource_pack_merge_test":{"priority":2,"second":true},"customGroups":[{"name":"example","value":2},{"name":"Other","value":3}] }"#.to_vec(), 5),
            ("assets/minecraft/punchy/compat/stacked_profile.json".into(), br#"{"same_path_profile":{"winner":"high","from_high":true}}"#.to_vec(), 9),
            ("assets/minecraft/punchy/compat/stacked_profile.json".into(), br#"{"same_path_profile":{"winner":"low","from_low":true}}"#.to_vec(), 3),
        ];
        let (loaded, used) = from_active_pack_tree(&animation_files, None, None, &compat_files).unwrap();
        assert!(used);
        let clips = crate::load_clips(&loaded.animations).unwrap();
        assert!(clips.contains_key("tree_first"));
        assert!(clips.contains_key("tree_second"));
        assert_eq!(clips["tree_collision"].length, 2.0);
        assert_eq!(clips["tree_same_path"].length, 3.0);
        assert_eq!(loaded.tuning["resource_pack_merge_test"]["priority"], 2);
        assert_eq!(loaded.tuning["same_path_profile"]["winner"], "high");
        assert_eq!(loaded.tuning["same_path_profile"]["from_high"], true);
        assert_eq!(loaded.tuning["same_path_profile"]["from_low"], true);
        assert_eq!(loaded.tuning["resource_pack_merge_test"]["first"], true);
        assert_eq!(loaded.tuning["resource_pack_merge_test"]["second"], true);
        let groups = loaded.tuning["customGroups"].as_array().unwrap();
        assert_eq!(groups.iter().filter(|v| v["name"].as_str().is_some_and(|n| n.eq_ignore_ascii_case("Example"))).count(), 1);
        assert_eq!(groups.iter().filter(|v| v["name"].as_str() == Some("Other")).count(), 1);
        assert_eq!(groups.iter().find(|v| v["name"].as_str().is_some_and(|n| n.eq_ignore_ascii_case("Example"))).unwrap()["value"], 2);
    }

    #[test]
    fn external_default_tuning_replaces_tuning_template_profile() {
        let compat_files = vec![
            ("assets/minecraft/punchy/compat/default_tuning.json".into(), br#"{"external_defaults_marker":true}"#.to_vec(), 1),
            ("assets/minecraft/punchy/compat/tuning_template.json".into(), br#"{"template_marker":true}"#.to_vec(), -1),
            ("assets/punchy/compat/tuning_template.json".into(), br#"{"external_template_marker":true}"#.to_vec(), 2),
        ];
        let (loaded, used) = from_active_pack_tree(&[], None, None, &compat_files).unwrap();
        assert!(used);
        assert_eq!(loaded.tuning["external_defaults_marker"], true);
        assert!(loaded.tuning.get("template_marker").is_none());
        assert_eq!(loaded.tuning["external_template_marker"], true);
    }

    #[test]
    fn missing_resource_pack_assets_keep_embedded_resources() {
        let base = embedded().unwrap();
        let (loaded, used) = from_active_pack(None, None, None, None, None).unwrap();
        assert!(!used);
        assert_eq!(loaded.animations, base.animations);
        assert_eq!(loaded.tuning, base.tuning);
        assert_eq!(loaded.bones.len(), base.bones.len());
    }

    #[test]
    fn geometry_coordinates_and_parent_validation() {
        let b=parse_bones(br#"{"minecraft:geometry":[{"bones":[{"name":"camera","pivot":[0,28,0]},{"name":"body","parent":"camera","pivot":[0,12,0]},{"name":"right_arm","parent":"body","pivot":[-11,16.6,-7]}]}]}"#).unwrap();
        assert_eq!(b["body"].position, Vec3::new(0., 16., 0.));
        assert!((b["right_arm"].position - Vec3::new(-11., -4.6, -7.)).length() < 0.0001);
        assert!(
            parse_bones(br#"{"minecraft:geometry":[{"bones":[{"name":"a","parent":"a"}]}]}"#)
                .is_err()
        );
        assert!(
            parse_bones(br#"{"minecraft:geometry":[{"bones":[{"name":"a","parent":"missing"}]}]}"#)
                .is_err()
        );
    }
}
