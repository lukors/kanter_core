use crate::{node_data::*, node_graph::*};
use image::FilterType;
use std::fmt;

#[derive(Copy, Clone, Debug, PartialEq)]
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

#[derive(Clone, Copy)]
pub enum Side {
    Input,
    Output,
}

pub enum NodeType {
    InputGray,
    InputRgba,
    OutputGray,
    OutputRgba,
    Graph(NodeGraph),
    Read(String),
    Write(String),
    Value(f32),
    Resize(Option<ResizePolicy>, Option<FilterType>),
    Add,
    Invert,
    Multiply,
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
            NodeType::Invert => write!(f, "Invert"),
            NodeType::Multiply => write!(f, "Multiply"),
        }
    }
}

pub struct Node {
    pub node_id: NodeId,
    pub node_type: NodeType,
    pub resize_policy: Option<ResizePolicy>,
    pub filter_type: Option<FilterType>,
}

impl Node {
    pub fn new(node_type: NodeType) -> Self {
        Self {
            node_id: NodeId(0),
            node_type,
            resize_policy: None,
            filter_type: None,
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
                NodeType::Invert => 1,
                NodeType::Multiply => 2,
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
                NodeType::Invert => 1,
                NodeType::Multiply => 1,
            },
        }
    }
}
