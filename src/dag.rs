// TODO:
// - Add Result things everywhere
// - Add a resize node, though nodes are able to output a different size than their input.
// - Implement same features as Channel Shuffle 1 & 2.
// - Implement CLI.
// - Make randomly generated test to try finding corner cases.
// - Make benchmark tests.
// - Optimize away the double-allocation when resizing an image before it's processed.
// - Make each node save the resized versions of their inputs,
//   and use them if they are still relevant.

use image::{DynamicImage, ImageBuffer};
use crate::error::{Result, TexProError};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::{mpsc, Arc},
    thread,
};

use crate::node::*;
use crate::shared::*;

#[derive(Default)]
pub struct TextureProcessor {
    nodes: HashMap<NodeId, Arc<Node>>,
    node_data: HashMap<NodeId, NodeData>,
    edges: Vec<Edge>,
}

#[derive(Debug, Clone)]
pub struct Edge {
    output_id: NodeId,
    input_id: NodeId,
    output_slot: Slot,
    input_slot: Slot,
}

impl Edge {
    fn new(output_id: NodeId, input_id: NodeId, output_slot: Slot, input_slot: Slot) -> Self {
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

    pub fn output_slot(&self) -> Slot {
        self.output_slot
    }

    pub fn input_slot(&self) -> Slot {
        self.input_slot
    }
}

impl TextureProcessor {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            node_data: HashMap::new(),
            edges: Vec::new(),
        }
    }

    fn add_node_internal(&mut self, node: Node, id: NodeId) {
        self.nodes.insert(id, Arc::new(node));
    }

    pub fn add_node(&mut self, node: Node) -> NodeId {
        if *node.get_type() == NodeType::Input {
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

    /// This function takes an image, creates a node for it and returns the NodeId.
    pub fn add_input_node(&mut self, image: &DynamicImage) -> NodeId {
        let id = self.new_id();

        self.add_node_internal(Node::new(NodeType::Input), id);

        let mut wrapped_buffers = HashMap::new();
        for (id, buffer) in deconstruct_image(&image).into_iter().enumerate() {
            wrapped_buffers.insert(Slot(id), Arc::new(buffer));
        }

        self.node_data
            .insert(id, NodeData::from_buffers(wrapped_buffers));

        id
    }

    pub fn connect(
        &mut self,
        id_1: NodeId,
        id_2: NodeId,
        slot_1: Slot,
        slot_2: Slot,
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

    pub fn slot_occupied(&self, id: NodeId, side: Side, slot: Slot) -> bool {
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

    pub fn process(&mut self) {
        struct ThreadMessage {
            id: NodeId,
            buffers: Vec<DetachedBuffer>,
        }

        let (send, recv) = mpsc::channel::<ThreadMessage>();
        let mut finished_nodes: HashSet<NodeId> = HashSet::with_capacity(self.nodes.len());
        let mut started_nodes: HashSet<NodeId> = HashSet::with_capacity(self.nodes.len());

        let mut queued_ids: VecDeque<NodeId> = VecDeque::from(self.get_root_ids());
        for item in &queued_ids {
            started_nodes.insert(*item);
        }

        'outer: while finished_nodes.len() < self.nodes.len() {
            for message in recv.try_iter() {
                self.set_node_finished(
                    message.id,
                    Some(message.buffers),
                    &mut started_nodes,
                    &mut finished_nodes,
                    &mut queued_ids,
                );
            }

            let current_id = match queued_ids.pop_front() {
                Some(id) => id,
                None => continue,
            };

            if self.node_data.contains_key(&current_id) {
                self.set_node_finished(
                    current_id,
                    None,
                    &mut started_nodes,
                    &mut finished_nodes,
                    &mut queued_ids,
                );
                continue;
            }

            let parent_ids = self
                .edges
                .iter()
                .filter(|edge| edge.input_id == current_id)
                .map(|edge| edge.output_id);

            for id in parent_ids {
                if !finished_nodes.contains(&id) {
                    queued_ids.push_back(current_id);
                    continue 'outer;
                }
            }

            let mut relevant_ids: Vec<NodeId> = Vec::new();
            for id in self.node_data.keys() {
                for edge in &self.edges {
                    if edge.output_id != *id {
                        continue;
                    } else {
                        relevant_ids.push(*id);
                    }
                }
            }

            let mut relevant_edges: Vec<Edge> = Vec::new();
            let mut input_data: Vec<DetachedBuffer> = Vec::new();
            for (id, node_data) in &self.node_data {
                if !relevant_ids.contains(&id) {
                    continue;
                }
                for edge in &self.edges {
                    for (slot, data_vec) in node_data.get_buffers().iter() {
                        if *slot == edge.output_slot
                            && *id == edge.output_id
                            && current_id == edge.input_id
                        {
                            input_data.push(DetachedBuffer::new(
                                Some(*id),
                                *slot,
                                node_data.get_size(),
                                Arc::clone(data_vec),
                            ));
                            relevant_edges.push(edge.clone());
                        }
                    }
                }
            }

            let current_node = Arc::clone(&self.nodes[&current_id]);
            let send = send.clone();

            thread::spawn(move || {
                let buffers = current_node
                    .process(&mut input_data, &relevant_edges)
                    .unwrap();

                match send.send(ThreadMessage {
                    id: current_id,
                    buffers,
                }) {
                    Ok(_) => (),
                    Err(e) => println!("{:?}", e),
                };
            });
        }
    }

    fn set_node_finished(
        &mut self,
        id: NodeId,
        buffers: Option<Vec<DetachedBuffer>>,
        started_nodes: &mut HashSet<NodeId>,
        finished_nodes: &mut HashSet<NodeId>,
        queued_ids: &mut VecDeque<NodeId>,
    ) {
        finished_nodes.insert(id);

        if let Some(buffers) = buffers {
            if !buffers.is_empty() {
                // let id = buffers[0].id;
                self.node_data.insert(id, NodeData::new(buffers[0].size()));
                for buffer in buffers {
                    self.node_data
                        .get_mut(&id)
                        .unwrap()
                        .get_buffers_mut()
                        .insert(buffer.slot(), buffer.buffer());
                }
            }
            // self.node_data[&id] = buffers;
        }

        for edge in &self.edges {
            if !started_nodes.contains(&edge.input_id) {
                queued_ids.push_back(edge.input_id);
                started_nodes.insert(edge.input_id);
            }
        }
    }

    // pub fn get_output_u8(&self, id: NodeId) -> Vec<u8> {
    //     self.node_data[&id]
    //         .iter()
    //         .map(|node_data| &node_data.value)
    //         .flatten()
    //         .map(|x| (x * 255.).min(255.) as u8)
    //         .collect()
    // }

    pub fn get_output_rgba(&self, id: NodeId) -> Result<Vec<u8>> {
        let buffers = self.node_data[&id].get_buffers();

        let empty_buffer: Buffer = ImageBuffer::new(0, 0);
        let mut sorted_value_vecs: Vec<&Buffer> = Vec::with_capacity(4);
        sorted_value_vecs.push(&empty_buffer);
        sorted_value_vecs.push(&empty_buffer);
        sorted_value_vecs.push(&empty_buffer);
        sorted_value_vecs.push(&empty_buffer);

        for (slot, buffer) in buffers {
            match slot {
                Slot(0) => sorted_value_vecs[0] = &buffer,
                Slot(1) => sorted_value_vecs[1] = &buffer,
                Slot(2) => sorted_value_vecs[2] = &buffer,
                Slot(3) => sorted_value_vecs[3] = &buffer,
                _ => continue,
            }
        }

        for value_vec in &sorted_value_vecs {
            if value_vec.is_empty() {
                panic!("Too few channels when trying to output rgba image");
            }
        }

        channels_to_rgba(&sorted_value_vecs)
    }

    fn new_id(&mut self) -> NodeId {
        loop {
            let id: NodeId = NodeId::new(rand::random());
            if !self.nodes.contains_key(&id) {
                return id;
            }
        }
    }

    pub fn get_root_ids(&self) -> Vec<NodeId> {
        self.nodes
            .keys()
            .filter(|node_id| {
                self.edges
                    .iter()
                    .map(|edge| edge.output_id)
                    .any(|x| x == **node_id)
            })
            .cloned()
            .collect::<Vec<NodeId>>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholder() {
        ()
    }
}
