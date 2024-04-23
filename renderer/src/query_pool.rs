use std::ops::Deref;

use ash::vk;

use crate::{context::Context, Destroy};

pub struct QueryPool {
    pool: vk::QueryPool,
    count: u32,
}

impl QueryPool {
    pub fn create(ctx: &Context, name: String, q_type: vk::QueryType, count: u32) -> Self {
        firestorm::profile_method!(create);

        let info = vk::QueryPoolCreateInfo::default()
            .query_type(q_type)
            .query_count(count);

        let pool = unsafe {
            ctx.create_query_pool(&info, None)
                .expect("Failed to create query pool")
        };
        ctx.set_debug_name(pool, &(name + " - Query Pool"));

        Self { pool, count }
    }

    pub fn read<T: Clone + Default>(&self, ctx: &Context) -> Vec<T> {
        firestorm::profile_method!(read);

        let mut results = vec![T::default(); self.count as _];
        unsafe {
            ctx.get_query_pool_results(
                self.pool,
                0,
                results.as_mut_slice(),
                vk::QueryResultFlags::WAIT,
            )
            .expect("Failed to get query pool results");
        }
        results
    }

    pub fn reset(&self, ctx: &Context, command_buffer: vk::CommandBuffer) {
        firestorm::profile_method!(reset);

        unsafe { ctx.cmd_reset_query_pool(command_buffer, self.pool, 0, self.count) };
    }
}

impl Destroy<Context> for QueryPool {
    unsafe fn destroy_with(&mut self, ctx: &Context) {
        firestorm::profile_method!(destroy_with);

        ctx.destroy_query_pool(self.pool, None);
    }
}

impl Deref for QueryPool {
    type Target = vk::QueryPool;

    fn deref(&self) -> &Self::Target {
        &self.pool
    }
}
