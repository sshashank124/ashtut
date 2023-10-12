use super::{commands::TempCommands, context::Context, Destroy};

pub struct Scope {
    pub commands: TempCommands,
    pub resources: Vec<Box<dyn Destroy<Context>>>,
}

impl Scope {
    fn create(commands: TempCommands) -> Self {
        Self {
            commands,
            resources: Vec::new(),
        }
    }

    pub fn add_resource(&mut self, resource: impl Destroy<Context> + 'static) {
        self.resources.push(Box::new(resource));
    }

    pub fn begin_on(ctx: &Context, commands: TempCommands) -> Self {
        let scope = Self::create(commands);
        scope.commands.begin_recording(ctx);
        scope
    }

    pub fn finish(mut self, ctx: &mut Context) {
        self.commands.finish_recording(ctx);
        self.commands.submit(ctx);
        unsafe { self.destroy_with(ctx) };
    }
}

impl Destroy<Context> for Scope {
    unsafe fn destroy_with(&mut self, ctx: &mut Context) {
        self.resources
            .iter_mut()
            .for_each(|resource| resource.destroy_with(ctx));
        self.commands.destroy_with(ctx);
    }
}
