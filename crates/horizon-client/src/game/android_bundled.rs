//! Fixed Android mod bundle. No phone-side native-mod loader or compiler.

use super::*;

use super::native_hooks::*;


#[cfg(target_os = "android")]
include!("../../../../mods/official/basic-hud/mixins/basic_hud/list.rs");
#[cfg(target_os = "android")]
include!("../../../../mods/official/basic-hud/mixins/basic_hud/settings.rs");
#[cfg(target_os = "android")]
include!("../../../../mods/official/basic-hud/mixins/basic_hud/click.rs");
#[cfg(target_os = "android")]
include!("../../../../mods/official/basic-hud/mixins/basic_hud/hud.rs");
#[cfg(target_os = "android")]
include!("../../../../mods/official/basic-hud/mixins/basic_hud/tick.rs");
#[cfg(target_os = "android")]
include!("../../../../mods/official/basic-hud/mixins/basic_hud/frame.rs");
#[cfg(target_os = "android")]
include!("../../../../mods/official/basic-hud/mixins/basic_hud/key.rs");

#[cfg(target_os = "android")]
include!("../../../../mods/official/visuality-horizon/mixins/visuality_horizon/tick.rs");
#[cfg(target_os = "android")]
include!("../../../../mods/official/visuality-horizon/mixins/visuality_horizon/frame.rs");
#[cfg(target_os = "android")]
include!("../../../../mods/official/visuality-horizon/mixins/visuality_horizon/list.rs");
#[cfg(target_os = "android")]
include!("../../../../mods/official/visuality-horizon/mixins/visuality_horizon/settings.rs");
#[cfg(target_os = "android")]
include!("../../../../mods/official/visuality-horizon/mixins/visuality_horizon/settings_click.rs");

#[cfg(target_os = "android")]
include!("../../../../mods/official/armorhud/mixins/armorhud/list.rs");
#[cfg(target_os = "android")]
include!("../../../../mods/official/armorhud/mixins/armorhud/settings.rs");
#[cfg(target_os = "android")]
include!("../../../../mods/official/armorhud/mixins/armorhud/click.rs");
#[cfg(target_os = "android")]
include!("../../../../mods/official/armorhud/mixins/armorhud/key.rs");
#[cfg(target_os = "android")]
include!("../../../../mods/official/armorhud/mixins/armorhud/hud.rs");

#[cfg(target_os = "android")]
include!("../../../../mods/official/jade/mixins/jade/frame.rs");
#[cfg(target_os = "android")]
include!("../../../../mods/official/jade/mixins/jade/hud.rs");
#[cfg(target_os = "android")]
include!("../../../../mods/official/jade/mixins/jade/list.rs");
#[cfg(target_os = "android")]
include!("../../../../mods/official/jade/mixins/jade/settings.rs");
#[cfg(target_os = "android")]
include!("../../../../mods/official/jade/mixins/jade/settings_click.rs");
#[cfg(target_os = "android")]
include!("../../../../mods/official/jade/mixins/jade/net_from_client.rs");
#[cfg(target_os = "android")]
include!("../../../../mods/official/jade/mixins/jade/net_from_server.rs");
#[cfg(target_os = "android")]
include!("../../../../mods/official/jade/mixins/jade/jade_command.rs");
#[cfg(target_os = "android")]
include!("../../../../mods/official/jade/mixins/jade/jade_perform_command.rs");
#[cfg(target_os = "android")]
include!("../../../../mods/official/jade/mixins/jade/jade_perform_console.rs");
#[cfg(target_os = "android")]
include!("../../../../mods/official/jade/mixins/jade/jade_register_commands.rs");
#[cfg(target_os = "android")]
include!("../../../../mods/official/jade/mixins/jade/screen_render_post.rs");
#[cfg(target_os = "android")]
include!("../../../../mods/official/jade/mixins/jade/screen_pressed.rs");
#[cfg(target_os = "android")]
include!("../../../../mods/official/jade/mixins/jade/screen_released.rs");
#[cfg(target_os = "android")]
include!("../../../../mods/official/jade/mixins/jade/screen_scrolled.rs");
#[cfg(target_os = "android")]
include!("../../../../mods/official/jade/mixins/jade/screen_key.rs");
#[cfg(target_os = "android")]
include!("../../../../mods/official/jade/mixins/jade/screen_typed.rs");
#[cfg(target_os = "android")]
include!("../../../../mods/official/jade/mixins/jade/resources_reloaded.rs");

#[cfg(target_os = "android")]
include!("../../../../mods/official/resourcify/mixins/resourcify/listed.rs");
#[cfg(target_os = "android")]
include!("../../../../mods/official/resourcify/mixins/resourcify/frame.rs");

#[cfg(target_os = "android")]
#[path = "../../../../mods/official/accurate-block-placement/src/runtime.rs"]
pub(super) mod accurate_block_placement;

#[cfg(target_os = "android")]
#[path = "../../../../mods/official/accurate-block-placement/src/key.rs"]
mod accurate_block_placement_key;
#[cfg(target_os = "android")]
#[path = "../../../../mods/official/accurate-block-placement/src/list.rs"]
mod accurate_block_placement_list;
#[cfg(target_os = "android")]
#[path = "../../../../mods/official/accurate-block-placement/src/settings.rs"]
mod accurate_block_placement_settings;
#[cfg(target_os = "android")]
#[path = "../../../../mods/official/accurate-block-placement/src/click.rs"]
mod accurate_block_placement_click;
#[cfg(target_os = "android")]
#[path = "../../../../mods/official/accurate-block-placement/src/frame.rs"]
mod accurate_block_placement_frame;

#[cfg(target_os = "android")]
#[path = "../../../../mods/official/connected-textures/src/frame.rs"]
mod connected_textures_frame;
#[cfg(target_os = "android")]
#[path = "../../../../mods/official/connected-textures/src/list.rs"]
mod connected_textures_list;
#[cfg(target_os = "android")]
#[path = "../../../../mods/official/connected-textures/src/settings.rs"]
mod connected_textures_settings;
#[cfg(target_os = "android")]
#[path = "../../../../mods/official/connected-textures/src/click.rs"]
mod connected_textures_click;

#[cfg(target_os = "android")]
#[path = "android_bundled/zoom/runtime.rs"]
pub(super) mod official_zoom;

#[cfg(target_os = "android")]
#[path = "android_bundled/zoom/zoom_key.rs"]
mod zoom_key;

#[cfg(target_os = "android")]
#[path = "android_bundled/zoom/zoom_wheel.rs"]
mod zoom_wheel;

#[cfg(target_os = "android")]
#[path = "android_bundled/zoom/zoom_zoom.rs"]
mod zoom_zoom;

#[cfg(target_os = "android")]
#[path = "android_bundled/zoom/zoom_list.rs"]
mod zoom_list;

#[cfg(target_os = "android")]
#[path = "android_bundled/zoom/zoom_settings.rs"]
mod zoom_settings;

#[cfg(target_os = "android")]
#[path = "android_bundled/zoom/zoom_click.rs"]
mod zoom_click;

#[cfg(target_os = "android")]
#[path = "android_bundled/minimap/runtime.rs"]
pub(super) mod official_minimap;

#[cfg(target_os = "android")]
#[path = "android_bundled/minimap/minimap_list.rs"]
mod minimap_list;

#[cfg(target_os = "android")]
#[path = "android_bundled/minimap/minimap_settings.rs"]
mod minimap_settings;

#[cfg(target_os = "android")]
#[path = "android_bundled/minimap/minimap_click.rs"]
mod minimap_click;

#[cfg(target_os = "android")]
#[path = "android_bundled/minimap/minimap_minimap.rs"]
mod minimap_minimap;

#[cfg(target_os = "android")]
#[path = "android_bundled/recipes/runtime.rs"]
pub(super) mod official_recipes;

#[cfg(target_os = "android")]
#[path = "android_bundled/recipes/glue.rs"]
pub(super) mod official_recipes_glue;

#[cfg(target_os = "android")]
#[path = "android_bundled/recipes/recipes_list.rs"]
mod recipes_list;

#[cfg(target_os = "android")]
#[path = "android_bundled/recipes/recipes_settings.rs"]
mod recipes_settings;

#[cfg(target_os = "android")]
#[path = "android_bundled/recipes/recipes_click.rs"]
mod recipes_click;

#[cfg(target_os = "android")]
#[path = "android_bundled/recipes/recipes_frame.rs"]
mod recipes_frame;

#[cfg(target_os = "android")]
#[path = "android_bundled/recipes/recipes_event.rs"]
mod recipes_event;

#[cfg(target_os = "android")]
#[path = "android_bundled/smartf3/runtime.rs"]
pub(super) mod smartf3;

#[cfg(target_os = "android")]
#[path = "android_bundled/smartf3/native.rs"]
pub(super) mod smartf3_native;

#[cfg(target_os = "android")]
#[path = "android_bundled/smartf3/smartf3_command.rs"]
mod smartf3_command;

#[cfg(target_os = "android")]
#[path = "android_bundled/smartf3/smartf3_list.rs"]
mod smartf3_list;

#[cfg(target_os = "android")]
#[path = "android_bundled/smartf3/smartf3_settings.rs"]
mod smartf3_settings;

#[cfg(target_os = "android")]
#[path = "android_bundled/smartf3/smartf3_click.rs"]
mod smartf3_click;

pub(super) fn native_key(game: &mut Game<'_>, key: &str, pressed: bool) {
    let _ = (&game, &key, &pressed);
    #[cfg(target_os = "android")]
    {
        zoom_key::run(&mut *game, key, pressed);
        accurate_block_placement_key::run(&mut *game, key, pressed);
        super::freecam_hooks::key(&mut *game, key, pressed);
        super::punchy::key(&mut *game, key, pressed);
        armorhud_key(&mut *game, key, pressed);
        basic_hud_key(&mut *game, key, pressed);
    }
}

pub(super) fn native_wheel(game: &mut Game<'_>, steps: f32, claimed: &mut bool) {
    let _ = (&game, &steps, &claimed);
    #[cfg(target_os = "android")]
    {
        zoom_wheel::run(&*game, steps, &mut *claimed);
    }
}

pub(super) fn native_zoom(game: &Game<'_>, zoom: &mut Option<NativeZoom>) {
    let _ = (&game, &zoom);
    #[cfg(target_os = "android")]
    {
        zoom_zoom::run(game, &mut *zoom);
    }
}

pub(super) fn native_mods(game: &mut Game<'_>, mods: &mut Vec<NativeMod>) {
    let _ = (&game, &mods);
    #[cfg(target_os = "android")]
    {
        zoom_list::run(&mut *mods);
        minimap_list::run(&mut *mods);
        recipes_list::run(&mut *mods);
        smartf3_list::run(&mut *mods);
        accurate_block_placement_list::run(&mut *mods);
        connected_textures_list::run(&mut *mods);
        super::mod_appleskin_glue::on_list(&mut *mods);
        armorhud_listed(&mut *mods);
        jade_listed(&mut *mods);
        resourcify_listed(&mut *mods);
        basic_hud_listed(&mut *mods, &mut *game);
        visuality_listed(&mut *mods);
        super::freecam_hooks::list(&mut *mods);
        mods.push(NativeMod {
            id: super::punchy::ID.into(),
            title: super::punchy::TITLE.into(),
            description: "Self-contained Rust animation preview. Animation data included.".into(),
            keys: vec![
                ["inspect".into(), "Inspect item".into(), "key.keyboard.f6".into()],
                ["reload".into(), "Reload animations".into(), "key.keyboard.f7".into()],
            ],
        });
    }
}

pub(super) fn native_settings(game: &mut Game<'_>, mod_id: &str, buttons: &mut Vec<String>) {
    let _ = (&game, &mod_id, &buttons);
    #[cfg(target_os = "android")]
    {
        zoom_settings::run(&mut *game, mod_id, &mut *buttons);
        minimap_settings::run(&mut *game, mod_id, &mut *buttons);
        recipes_settings::run(&mut *game, mod_id, &mut *buttons);
        smartf3_settings::run(&mut *game, mod_id, &mut *buttons);
        accurate_block_placement_settings::run(&mut *game, mod_id, &mut *buttons);
        connected_textures_settings::run(mod_id, &mut *buttons);
        super::mod_appleskin_glue::on_settings(&mut *game, mod_id, &mut *buttons);
        super::freecam_hooks::settings(&mut *game, mod_id, &mut *buttons);
        if mod_id == super::punchy::ID {
            super::punchy::settings(&*game, &mut *buttons);
        }
        armorhud_page(&mut *game, mod_id, &mut *buttons);
        jade_settings(&mut *game, mod_id, &mut *buttons);
        basic_hud_page(&mut *game, mod_id, &mut *buttons);
        visuality_page(&mut *game, mod_id, &mut *buttons);
    }
}

pub(super) fn native_settings_click(game: &mut Game<'_>, mod_id: &str, button: usize) {
    let _ = (&game, &mod_id, &button);
    #[cfg(target_os = "android")]
    {
        zoom_click::run(&mut *game, mod_id, button);
        minimap_click::run(&mut *game, mod_id, button);
        recipes_click::run(&mut *game, mod_id, button);
        smartf3_click::run(&mut *game, mod_id, button);
        accurate_block_placement_click::run(&mut *game, mod_id, button);
        connected_textures_click::run(mod_id, button);
        super::mod_appleskin_glue::on_click(&mut *game, mod_id, button);
        super::freecam_hooks::click(&mut *game, mod_id, button);
        if mod_id == super::punchy::ID {
            super::punchy::click(&*game, button);
        }
        armorhud_click(&mut *game, mod_id, button);
        jade_settings_click(&mut *game, mod_id, button);
        basic_hud_click(&mut *game, mod_id, button);
        visuality_click(&mut *game, mod_id, button);
    }
}

pub(super) fn native_minimap(game: &Game<'_>, minimap: &mut Option<NativeMinimap>) {
    let _ = (&game, &minimap);
    #[cfg(target_os = "android")]
    {
        minimap_minimap::run(game, &mut *minimap);
    }
}

#[cfg(target_os = "android")]
fn accurate_block_placement_tick(game: &mut Game<'_>) {
    use accurate_block_placement as abp;

    let Some(player) = game
        .player
        .as_ref()
        .map(|p| [p.position.x, p.position.y, p.position.z])
    else {
        abp::reset();
        return;
    };

    let hand = if game.held().is_some() {
        Hand::Main
    } else {
        Hand::Off
    };

    for sample in abp::due((game.yaw, game.pitch)) {
        if !abp::still_fresh(&sample, player) {
            continue;
        }

        let hit = Hit {
            pos: sample.pos,
            face: sample.face,
            point: sample.point,
        };
        let placed_at = sample.placement();
        let before = game.world.block(placed_at[0], placed_at[1], placed_at[2]);
        let held_before = game.with_hand(hand, |g| g.held().map(|stack| stack.count));
        let used = game.with_hand(hand, |g| g.use_hand(Some(&hit)));

        if matches!(used, Ok(true)) {
            // Prevent the ordinary held-use path from placing a duplicate.
            game.use_delay = 4;

            if !game.use_failed {
                let after = game.world.block(placed_at[0], placed_at[1], placed_at[2]);
                let held_after = game.with_hand(hand, |g| g.held().map(|stack| stack.count));
                let consumed = held_after < held_before;
                if after != before || consumed || game.creative {
                    abp::placed(&sample, placed_at, player);
                }
            }
        }
    }
}

pub(super) fn native_tick(game: &mut Game<'_>) {
    let _ = &game;
    #[cfg(target_os = "android")]
    {
        super::freecam_hooks::tick(&mut *game);
        super::punchy::tick(&mut *game);
        basic_hud_tick(&mut *game);
        visuality_tick(&mut *game);
        if game.native_window() {
            accurate_block_placement_tick(&mut *game);
        }
    }
}

#[cfg(target_os = "android")]
pub(super) fn punchy_hand_geometry(game: &mut Game<'_>, alpha: f64) -> Option<Vec<Vertex>> {
    super::punchy::begin(game, alpha);
    if !super::punchy::ready(game) {
        return None;
    }
    let mut out = super::punchy_geometry::render(game, alpha);
    super::punchy::arms(game, alpha, &mut out);
    for vertex in &mut out {
        vertex.layer &= !horizon_client::gfx::OIT_ENTITY;
    }
    Some(out)
}

pub(super) fn native_frame(game: &mut Game<'_>, elapsed: Duration) {
    let _ = (&game, &elapsed);
    #[cfg(target_os = "android")]
    {
        super::freecam_hooks::frame(&*game, elapsed);
        recipes_frame::run(&mut *game);
        accurate_block_placement_frame::run(&mut *game);
        connected_textures_frame::run(&mut *game);
        jade_frame(&mut *game);
        resourcify_frame(&mut *game);
        basic_hud_frame(&mut *game, elapsed);
        visuality_frame(&mut *game);
    }
}

pub(super) fn native_hud(
    game: &mut Game<'_>,
    batch: &mut horizon_client::hud::HudBatch,
    corners: &mut horizon_client::extras_hud::Corners,
) {
    #[cfg(target_os = "android")]
    {
        super::freecam_hooks::hud(game, batch);
        armorhud_hud(game, batch, corners);
        jade_hud(game, batch);
        basic_hud_draw(game, batch, corners);
    }
    let _ = (game, batch, corners);
}

pub(super) fn native_panel_event(game: &mut Game<'_>, mod_id: &str, event: &NativePanelEvent) {
    let _ = (&game, &mod_id, &event);
    #[cfg(target_os = "android")]
    {
        recipes_event::run(&mut *game, mod_id, event);
    }
}

pub(super) fn native_command(game: &mut Game<'_>, command: &str, ran: &mut bool) {
    let _ = (&game, &command, &ran);
    #[cfg(target_os = "android")]
    {
        smartf3_command::run(&mut *game, command, &mut *ran);
        jade_command(&mut *game, command, &mut *ran);
    }
}

#[cfg(target_os = "android")]
pub(super) fn native_client_resources_reloaded(game: &mut Game<'_>) {
    jade_resources_reloaded(game);
    super::punchy::resources_reloaded();
}

#[cfg(target_os = "android")]
pub(super) fn native_screen_render_post(
    game: &Game<'_>,
    batch: &mut horizon_client::hud::HudBatch,
    screen: &NativeScreenView<'_>,
) {
    jade_screen_render_post(game, batch, screen);
}

#[cfg(target_os = "android")]
pub(super) fn native_screen_mouse_button_pressed_pre(
    game: &Game<'_>, screen: &NativeScreenView<'_>, button: i32,
    modifiers: u32, canceled: &mut bool,
) {
    jade_screen_pressed(game, screen, button, modifiers, canceled);
}

#[cfg(target_os = "android")]
pub(super) fn native_screen_mouse_button_released_pre(
    game: &Game<'_>, screen: &NativeScreenView<'_>, canceled: &mut bool,
) {
    jade_screen_released(game, screen, canceled);
}

#[cfg(target_os = "android")]
pub(super) fn native_screen_mouse_scrolled_pre(
    game: &Game<'_>, screen: &NativeScreenView<'_>, scroll: [f64; 2],
    modifiers: u32, canceled: &mut bool,
) {
    jade_screen_scrolled(game, screen, scroll, modifiers, canceled);
}

#[cfg(target_os = "android")]
pub(super) fn native_screen_key_pressed_pre(
    game: &Game<'_>, screen: &NativeScreenView<'_>, key: &str, repeat: bool,
    modifiers: u32, canceled: &mut bool,
) {
    jade_screen_key(game, screen, key, repeat, modifiers, canceled);
}

#[cfg(target_os = "android")]
pub(super) fn native_screen_char_typed_pre(
    game: &Game<'_>, screen: &NativeScreenView<'_>, c: char, canceled: &mut bool,
) {
    jade_screen_typed(game, screen, c, canceled);
}

#[cfg(target_os = "android")]
pub(super) fn native_register_commands(commands: &mut Vec<super::native_server::NativeCommand>) {
    jade_register_commands(commands);
}

#[cfg(target_os = "android")]
pub(super) fn native_perform_command(
    game: &mut Game<'_>, player: &super::native_server::NativeServerPlayer,
    command: &str, ran: &mut bool,
) {
    jade_perform_command(game, player, command, ran);
}

#[cfg(target_os = "android")]
pub(super) fn native_perform_console_command(game: &mut Game<'_>, command: &str, ran: &mut bool) {
    jade_perform_console(game, command, ran);
}

#[cfg(target_os = "android")]
pub(super) fn native_net_from_client(game: &mut Game<'_>, player: u128, channel: &str, data: &[u8]) {
    jade_net_from_client(game, player, channel, data);
}

#[cfg(target_os = "android")]
pub(super) fn native_net_from_server(channel: &str, data: &[u8]) {
    jade_net_from_server(channel, data);
}

#[cfg(target_os = "android")]
pub(super) fn native_tooltip(
    game: &Game<'_>, stack: &horizon_client::items::ItemStack,
    lines: Vec<(horizon_client::text::Text, u32)>,
) -> Vec<(horizon_client::text::Text, u32)> {
    super::jade::item_tooltip(game, stack, lines)
}

#[cfg(target_os = "android")]
pub(super) fn reset() {
    official_zoom::reset();
    official_minimap::reset();
    accurate_block_placement::forget();
    horizon_client_render::native_connected_textures::reset();
    official_recipes::set_viewer(None);
    smartf3_native::reset();
    let _ = super::freecam::disable();
    horizon_client::motion::freecam_gate::set_freeze(false);
}
