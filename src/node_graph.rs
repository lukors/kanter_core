use crate::{
    edge::Edge,
    error::*,
    node::{mix::MixType, node_type::NodeType, Node, Side, SlotInput, SlotOutput},
};
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    fs::File,
    io::{self},
    mem,
    path::PathBuf,
    sync::atomic::Ordering,
};

#[derive(Clone, Default, Debug, Deserialize, Serialize)]
pub struct NodeGraph {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    #[serde(skip)]
    node_id_counter: NodeId,
}

impl NodeGraph {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            node_id_counter: NodeId(0),
        }
    }

    pub fn from_path(path: String) -> io::Result<Self> {
        let mut graph = Self::import_json(path)?;

        let node_id_counter =
            if let Some(node_id) = graph.nodes.iter().map(|node| node.node_id).max() {
                NodeId(node_id.0 + 1)
            } else {
                NodeId(0)
            };

        graph.node_id_counter = node_id_counter;

        Ok(graph)
    }

    pub fn set_mix_type(&mut self, node_id: NodeId, mix_type: MixType) -> Result<()> {
        if let Some(node_index) = self.index_of_node(node_id) {
            match self.nodes[node_index].node_type {
                NodeType::Mix(_) => {
                    let mut node_clone: Node = (self.nodes[node_index]).clone();
                    node_clone.node_type = NodeType::Mix(mix_type);

                    let _ = mem::replace(&mut self.nodes[node_index], node_clone);
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
        let mut output = self.node_id_counter;
        self.node_id_counter.0 += 1;

        while self.has_node_with_id(output).is_ok() {
            output = self.node_id_counter;
            self.node_id_counter.0 += 1;
        }

        output
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

    pub fn node(&self, node_id: NodeId) -> Result<Node> {
        self.nodes
            .iter()
            .find(|node| node.node_id == node_id)
            .cloned()
            .ok_or(TexProError::InvalidNodeId)
    }

    pub(crate) fn node_with_id_mut(&mut self, node_id: NodeId) -> Option<&mut Node> {
        self.nodes.iter_mut().find(|node| node.node_id == node_id)
    }

    fn avoid_name_collision(name_list: Vec<&String>, name: &str) -> String {
        let mut name_edit = name.to_string();

        while name_list.contains(&&name_edit) {
            // Find the last underscore
            if let Some((name, number)) = name_edit.rsplit_once('_') {
                if number.chars().all(char::is_numeric) {
                    let number = if let Ok(number) = number.parse::<u32>() {
                        number.wrapping_add(1)
                    } else {
                        0
                    };

                    name_edit = String::from(format!("{}_{}", name, number).as_str());
                } else {
                    name_edit = String::from(format!("{}_0", name).as_str());
                }
            } else {
                name_edit = String::from(format!("{}_0", name_edit).as_str());
            }
        }

        name_edit
    }

    fn add_node_internal(&mut self, mut node: Node, node_id: NodeId) -> Result<NodeId> {
        let node_type_clone = node.node_type.clone();

        if let Some(name) = node.node_type.name_mut() {
            if name.is_empty() {
                *name = String::from("untitled");
            }

            match node_type_clone {
                NodeType::InputGray(_) | NodeType::InputRgba(_) => {
                    *name = Self::avoid_name_collision(self.input_names(), name);
                }
                NodeType::OutputGray(_) | NodeType::OutputRgba(_) => {
                    *name = Self::avoid_name_collision(self.output_names(), name);
                }
                _ => unreachable!("Only inputs and outputs have names"),
            }
        }

        node.node_id = node_id;
        self.nodes.push(node);

        Ok(node_id)
    }

    pub fn input_nodes(&self) -> Vec<&Node> {
        self.nodes
            .iter()
            .filter(|node| node.node_type.is_input())
            .collect()
    }

    pub fn output_nodes(&self) -> Vec<&Node> {
        self.nodes
            .iter()
            .filter(|node| node.node_type.is_output())
            .collect()
    }

    pub fn input_names(&self) -> Vec<&String> {
        self.input_nodes()
            .iter()
            .map(|node| {
                if let NodeType::InputGray(name) | NodeType::InputRgba(name) = &node.node_type {
                    name
                } else {
                    unreachable!();
                }
            })
            .collect()
    }

    pub fn output_names(&self) -> Vec<&String> {
        self.output_nodes()
            .iter()
            .map(|node| {
                if let NodeType::OutputGray(name) | NodeType::OutputRgba(name) = &node.node_type {
                    name
                } else {
                    unreachable!();
                }
            })
            .collect()
    }

    /// Gives an Output node a new name, returning the old name.
    pub fn rename_output_node(&mut self, node_id: NodeId, new_name: &str) -> Result<String> {
        let name_list = self
            .output_names()
            .iter()
            .map(|name| (**name).clone())
            .collect::<Vec<String>>();
        let mut name_list = name_list.iter().collect::<Vec<&String>>();

        let mut node = self
            .node_with_id_mut(node_id)
            .ok_or(TexProError::InvalidNodeId)?;

        let old_name = if let NodeType::OutputRgba(name) | NodeType::OutputGray(name) =
            node.node_type.clone()
        {
            name
        } else {
            return Err(TexProError::InvalidNodeType);
        };

        let old_name_index = name_list
            .iter()
            .position(|name| **name == old_name)
            .unwrap();
        name_list.remove(old_name_index);

        node.node_type = match node.node_type {
            NodeType::OutputRgba(_) => {
                NodeType::OutputRgba(Self::avoid_name_collision(name_list, new_name))
            }
            NodeType::OutputGray(_) => {
                NodeType::OutputGray(Self::avoid_name_collision(name_list, new_name))
            }
            _ => return Err(TexProError::InvalidNodeType),
        };

        Ok(old_name)
    }

    pub fn input_slot_id_with_name(&self, name: &str) -> Option<SlotId> {
        self.input_nodes()
            .iter()
            .find(|node| node.node_type.name().unwrap() == name)
            .map(|node| SlotId(node.node_id.0))
    }

    pub fn output_slot_id_with_name(&self, name: &str) -> Option<SlotId> {
        self.output_nodes()
            .iter()
            .find(|node| node.node_type.name().unwrap() == name)
            .map(|node| SlotId(node.node_id.0))
    }

    pub fn input_slots(&self) -> Vec<SlotInput> {
        self.input_nodes()
            .iter()
            .map(|node| {
                let node_type = &node.node_type;

                SlotInput {
                    name: node_type.name().unwrap().to_string(),
                    slot_type: node_type.to_slot_type().unwrap(),
                    slot_id: SlotId(node.node_id.0),
                }
            })
            .collect()
    }

    pub fn output_slots(&self) -> Vec<SlotOutput> {
        self.output_nodes()
            .iter()
            .map(|node| {
                let node_type = &node.node_type;

                SlotOutput {
                    name: node_type.name().unwrap().to_string(),
                    slot_type: node_type.to_slot_type().unwrap(),
                    slot_id: SlotId(node.node_id.0),
                }
            })
            .collect()
    }

    pub fn add_node(&mut self, node: Node) -> Result<NodeId> {
        let node_id = self.new_id();
        self.add_node_internal(node, node_id)?;

        Ok(node_id)
    }

    pub fn add_node_with_id(&mut self, node: Node) -> Result<()> {
        if self.node(node.node_id).is_err() {
            let node_id = node.node_id;
            self.add_node_internal(node, node_id)?;
        } else {
            return Err(TexProError::InvalidNodeId);
        }

        Ok(())
    }

    /// Returns all `NodeId`s that belong to `OutputRgba` or `OutputGray` nodes.
    pub fn output_ids(&self) -> Vec<NodeId> {
        self.nodes
            .iter()
            .filter(|node| node.node_type.is_output())
            .map(|node| node.node_id)
            .collect()
    }

    pub fn input_ids(&self) -> Vec<NodeId> {
        self.nodes
            .iter()
            .filter(|node| node.node_type.is_input())
            .map(|node| node.node_id)
            .collect()
    }

    /// Returns the indices of all `Edge`s that connect to the given `NodeId`.
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

    /// Returns the indices of all `Edge`s that connect to the given `SlotId`.
    pub fn edge_indices_slot(&self, node_id: NodeId, side: Side, slot_id: SlotId) -> Vec<usize> {
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

    pub fn can_connect(
        &self,
        output_node_id: NodeId,
        input_node_id: NodeId,
        output_slot_id: SlotId,
        input_slot_id: SlotId,
    ) -> Result<()> {
        self.node(output_node_id)?
            .output_slot_with_id(output_slot_id)?;
        self.node(input_node_id)?
            .input_slot_with_id(input_slot_id)?;

        if self.slot_occupied(input_node_id, Side::Input, input_slot_id) {
            return Err(TexProError::SlotOccupied);
        }

        Ok(())
    }

    /// Try to create a connection, but don't force it if it's occupied.
    pub fn try_connect(
        &mut self,
        output_node_id: NodeId,
        input_node_id: NodeId,
        output_slot_id: SlotId,
        input_slot_id: SlotId,
    ) -> Result<()> {
        self.can_connect(output_node_id, input_node_id, output_slot_id, input_slot_id)?;

        self.edges.push(Edge::new(
            output_node_id,
            input_node_id,
            output_slot_id,
            input_slot_id,
        ));

        Ok(())
    }

    /// Force a connection to be created, if there already is a connection, it will be removed.
    pub fn connect(
        &mut self,
        output_node_id: NodeId,
        input_node_id: NodeId,
        output_slot_id: SlotId,
        input_slot_id: SlotId,
    ) -> Result<&Edge> {
        let new_edge = Edge::new(output_node_id, input_node_id, output_slot_id, input_slot_id);

        let output_node = self.node(output_node_id)?;
        let input_node = self.node(input_node_id)?;

        let output_slot_type = output_node.output_slot_with_id(output_slot_id)?.slot_type;
        let input_slot_type = input_node.input_slot_with_id(input_slot_id)?.slot_type;

        output_slot_type.fits(input_slot_type)?;

        // Discarding this result because we don't care if anything got disconnected.
        let _ = self.disconnect_slot(input_node_id, Side::Input, input_slot_id);

        if self.edges.contains(&new_edge) {
            return Err(TexProError::InvalidEdge);
        }
        self.edges.push(new_edge);

        if let Some(edge) = self.edges.last() {
            Ok(edge)
        } else {
            unreachable!("We just added an edge, it can't possibly be empty.");
        }
    }

    /// Check if a slot is occupied.
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

    pub fn remove_edge(&mut self, edge: Edge) -> Result<Edge> {
        if let Some(index_to_remove) = self.edges.iter().position(|edge_cmp| *edge_cmp == edge) {
            self.node(edge.input_id)?
                .cancel
                .store(true, Ordering::Relaxed);
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

    /// Removes all `Edge`s plugged into the given `NodeId`.
    fn disconnect_node(&mut self, node_id: NodeId) -> Result<Vec<Edge>> {
        self.node(node_id)?.cancel.store(true, Ordering::Relaxed);
        let mut removed_edges = Vec::new();

        for edge_index in self.edge_indices_node(node_id)?.into_iter().rev() {
            removed_edges.push(self.edges.remove(edge_index));
        }

        Ok(removed_edges)
    }

    /// Removes all `Edge`s plugged into the given `SlotId`.
    pub fn disconnect_slot(
        &mut self,
        node_id: NodeId,
        side: Side,
        slot_id: SlotId,
    ) -> Result<Vec<Edge>> {
        self.node(node_id)?.cancel.store(true, Ordering::Relaxed);

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

    /// Returns all `Edge`s plugged into the given `SlotId`.
    pub fn connected_edges(
        &self,
        node_id: NodeId,
        side: Side,
        slot_id: SlotId,
    ) -> Result<Vec<Edge>> {
        self.has_node_with_id(node_id)?;

        let mut edges = Vec::new();

        for edge_index in self.edge_indices_slot(node_id, side, slot_id) {
            edges.push(self.edges[edge_index]);
        }

        if edges.is_empty() {
            Err(TexProError::SlotNotOccupied)
        } else {
            Ok(edges)
        }
    }

    /// Gets all edges that are connected to input slots of this node.
    pub fn input_edges(&self, node_id: NodeId) -> Vec<Edge> {
        self.edges
            .iter()
            .filter(|edge| edge.input_id == node_id)
            .copied()
            .collect()
    }

    /// Returns the `NodeId`s of all immediate children of the given `NodeId` (not recursive).
    pub fn get_children(&self, node_id: NodeId) -> Result<Vec<NodeId>> {
        self.has_node_with_id(node_id)?;

        let mut children = self
            .edges
            .iter()
            .filter(|edge| edge.output_id == node_id)
            .map(|edge| edge.input_id)
            .collect::<Vec<NodeId>>();

        children.sort_unstable();
        children.dedup();

        Ok(children)
    }

    /// Returns the `NodeId`s of all children of the given `NodeId`.
    pub fn get_children_recursive(&self, node_id: NodeId) -> Result<Vec<NodeId>> {
        let children = self.get_children(node_id)?;
        let mut output = children.clone();

        for child in children {
            output.append(&mut self.get_children_recursive(child)?);
        }

        Ok(output)
    }

    /// Returns the `NodeId`s of all immediate parents of the given `NodeId` (not recursive).
    pub fn get_parents(&self, node_id: NodeId) -> Vec<NodeId> {
        let mut node_ids = self
            .edges
            .iter()
            .filter(|edge| edge.input_id == node_id)
            .map(|edge| edge.output_id)
            .collect::<Vec<NodeId>>();

        node_ids.sort_unstable();
        node_ids.dedup();
        node_ids
    }
}

#[derive(
    Clone, Copy, Debug, Default, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize,
)]
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

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Ord, PartialOrd, Deserialize, Serialize,
)]
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
