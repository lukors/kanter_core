use crate::{node_data::*, node_graph::*};
use image::FilterType;
use serde::{Deserialize, Serialize};
use std::fmt;

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

#[derive(Debug, Copy, Clone, PartialEq, Deserialize, Serialize)]
pub enum ResizeFilter {
    Nearest,
    Triangle,
    CatmullRom,
    Gaussian,
    Lanczos3,
}

impl From<FilterType> for ResizeFilter {
    fn from(filter_type: FilterType) -> Self {
        match filter_type {
            FilterType::Nearest => ResizeFilter::Nearest,
            FilterType::Triangle => ResizeFilter::Triangle,
            FilterType::CatmullRom => ResizeFilter::CatmullRom,
            FilterType::Gaussian => ResizeFilter::Gaussian,
            FilterType::Lanczos3 => ResizeFilter::Lanczos3,
        }
    }
}

impl Into<FilterType> for ResizeFilter {
    fn into(self) -> FilterType {
        match self {
            ResizeFilter::Nearest => FilterType::Nearest,
            ResizeFilter::Triangle => FilterType::Triangle,
            ResizeFilter::CatmullRom => FilterType::CatmullRom,
            ResizeFilter::Gaussian => FilterType::Gaussian,
            ResizeFilter::Lanczos3 => FilterType::Lanczos3,
        }
    }
}

#[derive(Clone, Copy)]
pub enum Side {
    Input,
    Output,
}

#[derive(Deserialize, Serialize)]
pub enum NodeType {
    InputGray,
    InputRgba,
    OutputGray,
    OutputRgba,
    Graph(NodeGraph),
    Read(String),
    Write(String),
    Value(f32),
    Resize(Option<ResizePolicy>, Option<ResizeFilter>),
    Add,
    Subtract,
    Invert,
    Multiply,
    HeightToNormal,
}

impl PartialEq for NodeType {
    fn eq(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

impl fmt::Debug for NodeType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // use NodeType::*;
        match self {
            NodeType::InputGray => write!(f, "InputGray"),
            NodeType::InputRgba => write!(f, "InputRgba"),
            NodeType::OutputGray => write!(f, "OutputGray"),
            NodeType::OutputRgba => write!(f, "OutputRgba"),
            NodeType::Graph(_) => write!(f, "Graph"),
            NodeType::Read(_) => write!(f, "Read"),
            NodeType::Write(_) => write!(f, "Write"),
            NodeType::Value(_) => write!(f, "Value"),
            NodeType::Resize(_, _) => write!(f, "Resize"),
            NodeType::Add => write!(f, "Add"),
            NodeType::Subtract => write!(f, "Subtract"),
            NodeType::Multiply => write!(f, "Multiply"),
            NodeType::HeightToNormal => write!(f, "HeightToNormal"),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Node {
    pub node_id: NodeId,
    pub node_type: NodeType,
    pub resize_policy: Option<ResizePolicy>,
    pub filter_type: Option<ResizeFilter>,
    pub physical_size: Option<PhysicalSize>,
}

impl Node {
    pub fn new(node_type: NodeType) -> Self {
        Self {
            node_id: NodeId(0),
            node_type,
            resize_policy: None,
            filter_type: None,
            physical_size: None,
        }
    }

    pub fn capacity(&self, side: Side) -> usize {
        match side {
            Side::Input => match self.node_type {
                NodeType::InputGray => 1,
                NodeType::InputRgba => 0,
                NodeType::OutputGray => 1,
                NodeType::OutputRgba => 4,
                NodeType::Graph(ref graph) => graph.input_count(),
                NodeType::Read(_) => 0,
                NodeType::Write(_) => 4,
                NodeType::Value(_) => 0,
                NodeType::Resize(_, _) => 2,
                NodeType::Add => 2,
                NodeType::Subtract => 2,
                NodeType::Multiply => 2,
                NodeType::HeightToNormal => 1,
            },
            Side::Output => match self.node_type {
                NodeType::InputGray => 1,
                NodeType::InputRgba => 4,
                NodeType::OutputGray => 1,
                NodeType::OutputRgba => 4,
                NodeType::Graph(ref graph) => graph.output_count(),
                NodeType::Read(_) => 4,
                NodeType::Write(_) => 0,
                NodeType::Value(_) => 1,
                NodeType::Resize(_, _) => 2,
                NodeType::Add => 1,
                NodeType::Subtract => 1,
                NodeType::Multiply => 1,
                NodeType::HeightToNormal => 3,
            },
        }
    }
}
