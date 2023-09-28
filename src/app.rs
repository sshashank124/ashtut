use std::ffi::CString;

use ash::vk;

use winit::{
    dpi::LogicalSize,
    event::{ElementState, Event, KeyboardInput, WindowEvent, VirtualKeyCode},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

use crate::util::{info, platform, validation};

pub struct App {
    _entry: ash::Entry,
    instance: ash::Instance,
    
    debug_utils_loader: ash::extensions::ext::DebugUtils,
    debug_messenger: vk::DebugUtilsMessengerEXT,
}

impl App {
    pub fn new() -> Self {
        let _entry = ash::Entry::linked();
        let instance = Self::create_vulkan_instance(&_entry);
        let (debug_utils_loader, debug_messenger) = validation::setup_debug_utils(&_entry, &instance);

        Self {
            _entry, instance,
            debug_utils_loader, debug_messenger,
        }
    }

    pub fn init_window(event_loop: &EventLoop<()>) -> Window {
        WindowBuilder::new()
            .with_title(info::WINDOW_TITLE)
            .with_inner_size(LogicalSize::<u32>::from(info::WINDOW_SIZE))
            .build(event_loop)
            .expect("Failed to create a window")
    }

    fn create_vulkan_instance(entry: &ash::Entry) -> ash::Instance {
        validation::check_validation_layer_support(entry).unwrap();

        let app_name = CString::new(info::WINDOW_TITLE).unwrap();
        let app_info = vk::ApplicationInfo::builder()
            .application_name(&app_name)
            .api_version(info::VK_API_VERSION);
        
        let instance_create_info = vk::InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_extension_names(platform::REQUIRED_EXTENSION_NAMES);
        
        let mut debug_messenger_create_info = validation::debug_messenger_create_info();
        let instance_create_info = if validation::VALIDATE_LAYERS {
            instance_create_info
                .enabled_layer_names(validation::VALIDATION_LAYERS)
                .push_next(&mut debug_messenger_create_info)
        } else {
            instance_create_info
        };

        unsafe {
            entry.create_instance(&instance_create_info, None)
                .expect("Failed to create Vulkan instance")
        }
    }

    fn render(&mut self) {
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
            if validation::VALIDATE_LAYERS {
                self.debug_utils_loader.destroy_debug_utils_messenger(self.debug_messenger, None);
            }
            self.instance.destroy_instance(None);
        }
    }
}