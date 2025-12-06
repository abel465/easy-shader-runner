use crate::GraphicsContext;
use egui_winit::winit::event::{ElementState, KeyEvent, MouseButton, TouchPhase};
use egui_winit::winit::event_loop::ActiveEventLoop;
use glam::*;

pub trait ControllerTrait: 'static {
    fn resize(&mut self, _size: UVec2);

    fn touch(&mut self, _id: u64, _phase: TouchPhase, _position: DVec2) {}

    fn mouse_move(&mut self, _position: DVec2) {}

    fn mouse_scroll(&mut self, _delta: DVec2) {}

    fn mouse_input(&mut self, _state: ElementState, _button: MouseButton) {}

    fn keyboard_input(&mut self, _key: KeyEvent) {}

    fn prepare_render(
        &mut self,
        gfx_ctx: &GraphicsContext,
        offset: Vec2,
    ) -> impl bytemuck::NoUninit;

    /// Run the compute shader after rendering
    #[cfg(feature = "compute")]
    fn update<
        F: Fn(
            UVec3, // dimensions
            UVec3, // threads (same as declared in compute shader)
            &[u8], // push_constants
        ),
    >(
        &mut self,
        _gfx_ctx: &GraphicsContext,
        _compute: F,
        _allowed_duration: f32,
    ) {
    }

    fn describe_bind_groups(
        &mut self,
        _gfx_ctx: &GraphicsContext,
    ) -> (Vec<wgpu::BindGroupLayout>, Vec<wgpu::BindGroup>) {
        (vec![], vec![])
    }

    fn describe_vertex_buffer_layouts(
        &mut self,
        _gfx_ctx: &GraphicsContext,
    ) -> Vec<wgpu::VertexBufferLayout<'static>> {
        vec![]
    }

    /// Called every frame.
    /// return Vertex buffer and number of vertices as well as Index buffer and number of indices
    #[allow(clippy::type_complexity)]
    fn get_vertex_index_buffer(
        &self,
    ) -> (Option<(&wgpu::Buffer, u32)>, Option<(&wgpu::Buffer, u32)>) {
        (None, None)
    }

    fn describe_wgpu_features_and_limits(
        &self,
        _supported_features: wgpu::Features,
        _supported_limits: wgpu::Limits,
    ) -> (wgpu::Features, wgpu::Limits) {
        (wgpu::Features::default(), wgpu::Limits::default())
    }

    fn ui(
        &mut self,
        _ctx: &egui::Context,
        _ui_state: &mut crate::ui::UiState,
        _gfx_ctx: &GraphicsContext,
    ) {
    }

    #[cfg(all(feature = "hot-reload-shader", not(target_arch = "wasm32")))]
    fn new_shader_module(&mut self) {}

    /// This callback is invoked during initialision, before the wgpu context is created.
    /// For example, it allows you to detect the available screen resolutions before the shader is first launched.
    fn app_resumed(&mut self, _event_loop: &ActiveEventLoop) {}
}
