use std::time::Instant;

use winit::{
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

use crate::{context::Context, render, util::Destroy};

mod conf {
    pub const WINDOW_TITLE: &str = "Learning Vulkan & Ash";
}

pub struct App {
    ctx: Context,
    render_pipeline: render::Pipeline,

    resized: bool,
    last_frame: Instant,
}

impl App {
    pub fn new(window: &Window) -> Self {
        let mut ctx = Context::init(window);
        let render_pipeline = render::Pipeline::create(&mut ctx);

        Self {
            ctx,
            render_pipeline,

            resized: false,
            last_frame: Instant::now(),
        }
    }

    fn render(&mut self) {
        if self.resized {
            self.recreate();
        }

        if matches!(
            self.render_pipeline.render(&self.ctx),
            Err(render::Error::NeedsRecreating)
        ) {
            self.resized = true;
        }

        let now = Instant::now();
        let fps = (now - self.last_frame).as_secs_f32().recip() as u32;
        print!("FPS: {fps:6?}\r");
        self.last_frame = now;
    }

    fn recreate(&mut self) {
        unsafe { self.ctx.device_wait_idle() };
        if self.ctx.refresh_surface_capabilities() {
            self.render_pipeline.recreate(&mut self.ctx);
        }
        self.resized = false;
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
                    self.resized = true;
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
            self.ctx.device_wait_idle();
            self.render_pipeline.destroy_with(&mut self.ctx);
            self.ctx.destroy_with(());
        }
    }
}
