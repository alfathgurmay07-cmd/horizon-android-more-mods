// Every shader module the renderer makes: Horizon's terrain shader is made ready for DirectX 12's
// FXC compiler first (`fxc_terrain`), so the game starts on DirectX 12. It runs whether the mod's
// effects are on or not: without it the game cannot start there at all. The module made again
// has no such lines, so this hook lets that call through.
fn horizon_physics_fxc(this: &On<'_>, descriptor: &wgpu::ShaderModuleDescriptor<'_>) -> ::core::ops::ControlFlow<wgpu::ShaderModule> {
    use crate::native_horizon_physics_sway as sway;
    let wgpu::ShaderSource::Wgsl(text) = &descriptor.source else {
        return ::core::ops::ControlFlow::Continue(());
    };
    let Some(fixed) = sway::fxc_terrain(text) else {
        return ::core::ops::ControlFlow::Continue(());
    };
    ::core::ops::ControlFlow::Break(this.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: descriptor.label,
        source: wgpu::ShaderSource::Wgsl(fixed.into()),
    }))
}
