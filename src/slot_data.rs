use crate::{error::*, node_graph::*, slot_image::SlotImage};
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};
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

impl From<(u32, u32)> for Size {
    fn from(other: (u32, u32)) -> Self {
        Size::new(other.0, other.1)
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

#[derive(Clone, Debug)]
pub struct SlotData {
    pub node_id: NodeId,
    pub slot_id: SlotId,
    pub image: SlotImage,
}

impl Display for SlotData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "NodeId: {}, SlotId: {}, Size: {}",
            self.node_id,
            self.slot_id,
            self.size().unwrap(),
        )
    }
}

impl SlotData {
    pub fn new(node_id: NodeId, slot_id: SlotId, image: SlotImage) -> Self {
        Self {
            node_id,
            slot_id,
            image,
        }
    }

    pub fn from_self(&self) -> Self {
        Self::new(self.node_id, self.slot_id, self.image.from_self())
    }

    pub fn size(&self) -> Result<Size> {
        self.image.size()
    }

    pub fn in_memory(&self) -> Result<bool> {
        for tbc in self.image.bufs().iter() {
            if !tbc.transient_buffer_sneaky().read()?.in_memory() {
                return Ok(false);
            }
        }

        Ok(true)
    }
}

pub trait SrgbColorSpace {
    fn linear_to_srgb(self) -> f32;
    fn srgb_to_linear(self) -> f32;
}

// source: https://entropymine.com/imageworsener/srgbformula/
impl SrgbColorSpace for f32 {
    fn linear_to_srgb(self) -> f32 {
        if self <= 0.0 {
            return self;
        }

        if self <= 0.0031308 {
            self * 12.92 // linear falloff in dark values
        } else {
            (1.055 * self.powf(1.0 / 2.4)) - 0.055 // gamma curve in other area
        }
    }

    fn srgb_to_linear(self) -> f32 {
        if self <= 0.0 {
            return self;
        }
        if self <= 0.04045 {
            self / 12.92 // linear falloff in dark values
        } else {
            ((self + 0.055) / 1.055).powf(2.4) // gamma curve in other area
        }
    }
}
