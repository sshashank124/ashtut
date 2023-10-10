use ash::vk;

use super::command_pool::CommandPool;

use crate::{
    context::{queue::Queue, Context},
    util::Destroy,
};

pub struct CommandBuilder {
    queue: vk::Queue,
    pool: CommandPool,
    pub command_buffer: vk::CommandBuffer,
    pub temporaries: Vec<Box<dyn Destroy<Context>>>,
}

impl CommandBuilder {
    pub fn new(ctx: &Context, queue: &Queue) -> Self {
        let pool = {
            let info = vk::CommandPoolCreateInfo::builder()
                .queue_family_index(queue.family_index)
                .flags(vk::CommandPoolCreateFlags::TRANSIENT);
            CommandPool::create(ctx, &info)
        };

        let command_buffer =
            { pool.allocate_command_buffer(ctx, &vk::CommandBufferAllocateInfo::default()) };

        let begin_info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        unsafe {
            ctx.begin_command_buffer(command_buffer, &begin_info)
                .expect("Failed to begin recording setup commands");
        }

        Self {
            queue: **queue,
            pool,
            command_buffer,
            temporaries: vec![],
        }
    }

    pub fn add_for_destruction(&mut self, resource: impl Destroy<Context> + 'static) {
        self.temporaries.push(Box::new(resource));
    }

    pub fn finish(mut self, ctx: &mut Context) {
        let command_buffers = [self.command_buffer];
        let submit_info = [vk::SubmitInfo::builder()
            .command_buffers(&command_buffers)
            .build()];

        unsafe {
            ctx.end_command_buffer(self.command_buffer)
                .expect("Failed to end recording setup commands");

            ctx.queue_submit(self.queue, &submit_info, vk::Fence::null())
                .expect("Failed to submit setup commands to `transfer` queue");

            ctx.queue_wait_idle(self.queue)
                .expect("Failed to wait for transfer queue to idle");
        }

        unsafe { self.destroy_with(ctx) };
    }
}

impl Destroy<Context> for CommandBuilder {
    unsafe fn destroy_with(&mut self, ctx: &mut Context) {
        for temporary in &mut self.temporaries {
            temporary.destroy_with(ctx);
        }
        self.pool.destroy_with(&mut ctx.device);
    }
}
