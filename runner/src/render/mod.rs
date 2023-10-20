pub mod pass;
mod sync_state;

use std::slice;

use ash::vk;
use shared::UniformObjects;

use crate::gpu::{
    context::Context,
    image::{format, Image},
    swapchain::Swapchain,
    sync_info::SyncInfo,
    Destroy,
};

use self::{
    pass::{offscreen, pathtracer, tonemap},
    sync_state::SyncState,
};

pub mod conf {
    pub const ASPECT_RATIO: f32 = 4.0 / 3.0;
    const HEIGHT: u32 = 768;
    pub const FRAME_RESOLUTION: ash::vk::Extent2D = ash::vk::Extent2D {
        height: HEIGHT,
        width: (HEIGHT as f32 * ASPECT_RATIO) as _,
    };
}

pub struct Renderer {
    // offscreen pass
    target: Image<{ format::HDR }>,
    pathtracer_pipeline: pathtracer::Pipeline,
    offscreen_pipeline: offscreen::Pipeline,

    // tonemap pass
    tonemap_pipeline: tonemap::Pipeline,
    swapchain: Swapchain,

    // state
    pub use_pathtracer: bool,
    state: SyncState,
}

pub enum Error {
    NeedsRecreating,
}

impl Renderer {
    pub fn create(ctx: &mut Context) -> Self {
        let target = {
            let info = vk::ImageCreateInfo {
                extent: conf::FRAME_RESOLUTION.into(),
                usage: vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::STORAGE,
                ..Default::default()
            };
            Image::create(ctx, "Intermediate Target", &info)
        };

        let pathtracer_pipeline = {
            let data = pathtracer::Data::create(ctx, &target);
            pathtracer::Pipeline::create(ctx, data)
        };

        let offscreen_pipeline = {
            let data = offscreen::Data::create(ctx);
            offscreen::Pipeline::create(ctx, data, &target)
        };

        let tonemap_pipeline = {
            let input_image = Image::new(ctx, target.image, format::HDR, None);
            let data = tonemap::Data::create(ctx, input_image);
            tonemap::Pipeline::create(ctx, data)
        };

        let swapchain = Swapchain::create(ctx, tonemap_pipeline.render_pass);

        let state = SyncState::create(ctx);

        Self {
            target,
            pathtracer_pipeline,
            offscreen_pipeline,

            tonemap_pipeline,
            swapchain,

            use_pathtracer: false,
            state,
        }
    }

    pub fn render(&mut self, ctx: &Context, uniforms: &UniformObjects) -> Result<(), Error> {
        unsafe {
            ctx.wait_for_fences(
                slice::from_ref(&self.state.in_flight_fence()),
                true,
                u64::MAX,
            )
            .expect("Failed to wait for fence");
        }

        if self.use_pathtracer {
            self.pathtracer_pipeline.run(ctx, &SyncInfo::default());
        } else {
            self.offscreen_pipeline.uniforms.update(uniforms);
            self.offscreen_pipeline.run(ctx, &SyncInfo::default());
        }

        let (image_index, needs_recreating) = self
            .swapchain
            .get_next_image(ctx, self.state.image_available_semaphore());
        let image_index = image_index as usize;

        let needs_recreating = needs_recreating || {
            unsafe {
                ctx.reset_fences(slice::from_ref(&self.state.in_flight_fence()))
                    .expect("Failed to reset fence");
            }

            self.tonemap_pipeline.run(
                ctx,
                image_index,
                &SyncInfo {
                    wait_on: Some(self.state.image_available_semaphore()),
                    signal_to: Some(self.state.render_finished_semaphore()),
                    fence: Some(self.state.in_flight_fence()),
                },
                &self.swapchain.target,
            );

            self.swapchain.present_to_when(
                ctx,
                image_index,
                slice::from_ref(&self.state.render_finished_semaphore()),
            )
        };

        self.state.advance();

        (!needs_recreating)
            .then_some(())
            .ok_or(Error::NeedsRecreating)
    }

    pub fn recreate(&mut self, ctx: &mut Context) {
        unsafe {
            self.swapchain.destroy_with(ctx);
        }
        self.swapchain = Swapchain::create(ctx, self.tonemap_pipeline.render_pass);
    }
}

impl Destroy<Context> for Renderer {
    unsafe fn destroy_with(&mut self, ctx: &mut Context) {
        self.state.destroy_with(ctx);

        self.swapchain.destroy_with(ctx);
        self.tonemap_pipeline.destroy_with(ctx);

        self.offscreen_pipeline.destroy_with(ctx);
        self.pathtracer_pipeline.destroy_with(ctx);
        self.target.destroy_with(ctx);
    }
}
