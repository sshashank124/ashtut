use ash::vk;

use super::instance::Instance;

pub struct Features {
    // Core
    pub v_1_0: V10Features,
    pub v_1_1: V11Features,
    pub v_1_2: V12Features,
    pub v_1_3: V13Features,
    // Ray Tracing
    pub acceleration_structure: AccelerationStructureFeatures,
    pub ray_tracing_pipeline: RayTracingPipelineFeatures,
}

pub struct V10Features {
    sampler_anisotropy: bool,
    shader_int64: bool,
}
pub struct V11Features {
    storage_buffer16_bit_access: bool,
    uniform_and_storage_buffer16_bit_access: bool,
}
pub struct V12Features {
    buffer_device_address: bool,
    descriptor_binding_partially_bound: bool,
    descriptor_binding_variable_descriptor_count: bool,
    runtime_descriptor_array: bool,
    scalar_block_layout: bool,
    uniform_and_storage_buffer8_bit_access: bool,
    vulkan_memory_model: bool,
}
pub struct V13Features {
    dynamic_rendering: bool,
    synchronization2: bool,
}
pub struct AccelerationStructureFeatures {
    acceleration_structure: bool,
}
pub struct RayTracingPipelineFeatures {
    ray_tracing_pipeline: bool,
}

impl Features {
    pub fn get_supported(instance: &Instance, physical_device: vk::PhysicalDevice) -> Self {
        let mut ray_tracing_pipeline = vk::PhysicalDeviceRayTracingPipelineFeaturesKHR::default();
        let mut acceleration_structure =
            vk::PhysicalDeviceAccelerationStructureFeaturesKHR::default();

        let mut v_1_3 = vk::PhysicalDeviceVulkan13Features::default();
        let mut v_1_2 = vk::PhysicalDeviceVulkan12Features::default();
        let mut v_1_1 = vk::PhysicalDeviceVulkan11Features::default();

        let mut v_1_0 = vk::PhysicalDeviceFeatures2::default()
            .push_next(&mut ray_tracing_pipeline)
            .push_next(&mut acceleration_structure)
            .push_next(&mut v_1_3)
            .push_next(&mut v_1_2)
            .push_next(&mut v_1_1);

        unsafe { instance.get_physical_device_features2(physical_device, &mut v_1_0) };

        Self {
            v_1_0: V10Features::from(v_1_0),
            v_1_1: V11Features::from(v_1_1),
            v_1_2: V12Features::from(v_1_2),
            v_1_3: V13Features::from(v_1_3),
            acceleration_structure: AccelerationStructureFeatures::from(acceleration_structure),
            ray_tracing_pipeline: RayTracingPipelineFeatures::from(ray_tracing_pipeline),
        }
    }

    pub fn required<'a>() -> (
        vk::PhysicalDeviceFeatures2<'a>,
        [Box<dyn vk::ExtendsPhysicalDeviceFeatures2>; 5],
    ) {
        (
            vk::PhysicalDeviceFeatures2::default().features(V10Features::required()),
            [
                Box::new(V11Features::required()),
                Box::new(V12Features::required()),
                Box::new(V13Features::required()),
                Box::new(AccelerationStructureFeatures::required()),
                Box::new(RayTracingPipelineFeatures::required()),
            ],
        )
    }

    pub const fn supports_requirements(&self) -> bool {
        self.v_1_0.supports_requirements()
            && self.v_1_1.supports_requirements()
            && self.v_1_2.supports_requirements()
            && self.v_1_3.supports_requirements()
            && self.acceleration_structure.supports_requirements()
            && self.ray_tracing_pipeline.supports_requirements()
    }
}

impl V10Features {
    pub const fn supports_requirements(&self) -> bool {
        self.sampler_anisotropy && self.shader_int64
    }

    pub fn required() -> vk::PhysicalDeviceFeatures {
        vk::PhysicalDeviceFeatures::default()
            .sampler_anisotropy(true)
            .shader_int64(true)
    }
}

impl V11Features {
    pub const fn supports_requirements(&self) -> bool {
        self.storage_buffer16_bit_access && self.uniform_and_storage_buffer16_bit_access
    }

    pub fn required<'a>() -> vk::PhysicalDeviceVulkan11Features<'a> {
        vk::PhysicalDeviceVulkan11Features::default()
            .storage_buffer16_bit_access(true)
            .uniform_and_storage_buffer16_bit_access(true)
    }
}

impl V12Features {
    pub const fn supports_requirements(&self) -> bool {
        self.buffer_device_address
            && self.descriptor_binding_partially_bound
            && self.descriptor_binding_variable_descriptor_count
            && self.runtime_descriptor_array
            && self.scalar_block_layout
            && self.uniform_and_storage_buffer8_bit_access
            && self.vulkan_memory_model
    }

    pub fn required<'a>() -> vk::PhysicalDeviceVulkan12Features<'a> {
        vk::PhysicalDeviceVulkan12Features::default()
            .buffer_device_address(true)
            .descriptor_binding_partially_bound(true)
            .descriptor_binding_variable_descriptor_count(true)
            .runtime_descriptor_array(true)
            .scalar_block_layout(true)
            .uniform_and_storage_buffer8_bit_access(true)
            .vulkan_memory_model(true)
    }
}

impl V13Features {
    pub const fn supports_requirements(&self) -> bool {
        self.dynamic_rendering && self.synchronization2
    }

    pub fn required<'a>() -> vk::PhysicalDeviceVulkan13Features<'a> {
        vk::PhysicalDeviceVulkan13Features::default()
            .dynamic_rendering(true)
            .synchronization2(true)
    }
}

impl AccelerationStructureFeatures {
    pub const fn supports_requirements(&self) -> bool {
        self.acceleration_structure
    }

    pub fn required<'a>() -> vk::PhysicalDeviceAccelerationStructureFeaturesKHR<'a> {
        vk::PhysicalDeviceAccelerationStructureFeaturesKHR::default().acceleration_structure(true)
    }
}

impl RayTracingPipelineFeatures {
    pub const fn supports_requirements(&self) -> bool {
        self.ray_tracing_pipeline
    }

    pub fn required<'a>() -> vk::PhysicalDeviceRayTracingPipelineFeaturesKHR<'a> {
        vk::PhysicalDeviceRayTracingPipelineFeaturesKHR::default().ray_tracing_pipeline(true)
    }
}

impl From<vk::PhysicalDeviceFeatures2<'_>> for V10Features {
    fn from(f: vk::PhysicalDeviceFeatures2) -> Self {
        Self {
            sampler_anisotropy: f.features.sampler_anisotropy > 0,
            shader_int64: f.features.shader_int64 > 0,
        }
    }
}

impl From<vk::PhysicalDeviceVulkan11Features<'_>> for V11Features {
    fn from(f: vk::PhysicalDeviceVulkan11Features) -> Self {
        Self {
            storage_buffer16_bit_access: f.storage_buffer16_bit_access > 0,
            uniform_and_storage_buffer16_bit_access: f.uniform_and_storage_buffer16_bit_access > 0,
        }
    }
}

impl From<vk::PhysicalDeviceVulkan12Features<'_>> for V12Features {
    fn from(f: vk::PhysicalDeviceVulkan12Features) -> Self {
        Self {
            buffer_device_address: f.buffer_device_address > 0,
            descriptor_binding_partially_bound: f.descriptor_binding_partially_bound > 0,
            descriptor_binding_variable_descriptor_count: f
                .descriptor_binding_variable_descriptor_count
                > 0,
            runtime_descriptor_array: f.runtime_descriptor_array > 0,
            scalar_block_layout: f.scalar_block_layout > 0,
            uniform_and_storage_buffer8_bit_access: f.uniform_and_storage_buffer8_bit_access > 0,
            vulkan_memory_model: f.vulkan_memory_model > 0,
        }
    }
}

impl From<vk::PhysicalDeviceVulkan13Features<'_>> for V13Features {
    fn from(f: vk::PhysicalDeviceVulkan13Features) -> Self {
        Self {
            dynamic_rendering: f.dynamic_rendering > 0,
            synchronization2: f.synchronization2 > 0,
        }
    }
}

impl From<vk::PhysicalDeviceAccelerationStructureFeaturesKHR<'_>>
    for AccelerationStructureFeatures
{
    fn from(f: vk::PhysicalDeviceAccelerationStructureFeaturesKHR) -> Self {
        Self {
            acceleration_structure: f.acceleration_structure > 0,
        }
    }
}

impl From<vk::PhysicalDeviceRayTracingPipelineFeaturesKHR<'_>> for RayTracingPipelineFeatures {
    fn from(f: vk::PhysicalDeviceRayTracingPipelineFeaturesKHR) -> Self {
        Self {
            ray_tracing_pipeline: f.ray_tracing_pipeline > 0,
        }
    }
}
