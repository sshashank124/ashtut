use ash::vk;

use super::{
    commands::CommandsT,
    context::{queue::Queue, Context},
    Destroy,
};

#[allow(clippy::module_name_repetitions)]
pub type OneshotScope = Scope<false>;
#[allow(clippy::module_name_repetitions)]
pub type FlushableScope = Scope<true>;

pub struct Scope<const MULTI_USE: bool> {
    pub commands: CommandsT<{ MULTI_USE }>,
    pub resources: Vec<Box<dyn Destroy<Context>>>,
}

impl<const MULTI_USE: bool> Scope<{ MULTI_USE }> {
    fn create(commands: CommandsT<{ MULTI_USE }>) -> Self {
        Self {
            commands,
            resources: Vec::new(),
        }
    }

    pub fn begin_on(ctx: &Context, queue: &Queue) -> Self {
        let scope = Self::create(CommandsT::create_on_queue(ctx, queue));
        scope.commands.begin_recording(ctx);
        scope
    }

    pub fn add_resource(&mut self, resource: impl Destroy<Context> + 'static) {
        self.resources.push(Box::new(resource));
    }

    pub fn finish(mut self, ctx: &mut Context) {
        self.commands.submit(ctx, &vk::SubmitInfo::default(), None);
        unsafe { self.destroy_with(ctx) };
    }
}

impl<const MULTI_USE: bool> Destroy<Context> for Scope<{ MULTI_USE }> {
    unsafe fn destroy_with(&mut self, ctx: &mut Context) {
        self.resources
            .iter_mut()
            .for_each(|resource| resource.destroy_with(ctx));
        self.commands.destroy_with(ctx);
    }
}
