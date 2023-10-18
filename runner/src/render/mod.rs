pub mod pass;
mod sync_state;

use shared::UniformObjects;

use crate::gpu::{
    context::Context, framebuffers::Framebuffers, image::Image, swapchain::Swapchain,
    sync_info::SyncInfo, Destroy,
};

use self::{
    pass::{offscreen, pathtracer, tonemap},
    sync_state::SyncState,
};

pub mod conf {
    pub const ASPECT_RATIO: f32 = 4.0 / 3.0;
}

pub struct Renderer {
    // offscreen pass
    offscreen_pipeline: offscreen::Pipeline,
    intermediate_target: Framebuffers<{ offscreen::conf::IMAGE_FORMAT }>,

    // tonemap pass
    tonemap_pipeline: tonemap::Pipeline,
    swapchain: Swapchain,

    // state
    state: SyncState,
}

pub enum Error {
    NeedsRecreating,
}

impl Renderer {
    pub fn create(ctx: &mut Context) -> Self {
        let mut pathtracer_pipeline = {
            let data = pathtracer::Data::create(ctx);
            pathtracer::Pipeline::create(ctx, data)
        };
        unsafe { pathtracer_pipeline.destroy_with(ctx) };

        let offscreen_pipeline = {
            let data = offscreen::Data::create(ctx);
            offscreen::Pipeline::create(ctx, data)
        };

        let intermediate_target = Framebuffers::create_new(
            ctx,
            "Intermediate render target",
            offscreen_pipeline.render_pass,
            offscreen::conf::FRAME_RESOLUTION,
        );

        let tonemap_pipeline = {
            let input_image = Image::new(
                ctx,
                intermediate_target.colors[0].image,
                offscreen::conf::IMAGE_FORMAT,
                None,
            );
            let data = tonemap::Data::create(ctx, input_image);
            tonemap::Pipeline::create(ctx, data)
        };

        let swapchain = Swapchain::create(ctx, tonemap_pipeline.render_pass);

        let state = SyncState::create(ctx);

        Self {
            offscreen_pipeline,
            intermediate_target,

            tonemap_pipeline,
            swapchain,

            state,
        }
    }

    pub fn render(&mut self, ctx: &Context, uniforms: &UniformObjects) -> Result<(), Error> {
        unsafe {
            ctx.wait_for_fences(self.state.in_flight_fence(), true, u64::MAX)
                .expect("Failed to wait for fence");
        }

        self.offscreen_pipeline.uniforms.update(uniforms);

        self.offscreen_pipeline
            .run(ctx, 0, &SyncInfo::default(), &self.intermediate_target);

        let (image_index, needs_recreating) = self
            .swapchain
            .get_next_image(ctx, self.state.image_available_semaphore()[0]);
        let image_index = image_index as usize;

        let needs_recreating = needs_recreating || {
            unsafe {
                ctx.reset_fences(self.state.in_flight_fence())
                    .expect("Failed to reset fence");
            }

            self.tonemap_pipeline.run(
                ctx,
                image_index,
                &SyncInfo {
                    wait_on: self.state.image_available_semaphore(),
                    signal_to: self.state.render_finished_semaphore(),
                    fence: Some(self.state.in_flight_fence()[0]),
                },
                &self.swapchain.render_target,
            );

            self.swapchain
                .present_to_when(ctx, image_index, self.state.render_finished_semaphore())
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

        self.intermediate_target.destroy_with(ctx);
        self.offscreen_pipeline.destroy_with(ctx);
    }
}
