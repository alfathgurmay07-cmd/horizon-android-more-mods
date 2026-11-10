// The terrain shader of a Horizon shader pack, with water that waves and plants that sway. The
// game's own shader text is built first, then the waves and sway are put into it. If the result
// does not parse, the pack keeps the game's own shader: this must never cost the pack.
fn horizon_physics_shader(
    stage: &horizon_shader_api::check::CheckedStage,
    tags: &[(u32, u32)],
    materials: &[(u32, u32)],
    vertex: bool,
    displaced: bool,
    api: horizon_shader_api::Api,
) -> ::core::ops::ControlFlow<Result<naga::Module, String>> {
    use crate::native_horizon_physics_sway as sway;
    let source = stage.source.as_str();
    let text = shader_source(source, tags, materials, (vertex, displaced), api);
    // The text is the pack's source, then the game's terrain: only the game's part is patched.
    // The game's terrain in it, ready for DirectX 12's FXC.
    let text = sway::fxc_terrain(&text).unwrap_or(text);
    let patched = text
        .strip_prefix(source)
        .and_then(|rest| sway::patch(rest, source))
        .map(|patched| format!("{source}{patched}"));
    if let Some(patched) = patched {
        if let Ok(module) = naga::front::wgsl::parse_str(&patched) {
            return ::core::ops::ControlFlow::Break(Ok(module));
        }
    }
    ::core::ops::ControlFlow::Break(
        naga::front::wgsl::parse_str(&text)
            .map_err(|e| format!("{}: {}", stage.file, e.emit_to_string(&text))),
    )
}
