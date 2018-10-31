// TODO:
// Break inputs down into channels and process only channels
// Use only the simplest nodes possible that operate only on for instance two channels
// Panic when input/output rules are not followed
// Clean up the code, add error handling and so on
// Implement read and write nodes
// Implement tests
// Implement GUI

extern crate image;
extern crate rand;

use image::{GenericImageView, DynamicImage, ImageBuffer};
use rand::prelude;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::Path,
    sync::{mpsc, Arc},
    thread,
};

struct TextureProcessor {
    nodes: HashMap<NodeId, Arc<Node>>,
    node_data: HashMap<NodeId, Arc<NodeData>>,
    edges: HashMap<NodeId, Vec<NodeId>>,
}

type ChannelPixel = f64;

impl TextureProcessor {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            node_data: HashMap::new(),
            edges: HashMap::new(),
        }
    }

    pub fn add_node(&mut self, node_type: NodeType) -> NodeId {
        if node_type == NodeType::Input {
            panic!("Use the `add_input` function when adding an input node");
        }
        self.add_node_internal(node_type)
    }

    pub fn add_inputs(&mut self, input: DynamicImage) -> Vec<NodeId> {
        let node_data_vec = Self::deconstruct_image(input);
        let ids = Vec::new();

        for node_data in node_data_vec {
            let id = self.add_node_internal(NodeType::Input);
            self.node_data.insert(id, Arc::new(node_data));
            ids.push(id);
        }

        ids
    }

    fn deconstruct_image(image: DynamicImage) -> Vec<NodeData> {
        let mut raw_pixels = image.raw_pixels();
        let channel_count = raw_pixels.len() / (image.width() as usize * image.height() as usize);
        let mut node_data_vec = Vec::with_capacity(channel_count);

        for channel in 0..channel_count {
            node_data_vec.push(NodeData::new());
        }

        let mut current_channel = 0;

        for component in raw_pixels.into_iter() {
            node_data_vec[current_channel].value.push(component);
            current_channel = current_channel % channel_count;
        }

        node_data_vec
    }

    fn add_node_internal(&mut self, node_type: NodeType) -> NodeId {
        let node = Node{ node_type };

        let id = self.new_id();
        self.nodes.insert(id, Arc::new(node));
        self.edges.insert(id, Vec::new());
        id
    }

    pub fn connect(&mut self, id_1: NodeId, id_2: NodeId) {
        if !self.nodes.contains_key(&id_1) || !self.nodes.contains_key(&id_2) {
            return;
        }

        self.edges
            .get_mut(&id_1)
            .map(|connections| connections.push(id_2));
    }

    fn reversed_edges(&self) -> HashMap<NodeId, Vec<NodeId>> {
        let mut reversed_edges: HashMap<NodeId, Vec<NodeId>> =
            HashMap::with_capacity(self.edges.len());

        for key in self.edges.keys() {
            reversed_edges.insert(*key, Vec::new());
        }

        for (id, target_ids) in self.edges.iter() {
            for target_id in target_ids {
                reversed_edges.entry(*target_id).and_modify(|e| e.push(*id));
            }
        }
        reversed_edges
    }

    pub fn process(&mut self) {
        struct ThreadMessage {
            node_id: NodeId,
            node_data: NodeData,
        }

        let reversed_edges: HashMap<NodeId, Vec<NodeId>> = self.reversed_edges();

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
                    message.node_id,
                    Some(message.node_data),
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

            let parent_ids = reversed_edges.get(&current_id).unwrap();
            for id in parent_ids {
                if !finished_nodes.contains(id) {
                    queued_ids.push_back(current_id);
                    continue 'outer;
                }
            }
            println!("Started node: {:?}", current_id);

            let input_data: Vec<Arc<NodeData>> = parent_ids
                .iter()
                .map(|id| Arc::clone(self.node_data.get(id).unwrap()))
                .collect();
            let current_node = Arc::clone(self.nodes.get(&current_id).unwrap());
            let send = send.clone();

            thread::spawn(move || {
                let node_data = current_node.process(&input_data).unwrap();
                send.send(ThreadMessage {
                    node_id: current_id,
                    node_data,
                }).unwrap();
            });
        }
    }

    fn set_node_finished(
        &mut self,
        id: NodeId,
        data: Option<NodeData>,
        started_nodes: &mut HashSet<NodeId>,
        finished_nodes: &mut HashSet<NodeId>,
        queued_ids: &mut VecDeque<NodeId>,
    ) {
        println!("Finished node: {:?}", id);
        finished_nodes.insert(id);

        if let Some(x) = data {
            self.node_data.insert(id, Arc::new(x));
        }

        for child_id in self.edges.get(&id).unwrap() {
            if !started_nodes.contains(child_id) {
                queued_ids.push_back(*child_id);
                started_nodes.insert(*child_id);
            }
        }
    }

    pub fn get_output(&self, id: NodeId) -> &NodeData {
        &self.node_data.get(&id).unwrap()
    }

    fn new_id(&mut self) -> NodeId {
        loop {
            let id: NodeId = NodeId(rand::random());
            if !self.nodes.contains_key(&id) {
                return id
            }
        }
    }

    fn get_root_ids(&self) -> Vec<NodeId> {
        self.reversed_edges()
            .iter()
            .filter(|(_, v)| v.is_empty())
            .map(|(k, _)| *k)
            .collect()
    }
}

#[derive(PartialEq)]
pub enum NodeType {
    Input,
    Add,
    Multiply,
}

struct Node {
    node_type: NodeType,
}

struct NodeData {
    width: u32,
    height: u32,
    value: Vec<ChannelPixel>,
}

impl NodeData {
    // fn new(width: u32, height: u32, value: &[ChannelPixel]) -> Self {
    fn new() -> Self {
        Self {
            width: 0,
            height: 0,
            value: Vec::new(),
        }
    }

    fn with_content(width: u32, height: u32, value: &[ChannelPixel]) -> Self {
        Self {
            width,
            height,
            value: value.to_vec(),
        }
    }
}

impl Node {
    pub fn new(node_type: NodeType) -> Self {
        Node { node_type }
    }

    pub fn process(&self, input: &[Arc<NodeData>]) -> Option<NodeData> {
        match self.node_type {
            NodeType::Input => None,
            NodeType::Add => Self::add(&input[0], &input[1]),
            NodeType::Multiply => Self::multiply(&input[0], &input[1]),
        }
    }

    fn add(input_0: &NodeData, input_1: &NodeData) -> Option<NodeData> {
        let data: Vec<ChannelPixel> = input_0.value.iter().zip(input_1.value).map(|(x, y)| x + y).collect();
        Some( NodeData::with_content(input_0.width, input_0.height, &data) )
    }

    fn multiply(input_0: &NodeData, input_1: &NodeData) -> Option<NodeData> {
        let data: Vec<ChannelPixel> = input_0.value.iter().zip(input_1.value).map(|(x, y)| x * y).collect();
        Some( NodeData::with_content(input_0.width, input_0.height, &data) )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
struct NodeId(u32);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integration_test() {
        let mut tex_pro = TextureProcessor::new();

        let image_0 = image::open(&Path::new(&"data/image_1.png"))
            .unwrap();
        let image_1 = image::open(&Path::new(&"data/image_2.png"))
            .unwrap();
        let image_2 = image::open(&Path::new(&"data/heart_256.png"))
            .unwrap();
        let image_3 = image::open(&Path::new(&"data/heart_256.png"))
            .unwrap();

        let node_0 = tex_pro.add_inputs(image_0)[0];
        let node_1 = tex_pro.add_inputs(image_1)[0];
        let node_2 = tex_pro.add_inputs(image_2)[0];
        let node_3 = tex_pro.add_inputs(image_3)[0];
        let node_4 = tex_pro.add_node(NodeType::Add);
        let node_5 = tex_pro.add_node(NodeType::Add);
        let node_6 = tex_pro.add_node(NodeType::Multiply);
        let node_7 = tex_pro.add_node(NodeType::Add);

        tex_pro.connect(node_0, node_4);
        tex_pro.connect(node_1, node_4);
        tex_pro.connect(node_1, node_5);
        tex_pro.connect(node_2, node_5);
        tex_pro.connect(node_5, node_6);
        tex_pro.connect(node_4, node_6);
        tex_pro.connect(node_6, node_7);
        tex_pro.connect(node_3, node_7);

        tex_pro.process();

        image::save_buffer(
            &Path::new(&"out/node_0.png"),
            &tex_pro.get_output(node_0).value,
            256,
            256,
            image::ColorType::RGBA(8),
        ).unwrap();
        image::save_buffer(
            &Path::new(&"out/node_1.png"),
            &tex_pro.get_output(node_1).value,
            256,
            256,
            image::ColorType::RGBA(8),
        ).unwrap();
        image::save_buffer(
            &Path::new(&"out/node_2.png"),
            &tex_pro.get_output(node_2).value,
            256,
            256,
            image::ColorType::RGBA(8),
        ).unwrap();
        image::save_buffer(
            &Path::new(&"out/node_3.png"),
            &tex_pro.get_output(node_3).value,
            256,
            256,
            image::ColorType::RGBA(8),
        ).unwrap();
        image::save_buffer(
            &Path::new(&"out/node_4.png"),
            &tex_pro.get_output(node_4).value,
            256,
            256,
            image::ColorType::RGBA(8),
        ).unwrap();
        image::save_buffer(
            &Path::new(&"out/node_5.png"),
            &tex_pro.get_output(node_5).value,
            256,
            256,
            image::ColorType::RGBA(8),
        ).unwrap();
        image::save_buffer(
            &Path::new(&"out/node_6.png"),
            &tex_pro.get_output(node_6).value,
            256,
            256,
            image::ColorType::RGBA(8),
        ).unwrap();
        image::save_buffer(
            &Path::new(&"out/node_7.png"),
            &tex_pro.get_output(node_7).value,
            256,
            256,
            image::ColorType::RGBA(8),
        ).unwrap();
    }
}
