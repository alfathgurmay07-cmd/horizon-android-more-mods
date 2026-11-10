//! Punchy 3.1b animation data semantics, recovered from its shipped JVM classes.
//! Source anchors: PunchyAnimationLoader, AnimationExpression, KeyEasing,
//! PoseHandler's sample*Channel and quaternion helpers, AnimationSoundCursor.
use glam::{EulerRot, Mat4, Quat, Vec3};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug)]
enum Expression {
    Number(f64),
    Time,
    Unary(bool, Box<Self>),
    Binary(u8, Box<Self>, Box<Self>),
    Function(String, Vec<Self>),
}
impl Expression {
    fn compile(text: &str) -> Result<Self, String> {
        let mut p = Parser {
            s: text.as_bytes(),
            i: 0,
            depth: 0,
        };
        let n = p.expr()?;
        p.ws();
        if p.i != p.s.len() {
            return Err(format!("trailing expression input: {text}"));
        }
        Ok(n)
    }
    fn eval(&self, t: f64) -> f64 {
        match self {
            Self::Number(n) => *n,
            Self::Time => t,
            Self::Unary(neg, n) => {
                if *neg {
                    -n.eval(t)
                } else {
                    n.eval(t)
                }
            }
            Self::Binary(op, a, b) => {
                let (a, b) = (a.eval(t), b.eval(t));
                match op {
                    b'+' => a + b,
                    b'-' => a - b,
                    b'*' => a * b,
                    b'/' => {
                        if b == 0.0 {
                            0.0
                        } else {
                            a / b
                        }
                    }
                    _ => 0.0,
                }
            }
            Self::Function(name, args) => {
                let v: Vec<f64> = args.iter().map(|n| n.eval(t)).collect();
                let x = v.first().copied().unwrap_or(0.0);
                match name.as_str() {
                    "sin" => x.to_radians().sin(),
                    "cos" => x.to_radians().cos(),
                    "tan" => x.to_radians().tan(),
                    "abs" => x.abs(),
                    "sqrt" => x.max(0.0).sqrt(),
                    "floor" => x.floor(),
                    "ceil" => x.ceil(),
                    "round" => x.round_ties_even(),
                    "min" => v.into_iter().reduce(f64::min).unwrap_or(0.0),
                    "max" => v.into_iter().reduce(f64::max).unwrap_or(0.0),
                    "clamp" if v.len() >= 3 => v[1].max(v[2].min(x)),
                    "clamp" => x,
                    _ => 0.0,
                }
            }
        }
    }
}
struct Parser<'a> {
    s: &'a [u8],
    i: usize,
    depth: usize,
}
impl Parser<'_> {
    fn ws(&mut self) {
        while self.s.get(self.i).is_some_and(u8::is_ascii_whitespace) {
            self.i += 1;
        }
    }
    fn take(&mut self, c: u8) -> bool {
        self.ws();
        if self.s.get(self.i) == Some(&c) {
            self.i += 1;
            true
        } else {
            false
        }
    }
    fn expr(&mut self) -> Result<Expression, String> {
        self.binary(false)
    }
    fn binary(&mut self, term: bool) -> Result<Expression, String> {
        let mut a = if term {
            self.unary()?
        } else {
            self.binary(true)?
        };
        loop {
            self.ws();
            let Some(&op) = self.s.get(self.i) else { break };
            if !(if term {
                op == b'*' || op == b'/'
            } else {
                op == b'+' || op == b'-'
            }) {
                break;
            }
            self.i += 1;
            let b = if term {
                self.unary()?
            } else {
                self.binary(true)?
            };
            a = Expression::Binary(op, Box::new(a), Box::new(b));
        }
        Ok(a)
    }
    fn unary(&mut self) -> Result<Expression, String> {
        self.depth += 1;
        if self.depth > 128 {
            return Err("expression nesting exceeds 128".into());
        }
        let n = if self.take(b'-') {
            Expression::Unary(true, Box::new(self.unary()?))
        } else if self.take(b'+') {
            Expression::Unary(false, Box::new(self.unary()?))
        } else {
            self.primary()?
        };
        self.depth -= 1;
        Ok(n)
    }
    fn primary(&mut self) -> Result<Expression, String> {
        self.ws();
        if self.take(b'(') {
            let n = self.expr()?;
            if !self.take(b')') {
                return Err("missing closing parenthesis".into());
            }
            return Ok(n);
        }
        let start = self.i;
        if self
            .s
            .get(self.i)
            .is_some_and(|c| c.is_ascii_digit() || *c == b'.')
        {
            while self
                .s
                .get(self.i)
                .is_some_and(|c| c.is_ascii_digit() || *c == b'.')
            {
                self.i += 1
            }
            if self.s.get(self.i).is_some_and(|c| *c == b'e' || *c == b'E') {
                self.i += 1;
                if self.s.get(self.i).is_some_and(|c| *c == b'+' || *c == b'-') {
                    self.i += 1
                }
                while self.s.get(self.i).is_some_and(u8::is_ascii_digit) {
                    self.i += 1
                }
            }
            return Ok(Expression::Number(
                std::str::from_utf8(&self.s[start..self.i])
                    .unwrap()
                    .parse()
                    .unwrap_or(0.0),
            ));
        }
        if !self
            .s
            .get(self.i)
            .is_some_and(|c| c.is_ascii_alphabetic() || *c == b'_')
        {
            return Err("expected expression".into());
        }
        self.i += 1;
        while self
            .s
            .get(self.i)
            .is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_' || *c == b'.')
        {
            self.i += 1
        }
        let mut name = String::from_utf8_lossy(&self.s[start..self.i]).to_lowercase();
        if name.starts_with("q.") {
            name = format!("query.{}", &name[2..])
        }
        if name.starts_with("math.") {
            name = name[5..].to_owned()
        }
        if self.take(b'(') {
            let mut args = Vec::new();
            if !self.take(b')') {
                loop {
                    args.push(self.expr()?);
                    if self.take(b')') {
                        break;
                    }
                    if !self.take(b',') {
                        return Err("expected comma".into());
                    }
                }
            }
            Ok(Expression::Function(name, args))
        } else {
            Ok(match name.as_str() {
                "query.anim_time" | "query.animtime" => Expression::Time,
                "pi" => Expression::Number(std::f64::consts::PI),
                "e" => Expression::Number(std::f64::consts::E),
                _ => Expression::Number(0.0),
            })
        }
    }
}

#[derive(Clone, Debug)]
struct VectorValue {
    base: Vec3,
    expressions: [Option<Expression>; 3],
}
impl VectorValue {
    fn parse(v: &Value, fallback: f32) -> Result<Self, String> {
        let v = if v.is_object() {
            v.get("post")
                .or_else(|| v.get("vector"))
                .or_else(|| v.get("pre"))
                .unwrap_or(v)
        } else {
            v
        };
        let mut result = Self {
            base: Vec3::splat(fallback),
            expressions: [None, None, None],
        };
        if let Some(a) = v.as_array() {
            if a.len() != 3 {
                return Err("expected three vector components".into());
            }
            for (i, v) in a.iter().enumerate() {
                if let Some(n) = v
                    .as_f64()
                    .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
                {
                    result.base[i] = n as f32
                } else if let Some(s) = v.as_str() {
                    let e = Expression::compile(s)?;
                    result.base[i] = e.eval(0.0) as f32;
                    result.expressions[i] = Some(e)
                } else {
                    return Err("invalid vector component".into());
                }
            }
        }
        Ok(result)
    }
    fn sample(&self, time: f32) -> Vec3 {
        let mut v = self.base;
        for (i, e) in self.expressions.iter().enumerate() {
            if let Some(e) = e {
                v[i] = e.eval(time as f64) as f32
            }
        }
        v
    }
}
#[derive(Clone, Debug)]
pub struct Keyframe {
    pub time: f32,
    pub interpolation: String,
    pub easing: Option<usize>,
    pre: VectorValue,
    post: VectorValue,
}
#[derive(Clone, Debug, Default)]
pub struct BoneTrack {
    pub position: Vec<Keyframe>,
    pub rotation: Vec<Keyframe>,
    pub scale: Vec<Keyframe>,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
    pub euler: Vec3,
}
impl Default for Pose {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
            euler: Vec3::ZERO,
        }
    }
}
impl Pose {
    pub fn matrix(self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.position / 16.0)
    }
    pub fn blend(self, to: Self, weight: f32) -> Self {
        Self {
            position: self.position.lerp(to.position, weight),
            rotation: self.rotation.slerp(to.rotation, weight),
            scale: self.scale.lerp(to.scale, weight),
            euler: self.euler.lerp(to.euler, weight),
        }
    }
}
#[derive(Clone, Debug)]
pub struct Clip {
    pub name: String,
    pub length: f32,
    pub looped: bool,
    pub hold_on_last_frame: bool,
    pub length_defined: bool,
    pub loop_defined: bool,
    pub tracks: BTreeMap<String, BoneTrack>,
    pub effects: BTreeSet<String>,
    pub timeline: Vec<(f32, BTreeSet<String>)>,
    pub sounds: Vec<(f32, BTreeSet<String>)>,
}
impl Clip {
    pub fn local_time(&self, time: f32) -> f32 {
        if self.looped && self.length > 0.0 {
            time.rem_euclid(self.length)
        } else if self.length > 0.0 {
            time.clamp(0.0, self.length)
        } else {
            time.max(0.0)
        }
    }
    pub fn effect_active(&self, effect: &str, time: f32) -> bool {
        self.effects.contains(effect)
            || self
                .timeline
                .iter()
                .any(|(t, e)| *t <= time && e.contains(effect))
    }
    pub fn has_effect(&self, effect: &str) -> bool {
        self.effects.contains(effect) || self.timeline.iter().any(|(_, e)| e.contains(effect))
    }
    pub fn active_effects(&self, time: f32) -> BTreeSet<String> {
        let mut out = self.effects.clone();
        for (t, e) in &self.timeline {
            if *t <= time {
                out.extend(e.iter().cloned())
            }
        }
        out
    }
    pub fn effect_triggered_between(&self, effect: &str, start: f32, end: f32) -> bool {
        self.timeline.iter().any(|(t, e)| {
            e.contains(effect)
                && if end < start {
                    *t > start || *t <= end
                } else {
                    *t > start && *t <= end
                }
        })
    }
    pub fn sample(&self, bone: &str, time: f32, expression_time: f32) -> Pose {
        self.sample_with_java_block(bone, time, expression_time, false)
    }
    pub fn sample_with_java_block(
        &self,
        bone: &str,
        time: f32,
        expression_time: f32,
        java_block: bool,
    ) -> Pose {
        let Some(track) = self.tracks.get(bone) else {
            return Pose::default();
        };
        let v2 = self.effect_active("use_keyframes_v2", time);
        let raw_position = sample_channel(
            &track.position,
            time,
            expression_time,
            Vec3::ZERO,
            false,
            false,
        );
        let position = if v2 {
            Vec3::new(raw_position.x, -raw_position.y, raw_position.z)
        } else if bone.to_lowercase().contains("itemgrip") {
            Vec3::new(-raw_position.x, -raw_position.z, raw_position.y)
        } else {
            Vec3::new(raw_position.x, -raw_position.z, -raw_position.y)
        };
        let mut scale =
            sample_channel(&track.scale, time, expression_time, Vec3::ONE, false, false);
        if bone.to_lowercase().contains("item") {
            scale = Vec3::new(scale.x, scale.z, scale.y)
        }
        let raw_euler =
            sample_channel(&track.rotation, time, expression_time, Vec3::ZERO, true, v2);
        let euler = if v2 {
            raw_euler
        } else {
            Vec3::new(raw_euler.x, -raw_euler.z, raw_euler.y)
        };
        let rotation = if v2 {
            quat_xyz(euler)
        } else if java_block {
            quat_xyz(Vec3::new(-euler.x, -euler.z, -euler.y))
        } else {
            let xyz = !self.effect_active("rotation_zyx", time)
                && !self.effect_active("rot_zyx", time)
                && (self.effect_active("rotation_xyz", time)
                    || self.effect_active("rot_xyz", time)
                    || ["mace", "trident", "crossbow"]
                        .iter()
                        .any(|s| self.name.contains(s)));
            if xyz {
                quat_xyz(euler)
            } else {
                Quat::from_euler(
                    EulerRot::ZYX,
                    euler.z.to_radians(),
                    euler.y.to_radians(),
                    euler.x.to_radians(),
                )
            }
        };
        Pose {
            position,
            rotation,
            scale,
            euler,
        }
    }
    pub fn merge(&self, overlay: &Self) -> Self {
        let mut result = self.clone();
        if !overlay.name.trim().is_empty() {
            result.name = overlay.name.clone()
        }
        if overlay.length_defined {
            result.length = overlay.length
        }
        if overlay.loop_defined {
            result.looped = overlay.looped;
            result.hold_on_last_frame = overlay.hold_on_last_frame
        }
        result.length_defined |= overlay.length_defined;
        result.loop_defined |= overlay.loop_defined;
        result.tracks.extend(overlay.tracks.clone());
        result.effects.extend(overlay.effects.clone());
        merge_timed(&mut result.timeline, &overlay.timeline);
        merge_timed(&mut result.sounds, &overlay.sounds);
        result
    }
}
fn quat_xyz(e: Vec3) -> Quat {
    Quat::from_euler(
        EulerRot::XYZ,
        e.x.to_radians(),
        e.y.to_radians(),
        e.z.to_radians(),
    )
}
fn euler_xyz(q: Quat) -> Vec3 {
    let (x, y, z) = q.to_euler(EulerRot::XYZ);
    Vec3::new(x, y, z) * (180.0 / std::f32::consts::PI)
}
fn catmull(a: Vec3, b: Vec3, c: Vec3, d: Vec3, t: f32) -> Vec3 {
    let t2 = t * t;
    let t3 = t * t2;
    b * (2.0 * t3 - 3.0 * t2 + 1.0)
        + c * (-2.0 * t3 + 3.0 * t2)
        + (c - a) * 0.5 * (t3 - 2.0 * t2 + t)
        + (d - b) * 0.5 * (t3 - t2)
}
fn sample_channel(
    keys: &[Keyframe],
    time: f32,
    eval: f32,
    fallback: Vec3,
    rotation: bool,
    v2: bool,
) -> Vec3 {
    if keys.is_empty() {
        return fallback;
    }
    let prev = keys.iter().rposition(|k| k.time <= time);
    let next = keys.iter().position(|k| k.time >= time);
    let (Some(i), Some(j)) = (prev, next) else {
        return if let Some(i) = prev {
            keys[i].post.sample(eval)
        } else if rotation && v2 {
            keys[next.unwrap()].pre.sample(eval)
        } else {
            keys[next.unwrap()].post.sample(eval)
        };
    };
    let (a, b) = (&keys[i], &keys[j]);
    if i == j || (rotation && v2 && (a.time - b.time).abs() <= 1e-4) {
        return if rotation && v2 {
            a.pre.sample(eval)
        } else {
            a.post.sample(eval)
        };
    }
    let progress = ((time - a.time) / (b.time - a.time)).clamp(0.0, 1.0);
    let eased = b
        .easing
        .map_or(progress, |c| ease(c, progress as f64) as f32);
    let from = a.post.sample(eval);
    let to = if rotation && v2 {
        b.pre.sample(eval)
    } else {
        b.post.sample(eval)
    };
    let same = (from - to).abs().max_element() <= 1e-4;
    if same {
        return from;
    }
    let mode = |s: &str| a.interpolation == s || b.interpolation == s;
    if rotation && v2 && (mode("spherical_linear") || mode("spherical_squad")) {
        let q1 = quat_xyz(from);
        let q2 = quat_xyz(to);
        if !(0.0..=1.0).contains(&eased) || !mode("spherical_squad") {
            return euler_xyz(q1.slerp(q2, eased));
        }
        let q0 = quat_xyz(keys[i.saturating_sub(1)].post.sample(eval));
        let q3 = quat_xyz(keys[(j + 1).min(keys.len() - 1)].pre.sample(eval));
        return euler_xyz(squad(q0, q1, q2, q3, eased));
    }
    if mode("catmullrom") {
        let p0 = keys[i.saturating_sub(1)].post.sample(eval);
        let p1 = if rotation && v2 {
            a.pre.sample(eval)
        } else {
            from
        };
        let p3 = if rotation && v2 {
            keys[(j + 1).min(keys.len() - 1)].pre.sample(eval)
        } else {
            keys[(j + 1).min(keys.len() - 1)].post.sample(eval)
        };
        return catmull(p0, p1, to, p3, eased);
    }
    if rotation && !v2 {
        let mut delta = to - from;
        for d in delta.as_mut() {
            if d.abs() > 180.0 && d.abs() < 359.9999 {
                *d -= d.signum() * 360.0
            }
        }
        from + delta * eased
    } else {
        from.lerp(to, eased)
    }
}
fn qlog(q: Quat) -> Vec3 {
    let v = Vec3::new(q.x, q.y, q.z);
    let s = v.length();
    if s > 5e-4 { v * (s.atan2(q.w) / s) } else { v }
}
fn qexp(v: Vec3) -> Quat {
    let theta = v.length();
    let c = theta.cos();
    let v = if c < 0.9995 && theta > 1e-7 {
        v * (theta.sin() / theta)
    } else {
        v
    };
    Quat::from_xyzw(v.x, v.y, v.z, c)
}
fn intermediate(mut a: Quat, b: Quat, mut c: Quat) -> Quat {
    if b.dot(a) < 0.0 {
        a = -a
    }
    if b.dot(c) < 0.0 {
        c = -c
    }
    b * qexp((qlog(a.conjugate() * b) - qlog(b.conjugate() * c)) * 0.25)
}
fn squad(a: Quat, b: Quat, c: Quat, d: Quat, t: f32) -> Quat {
    b.slerp(c, t).slerp(
        intermediate(a, b, c).slerp(intermediate(b, c, d), t),
        2.0 * t * (1.0 - t),
    )
}

pub const EASING_NAMES: [&str; 30] = [
    "easeInSine",
    "easeOutSine",
    "easeInOutSine",
    "easeInQuad",
    "easeOutQuad",
    "easeInOutQuad",
    "easeInCubic",
    "easeOutCubic",
    "easeInOutCubic",
    "easeInQuart",
    "easeOutQuart",
    "easeInOutQuart",
    "easeInQuint",
    "easeOutQuint",
    "easeInOutQuint",
    "easeInExpo",
    "easeOutExpo",
    "easeInOutExpo",
    "easeInCirc",
    "easeOutCirc",
    "easeInOutCirc",
    "easeInBack",
    "easeOutBack",
    "easeInOutBack",
    "easeInElastic",
    "easeOutElastic",
    "easeInOutElastic",
    "easeInBounce",
    "easeOutBounce",
    "easeInOutBounce",
];
pub fn named_easing(name: &str) -> Option<usize> {
    let n = name.trim().replace(['_', '-', ' '], "").to_lowercase();
    EASING_NAMES.iter().position(|s| s.to_lowercase() == n)
}
fn bounce(x: f64) -> f64 {
    if x < 1.0 / 2.75 {
        7.5625 * x * x
    } else if x < 2.0 / 2.75 {
        7.5625 * (x - 1.5 / 2.75).powi(2) + 0.75
    } else if x < 2.5 / 2.75 {
        7.5625 * (x - 2.25 / 2.75).powi(2) + 0.9375
    } else {
        7.5625 * (x - 2.625 / 2.75).powi(2) + 0.984375
    }
}
pub fn ease(curve: usize, x: f64) -> f64 {
    use std::f64::consts::PI;
    match curve {
        0 => 1.0 - (x * PI / 2.0).cos(),
        1 => (x * PI / 2.0).sin(),
        2 => -((PI * x).cos() - 1.0) / 2.0,
        3..=14 => {
            let n = 2 + (curve - 3) / 3;
            match (curve - 3) % 3 {
                0 => x.powi(n as i32),
                1 => 1.0 - (1.0 - x).powi(n as i32),
                _ => {
                    if x < 0.5 {
                        2.0_f64.powi(n as i32 - 1) * x.powi(n as i32)
                    } else {
                        1.0 - (-2.0 * x + 2.0).powi(n as i32) / 2.0
                    }
                }
            }
        }
        15 => {
            if x == 0.0 {
                0.0
            } else {
                2.0_f64.powf(10.0 * x - 10.0)
            }
        }
        16 => {
            if x == 1.0 {
                1.0
            } else {
                1.0 - 2.0_f64.powf(-10.0 * x)
            }
        }
        17 => {
            if x == 0.0 || x == 1.0 {
                x
            } else if x < 0.5 {
                2.0_f64.powf(20.0 * x - 10.0) / 2.0
            } else {
                (2.0 - 2.0_f64.powf(-20.0 * x + 10.0)) / 2.0
            }
        }
        18 => 1.0 - (1.0 - x * x).sqrt(),
        19 => (1.0 - (x - 1.0).powi(2)).sqrt(),
        20 => {
            if x < 0.5 {
                (1.0 - (1.0 - (2.0 * x).powi(2)).sqrt()) / 2.0
            } else {
                ((1.0 - (-2.0 * x + 2.0).powi(2)).sqrt() + 1.0) / 2.0
            }
        }
        21 => 2.70158 * x * x * x - 1.70158 * x * x,
        22 => 1.0 + 2.70158 * (x - 1.0).powi(3) + 1.70158 * (x - 1.0).powi(2),
        23 => {
            if x < 0.5 {
                (2.0 * x).powi(2) * (7.189819 * x - 2.5949095) / 2.0
            } else {
                ((2.0 * x - 2.0).powi(2) * (3.5949095 * (2.0 * x - 2.0) + 2.5949095) + 2.0) / 2.0
            }
        }
        24 => {
            if x == 0.0 || x == 1.0 {
                x
            } else {
                -2.0_f64.powf(10.0 * x - 10.0) * ((10.0 * x - 10.75) * (2.0 * PI / 3.0)).sin()
            }
        }
        25 => {
            if x == 0.0 || x == 1.0 {
                x
            } else {
                2.0_f64.powf(-10.0 * x) * ((10.0 * x - 0.75) * (2.0 * PI / 3.0)).sin() + 1.0
            }
        }
        26 => {
            if x == 0.0 || x == 1.0 {
                x
            } else if x < 0.5 {
                -2.0_f64.powf(20.0 * x - 10.0) * ((20.0 * x - 11.125) * (4.0 * PI / 9.0)).sin()
                    / 2.0
            } else {
                2.0_f64.powf(-20.0 * x + 10.0) * ((20.0 * x - 11.125) * (4.0 * PI / 9.0)).sin()
                    / 2.0
                    + 1.0
            }
        }
        27 => 1.0 - bounce(1.0 - x),
        28 => bounce(x),
        29 => {
            if x < 0.5 {
                (1.0 - bounce(1.0 - 2.0 * x)) / 2.0
            } else {
                (1.0 + bounce(2.0 * x - 1.0)) / 2.0
            }
        }
        _ => x,
    }
}
fn parse_channel(value: Option<&Value>, fallback: f32) -> Result<Vec<Keyframe>, String> {
    let Some(value) = value else {
        return Ok(vec![]);
    };
    let mut keys = Vec::new();
    let mut add = |time: f32, v: &Value| -> Result<(), String> {
        let raw = VectorValue::parse(v, fallback)?;
        let pre = if let Some(p) = v.get("pre").or_else(|| v.get("post")) {
            VectorValue::parse(p, fallback)?
        } else {
            raw.clone()
        };
        let post = if let Some(p) = v.get("post").or_else(|| v.get("pre")) {
            VectorValue::parse(p, fallback)?
        } else {
            raw
        };
        let easing = v
            .get("easing")
            .and_then(Value::as_str)
            .and_then(named_easing);
        let interpolation = v
            .get("lerp_mode")
            .or_else(|| v.get("easing"))
            .and_then(Value::as_str)
            .unwrap_or("linear")
            .trim()
            .to_lowercase();
        keys.push(Keyframe {
            time,
            interpolation: if easing.is_some() && v.get("lerp_mode").is_none() {
                "linear".into()
            } else {
                interpolation
            },
            easing,
            pre,
            post,
        });
        Ok(())
    };
    if value.is_array() {
        add(0.0, value)?
    } else if let Some(map) = value.as_object() {
        for (t, v) in map {
            let time = t
                .parse::<f32>()
                .map_err(|_| format!("invalid channel time {t}"))?;
            if !time.is_finite() {
                return Err("non-finite key time".into());
            }
            add(time, v)?
        }
    }
    keys.sort_by(|a, b| a.time.total_cmp(&b.time));
    Ok(keys)
}
fn strings(v: &Value, out: &mut BTreeSet<String>) {
    match v {
        Value::String(s) => out.extend(
            s.split(';')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_lowercase),
        ),
        Value::Array(a) => {
            for v in a {
                strings(v, out)
            }
        }
        Value::Object(m) => {
            for v in m.values() {
                strings(v, out)
            }
        }
        _ => {}
    }
}
fn parse_timed(value: Option<&Value>, sound: bool) -> Vec<(f32, BTreeSet<String>)> {
    let mut out = Vec::new();
    if let Some(map) = value.and_then(Value::as_object) {
        for (t, v) in map {
            let time = t.parse::<f32>().unwrap_or(0.0);
            if !time.is_finite() {
                continue;
            }
            let mut values = BTreeSet::new();
            if sound {
                if let Some(s) = v
                    .as_str()
                    .or_else(|| v.get("effect").and_then(Value::as_str))
                    && !s.trim().is_empty()
                {
                    values.insert(s.trim().to_owned());
                }
            } else {
                strings(v, &mut values)
            }
            if !values.is_empty() {
                out.push((time, values))
            }
        }
    }
    out.sort_by(|a, b| a.0.total_cmp(&b.0));
    out
}
fn merge_timed(target: &mut Vec<(f32, BTreeSet<String>)>, overlay: &[(f32, BTreeSet<String>)]) {
    for (t, e) in overlay {
        if let Some((_, set)) = target.iter_mut().find(|(x, _)| *x == *t) {
            set.extend(e.iter().cloned())
        } else {
            target.push((*t, e.clone()))
        }
    }
    target.sort_by(|a, b| a.0.total_cmp(&b.0));
}
pub fn load_clips(bytes: &[u8]) -> Result<BTreeMap<String, Clip>, String> {
    let json: Value = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    let mut out = BTreeMap::new();
    let Some(animations) = json.get("animations").and_then(Value::as_object) else {
        return Ok(out);
    };
    for (name, value) in animations {
        let Some(data) = value.as_object() else {
            continue;
        };
        let loop_value = data.get("loop");
        let loop_text = loop_value
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_lowercase();
        let mut effects = BTreeSet::new();
        for key in ["effects", "effect", "events", "event"] {
            if let Some(v) = data.get(key) {
                strings(v, &mut effects)
            }
        }
        let mut clip = Clip {
            name: name.clone(),
            length: data
                .get("animation_length")
                .and_then(Value::as_f64)
                .unwrap_or(0.0) as f32,
            looped: loop_value.and_then(Value::as_bool).unwrap_or(false)
                || loop_text == "loop"
                || loop_text == "true",
            hold_on_last_frame: loop_text == "hold_on_last_frame",
            length_defined: data.contains_key("animation_length"),
            loop_defined: data.contains_key("loop"),
            tracks: BTreeMap::new(),
            effects,
            timeline: parse_timed(data.get("timeline"), false),
            sounds: parse_timed(data.get("sound_effects"), true),
        };
        if let Some(bones) = data.get("bones").and_then(Value::as_object) {
            for (bone, v) in bones {
                let mut track = BoneTrack {
                    position: parse_channel(v.get("position"), 0.0)
                        .map_err(|e| format!("{name}/{bone}/position: {e}"))?,
                    rotation: parse_channel(v.get("rotation"), 0.0)
                        .map_err(|e| format!("{name}/{bone}/rotation: {e}"))?,
                    scale: parse_channel(v.get("scale"), 1.0)
                        .map_err(|e| format!("{name}/{bone}/scale: {e}"))?,
                };
                // The Java loader uses a shared builder by timestamp: the last channel's
                // interpolation annotation wins even when sampling another channel.
                let mut annotations = Vec::new();
                for channel in [v.get("rotation"), v.get("position"), v.get("scale")]
                    .into_iter()
                    .flatten()
                {
                    if let Some(map) = channel.as_object() {
                        for (t, key) in map {
                            if key.get("lerp_mode").is_some() || key.get("easing").is_some() {
                                let parsed = parse_channel(Some(&serde_json::json!({t:key})), 0.0)?;
                                if let Some(k) = parsed.into_iter().next() {
                                    annotations.push((k.time, k.interpolation, k.easing));
                                }
                            }
                        }
                    }
                }
                for keys in [&mut track.rotation, &mut track.position, &mut track.scale] {
                    for k in keys {
                        for (t, mode, curve) in &annotations {
                            if k.time == *t {
                                k.interpolation = mode.clone();
                                k.easing = *curve
                            }
                        }
                    }
                }
                clip.tracks.insert(bone.clone(), track);
            }
        }
        out.insert(name.clone(), clip);
    }
    Ok(out)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SoundRange {
    pub start: f32,
    pub end: f32,
    pub include_start: bool,
}
#[derive(Clone, Debug, Default)]
pub struct SoundCursor {
    initialized: bool,
    last_clip: u64,
    last_playback: u64,
    last_time: f32,
    lead_in: Option<(u64, u64)>,
}
impl SoundCursor {
    pub fn reset(&mut self) {
        *self = Self::default()
    }
    pub fn include_lead_in(&mut self, clip: u64, playback: u64) {
        self.lead_in = Some((clip, playback))
    }
    pub fn advance(
        &mut self,
        clip: u64,
        playback: u64,
        entry: f32,
        time: f32,
        length: f32,
    ) -> (bool, Vec<SoundRange>) {
        let new = !self.initialized || playback != self.last_playback;
        let changed = clip != self.last_clip;
        let lead = new && self.lead_in == Some((clip, playback));
        self.lead_in = None;
        let start = if lead {
            0.0
        } else if !new && !changed {
            self.last_time
        } else {
            entry
        };
        let ranges = if time < start {
            vec![
                SoundRange {
                    start,
                    end: length,
                    include_start: false,
                },
                SoundRange {
                    start: 0.0,
                    end: time,
                    include_start: true,
                },
            ]
        } else {
            vec![SoundRange {
                start,
                end: time,
                include_start: new && start == 0.0,
            }]
        };
        self.initialized = true;
        self.last_clip = clip;
        self.last_playback = playback;
        self.last_time = time;
        (new, ranges)
    }
    pub fn sounds(
        &mut self,
        clip: &Clip,
        identity: u64,
        playback: u64,
        entry: f32,
        time: f32,
    ) -> Vec<String> {
        let (_, ranges) = self.advance(identity, playback, entry, time, clip.length);
        let mut out = Vec::new();
        for r in ranges {
            for (t, values) in &clip.sounds {
                if *t <= r.end && (*t > r.start || r.include_start && *t == r.start) {
                    out.extend(values.iter().cloned())
                }
            }
        }
        out
    }
}

/// A live pose timeline. Interrupted transitions retain the old timeline tree,
/// rather than freezing the outgoing pose at the instant of the interruption.
#[derive(Clone)]
pub struct TimelinePose {
    pub clip: std::sync::Arc<Clip>,
    pub time_offset: f32,
    pub expression_offset: f32,
}
#[derive(Clone)]
enum FadeNode {
    Leaf(TimelinePose),
    Blend {
        from: Box<Self>,
        to: Box<Self>,
        duration: f32,
        elapsed: f32,
        previous: f32,
    },
    Shift {
        source: Box<Self>,
        time: f32,
        expression: f32,
    },
    Override {
        source: Box<Self>,
        replacement: TimelinePose,
    },
}
impl FadeNode {
    fn sample(&self, bone: &str, time: f32, expression: f32, partial: f32) -> Pose {
        match self {
            Self::Leaf(p) => {
                p.clip
                    .sample(bone, time + p.time_offset, expression + p.expression_offset)
            }
            Self::Blend {
                from,
                to,
                duration,
                elapsed,
                previous,
            } => {
                let x = ((previous + (elapsed - previous) * partial) / duration).clamp(0.0, 1.0);
                from.sample(bone, time, expression, partial).blend(
                    to.sample(bone, time, expression, partial),
                    x * x * (3.0 - 2.0 * x),
                )
            }
            Self::Shift {
                source,
                time: offset,
                expression: eval,
            } => source.sample(bone, time + offset, expression + eval, partial),
            Self::Override {
                source,
                replacement,
            } => {
                if replacement.clip.tracks.contains_key(bone) {
                    replacement.clip.sample(
                        bone,
                        time + replacement.time_offset,
                        expression + replacement.expression_offset,
                    )
                } else {
                    source.sample(bone, time, expression, partial)
                }
            }
        }
    }
    fn tick(self, seconds: f32) -> Self {
        match self {
            Self::Blend {
                from,
                to,
                duration,
                elapsed,
                ..
            } => {
                let from = Box::new(from.tick(seconds));
                if elapsed >= duration {
                    *to
                } else {
                    Self::Blend {
                        from,
                        to,
                        duration,
                        elapsed: (elapsed + seconds).min(duration),
                        previous: elapsed,
                    }
                }
            }
            Self::Shift {
                source,
                time,
                expression,
            } => Self::Shift {
                source: Box::new(source.tick(seconds)),
                time,
                expression,
            },
            Self::Override {
                source,
                replacement,
            } => Self::Override {
                source: Box::new(source.tick(seconds)),
                replacement,
            },
            other => other,
        }
    }
    fn transitioning(&self) -> bool {
        match self {
            Self::Blend { .. } => true,
            Self::Shift { source, .. } | Self::Override { source, .. } => source.transitioning(),
            _ => false,
        }
    }
    fn tracks(&self, out: &mut BTreeSet<String>) {
        match self {
            Self::Leaf(p) => out.extend(p.clip.tracks.keys().cloned()),
            Self::Blend { from, to, .. } => {
                from.tracks(out);
                to.tracks(out)
            }
            Self::Shift { source, .. } => source.tracks(out),
            Self::Override {
                source,
                replacement,
            } => {
                source.tracks(out);
                out.extend(replacement.clip.tracks.keys().cloned())
            }
        }
    }
}
pub struct TimelineCrossfade {
    root: Option<FadeNode>,
}
impl TimelineCrossfade {
    pub fn new(pose: TimelinePose) -> Self {
        Self {
            root: Some(FadeNode::Leaf(pose)),
        }
    }
    pub fn retarget(&mut self, pose: TimelinePose, seconds: f32) {
        let to = FadeNode::Leaf(pose);
        self.root = Some(if seconds > 0.0 {
            FadeNode::Blend {
                from: Box::new(self.root.take().unwrap()),
                to: Box::new(to),
                duration: seconds,
                elapsed: 0.0,
                previous: 0.0,
            }
        } else {
            to
        });
    }
    pub fn replace_tracks(&mut self, pose: TimelinePose) {
        if !pose.clip.tracks.is_empty() {
            self.root = Some(FadeNode::Override {
                source: Box::new(self.root.take().unwrap()),
                replacement: pose,
            });
        }
    }
    pub fn shift_timeline(&mut self, time: f32, expression: f32) {
        if time != 0.0 || expression != 0.0 {
            self.root = Some(FadeNode::Shift {
                source: Box::new(self.root.take().unwrap()),
                time,
                expression,
            });
        }
    }
    pub fn tick(&mut self, seconds: f32) {
        self.root = Some(self.root.take().unwrap().tick(seconds.max(0.0)))
    }
    pub fn is_transitioning(&self) -> bool {
        self.root.as_ref().unwrap().transitioning()
    }
    pub fn tracks(&self) -> BTreeSet<String> {
        let mut tracks = BTreeSet::new();
        self.root.as_ref().unwrap().tracks(&mut tracks);
        tracks
    }
    pub fn sample(&self, bone: &str, time: f32, expression: f32, partial: f32) -> Pose {
        self.root
            .as_ref()
            .unwrap()
            .sample(bone, time, expression, partial.clamp(0.0, 1.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn close(actual: f32, expected: f32) {
        assert!((actual - expected).abs() < 1e-4, "{actual} != {expected}")
    }
    #[test]
    fn expressions_follow_punchy_degree_math_and_safe_division() {
        for (s, t, expected) in [
            ("math.sin(q.anim_time * 90) * 2 + 3", 1.0, 5.0),
            ("math.round(2.5)+math.round(3.5)", 0.0, 6.0),
            ("clamp(-4, -2, 9) + 9 / 0", 0.0, -2.0),
            ("sqrt(-5)+unknown_variable", 0.0, 0.0),
            ("1e-2 + 2E+1", 0.0, 20.01),
        ] {
            close(Expression::compile(s).unwrap().eval(t) as f32, expected)
        }
        assert!(Expression::compile("2 +").is_err());
    }
    #[test]
    fn easing_covers_curves_and_overshoot() {
        for curve in 0..30 {
            close(ease(curve, 0.0) as f32, 0.0);
            close(ease(curve, 1.0) as f32, 1.0);
            assert!(ease(curve, 0.4).is_finite())
        }
        close(ease(3, 0.5) as f32, 0.25);
        close(ease(7, 0.5) as f32, 0.875);
        assert!(ease(22, 0.6) > 1.0);
        assert_eq!(named_easing("ease-out-back"), Some(22));
    }
    #[test]
    fn channels_have_independent_times_and_legacy_axis_mapping() {
        let clips=load_clips(br#"{"animations":{"test":{"animation_length":2,"bones":{"rightarm":{"position":{"0":[0,0,0],"2":[16,8,4]},"rotation":{"1":[20,30,40]}}}}}}"#).unwrap();
        let p = clips["test"].sample("rightarm", 1.0, 1.0);
        assert_eq!(p.position, Vec3::new(8.0, -2.0, -4.0));
        assert_eq!(p.euler, Vec3::new(20.0, -40.0, 30.0));
    }
    #[test]
    fn v2_rotation_pre_post_discontinuity_and_effects() {
        let clips=load_clips(br#"{"animations":{"test":{"loop":"hold_on_last_frame","effects":"USE_KEYFRAMES_V2; foo","timeline":{"0.5":"BAR"},"bones":{"arm":{"position":[1,2,3],"rotation":{"0":{"pre":[0,0,0],"post":[20,0,0]},"1":{"pre":[80,0,0],"post":[100,0,0]}}}}}}}"#).unwrap();
        let c = &clips["test"];
        close(c.sample("arm", 0.0, 0.0).euler.x, 0.0);
        close(c.sample("arm", 0.5, 0.5).euler.x, 50.0);
        close(c.sample("arm", 1.0, 1.0).euler.x, 80.0);
        close(c.sample("arm", 1.1, 1.1).euler.x, 100.0);
        assert_eq!(
            c.sample("arm", 0.5, 0.5).position,
            Vec3::new(1.0, -2.0, 3.0)
        );
        assert!(c.effect_active("bar", 0.5));
        assert!(!c.effect_active("bar", 0.4));
        assert!(c.hold_on_last_frame);
    }
    #[test]
    fn sound_cursor_handles_new_playback_loops_and_repeated_frames() {
        let clips=load_clips(br#"{"animations":{"test":{"animation_length":1,"sound_effects":{"0":{"effect":"A"},"0.5":"B","0.9":"C"}}}}"#).unwrap();
        let mut cursor = SoundCursor::default();
        let c = &clips["test"];
        assert_eq!(cursor.sounds(c, 1, 1, 0.0, 0.6), ["A", "B"]);
        assert!(cursor.sounds(c, 1, 1, 0.0, 0.6).is_empty());
        assert_eq!(cursor.sounds(c, 1, 1, 0.0, 0.1), ["C", "A"]);
        assert_eq!(cursor.sounds(c, 1, 2, 0.5, 0.7), Vec::<String>::new());
        cursor.include_lead_in(1, 3);
        assert_eq!(cursor.sounds(c, 1, 3, 0.5, 0.7), ["A", "B"]);
    }
    #[test]
    fn fixture_resources_load_and_sample_finitely() {
        let workspace = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let paths = [
            "build/punchy-analysis/artifact/resourcepacks/punchy/assets/minecraft/punchy/animations/player.animation.json",
            "build/punchy-analysis/artifact/assets/punchy/animations/player_3_p.animation.json",
        ];
        let mut clips_count = 0;
        let mut samples = 0;
        for path in paths {
            let bytes=std::fs::read(workspace.join(path)).expect("extract the supplied Punchy JAR into build/punchy-analysis/artifact before running parity fixtures");
            let clips = load_clips(&bytes).unwrap();
            clips_count += clips.len();
            for clip in clips.values() {
                for bone in clip.tracks.keys() {
                    for fraction in [0.0, 0.125, 0.5, 0.875, 1.0] {
                        let time = clip.length * fraction;
                        let p = clip.sample(bone, time, time);
                        assert!(
                            p.position.is_finite() && p.scale.is_finite() && p.rotation.is_finite(),
                            "{} {bone} {time}",
                            clip.name
                        );
                        samples += 1;
                    }
                }
            }
        }
        assert!(clips_count > 100, "{clips_count}");
        assert!(samples > 1000, "{samples}");
    }
    #[test]
    fn interrupted_crossfade_preserves_live_source_and_partial_interpolation() {
        let clips=load_clips(br#"{"animations":{"a":{"bones":{"arm":{"position":[0,0,0]}}},"b":{"bones":{"arm":{"position":[10,0,0]}}},"c":{"bones":{"arm":{"position":[20,0,0]}}}}}"#).unwrap();
        let make = |name: &str| TimelinePose {
            clip: std::sync::Arc::new(clips[name].clone()),
            time_offset: 0.0,
            expression_offset: 0.0,
        };
        let mut fade = TimelineCrossfade::new(make("a"));
        fade.retarget(make("b"), 1.0);
        fade.tick(0.5);
        close(fade.sample("arm", 0.0, 0.0, 1.0).position.x, 5.0);
        fade.retarget(make("c"), 1.0);
        fade.tick(0.5);
        close(fade.sample("arm", 0.0, 0.0, 1.0).position.x, 15.0);
        close(fade.sample("arm", 0.0, 0.0, 0.0).position.x, 5.0);
    }
}

#[path = "actions.rs"]
pub mod actions;
#[path = "resources.rs"]
pub mod resources;

#[cfg(test)]
mod jar_parity {
    use super::*;
    /// Generated by tools/MathOracle.java calling the actual JAR classes on Java 25.
    #[test]
    fn actual_jar_math_and_sound_parity() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../build/punchy-analysis/math-oracle.tsv");
        let rows = std::fs::read_to_string(path)
            .expect("run native/punchy/tools/check_math_parity.py first");
        let mut count = 0;
        let mut cursor = SoundCursor::default();
        for row in rows.lines() {
            let fields: Vec<_> = row.split('\t').collect();
            match fields[0] {
                "ease" => {
                    let curve = fields[1].parse().unwrap();
                    let x = fields[2].parse().unwrap();
                    let expected: f64 = fields[3].parse().unwrap();
                    assert!((ease(curve, x) - expected).abs() < 1e-12, "{row}");
                }
                "expr" => {
                    let expr = Expression::compile(fields[1]).unwrap();
                    let time = fields[2].parse().unwrap();
                    let expected: f64 = fields[3].parse().unwrap();
                    assert!(
                        (expr.eval(time) - expected).abs() < 1e-12,
                        "{row} vs {}",
                        expr.eval(time)
                    );
                }
                "sound" => {
                    let i: usize = fields[1].parse().unwrap();
                    if i == 5 {
                        cursor.include_lead_in(1, 3)
                    }
                    let playbacks = [1, 1, 1, 1, 2, 3];
                    let entry = [0.0, 0.0, 0.0, 0.0, 0.5, 0.5];
                    let times = [0.6, 0.6, 0.1, 0.8, 0.7, 0.7];
                    let (new, ranges) = cursor.advance(1, playbacks[i], entry[i], times[i], 1.0);
                    assert_eq!(new.to_string(), fields[2]);
                    assert_eq!(ranges.len(), fields.len() - 3);
                    for (range, text) in ranges.iter().zip(&fields[3..]) {
                        let parts: Vec<_> = text.split(',').collect();
                        assert_eq!(range.start, parts[0].parse::<f32>().unwrap());
                        assert_eq!(range.end, parts[1].parse::<f32>().unwrap());
                        assert_eq!(range.include_start.to_string(), parts[2]);
                    }
                }
                _ => panic!("unknown oracle row {row}"),
            };
            count += 1;
        }
        assert_eq!(count, 338);
    }
}

#[path = "tuning.rs"]
pub mod tuning;

#[path = "horizon_pose.rs"]
pub mod horizon_pose;

#[path = "skeleton.rs"]
pub mod skeleton;

#[path = "attachments.rs"]
pub mod attachments;
#[path = "playback.rs"]
pub mod playback;

#[path = "phases.rs"]
pub mod phases;

pub mod policy;

pub mod player;
