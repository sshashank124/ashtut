use std::collections::HashMap;

use gltf::{buffer, mesh, scene};

use shared::{self, glam::Mat4};

use super::bounding_box::BoundingBox;

pub struct GltfScene {
    pub data: Data,
    pub info: Info,
}

#[derive(Default)]
pub struct Data {
    pub indices: Vec<u32>,
    pub vertices: Vec<shared::Vertex>,
}

#[derive(Debug, Default)]
pub struct Info {
    pub primitive_infos: Vec<shared::PrimitiveInfo>,
    pub primitive_sizes: Vec<PrimitiveSize>,
    pub instances: Vec<Instance>,
    pub bounding_box: BoundingBox,
}

#[derive(Clone, Debug)]
pub struct PrimitiveSize {
    pub indices_size: u32,
    pub vertices_size: u32,
}

#[derive(Clone, Debug)]
pub struct Instance {
    pub primitive_index: usize,
    pub transform: Mat4,
}

impl GltfScene {
    pub fn load(filename: &str) -> Self {
        let (document, buffers, _images) = gltf::import(filename).expect("Couldn't import file");

        let scene = document
            .default_scene()
            .unwrap_or_else(|| document.scenes().next().expect("No scenes found"));

        let mut info = Info::default();
        let mut data = Data::default();

        let mut bounding_boxes = Vec::new();
        let mut processed_meshes = HashMap::new();

        scene
            .nodes()
            .traverse_meshes(Mat4::IDENTITY, &mut |mesh: &mesh::Mesh<'_>, transform| {
                let primitives_range = processed_meshes
                    .entry(mesh.index())
                    .or_insert_with(|| {
                        mesh.primitives().for_each(|primitive| {
                            let (primitive_info, primitive_size) =
                                data.add_primitive(&primitive, &buffers);
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
            });

        info.bounding_box = info
            .instances
            .iter()
            .map(|instance| bounding_boxes[instance.primitive_index].transform(instance.transform))
            .fold(BoundingBox::default(), BoundingBox::union);

        Self { data, info }
    }
}

impl Data {
    fn add_primitive(
        &mut self,
        primitive: &mesh::Primitive<'_>,
        raw_buffers: &[buffer::Data],
    ) -> (shared::PrimitiveInfo, PrimitiveSize) {
        let reader = primitive.reader(|buffer| Some(&raw_buffers[buffer.index()]));

        let indices = reader.read_indices().expect("No indices found").into_u32();
        let indices_offset = self.indices.len() as _;
        self.indices.extend(indices);
        let indices_size = self.indices.len() as u32 - indices_offset;

        let positions = reader.read_positions().expect("No positions found");
        let tex_coords = reader
            .read_tex_coords(0)
            .map(mesh::util::ReadTexCoords::into_f32);
        #[allow(clippy::option_if_let_else)]
        let tex_coords: Box<dyn Iterator<Item = [f32; 2]>> = if let Some(tc) = tex_coords {
            Box::new(tc)
        } else {
            Box::new(std::iter::repeat_with(Default::default))
        };

        let vertices = positions.zip(tex_coords).map(shared::Vertex::from);
        let vertices_offset = self.vertices.len() as _;
        self.vertices.extend(vertices);
        let vertices_size = self.vertices.len() as u32 - vertices_offset;

        let primitive_info = shared::PrimitiveInfo {
            indices_offset,
            vertices_offset,
        };

        let primitive_size = PrimitiveSize {
            indices_size,
            vertices_size,
        };

        (primitive_info, primitive_size)
    }
}

trait Traversable {
    fn traverse_meshes(self, transform: Mat4, f: &mut impl FnMut(&mesh::Mesh<'_>, Mat4));
}

impl Traversable for scene::Node<'_> {
    fn traverse_meshes(self, transform: Mat4, f: &mut impl FnMut(&mesh::Mesh<'_>, Mat4)) {
        let global_transform = transform * Mat4::from_cols_array_2d(&self.transform().matrix());
        if let Some(mesh) = self.mesh() {
            f(&mesh, global_transform);
        }
        self.children().traverse_meshes(global_transform, f);
    }
}

macro_rules! impl_traversable {
    ($t:ty) => {
        impl Traversable for $t {
            fn traverse_meshes(self, transform: Mat4, f: &mut impl FnMut(&mesh::Mesh<'_>, Mat4)) {
                self.for_each(|elem| elem.traverse_meshes(transform, f));
            }
        }
    };
}
impl_traversable!(scene::iter::Nodes<'_>);
impl_traversable!(scene::iter::Children<'_>);

impl PrimitiveSize {
    pub const fn count(&self) -> u32 {
        self.indices_size / 3
    }
}
