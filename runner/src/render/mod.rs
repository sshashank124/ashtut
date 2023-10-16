pub mod pass;
mod sync_state;

use shared::UniformObjects;

use crate::gpu::{
    context::Context, framebuffers::Framebuffers, image::Image, pipeline::Pipeline,
    swapchain::Swapchain, sync_info::SyncInfo, Destroy,
};

use self::{
    pass::{
        offscreen::Offscreen, tonemap::Tonemap, OffscreenPipeline, PathTracer, TonemapPipeline,
    },
    sync_state::SyncState,
};

pub mod conf {
    pub const OUTPUT_IMAGE_FORMAT: ash::vk::Format = crate::gpu::image::format::HDR;
}

pub struct Renderer {
    // offscreen pass
    offscreen_pass: OffscreenPipeline,
    intermediate_target: Framebuffers<{ conf::OUTPUT_IMAGE_FORMAT }>,

    // tonemap pass
    tonemap_pass: TonemapPipeline,
    swapchain: Swapchain,

    // state
    state: SyncState,
}

pub enum Error {
    NeedsRecreating,
}

impl Renderer {
    pub fn create(ctx: &mut Context) -> Self {
        let mut raytrace_pass = {
            let contents = PathTracer::create(ctx);
            Pipeline::create(ctx, contents)
        };
        unsafe { raytrace_pass.destroy_with(ctx) };

        let offscreen_pass = {
            let contents = Offscreen::create(ctx);
            Pipeline::create(ctx, contents)
        };

        let intermediate_target = Framebuffers::create_new(
            ctx,
            "Intermediate render target",
            offscreen_pass.spec.render_pass,
            pass::offscreen::conf::FRAME_RESOLUTION,
        );

        let tonemap_pass = {
            let input_image = Image::new(
                ctx,
                intermediate_target.colors[0].image,
                conf::OUTPUT_IMAGE_FORMAT,
                None,
            );
            let contents = Tonemap::create_for(ctx, input_image);
            Pipeline::create(ctx, contents)
        };

        let swapchain = Swapchain::create(ctx, tonemap_pass.spec.render_pass);

        let state = SyncState::create(ctx);

        Self {
            offscreen_pass,
            intermediate_target,

            tonemap_pass,
            swapchain,

            state,
        }
    }

    pub fn render(&mut self, ctx: &Context, uniforms: &UniformObjects) -> Result<(), Error> {
        unsafe {
            ctx.wait_for_fences(self.state.in_flight_fence(), true, u64::MAX)
                .expect("Failed to wait for fence");
        }

        self.offscreen_pass.contents.uniforms.update(uniforms);

        self.offscreen_pass
            .run(ctx, 0, &self.intermediate_target, &SyncInfo::default());

        let (image_index, needs_recreating) = self
            .swapchain
            .get_next_image(self.state.image_available_semaphore()[0]);
        let image_index = image_index as usize;

        let needs_recreating = needs_recreating || {
            unsafe {
                ctx.reset_fences(self.state.in_flight_fence())
                    .expect("Failed to reset fence");
            }

            self.tonemap_pass.run(
                ctx,
                image_index,
                &self.swapchain.render_target,
                &SyncInfo {
                    wait_on: self.state.image_available_semaphore(),
                    signal_to: self.state.render_finished_semaphore(),
                    fence: Some(self.state.in_flight_fence()[0]),
                },
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
        self.swapchain = Swapchain::create(ctx, self.tonemap_pass.spec.render_pass);
    }
}

impl Destroy<Context> for Renderer {
    unsafe fn destroy_with(&mut self, ctx: &mut Context) {
        self.state.destroy_with(ctx);

        self.swapchain.destroy_with(ctx);
        self.tonemap_pass.destroy_with(ctx);

        self.intermediate_target.destroy_with(ctx);
        self.offscreen_pass.destroy_with(ctx);
    }
}
