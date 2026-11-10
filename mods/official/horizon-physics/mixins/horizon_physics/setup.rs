// Horizon Physics' own screens: the quick setup (on the title screen the first time the game
// starts with the mod, and from the Physics Settings screen) and the Physics Settings screen
// (Options > Physics Settings...). Their layout, pages and live pictures are the runtime's
// (`setup_ui`, `menu_ui`, `SetupScene`); this module draws them with the HUD's own quads and
// text, and the game's own textures from its atlas.
use crate::hud::{HudAtlas, HudBatch, rgba};
use crate::native_horizon_physics as hp;

fn color(c: [u8; 4]) -> u32 {
    rgba(c[0], c[1], c[2], c[3])
}

fn unit(v: f32) -> u8 {
    (v.clamp(0.0, 1.0) * 255.0).round() as u8
}

/// Draws `items`, the pictures from `scenes`.
fn draw_items(atlas: &HudAtlas, batch: &mut HudBatch, items: Vec<hp::SetupUi>, scenes: &[&hp::SetupScene]) {
    let white = atlas.font.white();
    let flat = [(white.u0 + white.u1) * 0.5, (white.v0 + white.v1) * 0.5];
    let sprites: Vec<Option<[f32; 4]>> = hp::SETUP_SPRITES
        .iter()
        .map(|n| if n.is_empty() { None } else { atlas.sprites.get(*n).map(|r| [r.u0, r.v0, r.u1, r.v1]) })
        .collect();
    let mut clip: Option<(usize, [f32; 4])> = None;
    for item in items {
        match item {
            hp::SetupUi::Clip { x, y, w, h } => clip = Some((batch.vertices.len(), [x, y, x + w, y + h])),
            hp::SetupUi::Unclip => {
                if let Some((start, r)) = clip.take() {
                    batch.clip_since(start, r[0], r[1], r[2], r[3]);
                }
            }
            hp::SetupUi::Rect { x, y, w, h, color: c } => batch.fill(atlas, x, y, w, h, color(c)),
            hp::SetupUi::Gradient { x, y, w, h, top, bottom } => batch.fill_gradient(atlas, (x, y, w, h), color(top), color(bottom)),
            hp::SetupUi::Text { text, x, y, color: c } => {
                let tw = HudBatch::text_width(atlas, &text);
                batch.text(atlas, &text, (x - tw * 0.5).round(), y, color(c), true);
            }
            hp::SetupUi::Label { text, x, y, color: c, scale, left } => {
                let tw = HudBatch::text_width(atlas, &text) * scale;
                let x0 = if left { x } else { x - tw * 0.5 };
                let start = batch.vertices.len();
                batch.text(atlas, &text, x0, y, color(c), true);
                if (scale - 1.0).abs() > 1e-3 {
                    batch.scale_since(start, x0, y, scale, scale);
                }
            }
            hp::SetupUi::Tiles { x, y, w, h, sprite, size, color: c } => {
                let Some(r) = sprites.get(usize::from(sprite)).copied().flatten() else {
                    batch.fill(atlas, x, y, w, h, color(c));
                    continue;
                };
                let start = batch.vertices.len();
                let mut ty = y;
                while ty < y + h {
                    let mut tx = x;
                    while tx < x + w {
                        batch.quad(
                            [[tx, ty], [tx + size, ty], [tx + size, ty + size], [tx, ty + size]],
                            [[r[0], r[1]], [r[2], r[1]], [r[2], r[3]], [r[0], r[3]]],
                            color(c),
                        );
                        tx += size;
                    }
                    ty += size;
                }
                batch.clip_since(start, x, y, x + w, y + h);
            }
            hp::SetupUi::Scene { index, x, y, w: sw, h: sh } => {
                let Some(scene) = scenes.get(index) else {
                    continue;
                };
                let mut quads = scene.quads(x + sw * 0.5, y + sh * 0.5, sw, sh);
                // The solid ones first, then the see-through ones (shadows, rain, smoke) over them.
                quads.sort_by_key(|q| q.color[3] < 1.0);
                let start = batch.vertices.len();
                for q in quads {
                    let uv = match sprites.get(usize::from(q.sprite)).copied().flatten() {
                        Some(r) => q.uv.map(|t| [r[0] + (r[2] - r[0]) * t[0], r[1] + (r[3] - r[1]) * t[1]]),
                        None => [flat; 4],
                    };
                    let c = rgba(unit(q.color[0]), unit(q.color[1]), unit(q.color[2]), unit(q.color[3]));
                    batch.quad_3d(q.p, uv, c);
                }
                batch.depth_since(start, [x, y, x + sw, y + sh]);
            }
        }
    }
}

/// Draws the quick setup over the whole screen, the pointer at `cursor`.
pub fn draw(atlas: &HudAtlas, batch: &mut HudBatch, cursor: (f32, f32)) {
    let (w, h) = (batch.width.ceil(), batch.height.ceil());
    hp::with_setup(|s| {
        let items = hp::setup_ui(s.page, w, h, cursor);
        let scenes: Vec<&hp::SetupScene> = s.scenes.iter().collect();
        draw_items(atlas, batch, items, &scenes);
    });
}

/// Draws the Physics Settings screen (or the quick setup it opened) over the whole screen.
pub fn draw_menu(atlas: &HudAtlas, batch: &mut HudBatch, cursor: (f32, f32)) {
    if hp::menu_view() == hp::MenuView::Setup {
        draw(atlas, batch, cursor);
        return;
    }
    let (w, h) = (batch.width.ceil(), batch.height.ceil());
    hp::with_menu(|m| {
        let items = hp::menu_ui(m, w, h, cursor);
        draw_items(atlas, batch, items, &[&m.picture]);
    });
}
