use spirv_builder::{MetadataPrintout, SpirvBuilder};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let profile = std::env::var("PROFILE").unwrap();
    println!("cargo:rustc-env=PROFILE={profile}");
    for path in std::fs::read_dir("../shaders").unwrap() {
        SpirvBuilder::new(path.unwrap().path(), "spirv-unknown-vulkan1.2")
            .print_metadata(MetadataPrintout::Full)
            .build()?;
    }
    Ok(())
}
