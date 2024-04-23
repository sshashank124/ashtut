use ash::vk;

use super::{commands::Commands, context::Context, Destroy};

pub struct Scope {
    pub commands: Commands,
    resources: Vec<Box<dyn Destroy<Context>>>,
}

impl Scope {
    pub fn new(commands: Commands) -> Self {
        Self {
            commands,
            resources: Vec::new(),
        }
    }

    pub fn add_resource(&mut self, resource: impl Destroy<Context> + 'static) {
        self.resources.push(Box::new(resource));
    }

    pub fn finish(mut self, ctx: &Context) {
        self.commands.submit(ctx, &vk::SubmitInfo::default(), None);
        unsafe { self.destroy_with(ctx) };
    }
}

impl Destroy<Context> for Scope {
    unsafe fn destroy_with(&mut self, ctx: &Context) {
        firestorm::profile_method!(destroy_with);

        self.commands.destroy_with(ctx);
        self.resources.destroy_with(ctx);
    }
}
