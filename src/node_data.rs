use crate::{
    node::BufferEnum,
    node_graph::*,
};
use image::{ImageBuffer, Luma};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct NodeData {
    pub size: Size,
    pub slot_id: SlotId,
    pub node_id: NodeId,
    pub buffer: BufferEnum,
}

pub type ChannelPixel = f32;
pub type Buffer = Box<ImageBuffer<Luma<ChannelPixel>, Vec<ChannelPixel>>>;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

impl Size {
    pub fn new(width: u32, height: u32) -> Self {
        Size { width, height }
    }

    pub fn pixel_count(self) -> u32 {
        self.width * self.height
    }
}

// impl PartialEq for NodeData {
//     fn eq(&self, other: &Self) -> bool {
//         if self.size != other.size
//             || !self.buffer.eq_enum(&other.buffer) {
//             return false
//         }

//         const ERROR_MARGIN = .0001;
//         self.buffers()
//             .iter()
//             .flat_map(|b| b.pixels())
//             .zip(
//                 other.buffers()
//                 .iter()
//                 .flat_map(|b| b.pixels())
//             )
//             .all(|(p1, p2)| (p1 - p2).abs() < ERROR_MARGIN)

//     //     for buffer in self.buffers()
//     //         .buffers()
//     //         .pixels()
//     //         .zip(other.buffer.pixels())
//     //         .all(|(p1, p2)| p1 == p2)
//     }
// }

impl NodeData {
    pub fn new(node_id: NodeId, slot_id: SlotId, size: Size, buffer: BufferEnum) -> Self {
        Self {
            node_id,
            slot_id,
            size,
            buffer,
        }
    }

    pub fn resized(&self, node_id: NodeId, slot_id: SlotId, size: Size, filter_type: FilterType) -> Self {
        imageops::resize(
            &node_data.buffer,
            size.width,
            size.height,
            filter_type.into(),
        )

        NodeData::new(node_id, slot_id, size, buffer_enum)
    }

    pub fn width(&self) -> u32 {
        self.size.width
    }

    pub fn height(&self) -> u32 {
        self.size.height
    }

    pub fn to_rgba_u8(&self) -> Vec<u8> {
        fn clamp_float(input: f32) -> f32 {
            if input < 0. {
                0.
            } else if input > 1. {
                1.
            } else {
                input
            }
        }

        let buffers = self.buffer.buffers_rgba();

        buffers[0]
            .pixels()
            .zip(buffers[1].pixels())
            .zip(buffers[2].pixels())
            .zip(buffers[3].pixels())
            .map(|(((r, g), b), a)| vec![r, g, b, a].into_iter())
            .flatten()
            .map(|x| (clamp_float(x[0]) * 255.).min(255.) as u8)
            .collect()
    }

    pub fn buffers_option(&self) -> Vec<Option<Arc<Buffer>>> {
        match self.buffer {
            BufferEnum::Gray(b) => vec![b.clone()],
            BufferEnum::Rgba(b) => vec![b.r.clone(), b.g.clone(), b.b.clone(), b.a.clone()],
        }
    }

    pub fn buffers(&self) -> Vec<Arc<Buffer>> {
        let (width, height) = (self.size.width, self.size.height);
        let size = (width * height) as usize;

        match self.buffer {
            BufferEnum::Gray(option_buffer) => {
                let buffer = if let Some(buffer) = option_buffer {
                    buffer
                } else {
                    Arc::new(Box::new(ImageBuffer::from_raw(width, height, vec![0.; size]).expect("Unable to create `ImageBuffer`")))
                };

                vec![
                Arc::clone(&buffer),
                Arc::clone(&buffer),
                Arc::clone(&buffer),
                Arc::new(Box::new(ImageBuffer::from_raw(width, height, vec![1.; size]).expect("Unable to create `ImageBuffer`")))
                ]
            }
            BufferEnum::Rgba(b) => {
                let mut output = Vec::new();

                for (i, buffer) in self.buffers_option().iter().enumerate() {
                    output.push(
                        if let Some(buffer) = buffer{
                            Arc::clone(buffer)
                        } else {
                            let value = if i == 3 { // If it's the alpha channel
                                1.
                            } else {
                                0.
                            };

                            Arc::new(Box::new(ImageBuffer::from_raw(width, height, vec![value; size]).expect("Unable to create `ImageBuffer`")))
                        }
                    )
                }

                output
            }
        }
    }

    // pub fn from_buffer(buffer: Buffer) -> Self {
    //     let (width, height) = buffer.dimensions();
    //     Self {
    //         size: Size::new(width, height),
    //         slot_id:
    // }

    // pub fn from_buffers(buffers: HashMap<Slot, Arc<Buffer>>) -> Self {
    //     if buffers.is_empty() {
    //         panic!("Attempted to create a `NodeData` with empty buffers.");
    //     }

    //     let (width, height) = buffers.values().next().unwrap().dimensions();
    //     for buffer in buffers.values() {
    //         if buffer.dimensions() != (width, height) {
    //             panic!("Attempted to create `NodeData` with differently sized buffers in slots.");
    //         }
    //     }

    //     Self {
    //         size: Size::new(width, height),
    //         buffers,
    //     }
    // }

    // pub fn get_buffers(&self) -> &HashMap<Slot, Arc<Buffer>> {
    //     &self.buffers
    // }

    // pub fn get_buffers_mut(&mut self) -> &mut HashMap<Slot, Arc<Buffer>> {
    //     &mut self.buffers
    // }

    // pub fn get_size(&self) -> Size {
    //     self.size
    // }
}
