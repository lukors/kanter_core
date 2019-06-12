use crate::{error::*, node::*};
use std::{collections::hash_map::HashMap, sync::Arc};

/// Cannot derive Debug because Node can't derive Debug because FilterType doesn't derive debug.
#[derive(Default, Clone)]
pub struct NodeGraph {
    input_slots: Vec<(SlotId, NodeId)>,
    nodes: Vec<Arc<Node>>,
    pub edges: Vec<Edge>,
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

    fn add_node_internal(&mut self, mut node: Node, id: NodeId) {
        node.node_id = id;
        self.nodes.push(Arc::new(node));
    }

    /// Is used for adding inputs so that you can decide which slot the input should map to.
    pub fn add_node_input(&mut self, slot_id: SlotId) -> Result<NodeId> {
        unimplemented!();
        Ok(self.add_node(Node::new(NodeType::Input)))
    }

    pub fn add_node(&mut self, node: Node) -> NodeId {
        // TODO: Should crash if you try to add an input node.

        let id = self.new_id();
        self.add_node_internal(node, id);
        id
    }

    pub fn add_node_with_id(&mut self, node: Node, id: NodeId) -> NodeId {
        self.add_node_internal(node, id);
        id
    }

    pub fn input_count(&self) -> usize {
        self.nodes
            .iter()
            .filter(|node| node.node_type == NodeType::Input)
            .count()
    }

    pub fn output_count(&self) -> usize {
        self.nodes
            .iter()
            .filter(|node| node.node_type == NodeType::Output)
            .count()
    }

    pub fn output_ids(&self) -> Vec<NodeId> {
        self.nodes
            .iter()
            .filter(|node| node.node_type == NodeType::Output)
            .map(|node| node.node_id)
            .collect()
    }

    pub fn input_ids(&self) -> Vec<NodeId> {
        self.nodes
            .iter()
            .filter(|node| node.node_type == NodeType::Input)
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
