//! Runtime data is compiled into the native mod. JAR access below is for development oracles only.
use glam::{EulerRot, Mat4, Quat, Vec3};
use serde_json::Value;
use std::{collections::BTreeMap, fs::File, io::Read, path::Path};

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
/// All data required by the implemented animation/renderer slice is linked into
/// the native executable. No archive, Java runtime or config file is opened.
pub fn embedded() -> Result<Resources, String> {
    let animations = include_bytes!("data/animations.json");
    let geometry = include_bytes!("data/geometry.json");
    let tuning = include_bytes!("data/tuning.json");
    Ok(Resources {
        animations: animations.to_vec(),
        bones: parse_bones(geometry)?,
        tuning: serde_json::from_slice(tuning).map_err(|e| e.to_string())?,
    })
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
