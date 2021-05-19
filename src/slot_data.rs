use crate::{node::EmbeddedNodeDataId, node_graph::*};
use image::{ImageBuffer, Luma};
use serde::{Deserialize, Serialize};
use std::{fmt, sync::Arc};

#[derive(Debug, Clone)]
pub struct SlotData {
    pub size: Size,
    pub slot_id: SlotId,
    pub node_id: NodeId,
    pub buffer: Arc<Buffer>,
}
#[derive(Debug, Clone)]
pub struct EmbeddedNodeData {
    pub size: Size,
    pub slot_id: SlotId,
    pub node_data_id: EmbeddedNodeDataId,
    pub buffer: Arc<Buffer>,
}

impl EmbeddedNodeData {
    pub fn from_node_data(node_data: Arc<SlotData>, node_data_id: EmbeddedNodeDataId) -> Self {
        Self {
            size: node_data.size,
            buffer: Arc::clone(&node_data.buffer),
            node_data_id,
            slot_id: node_data.slot_id,
        }
    }
}

pub type Buffer = Box<ImageBuffer<Luma<ChannelPixel>, Vec<ChannelPixel>>>;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

impl fmt::Display for Size {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}x{}", self.width, self.height)
    }
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

impl PartialEq for SlotData {
    fn eq(&self, other: &Self) -> bool {
        self.size == other.size
            && self
                .buffer
                .pixels()
                .zip(other.buffer.pixels())
                .all(|(p1, p2)| p1 == p2)
    }
}

impl Eq for SlotData {}

impl SlotData {
    pub fn new(node_id: NodeId, slot_id: SlotId, size: Size, buffer: Arc<Buffer>) -> Self {
        Self {
            node_id,
            slot_id,
            size,
            buffer,
        }
    }
}
