fn freecam_turn(xo: f64, yo: f64) -> core::ops::ControlFlow<()> {
    crate::game::umbx_freecam_glue::turn(xo, yo)
}
