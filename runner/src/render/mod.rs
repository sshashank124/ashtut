mod pass;
mod sync_state;

use std::slice;

use crate::{
    data::gltf_scene::GltfScene,
    gpu::{
        context::Context, scope::OneshotScope, swapchain::Swapchain, sync_info::SyncInfo, Destroy,
    },
};

use self::{
    pass::{common, pathtracer, rasterizer, tonemap},
    sync_state::SyncState,
};

pub mod conf {}

pub struct Renderer {
    // render pass
    common: common::Data,
    pathtracer_pipeline: pathtracer::Pipeline,
    rasterizer_pipeline: rasterizer::Pipeline,

    // tonemap pass
    tonemap_pipeline: tonemap::Pipeline,
    swapchain: Swapchain,

    // state
    uniforms: shared::Uniforms,
    use_pathtracer: bool,
    frame: u32,
    state: SyncState,
}

pub enum Error {
    NeedsRecreating,
}

impl Renderer {
    pub fn create(ctx: &mut Context, scene: GltfScene, camera: shared::Camera) -> Self {
        let mut common = common::Data::create(ctx, scene);
        let uniforms = shared::Uniforms { camera };
        common.uniforms.update(&uniforms);

        let pathtracer_pipeline = pathtracer::Pipeline::create(ctx, &common);

        let mut init_scope = OneshotScope::begin_on(ctx, ctx.queues.transfer());

        let rasterizer_pipeline = rasterizer::Pipeline::create(ctx, &mut init_scope, &common);
        let tonemap_pipeline = tonemap::Pipeline::create(ctx, &common);
        let swapchain = Swapchain::create(ctx, &mut init_scope, tonemap_pipeline.render_pass);

        init_scope.finish(ctx);

        let state = SyncState::create(ctx);

        Self {
            common,
            pathtracer_pipeline,
            rasterizer_pipeline,

            tonemap_pipeline,
            swapchain,

            uniforms,
            frame: 0,
            use_pathtracer: true,
            state,
        }
    }

    pub fn render(&mut self, ctx: &Context) -> Result<(), Error> {
        unsafe {
            ctx.wait_for_fences(
                slice::from_ref(&self.state.in_flight_fence()),
                true,
                u64::MAX,
            )
            .expect("Failed to wait for fence");
        }

        let sync_info = SyncInfo::default();
        if self.use_pathtracer {
            self.pathtracer_pipeline
                .run(ctx, &self.common, self.frame, &sync_info);
        } else {
            self.rasterizer_pipeline.run(ctx, &self.common, &sync_info);
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

        self.frame += 1;
        self.state.advance();

        (!needs_recreating)
            .then_some(())
            .ok_or(Error::NeedsRecreating)
    }

    pub fn update_camera(&mut self, camera: shared::Camera) {
        self.uniforms.camera = camera;
        self.common.uniforms.update(&self.uniforms);
        self.frame = 0;
    }

    pub fn toggle_renderer(&mut self) {
        self.use_pathtracer = !self.use_pathtracer;
        self.frame = 0;
    }

    pub fn recreate(&mut self, ctx: &mut Context) {
        unsafe {
            self.swapchain.destroy_with(ctx);
        }
        let mut init_scope = OneshotScope::begin_on(ctx, ctx.queues.transfer());
        self.swapchain = Swapchain::create(ctx, &mut init_scope, self.tonemap_pipeline.render_pass);
        init_scope.finish(ctx);
    }
}

impl Destroy<Context> for Renderer {
    unsafe fn destroy_with(&mut self, ctx: &mut Context) {
        self.state.destroy_with(ctx);

        self.swapchain.destroy_with(ctx);
        self.tonemap_pipeline.destroy_with(ctx);

        self.rasterizer_pipeline.destroy_with(ctx);
        self.pathtracer_pipeline.destroy_with(ctx);
        self.common.destroy_with(ctx);
    }
}
