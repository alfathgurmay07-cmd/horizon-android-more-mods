// Options' first page: a wide "Physics Settings..." button in the gap above the others (on a
// short screen: beside Credits, in the last row's free half). Its press is the mouse hook's
// (it knows where the button is from here).
fn horizon_physics_options_layout(
    ui: &mut OptionsUi,
    s: &Settings,
    w: f32,
    h: f32,
) -> ::core::ops::ControlFlow<(Vec<Widget>, f32, f32, f32)> {
    use crate::native_horizon_physics_sway as sway;
    if ui.page() != Page::Main || !sway::physics_button_wanted() || !sway::layout_enter() {
        return ::core::ops::ControlFlow::Continue(());
    }
    let (mut widgets, top, bottom, content) = ui.layout(s, w, h);
    sway::layout_leave();
    let grid: Vec<usize> = widgets
        .iter()
        .enumerate()
        .filter(|e| e.1.w == 150.0 && e.1.y > 40.0 && e.1.y < h - 30.0)
        .map(|e| e.0)
        .collect();
    let gy = grid.iter().map(|i| widgets[*i].y).fold(f32::INFINITY, f32::min);
    let last = grid.iter().map(|i| widgets[*i].y).fold(f32::NEG_INFINITY, f32::max);
    if gy.is_finite() {
        let cx = (w / 2.0).floor();
        // Room between the FOV row (down to 49) and the grid: the wide button goes there and
        // nothing moves.
        let wide = gy - 24.0;
        // Else the free half of the grid's last row (Credits stands alone there), else the grid
        // moves down a row, as long as it stays clear of Done.
        let alone = grid.iter().filter(|i| (widgets[**i].y - last).abs() < 0.5).count() == 1;
        if wide >= 53.0 {
            widgets.push(button(cx - 154.0, wide, 308.0, Text::literal("Physics Settings..."), Act::None));
            sway::set_physics_button([cx - 154.0, wide, 308.0, 20.0]);
        } else if alone {
            widgets.push(button(cx + 4.0, last, 150.0, Text::literal("Physics Settings..."), Act::None));
            sway::set_physics_button([cx + 4.0, last, 150.0, 20.0]);
        } else if last + 24.0 + 20.0 <= h - 31.0 {
            for i in &grid {
                widgets[*i].y += 24.0;
            }
            widgets.push(button(cx - 154.0, gy, 308.0, Text::literal("Physics Settings..."), Act::None));
            sway::set_physics_button([cx - 154.0, gy, 308.0, 20.0]);
        } else {
            sway::set_physics_button([-1000.0, -1000.0, 0.0, 0.0]);
        }
    }
    ::core::ops::ControlFlow::Break((widgets, top, bottom, content))
}
