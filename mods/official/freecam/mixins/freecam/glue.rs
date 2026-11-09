use super::perspective::CameraType;
use super::umbx_freecam as freecam;
use super::{Game, Screen};
use std::ops::ControlFlow;
use winit::event::{ElementState, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

const MOVES: [KeyCode; 7] = [KeyCode::KeyW, KeyCode::KeyS, KeyCode::KeyA, KeyCode::KeyD, KeyCode::Space, KeyCode::ShiftLeft, KeyCode::ControlLeft];
const VIEWS: [CameraType; 3] = [CameraType::FirstPerson, CameraType::ThirdPersonBack, CameraType::ThirdPersonFront];

fn settings(game: &Game<'_>) -> freecam::Settings {
    freecam::settings(|key| game.native_setting(freecam::ID, key))
}

fn store(game: &mut Game<'_>, settings: freecam::Settings) {
    freecam::set_settings(settings);
    for (key, value) in settings.stored() {
        game.native_set_setting(freecam::ID, key, &value);
    }
}

pub(super) fn key(game: &mut Game<'_>, key: &str, pressed: bool) {
    if !pressed || !game.native_on(freecam::ID) || !game.native_playing() || key != game.native_mod_key(freecam::ID, freecam::KEY, freecam::DEFAULT_KEY) {
        return;
    }
    if freecam::active() {
        exit(game);
    } else {
        enter(game);
    }
}

fn enter(game: &mut Game<'_>) {
    let Some(player) = game.player.as_ref() else { return };
    let eye = [player.position.x, player.position.y + f64::from(game.eye), player.position.z];
    settings(game);
    let view = VIEWS.iter().position(|v| *v == game.camera_type).unwrap_or(0) as u8;
    freecam::start(eye, game.yaw, game.pitch, view);
    for key in MOVES.iter() {
        game.keys.remove(key);
    }
    game.camera_type = CameraType::ThirdPersonBack;
}

fn exit(game: &mut Game<'_>) {
    let Some(view) = freecam::stop() else { return };
    game.camera_type = VIEWS[usize::from(view).min(VIEWS.len() - 1)];
    for key in MOVES {
        if game.physical_keys.contains(&key) {
            game.keys.insert(key);
        }
    }
}

pub(super) fn window(game: &mut Game<'_>, event: &WindowEvent) -> ControlFlow<()> {
    if !freecam::active() {
        return ControlFlow::Continue(());
    }
    let WindowEvent::KeyboardInput { event, .. } = event else { return ControlFlow::Continue(()) };
    let PhysicalKey::Code(code) = event.physical_key else { return ControlFlow::Continue(()) };
    let Some(input) = MOVES.iter().position(|k| *k == code) else { return ControlFlow::Continue(()) };
    let down = event.state == ElementState::Pressed;
    if down && game.screen != Screen::Playing {
        return ControlFlow::Continue(());
    }
    freecam::hold(input, down);
    if down {
        game.physical_keys.insert(code);
    } else {
        game.physical_keys.remove(&code);
    }
    ControlFlow::Break(())
}

pub(super) fn mapped(
    game: &mut Game<'_>,
    key: KeyCode,
    pressed: bool,
    repeat: bool,
) -> ControlFlow<horizon_client::Result<()>> {
    if repeat || !freecam::active() || !game.native_on(freecam::ID) {
        return ControlFlow::Continue(());
    }
    if pressed && !game.native_playing() {
        return ControlFlow::Continue(());
    }
    let Some(input) = MOVES.iter().position(|k| *k == key) else {
        return ControlFlow::Continue(());
    };
    if pressed && game.screen != Screen::Playing {
        return ControlFlow::Continue(());
    }
    freecam::hold(input, pressed);
    if pressed {
        game.physical_keys.insert(key);
    } else {
        game.physical_keys.remove(&key);
    }
    ControlFlow::Break(Ok(()))
}

pub(super) fn turn(xo: f64, yo: f64) -> ControlFlow<()> {
    if freecam::turn(xo, yo) { ControlFlow::Break(()) } else { ControlFlow::Continue(()) }
}

pub(super) fn camera(partial: f32) -> ControlFlow<Option<(glam::DVec3, f32, f32)>> {
    freecam::view(partial).map_or(ControlFlow::Continue(()), |([x, y, z], yaw, pitch)| ControlFlow::Break(Some((glam::DVec3::new(x, y, z), yaw, pitch))))
}

pub(super) fn tick(game: &mut Game<'_>) {
    if !freecam::active() {
        return;
    }
    if game.player.is_none() || !game.native_on(freecam::ID) {
        exit(game);
        return;
    }
    game.camera_type = CameraType::ThirdPersonBack;
    let noclip = settings(game).noclip;
    let game = &*game;
    freecam::tick(|[x, y, z]| !noclip && game.res.facts().get(game.world.block(x, y, z)).full_collision);
}

pub(super) fn wheel(game: &mut Game<'_>, steps: f32, claimed: &mut bool) {
    if !game.native_on(freecam::ID) {
        return;
    }
    if let Some(settings) = freecam::wheel(steps) {
        store(game, settings);
        *claimed = true;
    }
}

pub(super) fn listed(mods: &mut Vec<super::native_hooks::NativeMod>) {
    mods.push(super::native_hooks::NativeMod {
        id: freecam::ID.into(),
        title: format!("Freecam+ {}", freecam::VERSION),
        description: freecam::DESCRIPTION.into(),
        keys: vec![[freecam::KEY.into(), "Toggle freecam".into(), freecam::DEFAULT_KEY.into()]],
    });
}

pub(super) fn page(game: &Game<'_>, mod_id: &str, buttons: &mut Vec<String>) {
    if mod_id == freecam::ID {
        buttons.extend(settings(game).labels());
    }
}

pub(super) fn click(game: &mut Game<'_>, mod_id: &str, button: usize) {
    if mod_id != freecam::ID {
        return;
    }
    let mut settings = settings(game);
    settings.click(button);
    store(game, settings);
}
