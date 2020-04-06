use crate::{error::*, node::*, shared::has_dup};
use std::{collections::hash_map::HashMap, fmt, sync::Arc};

/// Cannot derive Debug because Node can't derive Debug because FilterType doesn't derive debug.
#[derive(Default, Clone)]
pub struct NodeGraph {
    input_mappings: Vec<InputMapping>,
    nodes: Vec<Arc<Node>>,
    pub edges: Vec<Edge>,
}

impl fmt::Debug for NodeGraph {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "NodeGraph {{ input_mappings: {:?}, edges: {:?} }}", self.input_mappings, self.edges)
    }
}

impl NodeGraph {
    pub fn new() -> Self {
        Self {
            input_mappings: Vec::new(),
            nodes: Vec::new(),
            edges: Vec::new(),
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

    // COMMENTED OUT DUE TO TOTALLY WRONG IMPLEMENTATION
    /// Returns all input `SlotId`s associated with the given `NodeId`.
    // pub fn node_input_slots(&self, node_id: NodeId) -> Vec<SlotId> {
    //     self.input_slots.iter()
    //         .filter(|(_, i_node_id)| node_id == *i_node_id)
    //         .map(|node_id| node_id.0)
    //         .collect()
    // }

    /// Returns all input `SlotId`s associated with the given `NodeId`.
    // pub fn external_input_slots(&self) -> Vec<SlotId> {
    //     self.input_mappings.iter()
    //         .map(|input_mapping| input_mapping.external_slot)
    //         .collect()
    // }

    /// Returns the `NodeId` and `SlotId` associated with the given
    /// external `SlotId` in `input_mappings`.
    pub fn input_mapping(&self, external_slot: SlotId) -> Result<(NodeId, SlotId)> {
        let input_mapping = self.input_mappings.iter()
            .find(|input_mapping| input_mapping.external_slot == external_slot);
        
        match input_mapping {
            Some(input_mapping) => Ok((input_mapping.input_id, input_mapping.input_slot)),
            None => Err(TexProError::InvalidSlotId),
        }
    }

    fn has_node_with_id(&self, node_id: NodeId) -> bool {
        self.nodes.iter().any(|node| node.node_id == node_id)
    }

    pub fn nodes(&self) -> &Vec<Arc<Node>> {
        &self.nodes
    }

    pub fn node_with_id(&self, node_id: NodeId) -> Option<&Arc<Node>> {
        self.nodes.iter().find(|node| node.node_id == node_id)
    }

    fn edges(&self) -> &[Edge] {
        &self.edges
    }

    fn add_node_internal(&mut self, mut node: Node, node_id: NodeId) {
        node.node_id = node_id;
        self.nodes.push(Arc::new(node));
    }

    /// Adds an external grayscale input at the given `SlotId`.
    pub fn add_external_input_gray(&mut self, external_slot: SlotId) -> Result<NodeId> {
        if self.external_slot_occupied(external_slot) {
            return Err(TexProError::SlotOccupied)
        }

        let input_id = self.new_id();
        self.add_node_internal(Node::new(NodeType::InputGray), input_id);

        self.input_mappings.push(InputMapping {
            external_slot,
            input_id,
            input_slot: SlotId(0),
        });

        Ok(input_id)
    }

    /// Adds an input so that you can decide which slot the input should map to.
    pub fn add_external_input_rgba(&mut self, external_slots: Vec<SlotId>) -> Result<NodeId> {
        if external_slots.len() != 4 || has_dup(&external_slots) {
            return Err(TexProError::InvalidNodeId)
        }

        let input_id = self.new_id();
        self.add_node_internal(Node::new(NodeType::InputRgba), input_id);

        for (i, external_slot) in external_slots.to_vec().iter().enumerate() {
            if self.external_slot_occupied(*external_slot) {
                return Err(TexProError::SlotOccupied)
            } else {
                self.input_mappings.push(InputMapping {
                    external_slot: *external_slot,
                    input_id,
                    input_slot: SlotId(i as u32),
                });
            }
        }

        Ok(input_id)
    }

    fn external_slot_occupied(&self, input_slot_id: SlotId) -> bool {
        self.input_mappings.iter().any(|input_mapping| input_mapping.external_slot == input_slot_id)
    }

    pub fn add_node(&mut self, node: Node) -> Result<NodeId> {
        if node.node_type == NodeType::InputRgba || node.node_type == NodeType::InputGray {
            return Err(TexProError::InvalidNodeType)
        }

        let node_id = self.new_id();
        self.add_node_internal(node, node_id);
        
        Ok(node_id)
    }

    pub fn add_node_with_id(&mut self, node: Node, node_id: NodeId) -> Result<NodeId> {
        if self.node_with_id(node_id).is_some() {
            return Err(TexProError::InvalidNodeId)
        }

        self.add_node_internal(node, node_id);
        
        Ok(node_id)
    }

    pub fn input_count(&self) -> usize {
        let input_rgba_count = self.nodes
            .iter()
            .filter(|node| node.node_type == NodeType::InputRgba)
            .count();

        let input_gray_count = self.nodes
            .iter()
            .filter(|node| node.node_type == NodeType::InputGray)
            .count();

        input_rgba_count*4 + input_gray_count
    }

    pub fn output_count(&self) -> usize {
        let output_rgba_count = self.nodes
            .iter()
            .filter(|node| node.node_type == NodeType::OutputRgba)
            .count();

        let output_gray_count = self.nodes
            .iter()
            .filter(|node| node.node_type == NodeType::Output)
            .count();

        output_rgba_count*4 + output_gray_count
    }

    pub fn external_output_ids(&self) -> Vec<NodeId> {
        self.nodes
            .iter()
            .filter(|node| node.node_type == NodeType::OutputRgba || node.node_type == NodeType::Output)
            .map(|node| node.node_id)
            .collect()
    }

    pub fn external_input_ids(&self) -> Vec<NodeId> {
        self.nodes
            .iter()
            .filter(|node| node.node_type == NodeType::InputRgba || node.node_type ==  NodeType::InputGray)
            .map(|node| node.node_id)
            .collect()
    }

    pub fn connect(
        &mut self,
        id_1: NodeId,
        id_2: NodeId,
        slot_1: SlotId,
        slot_2: SlotId,
    ) -> Result<()> {
        if !self.has_node_with_id(id_1) || !self.has_node_with_id(id_2) {
            return Err(TexProError::InvalidNodeId);
        }

        if self.slot_occupied(id_2, Side::Input, slot_2) {
            return Err(TexProError::SlotOccupied);
        }

        self.edges.push(Edge::new(id_1, id_2, slot_1, slot_2));

        Ok(())
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
}

#[derive(Clone, Copy, Debug)]
struct InputMapping {
    external_slot: SlotId,
    input_id: NodeId,
    input_slot: SlotId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct NodeId(pub u32);

impl NodeId {
    pub fn as_usize(&self) -> usize {
        self.0 as usize
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct SlotId(pub u32);

impl SlotId {
    pub fn as_usize(&self) -> usize {
        self.0 as usize
    }
}

#[derive(Debug, Clone, Copy)]
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
