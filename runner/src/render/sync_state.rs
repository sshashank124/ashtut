use ash::vk;

use crate::{
    context::Context,
    util::{self, Destroy},
};

pub mod conf {
    pub const MAX_FRAMES_IN_FLIGHT: usize = 2;
}

pub struct SyncState {
    image_available: Vec<vk::Semaphore>,
    render_finished: Vec<vk::Semaphore>,
    in_flight: Vec<vk::Fence>,
    pub current_frame: usize,
}

impl SyncState {
    pub fn create(ctx: &Context) -> Self {
        let mut image_available = Vec::with_capacity(conf::MAX_FRAMES_IN_FLIGHT);
        let mut render_finished = Vec::with_capacity(conf::MAX_FRAMES_IN_FLIGHT);
        let mut in_flight = Vec::with_capacity(conf::MAX_FRAMES_IN_FLIGHT);

        for _ in 0..conf::MAX_FRAMES_IN_FLIGHT {
            image_available.push(ctx.device.create_semaphore("image_available"));
            render_finished.push(ctx.device.create_semaphore("render_finished"));
            in_flight.push(ctx.device.create_fence("in_flight", true));
        }

        Self {
            image_available,
            render_finished,
            in_flight,
            current_frame: 0,
        }
    }

    pub fn image_available_semaphore(&self) -> &[vk::Semaphore] {
        &self.image_available[util::solo_range(self.current_frame)]
    }

    pub fn render_finished_semaphore(&self) -> &[vk::Semaphore] {
        &self.render_finished[util::solo_range(self.current_frame)]
    }

    pub fn in_flight_fence(&self) -> &[vk::Fence] {
        &self.in_flight[util::solo_range(self.current_frame)]
    }

    pub fn advance(&mut self) {
        self.current_frame = (self.current_frame + 1) % conf::MAX_FRAMES_IN_FLIGHT;
    }
}

impl<'a> Destroy<&'a Context> for SyncState {
    unsafe fn destroy_with(&mut self, ctx: &'a Context) {
        for i in 0..conf::MAX_FRAMES_IN_FLIGHT {
            ctx.device.destroy_semaphore(self.image_available[i], None);
            ctx.device.destroy_semaphore(self.render_finished[i], None);
            ctx.device.destroy_fence(self.in_flight[i], None);
        }
    }
}
