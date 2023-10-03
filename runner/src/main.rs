#![allow(incomplete_features)]
#![feature(generic_const_exprs)]
#![feature(offset_of)]

mod app;
mod buffer;
mod device;
mod instance;
mod render_pipeline;
mod surface;
mod swapchain;
mod util;
mod validator;
mod vertex;

use winit::event_loop::EventLoop;

use app::App;

fn main() {
    let event_loop = EventLoop::new();
    let window = App::init_window(&event_loop);
    let app = App::new(&window);
    app.run(event_loop, window);
}
