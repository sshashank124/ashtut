use ash::vk;

use crate::{context::Context, util::Destroy, wrapper::command_pool::CommandPool};

pub struct Setup {
    transfer_pool: CommandPool,
    pub transfer_commands: vk::CommandBuffer,
}

impl Setup {
    pub fn init(ctx: &Context) -> Self {
        let transfer_pool = {
            let info = vk::CommandPoolCreateInfo::builder()
                .queue_family_index(ctx.queues.transfer().family_index)
                .flags(vk::CommandPoolCreateFlags::TRANSIENT);
            CommandPool::create(ctx, &info)
        };

        let transfer_commands = {
            transfer_pool.allocate_command_buffer(ctx, &vk::CommandBufferAllocateInfo::default())
        };

        let begin_info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        unsafe {
            ctx.begin_command_buffer(transfer_commands, &begin_info)
                .expect("Failed to begin recording setup commands");
        }

        Self {
            transfer_pool,
            transfer_commands,
        }
    }

    pub fn finish(mut self, ctx: &Context) {
        let command_buffers = [self.transfer_commands];
        let submit_info = [vk::SubmitInfo::builder()
            .command_buffers(&command_buffers)
            .build()];

        unsafe {
            ctx.end_command_buffer(self.transfer_commands)
                .expect("Failed to end recording setup commands");

            ctx.queue_submit(**ctx.queues.transfer(), &submit_info, vk::Fence::null())
                .expect("Failed to submit setup commands to `transfer` queue");

            ctx.queue_wait_idle(**ctx.queues.transfer())
                .expect("Failed to wait for transfer queue to idle");
        }

        unsafe { self.destroy_with(ctx) };
    }
}

impl<'a> Destroy<&'a Context> for Setup {
    unsafe fn destroy_with(&mut self, ctx: &'a Context) {
        self.transfer_pool.destroy_with(ctx);
    }
}
