use self::mesh::Mesh;

pub mod mesh;

pub struct Model {
    pub mesh: Mesh,
    pub texture: image::RgbaImage,
}

impl Model {
    pub fn from_files(mesh_file: &str, texture_file: &str) -> Self {
        Self {
            mesh: Mesh::from_file(mesh_file),
            texture: crate::util::load_image_from_file(texture_file),
        }
    }

    #[allow(dead_code)]
    pub fn demo_viking_room() -> Self {
        Self::from_files(
            "assets/models/viking_room.obj",
            "assets/textures/viking_room.png",
        )
    }

    #[allow(dead_code)]
    pub fn demo_2_planes() -> Self {
        Self {
            mesh: Mesh::demo_2_planes(),
            texture: crate::util::load_image_from_file("assets/textures/statue.jpg"),
        }
    }
}
