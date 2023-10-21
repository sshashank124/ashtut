use std::{ops::Deref, slice};

use ash::vk;
use shared::{bytemuck, Vertex};

use crate::gpu::{model::Model, query_pool::QueryPool, scope::FlushableScope};

use super::{buffer::Buffer, context::Context, Descriptions, Destroy};

pub struct AccelerationStructures {
    blases: Vec<AccelerationStructure>,
    pub tlas: AccelerationStructure,
}

pub struct AccelerationStructure {
    accel: vk::AccelerationStructureKHR,
    address: vk::DeviceAddress,
    buffer: Buffer,
}

struct BuildInfo<'a> {
    geometry: vk::AccelerationStructureBuildGeometryInfoKHR,
    sizes: vk::AccelerationStructureBuildSizesInfoKHR,
    ranges: Vec<vk::AccelerationStructureBuildRangeInfoKHR>,
    _p: std::marker::PhantomData<&'a ()>,
}

struct GeometryInfo<'a> {
    geometries: Vec<vk::AccelerationStructureGeometryKHR>,
    ranges: Vec<vk::AccelerationStructureBuildRangeInfoKHR>,
    _p: std::marker::PhantomData<&'a ()>,
}

struct InstancesInfo {
    instances: Vec<Instance>,
    buffer: Buffer,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Instance(vk::AccelerationStructureInstanceKHR);
unsafe impl bytemuck::Zeroable for Instance {}
unsafe impl bytemuck::Pod for Instance {}

impl AccelerationStructures {
    pub fn build(ctx: &mut Context, models: &[Model]) -> Self {
        assert!(!models.is_empty(), "Please provide at least 1 model");

        let mut scope = FlushableScope::begin_on(ctx, ctx.queues.compute());
        let blases = Self::build_blases(ctx, &mut scope, models);
        let tlas = Self::build_tlas(ctx, &mut scope, &blases);
        scope.finish(ctx);

        Self { blases, tlas }
    }

    fn build_tlas(
        ctx: &mut Context,
        scope: &mut FlushableScope,
        blases: &[AccelerationStructure],
    ) -> AccelerationStructure {
        let instances_info = InstancesInfo::for_instances(ctx, scope, blases);
        let geometry_info = GeometryInfo::for_instances(ctx, &instances_info);
        let mut build_info = BuildInfo::for_geometry(ctx, false, &geometry_info);
        scope.add_resource(instances_info);

        AccelerationStructure::build(ctx, scope, &mut build_info, None)
    }

    pub fn build_blases(
        ctx: &mut Context,
        scope: &mut FlushableScope,
        models: &[Model],
    ) -> Vec<AccelerationStructure> {
        let geometry_infos = GeometryInfo::for_models(ctx, models);
        let mut build_infos = BuildInfo::for_geometries(ctx, true, &geometry_infos);

        let max_scratch_size = build_infos
            .iter()
            .map(|build_info| build_info.sizes.build_scratch_size)
            .max()
            .unwrap();

        let scratch_address = AccelerationStructure::create_scratch(ctx, scope, max_scratch_size);

        let query_type = vk::QueryType::ACCELERATION_STRUCTURE_COMPACTED_SIZE_KHR;
        let query_pool = QueryPool::create(ctx, query_type, build_infos.len() as _);
        query_pool.reset(ctx, scope);

        let mut uncompacted = Vec::with_capacity(build_infos.len());

        for (idx, build_info) in build_infos.iter_mut().enumerate() {
            uncompacted.push(AccelerationStructure::build(
                ctx,
                scope,
                build_info,
                Some(scratch_address),
            ));

            unsafe {
                ctx.cmd_pipeline_barrier(
                    scope.commands.buffer,
                    vk::PipelineStageFlags::ACCELERATION_STRUCTURE_BUILD_KHR,
                    vk::PipelineStageFlags::ACCELERATION_STRUCTURE_BUILD_KHR,
                    vk::DependencyFlags::empty(),
                    slice::from_ref(
                        &vk::MemoryBarrier::builder()
                            .src_access_mask(vk::AccessFlags::ACCELERATION_STRUCTURE_WRITE_KHR)
                            .dst_access_mask(vk::AccessFlags::ACCELERATION_STRUCTURE_READ_KHR),
                    ),
                    &[],
                    &[],
                );

                ctx.ext.accel.cmd_write_acceleration_structures_properties(
                    scope.commands.buffer,
                    slice::from_ref(&build_info.geometry.dst_acceleration_structure),
                    query_type,
                    *query_pool,
                    idx as _,
                );
            }
        }

        scope.commands.flush(ctx);

        let compact_sizes: Vec<vk::DeviceSize> = query_pool.read(ctx);

        let mut compacted = Vec::with_capacity(build_infos.len());

        for (idx, build_info) in build_infos.iter_mut().enumerate() {
            build_info.sizes.acceleration_structure_size = compact_sizes[idx];
            compacted.push(AccelerationStructure::init(ctx, build_info));

            unsafe {
                let copy_info = vk::CopyAccelerationStructureInfoKHR::builder()
                    .mode(vk::CopyAccelerationStructureModeKHR::COMPACT)
                    .src(uncompacted[idx].accel)
                    .dst(compacted[idx].accel);

                ctx.ext
                    .accel
                    .cmd_copy_acceleration_structure(scope.commands.buffer, &copy_info);
            }
        }

        scope.add_resource(uncompacted);
        scope.add_resource(query_pool);

        compacted
    }
}

impl AccelerationStructure {
    fn init(ctx: &mut Context, build_info: &BuildInfo) -> Self {
        let buffer = Buffer::create(
            ctx,
            "Acceleration Structure - Buffer",
            vk::BufferCreateInfo {
                usage: vk::BufferUsageFlags::ACCELERATION_STRUCTURE_STORAGE_KHR
                    | vk::BufferUsageFlags::STORAGE_BUFFER
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
            ctx.ext
                .accel
                .create_acceleration_structure(&create_info, None)
                .expect("Failed to create acceleration structure")
        };

        let address = unsafe {
            let info = vk::AccelerationStructureDeviceAddressInfoKHR::builder()
                .acceleration_structure(accel);
            ctx.ext
                .accel
                .get_acceleration_structure_device_address(&info)
        };

        Self {
            accel,
            address,
            buffer,
        }
    }

    fn build(
        ctx: &mut Context,
        scope: &mut FlushableScope,
        build_info: &mut BuildInfo,
        scratch_address: Option<vk::DeviceAddress>,
    ) -> Self {
        build_info.geometry.scratch_data.device_address = scratch_address.unwrap_or_else(|| {
            Self::create_scratch(ctx, scope, build_info.sizes.build_scratch_size)
        });

        let accel = Self::init(ctx, build_info);
        build_info.geometry.dst_acceleration_structure = accel.accel;

        unsafe {
            ctx.ext.accel.cmd_build_acceleration_structures(
                scope.commands.buffer,
                slice::from_ref(&build_info.geometry),
                slice::from_ref(&build_info.ranges.as_slice()),
            );
        }

        accel
    }

    fn create_scratch(
        ctx: &mut Context,
        scope: &mut FlushableScope,
        size: vk::DeviceSize,
    ) -> vk::DeviceAddress {
        let scratch = Buffer::create(
            ctx,
            "Acceleration Structure - Build Scratch",
            vk::BufferCreateInfo {
                usage: vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                    | vk::BufferUsageFlags::STORAGE_BUFFER,
                size,
                ..Default::default()
            },
            gpu_allocator::MemoryLocation::GpuOnly,
        );
        let address = scratch.get_device_address(ctx);
        scope.add_resource(scratch);
        address
    }
}

impl<'a> BuildInfo<'a> {
    fn for_geometry(ctx: &Context, bottom_level: bool, geometry_info: &'a GeometryInfo) -> Self {
        let ranges = geometry_info.ranges.clone();

        let ty = if bottom_level {
            vk::AccelerationStructureTypeKHR::BOTTOM_LEVEL
        } else {
            vk::AccelerationStructureTypeKHR::TOP_LEVEL
        };

        let compaction_flag = if bottom_level {
            vk::BuildAccelerationStructureFlagsKHR::ALLOW_COMPACTION
        } else {
            vk::BuildAccelerationStructureFlagsKHR::empty()
        };

        let geometry = vk::AccelerationStructureBuildGeometryInfoKHR::builder()
            .ty(ty)
            .mode(vk::BuildAccelerationStructureModeKHR::BUILD)
            .flags(vk::BuildAccelerationStructureFlagsKHR::PREFER_FAST_TRACE | compaction_flag)
            .geometries(&geometry_info.geometries)
            .build();

        let primitive_counts = ranges
            .iter()
            .map(|range| range.primitive_count)
            .collect::<Vec<_>>();

        let sizes = unsafe {
            ctx.ext.accel.get_acceleration_structure_build_sizes(
                vk::AccelerationStructureBuildTypeKHR::DEVICE,
                &geometry,
                &primitive_counts,
            )
        };

        Self {
            geometry,
            sizes,
            ranges,
            _p: std::marker::PhantomData,
        }
    }

    fn for_geometries(
        ctx: &Context,
        bottom_level: bool,
        geometry_infos: &'a [GeometryInfo],
    ) -> Vec<Self> {
        geometry_infos
            .iter()
            .map(|geometry_info| Self::for_geometry(ctx, bottom_level, geometry_info))
            .collect()
    }
}

impl<'a> GeometryInfo<'a> {
    fn new(geometry: vk::AccelerationStructureGeometryKHR, num_primitives: u32) -> Self {
        let range = vk::AccelerationStructureBuildRangeInfoKHR::builder()
            .primitive_count(num_primitives)
            .build();

        Self {
            geometries: vec![geometry],
            ranges: vec![range],
            _p: std::marker::PhantomData,
        }
    }

    fn for_instances(ctx: &Context, instances_info: &InstancesInfo) -> Self {
        let device_address = instances_info.buffer.get_device_address(ctx);

        let geometry = vk::AccelerationStructureGeometryKHR::builder()
            .geometry_type(vk::GeometryTypeKHR::INSTANCES)
            .geometry(vk::AccelerationStructureGeometryDataKHR {
                instances: vk::AccelerationStructureGeometryInstancesDataKHR::builder()
                    .data(vk::DeviceOrHostAddressConstKHR { device_address })
                    .build(),
            })
            .build();

        Self::new(geometry, instances_info.instances.len() as _)
    }

    fn for_model(ctx: &Context, model: &'a Model) -> Self {
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

        Self::new(geometry, model.mesh.num_primitives() as _)
    }

    fn for_models(ctx: &Context, models: &'a [Model]) -> Vec<Self> {
        models
            .iter()
            .map(|model| Self::for_model(ctx, model))
            .collect()
    }
}

impl InstancesInfo {
    fn for_instances(
        ctx: &mut Context,
        scope: &mut FlushableScope,
        blases: &[AccelerationStructure],
    ) -> Self {
        let instances = Instance::for_blases(blases);

        let buffer = Buffer::create_with_data(
            ctx,
            "Acceleration Structure - Instances",
            vk::BufferCreateInfo {
                usage: vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR
                    | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
                ..Default::default()
            },
            slice::from_ref(&bytemuck::cast_slice(&instances)),
        );

        unsafe {
            ctx.cmd_pipeline_barrier(
                scope.commands.buffer,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::ACCELERATION_STRUCTURE_BUILD_KHR,
                vk::DependencyFlags::empty(),
                slice::from_ref(
                    &vk::MemoryBarrier::builder()
                        .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                        .dst_access_mask(vk::AccessFlags::ACCELERATION_STRUCTURE_WRITE_KHR),
                ),
                &[],
                &[],
            );
        }

        Self { instances, buffer }
    }
}

impl Instance {
    fn for_blas(blas: &AccelerationStructure, transform: Option<vk::TransformMatrixKHR>) -> Self {
        Self(vk::AccelerationStructureInstanceKHR {
            transform: transform.unwrap_or(vk::TransformMatrixKHR {
                matrix: [
                    1.0, 0.0, 0.0, 0.0, // row 1
                    0.0, 1.0, 0.0, 0.0, // row 2
                    0.0, 0.0, 1.0, 0.0, // row 3
                ],
            }),
            acceleration_structure_reference: vk::AccelerationStructureReferenceKHR {
                device_handle: blas.address,
            },
            instance_custom_index_and_mask: vk::Packed24_8::new(0, 0xff),
            instance_shader_binding_table_record_offset_and_flags: vk::Packed24_8::new(
                0,
                vk::GeometryInstanceFlagsKHR::TRIANGLE_FACING_CULL_DISABLE.as_raw() as _,
            ),
        })
    }

    fn for_blases(blases: &[AccelerationStructure]) -> Vec<Self> {
        blases
            .iter()
            .map(|blas| Self::for_blas(blas, None))
            .collect()
    }
}

impl Destroy<Context> for AccelerationStructures {
    unsafe fn destroy_with(&mut self, ctx: &mut Context) {
        self.tlas.destroy_with(ctx);
        self.blases.destroy_with(ctx);
    }
}

impl Destroy<Context> for AccelerationStructure {
    unsafe fn destroy_with(&mut self, ctx: &mut Context) {
        ctx.ext
            .accel
            .destroy_acceleration_structure(self.accel, None);
        self.buffer.destroy_with(ctx);
    }
}

impl Destroy<Context> for InstancesInfo {
    unsafe fn destroy_with(&mut self, ctx: &mut Context) {
        self.buffer.destroy_with(ctx);
    }
}

impl Deref for AccelerationStructure {
    type Target = vk::AccelerationStructureKHR;
    fn deref(&self) -> &Self::Target {
        &self.accel
    }
}
