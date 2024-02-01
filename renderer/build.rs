use std::{collections::HashMap, path::Path};

struct Compiler {
    compiler: shaderc::Compiler,
    sources: HashMap<String, String>,
}

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

impl Compiler {
    fn new(shaders_dir: impl AsRef<Path>) -> Result<Self> {
        let compiler = shaderc::Compiler::new().ok_or("Unable to initialize compiler")?;

        let shaders_dir = shaders_dir
            .as_ref()
            .to_str()
            .ok_or("Unable to read directory name")?;
        println!("cargo:rerun-if-changed={shaders_dir}");

        let mut sources = HashMap::new();
        for entry in std::fs::read_dir(shaders_dir)? {
            let entry = entry?;
            let file_name = entry.file_name();
            let file_name = file_name.to_str().ok_or("Unable to read file name")?;
            sources.insert(file_name.into(), std::fs::read_to_string(entry.path())?);
        }

        Ok(Self { compiler, sources })
    }

    fn options(&self) -> Result<shaderc::CompileOptions> {
        let mut options =
            shaderc::CompileOptions::new().ok_or("Unable to create shader options")?;
        options.set_target_env(
            shaderc::TargetEnv::Vulkan,
            shaderc::EnvVersion::Vulkan1_3 as _,
        );
        options.set_target_spirv(shaderc::SpirvVersion::V1_6);
        options.set_source_language(shaderc::SourceLanguage::GLSL);
        options.set_optimization_level(shaderc::OptimizationLevel::Performance);
        options.set_generate_debug_info();
        options.set_warnings_as_errors();
        options.set_include_callback(|source, _, _, _| {
            self.sources.get(source).map_or_else(
                || Err(format!("Unable to resolve source {source}")),
                |content| {
                    Ok(shaderc::ResolvedInclude {
                        resolved_name: source.into(),
                        content: content.into(),
                    })
                },
            )
        });
        Ok(options)
    }

    fn compile_shaders(&self, options: &shaderc::CompileOptions) -> Result<()> {
        for (name, contents) in &self.sources {
            if let Some(shader_kind) = Self::shader_kind(name)? {
                self.compile_shader(name, contents, shader_kind, options)?;
            }
        }

        Ok(())
    }

    fn compile_shader(
        &self,
        name: &str,
        source: &str,
        shader_kind: shaderc::ShaderKind,
        options: &shaderc::CompileOptions,
    ) -> Result<()> {
        let assembly =
            self.compiler
                .compile_into_spirv(source, shader_kind, name, "main", Some(options))?;
        let out_dir = std::env::var("OUT_DIR")?;
        let out_file = Path::new(&out_dir).join(name);
        std::fs::write(&out_file, assembly.as_binary_u8())?;
        println!(
            "cargo:rustc-env={name}={}",
            out_file.to_str().ok_or("Unable to read output filename")?
        );

        Ok(())
    }

    fn shader_kind(file: impl AsRef<Path>) -> Result<Option<shaderc::ShaderKind>> {
        let file_stem = file.as_ref().file_stem().ok_or(
            "Unable to read file stem, shader files should ideally have the .glsl extension",
        )?;
        let extension = Path::new(file_stem)
            .extension()
            .ok_or("Unable to read file kind, files should generally be named <name>.<kind>.glsl")?
            .to_str();

        Ok(match extension {
            Some("vert") => Some(shaderc::ShaderKind::Vertex),
            Some("frag") => Some(shaderc::ShaderKind::Fragment),
            Some("rgen") => Some(shaderc::ShaderKind::RayGeneration),
            Some("rmiss") => Some(shaderc::ShaderKind::Miss),
            Some("rchit") => Some(shaderc::ShaderKind::ClosestHit),
            _ => None,
        })
    }
}

fn main() -> Result<()> {
    let compiler = Compiler::new("../shaders")?;
    let options = compiler.options()?;
    compiler.compile_shaders(&options)?;

    Ok(())
}
