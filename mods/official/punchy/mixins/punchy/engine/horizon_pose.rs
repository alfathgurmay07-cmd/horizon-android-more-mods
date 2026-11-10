//! Revision-specific first-person pose adapter shared by the game binary and
//! `horizon-client` library through this engine crate. This applies additive
//! authored deltas to existing Horizon poses; it is not Punchy's full renderer.
use glam::Mat4;
use std::cell::Cell;

/// Physical arm indices: right = 0 (`left == false`), left = 1 (`left == true`).
#[derive(Clone, Copy, Debug)]
pub struct Frame {
    pub authored: [Option<crate::attachments::StandardPose>; 2],
    pub enabled: bool,
    pub deltas: [Mat4; 2],
    pub motion: [Mat4; 2],
    pub active: [bool; 2],
}
impl Default for Frame {
    fn default() -> Self {
        Self {
            authored: [None, None],
            enabled: false,
            deltas: [Mat4::IDENTITY; 2],
            motion: [Mat4::IDENTITY; 2],
            active: [false; 2],
        }
    }
}
thread_local! {
    static FRAME: Cell<Frame> = Cell::new(Frame::default());
}
/// Publish a complete frame at the beginning of first-person rendering.
/// Replacing the whole frame clears ownership left over from the previous one.
pub fn set_frame(frame: Frame) {
    FRAME.set(frame);
}
/// Apply the published view-space delta before the existing pose matrix.
pub fn transform(left: bool, value: Mat4) -> Mat4 {
    FRAME.with(|slot| {
        let frame = slot.get();
        let i = usize::from(left);
        if frame.enabled {
            let value = frame.motion[i] * value;
            if frame.active[i] {
                frame.deltas[i] * value
            } else {
                value
            }
        } else {
            value
        }
    })
}
/// Whether the current clip owns this physical arm and replaces vanilla swing.
pub fn motion(left: bool) -> Mat4 {
    FRAME.with(|slot| {
        let frame = slot.get();
        if frame.enabled { frame.motion[usize::from(left)] } else { Mat4::IDENTITY }
    })
}
pub fn suppress_swing(left: bool) -> bool {
    FRAME.with(|slot| {
        let frame = slot.get();
        frame.enabled && frame.active[usize::from(left)]
    })
}

/// Authored source-skeleton poses, when the runtime opted into that renderer.
pub fn authored(left: bool) -> Option<crate::attachments::StandardPose> {
    FRAME.with(|slot| {
        let frame = slot.get();
        if frame.enabled {
            frame.authored[usize::from(left)]
        } else {
            None
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;
    #[test]
    fn disabled_adapter_preserves_pose_even_with_active_clip() {
        let pose = Mat4::from_rotation_y(0.4);
        set_frame(Frame {
            authored: [None, None],
            enabled: false,
            active: [true; 2],
            deltas: [Mat4::from_scale(Vec3::splat(2.0)); 2],
            motion: [Mat4::IDENTITY; 2],
        });
        assert_eq!(transform(false, pose), pose);
        assert_eq!(transform(true, pose), pose);
        assert!(!suppress_swing(false));
        assert!(!suppress_swing(true));
    }
    #[test]
    fn motion_layer_applies_without_taking_over_vanilla_swing() {
        let base = Mat4::from_translation(Vec3::new(0.0, 0.0, 1.0));
        let delta = Mat4::from_translation(Vec3::new(0.2, 0.0, 0.0));
        set_frame(Frame {
            authored: [None, None],
            enabled: true,
            deltas: [Mat4::IDENTITY; 2],
            motion: [delta; 2],
            active: [false; 2],
        });
        assert_eq!(transform(false, base), delta * base);
        assert!(!suppress_swing(false));
        assert_eq!(motion(true), delta);
    }

    #[test]
    fn physical_arm_ownership_is_independent_and_new_frame_clears_it() {
        let base = Mat4::from_rotation_z(0.3);
        let right = Mat4::from_translation(Vec3::new(1.0, 2.0, 3.0));
        let left = Mat4::from_rotation_x(0.7);
        set_frame(Frame {
            authored: [None, None],
            enabled: true,
            active: [true, false],
            deltas: [right, left],
            motion: [Mat4::IDENTITY; 2],
        });
        assert_eq!(transform(false, base), right * base);
        assert_eq!(transform(true, base), base);
        assert!(suppress_swing(false));
        assert!(!suppress_swing(true));
        set_frame(Frame {
            authored: [None, None],
            enabled: true,
            active: [false, true],
            deltas: [right, left],
            motion: [Mat4::IDENTITY; 2],
        });
        assert_eq!(transform(false, base), base);
        assert_eq!(transform(true, base), left * base);
        assert!(!suppress_swing(false));
        assert!(suppress_swing(true));
        set_frame(Frame::default());
        assert_eq!(transform(true, base), base);
        assert!(!suppress_swing(true));
    }
}
