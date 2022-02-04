pub mod combine_rgba;
pub mod embed;
pub mod graph;
pub mod height_to_normal;
pub mod input_gray;
pub mod input_rgba;
pub mod mix;
pub mod node_type;
pub mod output;
pub mod process_shared;
pub mod image;
pub mod separate_rgba;
pub mod value;
pub mod write;

use crate::{
    error::{Result, TexProError},
    node_graph::*,
    priority::Priority,
    slot_data::*,
    slot_image::Buffer,
    transient_buffer::{TransientBuffer, TransientBufferContainer},
};
use ::image::imageops::FilterType;
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    sync::{atomic::AtomicBool, Arc, RwLock},
};

use self::node_type::NodeType;

#[derive(Copy, Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum ResizePolicy {
    MostPixels,
    LeastPixels,
    LargestAxes,
    SmallestAxes,
    SpecificSlot(SlotId),
    SpecificSize(Size),
}

impl Default for ResizePolicy {
    fn default() -> Self {
        ResizePolicy::MostPixels
    }
}

impl fmt::Display for ResizePolicy {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::MostPixels => write!(f, "Most pixels"),
            Self::LeastPixels => write!(f, "Least Pixels"),
            Self::LargestAxes => write!(f, "Largest Axes"),
            Self::SmallestAxes => write!(f, "Smallest Axes"),
            Self::SpecificSlot(i) => write!(f, "Slot: {}", i),
            Self::SpecificSize(i) => write!(f, "Size: {}", i),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Deserialize, Serialize)]
pub enum ResizeFilter {
    Nearest,
    Triangle,
    CatmullRom,
    Gaussian,
    Lanczos3,
}

impl Default for ResizeFilter {
    fn default() -> Self {
        Self::Triangle
    }
}

impl fmt::Display for ResizeFilter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Nearest => write!(f, "Nearest neighbour"),
            Self::Triangle => write!(f, "Triangle"),
            Self::CatmullRom => write!(f, "CatmullRom"),
            Self::Gaussian => write!(f, "Gaussian"),
            Self::Lanczos3 => write!(f, "Lanczos3"),
        }
    }
}

impl From<ResizeFilter> for FilterType {
    fn from(resize_filter: ResizeFilter) -> FilterType {
        match resize_filter {
            ResizeFilter::Nearest => Self::Nearest,
            ResizeFilter::Triangle => Self::Triangle,
            ResizeFilter::CatmullRom => Self::CatmullRom,
            ResizeFilter::Gaussian => Self::Gaussian,
            ResizeFilter::Lanczos3 => Self::Lanczos3,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Side {
    Input,
    Output,
}

impl Default for Side {
    fn default() -> Self {
        Self::Input
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Node {
    pub node_id: NodeId,
    pub node_type: NodeType,
    pub resize_policy: ResizePolicy,
    pub resize_filter: ResizeFilter,
    #[serde(skip)]
    pub priority: Arc<Priority>,
    #[serde(skip)]
    pub cancel: Arc<AtomicBool>,
}

impl Node {
    pub fn new(node_type: NodeType) -> Self {
        Self {
            node_id: NodeId(0),
            node_type,
            resize_policy: ResizePolicy::default(),
            resize_filter: ResizeFilter::default(),
            priority: Arc::new(Priority::new()),
            cancel: Arc::new(false.into()),
        }
    }

    pub fn with_id(node_type: NodeType, node_id: NodeId) -> Self {
        Self {
            node_id,
            node_type,
            resize_policy: ResizePolicy::default(),
            resize_filter: ResizeFilter::default(),
            priority: Arc::new(Priority::new()),
            cancel: Arc::new(false.into()),
        }
    }

    pub fn node_id(mut self, node_id: NodeId) -> Self {
        self.node_id = node_id;
        self
    }

    pub fn resize_policy(mut self, resize_policy: ResizePolicy) -> Self {
        self.resize_policy = resize_policy;
        self
    }

    pub fn resize_filter(mut self, resize_filter: ResizeFilter) -> Self {
        self.resize_filter = resize_filter;
        self
    }

    pub fn input_slot_with_id(&self, slot_id: SlotId) -> Result<Slot> {
        self.input_slots()
            .into_iter()
            .find(|slot| slot.slot_id == slot_id)
            .ok_or(TexProError::InvalidSlotId)
    }

    pub fn output_slot_with_id(&self, slot_id: SlotId) -> Result<Slot> {
        self.output_slots()
            .into_iter()
            .find(|slot| slot.slot_id == slot_id)
            .ok_or(TexProError::InvalidSlotId)
    }

    pub fn input_slot_with_name(&self, name: String) -> Result<Slot> {
        self.input_slots()
            .into_iter()
            .find(|slot| slot.name == name)
            .ok_or(TexProError::InvalidName)
    }

    pub fn output_slot_with_name(&self, name: String) -> Result<Slot> {
        self.output_slots()
            .into_iter()
            .find(|slot| slot.name == name)
            .ok_or(TexProError::InvalidName)
    }

    pub fn filter_type(&mut self, rf: ResizeFilter) {
        self.resize_filter = rf;
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SlotType {
    Gray,
    Rgba,
    GrayOrRgba,
}

impl Default for SlotType {
    fn default() -> Self {
        Self::GrayOrRgba
    }
}

impl SlotType {
    pub fn fits(&self, other: Self) -> Result<()> {
        if match self {
            Self::Gray => other == Self::Gray || other == Self::GrayOrRgba,
            Self::Rgba => other == Self::Rgba || other == Self::GrayOrRgba,
            Self::GrayOrRgba => true,
        } {
            Ok(())
        } else {
            Err(TexProError::InvalidSlotType)
        }
    }
}

#[derive(Clone, Debug)]
pub struct Slot {
    pub name: String,
    pub slot_id: SlotId,
    pub slot_type: SlotType,
}

impl Slot {
    pub fn new(name: String, slot_id: SlotId, slot_type: SlotType) -> Self {
        Self {
            name,
            slot_id,
            slot_type,
        }
    }
}

fn pixel_buffer(value: ChannelPixel) -> Arc<TransientBufferContainer> {
    Arc::new(TransientBufferContainer::new(Arc::new(RwLock::new(
        TransientBuffer::new(Box::new(Buffer::from_raw(1, 1, vec![value]).unwrap())),
    ))))
}

pub(crate) type SlotInput = Slot;
pub(crate) type SlotOutput = Slot;
