use crate::{error::*, node::EmbeddedNodeDataId, node_graph::*};
use image::{ImageBuffer, Luma};
use serde::{Deserialize, Serialize};
use std::{fmt, mem, sync::Arc};

#[derive(Debug, Clone)]
pub enum SlotImage {
    Gray(Arc<BoxBuffer>),
    Rgba([Arc<BoxBuffer>; 4]),
}

impl PartialEq for SlotImage {
    fn eq(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

impl SlotImage {
    pub fn from_value(size: Size, value: ChannelPixel, rgba: bool) -> Self {
        if rgba {
            Self::Rgba([
                Arc::new(Box::new(
                    Buffer::from_raw(size.width, size.height, vec![value; size.pixel_count()])
                        .unwrap(),
                )),
                Arc::new(Box::new(
                    Buffer::from_raw(size.width, size.height, vec![value; size.pixel_count()])
                        .unwrap(),
                )),
                Arc::new(Box::new(
                    Buffer::from_raw(size.width, size.height, vec![value; size.pixel_count()])
                        .unwrap(),
                )),
                Arc::new(Box::new(
                    Buffer::from_raw(size.width, size.height, vec![1.0; size.pixel_count()])
                        .unwrap(),
                )),
            ])
        } else {
            Self::Gray(Arc::new(Box::new(
                Buffer::from_raw(size.width, size.height, vec![value; size.pixel_count()]).unwrap(),
            )))
        }
    }

    pub fn from_buffers_rgba(buffers: &mut [Buffer]) -> Result<Self> {
        if buffers.len() != 4 {
            return Err(TexProError::InvalidBufferCount);
        }

        let mut buffers = buffers.to_vec();
        buffers.reverse();
        
        Ok(Self::Rgba([
            Arc::new(Box::new(buffers.pop().unwrap())),
            Arc::new(Box::new(buffers.pop().unwrap())),
            Arc::new(Box::new(buffers.pop().unwrap())),
            Arc::new(Box::new(buffers.pop().unwrap())),
        ]))
    }

    pub fn from_buffers_rgb(buffers: &mut [Buffer]) -> Result<Self> {
        if buffers.len() != 3 {
            return Err(TexProError::InvalidBufferCount);
        }

        let (width, height) = (buffers[0].width(), buffers[0].height());
        let mut buffers = buffers.to_vec();

        buffers.push(Buffer::from_raw(width, height, vec![1.0; (width * height) as usize])
        .unwrap());
        
        Self::from_buffers_rgba(&mut buffers)
    }

    pub fn size(&self) -> Size {
        match self {
            Self::Gray(buf) => Size::new(buf.width(), buf.height()),
            Self::Rgba(bufs) => Size::new(bufs[0].width(), bufs[0].height()),
        }
    }

    pub fn is_rgba(&self) -> bool {
        mem::discriminant(self)
            == mem::discriminant(&Self::Rgba([
                Arc::new(Box::new(Buffer::from_raw(0, 0, Vec::new()).unwrap())),
                Arc::new(Box::new(Buffer::from_raw(0, 0, Vec::new()).unwrap())),
                Arc::new(Box::new(Buffer::from_raw(0, 0, Vec::new()).unwrap())),
                Arc::new(Box::new(Buffer::from_raw(0, 0, Vec::new()).unwrap())),
            ]))
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

    pub fn pixel_count(self) -> usize {
        (self.width * self.height) as usize
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

    pub fn from_value(size: Size, value: ChannelPixel, rgba: bool) -> Self {
        Self::new(
            NodeId(0),
            SlotId(0),
            size,
            Arc::new(SlotImage::from_value(size, value, rgba)),
        )
    }
}

impl SlotImage {
    pub fn to_rgba(&self) -> Vec<u8> {
        match self {
            Self::Gray(buf) => buf
                .pixels()
                .map(|x| {
                    let value = ((x[0].clamp(0.0, 1.0) * 255.).min(255.)) as u8;
                    vec![value, value, value, 255]
                })
                .flatten()
                .collect(),
            Self::Rgba(bufs) => bufs[0]
                .pixels()
                .zip(bufs[1].pixels())
                .zip(bufs[2].pixels())
                .zip(bufs[3].pixels())
                .map(|(((r, g), b), a)| vec![r, g, b, a].into_iter())
                .flatten()
                .map(|x| ((x[0].clamp(0.0, 1.0) * 255.).min(255.)) as u8)
                .collect(),
        }
    }
}
