use crate::{error::*, node::{MixType, Node, NodeType, Side, SlotInput, SlotOutput}};
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    fs::File,
    io::{self},
    mem,
    path::PathBuf,
};

#[derive(Clone, Default, Debug, Deserialize, Serialize)]
pub struct NodeGraph {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

impl NodeGraph {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }

    pub fn from_path(path: String) -> io::Result<Self> {
        Self::import_json(path)
    }

    pub fn set_mix_type(&mut self, node_id: NodeId, mix_type: MixType) -> Result<()> {
        if let Some(node_index) = self.index_of_node(node_id) {
            match self.nodes[node_index].node_type {
                NodeType::Mix(_) => {
                    let mut node_clone: Node = (self.nodes[node_index]).clone();
                    node_clone.node_type = NodeType::Mix(mix_type);

                    #[allow(unused_must_use)]
                    {
                        mem::replace(&mut self.nodes[node_index], node_clone);
                    }
                    Ok(())
                }
                _ => Err(TexProError::InvalidNodeId),
            }
        } else {
            Err(TexProError::InvalidNodeId)
        }
    }

    pub fn set_image_node_path(&mut self, node_id: NodeId, path: PathBuf) -> Result<()> {
        if let Some(node_index) = self.index_of_node(node_id) {
            match self.nodes[node_index].node_type {
                NodeType::Image(_) => {
                    let mut node_clone: Node = (self.nodes[node_index]).clone();
                    node_clone.node_type = NodeType::Image(path);

                    #[allow(unused_must_use)]
                    {
                        mem::replace(&mut self.nodes[node_index], node_clone);
                    }
                    Ok(())
                }
                _ => Err(TexProError::InvalidNodeId),
            }
        } else {
            Err(TexProError::InvalidNodeId)
        }
    }

    /// Generates a new unique NodeId.
    pub fn new_id(&mut self) -> NodeId {
        loop {
            let id = NodeId(rand::random());
            if self.has_node_with_id(id).is_err() {
                return id;
            }
        }
    }

    pub fn export_json(&self, path: String) -> io::Result<()> {
        let file = File::create(path)?;
        serde_json::to_writer_pretty(&file, self)?;
        Ok(())
    }

    fn import_json(path: String) -> io::Result<Self> {
        let file = File::open(path)?;
        Ok(serde_json::from_reader(file)?)
    }

    fn index_of_node(&self, node_id: NodeId) -> Option<usize> {
        self.nodes.iter().position(|node| node.node_id == node_id)
    }

    pub fn has_node_with_id(&self, node_id: NodeId) -> Result<()> {
        if self.nodes.iter().any(|node| node.node_id == node_id) {
            Ok(())
        } else {
            Err(TexProError::InvalidNodeId)
        }
    }

    pub fn nodes(&self) -> &Vec<Node> {
        &self.nodes
    }

    pub fn node_ids(&self) -> Vec<NodeId> {
        self.nodes.iter().map(|node| node.node_id).collect()
    }

    pub fn node_with_id(&self, node_id: NodeId) -> Result<Node> {
        self.nodes
            .iter()
            .find(|node| node.node_id == node_id)
            .cloned()
            .ok_or(TexProError::InvalidNodeId)
    }

    pub(crate) fn node_with_id_mut(&mut self, node_id: NodeId) -> Option<&mut Node> {
        self.nodes.iter_mut().find(|node| node.node_id == node_id)
    }

    fn add_node_internal(&mut self, mut node: Node, node_id: NodeId) -> Result<NodeId> {
        if let Some(name) = node.node_type.name() {
            if name.is_empty() {
                return Err(TexProError::InvalidName);
            }
        }

        match node.node_type {
            NodeType::InputGray(ref name) |
            NodeType::InputRgba(ref name) => {
                if self.input_names().contains(&&name) {
                    return Err(TexProError::InvalidName);
                }
            }
            NodeType::OutputGray(ref name) |
            NodeType::OutputRgba(ref name) => {
                if self.output_names().contains(&&name) {
                    return Err(TexProError::InvalidName);
                }
            }
            _ => ()
        }
        
        node.node_id = node_id;
        self.nodes.push(node);

        Ok(node_id)
    }

    pub fn input_nodes(&self) -> Vec<&Node> {
        self.nodes.iter().filter(|node| node.node_type.is_input()).collect()
    }

    pub fn output_nodes(&self) -> Vec<&Node> {
        self.nodes.iter().filter(|node| node.node_type.is_output()).collect()
    }

    pub fn input_names(&self) -> Vec<&String> {
        self.input_nodes().iter().map(|node| {
            if let NodeType::InputGray(name) | NodeType::InputRgba(name) = &node.node_type {
                name
            } else {
                unreachable!();
            }
        }).collect()
    }

    pub fn output_names(&self) -> Vec<&String> {
        self.output_nodes().iter().map(|node| {
            if let NodeType::OutputGray(name) | NodeType::OutputRgba(name) = &node.node_type {
                name
            } else {
                unreachable!();
            }
        }).collect()
    }

    pub fn input_slot_id_with_name(&self, name: &str) -> Option<SlotId> {
        if let Some(node) = self.input_nodes().iter().find(|node| node.node_type.name().unwrap() == name) {
            Some(SlotId(node.node_id.0))
        } else {
            None
        }
    }

    pub fn output_slot_id_with_name(&self, name: &str) -> Option<SlotId> {
        if let Some(node) = self.output_nodes().iter().find(|node| node.node_type.name().unwrap() == name) {
            Some(SlotId(node.node_id.0))
        } else {
            None
        }
    }

    pub fn input_slots(&self) -> Vec<SlotInput> {
        self.input_nodes().iter().map(|node| {
            let node_type = &node.node_type;
            
            SlotInput {
                name: node_type.name().unwrap().to_string(),
                slot_type: node_type.to_slot_type().unwrap(),
                slot_id: SlotId(node.node_id.0),
            }
        }).collect()
    }

    pub fn output_slots(&self) -> Vec<SlotOutput> {
        self.output_nodes().iter().map(|node| {
            let node_type = &node.node_type;
            
            SlotOutput {
                name: node_type.name().unwrap().to_string(),
                slot_type: node_type.to_slot_type().unwrap(),
                slot_id: SlotId(node.node_id.0),
            }
        }).collect()
    }

    pub fn add_node(&mut self, node: Node) -> Result<NodeId> {
        let node_id = self.new_id();
        self.add_node_internal(node, node_id)?;

        Ok(node_id)
    }

    pub fn add_node_with_id(&mut self, node: Node, node_id: NodeId) -> Result<NodeId> {
        if self.node_with_id(node_id).is_err() {
            self.add_node_internal(node, node_id)?;
        } else {
            return Err(TexProError::InvalidNodeId);
        }

        Ok(node_id)
    }

    /// Returns all `NodeId`s that belong to `OutputRgba` or `OutputGray` nodes.
    pub fn output_ids(&self) -> Vec<NodeId> {
        self.nodes
            .iter()
            .filter(|node| {
                node.node_type.is_output()
            })
            .map(|node| node.node_id)
            .collect()
    }

    pub fn external_input_ids(&self) -> Vec<NodeId> {
        self.nodes
            .iter()
            .filter(|node| {
                node.node_type.is_input()
            })
            .map(|node| node.node_id)
            .collect()
    }

    pub fn edge_indices_node(&mut self, node_id: NodeId) -> Result<Vec<usize>> {
        self.has_node_with_id(node_id)?;

        Ok(self
            .edges
            .iter()
            .enumerate()
            .filter(|(_, edge)| edge.output_id == node_id || edge.input_id == node_id)
            .map(|(i, _)| i)
            .collect())
    }

    pub fn edge_indices_slot(
        &mut self,
        node_id: NodeId,
        side: Side,
        slot_id: SlotId,
    ) -> Vec<usize> {
        self.edges
            .iter()
            .enumerate()
            .filter(|(_, edge)| match side {
                Side::Input => edge.input_id == node_id && edge.input_slot == slot_id,
                Side::Output => edge.output_id == node_id && edge.output_slot == slot_id,
            })
            .map(|(i, _)| i)
            .collect()
    }

    pub fn try_connect(
        &mut self,
        output_node: NodeId,
        input_node: NodeId,
        output_slot: SlotId,
        input_slot: SlotId,
    ) -> Result<()> {
        self.has_node_with_id(output_node)?;
        self.has_node_with_id(input_node)?;

        if self.slot_occupied(input_node, Side::Input, input_slot) {
            return Err(TexProError::SlotOccupied);
        }

        self.edges
            .push(Edge::new(output_node, input_node, output_slot, input_slot));

        Ok(())
    }

    pub fn connect(
        &mut self,
        output_node_id: NodeId,
        input_node_id: NodeId,
        output_slot: SlotId,
        input_slot: SlotId,
    ) -> Result<&Edge> {
        let output_node = self.node_with_id(output_node_id)?;
        let input_node = self.node_with_id(input_node_id)?;

        output_node.output_slot_with_id(output_slot)?;
        input_node.input_slot_with_id(input_slot)?;

        let _ = self.disconnect_slot(input_node_id, Side::Input, input_slot);

        self.edges.push(Edge::new(
            output_node_id,
            input_node_id,
            output_slot,
            input_slot,
        ));

        if let Some(edge) = self.edges.last() {
            Ok(edge)
        } else {
            unreachable!("We just added an edge, it can't possibly be empty.");
        }
    }

    pub fn connect_arbitrary(
        &mut self,
        a_node: NodeId,
        a_side: Side,
        a_slot: SlotId,
        b_node: NodeId,
        b_side: Side,
        b_slot: SlotId,
    ) -> Result<&Edge> {
        if a_node == b_node || a_side == b_side {
            return Err(TexProError::Generic);
        }

        match a_side {
            Side::Input => self.connect(b_node, a_node, b_slot, a_slot),
            Side::Output => self.connect(a_node, b_node, a_slot, b_slot),
        }
    }

    pub fn slot_occupied(&self, id: NodeId, side: Side, slot: SlotId) -> bool {
        match side {
            Side::Input => self
                .edges
                .iter()
                .any(|edge| edge.input_id == id && edge.input_slot == slot),
            Side::Output => self
                .edges
                .iter()
                .any(|edge| edge.output_id == id && edge.output_slot == slot),
        }
    }

    pub fn remove_edge_specific(
        &mut self,
        output_node: NodeId,
        input_node: NodeId,
        output_slot: SlotId,
        input_slot: SlotId,
    ) -> Result<Edge> {
        let edge_compare = Edge::new(output_node, input_node, output_slot, input_slot);

        if let Some(index_to_remove) = self.edges.iter().position(|edge| *edge == edge_compare) {
            Ok(self.edges.remove(index_to_remove))
        } else {
            Err(TexProError::InvalidEdge)
        }
    }

    pub fn remove_node(&mut self, node_id: NodeId) -> Result<(Node, Vec<Edge>)> {
        let removed_edges = self.disconnect_node(node_id)?;
        let index_to_remove = self
            .nodes
            .iter()
            .position(|node| node.node_id == node_id)
            .ok_or(TexProError::InvalidNodeId)?;
        Ok((self.nodes.remove(index_to_remove), removed_edges))
    }

    fn disconnect_node(&mut self, node_id: NodeId) -> Result<Vec<Edge>> {
        let mut removed_edges = Vec::new();

        for edge_index in self.edge_indices_node(node_id)?.into_iter().rev() {
            removed_edges.push(self.edges.remove(edge_index));
        }

        Ok(removed_edges)
    }

    pub fn disconnect_slot(
        &mut self,
        node_id: NodeId,
        side: Side,
        slot_id: SlotId,
    ) -> Result<Vec<Edge>> {
        self.has_node_with_id(node_id)?;

        let mut removed_edges = Vec::new();

        for edge_index in self.edge_indices_slot(node_id, side, slot_id) {
            removed_edges.push(self.edges.remove(edge_index));
        }

        if removed_edges.is_empty() {
            Err(TexProError::SlotNotOccupied)
        } else {
            Ok(removed_edges)
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct NodeId(pub u32);

impl NodeId {
    pub fn as_usize(self) -> usize {
        self.0 as usize
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd, Deserialize, Serialize)]
pub struct SlotId(pub u32);

impl fmt::Display for SlotId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl SlotId {
    pub fn as_usize(self) -> usize {
        self.0 as usize
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Serialize)]
pub struct Edge {
    pub output_id: NodeId,
    pub input_id: NodeId,
    pub output_slot: SlotId,
    pub input_slot: SlotId,
}

impl Edge {
    pub fn new(
        output_id: NodeId,
        input_id: NodeId,
        output_slot: SlotId,
        input_slot: SlotId,
    ) -> Self {
        Self {
            output_id,
            input_id,
            output_slot,
            input_slot,
        }
    }

    pub fn output_id(&self) -> NodeId {
        self.output_id
    }

    pub fn input_id(&self) -> NodeId {
        self.input_id
    }

    pub fn output_slot(&self) -> SlotId {
        self.output_slot
    }

    pub fn input_slot(&self) -> SlotId {
        self.input_slot
    }
}
