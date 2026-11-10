// A settings button: the setting moves to its next value and is saved.
fn horizon_physics_settings_click(mod_id: &str, button: usize) {
    use horizon_client::native_horizon_physics as hp;
    if mod_id != hp::ID {
        return;
    }
    if !hp::loaded() {
        hp::load(crate::game::Game::native_config_dir().ok());
    }
    hp::setting_click(button);
}
