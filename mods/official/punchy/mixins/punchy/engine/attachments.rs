//! Standard skin-arm and vanilla-item attachment chain recovered from
//! PunchyArmRenderer.applyGeoArmToPlayer/renderItemInHand. Procedural physics,
//! custom mesh renderers, visibility ownership and animated tuning weights are
//! separate layers; callers supply the last two tuning weights explicitly.
use crate::{
    Clip, Pose,
    skeleton::{
        Skeleton, joml_inverse, joml_quaternion_matrix, joml_rotation, joml_scale, v2_grip,
        view_flip,
    },
    tuning::Transform,
};
use glam::{EulerRot, Mat4, Quat, Vec3};

#[derive(Clone, Copy, Debug)]
pub struct StandardPose {
    /// Shoulder-space arm matrix, ready for a skin arm with its model rest
    /// transform removed. Includes Punchy's outer Z180 view conversion.
    pub arm: Mat4,
    /// Matrix before THIRD_PERSON_(LEFT|RIGHT)_HAND model display transform.
    pub item: Mat4,
}
fn rotation_xyz(degrees: [f32; 3]) -> Mat4 {
    Mat4::from_quat(Quat::from_euler(
        EulerRot::XYZ,
        degrees[0].to_radians(),
        degrees[1].to_radians(),
        degrees[2].to_radians(),
    ))
}
/// Source multiplies camera local * body local * arm local; it does not walk
/// camera_animation as an additional ancestor at this renderer boundary.
#[allow(clippy::too_many_arguments)]
pub fn standard_pose(
    skeleton: &Skeleton<'_>,
    left: bool,
    animation: Option<(&Clip, f32, f32)>,
    weight: f32,
    arm_tuning: &Transform,
    item_tuning: &Transform,
    tuning_factor: f32,
    arm_rotation_factor: f32,
    legacy_item_rotation_order: bool,
) -> StandardPose {
    standard_pose_owned(
        skeleton,
        left,
        animation,
        weight,
        true,
        arm_tuning,
        item_tuning,
        tuning_factor,
        arm_rotation_factor,
        legacy_item_rotation_order,
    )
}

/// Camera and body animation is shared even when the action owns one arm.
#[allow(clippy::too_many_arguments)]
pub fn standard_pose_owned(
    skeleton: &Skeleton<'_>,
    left: bool,
    animation: Option<(&Clip, f32, f32)>,
    weight: f32,
    owns_arm: bool,
    arm_tuning: &Transform,
    item_tuning: &Transform,
    tuning_factor: f32,
    arm_rotation_factor: f32,
    legacy_item_rotation_order: bool,
) -> StandardPose {
    attach(
        sample_frame(skeleton, left, animation, weight, owns_arm),
        left,
        arm_tuning,
        item_tuning,
        tuning_factor,
        arm_rotation_factor,
        legacy_item_rotation_order,
    )
}

/// Local transforms remain separate until after the live transition. Blending
/// completed world/item matrices would change the camera/body/arm hierarchy.
#[derive(Clone, Copy, Debug)]
pub struct AttachmentFrame {
    pub camera: Mat4,
    pub body: Mat4,
    pub arm: Mat4,
    pub grip_position: Vec3,
    pub grip_rotation: Mat4,
    pub grip_delta: Vec3,
    pub grip_scale: Vec3,
}
impl AttachmentFrame {
    pub fn blend(self, to: Self, weight: f32) -> Self {
        let w = weight.clamp(0., 1.);
        if w == 0. {
            return self;
        }
        if w == 1. {
            return to;
        }
        let blend = |a, b| crate::skeleton::blend_matrices(a, b, w);
        Self {
            camera: blend(self.camera, to.camera),
            body: blend(self.body, to.body),
            arm: blend(self.arm, to.arm),
            grip_position: self.grip_position.lerp(to.grip_position, w),
            grip_rotation: blend(self.grip_rotation, to.grip_rotation),
            grip_delta: self.grip_delta.lerp(to.grip_delta, w),
            grip_scale: self.grip_scale.lerp(to.grip_scale, w),
        }
    }
}
pub fn sample_frame(
    skeleton: &Skeleton<'_>,
    left: bool,
    animation: Option<(&Clip, f32, f32)>,
    weight: f32,
    owns_arm: bool,
) -> AttachmentFrame {
    let side = if left { "left" } else { "right" };
    let camera = skeleton.local("camera", animation, weight);
    let body = skeleton.local("body", animation, weight);
    let animation = animation.filter(|_| owns_arm);
    let arm = skeleton.local(&format!("{side}_arm"), animation, weight);
    let grip_name = format!("itemgrip_{side}");
    let base = skeleton.base(&grip_name).w_axis.truncate();
    let grip_position = Vec3::new(-base.x, base.z, -base.y);
    let grip_local = skeleton.local(&grip_name, animation, weight);
    let grip_scale = joml_scale(grip_local);
    let v2 = animation.is_some_and(|(c, t, _)| c.effect_active("use_keyframes_v2", t));
    let (grip_rotation, grip_delta) = if v2 {
        let (clip, time, expression) = animation.unwrap();
        let sample = Pose::default().blend(
            clip.sample(&grip_name, time, expression),
            weight.clamp(0., 1.),
        );
        let grip = v2_grip(sample.position / 16., sample.euler, Vec3::ONE);
        (
            v2_grip(Vec3::ZERO, sample.euler, Vec3::ONE),
            grip.w_axis.truncate(),
        )
    } else {
        let delta = skeleton.base(&grip_name).inverse() * grip_local;
        (
            joml_quaternion_matrix(joml_inverse(joml_rotation(grip_local))),
            delta.w_axis.truncate(),
        )
    };
    AttachmentFrame {
        camera,
        body,
        arm,
        grip_position,
        grip_rotation,
        grip_delta,
        grip_scale,
    }
}
#[allow(clippy::too_many_arguments)]
pub fn attach(
    frame: AttachmentFrame,
    left: bool,
    arm_tuning: &Transform,
    item_tuning: &Transform,
    tuning_factor: f32,
    arm_rotation_factor: f32,
    legacy_item_rotation_order: bool,
) -> StandardPose {
    let AttachmentFrame {
        camera,
        body,
        arm,
        grip_position,
        grip_rotation,
        grip_delta,
        grip_scale,
    } = frame;
    let mut arm = camera * body * arm;
    let factor = tuning_factor.clamp(0., 1.);
    // JOML.translateLocal pre-multiplies the world matrix. Source offsets are
    // converted to pixels before the ModelPart boundary divides them by16.
    arm = Mat4::from_translation(Vec3::from_array(arm_tuning.offset) * factor) * arm;
    arm *= rotation_xyz(
        arm_tuning
            .rotation
            .map(|r| r * factor * arm_rotation_factor.clamp(0., 1.)),
    );
    arm *= Mat4::from_scale(Vec3::splat(1. + (arm_tuning.scale - 1.) * factor));

    let tuning_pivot = Vec3::from_array(item_tuning.pivot);
    let marker = Vec3::new(if left { 0.04 } else { -0.04 }, -0.14, 0.11);
    let pivot = marker + tuning_pivot;
    let mut item = arm
        * Mat4::from_rotation_x(-std::f32::consts::FRAC_PI_2)
        * Mat4::from_rotation_y(std::f32::consts::PI)
        * Mat4::from_translation(Vec3::new(if left { -1. } else { 1. } / 16., 0.125, -0.625))
        * Mat4::from_translation(grip_position + grip_delta)
        * Mat4::from_translation(pivot)
        * grip_rotation
        * Mat4::from_translation(-pivot)
        * Mat4::from_scale(grip_scale)
        * Mat4::from_scale(Vec3::splat(item_tuning.scale))
        * Mat4::from_translation(tuning_pivot)
        * Mat4::from_translation(marker);
    let [x, y, z] = item_tuning.rotation.map(f32::to_radians);
    item *= if legacy_item_rotation_order {
        Mat4::from_rotation_z(z) * Mat4::from_rotation_x(x) * Mat4::from_rotation_y(y)
    } else {
        rotation_xyz(item_tuning.rotation)
    };
    item *= Mat4::from_translation(-marker)
        * Mat4::from_translation(
            Vec3::from_array(item_tuning.offset)
                + Vec3::new(if left { 0.04 } else { -0.04 }, -0.03, -0.02),
        );
    StandardPose {
        arm: view_flip() * arm,
        item: view_flip() * item,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::{GEOMETRY, member, parse_bones};
    #[test]
    fn actual_source_rest_arm_pivots_and_item_alignment_are_finite() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../MODS/newmods/punchy-3.1b-fabric-26.3.jar");
        let bones = parse_bones(&member(&root, GEOMETRY).unwrap()).unwrap();
        let skeleton = Skeleton::new(&bones).unwrap();
        let r = standard_pose(
            &skeleton,
            false,
            None,
            1.,
            &Transform::default(),
            &Transform::default(),
            1.,
            1.,
            false,
        );
        let l = standard_pose(
            &skeleton,
            true,
            None,
            1.,
            &Transform::default(),
            &Transform::default(),
            1.,
            1.,
            false,
        );
        let p = r.arm.transform_point3(Vec3::ZERO);
        assert!((p - Vec3::new(11. / 16., -11.4 / 16., -7. / 16.)).length() < 1e-5);
        assert!((l.arm.transform_point3(Vec3::ZERO).x + 11. / 16.).abs() < 1e-5);
        assert!(r.item.is_finite() && l.item.is_finite());
        assert!((r.item.determinant() - 1.).abs() < 1e-5);
    }
    #[test]
    fn pickaxe_inspect_moves_the_item_through_the_matching_hand_grip() {
        let resources = crate::resources::embedded().unwrap();
        let skeleton = Skeleton::new(&resources.bones).unwrap();
        let clips = crate::load_clips(&resources.animations).unwrap();
        let transform = Transform::default();
        for (name, left, grip) in [
            ("new_inspect_pickaxe_left", true, "itemgrip_left"),
            ("new_inspect_pickaxe_right", false, "itemgrip_right"),
        ] {
            let clip = &clips[name];
            assert!(clip.tracks.contains_key(grip));
            let rest = sample_frame(&skeleton, left, None, 1., true);
            let inspected = sample_frame(&skeleton, left, Some((clip, 0.25, 0.25)), 1., true);
            assert!((inspected.grip_delta - rest.grip_delta).length() > 0.001
                || !inspected.grip_rotation.abs_diff_eq(rest.grip_rotation, 0.001));
            let rest_pose = attach(rest, left, &transform, &transform, 1., 1., false);
            let inspected_pose = attach(inspected, left, &transform, &transform, 1., 1., false);
            assert!(!inspected_pose.item.abs_diff_eq(rest_pose.item, 0.001));
        }
    }

    #[test]
    fn unowned_arm_keeps_shared_body_motion_without_its_arm_or_grip_track() {
        let resources = crate::resources::embedded().unwrap();
        let skeleton = Skeleton::new(&resources.bones).unwrap();
        let clips=crate::load_clips(br#"{"animations":{"shared":{"effects":"use_keyframes_v2","bones":{"body":{"position":[8,0,0]},"left_arm":{"rotation":[70,0,0]},"itemgrip_left":{"position":[0,30,0]}}},"root_only":{"effects":"use_keyframes_v2","bones":{"body":{"position":[8,0,0]}}}}}"#).unwrap();
        let tuning = Transform::default();
        let shared = standard_pose_owned(
            &skeleton,
            true,
            Some((&clips["shared"], 0., 0.)),
            1.,
            false,
            &tuning,
            &tuning,
            1.,
            1.,
            false,
        );
        let expected = standard_pose(
            &skeleton,
            true,
            Some((&clips["root_only"], 0., 0.)),
            1.,
            &tuning,
            &tuning,
            1.,
            1.,
            false,
        );
        let idle = standard_pose(&skeleton, true, None, 1., &tuning, &tuning, 1., 1., false);
        assert!(shared.arm.abs_diff_eq(expected.arm, 1e-6));
        assert!(shared.item.abs_diff_eq(expected.item, 1e-6));
        assert!(!shared.arm.abs_diff_eq(idle.arm, 1e-6));
    }
    #[test]
    fn zero_tuning_weight_removes_arm_override_without_removing_item_tuning() {
        let bones=crate::resources::parse_bones(br#"{"minecraft:geometry":[{"bones":[{"name":"camera"},{"name":"body","parent":"camera"},{"name":"right_arm","parent":"body"},{"name":"itemgrip_right","parent":"right_arm"}]}]}"#).unwrap();
        let skeleton = Skeleton::new(&bones).unwrap();
        let none = Transform::default();
        let arm = Transform {
            offset: [10., 3., -2.],
            rotation: [80., 20., 30.],
            scale: 3.,
            ..Default::default()
        };
        let tuned = standard_pose(&skeleton, false, None, 1., &arm, &none, 0., 1., false);
        let base = standard_pose(&skeleton, false, None, 1., &none, &none, 1., 1., false);
        assert!(tuned.arm.abs_diff_eq(base.arm, 1e-6));
        let item = Transform {
            offset: [1., 0., 0.],
            ..Default::default()
        };
        let changed = standard_pose(&skeleton, false, None, 1., &arm, &item, 0., 1., false);
        assert!(changed.arm.abs_diff_eq(base.arm, 1e-6));
        assert!(!changed.item.abs_diff_eq(base.item, 1e-6));
    }
}
