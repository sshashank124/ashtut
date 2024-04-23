use ash::vk;

use crate::{context::Context, Destroy};

pub mod conf {
    pub const MAX_FRAMES_IN_FLIGHT: usize = 2;
}

pub struct SyncState {
    frame_available: [vk::Semaphore; conf::MAX_FRAMES_IN_FLIGHT],
    frame_ready: [vk::Semaphore; conf::MAX_FRAMES_IN_FLIGHT],
    in_flight: [vk::Fence; conf::MAX_FRAMES_IN_FLIGHT],
    pub current_frame: usize,
}

impl SyncState {
    pub fn create(ctx: &Context) -> Self {
        firestorm::profile_method!(create);

        let frame_available =
            core::array::from_fn(|i| ctx.create_semaphore(&format!("image_available#{i}")));

        let frame_ready =
            core::array::from_fn(|i| ctx.create_semaphore(&format!("render_finished#{i}")));

        let in_flight = core::array::from_fn(|i| ctx.create_fence(&format!("in_flight#{i}"), true));

        Self {
            frame_available,
            frame_ready,
            in_flight,
            current_frame: 0,
        }
    }

    pub const fn frame_available_semaphore(&self) -> vk::Semaphore {
        self.frame_available[self.current_frame]
    }

    pub const fn frame_ready_semaphore(&self) -> vk::Semaphore {
        self.frame_ready[self.current_frame]
    }

    pub const fn in_flight_fence(&self) -> vk::Fence {
        self.in_flight[self.current_frame]
    }

    pub fn advance(&mut self) {
        self.current_frame = (self.current_frame + 1) % conf::MAX_FRAMES_IN_FLIGHT;
    }
}

impl Destroy<Context> for SyncState {
    unsafe fn destroy_with(&mut self, ctx: &Context) {
        firestorm::profile_method!(destroy_with);

        for i in 0..conf::MAX_FRAMES_IN_FLIGHT {
            ctx.destroy_semaphore(self.frame_available[i], None);
            ctx.destroy_semaphore(self.frame_ready[i], None);
            ctx.destroy_fence(self.in_flight[i], None);
        }
    }
}
