use std::time::Instant;

use winit::{
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

use crate::{
    device::Device,
    instance::Instance,
    render_pipeline::{RenderError, RenderPipeline},
    surface::Surface,
    util::{info, Destroy},
};

pub struct App {
    instance: Instance,
    surface: Surface,
    device: Device,
    render_pipeline: RenderPipeline,

    resized: bool,
    last_frame: Instant,
}

impl App {
    pub fn new(window: &Window) -> Self {
        let instance = Instance::new();
        let surface_descriptor = instance.create_surface_on(window);
        let (device, mut surface) = instance.request_device_for(surface_descriptor);
        let render_pipeline = RenderPipeline::create(&device, &mut surface, &instance);

        Self {
            instance,
            surface,
            device,
            render_pipeline,

            resized: false,
            last_frame: Instant::now(),
        }
    }

    fn render(&mut self) {
        if self.resized {
            self.recreate();
        }

        if let Err(RenderError::NeedsRecreating) = self.render_pipeline.render(&self.device) {
            self.resized = true;
        }

        let now = Instant::now();
        let fps = (now - self.last_frame).as_secs_f32().recip() as u32;
        print!("FPS: {:6?}\r", fps);
        self.last_frame = now;
    }

    fn recreate(&mut self) {
        let prerecreate = Instant::now();

        unsafe { self.device.wait_until_idle() };

        self.surface
            .refresh_capabilities(self.device.physical_device);

        if !self.surface.config.invalid_extent() {
            self.render_pipeline
                .recreate(&self.device, &mut self.surface, &self.instance);
        }

        self.resized = false;

        println!("Recreating took {:?}", Instant::now() - prerecreate);
    }

    pub fn init_window(event_loop: &EventLoop<()>) -> Window {
        WindowBuilder::new()
            .with_title(info::WINDOW_TITLE)
            // .with_fullscreen(Some(winit::window::Fullscreen::Borderless(None)))
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
            self.device.wait_until_idle();

            self.render_pipeline.destroy_with(&self.device);
            self.device.destroy_with(());
            self.surface.destroy_with(());
            self.instance.destroy_with(());
        }
    }
}
