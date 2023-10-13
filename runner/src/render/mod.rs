pub mod pass;
mod swapchain;
mod sync_state;

use shared::UniformObjects;

use crate::gpu::{
    context::Context, framebuffer::Framebuffers, image::format, scope::Scope, Destroy,
};

use self::{swapchain::Swapchain, sync_state::SyncState};

pub struct Renderer {
    // offscreen pass
    offscreen_pass: pass::Offscreen,
    intermediate_target: Framebuffers<{ format::HDR }>,

    // tonemap pass
    tonemap_pass: pass::Tonemap,
    swapchain: Swapchain,

    // state
    state: SyncState,
}

pub enum Error {
    NeedsRecreating,
}

impl Renderer {
    pub fn create(ctx: &mut Context) -> Self {
        let mut setup_scope = Scope::begin_on(ctx, ctx.queues.graphics());

        let offscreen_pass = pass::Offscreen::create(ctx, &mut setup_scope);
        let intermediate_target = Framebuffers::create_new(
            ctx,
            "Intermediate render target",
            &offscreen_pass.render_pass,
            pass::offscreen::conf::FRAME_RESOLUTION,
        );

        let tonemap_pass = pass::Tonemap::create(ctx, &intermediate_target);
        let swapchain = Swapchain::create(ctx, &tonemap_pass.render_pass);

        setup_scope.finish(ctx);

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

        self.offscreen_pass
            .draw(ctx, &self.intermediate_target, uniforms);

        let (image_index, needs_recreating) = self
            .swapchain
            .acquire_next_image_and_signal(self.state.image_available_semaphore()[0]);
        let image_index = image_index as usize;

        let needs_recreating = needs_recreating || {
            unsafe {
                ctx.reset_fences(self.state.in_flight_fence())
                    .expect("Failed to reset fence");
            }

            self.tonemap_pass.draw(
                ctx,
                image_index,
                self.state.image_available_semaphore(),
                self.state.render_finished_semaphore(),
                self.state.in_flight_fence()[0],
                (&self.intermediate_target, &self.swapchain.render_target),
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
        self.swapchain = Swapchain::create(ctx, &self.tonemap_pass.render_pass);
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
