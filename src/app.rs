use std::time::Instant;

use ash::vk;

use winit::{
    dpi::LogicalSize,
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

use crate::{
    command_pool::CommandPool,
    device::Device,
    graphics_pipeline::GraphicsPipeline,
    instance::Instance,
    physical_device::PhysicalDevice,
    util::{self, info, Destroy},
};

pub struct App {
    instance: Instance,
    physical_device: PhysicalDevice,
    device: Device,
    graphics_pipeline: GraphicsPipeline,
    command_pool: CommandPool,

    image_available: Vec<vk::Semaphore>,
    render_finished: Vec<vk::Semaphore>,
    in_flight: Vec<vk::Fence>,
    current_frame: usize,

    last_time: Instant,
}

const MAX_FRAMES_IN_FLIGHT: usize = 2;

impl App {
    pub fn new(window: &Window) -> Self {
        let instance = Instance::create();
        let physical_device = PhysicalDevice::pick(&instance, window);
        let device = Device::create(&instance, &physical_device);
        let graphics_pipeline = GraphicsPipeline::create(&instance, &physical_device, &device);
        let command_pool = CommandPool::create(&physical_device, &device, &graphics_pipeline);

        let (image_available_semaphores, render_finished_semaphores, in_flight_fences) =
            Self::create_synchronizers(&device);

        Self {
            instance,
            physical_device,
            device,
            graphics_pipeline,
            command_pool,

            image_available: image_available_semaphores,
            render_finished: render_finished_semaphores,
            in_flight: in_flight_fences,
            current_frame: 0,
            last_time: Instant::now(),
        }
    }

    fn create_synchronizers(
        device: &Device,
    ) -> (Vec<vk::Semaphore>, Vec<vk::Semaphore>, Vec<vk::Fence>) {
        let semaphore_create_info = vk::SemaphoreCreateInfo::builder();
        let fence_create_info =
            vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED);

        let mut image_available_semaphores = Vec::with_capacity(MAX_FRAMES_IN_FLIGHT);
        let mut render_finished_semaphores = Vec::with_capacity(MAX_FRAMES_IN_FLIGHT);
        let mut in_flight_fences = Vec::with_capacity(MAX_FRAMES_IN_FLIGHT);

        for _ in 0..MAX_FRAMES_IN_FLIGHT {
            image_available_semaphores.push(unsafe {
                device
                    .create_semaphore(&semaphore_create_info, None)
                    .expect("Failed to create `image_available` semaphore")
            });
            render_finished_semaphores.push(unsafe {
                device
                    .create_semaphore(&semaphore_create_info, None)
                    .expect("Failed to create `render_finished` semaphore")
            });
            in_flight_fences.push(unsafe {
                device
                    .create_fence(&fence_create_info, None)
                    .expect("Failed to create `in_flight` fence")
            });
        }

        (
            image_available_semaphores,
            render_finished_semaphores,
            in_flight_fences,
        )
    }

    fn render(&mut self) {
        let (image_index, _) = unsafe {
            self.device
                .wait_for_fences(
                    &self.in_flight[util::solo_range(self.current_frame)],
                    true,
                    u64::MAX,
                )
                .expect("Failed to wait for `in_flight` fence");

            self.device
                .reset_fences(&self.in_flight[util::solo_range(self.current_frame)])
                .expect("Failed to reset `in_flight` fence");

            self.graphics_pipeline
                .swapchain
                .loader
                .acquire_next_image(
                    self.graphics_pipeline.swapchain.swapchain,
                    u64::MAX,
                    self.image_available[self.current_frame],
                    vk::Fence::null(),
                )
                .expect("Failed to acquite next image")
        };

        let render_finished = &self.render_finished[util::solo_range(self.current_frame)];

        let submit_infos = [vk::SubmitInfo::builder()
            .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
            .wait_semaphores(&self.image_available[util::solo_range(self.current_frame)])
            .command_buffers(&self.command_pool.buffers[util::solo_range(image_index as usize)])
            .signal_semaphores(render_finished)
            .build()];

        unsafe {
            self.device
                .queue_submit(
                    self.device.graphics_queue,
                    &submit_infos,
                    self.in_flight[self.current_frame],
                )
                .expect("Failed to submit through `graphics` queue");
        }

        let swapchains = [self.graphics_pipeline.swapchain.swapchain];
        let image_indices = [image_index];

        let present_info = vk::PresentInfoKHR::builder()
            .wait_semaphores(render_finished)
            .swapchains(&swapchains)
            .image_indices(&image_indices);

        unsafe {
            self.graphics_pipeline
                .swapchain
                .loader
                .queue_present(self.device.present_queue, &present_info)
                .expect("Failed to present through `present` queue");
        }

        self.current_frame = (self.current_frame + 1) % MAX_FRAMES_IN_FLIGHT;
        
        let now = Instant::now();
        let fps = (now - self.last_time).as_secs_f32().recip() as u32;
        print!("FPS: {:?}\r", fps);
        self.last_time = now;
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
        unsafe {
            self.device
                .device_wait_idle()
                .expect("Failed to wait for idle");

            for i in 0..MAX_FRAMES_IN_FLIGHT {
                self.device.destroy_semaphore(self.image_available[i], None);
                self.device.destroy_semaphore(self.render_finished[i], None);
                self.device.destroy_fence(self.in_flight[i], None);
            }
        }

        self.command_pool.destroy_with(&self.device);
        self.graphics_pipeline.destroy_with(&self.device);
        self.device.destroy_with(());
        self.physical_device.destroy_with(());
        self.instance.destroy_with(());
    }
}
