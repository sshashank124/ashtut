use std::{env, path::Path};

use scene::{gltf::Gltf, FileLoader};

fn main() {
    let filename = env::args().nth(1).expect("No asset filename provided");
    let filepath = Path::new(&filename);

    let scene = if Gltf::can_load(filepath) {
        Gltf::load(filepath)
    } else {
        panic!("No loader found");
    };

    scene::save(&scene, filepath);
}
