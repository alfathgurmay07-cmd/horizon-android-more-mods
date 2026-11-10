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
