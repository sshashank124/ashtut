use std::{env, path::Path};

use scene::io::FileLoader;

fn main() {
    let filename = env::args().nth(1).expect("No asset filename provided");
    let filepath = Path::new(&filename);

    let scene = if scene::gltf::Gltf::can_load(filepath) {
        scene::gltf::Gltf::load(filepath)
    } else {
        panic!("No loader found");
    };

    scene::io::save(&scene, filepath);
}
