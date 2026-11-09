fn freecam_camera(partial: f32) -> core::ops::ControlFlow<Option<(glam::DVec3, f32, f32)>> {
    crate::game::umbx_freecam_glue::camera(partial)
}
