use std::{collections::HashMap, fs::File, io::BufReader, path::Path};

use gltf::{image, mesh, texture};

use crate::{
    io::FileLoader, BoundingBox, Image, Instance, Material, PrimitiveInfo, PrimitiveSize, Scene,
    TextureInfo, Vertex,
};

pub struct Gltf;

impl FileLoader for Gltf {
    const SUPPORTED_EXTENSIONS: &'static [&'static str] = &["gltf", "glb"];

    #[allow(clippy::too_many_lines)]
    fn load(filename: impl AsRef<Path>) -> Scene {
        let filename = filename.as_ref();
        let filedir = filename.parent().unwrap_or_else(|| Path::new("./"));

        let gltf::Gltf { document, blob } = {
            let file = File::open(filename).expect("Couldn't open gltf file");
            let reader = BufReader::new(file);
            gltf::Gltf::from_reader(reader).expect("Couldn't read gltf headers")
        };
        let buffers = gltf::import_buffers(&document, Some(filedir), blob)
            .expect("Unable to read gltf buffers");

        let default_scene = document
            .default_scene()
            .unwrap_or_else(|| document.scenes().next().expect("No scenes found"));

        let mut scene = Scene::default();

        // json image index -> loaded image index
        let mut processed_images = HashMap::new();
        let mut handle_image = |scene: &mut Scene, image: gltf::Image| {
            *processed_images.entry(image.index()).or_insert_with(|| {
                scene.data.images.push(Image {
                    source: match image.source() {
                        image::Source::Uri { uri, .. } => filedir.join(uri),
                        image::Source::View { .. } => panic!("Embedded images not supported"),
                    },
                });
                scene.data.images.len() - 1
            })
        };

        // json texture index -> loaded texture index
        let mut processed_textures = HashMap::new();
        let mut handle_texture = |scene: &mut Scene, tex_info: texture::Info| {
            let texture = tex_info.texture();
            *processed_textures
                .entry(texture.index())
                .or_insert_with(|| {
                    let image_index = handle_image(scene, texture.source()) as _;
                    scene.info.textures.push(TextureInfo { image_index });
                    scene.info.textures.len() - 1
                })
        };

        // json material index -> loaded material index
        let mut processed_materials = HashMap::new();
        let mut handle_material = |scene: &mut Scene, material: gltf::Material| {
            *processed_materials
                .entry(material.index().unwrap_or_default())
                .or_insert_with(|| {
                    let pbr = material.pbr_metallic_roughness();
                    let color_texture = pbr
                        .base_color_texture()
                        .map_or(-1, |tex_info| handle_texture(scene, tex_info) as _);
                    let emittance_texture = material
                        .emissive_texture()
                        .map_or(-1, |tex_info| handle_texture(scene, tex_info) as _);
                    let metallic_roughness_texture = pbr
                        .metallic_roughness_texture()
                        .map_or(-1, |tex_info| handle_texture(scene, tex_info) as _);
                    scene.data.materials.push(Material {
                        color: glam::Vec4::from(pbr.base_color_factor()).truncate(),
                        color_texture,
                        emittance: material.emissive_factor().into(),
                        emittance_texture,
                        metallic: pbr.metallic_factor(),
                        roughness: pbr.roughness_factor(),
                        metallic_roughness_texture,
                    });
                    scene.data.materials.len() - 1
                })
        };

        let mut bounding_boxes = Vec::new();
        let mut add_primitive = |scene: &mut Scene, primitive: mesh::Primitive| {
            assert_eq!(primitive.mode(), mesh::Mode::Triangles);

            let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

            let indices = reader.read_indices().expect("No indices found").into_u32();

            let positions = reader.read_positions().expect("No positions found");
            let normals = reader.read_normals().map_or_else(
                || Box::new(std::iter::repeat_with(Default::default)) as Box<_>,
                |nn| Box::new(nn) as Box<dyn Iterator<Item = [f32; 3]>>,
            );
            let tex_coords0 = reader
                .read_tex_coords(0)
                .map(mesh::util::ReadTexCoords::into_f32)
                .map_or_else(
                    || Box::new(std::iter::repeat_with(Default::default)) as Box<_>,
                    |uv| Box::new(uv) as Box<dyn Iterator<Item = [f32; 2]>>,
                );
            let tex_coords1 = reader
                .read_tex_coords(1)
                .map(mesh::util::ReadTexCoords::into_f32)
                .map_or_else(
                    || Box::new(std::iter::repeat_with(Default::default)) as Box<_>,
                    |uv| Box::new(uv) as Box<dyn Iterator<Item = [f32; 2]>>,
                );

            let vertices = positions
                .zip(normals)
                .zip(tex_coords0)
                .zip(tex_coords1)
                .map(Vertex::from);

            let material = handle_material(scene, primitive.material()) as _;

            let bbox = primitive.bounding_box();
            let bounding_box = BoundingBox::new(bbox.min, bbox.max);

            // Add primitive to scene
            let indices_offset = scene.data.indices.len() as u32;
            scene.data.indices.extend(indices);
            let indices_size = scene.data.indices.len() as u32 - indices_offset;

            let vertices_offset = scene.data.vertices.len() as u32;
            scene.data.vertices.extend(vertices);
            let vertices_size = scene.data.vertices.len() as u32 - vertices_offset;

            scene.info.primitive_infos.push(PrimitiveInfo {
                indices_offset,
                vertices_offset,
                material,
            });

            scene.info.primitive_sizes.push(PrimitiveSize {
                indices_size,
                vertices_size,
            });

            bounding_boxes.push(bounding_box);
        };

        // json mesh index -> loaded primitives range
        let mut processed_meshes = HashMap::new();
        let mut handle_mesh = |scene: &mut Scene, mesh: &mesh::Mesh| {
            processed_meshes
                .entry(mesh.index())
                .or_insert_with(|| {
                    let primitives_start = scene.info.primitive_infos.len();
                    mesh.primitives()
                        .for_each(|primitive| add_primitive(scene, primitive));
                    let primitives_end = scene.info.primitive_infos.len();
                    primitives_start..primitives_end
                })
                .clone()
        };

        default_scene.nodes().traverse_meshes(
            glam::Mat4::IDENTITY,
            &mut |mesh: &mesh::Mesh<'_>, transform| {
                let primitives_range = handle_mesh(&mut scene, mesh);

                scene
                    .info
                    .instances
                    .extend(primitives_range.map(|primitive_index| Instance {
                        primitive_index,
                        transform,
                    }));
            },
        );

        scene.info.bounding_box = scene
            .info
            .instances
            .iter()
            .map(|instance| bounding_boxes[instance.primitive_index].transform(instance.transform))
            .fold(BoundingBox::default(), BoundingBox::union);

        scene
    }
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
