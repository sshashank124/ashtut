use super::{commands::Commands, context::Context, Destroy};

#[allow(clippy::module_name_repetitions)]
pub type TempScope = ScopeGen<false>;
pub type Scope = ScopeGen<true>;

#[allow(clippy::module_name_repetitions)]
pub struct ScopeGen<const MULTI_USE: bool> {
    pub commands: Commands<MULTI_USE>,
    pub resources: Vec<Box<dyn Destroy<Context>>>,
}

impl<const MULTI_USE: bool> ScopeGen<MULTI_USE> {
    fn create(commands: Commands<MULTI_USE>) -> Self {
        Self {
            commands,
            resources: Vec::new(),
        }
    }

    pub fn add_resource(&mut self, resource: impl Destroy<Context> + 'static) {
        self.resources.push(Box::new(resource));
    }
}

impl TempScope {
    pub fn begin_on(ctx: &Context, commands: Commands<false>) -> Self {
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

impl Scope {
    pub fn create_on(commands: Commands<true>) -> Self {
        Self::create(commands)
    }
}

impl<const MULTI_USE: bool> Destroy<Context> for ScopeGen<MULTI_USE> {
    unsafe fn destroy_with(&mut self, ctx: &mut Context) {
        self.resources
            .iter_mut()
            .for_each(|resource| resource.destroy_with(ctx));
        self.commands.destroy_with(ctx);
    }
}
