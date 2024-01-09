use std::{collections::HashMap, path::Path};

use gltf::{buffer, mesh};

use shared::{
    self,
    bounding_box::BoundingBox,
    scene::{Instance, Material, PrimitiveInfo, PrimitiveSize},
};

use super::{Data, FileLoader, Info, Scene};

pub struct Gltf;

impl FileLoader for Gltf {
    const SUPPORTED_EXTENSIONS: &'static [&'static str] = &["gltf", "glb"];

    fn load(filename: impl AsRef<Path>) -> Scene {
        let (document, buffers, _images) = gltf::import(filename).expect("Couldn't import file");

        let scene = document
            .default_scene()
            .unwrap_or_else(|| document.scenes().next().expect("No scenes found"));

        let mut info = Info::default();
        let mut data = Data::default();

        let mut bounding_boxes = Vec::new();

        // json mesh index -> loaded primitives range
        let mut processed_meshes = HashMap::new();
        // json material index -> loaded material index
        let mut processed_materials = HashMap::new();

        scene.nodes().traverse_meshes(
            glam::Mat4::IDENTITY,
            &mut |mesh: &mesh::Mesh<'_>, transform| {
                let primitives_range = processed_meshes
                    .entry(mesh.index())
                    .or_insert_with(|| {
                        mesh.primitives().for_each(|primitive| {
                            let (primitive_info, primitive_size) = add_primitive(
                                &mut data,
                                &primitive,
                                &buffers,
                                &mut processed_materials,
                            );
                            info.primitive_infos.push(primitive_info);
                            info.primitive_sizes.push(primitive_size);
                            let bbox = primitive.bounding_box();
                            bounding_boxes.push(BoundingBox::new(bbox.min, bbox.max));
                        });
                        // range of newly added primitive indices
                        (info.primitive_infos.len() - mesh.primitives().len())
                            ..info.primitive_infos.len()
                    })
                    .clone();

                info.instances
                    .extend(primitives_range.map(|primitive_index| Instance {
                        primitive_index,
                        transform,
                    }));
            },
        );

        info.bounding_box = info
            .instances
            .iter()
            .map(|instance| bounding_boxes[instance.primitive_index].transform(instance.transform))
            .fold(BoundingBox::default(), BoundingBox::union);

        Scene { data, info }
    }
}

fn add_primitive(
    data: &mut Data,
    primitive: &mesh::Primitive<'_>,
    raw_buffers: &[buffer::Data],
    processed_materials: &mut HashMap<usize, usize>,
) -> (PrimitiveInfo, PrimitiveSize) {
    assert_eq!(primitive.mode(), mesh::Mode::Triangles);

    let reader = primitive.reader(|buffer| Some(&raw_buffers[buffer.index()]));

    let indices = reader.read_indices().expect("No indices found").into_u32();
    let indices_offset = data.indices.len() as _;
    data.indices.extend(indices);
    let indices_size = data.indices.len() as u32 - indices_offset;

    let positions = reader.read_positions().expect("No positions found");
    let normals = reader.read_normals().map_or_else(
        || Box::new(std::iter::repeat_with(Default::default)) as Box<_>,
        |nn| Box::new(nn) as Box<dyn Iterator<Item = [f32; 3]>>,
    );
    let tex_coords = reader
        .read_tex_coords(0)
        .map(mesh::util::ReadTexCoords::into_f32)
        .map_or_else(
            || Box::new(std::iter::repeat_with(Default::default)) as Box<_>,
            |uv| Box::new(uv) as Box<dyn Iterator<Item = [f32; 2]>>,
        );

    let vertices = positions
        .zip(normals)
        .zip(tex_coords)
        .map(shared::Vertex::from);
    let vertices_offset = data.vertices.len() as _;
    data.vertices.extend(vertices);
    let vertices_size = data.vertices.len() as u32 - vertices_offset;

    let material = primitive.material();
    let material = *processed_materials
        .entry(material.index().unwrap_or_default())
        .or_insert_with(|| add_material(data, &material)) as _;

    let primitive_info = PrimitiveInfo {
        indices_offset,
        vertices_offset,
        material,
    };

    let primitive_size = PrimitiveSize {
        indices_size,
        vertices_size,
    };

    (primitive_info, primitive_size)
}

fn add_material(data: &mut Data, material: &gltf::Material) -> usize {
    let index = data.materials.len();

    data.materials.push(Material {
        color: material.pbr_metallic_roughness().base_color_factor().into(),
        emittance: glam::Vec3::from(material.emissive_factor()).extend(1.0),
    });

    index
}

trait Traversable {
    fn traverse_meshes(
        self,
        transform: glam::Mat4,
        f: &mut impl FnMut(&mesh::Mesh<'_>, glam::Mat4),
    );
}

impl Traversable for gltf::scene::Node<'_> {
    fn traverse_meshes(
        self,
        transform: glam::Mat4,
        f: &mut impl FnMut(&mesh::Mesh<'_>, glam::Mat4),
    ) {
        let global_transform =
            transform * glam::Mat4::from_cols_array_2d(&self.transform().matrix());
        if let Some(mesh) = self.mesh() {
            f(&mesh, global_transform);
        }
        self.children().traverse_meshes(global_transform, f);
    }
}

macro_rules! impl_traversable {
    ($t:ty) => {
        impl Traversable for $t {
            fn traverse_meshes(
                self,
                transform: glam::Mat4,
                f: &mut impl FnMut(&mesh::Mesh<'_>, glam::Mat4),
            ) {
                self.for_each(|elem| elem.traverse_meshes(transform, f));
            }
        }
    };
}
impl_traversable!(gltf::scene::iter::Nodes<'_>);
impl_traversable!(gltf::scene::iter::Children<'_>);
