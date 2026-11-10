from pathlib import Path
import zipfile
import json

ROOT = Path(__file__).resolve().parents[1]
ENGINE = ROOT / "mods/official/punchy/mixins/punchy/engine"
RUNTIME = ROOT / "mods/official/punchy/mixins/punchy/runtime.rs"


def test_hand_motion_engine_is_exported():
    source = (ENGINE / "engine.rs").read_text()
    assert '#[path = "hand_motion.rs"]' in source
    assert "pub mod hand_motion;" in source


def test_hand_motion_tracks_idle_walk_sprint_and_physics_layers():
    source = (ENGINE / "hand_motion.rs").read_text()
    for name in [
        "walk_phase",
        "sprint_phase",
        "idle_jitter",
        "look_spring",
        "jump_velocity",
        "fall_phase",
        "flight_weight",
        "previous_hands",
        "current_hands",
    ]:
        assert name in source


def test_hand_motion_covers_climb_crawl_riding_and_boat_rowing():
    source = (ENGINE / "hand_motion.rs").read_text()
    runtime = RUNTIME.read_text()
    for mode in ["Climb", "Crawl", "Ride", "RideIdle", "Rowing"]:
        assert mode in source
        assert f"Mode::{mode}" in runtime
    assert "last_climbable" in runtime
    assert "Pose::Swimming" in runtime
    assert "equine::is_equine" in runtime
    assert "boat.paddling" in runtime


def test_original_builtin_locomotion_clips_are_resolved():
    runtime = RUNTIME.read_text()
    for clip in [
        "punchy_moves_walk_forward",
        "punchy_moves_walk_backward",
        "punchy_moves_walk_left",
        "punchy_moves_walk_right",
        "punchy_moves_sprint_forward",
        "punchy_moves_sprint_left",
        "punchy_moves_sprint_right",
        "punchy_moves_jump",
        "punchy_moves_land",
        "climb2",
        "crawl",
        "newhorse_riding_loop",
        "newhorse_riding_idle",
        "swiming",
    ]:
        assert clip in runtime


def test_builtin_animation_archive_contains_expected_extended_clips():
    archive = ENGINE / "data/punchy_builtin.zip"
    with zipfile.ZipFile(archive) as zf:
        data = json.loads(zf.read("assets/minecraft/punchy/animations/player.animation.json"))
    animations = data["animations"]
    for clip in ["climb", "climb2", "crawl", "newhorse_riding_loop", "newhorse_riding_idle", "swiming"]:
        assert clip in animations


def test_motion_adapter_keeps_action_ownership_separate_from_physics():
    adapter = (ENGINE / "horizon_pose.rs").read_text()
    runtime = RUNTIME.read_text()
    assert "pub motion: [Mat4; 2]" in adapter
    assert "pub active: [bool; 2]" in adapter
    assert "frame.motion[i] * value" in adapter
    assert "motion_delta(&s, false" in runtime
    assert "motion_delta(&s, true" in runtime
    assert "action_owns_arm(state, left)" in runtime


def test_player_motion_inputs_feed_hand_motion_runtime():
    runtime = RUNTIME.read_text()
    for field in [
        "player.applied_input[1]",
        "player.applied_input[0]",
        "player.velocity",
        "player.ground.on_ground",
        "player.sprinting",
        "player.swimming",
        "player.fall_flying",
        "player.flying",
        "player.sleeping",
    ]:
        assert field in runtime


def test_transient_render_frame_resets_motion_and_action_layers():
    adapter = (ENGINE / "horizon_pose.rs").read_text()
    runtime = RUNTIME.read_text()
    assert "motion: [Mat4::IDENTITY; 2]" in adapter
    assert "active: [false; 2]" in adapter
    assert "set_frame(punchy_engine::horizon_pose::Frame" in runtime


def test_bedrock_grip_pivot_uses_original_global_pivot_axes():
    resources = (ENGINE / "resources.rs").read_text()
    attachments = (ENGINE / "attachments.rs").read_text()
    assert "pub pivot: Vec3" in resources
    assert "pivot," in resources
    assert "Vec3::new(-pivot.x, pivot.z, pivot.y) / 16.0" in attachments
    assert "grip_position_uses_global_bedrock_pivot" in attachments


def test_punchy_pendulum_modes_and_tuning_profiles_are_wired():
    module = (ENGINE / "pendulum.rs").read_text()
    engine = (ENGINE / "engine.rs").read_text()
    tuning = (ENGINE / "tuning.rs").read_text()
    runtime = RUNTIME.read_text()
    attachments = (ENGINE / "attachments.rs").read_text()
    assert '#[path = "pendulum.rs"]' in engine
    assert "FullPendulum" in module and "ForwardPendulum" in module
    assert "pub fn physics_for" in tuning
    assert '"FULL_PENDULUM"' in tuning and '"FORWARD_PENDULUM"' in tuning
    assert "s.pendulum_states[i].update(pendulum_input" in runtime
    assert "attach_with_pendulum" in runtime and "state.matrix(item" in attachments


def test_pendulum_uses_camera_velocity_limits_and_hand_space_order():
    module = (ENGINE / "pendulum.rs").read_text()
    attachments = (ENGINE / "attachments.rs").read_text()
    for token in ["yaw_delta", "pitch_delta", "camera_sens", "gravity", "motion_sens", "ground_transition_cooldown", "let decay ="]:
        assert token in module
    assert "finite(Some(input.dt), 1.0 / 60.0)" in module
    assert "settings.hand_space != Some(false)" in attachments
    assert "settings.hand_space == Some(false)" in attachments


def test_vanilla_item_display_context_uses_punchy_up_vector_fallback():
    geometry = (ROOT / "mods/official/punchy/mixins/punchy/geometry.rs").read_text()
    assert "ThirdPersonRightHand" in geometry
    assert "FirstPersonRightHand" in geometry
    assert "third_up.dot(first_up) < 0.6" in geometry
    assert "!is_food && (is_block || kind != \"INGREDIENT\")" in geometry


def test_pendulum_uses_horizon_player_walk_animation_state():
    runtime = (ROOT / "mods/official/punchy/mixins/punchy/runtime.rs").read_text()
    assert "model.limb_speed, model.limb_position" in runtime
    assert "player.is_passenger" in runtime
    pendulum_input = runtime.split("fn pendulum_frame_input", 1)[1].split("fn trigger_pendulum_action", 1)[0]
    assert "if player.is_passenger" not in pendulum_input


def test_item_tuning_pivot_is_not_cancelled_by_item_pretransform():
    attachments = (ENGINE / "attachments.rs").read_text()
    assert "item *= Mat4::from_translation(-marker);" in attachments
    assert "item *= Mat4::from_translation(-pivot);" not in attachments


def test_pendulum_uses_processed_movement_and_original_action_spring():
    runtime = RUNTIME.read_text()
    module = (ENGINE / "pendulum.rs").read_text()
    assert "player.applied_input[0].clamp(-1.0, 1.0) * 1.5" in runtime
    assert "-player.applied_input[1].clamp(-1.0, 1.0) * 1.6" in runtime
    assert "0.7_f32.powf(dt * 20.0)" in runtime
    assert "state.pendulum_action_velocities[index] += 10.0" in runtime
    assert "-60.0 * state.pendulum_action_values[index]" in runtime
    assert "state.pendulum_action_kicks[index] += 1.0" in runtime
    assert "let consumed = s.pendulum_states[i].update" in runtime
    assert 'effect_active("disable_pendulum", time)' in runtime
    assert 'effect_active("disable_pendulum_physics", time)' in runtime
    assert "return false;" in module and "        true\n    }" in module


def test_renderer_up_vector_uses_glam_vec3_type():
    geometry = (ROOT / "mods/official/punchy/mixins/punchy/geometry.rs").read_text()
    assert "transform_vector3(glam::Vec3::Y)" in geometry
    assert "else { glam::Vec3::Y }" in geometry
    assert "transform_vector3(Vec3::Y)" not in geometry


def test_pendulum_physics_profiles_are_resolved_before_mutating_state():
    runtime = RUNTIME.read_text()
    assert "let source_bones = s.source.as_ref().map(|source| source.bones.clone());" in runtime
    assert "let physics_profiles: Option<[punchy_engine::tuning::PhysicsProfile; 2]>" in runtime
    assert "tuning.physics_for(&context, left)" in runtime
    assert "let physics = physics_profiles[i];" in runtime
    assert "let tuning = s.tuning.clone();" not in runtime


def test_render_item_id_is_owned_before_mutating_pendulum_state():
    runtime = RUNTIME.read_text()
    assert "let id = s.last_items[h].clone();" in runtime
    assert "let tags = held_tags(game, &id);" in runtime
    assert "let arm = editor_transform(&s, &id, left, \"arm\", &tags);" in runtime
    assert "let item = editor_transform(&s, &id, left, \"item\", &tags);" in runtime
    assert "let id = &s.last_items[h];" not in runtime
