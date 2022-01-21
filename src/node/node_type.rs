use crate::{
    edge::Edge, error::Result, node_graph::*, shared::resize_buffers, slot_data::SlotData,
    texture_processor::TextureProcessor,
};
use serde::{Deserialize, Serialize};
use std::{
    fmt, mem,
    path::PathBuf,
    sync::{atomic::Ordering, Arc},
};

use super::{
    embed::{EmbeddedSlotData, EmbeddedSlotDataId},
    mix::MixType,
    Node, SlotInput, SlotOutput, SlotType, *,
};
#[derive(Deserialize, Serialize, Clone)]
pub enum NodeType {
    InputGray(String),
    InputRgba(String),
    OutputGray(String),
    OutputRgba(String),
    Graph(NodeGraph),
    Image(PathBuf),
    Embed(EmbeddedSlotDataId), // Maybe `Image` can handle both embedded and external images?
    Write(PathBuf),            // Probably remove this type, leave saving to application.
    Value(f32),
    Mix(MixType),
    HeightToNormal,
    SeparateRgba,
    CombineRgba,
}

impl fmt::Debug for NodeType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::InputGray(name) => write!(f, "InputGray: {}", name),
            Self::InputRgba(name) => write!(f, "InputRgba: {}", name),
            Self::OutputGray(name) => write!(f, "OutputGray: {}", name),
            Self::OutputRgba(name) => write!(f, "OutputRgba: {}", name),
            Self::Graph(_) => write!(f, "Graph"),
            Self::Image(_) => write!(f, "Image"),
            Self::Embed(_) => write!(f, "NodeData"),
            Self::Write(_) => write!(f, "Write"),
            Self::Value(value) => write!(f, "Value: {}", value),
            Self::Mix(_) => write!(f, "Mix"),
            Self::HeightToNormal => write!(f, "HeightToNormal"),
            Self::SeparateRgba => write!(f, "SeparateRgba"),
            Self::CombineRgba => write!(f, "CombineRgba"),
        }
    }
}

impl PartialEq for NodeType {
    fn eq(&self, other: &Self) -> bool {
        mem::discriminant(self) == mem::discriminant(other)
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

fn process_node_internal(
    node: Node,
    slot_datas: &[Arc<SlotData>],
    embedded_slot_datas: &[Arc<EmbeddedSlotData>],
    input_slot_datas: &[Arc<SlotData>],
    tex_pro: &Arc<TextureProcessor>,
) -> Result<Vec<Arc<SlotData>>> {
    let shutdown = Arc::clone(&tex_pro.shutdown);

    let output = match node.node_type {
        NodeType::InputRgba(_) => input_rgba::process(&node, input_slot_datas),
        NodeType::InputGray(_) => input_gray::process(&node, input_slot_datas),
        NodeType::OutputRgba(_) | NodeType::OutputGray(_) => output::process(slot_datas, &node),
        NodeType::Graph(ref node_graph) => graph::process(slot_datas, &node, node_graph, tex_pro)?,
        NodeType::Image(ref path) => read::process(&node, path)?,
        NodeType::Embed(embedded_node_data_id) => {
            embed::process(&node, embedded_slot_datas, embedded_node_data_id)?
        }
        NodeType::Write(ref path) => write::process(slot_datas, path)?,
        NodeType::Value(val) => value::process(&node, val),
        NodeType::Mix(mix_type) => mix::process(slot_datas, &node, mix_type)?,
        NodeType::HeightToNormal => height_to_normal::process(shutdown, slot_datas, &node)?,
        NodeType::SeparateRgba => separate_rgba::process(slot_datas, &node)?,
        NodeType::CombineRgba => combine_rgba::process(slot_datas, &node)?,
    };

    if !matches!(
        node.node_type,
        NodeType::OutputGray(..) | NodeType::OutputRgba(..)
    ) && output.len() != node.output_slots().len()
    {
        println!(
            "ERROR: the number of output buffers {} does not match the number of output slots {}",
            output.len(),
            node.output_slots().len()
        );
        Err(TexProError::InvalidBufferCount)
    } else {
        Ok(output)
    }
}

impl Node {
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
            NodeType::Embed(_) => Vec::new(),
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
            NodeType::Embed(_) => {
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
}

pub(crate) fn process_node(
    node: Node,
    slot_datas: &[Arc<SlotData>],
    embedded_slot_datas: &[Arc<EmbeddedSlotData>],
    input_slot_datas: &[Arc<SlotData>],
    edges: &[Edge],
    tex_pro: Arc<TextureProcessor>,
) -> Result<Vec<Arc<SlotData>>> {
    assert_eq!(
        edges.len(),
        slot_datas.len(),
        "NodeType: {:?}",
        node.node_type
    );

    // Slot datas resized, sorted by input `SlotId` and given the `SlotId` they belong in.
    let slot_datas = {
        let mut edges = edges.to_vec();
        edges.sort_unstable_by(|a, b| a.input_slot.cmp(&b.input_slot));

        let slot_datas: Vec<Arc<SlotData>> =
            resize_buffers(slot_datas, &edges, node.resize_policy, node.resize_filter)?;

        assign_slot_ids(&slot_datas, &edges)
    };

    let output = process_node_internal(
        node,
        &slot_datas,
        embedded_slot_datas,
        input_slot_datas,
        &tex_pro,
    )?;

    Ok(output)
}

fn assign_slot_ids(slot_datas: &[Arc<SlotData>], edges: &[Edge]) -> Vec<Arc<SlotData>> {
    edges
        .iter()
        .map(|edge| {
            let slot_data = slot_datas
                .iter()
                .find(|slot_data| {
                    edge.output_slot == slot_data.slot_id && edge.output_id == slot_data.node_id
                })
                .unwrap();
            Arc::new(SlotData::new(
                edge.input_id,
                edge.input_slot,
                slot_data.image.clone(),
            ))
        })
        .collect::<Vec<Arc<SlotData>>>()
}
