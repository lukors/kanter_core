use crate::{error::*, node::*, shared::has_dup};
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    fs::File,
    io::{self},
    mem,
    sync::Arc,
};

/// Cannot derive Debug because Node can't derive Debug because FilterType doesn't derive debug.
#[derive(Clone, Default, Deserialize, Serialize)]
pub struct NodeGraph {
    input_mappings: Vec<ExternalMapping>,
    output_mappings: Vec<ExternalMapping>,
    nodes: Vec<Arc<Node>>,
    pub edges: Vec<Edge>,
}

impl fmt::Debug for NodeGraph {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "NodeGraph {{ input_mappings: {:?}, edges: {:?} }}",
            self.input_mappings, self.edges
        )
    }
}

impl NodeGraph {
    pub fn new() -> Self {
        Self {
            input_mappings: Vec::new(),
            output_mappings: Vec::new(),
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
                    let mut node_clone: Node = (*self.nodes[node_index]).clone();
                    node_clone.node_type = NodeType::Mix(mix_type);

                    mem::replace(&mut self.nodes[node_index], Arc::new(node_clone));
                    Ok(())
                }
                _ => Err(TexProError::InvalidNodeId),
            }
        } else {
            Err(TexProError::InvalidNodeId)
        }
    }

    fn new_id(&mut self) -> NodeId {
        loop {
            let id = NodeId(rand::random());
            if !self.has_node_with_id(id) {
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

    /// Returns the `NodeId` and `SlotId` associated with the given
    /// external `SlotId` in `input_mappings`.
    pub fn input_mapping(&self, external_slot: SlotId) -> Result<(NodeId, SlotId)> {
        self.resolve_mapping(external_slot, &self.input_mappings)
    }

    /// Returns the `NodeId` and `SlotId` associated with the given
    /// external `SlotId` in `input_mappings`.
    pub fn output_mapping(&self, external_slot: SlotId) -> Result<(NodeId, SlotId)> {
        self.resolve_mapping(external_slot, &self.output_mappings)
    }

    /// Returns the `NodeId` and `SlotId` associated with the given
    /// external `SlotId` in the given `ExternalMapping`.
    fn resolve_mapping(
        &self,
        external_slot: SlotId,
        external_mappings: &[ExternalMapping],
    ) -> Result<(NodeId, SlotId)> {
        let external_mapping = external_mappings
            .iter()
            .find(|external_mapping| external_mapping.external_slot == external_slot);

        match external_mapping {
            Some(external_mapping) => Ok((
                external_mapping.internal_node,
                external_mapping.internal_slot,
            )),
            None => Err(TexProError::InvalidSlotId),
        }
    }

    fn index_of_node(&self, node_id: NodeId) -> Option<usize> {
        self.nodes.iter().position(|node| node.node_id == node_id)
    }

    fn has_node_with_id(&self, node_id: NodeId) -> bool {
        self.nodes.iter().any(|node| node.node_id == node_id)
    }

    pub fn nodes(&self) -> &Vec<Arc<Node>> {
        &self.nodes
    }

    pub fn node_ids(&self) -> Vec<NodeId> {
        self.nodes.iter().map(|node| node.node_id).collect()
    }

    pub fn node_with_id(&self, node_id: NodeId) -> Option<&Arc<Node>> {
        self.nodes.iter().find(|node| node.node_id == node_id)
    }

    fn add_node_internal(&mut self, mut node: Node, node_id: NodeId) {
        node.node_id = node_id;
        self.nodes.push(Arc::new(node));
    }

    /// Adds a grayscale input node and exposes its slots externally at the given `SlotId`.
    pub fn add_external_input_gray(&mut self, external_slot: SlotId) -> Result<NodeId> {
        if self.external_input_occupied(external_slot) {
            return Err(TexProError::SlotOccupied);
        }

        let internal_node = self.new_id();
        self.add_node_internal(Node::new(NodeType::InputGray), internal_node);

        self.input_mappings.push(ExternalMapping {
            external_slot,
            internal_node,
            internal_slot: SlotId(0),
        });

        Ok(internal_node)
    }

    /// Adds an rgba input node and exposes its slots externally at the given `SlotId`s.
    pub fn add_external_input_rgba(&mut self, external_slots: Vec<SlotId>) -> Result<NodeId> {
        if external_slots.len() != 4 || has_dup(&external_slots) {
            return Err(TexProError::InvalidNodeId);
        }
        for external_slot in &external_slots {
            if self.external_input_occupied(*external_slot) {
                return Err(TexProError::SlotOccupied);
            }
        }

        let internal_node = self.new_id();
        self.add_node_internal(Node::new(NodeType::InputRgba), internal_node);

        for (i, external_slot) in external_slots.iter().enumerate() {
            self.input_mappings.push(ExternalMapping {
                external_slot: *external_slot,
                internal_node,
                internal_slot: SlotId(i as u32),
            });
        }

        Ok(internal_node)
    }

    /// Adds a grayscale output node and exposes its slots externally at the given `SlotId`.
    pub fn add_external_output_gray(&mut self, external_slot: SlotId) -> Result<NodeId> {
        if self.external_output_occupied(external_slot) {
            return Err(TexProError::SlotOccupied);
        }

        let internal_node = self.new_id();
        self.add_node_internal(Node::new(NodeType::OutputGray), internal_node);

        self.output_mappings.push(ExternalMapping {
            external_slot,
            internal_node,
            internal_slot: SlotId(0),
        });

        Ok(internal_node)
    }

    /// Adds an rgba output node and exposes its slots externally at the given `SlotId`s.
    pub fn add_external_output_rgba(&mut self, external_slots: Vec<SlotId>) -> Result<NodeId> {
        if external_slots.len() != 4 || has_dup(&external_slots) {
            return Err(TexProError::InvalidNodeId);
        }
        for external_slot in &external_slots {
            if self.external_output_occupied(*external_slot) {
                return Err(TexProError::SlotOccupied);
            }
        }

        let internal_node = self.new_id();
        self.add_node_internal(Node::new(NodeType::OutputRgba), internal_node);

        for (i, external_slot) in external_slots.iter().enumerate() {
            self.output_mappings.push(ExternalMapping {
                external_slot: *external_slot,
                internal_node,
                internal_slot: SlotId(i as u32),
            });
        }

        Ok(internal_node)
    }

    /// Checks if the given external input `SlotId` is occupied.
    fn external_input_occupied(&self, external_slot_check: SlotId) -> bool {
        self.input_mappings
            .iter()
            .any(|input_mapping| input_mapping.external_slot == external_slot_check)
    }

    /// Checks if the given external output `SlotId` is occupied.
    fn external_output_occupied(&self, external_slot_check: SlotId) -> bool {
        self.output_mappings
            .iter()
            .any(|output_mapping| output_mapping.external_slot == external_slot_check)
    }

    pub fn add_node(&mut self, node: Node) -> Result<NodeId> {
        if node.node_type == NodeType::InputRgba || node.node_type == NodeType::InputGray {
            return Err(TexProError::InvalidNodeType);
        }

        let node_id = self.new_id();
        self.add_node_internal(node, node_id);

        Ok(node_id)
    }

    pub fn add_node_with_id(&mut self, node: Node, node_id: NodeId) -> Result<NodeId> {
        if self.node_with_id(node_id).is_some() {
            return Err(TexProError::InvalidNodeId);
        }

        self.add_node_internal(node, node_id);

        Ok(node_id)
    }

    pub fn input_count(&self) -> usize {
        let input_rgba_count = self
            .nodes
            .iter()
            .filter(|node| node.node_type == NodeType::InputRgba)
            .count();

        let input_gray_count = self
            .nodes
            .iter()
            .filter(|node| node.node_type == NodeType::InputGray)
            .count();

        input_rgba_count * 4 + input_gray_count
    }

    pub fn output_count(&self) -> usize {
        let output_rgba_count = self
            .nodes
            .iter()
            .filter(|node| node.node_type == NodeType::OutputRgba)
            .count();

        let output_gray_count = self
            .nodes
            .iter()
            .filter(|node| node.node_type == NodeType::OutputGray)
            .count();

        output_rgba_count * 4 + output_gray_count
    }

    pub fn external_output_ids(&self) -> Vec<NodeId> {
        self.nodes
            .iter()
            .filter(|node| {
                node.node_type == NodeType::OutputRgba || node.node_type == NodeType::OutputGray
            })
            .map(|node| node.node_id)
            .collect()
    }

    pub fn external_input_ids(&self) -> Vec<NodeId> {
        self.nodes
            .iter()
            .filter(|node| {
                node.node_type == NodeType::InputRgba || node.node_type == NodeType::InputGray
            })
            .map(|node| node.node_id)
            .collect()
    }

    pub fn edges_in_slot(
        &mut self,
        node_id: NodeId,
        side: Side,
        slot_id: SlotId,
    ) -> Vec<(usize, &Edge)> {
        self.edges
            .iter()
            .enumerate()
            .filter(|(_, edge)| match side {
                Side::Input => edge.input_id == node_id && edge.input_slot == slot_id,
                Side::Output => edge.output_id == node_id && edge.output_slot == slot_id,
            })
            .collect()
    }

    pub fn disconnect_slot(&mut self, node_id: NodeId, side: Side, slot_id: SlotId) {
        let mut edge_indices_to_remove: Vec<usize> = self
            .edges_in_slot(node_id, side, slot_id)
            .iter()
            .map(|(i, _)| *i)
            .collect();

        edge_indices_to_remove.sort_unstable();
        for i in edge_indices_to_remove.iter().rev() {
            self.edges.remove(*i);
        }
    }

    pub fn try_connect(
        &mut self,
        output_node: NodeId,
        input_node: NodeId,
        output_slot: SlotId,
        input_slot: SlotId,
    ) -> Result<()> {
        if !self.has_node_with_id(output_node) || !self.has_node_with_id(input_node) {
            return Err(TexProError::InvalidNodeId);
        }

        if self.slot_occupied(input_node, Side::Input, input_slot) {
            return Err(TexProError::SlotOccupied);
        }

        self.edges
            .push(Edge::new(output_node, input_node, output_slot, input_slot));

        Ok(())
    }

    pub fn connect(
        &mut self,
        output_node: NodeId,
        input_node: NodeId,
        output_slot: SlotId,
        input_slot: SlotId,
    ) -> Result<()> {
        if !self.has_node_with_id(output_node) || !self.has_node_with_id(input_node) {
            return Err(TexProError::InvalidNodeId);
        }

        self.disconnect_slot(input_node, Side::Input, input_slot);

        self.edges
            .push(Edge::new(output_node, input_node, output_slot, input_slot));

        Ok(())
    }

    pub fn try_connect_arbitrary(
        &mut self,
        a_node: NodeId,
        a_side: Side,
        a_slot: SlotId,
        b_node: NodeId,
        b_side: Side,
        b_slot: SlotId,
    ) -> Result<()> {
        if a_node == b_node || a_side == b_side {
            return Err(TexProError::Generic);
        }

        match a_side {
            Side::Input => self.try_connect(b_node, a_node, b_slot, a_slot),
            Side::Output => self.try_connect(a_node, b_node, a_slot, b_slot),
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
    ) -> Result<()> {
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

    pub fn remove_edge(
        &mut self,
        output_node: NodeId,
        input_node: NodeId,
        output_slot: SlotId,
        input_slot: SlotId,
    ) {
        let edge_compare = Edge::new(output_node, input_node, output_slot, input_slot);

        if let Some(index_to_remove) = self.edges.iter().position(|edge| *edge == edge_compare) {
            self.edges.remove(index_to_remove);
        }
    }

    pub fn remove_node(&mut self, node_id: NodeId) {
        self.disconnect_node(node_id);

        if let Some(index_to_remove) = self.nodes.iter().position(|node| node.node_id == node_id) {
            self.nodes.remove(index_to_remove);
        }
    }

    fn disconnect_node(&mut self, node_id: NodeId) {
        while let Some(edge_index) = self
            .edges
            .iter()
            .rposition(|edge| edge.output_id == node_id || edge.input_id == node_id)
        {
            self.edges.remove(edge_index);
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
struct ExternalMapping {
    external_slot: SlotId,
    internal_node: NodeId,
    internal_slot: SlotId,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct NodeId(pub u32);

impl NodeId {
    pub fn as_usize(self) -> usize {
        self.0 as usize
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd, Deserialize, Serialize)]
pub struct SlotId(pub u32);

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
            output_slot,
            input_id,
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
