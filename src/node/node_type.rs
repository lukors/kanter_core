use crate::{
    node_graph::*,
};
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    mem,
    path::PathBuf,
};

use super::{EmbeddedSlotDataId, MixType, SlotType};
#[derive(Deserialize, Serialize, Clone)]
pub enum NodeType {
    InputGray(String),
    InputRgba(String),
    OutputGray(String),
    OutputRgba(String),
    Graph(NodeGraph),
    Image(PathBuf),
    Embedded(EmbeddedSlotDataId), // Maybe `Image` can handle both embedded and external images?
    Write(PathBuf),               // Probably remove this type, leave saving to application.
    Value(f32),
    Mix(MixType),
    HeightToNormal,
    SeparateRgba,
    CombineRgba,
}

impl PartialEq for NodeType {
    fn eq(&self, other: &Self) -> bool {
        mem::discriminant(self) == mem::discriminant(other)
    }
}

impl fmt::Debug for NodeType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            NodeType::InputGray(name) => write!(f, "InputGray: {}", name),
            NodeType::InputRgba(name) => write!(f, "InputRgba: {}", name),
            NodeType::OutputGray(name) => write!(f, "OutputGray: {}", name),
            NodeType::OutputRgba(name) => write!(f, "OutputRgba: {}", name),
            NodeType::Graph(_) => write!(f, "Graph"),
            NodeType::Image(_) => write!(f, "Image"),
            NodeType::Embedded(_) => write!(f, "NodeData"),
            NodeType::Write(_) => write!(f, "Write"),
            NodeType::Value(value) => write!(f, "Value: {}", value),
            NodeType::Mix(_) => write!(f, "Mix"),
            NodeType::HeightToNormal => write!(f, "HeightToNormal"),
            NodeType::SeparateRgba => write!(f, "SeparateRgba"),
            NodeType::CombineRgba => write!(f, "CombineRgba"),
        }
    }
}

impl NodeType {
    pub fn is_input(&self) -> bool {
        *self == Self::InputGray(String::new()) || *self == Self::InputRgba(String::new())
    }

    pub fn is_output(&self) -> bool {
        *self == Self::OutputGray(String::new()) || *self == Self::OutputRgba(String::new())
    }

    pub fn name(&self) -> Option<&String> {
        if let Self::InputGray(name)
        | Self::InputRgba(name)
        | Self::OutputGray(name)
        | Self::OutputRgba(name) = self
        {
            Some(name)
        } else {
            None
        }
    }

    pub fn name_mut(&mut self) -> Option<&mut String> {
        if let Self::InputGray(name)
        | Self::InputRgba(name)
        | Self::OutputGray(name)
        | Self::OutputRgba(name) = self
        {
            Some(name)
        } else {
            None
        }
    }

    pub fn to_slot_type(&self) -> Option<SlotType> {
        match self {
            Self::InputGray(_) | Self::OutputGray(_) => Some(SlotType::Gray),
            Self::InputRgba(_) | Self::OutputRgba(_) => Some(SlotType::Rgba),
            _ => None,
        }
    }
}