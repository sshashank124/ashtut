use std::collections::HashMap;

use gltf::{buffer, mesh, scene};

use shared::{glam::Mat4, Vertex};

use super::{bounding_box::BoundingBox, Instance, Primitive, Range};

pub struct GltfScene {
    pub data: Data,
    pub primitives: Vec<Primitive>,
    pub instances: Vec<Instance>,
    pub bounding_box: BoundingBox,
}

#[derive(Default)]
pub struct Data {
    pub indices: Vec<u32>,
    pub vertices: Vec<Vertex>,
}

impl GltfScene {
    pub fn load(filename: &str) -> Self {
        let (document, buffers, _images) = gltf::import(filename).expect("Couldn't import file");

        let scene = document
            .default_scene()
            .unwrap_or_else(|| document.scenes().next().expect("No scenes found"));

        let mut data = Data::default();
        let mut primitives = Vec::new();
        let mut instances = Vec::new();
        let mut processed_meshes = HashMap::new();
        let mut bounding_boxes = Vec::new();

        scene
            .nodes()
            .traverse_meshes(Mat4::IDENTITY, &mut |mesh: &mesh::Mesh<'_>, transform| {
                let primitives_range = processed_meshes
                    .entry(mesh.index())
                    .or_insert_with(|| {
                        mesh.primitives().for_each(|primitive| {
                            primitives.push(data.add_primitive(&primitive, &buffers));
                            let bbox = primitive.bounding_box();
                            bounding_boxes.push(BoundingBox::new(bbox.min, bbox.max));
                        });
                        // range of newly added primitive indices
                        (primitives.len() - mesh.primitives().len())..primitives.len()
                    })
                    .clone();

                instances.extend(primitives_range.map(|primitive_index| Instance {
                    primitive_index,
                    transform,
                }));
            });

        let bounding_box = instances
            .iter()
            .map(|instance| bounding_boxes[instance.primitive_index].transform(instance.transform))
            .fold(BoundingBox::default(), BoundingBox::union);

        Self {
            data,
            primitives,
            instances,
            bounding_box,
        }
    }
}

impl Data {
    fn add_primitive(
        &mut self,
        primitive: &mesh::Primitive<'_>,
        raw_buffers: &[buffer::Data],
    ) -> Primitive {
        let reader = primitive.reader(|buffer| Some(&raw_buffers[buffer.index()]));

        let indices = reader.read_indices().expect("No indices found").into_u32();
        let start = self.indices.len();
        self.indices.extend(indices);
        let end = self.indices.len();
        let indices = Range { start, end };

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

        let vertices = positions.zip(tex_coords).map(Vertex::from);
        let start = self.vertices.len();
        self.vertices.extend(vertices);
        let end = self.vertices.len();
        let vertices = Range { start, end };

        Primitive { indices, vertices }
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
