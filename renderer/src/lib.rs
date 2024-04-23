#![feature(adt_const_params)]

mod acceleration_structure;
mod buffer;
mod commands;
mod context;
mod descriptors;
mod image;
mod memory;
mod passes;
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

use {context::Context, swapchain::Swapchain, sync_info::SyncInfo, sync_state::SyncState};

mod conf {
    pub const VK_API_VERSION: u32 = ash::vk::make_api_version(0, 1, 3, 0);

    pub const INTERMEDIATE_FORMAT: super::image::Format = super::image::Format::Hdr;
}

trait Destroy<C> {
    unsafe fn destroy_with(&mut self, ctx: &C);
}

pub struct Renderer {
    // passes
    data: passes::Data<{ conf::INTERMEDIATE_FORMAT }>,
    pathtracer_pipeline: passes::pathtracer::Pipeline,
    rasterizer_pipeline: passes::rasterizer::Pipeline,
    tonemap_pipeline:
        passes::tonemap::Pipeline<{ conf::INTERMEDIATE_FORMAT }, { image::Format::Swapchain }>,

    swapchain: Swapchain,

    // state
    use_pathtracer: bool,
    frame: u32,
    state: SyncState,

    ctx: Context,
}

pub enum Error {
    NeedsRecreating,
}

impl Renderer {
    pub fn create(
        name: &str,
        window: &impl HasWindowHandle,
        scene: scene::Scene,
        resolution: (u32, u32),
        camera: inputs::Camera,
    ) -> Self {
        firestorm::profile_method!(create);

        let ctx = Context::init(name, window);

        let data = passes::Data::create(&ctx, scene, resolution, camera);

        let pathtracer_pipeline = passes::pathtracer::Pipeline::create(&ctx, &data);
        let rasterizer_pipeline = passes::rasterizer::Pipeline::create(&ctx, &data);
        let tonemap_pipeline = passes::tonemap::Pipeline::create(&ctx, &data);

        let swapchain = Swapchain::create(&ctx);

        let state = SyncState::create(&ctx);

        Self {
            data,
            pathtracer_pipeline,
            rasterizer_pipeline,
            tonemap_pipeline,

            swapchain,

            frame: 0,
            use_pathtracer: true,
            state,

            ctx,
        }
    }

    pub fn render(&mut self) -> Result<(), Error> {
        firestorm::profile_method!(render);

        unsafe {
            self.ctx
                .wait_for_fences(
                    slice::from_ref(&self.state.in_flight_fence()),
                    true,
                    u64::MAX,
                )
                .expect("Failed to wait for fence");
        }

        self.data.uniforms.update(&self.ctx);

        let sync_info = SyncInfo {
            wait_on: vec![],
            signal_to: vec![],
            fence: None,
        };
        if self.use_pathtracer {
            self.pathtracer_pipeline
                .run(&self.ctx, &self.data, self.frame, &sync_info);
        } else {
            self.rasterizer_pipeline
                .run(&self.ctx, &self.data, &sync_info);
        }

        let (image_index, needs_recreating) = self
            .swapchain
            .get_next_image(&self.ctx, self.state.frame_available_semaphore());
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
                    wait_on: vec![self.state.frame_available_semaphore()],
                    signal_to: vec![self.state.frame_ready_semaphore()],
                    fence: Some(self.state.in_flight_fence()),
                },
                &self.swapchain.images[image_index],
            );

            self.swapchain.present_to_when(
                &self.ctx,
                image_index,
                slice::from_ref(&self.state.frame_ready_semaphore()),
            )
        };

        self.frame += 1;
        self.state.advance();

        (!needs_recreating)
            .then_some(())
            .ok_or(Error::NeedsRecreating)
    }

    pub fn update_camera(&mut self, camera: inputs::Camera) {
        self.data.uniforms.update_camera(camera);
        self.frame = 0;
    }

    pub fn toggle_renderer(&mut self) {
        self.use_pathtracer = !self.use_pathtracer;
        self.frame = 0;
    }

    pub fn recreate(&mut self) -> bool {
        firestorm::profile_method!(recreate);

        unsafe {
            self.ctx.wait_idle();
        }

        let is_valid = self.ctx.refresh_surface_capabilities();

        if is_valid {
            unsafe {
                self.swapchain.destroy_with(&self.ctx);
            }
            self.swapchain = Swapchain::create(&self.ctx);
        }

        is_valid
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        firestorm::profile_method!(drop);

        unsafe {
            self.ctx.wait_idle();

            self.state.destroy_with(&self.ctx);

            self.swapchain.destroy_with(&self.ctx);
            self.tonemap_pipeline.destroy_with(&self.ctx);

            self.rasterizer_pipeline.destroy_with(&self.ctx);
            self.pathtracer_pipeline.destroy_with(&self.ctx);
            self.data.destroy_with(&self.ctx);
        }
    }
}

impl<T: Destroy<C>, C> Destroy<C> for Vec<T> {
    unsafe fn destroy_with(&mut self, ctx: &C) {
        self.iter_mut().for_each(|e| e.destroy_with(ctx));
    }
}

impl<T: Destroy<C> + ?Sized, C> Destroy<C> for Box<T> {
    unsafe fn destroy_with(&mut self, ctx: &C) {
        self.deref_mut().destroy_with(ctx);
    }
}
