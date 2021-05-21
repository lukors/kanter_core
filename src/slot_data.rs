use crate::{node::EmbeddedNodeDataId, node_graph::*};
use image::{ImageBuffer, Luma};
use serde::{Deserialize, Serialize};
use std::{fmt, sync::Arc};

#[derive(Debug, Clone)]
pub enum SlotImage {
    Gray(Arc<BoxBuffer>),
    Rgba([Arc<BoxBuffer>; 4]),
}

impl SlotImage {
    pub fn size(&self) -> Size {
        match self {
            Self::Gray(buf) => Size::new(buf.width(), buf.height()),
            Self::Rgba(bufs) => Size::new(bufs[0].width(), bufs[0].height()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SlotData {
    pub node_id: NodeId,
    pub slot_id: SlotId,
    pub size: Size,
    pub image: Arc<SlotImage>,
}
#[derive(Debug, Clone)]
pub struct EmbeddedNodeData {
    pub node_data_id: EmbeddedNodeDataId,
    pub slot_id: SlotId,
    pub size: Size,
    pub image: Arc<SlotImage>,
}

impl EmbeddedNodeData {
    pub fn from_node_data(node_data: Arc<SlotData>, node_data_id: EmbeddedNodeDataId) -> Self {
        Self {
            node_data_id,
            slot_id: node_data.slot_id,
            size: node_data.size,
            image: Arc::clone(&node_data.image),
        }
    }
}

pub type Buffer = ImageBuffer<Luma<ChannelPixel>, Vec<ChannelPixel>>;
pub type BoxBuffer = Box<Buffer>;

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

// impl PartialEq for SlotData {
//     fn eq(&self, other: &Self) -> bool {
//         self.size == other.size
//             && self
//                 .data
//                 .pixels()
//                 .zip(other.data.pixels())
//                 .all(|(p1, p2)| p1 == p2)
//     }
// }

// impl Eq for SlotData {}

impl SlotData {
    pub fn new(node_id: NodeId, slot_id: SlotId, size: Size, image: Arc<SlotImage>) -> Self {
        Self {
            node_id,
            slot_id,
            size,
            image,
        }
    }
}

impl SlotImage {
    pub fn to_rgba(&self) -> Vec<u8> {
        fn clamp_float(input: f32) -> f32 {
            if input < 0. {
                0.
            } else if input > 1. {
                1.
            } else {
                input
            }
        }

        match self {
            Self::Gray(buf) => buf
                .pixels()
                .map(|x| (clamp_float(x[0]) * 255.).min(255.) as u8)
                .collect(),
            Self::Rgba(bufs) => bufs[0]
                .pixels()
                .zip(bufs[1].pixels())
                .zip(bufs[2].pixels())
                .zip(bufs[3].pixels())
                .map(|(((r, g), b), a)| vec![r, g, b, a].into_iter())
                .flatten()
                .map(|x| (clamp_float(x[0]) * 255.).min(255.) as u8)
                .collect(),
        }
    }
}
