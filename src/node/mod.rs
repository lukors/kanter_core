pub mod node_type;
use crate::{
    error::{Result, TexProError},
    node_graph::*,
    slot_data::*,
};
use image::FilterType;
use serde::{Deserialize, Serialize};
use std::{
    fmt::{self, Display},
    mem,
    path::PathBuf,
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

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, PartialOrd, Serialize)]
pub struct EmbeddedSlotDataId(pub u32);

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

#[derive(Deserialize, Serialize, Copy, Clone, PartialEq)]
pub enum MixType {
    Add,
    Subtract,
    Multiply,
    Divide,
    Pow,
}

impl Default for MixType {
    fn default() -> Self {
        Self::Add
    }
}

impl Display for MixType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Add => "Add",
                Self::Subtract => "Subtract",
                Self::Multiply => "Multiply",
                Self::Divide => "Divide",
                Self::Pow => "Power",
            }
        )
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Node {
    pub node_id: NodeId,
    pub node_type: NodeType,
    pub resize_policy: ResizePolicy,
    pub resize_filter: ResizeFilter,
}

impl Node {
    pub fn new(node_type: NodeType) -> Self {
        Self {
            node_id: NodeId(0),
            node_type,
            resize_policy: ResizePolicy::default(),
            resize_filter: ResizeFilter::default(),
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

    pub fn input_slots(&self) -> Vec<SlotInput> {
        match self.node_type {
            NodeType::InputGray(_) => Vec::new(),
            NodeType::InputRgba(_) => Vec::new(),
            NodeType::OutputGray(_) => {
                vec![SlotInput::new("input".into(), SlotId(0), SlotType::Gray)]
            }
            NodeType::OutputRgba(_) => {
                vec![SlotInput::new("input".into(), SlotId(0), SlotType::Rgba)]
            }
            NodeType::Graph(ref graph) => graph.input_slots(),
            NodeType::Image(_) => Vec::new(),
            NodeType::Embedded(_) => Vec::new(),
            NodeType::Write(_) => unimplemented!(),
            NodeType::Value(_) => Vec::new(),
            NodeType::Mix(_) => vec![
                SlotInput::new("left".into(), SlotId(0), SlotType::GrayOrRgba),
                SlotInput::new("right".into(), SlotId(1), SlotType::GrayOrRgba),
            ],
            NodeType::HeightToNormal => {
                vec![SlotInput::new("input".into(), SlotId(0), SlotType::Gray)]
            }
            NodeType::SeparateRgba => {
                vec![SlotInput::new("input".into(), SlotId(0), SlotType::Rgba)]
            }
            NodeType::CombineRgba => vec![
                SlotInput::new("red".into(), SlotId(0), SlotType::Gray),
                SlotInput::new("green".into(), SlotId(1), SlotType::Gray),
                SlotInput::new("blue".into(), SlotId(2), SlotType::Gray),
                SlotInput::new("alpha".into(), SlotId(3), SlotType::Gray),
            ],
        }
    }

    pub fn output_slots(&self) -> Vec<SlotOutput> {
        match self.node_type {
            NodeType::InputGray(_) => {
                vec![SlotOutput::new("output".into(), SlotId(0), SlotType::Gray)]
            }
            NodeType::InputRgba(_) => {
                vec![SlotOutput::new("output".into(), SlotId(0), SlotType::Rgba)]
            }
            NodeType::OutputGray(_) => Vec::new(),
            NodeType::OutputRgba(_) => Vec::new(),
            NodeType::Graph(ref graph) => graph.output_slots(),
            NodeType::Image(_) => vec![SlotOutput::new("output".into(), SlotId(0), SlotType::Rgba)],
            NodeType::Embedded(_) => {
                vec![SlotOutput::new("output".into(), SlotId(0), SlotType::Rgba)]
            }
            NodeType::Write(_) => unimplemented!(),
            NodeType::Value(_) => vec![SlotOutput::new("output".into(), SlotId(0), SlotType::Gray)],
            NodeType::Mix(_) => vec![SlotOutput::new(
                "output".into(),
                SlotId(0),
                SlotType::GrayOrRgba,
            )],
            NodeType::HeightToNormal => {
                vec![SlotOutput::new("output".into(), SlotId(0), SlotType::Rgba)]
            }
            NodeType::SeparateRgba => vec![
                SlotOutput::new("red".into(), SlotId(0), SlotType::Gray),
                SlotOutput::new("green".into(), SlotId(1), SlotType::Gray),
                SlotOutput::new("blue".into(), SlotId(2), SlotType::Gray),
                SlotOutput::new("alpha".into(), SlotId(3), SlotType::Gray),
            ],
            NodeType::CombineRgba => {
                vec![SlotOutput::new("output".into(), SlotId(0), SlotType::Rgba)]
            }
        }
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

pub(crate) type SlotInput = Slot;
pub(crate) type SlotOutput = Slot;
