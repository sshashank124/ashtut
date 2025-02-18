mod app;
mod data;
mod input;

use std::env;

use winit::event_loop::EventLoop;

use app::App;

fn main() {
    let scene_file = env::args().nth(1).expect("Please specify a scene file");

    let event_loop = EventLoop::new().expect("Failed to create event loop");

    let window = App::window_builder()
        .build(&event_loop)
        .expect("Failed to create window");

    let app = App::new(&window, &scene_file);

    app.run(event_loop);

    if firestorm::enabled() {
        firestorm::save("./profiling_results/").expect("Failed to save profiling results");
    }
}
