use std::mem;

use ash::vk;

use shared::{bytemuck, Vertex};

use crate::util::Descriptions;

impl Descriptions for Vertex {
    type BindingType = vk::VertexInputBindingDescription;
    const NUM_BINDINGS: usize = 1;

    fn bindings_description() -> [Self::BindingType; Self::NUM_BINDINGS] {
        [vk::VertexInputBindingDescription {
            binding: 0,
            stride: mem::size_of::<Self>() as u32,
            input_rate: vk::VertexInputRate::VERTEX,
        }]
    }

    type AttributeType = vk::VertexInputAttributeDescription;
    const NUM_ATTRIBUTES: usize = 2;

    fn attributes_description() -> [Self::AttributeType; Self::NUM_ATTRIBUTES] {
        [
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 0,
                format: vk::Format::R32G32_SFLOAT,
                offset: bytemuck::offset_of!(Self, position) as u32,
            },
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 1,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: bytemuck::offset_of!(Self, color) as u32,
            },
        ]
    }
}
