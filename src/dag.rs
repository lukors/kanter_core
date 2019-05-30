use crate::{
    error::Result,
    node_data::*,
    node_graph::*,
    process::*,
};
use image::ImageBuffer;
use std::{
    collections::{HashSet, VecDeque},
    sync::{mpsc, Arc},
    thread,
};

use crate::node::*;
use crate::shared::*;

#[derive(Default)]
pub struct TextureProcessor {
    pub node_graph: NodeGraph,
    pub node_datas: Vec<Arc<NodeData>>,
}

impl TextureProcessor {
    pub fn new() -> Self {
        Self {
            node_graph: NodeGraph::new(),
            node_datas: Vec::new(),
        }
    }

    pub fn process(&mut self) {
        struct ThreadMessage {
            node_id: NodeId,
            node_datas: Vec<Arc<NodeData>>,
        }

        let (send, recv) = mpsc::channel::<ThreadMessage>();
        let mut finished_nodes: HashSet<NodeId> = HashSet::with_capacity(self.node_graph.nodes.len());
        let mut started_nodes: HashSet<NodeId> = HashSet::with_capacity(self.node_graph.nodes.len());

        let mut queued_ids: VecDeque<NodeId> = VecDeque::from(self.get_root_ids());
        for item in &queued_ids {
            started_nodes.insert(*item);
        }

        'outer: while finished_nodes.len() < self.node_graph.nodes.len() {
            for message in recv.try_iter() {
                self.set_node_finished(
                    message.node_id,
                    &mut Some(message.node_datas),
                    &mut started_nodes,
                    &mut finished_nodes,
                    &mut queued_ids,
                );
            }

            let current_id = match queued_ids.pop_front() {
                Some(id) => id,
                None => continue,
            };

            if self.node_datas.iter().any(|x| x.node_id == current_id) {
                self.set_node_finished(
                    current_id,
                    &mut None,
                    &mut started_nodes,
                    &mut finished_nodes,
                    &mut queued_ids,
                );
                continue;
            }

            let parent_ids = self.node_graph.edges
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
            for node_data in &self.node_datas {
                for edge in &self.node_graph.edges {
                    if edge.output_id != node_data.node_id {
                        continue;
                    } else {
                        relevant_ids.push(node_data.node_id);
                    }
                }
            }

            // Put the `Arc<Buffer>`s and `Edge`s relevant for the calculation of this node into
            // lists.
            let mut relevant_edges: Vec<Edge> = Vec::new();
            let mut input_data: Vec<Arc<NodeData>> = Vec::new();
            for node_data in &self.node_datas {
                if !relevant_ids.contains(&node_data.node_id) {
                    continue;
                }
                for edge in &self.node_graph.edges {
                    if node_data.slot_id == edge.output_slot
                        && node_data.node_id == edge.output_id
                        && current_id == edge.input_id
                    {
                        input_data.push( Arc::clone(node_data) );
                        relevant_edges.push( edge.clone() );
                    }
                }
            }

            // Spawn a thread and calculate the node in it and send back the new `node_data`s for
            // each slot in the node.
            let current_node = Arc::clone(&self.node_graph.nodes[&current_id]);
            let send = send.clone();

            thread::spawn(move || {
                let node_datas: Vec<Arc<NodeData>> = Self::process_node(current_node, &mut input_data, relevant_edges);

                match send.send(ThreadMessage {
                    node_id: current_id,
                    node_datas,
                }) {
                    Ok(_) => (),
                    Err(e) => println!("{:?}", e),
                };
            });
        }
    }

    fn process_node(node: Arc<Node>, data: &mut Vec<Arc<NodeData>>, edges: Vec<Edge>) -> Vec<Arc<NodeData>> {
        unimplemented!()
    }

    /// Takes a node and the data it generated, marks it as finished and puts the data in the
    /// `TextureProcessor`'s data vector.
    /// Then it adds any child `NodeId`s of the input `NodeId` to the list of `NodeId`s to process.
    fn set_node_finished(
        &mut self,
        id: NodeId,
        // For refactoring: Used to be `buffers: Option<Vec<Arc<Buffer>>>`:
        node_datas: &mut Option<Vec<Arc<NodeData>>>,
        started_nodes: &mut HashSet<NodeId>,
        finished_nodes: &mut HashSet<NodeId>,
        queued_ids: &mut VecDeque<NodeId>,
    ) {
        finished_nodes.insert(id);

        if let Some(node_datas) = node_datas {
            self.node_datas.append(node_datas);
            // self.node_datas.push(NodeData::new(node_datas[0].size()));
            // for node_data in node_datas {
                // self.node_datas.push(node_data);
                // self.node_datas
                //     .get_mut(&id)
                //     .unwrap()
                //     .get_buffers_mut()
                //     .insert(node_data.slot(), node_data.buffer());
            // }
            // self.node_datas[&id] = buffers;
        }

        // Add any child node to the input `NodeId` to the list of nodes to potentially process.
        for edge in &self.node_graph.edges {
            let input_id = edge.input_id;
            if edge.output_id == id
            && !started_nodes.contains(&input_id) {
                queued_ids.push_back(input_id);
                started_nodes.insert(input_id);
            }
        }
    }

    // pub fn get_output_u8(&self, id: NodeId) -> Vec<u8> {
    //     self.node_datas[&id]
    //         .iter()
    //         .map(|node_data| &node_data.value)
    //         .flatten()
    //         .map(|x| (x * 255.).min(255.) as u8)
    //         .collect()
    // }
    fn get_node_datas(&self, id: NodeId) -> Vec<Arc<NodeData>> {
        self.node_datas.iter().filter(|&x| x.node_id == id).map(|x| Arc::clone(x)).collect()
    }

    pub fn get_output_rgba(&self, id: NodeId) -> Result<Vec<u8>> {
        let node_datas = self.get_node_datas(id);

        let empty_buffer: Buffer = Box::new(ImageBuffer::new(0, 0));
        let mut sorted_value_vecs: Vec<&Buffer> = Vec::with_capacity(4);
        sorted_value_vecs.push(&empty_buffer);
        sorted_value_vecs.push(&empty_buffer);
        sorted_value_vecs.push(&empty_buffer);
        sorted_value_vecs.push(&empty_buffer);

        // for node_data in node_datas {
        //     match node_data.slot_id {
        //         SlotId(0) => sorted_value_vecs[0] = &node_data.buffer,
        //         SlotId(1) => sorted_value_vecs[1] = &node_data.buffer,
        //         SlotId(2) => sorted_value_vecs[2] = &node_data.buffer,
        //         SlotId(3) => sorted_value_vecs[3] = &node_data.buffer,
        //         _ => continue,
        //     }
        // }

        sorted_value_vecs = node_datas.iter().map(|node_data| {
            match node_data.slot_id {
                SlotId(0) => &node_data.buffer,
                SlotId(1) => &node_data.buffer,
                SlotId(2) => &node_data.buffer,
                SlotId(3) => &node_data.buffer,
                _ => &empty_buffer,
            }
        }).collect();

        for value_vec in &sorted_value_vecs {
            if value_vec.is_empty() {
                panic!("Too few channels when trying to output rgba image");
            }
        }

        let sorted_value_vecs_refs: Vec<&Buffer> = sorted_value_vecs.iter().map(|buf| *buf).collect();
        channels_to_rgba(&sorted_value_vecs_refs)
    }

    pub fn get_root_ids(&self) -> Vec<NodeId> {
        self.node_graph.nodes
            .keys()
            .filter(|node_id| {
                self.node_graph.edges
                    .iter()
                    .map(|edge| edge.output_id)
                    .any(|x| x == **node_id)
            })
            .cloned()
            .collect::<Vec<NodeId>>()
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn placeholder() {
//         ()
//     }
// }
