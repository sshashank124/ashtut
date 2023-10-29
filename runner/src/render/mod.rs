mod pass;
mod sync_state;

use std::slice;

use shared::{glam, UniformObjects};

use crate::{
    data::{bounding_box::BoundingBox, gltf_scene::GltfScene},
    gpu::{
        context::Context, scope::OneshotScope, swapchain::Swapchain, sync_info::SyncInfo, Destroy,
    },
};

use self::{
    pass::{common, pathtracer, rasterizer, tonemap},
    sync_state::SyncState,
};

pub mod conf {
    pub const ASPECT_RATIO: f32 = 4.0 / 3.0;
    const HEIGHT: u32 = 768;
    pub const FRAME_RESOLUTION: ash::vk::Extent2D = ash::vk::Extent2D {
        height: HEIGHT,
        width: (HEIGHT as f32 * ASPECT_RATIO) as _,
    };
    pub const PATHTRACER_TOGGLE_THRESHOLD: u32 = 1200;
}

pub struct Renderer {
    // render pass
    common: common::Data,
    pathtracer_pipeline: pathtracer::Pipeline,
    rasterizer_pipeline: rasterizer::Pipeline,

    // tonemap pass
    tonemap_pipeline: tonemap::Pipeline,
    swapchain: Swapchain,

    // state
    uniforms: UniformObjects,
    pub use_pathtracer: bool,
    state: SyncState,
}

pub enum Error {
    NeedsRecreating,
}

impl Renderer {
    pub fn create(ctx: &mut Context, gltf_file: &str) -> Self {
        let scene = GltfScene::load(gltf_file);
        let common = common::Data::create(ctx, scene, conf::FRAME_RESOLUTION);

        let pathtracer_pipeline = pathtracer::Pipeline::create(ctx, &common);

        let mut init_scope = OneshotScope::begin_on(ctx, ctx.queues.transfer());

        let rasterizer_pipeline = rasterizer::Pipeline::create(ctx, &mut init_scope, &common);
        let tonemap_pipeline = tonemap::Pipeline::create(ctx, &common);
        let swapchain = Swapchain::create(ctx, &mut init_scope, tonemap_pipeline.render_pass);

        init_scope.finish(ctx);

        let uniforms = UniformObjects {
            view: Self::rotate_view_around(&common.scene.host_desc.bounding_box, 0),
            proj: shared::Transform::proj(glam::Mat4::perspective_rh(
                f32::to_radians(45.0),
                conf::ASPECT_RATIO,
                0.1,
                100.0,
            )),
        };

        let state = SyncState::create(ctx);

        Self {
            common,
            pathtracer_pipeline,
            rasterizer_pipeline,

            tonemap_pipeline,
            swapchain,

            uniforms,
            use_pathtracer: false,
            state,
        }
    }

    pub fn render(&mut self, ctx: &Context, elapsed_ms: u128) -> Result<(), Error> {
        unsafe {
            ctx.wait_for_fences(
                slice::from_ref(&self.state.in_flight_fence()),
                true,
                u64::MAX,
            )
            .expect("Failed to wait for fence");
        }

        self.use_pathtracer = ctx.surface.config.extent.width < conf::PATHTRACER_TOGGLE_THRESHOLD;

        self.uniforms.view =
            Self::rotate_view_around(&self.common.scene.host_desc.bounding_box, elapsed_ms);
        self.common.uniforms.update(&self.uniforms);

        let sync_info = SyncInfo::default();
        if self.use_pathtracer {
            self.pathtracer_pipeline.run(ctx, &sync_info);
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

        self.state.advance();

        (!needs_recreating)
            .then_some(())
            .ok_or(Error::NeedsRecreating)
    }

    fn rotate_view_around(bounds: &BoundingBox, elapsed_ms: u128) -> shared::Transform {
        let ms_per_rotation = 10000;
        let frac_millis = (elapsed_ms % ms_per_rotation) as f32 / ms_per_rotation as f32;
        let rotation = glam::Mat4::from_rotation_y(frac_millis * 2.0 * std::f32::consts::PI);
        let camera_pos =
            (rotation * (bounds.size() * 1.2).extend(1.0)).truncate() + bounds.center();

        shared::Transform::new(glam::Mat4::look_at_rh(
            camera_pos,
            bounds.center(),
            glam::Vec3::Y,
        ))
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
