use std::slice;

use ash::vk;

use crate::{
    context::{queue::Queue, Context},
    Destroy,
};

pub struct Commands {
    queue: vk::Queue,
    pool: vk::CommandPool,
    pub buffer: vk::CommandBuffer,
}

impl Commands {
    pub fn begin_on_queue(ctx: &Context, name: impl AsRef<str>, queue: &Queue) -> Self {
        let c = Self::create_on_queue(ctx, name, queue);
        c.begin_recording(ctx);
        c
    }

    pub fn create_on_queue(ctx: &Context, name: impl AsRef<str>, queue: &Queue) -> Self {
        let name = String::from(name.as_ref());
        let pool = {
            let info = vk::CommandPoolCreateInfo::default().queue_family_index(queue.family_index);
            unsafe {
                ctx.create_command_pool(&info, None)
                    .expect("Failed to create command pool")
            }
        };
        ctx.set_debug_name(pool, name.clone() + " - Command Pool");

        let buffer = unsafe {
            ctx.allocate_command_buffers(&vk::CommandBufferAllocateInfo {
                command_pool: pool,
                command_buffer_count: 1,
                ..Default::default()
            })
            .expect("Failed to allocate command buffer")[0]
        };
        ctx.set_debug_name(buffer, name + " - Command Buffer");

        Self {
            queue: **queue,
            pool,
            buffer,
        }
    }

    fn begin_recording(&self, ctx: &Context) {
        let begin_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        unsafe {
            ctx.begin_command_buffer(self.buffer, &begin_info)
                .expect("Failed to begin recording commands");
        }
    }

    fn finish_recording(&self, ctx: &Context) {
        unsafe {
            ctx.end_command_buffer(self.buffer)
                .expect("Failed to end recording commands");
        }
    }

    pub fn submit(&self, ctx: &Context, submit_info: &vk::SubmitInfo, fence: Option<vk::Fence>) {
        let submit_info = vk::SubmitInfo {
            command_buffer_count: 1,
            p_command_buffers: slice::from_ref(&self.buffer).as_ptr(),
            ..*submit_info
        };

        unsafe {
            self.finish_recording(ctx);

            ctx.queue_submit(
                self.queue,
                slice::from_ref(&submit_info),
                fence.unwrap_or_else(vk::Fence::null),
            )
            .expect("Failed to submit commands to queue");

            if fence.is_none() {
                ctx.queue_wait_idle(self.queue)
                    .expect("Failed to wait for queue to idle");
            }
        }
    }

    pub fn finish(mut self, ctx: &Context, submit_info: &vk::SubmitInfo, fence: Option<vk::Fence>) {
        self.submit(ctx, submit_info, fence);
        unsafe { self.destroy_with(ctx) };
    }

    pub fn reset(&self, ctx: &Context) {
        unsafe {
            ctx.reset_command_pool(self.pool, vk::CommandPoolResetFlags::empty())
                .expect("Failed to reset command pool");
        }
    }

    pub fn restart(&self, ctx: &Context) -> &Self {
        self.reset(ctx);
        self.begin_recording(ctx);
        self
    }

    pub fn flush(&self, ctx: &Context) -> &Self {
        self.submit(ctx, &vk::SubmitInfo::default(), None);
        self.restart(ctx)
    }
}

impl Destroy<Context> for Commands {
    unsafe fn destroy_with(&mut self, ctx: &Context) {
        ctx.destroy_command_pool(self.pool, None);
    }
}
