use std::time::Instant;

use shared::{glam, UniformObjects};
use winit::{
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

use crate::{
    gpu::{context::Context, Destroy},
    render,
};

mod conf {
    pub const WINDOW_TITLE: &str = "Learning Vulkan & Ash";
}

pub struct App {
    ctx: Context,
    renderer: render::Renderer,

    // state
    uniforms: UniformObjects,
    needs_resizing: bool,
    start_time: Instant,
}

impl App {
    pub fn new(window: &Window) -> Self {
        let mut ctx = Context::init(window);
        let renderer = render::Renderer::create(&mut ctx);

        let uniforms = UniformObjects {
            view: shared::Transform::new(glam::Mat4::look_at_rh(
                glam::vec3(2.0, -2.0, 2.0),
                glam::vec3(0.0, 0.0, 0.2),
                glam::vec3(0.0, 0.0, 1.0),
            )),
            proj: shared::Transform::proj(glam::Mat4::perspective_rh(
                f32::to_radians(45.0),
                render::conf::ASPECT_RATIO,
                0.1,
                100.0,
            )),
        };

        Self {
            ctx,
            renderer,

            uniforms,
            needs_resizing: false,
            start_time: Instant::now(),
        }
    }

    fn render(&mut self) {
        if self.needs_resizing {
            if self.recreate() {
                self.needs_resizing = false;
            } else {
                return;
            }
        }

        self.renderer.use_pathtracer = self.ctx.surface.config.extent.width > 1200;

        let millis_for_1_rotation = 3000;
        let frac_millis = (self.start_time.elapsed().as_millis() % millis_for_1_rotation) as f32
            / millis_for_1_rotation as f32;
        let rotation_angle = frac_millis * 2.0 * std::f32::consts::PI;
        self.uniforms.view = shared::Transform::new(
            glam::Mat4::from_rotation_z(rotation_angle)
                * glam::Mat4::look_at_rh(
                    glam::vec3(2.0, -2.0, 2.0),
                    glam::vec3(0.0, 0.0, 0.2),
                    glam::vec3(0.0, 0.0, 1.0),
                ),
        );

        if matches!(
            self.renderer.render(&self.ctx, &self.uniforms),
            Err(render::Error::NeedsRecreating)
        ) {
            self.needs_resizing = true;
        }
    }

    fn recreate(&mut self) -> bool {
        unsafe { self.ctx.wait_idle() };
        let is_valid = self.ctx.refresh_surface_capabilities();
        if is_valid {
            self.renderer.recreate(&mut self.ctx);
        }
        is_valid
    }

    pub fn init_window(event_loop: &EventLoop<()>) -> Window {
        WindowBuilder::new()
            .with_title(conf::WINDOW_TITLE)
            .build(event_loop)
            .expect("Failed to create a window")
    }

    pub fn run(mut self, event_loop: EventLoop<()>, window: Window) {
        event_loop.run(move |event, _, control_flow| match event {
            Event::RedrawRequested(_) => self.render(),
            Event::MainEventsCleared => window.request_redraw(),
            Event::WindowEvent { ref event, .. } => match event {
                WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                    self.needs_resizing = true;
                }
                WindowEvent::CloseRequested
                | WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            state: ElementState::Pressed,
                            virtual_keycode: Some(VirtualKeyCode::Escape),
                            ..
                        },
                    ..
                } => control_flow.set_exit(),
                _ => {}
            },
            _ => {}
        });
    }
}

impl Drop for App {
    fn drop(&mut self) {
        unsafe {
            self.ctx.wait_idle();
            self.renderer.destroy_with(&mut self.ctx);
            self.ctx.destroy_with(&mut ());
        }
    }
}
