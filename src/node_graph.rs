use crate::{
    error::*,
    node::*,
};
use std::{
    collections::hash_map::HashMap,
    sync::Arc,
};

/// Cannot derive Debug because Node can't derive Debug because FilterType doesn't derive debug.
#[derive(Default)]
pub struct NodeGraph {
    pub nodes: HashMap<NodeId, Arc<Node>>,
    pub edges: Vec<Edge>,
}

impl NodeGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
        }
    }

    fn new_id(&mut self) -> NodeId {
        loop {
            let id = NodeId(rand::random());
            if !self.nodes.contains_key(&id) {
                return id;
            }
        }
    }

    fn edges(&self) -> Vec<Edge> {
        self.edges
    }

    fn add_node_internal(&mut self, node: Node, id: NodeId) {
        self.nodes.insert(id, Arc::new(node));
    }

    pub fn add_node(&mut self, node: Node) -> NodeId {
        if *node.node_type() == NodeType::Input {
            panic!("Use the `add_input_node()` function when adding an input node");
        }
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
            .values()
            .filter(|node| *node.node_type() == NodeType::Input)
            .count()
    }

    pub fn output_count(&self) -> usize {
        self.nodes
            .values()
            .filter(|node| *node.node_type() == NodeType::Output)
            .count()
    }

    pub fn connect(
        &mut self,
        id_1: NodeId,
        id_2: NodeId,
        slot_1: SlotId,
        slot_2: SlotId,
    ) -> Result<()> {
        if !self.nodes.contains_key(&id_1) || !self.nodes.contains_key(&id_2) {
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

#[derive(Debug, Clone)]
pub struct Edge {
    pub output_id: NodeId,
    pub input_id: NodeId,
    pub output_slot: SlotId,
    pub input_slot: SlotId,
}

impl Edge {
    pub fn new(output_id: NodeId, input_id: NodeId, output_slot: SlotId, input_slot: SlotId) -> Self {
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
