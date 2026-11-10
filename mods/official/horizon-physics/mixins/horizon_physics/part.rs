// A model part is about to be drawn: while a dying mob is captured, note where its vertices start.
fn horizon_physics_part(out: &Vec<Vertex>) {
    crate::native_horizon_physics::capture_mark(out.len());
}
