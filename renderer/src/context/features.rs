use ash::vk;

use super::instance::Instance;

pub fn supported_by(instance: &Instance, physical_device: vk::PhysicalDevice) -> bool {
    let mut pageable_device_local_memory =
        vk::PhysicalDevicePageableDeviceLocalMemoryFeaturesEXT::default();
    let mut memory_priority = vk::PhysicalDeviceMemoryPriorityFeaturesEXT::default();

    let mut ray_tracing_pipeline = vk::PhysicalDeviceRayTracingPipelineFeaturesKHR::default();
    let mut acceleration_structure = vk::PhysicalDeviceAccelerationStructureFeaturesKHR::default();

    let mut v_1_3 = vk::PhysicalDeviceVulkan13Features::default();
    let mut v_1_2 = vk::PhysicalDeviceVulkan12Features::default();
    let mut v_1_1 = vk::PhysicalDeviceVulkan11Features::default();

    let mut v_1_0 = vk::PhysicalDeviceFeatures2::default()
        .push_next(&mut pageable_device_local_memory)
        .push_next(&mut memory_priority)
        .push_next(&mut ray_tracing_pipeline)
        .push_next(&mut acceleration_structure)
        .push_next(&mut v_1_3)
        .push_next(&mut v_1_2)
        .push_next(&mut v_1_1);

    unsafe { instance.get_physical_device_features2(physical_device, &mut v_1_0) };

    v_1_0.features.sampler_anisotropy > 0
            && v_1_0.features.shader_int64 > 0
            // 1.1
            && v_1_1.storage_buffer16_bit_access > 0
            && v_1_1.uniform_and_storage_buffer16_bit_access > 0
            // 1.2
            && v_1_2.buffer_device_address > 0
            && v_1_2.descriptor_binding_partially_bound > 0
            && v_1_2.descriptor_binding_variable_descriptor_count > 0
            && v_1_2.runtime_descriptor_array > 0
            && v_1_2.scalar_block_layout > 0
            && v_1_2.uniform_and_storage_buffer8_bit_access > 0
            && v_1_2.vulkan_memory_model > 0
            // 1.3
            && v_1_3.dynamic_rendering > 0
            && v_1_3.synchronization2 > 0
            // acceleration structure
            && acceleration_structure.acceleration_structure > 0
            // ray tracing pipeline
            && ray_tracing_pipeline.ray_tracing_pipeline > 0
            // memory priority
            && memory_priority.memory_priority > 0
            // pageable device local memory
            && pageable_device_local_memory.pageable_device_local_memory > 0
}

pub fn required<'a>() -> (
    vk::PhysicalDeviceFeatures2<'a>,
    [Box<dyn vk::ExtendsPhysicalDeviceFeatures2>; 7],
) {
    (
        vk::PhysicalDeviceFeatures2::default().features(
            vk::PhysicalDeviceFeatures::default()
                .sampler_anisotropy(true)
                .shader_int64(true),
        ),
        [
            Box::new(
                vk::PhysicalDeviceVulkan11Features::default()
                    .storage_buffer16_bit_access(true)
                    .uniform_and_storage_buffer16_bit_access(true),
            ),
            Box::new(
                vk::PhysicalDeviceVulkan12Features::default()
                    .buffer_device_address(true)
                    .descriptor_binding_partially_bound(true)
                    .descriptor_binding_variable_descriptor_count(true)
                    .descriptor_indexing(true)
                    .runtime_descriptor_array(true)
                    .scalar_block_layout(true)
                    .uniform_and_storage_buffer8_bit_access(true)
                    .vulkan_memory_model(true),
            ),
            Box::new(
                vk::PhysicalDeviceVulkan13Features::default()
                    .dynamic_rendering(true)
                    .synchronization2(true),
            ),
            Box::new(
                vk::PhysicalDeviceAccelerationStructureFeaturesKHR::default()
                    .acceleration_structure(true),
            ),
            Box::new(
                vk::PhysicalDeviceRayTracingPipelineFeaturesKHR::default()
                    .ray_tracing_pipeline(true),
            ),
            Box::new(vk::PhysicalDeviceMemoryPriorityFeaturesEXT::default().memory_priority(true)),
            Box::new(
                vk::PhysicalDevicePageableDeviceLocalMemoryFeaturesEXT::default()
                    .pageable_device_local_memory(true),
            ),
        ],
    )
}
