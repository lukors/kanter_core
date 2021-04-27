use crate::{
    error::{Result, TexProError},
    node::{EmbeddedNodeDataId, NodeType},
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

use crate::shared::*;

#[derive(Default)]
pub struct TextureProcessor {
    pub node_graph: NodeGraph,
    pub node_datas: Vec<Arc<NodeData>>,
    pub embedded_node_datas: Vec<Arc<EmbeddedNodeData>>,
    pub input_node_datas: Vec<Arc<NodeData>>,
}

impl TextureProcessor {
    pub fn new() -> Self {
        Self {
            node_graph: NodeGraph::new(),
            node_datas: Vec::new(),
            embedded_node_datas: Vec::new(),
            input_node_datas: Vec::new(),
        }
    }

    pub fn process(&mut self) {
        struct ThreadMessage {
            node_id: NodeId,
            node_datas: Result<Vec<Arc<NodeData>>>,
        }

        self.node_datas.clear();

        let (send, recv) = mpsc::channel::<ThreadMessage>();
        let mut finished_nodes: HashSet<NodeId> =
            HashSet::with_capacity(self.node_graph.nodes().len());
        let mut started_nodes: HashSet<NodeId> =
            HashSet::with_capacity(self.node_graph.nodes().len());

        let mut queued_ids: VecDeque<NodeId> = VecDeque::from(self.get_root_ids());
        for item in &queued_ids {
            started_nodes.insert(*item);
        }

        if queued_ids.is_empty() {
            return;
        }

        'outer: while finished_nodes.len() < started_nodes.len() {
            for message in recv.try_iter() {
                self.set_node_finished(
                    message.node_id,
                    &mut Some(message.node_datas.unwrap()),
                    &mut started_nodes,
                    &mut finished_nodes,
                    &mut queued_ids,
                );
            }

            let current_id = match queued_ids.pop_front() {
                Some(id) => id,
                None => continue,
            };

            let parent_node_ids = self
                .node_graph
                .edges
                .iter()
                .filter(|edge| edge.input_id == current_id)
                .map(|edge| edge.output_id);

            for parent_node_id in parent_node_ids {
                if !finished_nodes.contains(&parent_node_id) {
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
                        input_data.push(Arc::clone(node_data));
                        relevant_edges.push(*edge);
                    }
                }
            }

            // Spawn a thread and calculate the node in it and send back the new `node_data`s for
            // each slot in the node.
            let current_node = self.node_graph.node_with_id(current_id).unwrap().clone();
            let send = send.clone();

            let embedded_node_datas: Vec<Arc<EmbeddedNodeData>> = self
                .embedded_node_datas
                .iter()
                .map(|end| Arc::clone(&end))
                .collect();
            let input_node_datas: Vec<Arc<NodeData>> = self
                .input_node_datas
                .iter()
                .map(|nd| Arc::clone(&nd))
                .collect();

            thread::spawn(move || {
                let node_datas: Result<Vec<Arc<NodeData>>> = process_node(
                    current_node,
                    &input_data,
                    &embedded_node_datas,
                    &input_node_datas,
                    &relevant_edges,
                );

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

    /// Returns the width and height of the `NodeData` for the given `NodeId` as a `Size`.
    pub fn get_node_size(&self, node_id: NodeId) -> Option<Size> {
        if let Some(node_data) = self.node_datas.iter().find(|nd| nd.node_id == node_id) {
            Some(node_data.size)
        } else {
            None
        }
    }

    /// Takes a node and the data it generated, marks it as finished and puts the data in the
    /// `TextureProcessor`'s data vector.
    /// Then it adds any child `NodeId`s of the input `NodeId` to the list of `NodeId`s to process.
    fn set_node_finished(
        &mut self,
        id: NodeId,
        node_datas: &mut Option<Vec<Arc<NodeData>>>,
        started_nodes: &mut HashSet<NodeId>,
        finished_nodes: &mut HashSet<NodeId>,
        queued_ids: &mut VecDeque<NodeId>,
    ) {
        finished_nodes.insert(id);

        if let Some(node_datas) = node_datas {
            self.node_datas.append(node_datas);
        }

        // Add any child node of the input `NodeId` to the list of nodes to potentially process.
        for edge in &self.node_graph.edges {
            let input_id = edge.input_id;
            if edge.output_id == id && !started_nodes.contains(&input_id) {
                queued_ids.push_back(input_id);
                started_nodes.insert(input_id);
            }
        }
    }

    /// Embeds a `NodeData` in the `TextureProcessor` with an associated `EmbeddedNodeDataId`.
    /// The `EmbeddedNodeDataId` can be referenced using the assigned `EmbeddedNodeDataId` in a
    /// `NodeType::NodeData` node. This is useful when you want to transfer and use 'NodeData'
    /// between several `TextureProcessor`s.
    ///
    /// Get the `NodeData`s from a `Node` in a `TextureProcessor` by using the `get_node_data()`
    /// function.
    pub fn embed_node_data_with_id(
        &mut self,
        node_data: Arc<NodeData>,
        id: EmbeddedNodeDataId,
    ) -> Result<EmbeddedNodeDataId> {
        if self
            .embedded_node_datas
            .iter()
            .all(|end| end.node_data_id != id)
        {
            self.embedded_node_datas
                .push(Arc::new(EmbeddedNodeData::from_node_data(node_data, id)));
            Ok(id)
        } else {
            Err(TexProError::InvalidSlotId)
        }
    }

    /// Gets all `NodeData`s in this `TextureProcessor`.
    pub fn node_datas(&self, id: NodeId) -> Vec<Arc<NodeData>> {
        self.node_datas
            .iter()
            .filter(|&x| x.node_id == id)
            .map(|x| Arc::clone(x))
            .collect()
    }

    /// Gets any `NodeData`s associated with a given `NodeId`.
    pub fn get_node_data(&self, id: NodeId) -> Vec<Arc<NodeData>> {
        self.node_datas
            .iter()
            .filter(|nd| nd.node_id == id)
            .map(|nd| Arc::clone(&nd))
            .collect()
    }

    pub fn get_output(&self, node_id: NodeId) -> Result<Vec<u8>> {
        let node_datas = self.node_datas(node_id);
        if node_datas.is_empty() {
            return Err(TexProError::Generic);
        }

        let output_vecs = match self
            .node_graph
            .node_with_id(node_id)
            .ok_or(TexProError::InvalidNodeId)?
            .node_type
        {
            NodeType::OutputRgba => self.get_output_rgba(&node_datas)?,
            NodeType::OutputGray => self.get_output_gray(&node_datas)?,
            _ => return Err(TexProError::InvalidNodeType),
        };

        channels_to_rgba(&output_vecs)
    }

    fn get_output_rgba(&self, node_datas: &[Arc<NodeData>]) -> Result<Vec<Arc<Buffer>>> {
        let empty_buffer: Arc<Buffer> = Arc::new(Box::new(ImageBuffer::new(0, 0)));
        let mut sorted_value_vecs: Vec<Arc<Buffer>> = vec![Arc::clone(&empty_buffer); 4];

        for node_data in node_datas.iter() {
            sorted_value_vecs[node_data.slot_id.0 as usize] = Arc::clone(&node_data.buffer);
        }

        let (width, height) = (node_datas[0].size.width, node_datas[0].size.height);
        let size = (width * height) as usize;

        for (i, value_vec) in sorted_value_vecs.iter_mut().enumerate() {
            if !value_vec.is_empty() {
                continue;
            }

            // Should be black if R, G or B channel, and white if A.
            let buf_vec = if i == 3 {
                vec![1.; size]
            } else {
                vec![0.; size]
            };

            *value_vec = Arc::new(Box::new(
                ImageBuffer::from_raw(width, height, buf_vec).ok_or(TexProError::Generic)?,
            ))
        }

        Ok(sorted_value_vecs)
    }

    fn get_output_gray(&self, node_datas: &[Arc<NodeData>]) -> Result<Vec<Arc<Buffer>>> {
        assert_eq!(node_datas.len(), 1);
        let (width, height) = (node_datas[0].size.width, node_datas[0].size.height);
        let size = (width * height) as usize;

        let mut sorted_value_vecs: Vec<Arc<Buffer>> = vec![Arc::clone(&node_datas[0].buffer); 3];
        sorted_value_vecs.push(Arc::new(Box::new(
            ImageBuffer::from_raw(width, height, vec![1.; size]).ok_or(TexProError::Generic)?,
        )));

        Ok(sorted_value_vecs)
    }

    pub fn get_root_ids(&self) -> Vec<NodeId> {
        self.node_graph
            .nodes()
            .iter()
            .filter(|node| {
                self.node_graph
                    .edges
                    .iter()
                    .map(|edge| edge.output_id)
                    .any(|x| x == node.node_id)
            })
            .map(|node| node.node_id)
            .collect::<Vec<NodeId>>()
    }
}
