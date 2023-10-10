use shared::Vertex;

pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

impl Mesh {
    pub fn load_from_file(file_name: &str) -> Self {
        let mesh = &tobj::load_obj(file_name, &tobj::GPU_LOAD_OPTIONS)
            .expect("Failed to load OBJ file")
            .0[0]
            .mesh;

        Self::from_flat(&mesh.indices, &mesh.positions, &mesh.texcoords)
    }

    pub fn from_flat(indices: &[u32], positions: &[f32], tex_coords: &[f32]) -> Self {
        assert_eq!(positions.len() / 3, tex_coords.len() / 2);

        let vertices = positions
            .chunks_exact(3)
            .zip(
                tex_coords
                    .chunks_exact(2)
                    .map(|tex_coord| [tex_coord[0], 1.0 - tex_coord[1]]),
            )
            .map(|(pos, tex_coord)| Vertex::new(pos, &tex_coord))
            .collect();

        let indices = indices.to_owned();

        Self { vertices, indices }
    }

    pub fn vertex_data_size(&self) -> usize {
        std::mem::size_of_val(self.vertices.as_slice())
    }
}
