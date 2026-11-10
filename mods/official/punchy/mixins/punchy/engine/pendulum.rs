use glam::{Mat4, Quat, Vec3};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    None,
    FullPendulum,
    ForwardPendulum,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Settings {
    pub limit_deg: Option<f32>,
    pub limit_deg_up: Option<f32>,
    pub limit_deg_down: Option<f32>,
    pub force: Option<f32>,
    pub speed: Option<f32>,
    pub camera_sens: Option<f32>,
    pub gravity: Option<f32>,
    pub motion_sens: Option<f32>,
    pub invert: Option<bool>,
    pub hand_space: Option<bool>,
    pub enable_creature_physics: Option<bool>,
}

#[derive(Clone, Copy, Debug)]
pub struct Input {
    pub dt: f32,
    pub yaw: f32,
    pub yaw_delta: f32,
    pub pitch: f32,
    pub pitch_delta: f32,
    pub walk_speed: f32,
    pub walk_pos: f32,
    pub vel_y: f32,
    pub on_ground: bool,
    pub offset_x: f32,
    pub offset_z: f32,
    pub jump_value: f32,
    pub impact_value: f32,
    pub shield_impact_value: f32,
    pub action_value: f32,
    pub action_kick: f32,
}

impl Default for Input {
    fn default() -> Self {
        Self {
            dt: 1.0 / 60.0,
            yaw: 0.0,
            yaw_delta: 0.0,
            pitch: 0.0,
            pitch_delta: 0.0,
            walk_speed: 0.0,
            walk_pos: 0.0,
            vel_y: 0.0,
            on_ground: true,
            offset_x: 0.0,
            offset_z: 0.0,
            jump_value: 0.0,
            impact_value: 0.0,
            shield_impact_value: 0.0,
            action_value: 0.0,
            action_kick: 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct ResolvedSettings {
    limit_deg_up: f32,
    limit_deg_down: f32,
    force: f32,
    speed: f32,
    camera_sens: f32,
    gravity: f32,
    motion_sens: f32,
    invert: bool,
    has_explicit_limits: bool,
}

impl Settings {
    fn resolve(self) -> ResolvedSettings {
        let limit_deg = finite(self.limit_deg, 55.0);
        let mut limit_up = 55.0;
        let mut limit_down = 55.0;
        let has_limit = self.limit_deg.is_some();
        let has_up_down = self.limit_deg_up.is_some() || self.limit_deg_down.is_some();
        if let Some(value) = self.limit_deg_up {
            limit_up = finite(Some(value), 55.0);
        }
        if let Some(value) = self.limit_deg_down {
            limit_down = finite(Some(value), 55.0);
        }
        if has_limit && !has_up_down {
            limit_up = limit_deg;
            limit_down = limit_deg;
        } else if !has_up_down && limit_deg > 0.0 {
            limit_up = limit_deg;
            limit_down = limit_deg;
        }
        if has_up_down {
            let raw_up = limit_up;
            let raw_down = limit_down;
            limit_up = if raw_up > 0.0 { raw_up } else { raw_down };
            limit_down = if raw_down > 0.0 { raw_down } else { raw_up };
        }
        ResolvedSettings {
            limit_deg_up: limit_up.max(0.0),
            limit_deg_down: limit_down.max(0.0),
            force: finite(self.force, 0.75).max(0.0),
            speed: finite(self.speed, 16.0).max(0.001),
            camera_sens: finite(self.camera_sens, 0.3),
            gravity: finite(self.gravity, 25.0).max(0.0),
            motion_sens: finite(self.motion_sens, 0.2),
            invert: self.invert.unwrap_or(false),
            has_explicit_limits: has_limit || has_up_down,
        }
    }
}

#[derive(Clone, Debug)]
pub struct State {
    angle_x: f32,
    angle_z: f32,
    previous_angle_x: f32,
    previous_angle_z: f32,
    velocity_x: f32,
    velocity_z: f32,
    influence: f32,
    last_offset_x: f32,
    last_offset_z: f32,
    last_walk_bob: f32,
    last_jump_value: f32,
    last_impact_value: f32,
    last_shield_impact_value: f32,
    last_action_value: f32,
    last_vel_y: f32,
    motion_initialized: bool,
    vertical_initialized: bool,
    previous_on_ground: bool,
    ground_transition_cooldown: f32,
    initialized: bool,
    last_yaw: f32,
    last_pitch: f32,
    last_dt: f32,
    influence_initialized: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            angle_x: 0.0,
            angle_z: 0.0,
            previous_angle_x: 0.0,
            previous_angle_z: 0.0,
            velocity_x: 0.0,
            velocity_z: 0.0,
            influence: 1.0,
            last_offset_x: 0.0,
            last_offset_z: 0.0,
            last_walk_bob: 0.0,
            last_jump_value: 0.0,
            last_impact_value: 0.0,
            last_shield_impact_value: 0.0,
            last_action_value: 0.0,
            last_vel_y: 0.0,
            motion_initialized: false,
            vertical_initialized: false,
            previous_on_ground: true,
            ground_transition_cooldown: 0.0,
            initialized: false,
            last_yaw: 0.0,
            last_pitch: 0.0,
            last_dt: 1.0 / 60.0,
            influence_initialized: false,
        }
    }
}

impl State {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn update(&mut self, input: Input, settings: Settings, action_feedback_enabled: bool) -> bool {
        let settings = settings.resolve();
        let dt = finite(input.dt, 1.0 / 60.0).clamp(0.001, 1.0 / 30.0);
        self.last_dt = dt;
        if !self.initialized {
            self.initialize(input, settings);
            return false;
        }
        self.previous_angle_x = self.angle_x;
        self.previous_angle_z = self.angle_z;
        let others_scale = 1.0;
        let walk_sprint_scale = 1.0;
        let input_yaw = -input.yaw_delta * settings.camera_sens * others_scale;
        let input_pitch = -input.pitch_delta * settings.camera_sens * others_scale;
        let inertia = settings.force * 20.0;
        self.velocity_z -= input_yaw * inertia * 5.0;
        self.velocity_x += input_pitch * inertia;

        let current_walk_bob = walk_bob(input.walk_speed, input.walk_pos);
        if !self.motion_initialized {
            self.last_offset_x = input.offset_x;
            self.last_offset_z = input.offset_z;
            self.last_walk_bob = current_walk_bob;
            self.last_jump_value = input.jump_value;
            self.last_impact_value = input.impact_value;
            self.last_shield_impact_value = input.shield_impact_value;
            self.last_action_value = input.action_value;
            self.motion_initialized = true;
        }
        let delta_offset_x = input.offset_x - self.last_offset_x;
        let delta_offset_z = input.offset_z - self.last_offset_z;
        let delta_walk_bob = current_walk_bob - self.last_walk_bob;
        let delta_jump = (input.jump_value - self.last_jump_value) * others_scale;
        let delta_impact = (input.impact_value - self.last_impact_value) * others_scale;
        let delta_shield = (input.shield_impact_value - self.last_shield_impact_value) * others_scale;
        let delta_action = (input.action_value - self.last_action_value) * others_scale;
        self.last_offset_x = input.offset_x;
        self.last_offset_z = input.offset_z;
        self.last_walk_bob = current_walk_bob;
        self.last_jump_value = input.jump_value;
        self.last_impact_value = input.impact_value;
        self.last_shield_impact_value = input.shield_impact_value;
        self.last_action_value = input.action_value;

        let walk_yaw_velocity = delta_offset_x * 32.0;
        let walk_pitch_velocity = delta_offset_z * 32.0;
        if settings.motion_sens != 0.0 {
            let mut motion_yaw = walk_yaw_velocity * 18.0 * walk_sprint_scale;
            let motion_pitch = walk_pitch_velocity * 16.0 * walk_sprint_scale
                + delta_jump * 36.0
                + delta_impact * 42.0
                + delta_shield * 30.0;
            motion_yaw += delta_walk_bob * 18.0 * walk_sprint_scale;
            self.velocity_z -= motion_yaw * settings.motion_sens;
            self.velocity_x += motion_pitch * settings.motion_sens;
        }
        if action_feedback_enabled {
            self.velocity_x += delta_action * 34.0;
            if input.action_kick != 0.0 {
                let kick = input.action_kick * others_scale;
                self.velocity_x += kick * 220.0;
                self.angle_x += kick * 1.8;
            }
        }

        let motion_weight = settings.motion_sens.abs().max(0.55);
        self.ground_transition_cooldown = (self.ground_transition_cooldown - dt).max(0.0);
        if !self.vertical_initialized {
            self.last_vel_y = input.vel_y;
            self.previous_on_ground = input.on_ground;
            self.vertical_initialized = true;
        } else {
            let delta_velocity = input.vel_y - self.last_vel_y;
            self.last_vel_y = input.vel_y;
            let dt_scale = (0.016666668 / dt.max(0.0001)).clamp(0.35, 1.25);
            let vertical_impulse = (-delta_velocity * 460.0 * dt_scale).clamp(-120.0, 120.0) * others_scale;
            self.velocity_x += vertical_impulse;
            if self.ground_transition_cooldown <= 0.0 && self.previous_on_ground && !input.on_ground && input.vel_y > 0.08 {
                self.angle_x -= 4.5 * others_scale;
                self.velocity_x -= 180.0 * others_scale;
                self.ground_transition_cooldown = 0.08;
            } else if self.ground_transition_cooldown <= 0.0 && !self.previous_on_ground && input.on_ground {
                self.angle_x += 6.0 * others_scale;
                self.velocity_x += 240.0 * others_scale;
                self.ground_transition_cooldown = 0.08;
            }
            self.previous_on_ground = input.on_ground;
        }

        let mut target_pitch = input.pitch * others_scale
            + (-input.vel_y * 28.0).clamp(-18.0, 18.0) * others_scale * motion_weight;
        if settings.limit_deg_up > 0.0 || settings.limit_deg_down > 0.0 {
            target_pitch = target_pitch.clamp(-settings.limit_deg_down, settings.limit_deg_up);
        }
        let acceleration_x = (target_pitch - self.angle_x) * settings.gravity;
        let acceleration_z = -self.angle_z * (settings.gravity * 0.6);
        let damping = 2.2 / settings.speed + 0.45 * settings.gravity.sqrt();
        self.velocity_x += acceleration_x * dt;
        self.velocity_z += acceleration_z * dt;
        let decay = (-damping * dt).exp();
        self.velocity_x *= decay;
        self.velocity_z *= decay;
        self.angle_x += self.velocity_x * dt;
        self.angle_z += self.velocity_z * dt;

        if settings.has_explicit_limits || settings.limit_deg_up > 0.0 || settings.limit_deg_down > 0.0 {
            let limit_up = settings.limit_deg_up;
            let limit_down = settings.limit_deg_down;
            let limit_z = limit_up.max(limit_down);
            if settings.has_explicit_limits && limit_up == 0.0 && limit_down == 0.0 {
                self.angle_x = 0.0;
                self.velocity_x = 0.0;
            }
            if settings.has_explicit_limits && limit_z == 0.0 {
                self.angle_z = 0.0;
                self.velocity_z = 0.0;
            }
            let crossed_x = self.previous_angle_x > -limit_down && self.previous_angle_x < limit_up;
            let crossed_z = self.previous_angle_z.abs() < limit_z;
            if self.angle_x > limit_up {
                self.angle_x = limit_up;
                self.velocity_x = -self.velocity_x * 0.1 * 0.25;
            } else if self.angle_x < -limit_down {
                self.angle_x = -limit_down;
                if crossed_x && self.velocity_x < 0.0 {
                    self.velocity_x = -self.velocity_x * 0.1 * 0.25;
                }
            }
            if self.angle_z > limit_z {
                self.angle_z = limit_z;
                if crossed_z && self.velocity_z > 0.0 {
                    self.velocity_z = -self.velocity_z * 0.1 * 0.25;
                }
            } else if self.angle_z < -limit_z {
                self.angle_z = -limit_z;
                if crossed_z && self.velocity_z < 0.0 {
                    self.velocity_z = -self.velocity_z * 0.1 * 0.25;
                }
            }
        }

        self.settle();
        self.last_yaw = input.yaw;
        self.last_pitch = input.pitch;
        if !self.is_finite() {
            self.reset();
        }
        true
    }

    pub fn matrix(
        &mut self,
        current_pose: Mat4,
        mode: Mode,
        pivot: Vec3,
        partial: f32,
        influence: f32,
        settings: Settings,
    ) -> Mat4 {
        if mode == Mode::None || !self.initialized {
            return Mat4::IDENTITY;
        }
        let target_influence = influence.clamp(0.0, 1.0);
        if !self.influence_initialized {
            self.influence = target_influence;
            self.influence_initialized = true;
        } else {
            let blend = 1.0 - (-self.last_dt / 0.075).exp();
            self.influence += (target_influence - self.influence) * blend;
        }
        let mut rot_x = mix(self.previous_angle_x, self.angle_x, partial.clamp(0.0, 1.0));
        let mut rot_z = mix(self.previous_angle_z, self.angle_z, partial.clamp(0.0, 1.0));
        if mode == Mode::ForwardPendulum {
            rot_z = 0.0;
        }
        if settings.invert.unwrap_or(false) {
            rot_x = -rot_x;
            rot_z = -rot_z;
        }
        rot_x *= self.influence;
        rot_z *= self.influence;
        if !rot_x.is_finite() || !rot_z.is_finite() || (rot_x == 0.0 && rot_z == 0.0) {
            return Mat4::IDENTITY;
        }
        self.compute_matrix(current_pose, pivot, rot_x, rot_z, self.last_yaw, self.last_pitch)
    }

    fn initialize(&mut self, input: Input, settings: ResolvedSettings) {
        let motion_weight = settings.motion_sens.abs().max(0.55);
        let mut target_pitch = input.pitch + (-input.vel_y * 28.0).clamp(-18.0, 18.0) * motion_weight;
        if settings.limit_deg_up > 0.0 || settings.limit_deg_down > 0.0 {
            target_pitch = target_pitch.clamp(-settings.limit_deg_down, settings.limit_deg_up);
        }
        self.angle_x = target_pitch;
        self.previous_angle_x = target_pitch;
        self.angle_z = 0.0;
        self.previous_angle_z = 0.0;
        self.last_vel_y = input.vel_y;
        self.previous_on_ground = input.on_ground;
        self.vertical_initialized = true;
        self.last_offset_x = input.offset_x;
        self.last_offset_z = input.offset_z;
        self.last_walk_bob = walk_bob(input.walk_speed, input.walk_pos);
        self.last_jump_value = input.jump_value;
        self.last_impact_value = input.impact_value;
        self.last_shield_impact_value = input.shield_impact_value;
        self.last_action_value = input.action_value;
        self.motion_initialized = true;
        self.last_yaw = input.yaw;
        self.last_pitch = input.pitch;
        self.influence = 1.0;
        self.influence_initialized = false;
        self.initialized = true;
    }

    fn compute_matrix(&self, current_pose: Mat4, pivot: Vec3, rot_x: f32, rot_z: f32, yaw: f32, pitch: f32) -> Mat4 {
        if !current_pose.is_finite() {
            return Mat4::IDENTITY;
        }
        if current_pose.determinant() < 0.0 {
            let mirror = Mat4::from_scale(Vec3::new(-1.0, 1.0, 1.0));
            let unmirrored = current_pose * mirror;
            let mirrored_pivot = Vec3::new(-pivot.x, pivot.y, pivot.z);
            let swing = self.compute_matrix_positive(unmirrored, mirrored_pivot, rot_x, rot_z, yaw, pitch);
            return mirror * swing * mirror;
        }
        self.compute_matrix_positive(current_pose, pivot, rot_x, rot_z, yaw, pitch)
    }

    fn compute_matrix_positive(&self, current_pose: Mat4, pivot: Vec3, rot_x: f32, rot_z: f32, yaw: f32, pitch: f32) -> Mat4 {
        let pivot_world = current_pose.transform_point3(pivot);
        let (scale, rotation, translation) = current_pose.to_scale_rotation_translation();
        if !scale.is_finite() || !rotation.is_finite() || !translation.is_finite() {
            return Mat4::IDENTITY;
        }
        let rigid_pose = Mat4::from_rotation_translation(rotation.normalize(), translation);
        let yaw_rad = yaw.to_radians();
        let pitch_rad = pitch.to_radians();
        let cos_pitch = pitch_rad.cos();
        let mut camera_forward = Vec3::new(-yaw_rad.sin() * cos_pitch, -pitch_rad.sin(), yaw_rad.cos() * cos_pitch);
        if camera_forward.length_squared() < 0.000001 {
            camera_forward = Vec3::Z;
        } else {
            camera_forward = camera_forward.normalize();
        }
        let mut camera_right = Vec3::new(-yaw_rad.cos(), 0.0, -yaw_rad.sin());
        if camera_right.length_squared() < 0.000001 {
            camera_right = Vec3::X;
        } else {
            camera_right = camera_right.normalize();
        }
        let mut world_pendulum = Mat4::from_translation(pivot_world);
        if rot_z != 0.0 {
            world_pendulum *= Mat4::from_quat(Quat::from_axis_angle(camera_forward, rot_z.to_radians()));
        }
        if rot_x != 0.0 {
            world_pendulum *= Mat4::from_quat(Quat::from_axis_angle(camera_right, rot_x.to_radians()));
        }
        world_pendulum *= Mat4::from_translation(-pivot_world);
        rigid_pose.inverse() * world_pendulum * rigid_pose
    }

    fn settle(&mut self) {
        if self.angle_x.abs() < 0.00025 && self.velocity_x.abs() < 0.00025 {
            self.angle_x = 0.0;
            self.velocity_x = 0.0;
        }
        if self.angle_z.abs() < 0.00025 && self.velocity_z.abs() < 0.00025 {
            self.angle_z = 0.0;
            self.velocity_z = 0.0;
        }
    }

    fn is_finite(&self) -> bool {
        [
            self.angle_x, self.angle_z, self.previous_angle_x, self.previous_angle_z,
            self.velocity_x, self.velocity_z, self.influence, self.last_offset_x,
            self.last_offset_z, self.last_walk_bob, self.last_jump_value,
            self.last_impact_value, self.last_shield_impact_value, self.last_action_value,
            self.last_vel_y, self.ground_transition_cooldown, self.last_yaw, self.last_pitch,
        ]
        .into_iter()
        .all(f32::is_finite)
    }
}

fn finite(value: Option<f32>, fallback: f32) -> f32 {
    value.filter(|v| v.is_finite()).unwrap_or(fallback)
}

fn walk_bob(speed: f32, position: f32) -> f32 {
    if speed > 0.01 {
        (position * 1.1).sin() * speed
    } else {
        0.0
    }
}

fn mix(from: f32, to: f32, amount: f32) -> f32 {
    from + (to - from) * amount
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_inertia_and_vertical_impulses_stay_finite_and_bounded() {
        let mut state = State::default();
        let settings = Settings::default();
        state.update(Input { pitch: 24.0, ..Default::default() }, settings, false);
        for i in 0..180 {
            state.update(Input {
                dt: 1.0 / 60.0,
                yaw: (i as f32 * 3.0).sin() * 80.0,
                yaw_delta: 3.0,
                pitch: 24.0,
                pitch_delta: -0.4,
                walk_speed: 0.6,
                walk_pos: i as f32 * 0.2,
                vel_y: if i < 12 { 0.25 } else if i < 48 { -0.2 } else { 0.0 },
                on_ground: i > 48,
                offset_x: (i as f32 * 0.05).sin() * 0.1,
                offset_z: (i as f32 * 0.05).cos() * 0.1,
                ..Default::default()
            }, settings, false);
        }
        assert!(state.is_finite());
        assert!(state.angle_x.abs() <= 55.0);
        assert!(state.angle_z.abs() <= 55.0);
    }

    #[test]
    fn full_and_forward_pendulum_modes_have_distinct_rotation_axes() {
        let mut state = State::default();
        state.update(Input { pitch: 0.0, ..Default::default() }, Settings::default(), false);
        state.angle_x = 9.0;
        state.angle_z = 7.0;
        state.previous_angle_x = 9.0;
        state.previous_angle_z = 7.0;
        let pose = Mat4::from_translation(Vec3::new(0.2, 0.3, -0.5));
        let full = state.matrix(pose, Mode::FullPendulum, Vec3::new(0.04, -0.14, 0.11), 1.0, 1.0, Settings::default());
        let forward = state.matrix(pose, Mode::ForwardPendulum, Vec3::new(0.04, -0.14, 0.11), 1.0, 1.0, Settings::default());
        assert!(full.is_finite() && forward.is_finite());
        assert!(!full.abs_diff_eq(forward, 1e-4));
    }


    #[test]
    fn angular_limit_resolution_matches_punchy_partial_overrides() {
        let one_sided = Settings {
            limit_deg_up: Some(31.0),
            ..Default::default()
        }.resolve();
        assert_eq!(one_sided.limit_deg_up, 31.0);
        assert_eq!(one_sided.limit_deg_down, 55.0);
        let zero_pair = Settings {
            limit_deg_up: Some(0.0),
            limit_deg_down: Some(0.0),
            ..Default::default()
        }.resolve();
        assert_eq!(zero_pair.limit_deg_up, 0.0);
        assert_eq!(zero_pair.limit_deg_down, 0.0);
        let zero_upper = Settings {
            limit_deg_up: Some(0.0),
            limit_deg_down: Some(24.0),
            ..Default::default()
        }.resolve();
        assert_eq!(zero_upper.limit_deg_up, 24.0);
        assert_eq!(zero_upper.limit_deg_down, 24.0);
    }

    #[test]
    fn first_influence_is_applied_without_an_extra_blend_delay() {
        let mut state = State::default();
        state.update(Input::default(), Settings::default(), false);
        state.matrix(
            Mat4::IDENTITY,
            Mode::FullPendulum,
            Vec3::ZERO,
            1.0,
            0.25,
            Settings::default(),
        );
        assert_eq!(state.influence, 0.25);
    }

    #[test]
    fn action_kick_is_deferred_until_the_state_is_initialized() {
        let mut state = State::default();
        let settings = Settings::default();
        assert!(!state.update(
            Input { action_kick: 1.0, ..Default::default() },
            settings,
            true,
        ));
        assert_eq!(state.angle_x, 0.0);
        assert!(state.update(
            Input { action_kick: 1.0, ..Default::default() },
            settings,
            true,
        ));
        assert!(state.angle_x > 0.0);
    }

    #[test]
    fn disabled_action_feedback_does_not_inject_an_action_kick() {
        let mut state = State::default();
        let settings = Settings::default();
        state.update(Input::default(), settings, false);
        state.update(
            Input { action_kick: 1.0, ..Default::default() },
            settings,
            false,
        );
        assert_eq!(state.angle_x, 0.0);
        assert_eq!(state.velocity_x, 0.0);
    }

    #[test]
    fn item_change_reset_clears_old_spring_and_angles() {
        let mut state = State::default();
        state.update(Input::default(), Settings::default(), false);
        state.angle_x = 20.0;
        state.velocity_x = 7.0;
        state.reset();
        assert_eq!(state.angle_x, 0.0);
        assert_eq!(state.velocity_x, 0.0);
        assert!(!state.initialized);
    }
}
