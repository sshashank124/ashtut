use std::mem::ManuallyDrop;

use winit::{
    dpi::LogicalSize,
    event::{ElementState, Event, KeyboardInput, WindowEvent, VirtualKeyCode},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

use crate::{
    device::Device,
    instance::Instance,
    physical_device::PhysicalDevice,
    util::info,
};

pub struct App {
    instance: ManuallyDrop<Instance>,
    _physical_device: PhysicalDevice,
    device: ManuallyDrop<Device>,
}

impl App {
    pub fn new() -> Self {
        let entry = ash::Entry::linked();
        let instance = ManuallyDrop::new(Instance::new(&entry));
        let _physical_device = PhysicalDevice::pick(&instance);
        let device = ManuallyDrop::new(Device::create(&instance, &_physical_device));

        Self {
            instance,
            _physical_device,
            device,
        }
    }

    fn render(&mut self) {
    }

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
            },
            Event::MainEventsCleared => window.request_redraw(),
            Event::WindowEvent {
                window_id,
                ref event,
            } if window_id == window.id() => {
                match event {
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
                }
            },
            _ => {}
        });
    }
}

impl Drop for App {
    fn drop(&mut self) {
        unsafe {
            ManuallyDrop::drop(&mut self.device);
            ManuallyDrop::drop(&mut self.instance);
        }
    }
}