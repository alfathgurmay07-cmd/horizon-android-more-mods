// The settings page: one button per setting, each showing its value.
fn horizon_physics_settings(mod_id: &str, buttons: &mut Vec<String>) {
    use horizon_client::native_horizon_physics as hp;
    if mod_id != hp::ID {
        return;
    }
    if !hp::loaded() {
        hp::load(crate::game::Game::native_config_dir().ok());
    }
    buttons.extend(hp::setting_labels());
}
