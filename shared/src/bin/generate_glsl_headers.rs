use std::{env, path::Path};

use glsl::GlslStruct;
use shared::{inputs, scene};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

const GLSL_EXTENSION: &str = "h.glsl";

struct GlslHeader {
    name: &'static str,
    definitions: Vec<String>,
}

impl GlslHeader {
    fn glsl_definition(&self) -> String {
        String::from("// AUTO-GENERATED: do not edit\n\n") + &self.definitions.join("\n")
    }
}

fn main() -> Result<()> {
    let output_dirname = env::args().nth(1).expect("No output directory specified");
    let output_dir = Path::new(&output_dirname);
    assert!(output_dir.is_dir());

    let headers = [
        GlslHeader {
            name: "inputs",
            definitions: vec![
                inputs::Transform::glsl_struct_definition(),
                inputs::Camera::glsl_struct_definition(),
                inputs::Uniforms::glsl_struct_definition(),
                inputs::RasterizerConstants::glsl_struct_definition(),
                inputs::PathtracerConstants::glsl_struct_definition(),
            ],
        },
        GlslHeader {
            name: "scene",
            definitions: vec![
                scene::SceneDesc::glsl_struct_definition(),
                scene::Vertex::glsl_struct_definition(),
                scene::Material::glsl_struct_definition(),
                scene::PrimitiveInfo::glsl_struct_definition(),
            ],
        },
    ];

    for header in headers {
        let mut output_file = output_dir.to_owned();
        output_file.push(header.name);
        output_file.set_extension(GLSL_EXTENSION);
        std::fs::write(output_file, header.glsl_definition())?;
    }

    Ok(())
}
