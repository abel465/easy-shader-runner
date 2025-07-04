use crate::{
    context::GraphicsContext,
    controller::ControllerTrait,
    render_pass::RenderPass,
    ui::{Ui, UiState},
    user_event::CustomEvent,
};
#[cfg(not(target_arch = "wasm32"))]
use egui_winit::winit::platform::wayland::*;
use egui_winit::winit::{
    application::ApplicationHandler,
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoopProxy},
    keyboard::{Key, NamedKey},
    window::{Window, WindowId},
};
use std::borrow::Cow;
use std::sync::Arc;

pub struct Graphics<C: ControllerTrait> {
    rpass: RenderPass,
    ctx: GraphicsContext,
    controller: C,
    ui: Ui,
    ui_state: UiState,
    window: Arc<Window>,
}

pub struct Builder<C: ControllerTrait> {
    event_proxy: EventLoopProxy<CustomEvent<C>>,
    shader_bytes: Cow<'static, [u8]>,
    controller: C,
}

pub enum App<C: ControllerTrait> {
    Builder(Builder<C>),
    Building(#[cfg(target_arch = "wasm32")] Option<PhysicalSize<u32>>),
    Graphics(Graphics<C>),
}

impl<'a, C: ControllerTrait> App<C> {
    pub fn new(
        event_proxy: EventLoopProxy<CustomEvent<C>>,
        shader_bytes: Cow<'static, [u8]>,
        controller: C,
    ) -> Self {
        Self::Builder(Builder {
            event_proxy,
            shader_bytes,
            controller,
        })
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        let Self::Graphics(gfx) = self else {
            #[cfg(target_arch = "wasm32")]
            if let Self::Building(_) = self {
                *self = Self::Building(Some(size));
            }
            return;
        };
        if size.width != 0 && size.height != 0 {
            gfx.ctx.config.width = size.width;
            gfx.ctx.config.height = size.height;
            gfx.ctx.surface.configure(&gfx.ctx.device, &gfx.ctx.config);
        }
    }

    pub fn keyboard_input(&mut self, event: KeyEvent) {
        let Self::Graphics(gfx) = self else {
            return;
        };
        gfx.controller.keyboard_input(event);
    }

    pub fn mouse_input(&mut self, state: ElementState, button: MouseButton) {
        let Self::Graphics(gfx) = self else {
            return;
        };
        gfx.controller.mouse_input(state, button);
    }

    pub fn mouse_move(&mut self, position: PhysicalPosition<f64>) {
        let Self::Graphics(gfx) = self else {
            return;
        };
        let position = glam::dvec2(position.x, position.y) - gfx.rpass.shader_offset().as_dvec2();
        gfx.controller.mouse_move(position);
    }

    pub fn mouse_scroll(&mut self, delta: MouseScrollDelta) {
        let Self::Graphics(gfx) = self else {
            return;
        };
        let delta = match delta {
            MouseScrollDelta::LineDelta(x, y) => glam::dvec2(x as f64, y as f64),
            MouseScrollDelta::PixelDelta(p) => glam::dvec2(p.x, p.y) * 0.02,
        };
        #[cfg(target_arch = "wasm32")]
        let delta = delta * 0.2777778;
        gfx.controller.mouse_scroll(delta);
    }

    #[cfg(feature = "compute")]
    pub fn update(&mut self) {
        let Self::Graphics(gfx) = self else {
            return;
        };
        let frame_time = gfx
            .window
            .current_monitor()
            .and_then(|m| m.refresh_rate_millihertz().map(|x| x as f32 / 1000.0))
            .unwrap_or(60.0)
            .recip();
        gfx.controller.update(
            |dimensions, threads, push_constants| {
                gfx.rpass
                    .compute(&gfx.ctx, dimensions, threads, push_constants);
            },
            frame_time,
        );
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let Self::Graphics(gfx) = self else {
            return Ok(());
        };
        gfx.window.request_redraw();
        let result = gfx.rpass.render(
            &gfx.ctx,
            &gfx.window,
            &mut gfx.ui,
            &mut gfx.ui_state,
            &mut gfx.controller,
        );
        #[cfg(not(target_arch = "wasm32"))]
        gfx.ctx.set_vsync(gfx.ui_state.vsync);
        result
    }

    pub fn ui_consumes_event(&mut self, event: &WindowEvent) -> bool {
        let Self::Graphics(gfx) = self else {
            return false;
        };
        gfx.ui.consumes_event(&gfx.window, event)
    }

    #[cfg(all(feature = "hot-reload-shader", not(target_arch = "wasm32")))]
    pub fn new_module(&mut self, shader_path: &std::path::Path) {
        let Self::Graphics(gfx) = self else {
            return;
        };
        gfx.rpass.new_module(&gfx.ctx, shader_path);
        gfx.window.request_redraw();
    }
}

impl<C: ControllerTrait> ApplicationHandler<CustomEvent<C>> for App<C> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let Self::Builder(builder) = std::mem::replace(
            self,
            Self::Building(
                #[cfg(target_arch = "wasm32")]
                None,
            ),
        ) {
            let window_attributes = Window::default_attributes().with_title(crate::TITLE);
            let window_attributes = {
                cfg_if::cfg_if! {
                    if #[cfg(target_arch = "wasm32")] {
                        use egui_winit::winit::platform::web::WindowAttributesExtWebSys;
                        window_attributes.with_append(true)
                    } else {
                        window_attributes.with_name(crate::TITLE, "")
                    }
                }
            };
            let window = event_loop.create_window(window_attributes).unwrap();

            cfg_if::cfg_if! {
                if #[cfg(target_arch = "wasm32")] {
                    let size = web_sys::window()
                        .map(|win| {
                            win.document()
                                .and_then(|doc| doc.body().and_then(|body| {
                                    doc.get_element_by_id("loader").and_then(|loader| {
                                        body.remove_child(&loader.into()).ok()
                                    })
                                }));
                            let width = win.inner_width().unwrap().unchecked_into_f64() as u32;
                            let height = win.inner_height().unwrap().unchecked_into_f64() as u32;
                            PhysicalSize { width, height }
                        })
                        .expect("couldn't get window size");
                    wasm_bindgen_futures::spawn_local(create_graphics(builder, size, window));
                } else {
                    futures::executor::block_on(create_graphics(builder, window.inner_size(), window));
                }
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        if self.ui_consumes_event(&event) {
            return;
        }
        match event {
            WindowEvent::RedrawRequested => {
                if let Err(wgpu::SurfaceError::OutOfMemory) = self.render() {
                    event_loop.exit()
                }
                #[cfg(feature = "compute")]
                self.update();
            }
            WindowEvent::CloseRequested
            | WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state: ElementState::Pressed,
                        logical_key: Key::Named(NamedKey::Escape),
                        ..
                    },
                ..
            } => event_loop.exit(),
            WindowEvent::KeyboardInput { event, .. } => self.keyboard_input(event),
            WindowEvent::Resized(size) => self.resize(size),
            WindowEvent::MouseInput { state, button, .. } => self.mouse_input(state, button),
            WindowEvent::MouseWheel { delta, .. } => self.mouse_scroll(delta),
            WindowEvent::CursorMoved { position, .. } => self.mouse_move(position),
            _ => {}
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: CustomEvent<C>) {
        match event {
            CustomEvent::CreateWindow(gfx) => {
                gfx.window.request_redraw();
                cfg_if::cfg_if! {
                    if #[cfg(target_arch = "wasm32")] {
                        if let Self::Building(Some(size)) = std::mem::replace(self, Self::Graphics(gfx)) {
                            self.resize(size);
                        };
                    } else {
                        *self = Self::Graphics(gfx);
                    }
                };
            }
            #[cfg(all(feature = "hot-reload-shader", not(target_arch = "wasm32")))]
            CustomEvent::NewModule(shader_path) => self.new_module(&shader_path),
        }
    }
}

async fn create_graphics<C: ControllerTrait>(
    builder: Builder<C>,
    initial_size: PhysicalSize<u32>,
    window: Window,
) {
    let window = Arc::new(window);
    let ctx = GraphicsContext::new(window.clone(), initial_size).await;

    let ui = Ui::new(window.clone());

    let ui_state = UiState::new();

    let mut controller = builder.controller;
    let rpass = RenderPass::new(&ctx, &builder.shader_bytes, &mut controller);

    let gfx = Graphics {
        rpass,
        ctx,
        controller,
        ui,
        ui_state,
        window,
    };

    builder
        .event_proxy
        .send_event(CustomEvent::CreateWindow(gfx))
        .ok();
}
