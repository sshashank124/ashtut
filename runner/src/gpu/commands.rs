use ash::vk;

use super::{
    context::{queue::Queue, Context},
    Destroy,
};

#[allow(clippy::module_name_repetitions)]
pub type TempCommands = Template<false>;
pub type Commands = Template<true>;

pub struct Template<const MULTI_USE: bool> {
    queue: vk::Queue,
    pool: vk::CommandPool,
    pub buffer: vk::CommandBuffer,
}

impl<const MULTI_USE: bool> Template<{ MULTI_USE }> {
    pub fn create_on_queue(ctx: &Context, queue: &Queue) -> Self {
        let pool = {
            let transient_flag = if MULTI_USE {
                vk::CommandPoolCreateFlags::TRANSIENT
            } else {
                vk::CommandPoolCreateFlags::empty()
            };
            let info = vk::CommandPoolCreateInfo::builder()
                .queue_family_index(queue.family_index)
                .flags(transient_flag);
            unsafe {
                ctx.create_command_pool(&info, None)
                    .expect("Failed to create command pool")
            }
        };

        let buffer = unsafe {
            ctx.allocate_command_buffers(&vk::CommandBufferAllocateInfo {
                command_pool: pool,
                command_buffer_count: 1,
                ..Default::default()
            })
            .expect("Failed to allocate command buffer")[0]
        };

        Self {
            queue: **queue,
            pool,
            buffer,
        }
    }

    pub fn begin_recording(&self, ctx: &Context) {
        let one_time_submit_flag = if MULTI_USE {
            vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT
        } else {
            vk::CommandBufferUsageFlags::empty()
        };
        let begin_info = vk::CommandBufferBeginInfo::builder().flags(one_time_submit_flag);

        unsafe {
            ctx.begin_command_buffer(self.buffer, &begin_info)
                .expect("Failed to begin recording commands");
        }
    }

    pub fn finish_recording(&self, ctx: &Context) {
        unsafe {
            ctx.end_command_buffer(self.buffer)
                .expect("Failed to end recording commands");
        }
    }

    fn submit_to_queue(
        &self,
        ctx: &Context,
        submit_info: &vk::SubmitInfo,
        fence: Option<vk::Fence>,
    ) {
        let command_buffers = [self.buffer];
        let submit_info = [vk::SubmitInfo {
            command_buffer_count: command_buffers.len() as _,
            p_command_buffers: command_buffers.as_ptr(),
            ..*submit_info
        }];

        unsafe {
            ctx.queue_submit(
                self.queue,
                &submit_info,
                fence.unwrap_or_else(vk::Fence::null),
            )
            .expect("Failed to submit commands to queue");

            if fence.is_none() {
                ctx.queue_wait_idle(self.queue)
                    .expect("Failed to wait for queue to idle");
            }
        }
    }
}

impl Template<false> {
    pub fn submit(&self, ctx: &Context) {
        self.submit_to_queue(ctx, &vk::SubmitInfo::default(), None);
    }
}

impl Template<true> {
    pub fn submit(&self, ctx: &Context, submit_info: &vk::SubmitInfo, fence: vk::Fence) {
        self.submit_to_queue(ctx, submit_info, Some(fence));
    }

    pub fn reset(&self, ctx: &Context) {
        unsafe {
            ctx.reset_command_pool(self.pool, vk::CommandPoolResetFlags::empty())
                .expect("Failed to reset command pool");
        }
    }
}

impl<const MULTI_USE: bool> Destroy<Context> for Template<{ MULTI_USE }> {
    unsafe fn destroy_with(&mut self, ctx: &mut Context) {
        ctx.destroy_command_pool(self.pool, None);
    }
}
