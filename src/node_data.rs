use crate::{
    node::Slot,
    node_graph::*,
};
use image::{ImageBuffer, Luma};
use std::{
    collections::HashMap,
    sync::Arc,
};

#[derive(Debug)]
pub struct NodeData {
    pub size: Size,
    pub slot_id: Slot,
    pub node_id: NodeId,
    pub buffers: HashMap<Slot, Buffer>,
}

pub type Buffer = ImageBuffer<Luma<ChannelPixel>, Vec<ChannelPixel>>;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Size {
    width: u32,
    height: u32,
}

impl Size {
    pub fn new(width: u32, height: u32) -> Self {
        Size { width, height }
    }

    pub fn pixel_count(self) -> u32 {
        self.width * self.height
    }

    pub fn width(self) -> u32 {
        self.width
    }

    pub fn height(self) -> u32 {
        self.height
    }
}

pub type ChannelPixel = f32;


impl NodeData {
    pub fn new(node_id: NodeId, slot_id: SlotId, size: Size) -> Self {
        Self {
            node_id,
            slot_id, 
            size,
            buffers: HashMap::new(),
        }
    }

    pub fn from_buffers(buffers: HashMap<Slot, Arc<Buffer>>) -> Self {
        if buffers.is_empty() {
            panic!("Attempted to create a `NodeData` with empty buffers.");
        }

        let (width, height) = buffers.values().next().unwrap().dimensions();
        for buffer in buffers.values() {
            if buffer.dimensions() != (width, height) {
                panic!("Attempted to create `NodeData` with differently sized buffers in slots.");
            }
        }

        Self {
            size: Size::new(width, height),
            buffers,
        }
    }

    pub fn get_buffers(&self) -> &HashMap<Slot, Arc<Buffer>> {
        &self.buffers
    }

    pub fn get_buffers_mut(&mut self) -> &mut HashMap<Slot, Arc<Buffer>> {
        &mut self.buffers
    }

    pub fn get_size(&self) -> Size {
        self.size
    }
}