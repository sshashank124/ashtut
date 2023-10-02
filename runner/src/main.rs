mod app;
mod device;
mod instance;
mod render_pipeline;
mod surface;
mod swapchain;
mod util;
mod validator;

use winit::event_loop::EventLoop;

use app::App;

fn main() {
    let event_loop = EventLoop::new();
    let window = App::init_window(&event_loop);
    let app = App::new(&window);
    app.run(event_loop, window);
}
