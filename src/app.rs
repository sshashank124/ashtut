use winit::{
    dpi::LogicalSize,
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

use crate::{
    device::Device,
    graphics_pipeline::GraphicsPipeline,
    instance::Instance,
    physical_device::PhysicalDevice,
    surface::Surface,
    swapchain::Swapchain,
    util::{info, Destroy}, render_pass::RenderPass,
};

pub struct App {
    instance: Instance,
    surface: Surface,
    physical_device: PhysicalDevice,
    device: Device,
    swapchain: Swapchain,
    render_pass: RenderPass,
    graphics_pipeline: GraphicsPipeline,
}

impl App {
    pub fn new(window: &Window) -> Self {
        let entry = ash::Entry::linked();
        let instance = Instance::create(&entry);
        let surface = Surface::create(&entry, &instance, window);
        let (physical_device, surface_details) = PhysicalDevice::pick(&instance, &surface);
        let device = Device::create(&instance, &physical_device);
        let swapchain = Swapchain::create(
            &instance,
            &surface,
            surface_details,
            &physical_device,
            &device,
        );
        let render_pass = RenderPass::create(&device, swapchain.format);
        let graphics_pipeline = GraphicsPipeline::create(&device, &render_pass, &swapchain);

        Self {
            instance,
            surface,
            physical_device,
            device,
            swapchain,
            render_pass,
            graphics_pipeline,
        }
    }

    fn render(&mut self) {}

    pub fn init_window(event_loop: &EventLoop<()>) -> Window {
        WindowBuilder::new()
            .with_title(info::WINDOW_TITLE)
            .with_inner_size(LogicalSize::<u32>::from(info::WINDOW_SIZE))
            .build(event_loop)
            .expect("Failed to create a window")
    }

    pub fn run(mut self, event_loop: EventLoop<()>, window: Window) {
        event_loop.run(move |event, _, control_flow| match event {
            Event::RedrawRequested(window_id) if window_id == window.id() => {
                self.render();
            }
            Event::MainEventsCleared => window.request_redraw(),
            Event::WindowEvent {
                window_id,
                ref event,
            } if window_id == window.id() => match event {
                WindowEvent::CloseRequested
                | WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            state: ElementState::Pressed,
                            virtual_keycode: Some(VirtualKeyCode::Escape),
                            ..
                        },
                    ..
                } => *control_flow = ControlFlow::Exit,
                _ => {}
            },
            _ => {}
        });
    }
}

impl Drop for App {
    fn drop(&mut self) {
        self.graphics_pipeline.destroy_with(&self.device);
        self.render_pass.destroy_with(&self.device);
        self.swapchain.destroy_with(&self.device);
        self.device.destroy_with(());
        self.surface.destroy_with(());
        self.instance.destroy_with(());
    }
}
