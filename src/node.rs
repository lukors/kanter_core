use crate::error::Result;
use image::{FilterType, ImageBuffer, Luma};
use std::{collections::HashMap, path::Path, sync::Arc};

use crate::{
    dag::*,
    node_graph::*,
    node_data::*,
    shared::*,
};

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ResizePolicy {
    MostPixels,
    LeastPixels,
    LargestAxes,
    SmallestAxes,
    SpecificNode(NodeId),
    SpecificSize(Size),
}

#[derive(Clone, Copy, Debug, PartialEq, Ord, PartialOrd, Eq, Hash)]
pub struct Slot(pub usize);

impl Slot {
    fn as_usize(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy)]
pub enum Side {
    Input,
    Output,
}

pub enum NodeType {
    Input,
    Output,
    Graph(NodeGraph),
    Read(String),
    Write(String),
    Invert,
    Add,
    Multiply,
}

impl PartialEq for NodeType {
    fn eq(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

pub struct Node {
    pub node_type: NodeType,
    pub resize_policy: Option<ResizePolicy>,
    pub filter_type: Option<FilterType>,
}

impl Node {
    pub fn new(node_type: NodeType) -> Self {
        Self {
            node_type,
            resize_policy: None,
            filter_type: None,
        }
    }

    pub fn capacity(&self, side: Side) -> usize {
        match side {
            Side::Input => match self.node_type {
                NodeType::Input => 0,
                NodeType::Output => 4,
                NodeType::Graph(ref graph) => graph.input_count(),
                NodeType::Read(_) => 0,
                NodeType::Write(_) => 4,
                NodeType::Invert => 1,
                NodeType::Add => 2,
                NodeType::Multiply => 2,
            },
            Side::Output => match self.node_type {
                NodeType::Input => 4,
                NodeType::Output => 4,
                NodeType::Graph(ref graph) => graph.output_count(),
                NodeType::Read(_) => 4,
                NodeType::Write(_) => 0,
                NodeType::Invert => 1,
                NodeType::Add => 1,
                NodeType::Multiply => 1,
            },
        }
    }
}
