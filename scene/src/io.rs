use std::{
    fs::File,
    io::{BufReader, BufWriter},
    path::Path,
};

use super::Scene;

pub trait FileLoader {
    const SUPPORTED_EXTENSIONS: &'static [&'static str];
    fn load(filename: impl AsRef<Path>) -> Scene;

    fn can_load(filename: impl AsRef<Path>) -> bool {
        let extension = filename
            .as_ref()
            .extension()
            .and_then(|s| s.to_str())
            .expect("No file extension found");
        Self::SUPPORTED_EXTENSIONS.contains(&extension)
    }
}

const FILE_EXTENSION: &str = "tsnasset";

pub fn load(file: impl AsRef<Path>) -> Scene {
    let filepath = file.as_ref();
    assert!(
        filepath.extension().unwrap_or_default() == FILE_EXTENSION,
        "Asset must be preprocessed before loading"
    );
    let file = File::open(filepath).expect("Unable to open scene asset file");
    let reader = flate2::bufread::GzDecoder::new(BufReader::new(file));
    rmp_serde::decode::from_read(reader).expect("Failed to load scene asset")
}

pub fn save(scene: &Scene, file: impl AsRef<Path>) {
    let output_filename = file.as_ref().with_extension(FILE_EXTENSION);
    let output_file = File::create(&output_filename).expect("Unable to open file for writing");
    let mut writer =
        flate2::write::GzEncoder::new(BufWriter::new(output_file), flate2::Compression::default());
    rmp_serde::encode::write(&mut writer, &scene).expect("Failed to save processed asset");
    println!("Asset processed and saved to {}", output_filename.display());
}
