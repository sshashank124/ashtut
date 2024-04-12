#![feature(adt_const_params)]

mod acceleration_structure;
mod buffer;
mod commands;
mod context;
mod descriptors;
mod framebuffers;
mod image;
mod pass;
mod pipeline;
mod query_pool;
mod sampler;
mod scope;
mod shader_binding_table;
mod swapchain;
mod sync_info;
mod sync_state;
mod texture;
mod uniforms;
mod util;
mod world;

use std::{ops::DerefMut, slice};

use raw_window_handle::HasWindowHandle;

use shared::inputs;

use {context::Context, scope::OneshotScope, swapchain::Swapchain, sync_info::SyncInfo};

use self::{
    pass::{common, pathtracer, rasterizer, tonemap},
    sync_state::SyncState,
};

pub struct Renderer {
    ctx: Context,

    // render pass
    common: common::Data,
    pathtracer_pipeline: pathtracer::Pipeline,
    rasterizer_pipeline: rasterizer::Pipeline,

    // tonemap pass
    tonemap_pipeline: tonemap::Pipeline,
    swapchain: Swapchain,

    // state
    uniforms: inputs::Uniforms,
    use_pathtracer: bool,
    frame: u32,
    state: SyncState,
}

trait Destroy<C> {
    unsafe fn destroy_with(&mut self, ctx: &mut C);
}

pub enum Error {
    NeedsRecreating,
}

impl Renderer {
    pub fn create(
        name: impl AsRef<str>,
        window: &impl HasWindowHandle,
        scene: scene::Scene,
        resolution: (u32, u32),
        camera: inputs::Camera,
    ) -> Self {
        let mut ctx = Context::init(name.as_ref(), window);

        let mut common = common::Data::create(&mut ctx, scene, resolution);
        let uniforms = inputs::Uniforms { camera };
        common.uniforms.update(&uniforms);

        let pathtracer_pipeline = pathtracer::Pipeline::create(&mut ctx, &common);

        let init_scope = OneshotScope::begin_on(&ctx, "Initialization", ctx.queues.transfer());

        let rasterizer_pipeline = rasterizer::Pipeline::create(&mut ctx, &init_scope, &common);
        let tonemap_pipeline = tonemap::Pipeline::create(&ctx, &common);
        let swapchain = Swapchain::create(&mut ctx, &init_scope, tonemap_pipeline.render_pass);

        init_scope.finish(&mut ctx);

        let state = SyncState::create(&ctx);

        Self {
            ctx,

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

    pub fn render(&mut self) -> Result<(), Error> {
        unsafe {
            self.ctx
                .wait_for_fences(
                    slice::from_ref(&self.state.in_flight_fence()),
                    true,
                    u64::MAX,
                )
                .expect("Failed to wait for fence");
        }

        let sync_info = SyncInfo::default();
        if self.use_pathtracer {
            self.pathtracer_pipeline
                .run(&self.ctx, &self.common, self.frame, &sync_info);
        } else {
            self.rasterizer_pipeline
                .run(&self.ctx, &self.common, &sync_info);
        }

        let (image_index, needs_recreating) = self
            .swapchain
            .get_next_image(&self.ctx, self.state.image_available_semaphore());
        let image_index = image_index as usize;

        let needs_recreating = needs_recreating || {
            unsafe {
                self.ctx
                    .reset_fences(slice::from_ref(&self.state.in_flight_fence()))
                    .expect("Failed to reset fence");
            }

            self.tonemap_pipeline.run(
                &self.ctx,
                image_index,
                &SyncInfo {
                    wait_on: Some(self.state.image_available_semaphore()),
                    signal_to: Some(self.state.render_finished_semaphore()),
                    fence: Some(self.state.in_flight_fence()),
                },
                &self.swapchain.target,
            );

            self.swapchain.present_to_when(
                &self.ctx,
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

    pub fn update_camera(&mut self, camera: inputs::Camera) {
        self.uniforms.camera = camera;
        self.common.uniforms.update(&self.uniforms);
        self.frame = 0;
    }

    pub fn toggle_renderer(&mut self) {
        self.use_pathtracer = !self.use_pathtracer;
        self.frame = 0;
    }

    pub fn recreate(&mut self) -> bool {
        unsafe { self.ctx.wait_idle() };

        let is_valid = self.ctx.refresh_surface_capabilities();

        if is_valid {
            unsafe { self.swapchain.destroy_with(&mut self.ctx) };
            let init_scope =
                OneshotScope::begin_on(&self.ctx, "Initialization", self.ctx.queues.transfer());
            self.swapchain = Swapchain::create(
                &mut self.ctx,
                &init_scope,
                self.tonemap_pipeline.render_pass,
            );
            init_scope.finish(&mut self.ctx);
        }

        is_valid
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            self.ctx.wait_idle();

            self.state.destroy_with(&mut self.ctx);

            self.swapchain.destroy_with(&mut self.ctx);
            self.tonemap_pipeline.destroy_with(&mut self.ctx);

            self.rasterizer_pipeline.destroy_with(&mut self.ctx);
            self.pathtracer_pipeline.destroy_with(&mut self.ctx);
            self.common.destroy_with(&mut self.ctx);

            self.ctx.destroy_with(&mut ());
        }
    }
}

impl<T: Destroy<C>, C> Destroy<C> for Vec<T> {
    unsafe fn destroy_with(&mut self, ctx: &mut C) {
        self.iter_mut().for_each(|e| e.destroy_with(ctx));
    }
}

impl<T: Destroy<C> + ?Sized, C> Destroy<C> for Box<T> {
    unsafe fn destroy_with(&mut self, ctx: &mut C) {
        self.deref_mut().destroy_with(ctx);
    }
}
