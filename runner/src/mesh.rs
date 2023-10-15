use shared::Vertex;

pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

mod data_2_planes {
    pub const INDICES: &[u32] = &[
        0, 1, 2, 0, 2, 3, // Plane 1
        4, 5, 6, 4, 6, 7, // Plane 2
    ];
    pub const POSITIONS: &[f32] = &[
        -0.5, -0.5, 0.0, 0.5, -0.5, 0.0, 0.5, 0.5, 0.0, -0.5, 0.5, 0.0, // Plane 1
        -0.5, -0.5, -0.2, 0.5, -0.5, -0.2, 0.5, 0.5, -0.2, -0.5, 0.5, -0.2, // Plane 2
    ];
    pub const TEX_COORDS: &[f32] = &[
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, // Plane 1
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, // Plane 2
    ];
}

impl Mesh {
    pub fn from_file(file_name: &str) -> Self {
        let mesh = &tobj::load_obj(file_name, &tobj::GPU_LOAD_OPTIONS)
            .expect("Failed to load OBJ file")
            .0[0]
            .mesh;

        Self::from_slices(&mesh.indices, &mesh.positions, &mesh.texcoords)
    }

    pub fn from_slices(indices: &[u32], positions: &[f32], tex_coords: &[f32]) -> Self {
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

    pub fn num_primitives(&self) -> usize {
        self.indices.len() / 3
    }

    pub fn indices_offset(&self) -> usize {
        std::mem::size_of_val(self.vertices.as_slice())
    }

    pub fn demo_2_planes() -> Self {
        Self::from_slices(
            data_2_planes::INDICES,
            data_2_planes::POSITIONS,
            data_2_planes::TEX_COORDS,
        )
    }
}
