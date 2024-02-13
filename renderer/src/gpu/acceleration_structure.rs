use std::{ops::Deref, slice};

use ash::vk;

use crate::{
    gpu::{query_pool::QueryPool, scene::Scene, scope::FlushableScope},
    util,
};

use super::{buffer::Buffer, context::Context, Destroy};

pub struct AccelerationStructures {
    blases: Vec<AccelerationStructure>,
    pub tlas: AccelerationStructure,
}

pub struct AccelerationStructure {
    accel: vk::AccelerationStructureKHR,
    address: vk::DeviceAddress,
    buffer: Buffer,
}

#[derive(Debug)]
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
    pub fn build(ctx: &mut Context, scene: &Scene) -> Self {
        let mut scope =
            FlushableScope::begin_on(ctx, "Build Acceleration Structures", ctx.queues.compute());
        let blases = Self::build_blases(ctx, &mut scope, scene);
        let tlas = Self::build_tlas(ctx, &mut scope, scene, &blases);
        scope.finish(ctx);

        Self { blases, tlas }
    }

    fn build_tlas(
        ctx: &mut Context,
        scope: &mut FlushableScope,
        scene: &Scene,
        blases: &[AccelerationStructure],
    ) -> AccelerationStructure {
        let instances_info =
            InstancesInfo::for_instances(ctx, scope, &scene.host_info.instances, blases);
        let geometry_info = GeometryInfo::for_instances(ctx, &instances_info);
        let mut build_info = BuildInfo::for_geometry(ctx, false, &geometry_info);
        scope.add_resource(instances_info);

        AccelerationStructure::build(ctx, scope, "Top Level", &mut build_info, None)
    }

    pub fn build_blases(
        ctx: &mut Context,
        scope: &mut FlushableScope,
        scene: &Scene,
    ) -> Vec<AccelerationStructure> {
        let geometry_infos = GeometryInfo::for_primitives(scene);
        let mut build_infos = BuildInfo::for_geometries(ctx, true, &geometry_infos);

        let max_scratch_size = build_infos
            .iter()
            .map(|build_info| build_info.sizes.build_scratch_size)
            .max()
            .unwrap();

        let scratch_address =
            AccelerationStructure::create_scratch(ctx, scope, "Bottom Level", max_scratch_size);

        let query_type = vk::QueryType::ACCELERATION_STRUCTURE_COMPACTED_SIZE_KHR;
        let query_pool = QueryPool::create(
            ctx,
            "Acceleration Structure Compacted Size",
            query_type,
            build_infos.len() as _,
        );
        query_pool.reset(ctx, scope);

        let mut uncompacted = Vec::with_capacity(build_infos.len());

        for (idx, build_info) in build_infos.iter_mut().enumerate() {
            uncompacted.push(AccelerationStructure::build(
                ctx,
                scope,
                &idx.to_string(),
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
            compacted.push(AccelerationStructure::init(
                ctx,
                &idx.to_string(),
                build_info,
            ));

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
    fn init(ctx: &mut Context, name: impl AsRef<str>, build_info: &BuildInfo) -> Self {
        let object_name = String::from(name.as_ref()) + " - Acceleration Structure";

        let buffer = Buffer::create(
            ctx,
            &object_name,
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
        ctx.set_debug_name(accel, object_name);

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
        name: &str,
        build_info: &mut BuildInfo,
        scratch_address: Option<vk::DeviceAddress>,
    ) -> Self {
        build_info.geometry.scratch_data.device_address = scratch_address.unwrap_or_else(|| {
            Self::create_scratch(ctx, scope, name, build_info.sizes.build_scratch_size)
        });

        let accel = Self::init(ctx, name, build_info);
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
        name: impl AsRef<str>,
        size: vk::DeviceSize,
    ) -> vk::DeviceAddress {
        let min_alignment = ctx
            .physical_device
            .properties
            .acceleration_structure
            .min_acceleration_structure_scratch_offset_alignment as _;

        let scratch = Buffer::create(
            ctx,
            String::from(name.as_ref()) + " - Acceleration Structure Build Scratch",
            vk::BufferCreateInfo {
                usage: vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                    | vk::BufferUsageFlags::STORAGE_BUFFER,
                size: util::align_to(size as _, min_alignment) as _,
                ..Default::default()
            },
            gpu_allocator::MemoryLocation::GpuOnly,
        );
        let address = util::align_to(scratch.get_device_address(ctx) as _, min_alignment) as _;

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
    fn new(
        geometry: vk::AccelerationStructureGeometryKHR,
        range: vk::AccelerationStructureBuildRangeInfoKHR,
    ) -> Self {
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

        let range = vk::AccelerationStructureBuildRangeInfoKHR::builder()
            .primitive_count(instances_info.instances.len() as _)
            .build();

        Self::new(geometry, range)
    }

    fn for_primitive(
        scene: &'a Scene,
        primitive_info: &scene::PrimitiveInfo,
        primitive_size: &scene::PrimitiveSize,
    ) -> Self {
        let triangles = vk::AccelerationStructureGeometryTrianglesDataKHR::builder()
            .vertex_format(vk::Format::R32G32B32_SFLOAT)
            .vertex_stride(std::mem::size_of::<scene::Vertex>() as _)
            .max_vertex(primitive_size.vertices_size - 1)
            .vertex_data(vk::DeviceOrHostAddressConstKHR {
                device_address: scene.device_info.vertices_address
                    + bytemuck::offset_of!(scene::Vertex, position) as vk::DeviceAddress,
            })
            .index_type(vk::IndexType::UINT32)
            .index_data(vk::DeviceOrHostAddressConstKHR {
                device_address: scene.device_info.indices_address,
            })
            .build();

        let geometry = vk::AccelerationStructureGeometryKHR::builder()
            .geometry_type(vk::GeometryTypeKHR::TRIANGLES)
            .flags(vk::GeometryFlagsKHR::OPAQUE)
            .geometry(vk::AccelerationStructureGeometryDataKHR { triangles })
            .build();

        let range = vk::AccelerationStructureBuildRangeInfoKHR::builder()
            .primitive_count(primitive_size.count())
            .primitive_offset(primitive_info.indices_offset * std::mem::size_of::<u32>() as u32)
            .first_vertex(primitive_info.vertices_offset)
            .build();

        Self::new(geometry, range)
    }

    fn for_primitives(scene: &'a Scene) -> Vec<Self> {
        scene
            .host_info
            .primitive_infos
            .iter()
            .zip(scene.host_info.primitive_sizes.iter())
            .map(|(primitive_info, primitive_size)| {
                Self::for_primitive(scene, primitive_info, primitive_size)
            })
            .collect()
    }
}

impl InstancesInfo {
    fn for_instances(
        ctx: &mut Context,
        scope: &FlushableScope,
        instances: &[scene::Instance],
        blases: &[AccelerationStructure],
    ) -> Self {
        let instances = Instance::for_instances(instances, blases);

        let buffer = Buffer::create_with_data(
            ctx,
            "Instances",
            vk::BufferCreateInfo {
                usage: vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR
                    | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
                ..Default::default()
            },
            bytemuck::cast_slice(&instances),
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
    fn for_instance(instance: &scene::Instance, blases: &[AccelerationStructure]) -> Self {
        let t = &instance.transform;
        Self(vk::AccelerationStructureInstanceKHR {
            transform: vk::TransformMatrixKHR {
                matrix: [
                    t.x_axis.x, t.y_axis.x, t.z_axis.x, t.w_axis.x, // row 1
                    t.x_axis.y, t.y_axis.y, t.z_axis.y, t.w_axis.y, // row 2
                    t.x_axis.z, t.y_axis.z, t.z_axis.z, t.w_axis.z, // row 3
                ],
            },
            acceleration_structure_reference: vk::AccelerationStructureReferenceKHR {
                device_handle: blases[instance.primitive_index].address,
            },
            instance_custom_index_and_mask: vk::Packed24_8::new(
                instance.primitive_index as _,
                0xff,
            ),
            instance_shader_binding_table_record_offset_and_flags: vk::Packed24_8::new(
                0,
                vk::GeometryInstanceFlagsKHR::TRIANGLE_FACING_CULL_DISABLE.as_raw() as _,
            ),
        })
    }

    fn for_instances(instances: &[scene::Instance], blases: &[AccelerationStructure]) -> Vec<Self> {
        instances
            .iter()
            .map(|instance| Self::for_instance(instance, blases))
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
