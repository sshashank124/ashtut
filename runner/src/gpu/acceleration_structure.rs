use ash::{extensions::khr, vk};
use shared::{bytemuck, Vertex};

use crate::gpu::{model::Model, query_pool::QueryPool, scope::FlushableScope};

use super::{buffer::Buffer, context::Context, Descriptions, Destroy};

pub struct AccelerationStructures {
    handle: khr::AccelerationStructure,
    blases: Vec<Blas>,
}

pub struct Blas {
    accel: vk::AccelerationStructureKHR,
    buffer: Buffer,
}

struct BlasBuildInfo {
    geometry: vk::AccelerationStructureBuildGeometryInfoKHR,
    sizes: vk::AccelerationStructureBuildSizesInfoKHR,
    ranges: Vec<vk::AccelerationStructureBuildRangeInfoKHR>,
}

struct ModelInfo {
    geometries: Vec<vk::AccelerationStructureGeometryKHR>,
    ranges: Vec<vk::AccelerationStructureBuildRangeInfoKHR>,
}

impl AccelerationStructures {
    pub fn new(handle: khr::AccelerationStructure, count: usize) -> Self {
        Self {
            handle,
            blases: Vec::with_capacity(count),
        }
    }

    pub fn build_blases_for_models(ctx: &mut Context, models: &[Model]) -> Self {
        assert!(!models.is_empty(), "Please provide at least 1 model");

        let mut scope = FlushableScope::begin_on(ctx, ctx.queues.graphics());

        let handle = khr::AccelerationStructure::new(&ctx.instance, &ctx.device);

        let mut build_infos = models
            .iter()
            .map(|model| Self::get_build_info(&handle, Self::get_model_info(ctx, model)))
            .collect::<Vec<_>>();

        let max_scratch_size = build_infos
            .iter()
            .map(|blas_info| blas_info.sizes.build_scratch_size)
            .max()
            .unwrap();

        let scratch = Buffer::create(
            ctx,
            "Acceleration Structure - Build Scratch",
            vk::BufferCreateInfo {
                usage: vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                    | vk::BufferUsageFlags::STORAGE_BUFFER,
                size: max_scratch_size,
                ..Default::default()
            },
            gpu_allocator::MemoryLocation::GpuOnly,
        );
        let scratch_address = scratch.get_device_address(ctx);

        let query_type = vk::QueryType::ACCELERATION_STRUCTURE_COMPACTED_SIZE_KHR;
        let query_pool = QueryPool::create(ctx, query_type, build_infos.len() as _);
        query_pool.reset(ctx, &scope);

        let mut uncompacted = Self::new(handle.clone(), build_infos.len());

        for (idx, build_info) in build_infos.iter_mut().enumerate() {
            build_info.geometry.scratch_data.device_address = scratch_address;

            let blas = Blas::init(ctx, &handle, build_info);
            build_info.geometry.dst_acceleration_structure = blas.accel;
            uncompacted.blases.push(blas);

            unsafe {
                handle.cmd_build_acceleration_structures(
                    scope.commands.buffer,
                    &[build_info.geometry],
                    &[&build_info.ranges],
                );

                ctx.cmd_pipeline_barrier(
                    scope.commands.buffer,
                    vk::PipelineStageFlags::ACCELERATION_STRUCTURE_BUILD_KHR,
                    vk::PipelineStageFlags::ACCELERATION_STRUCTURE_BUILD_KHR,
                    vk::DependencyFlags::empty(),
                    &[vk::MemoryBarrier::builder()
                        .src_access_mask(vk::AccessFlags::ACCELERATION_STRUCTURE_WRITE_KHR)
                        .dst_access_mask(vk::AccessFlags::ACCELERATION_STRUCTURE_READ_KHR)
                        .build()],
                    &[],
                    &[],
                );

                handle.cmd_write_acceleration_structures_properties(
                    scope.commands.buffer,
                    &[build_info.geometry.dst_acceleration_structure],
                    query_type,
                    *query_pool,
                    idx as _,
                );
            }
        }

        scope.commands.flush(ctx);

        let compact_sizes: Vec<vk::DeviceSize> = query_pool.read(ctx);

        let mut compacted = Self::new(handle.clone(), build_infos.len());

        for (idx, build_info) in build_infos.iter_mut().enumerate() {
            build_info.sizes.acceleration_structure_size = compact_sizes[idx];
            compacted.blases.push(Blas::init(ctx, &handle, build_info));

            unsafe {
                let copy_info = vk::CopyAccelerationStructureInfoKHR::builder()
                    .mode(vk::CopyAccelerationStructureModeKHR::COMPACT)
                    .src(uncompacted.blases[idx].accel)
                    .dst(compacted.blases[idx].accel);

                handle.cmd_copy_acceleration_structure(scope.commands.buffer, &copy_info);
            }
        }

        scope.add_resource(uncompacted);
        scope.add_resource(query_pool);
        scope.add_resource(scratch);

        scope.finish(ctx);

        compacted
    }

    fn get_build_info(handle: &khr::AccelerationStructure, model_info: ModelInfo) -> BlasBuildInfo {
        let geometry = vk::AccelerationStructureBuildGeometryInfoKHR::builder()
            .ty(vk::AccelerationStructureTypeKHR::BOTTOM_LEVEL)
            .mode(vk::BuildAccelerationStructureModeKHR::BUILD)
            .flags(
                vk::BuildAccelerationStructureFlagsKHR::ALLOW_COMPACTION
                    | vk::BuildAccelerationStructureFlagsKHR::PREFER_FAST_TRACE,
            )
            .geometries(&model_info.geometries)
            .build();

        let primitive_counts = model_info
            .ranges
            .iter()
            .map(|range| range.primitive_count)
            .collect::<Vec<_>>();

        let sizes = unsafe {
            handle.get_acceleration_structure_build_sizes(
                vk::AccelerationStructureBuildTypeKHR::DEVICE,
                &geometry,
                &primitive_counts,
            )
        };

        BlasBuildInfo {
            geometry,
            sizes,
            ranges: model_info.ranges,
        }
    }

    fn get_model_info(ctx: &Context, model: &Model) -> ModelInfo {
        let (vertex_device_address, index_device_address) = model.buffer_device_addresses(ctx);
        let triangles = vk::AccelerationStructureGeometryTrianglesDataKHR::builder()
            .vertex_format(vk::Format::R32G32B32A32_SFLOAT)
            .vertex_stride(Vertex::size() as _)
            .max_vertex((model.mesh.vertices.len() - 1) as _)
            .vertex_data(vk::DeviceOrHostAddressConstKHR {
                device_address: vertex_device_address
                    + bytemuck::offset_of!(Vertex, position) as vk::DeviceAddress,
            })
            .index_type(vk::IndexType::UINT32)
            .index_data(vk::DeviceOrHostAddressConstKHR {
                device_address: index_device_address,
            })
            .build();

        let geometry = vk::AccelerationStructureGeometryKHR::builder()
            .geometry_type(vk::GeometryTypeKHR::TRIANGLES)
            .flags(vk::GeometryFlagsKHR::OPAQUE)
            .geometry(vk::AccelerationStructureGeometryDataKHR { triangles })
            .build();

        let range = vk::AccelerationStructureBuildRangeInfoKHR::builder()
            .primitive_count(model.mesh.num_primitives() as _)
            .build();

        ModelInfo {
            geometries: vec![geometry],
            ranges: vec![range],
        }
    }
}

impl Blas {
    fn init(
        ctx: &mut Context,
        handle: &khr::AccelerationStructure,
        build_info: &BlasBuildInfo,
    ) -> Self {
        let buffer = Buffer::create(
            ctx,
            "Acceleration Structure - Buffer",
            vk::BufferCreateInfo {
                usage: vk::BufferUsageFlags::ACCELERATION_STRUCTURE_STORAGE_KHR
                    | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
                size: build_info.sizes.acceleration_structure_size,
                ..Default::default()
            },
            gpu_allocator::MemoryLocation::GpuOnly,
        );

        let create_info = vk::AccelerationStructureCreateInfoKHR::builder()
            .ty(build_info.geometry.ty)
            .size(build_info.sizes.acceleration_structure_size)
            .buffer(*buffer);

        let accel = unsafe {
            handle
                .create_acceleration_structure(&create_info, None)
                .expect("Failed to create acceleration structure")
        };

        Self { accel, buffer }
    }
}

impl Destroy<Context> for AccelerationStructures {
    unsafe fn destroy_with(&mut self, ctx: &mut Context) {
        for accel in &mut self.blases {
            self.handle
                .destroy_acceleration_structure(accel.accel, None);
            accel.buffer.destroy_with(ctx);
        }
    }
}
