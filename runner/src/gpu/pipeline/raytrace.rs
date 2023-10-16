use ash::vk;

use crate::gpu::{context::Context, Destroy};

use super::Specialization;

pub struct RayTrace {}

impl RayTrace {
    pub fn create() -> Self {
        Self {}
    }
}

impl Specialization for RayTrace {
    const BIND_POINT: ash::vk::PipelineBindPoint = vk::PipelineBindPoint::RAY_TRACING_KHR;

    type Output = ();
}

impl Destroy<Context> for RayTrace {
    unsafe fn destroy_with(&mut self, ctx: &mut Context) {}
}
