use crate::{error::*, node::*};
use std::{collections::hash_map::HashMap, fmt, sync::Arc};

/// Cannot derive Debug because Node can't derive Debug because FilterType doesn't derive debug.
#[derive(Default, Clone)]
pub struct NodeGraph {
    input_slots: Vec<(SlotId, NodeId)>,
    nodes: Vec<Arc<Node>>,
    pub edges: Vec<Edge>,
}

impl fmt::Debug for NodeGraph {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "NodeGraph {{ input_slots: {:?}, edges: {:?} }}", self.input_slots, self.edges)
    }
}

impl NodeGraph {
    pub fn new() -> Self {
        Self {
            input_slots: Vec::new(),
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

    pub fn input_slot(&self, node_id: NodeId) -> SlotId {
        self.input_slots.iter().find(|(_, i_node_id)| node_id == *i_node_id).unwrap().0
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

    /// Adds an input so that you can decide which slot the input should map to.
    pub fn add_node_input(&mut self, slot_id: SlotId) -> Result<NodeId> {
        if self.input_slot_occupied(slot_id) {
            return Err(TexProError::SlotOccupied)
        }

        let node_id = self.new_id();
        self.add_node_internal(Node::new(NodeType::InputGray), node_id);
        self.input_slots.push((slot_id, node_id));

        Ok(node_id)
    }

    /// Adds an input so that you can decide which slot the input should map to.
    pub fn add_node_input_rgba(&mut self, slot_id: SlotId) -> Result<NodeId> {
        if self.input_slot_occupied(slot_id) {
            return Err(TexProError::SlotOccupied)
        }

        let node_id = self.new_id();
        self.add_node_internal(Node::new(NodeType::InputRgba), node_id);
        self.input_slots.push((slot_id, node_id));

        Ok(node_id)
    }

    fn input_slot_occupied(&self, input_slot_id: SlotId) -> bool {
        self.input_slots.iter().any(|(slot_id, _node_id)| input_slot_id == *slot_id)
    }

    pub fn add_node(&mut self, node: Node) -> Result<NodeId> {
        if node.node_type == NodeType::InputRgba || node.node_type == NodeType::InputGray {
            return Err(TexProError::InvalidNodeType)
        }

        let node_id = self.new_id();
        self.add_node_internal(node, node_id);
        
        Ok(node_id)
    }

    pub fn add_node_with_id(&mut self, node: Node, id: NodeId) -> NodeId {
        self.add_node_internal(node, id);
        id
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

    pub fn output_ids(&self) -> Vec<NodeId> {
        self.nodes
            .iter()
            .filter(|node| node.node_type == NodeType::OutputRgba)
            .map(|node| node.node_id)
            .collect()
    }

    pub fn input_ids(&self) -> Vec<NodeId> {
        self.nodes
            .iter()
            .filter(|node| node.node_type == NodeType::InputRgba)
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
