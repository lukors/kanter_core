use crate::node_graph::*;
use image::{ImageBuffer, Luma};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct NodeData {
    pub size: Size,
    pub slot_id: SlotId,
    pub node_id: NodeId,
    pub buffer: Arc<Buffer>,
}

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

pub type ChannelPixel = f32;

impl PartialEq for NodeData {
    fn eq(&self, other: &Self) -> bool {
        self.size == other.size
            && self
                .buffer
                .pixels()
                .zip(other.buffer.pixels())
                .all(|(p1, p2)| p1 == p2)
    }
}

impl Eq for NodeData {}

impl NodeData {
    pub fn new(node_id: NodeId, slot_id: SlotId, size: Size, buffer: Arc<Buffer>) -> Self {
        Self {
            node_id,
            slot_id,
            size,
            buffer,
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
