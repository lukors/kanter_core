use crate::{node_graph::*, slot_data::*};
use image::FilterType;
use serde::{Deserialize, Serialize};
use std::{
    fmt::{self, Display},
    path::PathBuf,
};

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
pub struct EmbeddedNodeDataId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Side {
    Input,
    Output,
}

#[derive(Deserialize, Serialize, Clone)]
pub enum NodeType {
    InputGray,
    InputRgba,
    OutputGray,
    OutputRgba,
    Graph(NodeGraph),
    Image(PathBuf),
    NodeData(EmbeddedNodeDataId),
    Write(PathBuf),
    Value(f32),
    Mix(MixType),
    HeightToNormal,
}

#[derive(Deserialize, Serialize, Copy, Clone, PartialEq)]
pub enum MixType {
    Add,
    Subtract,
    Multiply,
    Divide,
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
            }
        )
    }
}

impl PartialEq for NodeType {
    fn eq(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

impl fmt::Debug for NodeType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            NodeType::InputGray => write!(f, "InputGray"),
            NodeType::InputRgba => write!(f, "InputRgba"),
            NodeType::OutputGray => write!(f, "OutputGray"),
            NodeType::OutputRgba => write!(f, "OutputRgba"),
            NodeType::Graph(_) => write!(f, "Graph"),
            NodeType::Image(_) => write!(f, "Image"),
            NodeType::NodeData(_) => write!(f, "NodeData"),
            NodeType::Write(_) => write!(f, "Write"),
            NodeType::Value(_) => write!(f, "Value"),
            NodeType::Mix(_) => write!(f, "Mix"),
            NodeType::HeightToNormal => write!(f, "HeightToNormal"),
        }
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

    pub fn capacity(&self, side: Side) -> usize {
        match side {
            Side::Input => match self.node_type {
                NodeType::InputGray => 1,
                NodeType::InputRgba => 0,
                NodeType::OutputGray => 1,
                NodeType::OutputRgba => 4,
                NodeType::Graph(ref graph) => graph.input_count(),
                NodeType::Image(_) => 0,
                NodeType::NodeData(_) => 0,
                NodeType::Write(_) => 4,
                NodeType::Value(_) => 0,
                NodeType::Mix(_) => 4,
                NodeType::HeightToNormal => 1,
            },
            Side::Output => match self.node_type {
                NodeType::InputGray => 1,
                NodeType::InputRgba => 4,
                NodeType::OutputGray => 1,
                NodeType::OutputRgba => 4,
                NodeType::Graph(ref graph) => graph.output_count(),
                NodeType::Image(_) => 4,
                NodeType::NodeData(_) => 1,
                NodeType::Write(_) => 0,
                NodeType::Value(_) => 1,
                NodeType::Mix(_) => 1,
                NodeType::HeightToNormal => 3,
            },
        }
    }

    pub fn slot_exists(&self, slot_id: SlotId, side: Side) -> bool {
        slot_id.0 < self.capacity(side) as u32
    }

    pub fn filter_type(&mut self, rf: ResizeFilter) {
        self.resize_filter = rf;
    }
}
