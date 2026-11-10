use glam::{EulerRot, Mat4, Quat, Vec3};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Idle,
    WalkForward,
    WalkBackward,
    WalkLeft,
    WalkRight,
    SprintForward,
    SprintLeft,
    SprintRight,
    Jump,
    Fall,
    Land,
    Air,
    Flight,
    Swim,
    Climb,
    Crawl,
    Ride,
    RideIdle,
    Rowing,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Input {
    pub dt: f32,
    pub forward: f32,
    pub strafe: f32,
    pub horizontal_speed: f32,
    pub velocity_x: f32,
    pub velocity_z: f32,
    pub vertical_speed: f32,
    pub fall_distance: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub grounded: bool,
    pub sprinting: bool,
    pub sneaking: bool,
    pub swimming: bool,
    pub fall_flying: bool,
    pub flying: bool,
    pub spectator: bool,
    pub passenger: bool,
    pub sleeping: bool,
    pub climbing: bool,
    pub crawling: bool,
    pub riding_horse: bool,
    pub rowing: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Sample {
    pub mode: Mode,
    pub phase: f32,
    pub walk_phase: f32,
    pub sprint_phase: f32,
    pub air_time: f32,
    pub walk_weight: f32,
    pub sprint_weight: f32,
    pub idle_weight: f32,
    pub air_weight: f32,
    pub fall_weight: f32,
    pub flight_weight: f32,
    pub special_weight: f32,
}

#[derive(Clone, Copy, Debug, Default)]
struct Spring3 {
    value: Vec3,
    velocity: Vec3,
}

impl Spring3 {
    fn step(&mut self, target: Vec3, stiffness: f32, damping: f32, dt: f32) {
        let steps = ((dt / (1.0 / 120.0)).ceil() as usize).clamp(1, 16);
        let step = dt / steps as f32;
        for _ in 0..steps {
            let acceleration = (target - self.value) * stiffness - self.velocity * damping;
            self.velocity += acceleration * step;
            self.value += self.velocity * step;
        }
        if !self.value.is_finite() || !self.velocity.is_finite() {
            *self = Self::default();
        }
        self.value = self.value.clamp(Vec3::splat(-2.0), Vec3::splat(2.0));
        self.velocity = self.velocity.clamp(Vec3::splat(-30.0), Vec3::splat(30.0));
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct HandPose {
    position: Vec3,
    rotation: Vec3,
}

#[derive(Clone, Debug, Default)]
pub struct HandMotion {
    time: f32,
    previous_time: f32,
    walk_phase: f32,
    previous_walk_phase: f32,
    walk_phase_velocity: f32,
    sprint_phase: f32,
    previous_sprint_phase: f32,
    sprint_phase_velocity: f32,
    walk_weight: f32,
    previous_walk_weight: f32,
    sprint_weight: f32,
    previous_sprint_weight: f32,
    fall_phase: f32,
    previous_fall_phase: f32,
    fall_phase_velocity: f32,
    fall_weight: f32,
    previous_fall_weight: f32,
    flight_weight: f32,
    previous_flight_weight: f32,
    climb_phase: f32,
    previous_climb_phase: f32,
    crawl_phase: f32,
    previous_crawl_phase: f32,
    climb_weight: f32,
    previous_climb_weight: f32,
    crawl_weight: f32,
    previous_crawl_weight: f32,
    idle_jitter: [f32; 2],
    idle_jitter_target: [f32; 2],
    idle_jitter_timer: [f32; 2],
    air_time: f32,
    previous_air_time: f32,
    land_time: f32,
    previous_land_time: f32,
    air_ticks: u32,
    airborne_peak: f32,
    airborne_started_with_jump: bool,
    jump_value: f32,
    jump_velocity: f32,
    previous_grounded: Option<bool>,
    last_yaw: f32,
    last_pitch: f32,
    initialized_look: bool,
    last_velocity: Vec3,
    look_spring: Spring3,
    hands: [Spring3; 2],
    rotations: [Spring3; 2],
    land_remaining: f32,
    previous_hands: [HandPose; 2],
    current_hands: [HandPose; 2],
    mode: Mode,
    previous_mode: Mode,
}

impl HandMotion {
    pub fn tick(&mut self, input: Input) {
        let dt = input.dt.clamp(0.001, 0.1);
        self.previous_time = self.time;
        self.time += dt;
        self.previous_walk_phase = self.walk_phase;
        self.previous_sprint_phase = self.sprint_phase;
        self.previous_walk_weight = self.walk_weight;
        self.previous_sprint_weight = self.sprint_weight;
        self.previous_fall_phase = self.fall_phase;
        self.previous_fall_weight = self.fall_weight;
        self.previous_flight_weight = self.flight_weight;
        self.previous_climb_phase = self.climb_phase;
        self.previous_crawl_phase = self.crawl_phase;
        self.previous_climb_weight = self.climb_weight;
        self.previous_crawl_weight = self.crawl_weight;
        self.previous_air_time = self.air_time;
        self.previous_land_time = self.land_time;
        self.previous_hands = self.current_hands;
        self.previous_mode = self.mode;

        let forward = input.forward.clamp(-1.0, 1.0);
        let strafe = input.strafe.clamp(-1.0, 1.0);
        let input_amount = (forward * forward + strafe * strafe).sqrt().clamp(0.0, 1.0);
        let speed_amount = (input.horizontal_speed.max(0.0) / 0.22).clamp(0.0, 1.0);
        let move_amount = input_amount.max(speed_amount);
        let supported = input.grounded
            && !input.swimming
            && !input.fall_flying
            && !input.flying
            && !input.spectator
            && !input.passenger
            && !input.sleeping
            && !input.climbing
            && !input.crawling
            && !input.riding_horse
            && !input.rowing;
        let walk_allowed = supported && move_amount > 0.035;
        let sprint_allowed = walk_allowed && input.sprinting && (forward > 0.05 || strafe.abs() > 0.05);
        let sneak_factor = if input.sneaking { 0.38 } else { 1.0 };
        let walk_target = if walk_allowed { sneak_factor } else { 0.0 };
        let sprint_target = if sprint_allowed { 1.0 } else { 0.0 };
        self.walk_weight = smooth_toward(self.walk_weight, walk_target, if walk_target > self.walk_weight { 0.35 } else { 0.6 }, dt);
        self.sprint_weight = smooth_toward(self.sprint_weight, sprint_target, if sprint_target > self.sprint_weight { 0.07 } else { 0.35 }, dt);
        let walk_speed_target = if walk_allowed { move_amount } else { 0.0 };
        let sprint_speed_target = if sprint_allowed { (move_amount * 1.6).clamp(0.0, 1.0) } else { 0.0 };
        let walk_rate = 5.31 + (8.13 - 5.31) * walk_speed_target;
        let sprint_rate = 6.63 + (9.69 - 6.63) * sprint_speed_target;
        self.walk_phase_velocity = lerp_exp(self.walk_phase_velocity, walk_rate, dt, 0.25);
        self.sprint_phase_velocity = lerp_exp(self.sprint_phase_velocity, sprint_rate, dt, 0.25);
        if self.walk_weight > 0.001 {
            self.walk_phase += self.walk_phase_velocity * dt;
        } else {
            self.walk_phase_velocity = 0.0;
        }
        if self.sprint_weight > 0.001 {
            self.sprint_phase += self.sprint_phase_velocity * dt;
        } else {
            self.sprint_phase_velocity = 0.0;
        }

        let grounded_before = self.previous_grounded.unwrap_or(input.grounded);
        let just_left_ground = grounded_before && !input.grounded;
        let just_landed = !grounded_before && input.grounded && self.air_ticks > 1;
        if just_left_ground {
            self.airborne_started_with_jump = input.vertical_speed > 0.08;
            self.airborne_peak = 0.0;
            self.air_time = 0.0;
            self.air_ticks = 0;
            if self.airborne_started_with_jump {
                self.jump_velocity += -2.8;
            }
        } else if just_landed {
            self.jump_velocity += 2.4;
            self.air_time = 0.0;
            self.land_remaining = 0.24;
            self.land_time = 0.0;
        }
        if self.land_remaining > 0.0 { self.land_time += dt; }
        self.land_remaining = (self.land_remaining - dt).max(0.0);
        if !input.grounded {
            self.air_ticks = self.air_ticks.saturating_add(1);
            self.air_time += dt;
            self.airborne_peak = self.airborne_peak.max(input.fall_distance.max(0.0));
        } else if !just_landed {
            self.air_time = 0.0;
            self.air_ticks = 0;
            self.airborne_peak = 0.0;
        }
        self.previous_grounded = Some(input.grounded);
        let jump_acceleration = -75.0 * self.jump_value - 16.0 * self.jump_velocity;
        self.jump_velocity += jump_acceleration * dt;
        self.jump_value += self.jump_velocity * dt;
        if self.jump_value.abs() < 0.0001 && self.jump_velocity.abs() < 0.0001 {
            self.jump_value = 0.0;
            self.jump_velocity = 0.0;
        }

        let descending = (!input.grounded && !input.swimming && !input.fall_flying)
            && (input.vertical_speed < -0.04 || input.fall_distance > 0.15);
        let fall_target = if descending {
            ((-input.vertical_speed - 0.04) / 0.85).clamp(0.0, 1.0).max((input.fall_distance / 3.0).clamp(0.0, 1.0))
        } else {
            0.0
        };
        self.fall_weight = smooth_toward(self.fall_weight, fall_target, if fall_target > self.fall_weight { 0.16 } else { 0.2 }, dt);
        if self.fall_weight > 0.001 {
            let target_rate = -5.4 + (-8.8 + 5.4) * self.fall_weight;
            self.fall_phase_velocity = lerp_exp(self.fall_phase_velocity, target_rate, dt, 0.26);
            self.fall_phase += self.fall_phase_velocity * dt;
        } else {
            self.fall_phase_velocity = 0.0;
        }

        let speed = input.horizontal_speed.max(0.0);
        let flight_target = if input.fall_flying && !input.swimming {
            ((speed - 0.55) / 1.2).clamp(0.0, 1.0)
        } else if input.flying && !input.swimming {
            0.42
        } else {
            0.0
        };
        self.flight_weight = smooth_toward(self.flight_weight, flight_target, if flight_target > self.flight_weight { 0.52 } else { 0.24 }, dt);
        let climb_target = if input.climbing && !input.sleeping && !input.spectator && !input.passenger { 1.0 } else { 0.0 };
        let crawl_target = if input.crawling && !input.sleeping && !input.spectator && !input.passenger { 1.0 } else { 0.0 };
        self.climb_weight = smooth_toward(self.climb_weight, climb_target, if climb_target > self.climb_weight { 0.12 } else { 0.18 }, dt);
        self.crawl_weight = smooth_toward(self.crawl_weight, crawl_target, if crawl_target > self.crawl_weight { 0.22 } else { 0.28 }, dt);
        if self.climb_weight > 0.001 { self.climb_phase += 5.8 * dt; }
        if self.crawl_weight > 0.001 { self.crawl_phase += 4.6 * dt; }

        let (yaw_delta, pitch_delta) = if self.initialized_look {
            (wrap_degrees(input.yaw - self.last_yaw), input.pitch - self.last_pitch)
        } else {
            (0.0, 0.0)
        };
        self.initialized_look = true;
        self.last_yaw = input.yaw;
        self.last_pitch = input.pitch;
        let look_target = Vec3::new(
            (-pitch_delta * 0.0025).clamp(-0.09, 0.09),
            (-yaw_delta * 0.0022).clamp(-0.11, 0.11),
            (yaw_delta * 0.0012).clamp(-0.06, 0.06),
        );
        self.look_spring.step(look_target, 64.0, 15.0, dt);

        let velocity = Vec3::new(input.velocity_x, input.vertical_speed, input.velocity_z);
        let acceleration = (velocity - self.last_velocity).clamp(Vec3::splat(-2.0), Vec3::splat(2.0));
        self.last_velocity = velocity;

        self.mode = if (input.fall_flying || input.flying) && self.flight_weight > 0.01 {
            Mode::Flight
        } else if input.swimming {
            Mode::Swim
        } else if input.rowing {
            Mode::Rowing
        } else if input.riding_horse {
            if move_amount > 0.035 || input.horizontal_speed > 0.03 { Mode::Ride } else { Mode::RideIdle }
        } else if self.climb_weight > 0.01 {
            Mode::Climb
        } else if self.crawl_weight > 0.01 {
            Mode::Crawl
        } else if just_landed || self.land_remaining > 0.0 {
            Mode::Land
        } else if !input.grounded && self.airborne_peak >= 3.0 {
            Mode::Fall
        } else if !input.grounded && self.airborne_started_with_jump {
            Mode::Jump
        } else if !input.grounded {
            Mode::Air
        } else if sprint_allowed {
            if strafe > 0.01 { Mode::SprintLeft } else if strafe < -0.01 { Mode::SprintRight } else { Mode::SprintForward }
        } else if walk_allowed {
            if strafe > 0.01 { Mode::WalkLeft } else if strafe < -0.01 { Mode::WalkRight } else if forward < -0.01 { Mode::WalkBackward } else { Mode::WalkForward }
        } else {
            Mode::Idle
        };

        let idle_weight = if input.grounded && !input.swimming && !input.fall_flying && !input.flying && !input.spectator && !input.passenger && !input.sleeping && !input.climbing && !input.crawling && !input.riding_horse && !input.rowing {
            (1.0 - self.walk_weight.max(self.sprint_weight)).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let fall_phase = self.fall_phase;
        for index in 0..2 {
            let left = index == 1;
            let sign = if left { -1.0 } else { 1.0 };
            let phase_shift = if left { std::f32::consts::PI } else { 0.0 };
            let walk_phase = self.walk_phase + phase_shift;
            let sprint_phase = self.sprint_phase + phase_shift;
            let walk_swing = walk_phase.sin() * self.walk_weight;
            let sprint_swing = sprint_phase.sin() * self.sprint_weight;
            let side_input = strafe * 0.004 * sign;
            let crouch_scale = if input.sneaking { 0.58 } else { 1.0 };
            let mut target_position = Vec3::new(
                walk_swing * 0.014 + sprint_swing * 0.021 + side_input,
                walk_swing.abs() * 0.008 + sprint_swing.abs() * 0.013 + (self.time * 1.4 + phase_shift).sin() * 0.0025 * idle_weight,
                walk_swing * -0.008 + sprint_swing * -0.012,
            ) * crouch_scale;
            let mut target_rotation = Vec3::new(
                walk_swing * 0.052 + sprint_swing * 0.09,
                strafe * 0.035 * sign * (self.walk_weight + self.sprint_weight),
                -walk_swing * 0.038 * sign - sprint_swing * 0.065 * sign,
            );
            let idle_phase = self.time * 1.9 + if left { 0.8 } else { 0.0 };
            if self.idle_jitter_timer[index] <= 0.0 {
                let seed = ((self.time * 2.17 + index as f32 * 19.31).sin() * 43758.5453).abs().fract();
                self.idle_jitter_target[index] = seed * 2.0 - 1.0;
                self.idle_jitter_timer[index] = 0.35 + (seed.abs() * 0.45);
            }
            self.idle_jitter_timer[index] = (self.idle_jitter_timer[index] - dt).max(0.0);
            self.idle_jitter[index] = lerp_exp(self.idle_jitter[index], self.idle_jitter_target[index], dt, 0.12);
            target_position += Vec3::new(
                idle_phase.sin() * 0.0018 * idle_weight * sign,
                (idle_phase * 0.7).sin() * 0.0038 * idle_weight + self.idle_jitter[index] * 0.0016 * idle_weight,
                (idle_phase * 0.5).cos() * 0.0015 * idle_weight,
            );
            target_rotation += Vec3::new(
                (idle_phase * 0.8).sin() * 0.007 * idle_weight + self.idle_jitter[index] * 0.012 * idle_weight,
                self.look_spring.value.y * sign,
                (idle_phase * 0.65).sin() * 0.007 * idle_weight + self.look_spring.value.z * sign,
            );
            target_position.y += -self.jump_value * 0.0045;
            target_rotation.x += self.jump_value * 0.014;
            if self.fall_weight > 0.001 {
                let phase = fall_phase + phase_shift;
                target_position.y += phase.cos() * 0.011 * self.fall_weight;
                target_position.z += phase.sin() * 0.017 * self.fall_weight;
                target_rotation.x += (phase + 0.35).sin() * 0.085 * self.fall_weight;
                target_rotation.z += (phase + 0.12).cos() * 0.06 * self.fall_weight * sign;
            }
            if self.flight_weight > 0.001 {
                target_position.y += 0.045 * self.flight_weight;
                target_position.z -= 0.10 * self.flight_weight;
                target_rotation.y += -0.42 * sign * self.flight_weight;
                target_rotation.x += -0.05 * self.flight_weight;
            }
            if self.climb_weight > 0.001 {
                let climb_phase = self.climb_phase + phase_shift;
                let climb_swing = climb_phase.sin() * self.climb_weight;
                target_position += Vec3::new(sign * 0.004, climb_swing * 0.024, -0.018 * self.climb_weight);
                target_rotation += Vec3::new(-0.18 * self.climb_weight + climb_swing * 0.08, 0.0, climb_swing * 0.12 * sign);
            }
            if input.rowing {
                let row_phase = self.time * 4.2 + phase_shift;
                target_position += Vec3::new(row_phase.sin() * 0.028 * sign, row_phase.cos().abs() * 0.012, -0.025);
                target_rotation += Vec3::new(0.22 + row_phase.sin() * 0.16, 0.0, row_phase.cos() * 0.10 * sign);
            }
            if self.crawl_weight > 0.001 {
                let crawl_phase = self.crawl_phase + phase_shift;
                let crawl_swing = crawl_phase.sin() * self.crawl_weight;
                target_position += Vec3::new(crawl_swing * 0.009, crawl_swing.abs() * 0.008, -0.028 * self.crawl_weight);
                target_rotation += Vec3::new(0.13 * self.crawl_weight + crawl_swing * 0.045, 0.0, -crawl_swing * 0.10 * sign);
            }
            if input.swimming {
                let swim_phase = self.time * 3.2 + phase_shift;
                target_position.y += swim_phase.sin() * 0.012;
                target_rotation.x += swim_phase.cos() * 0.06;
                target_rotation.z += swim_phase.sin() * 0.045 * sign;
            }
            let acceleration_target = Vec3::new(
                (-acceleration.x * 0.010 * sign).clamp(-0.018, 0.018),
                (-acceleration.y * 0.004).clamp(-0.012, 0.012),
                (-acceleration.z * 0.010).clamp(-0.018, 0.018),
            );
            target_position += acceleration_target;
            self.hands[index].step(target_position, 58.0, 15.0, dt);
            let rotation_spring_target = target_rotation + Vec3::new(self.look_spring.value.x, 0.0, 0.0);
            self.rotations[index].step(rotation_spring_target, 72.0, 17.0, dt);
            self.current_hands[index] = HandPose {
                position: self.hands[index].value,
                rotation: self.rotations[index].value.clamp(Vec3::splat(-1.2), Vec3::splat(1.2)),
            };
        }
        if input.grounded || input.flying || input.spectator {
            self.airborne_started_with_jump = false;
        }
    }

    pub fn sample(&self, partial: f32) -> Sample {
        let t = partial.clamp(0.0, 1.0);
        let walk_phase = mix(self.previous_walk_phase, self.walk_phase, t);
        let sprint_phase = mix(self.previous_sprint_phase, self.sprint_phase, t);
        let mode = if t < 0.5 { self.previous_mode } else { self.mode };
        let (phase, weight) = match mode {
            Mode::WalkForward | Mode::WalkBackward | Mode::WalkLeft | Mode::WalkRight => (walk_phase, mix(self.previous_walk_weight, self.walk_weight, t)),
            Mode::SprintForward | Mode::SprintLeft | Mode::SprintRight => (sprint_phase, mix(self.previous_sprint_weight, self.sprint_weight, t)),
            Mode::Jump | Mode::Fall | Mode::Air => (mix(self.previous_air_time, self.air_time, t), if mode == Mode::Jump { 0.62 } else if mode == Mode::Fall { mix(self.previous_fall_weight, self.fall_weight, t).max(0.45) } else { 0.0 }),
            Mode::Land => (mix(self.previous_land_time, self.land_time, t), 0.82),
            Mode::Climb => (mix(self.previous_climb_phase, self.climb_phase, t), mix(self.previous_climb_weight, self.climb_weight, t)),
            Mode::Crawl => (mix(self.previous_crawl_phase, self.crawl_phase, t), mix(self.previous_crawl_weight, self.crawl_weight, t)),
            _ => (mix(self.previous_time, self.time, t), 0.0),
        };
        Sample {
            mode,
            phase,
            walk_phase,
            sprint_phase,
            air_time: mix(self.previous_air_time, self.air_time, t),
            walk_weight: mix(self.previous_walk_weight, self.walk_weight, t),
            idle_weight: if mode == Mode::Idle { (1.0 - mix(self.previous_walk_weight, self.walk_weight, t).max(mix(self.previous_sprint_weight, self.sprint_weight, t))).clamp(0.0, 1.0) } else { 0.0 },
            sprint_weight: mix(self.previous_sprint_weight, self.sprint_weight, t),
            air_weight: weight,
            fall_weight: mix(self.previous_fall_weight, self.fall_weight, t),
            flight_weight: mix(self.previous_flight_weight, self.flight_weight, t),
            special_weight: match mode {
                Mode::Climb => mix(self.previous_climb_weight, self.climb_weight, t),
                Mode::Crawl => mix(self.previous_crawl_weight, self.crawl_weight, t),
                Mode::Ride | Mode::RideIdle => 0.92,
                Mode::Rowing => 1.0,
                _ => 0.0,
            },
        }
    }

    pub fn matrix(&self, left: bool, partial: f32, action_weight: f32) -> Mat4 {
        self.matrix_with_influence(left, partial, action_weight, 1.0)
    }

    pub fn matrix_with_influence(&self, left: bool, partial: f32, action_weight: f32, influence: f32) -> Mat4 {
        let t = partial.clamp(0.0, 1.0);
        let index = usize::from(left);
        let from = self.previous_hands[index];
        let to = self.current_hands[index];
        let position = from.position.lerp(to.position, t);
        let rotation = from.rotation.lerp(to.rotation, t);
        let action_factor = (1.0 - action_weight.clamp(0.0, 1.0) * 0.78) * influence.clamp(0.0, 1.0);
        let position = position * action_factor;
        let rotation = rotation * action_factor;
        let quaternion = Quat::from_euler(EulerRot::ZYX, rotation.z, rotation.y, rotation.x);
        let pivot = Vec3::new(if left { -0.56 } else { 0.56 }, -0.52, -0.72);
        Mat4::from_translation(pivot + position) * Mat4::from_quat(quaternion) * Mat4::from_translation(-pivot)
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn is_jumping(&self) -> bool {
        self.mode == Mode::Jump
    }

    pub fn is_landing(&self) -> bool {
        self.mode == Mode::Land
    }
}

fn smooth_toward(current: f32, target: f32, duration: f32, dt: f32) -> f32 {
    let factor = (dt / duration.max(0.001)).clamp(0.0, 1.0);
    (current + (target - current) * factor).clamp(0.0, 1.0)
}

fn lerp_exp(current: f32, target: f32, dt: f32, response: f32) -> f32 {
    let factor = 1.0 - (-dt / response.max(0.001)).exp();
    current + (target - current) * factor
}

fn mix(from: f32, to: f32, t: f32) -> f32 {
    from + (to - from) * t
}

fn wrap_degrees(value: f32) -> f32 {
    (value + 180.0).rem_euclid(360.0) - 180.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_motion_is_bounded_and_independent_for_each_hand() {
        let mut motion = HandMotion::default();
        for tick in 0..240 {
            motion.tick(Input { dt: 0.05, grounded: true, yaw: tick as f32 * 0.02, ..Default::default() });
        }
        let right = motion.matrix(false, 1.0, 0.0).to_scale_rotation_translation();
        let left = motion.matrix(true, 1.0, 0.0).to_scale_rotation_translation();
        assert!(right.2.is_finite() && left.2.is_finite());
        assert_ne!(right.2, left.2);
        assert!(right.2.length() < 0.05 && left.2.length() < 0.05);
    }

    #[test]
    fn walk_and_sprint_blends_ramp_and_decay_without_nan() {
        let mut motion = HandMotion::default();
        for _ in 0..60 {
            motion.tick(Input { dt: 0.05, forward: 1.0, horizontal_speed: 0.12, velocity_z: 0.12, grounded: true, ..Default::default() });
        }
        let walking = motion.sample(1.0);
        assert!(walking.walk_weight > 0.8);
        assert!(walking.walk_phase > 1.0);
        for _ in 0..60 {
            motion.tick(Input { dt: 0.05, forward: 1.0, horizontal_speed: 0.15, velocity_z: 0.15, grounded: true, sprinting: true, ..Default::default() });
        }
        assert!(motion.sample(1.0).sprint_weight > 0.8);
        for _ in 0..90 {
            motion.tick(Input { dt: 0.05, grounded: true, ..Default::default() });
        }
        assert!(motion.sample(1.0).walk_weight < 0.05);
        assert!(motion.sample(1.0).sprint_weight < 0.05);
    }

    #[test]
    fn jump_landing_and_fall_have_state_transitions() {
        let mut motion = HandMotion::default();
        motion.tick(Input { dt: 0.05, grounded: true, ..Default::default() });
        motion.tick(Input { dt: 0.05, grounded: false, vertical_speed: 0.35, ..Default::default() });
        assert!(motion.is_jumping());
        for _ in 0..50 {
            motion.tick(Input { dt: 0.05, grounded: false, vertical_speed: -0.7, fall_distance: 3.2, ..Default::default() });
        }
        assert_eq!(motion.mode(), Mode::Fall);
        motion.tick(Input { dt: 0.05, grounded: true, vertical_speed: 0.0, ..Default::default() });
        assert!(motion.is_landing());
        let mut ledge = HandMotion::default();
        ledge.tick(Input { dt: 0.05, grounded: true, ..Default::default() });
        ledge.tick(Input { dt: 0.05, grounded: false, vertical_speed: -0.15, ..Default::default() });
        assert_eq!(ledge.mode(), Mode::Air);
    }

    #[test]
    fn look_wrap_and_flight_are_stable() {
        let mut motion = HandMotion::default();
        motion.tick(Input { dt: 0.05, grounded: true, yaw: 179.0, ..Default::default() });
        motion.tick(Input { dt: 0.05, grounded: false, yaw: -179.0, vertical_speed: 0.1, fall_flying: true, horizontal_speed: 1.0, ..Default::default() });
        let matrix = motion.matrix(false, 0.5, 0.0);
        assert!(matrix.to_cols_array().iter().all(|v| v.is_finite()));
        assert!(motion.sample(1.0).flight_weight > 0.0);
    }

    #[test]
    fn climbing_and_crawling_use_distinct_motion_layers() {
        let mut climbing = HandMotion::default();
        climbing.tick(Input { dt: 0.05, climbing: true, vertical_speed: 0.12, ..Default::default() });
        assert_eq!(climbing.mode(), Mode::Climb);
        assert!(climbing.sample(1.0).special_weight > 0.0);
        let mut crawling = HandMotion::default();
        crawling.tick(Input { dt: 0.05, crawling: true, grounded: true, forward: 1.0, ..Default::default() });
        assert_eq!(crawling.mode(), Mode::Crawl);
        assert!(crawling.sample(1.0).special_weight > 0.0);
        let mut riding = HandMotion::default();
        riding.tick(Input { dt: 0.05, riding_horse: true, passenger: true, ..Default::default() });
        assert_eq!(riding.mode(), Mode::RideIdle);
        let mut rowing = HandMotion::default();
        rowing.tick(Input { dt: 0.05, rowing: true, passenger: true, ..Default::default() });
        assert_eq!(rowing.mode(), Mode::Rowing);
    }
}
