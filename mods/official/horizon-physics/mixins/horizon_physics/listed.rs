// Horizon Physics' row in Options > Mods; its settings are on the row's page.
fn horizon_physics_listed(mods: &mut Vec<crate::game::native_hooks::NativeMod>) {
    use horizon_client::native_horizon_physics as hp;
    mods.push(crate::game::native_hooks::NativeMod {
        id: hp::ID.into(),
        title: format!("{} {}", hp::NAME, hp::VERSION),
        description: hp::DESCRIPTION.into(),
        keys: Vec::new(),
    });
}
