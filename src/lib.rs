// TODO:
// Add output node
// Panic when input/output rules are not followed
// Clean up the code, add error handling and so on
// Make the input node into a single node with 4 (or however many channels) outputs
// Implement read and write nodes
// Implement tests
// Implement GUI

extern crate image;
extern crate rand;

use image::{DynamicImage, GenericImageView, ImageBuffer};
use rand::prelude;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::Path,
    sync::{mpsc, Arc},
    thread,
};

struct TextureProcessor {
    nodes: HashMap<NodeId, Arc<Node>>,
    node_data: HashMap<NodeId, Vec<Arc<NodeData>>>,
    edges: Vec<Edge>,
}

struct Edge {
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
}

#[derive(Debug)]
struct NodeData {
    slot: Slot,
    width: u32,
    height: u32,
    value: Vec<ChannelPixel>,
}

impl NodeData {
    fn new(slot: Slot) -> Self {
        Self {
            slot,
            width: 0,
            height: 0,
            value: Vec::new(),
        }
    }

    fn with_content(slot: Slot, width: u32, height: u32, value: &[ChannelPixel]) -> Self {
        Self {
            slot,
            width,
            height,
            value: value.to_vec(),
        }
    }
}

type ChannelPixel = f64;

impl TextureProcessor {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            node_data: HashMap::new(),
            edges: Vec::new(),
        }
    }

    fn add_node_internal(&mut self, node_type: NodeType, id: NodeId) -> NodeId {
        let node = Node(node_type);

        self.nodes.insert(id, Arc::new(node));
        id
    }

    pub fn add_node(&mut self, node_type: NodeType) -> NodeId {
        if node_type == NodeType::Input {
            panic!("Use the `add_input_node` function when adding an input node");
        }
        let id = self.new_id();
        self.add_node_internal(node_type, id)
    }

    pub fn add_node_with_id(&mut self, node_type: NodeType, id: NodeId) -> NodeId {
        self.add_node_internal(node_type, id)
    }

    pub fn add_input_node(&mut self, input: DynamicImage) -> NodeId {
        let mut id = self.new_id();

        self.add_node_internal(NodeType::Input, id);
        self.node_data.insert(id, Self::deconstruct_image(input));

        id
    }

    fn deconstruct_image(image: DynamicImage) -> Vec<Arc<NodeData>> {
        let raw_pixels = image.raw_pixels();
        let channel_count = raw_pixels.len() / (image.width() as usize * image.height() as usize);
        let mut node_data_vec = Vec::with_capacity(channel_count);

        for x in 0..channel_count {
            node_data_vec.push(NodeData::new(Slot(x)));
        }

        let mut current_channel = 0;

        for component in raw_pixels.into_iter() {
            node_data_vec[current_channel]
                .value
                .push(component as f64 / 255.);
            current_channel = (current_channel + 1) % channel_count;
        }

        node_data_vec
            .into_iter()
            .map(|node_data| Arc::new(node_data))
            .collect()
    }

    pub fn connect(&mut self, id_1: NodeId, id_2: NodeId, slot_1: Slot, slot_2: Slot) {
        if !self.nodes.contains_key(&id_1) || !self.nodes.contains_key(&id_2) {
            panic!("Tried connecting to a node that doesn't exist");
        }

        if self.slot_occupied(id_2, Side::Input, slot_2) {
            panic!("Tried adding an input to an occupied input slot");
        }

        self.edges.push(Edge::new(id_1, id_2, slot_1, slot_2));
    }

    pub fn slot_vacant(&self, id: NodeId, side: Side, slot: Slot) -> bool {
        !self.slot_occupied(id, side, slot)
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

    // fn reversed_edges(&self) -> HashMap<NodeId, Vec<Edge>> {
    //     let mut reversed_edges: HashMap<NodeId, Vec<Edge>> =
    //         HashMap::with_capacity(self.edges.len());

    //     for key in self.edges.keys() {
    //         reversed_edges.insert(*key, Vec::new());
    //     }

    //     for (id, edges) in self.edges.iter() {
    //         for edge in edges {
    //             let reversed_edge = Edge::new(edge.input_slot, *id, edge.output_slot);
    //             reversed_edges.entry(edge.input_id).and_modify(|e| e.push(reversed_edge));
    //         }
    //     }

    //     reversed_edges
    // }

    pub fn process(&mut self) {
        struct ThreadMessage {
            node_id: NodeId,
            node_data: Vec<Arc<NodeData>>,
        }

        let (send, recv) = mpsc::channel::<ThreadMessage>();
        let mut finished_nodes: HashSet<NodeId> = HashSet::with_capacity(self.nodes.len());
        let mut started_nodes: HashSet<NodeId> = HashSet::with_capacity(self.nodes.len());

        let mut queued_ids: VecDeque<NodeId> = VecDeque::from(self.get_root_ids());
        println!("queued_ids: {:?}", queued_ids);
        // panic!("end");
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

            // println!("queued_ids: {:?}", queued_ids);

            let current_id = match queued_ids.pop_front() {
                Some(id) => id,
                None => continue,
            };

            println!("self.node_data.keys(): {:?}", self.node_data.keys());
            println!("current_id: {:?}", current_id);
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

            println!("started_nodes: {:?}", started_nodes);
            let parent_ids = self
                .edges
                .iter()
                .filter(|edge| edge.input_id == current_id)
                .map(|edge| edge.output_id);

            for id in parent_ids {
                // println!("id: {:?}", id);
                // println!("finished_nodes: {:?}", finished_nodes);
                if !finished_nodes.contains(&id) {
                    queued_ids.push_back(current_id);
                    continue 'outer;
                }
            }
            // panic!("yes");
            println!("Started node: {:?}", current_id);

            // let input_data: Vec<Arc<NodeData>> = reversed_edges
            //     .iter()
            //     .map(|id| self.node_data.get(id).unwrap())
            //     .flatten()
            //     .map(|node_data| Arc::clone(node_data))
            //     .collect();

            // let input_data: Vec<Arc<NodeData>> =
            //     reversed_edges.get(current_id).unwrap()
            //     .iter()
            //     .map(|edge| self.node_data.get(edge.input_id).unwrap())

            // let input_data: Vec<Arc<NodeData>> =
            //     self.edges
            //     .iter()
            //     .filter(|edge| edge.input_id == current_id)
            //     .map(|edge| self.node_data.get(edge.input_id))
            //     .filter(|node_id| {
            //         self.edges.iter().any(|edge| edge.output_slot == node_id.slot)
            //     }

            // let mut input_data: Vec<Arc<NodeData>>;
            // let input_data: Vec<Arc<NodeData>> = self.edges.iter()
            //     .filter(|edge| edge.input_id == current_id)
            //     .map(|edge| {
            //         // let node_data_vec = self.node_data.get(&edge.input_id).unwrap();
            //         self.node_data.get(&edge.input_id).unwrap().iter()
            //             .filter(|node_data| node_data.slot == edge.output_slot)
            //             .map(|node_data| Arc::clone(node_data))
            //     })
            //     .flatten()
            //     .collect();

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

            let mut input_data: Vec<Arc<NodeData>> = Vec::new();
            for (id, data_vec) in self.node_data.iter() {
                if !relevant_ids.contains(&id) {
                    continue;
                }
                // let slots = data_vec.iter().map(|data| data.slot);
                for edge in &self.edges {
                    for data in data_vec.iter() {
                        if data.slot == edge.output_slot {
                            input_data.push(Arc::clone(data));
                        }
                    }
                }
            }

            // let input_data: Vec<Arc<NodeData>> = parent_ids
            //     .iter()
            //     .map(|id| self.node_data.get(id).unwrap())
            //     .flatten()
            //     .map(|node_data| Arc::clone(node_data))
            //     .collect();

            println!("input_data: {:?}", input_data.len());

            let current_node = Arc::clone(self.nodes.get(&current_id).unwrap());
            let send = send.clone();

            thread::spawn(move || {
                let node_data = current_node.process(&input_data).unwrap();
                match send.send(ThreadMessage {
                    node_id: current_id,
                    node_data,
                }) {
                    Ok(_) => (),
                    Err(e) => println!("{:?}", e),
                };
            });
        }
    }

    pub fn id_hashmap_from_edge_hashmap(
        edges: &HashMap<NodeId, Vec<Edge>>,
    ) -> HashMap<NodeId, Vec<NodeId>> {
        let mut output = HashMap::with_capacity(edges.len());

        for (id, edge) in edges {
            output.insert(*id, edge.iter().map(|edge| edge.input_id).collect());
        }

        output
    }

    fn set_node_finished(
        &mut self,
        id: NodeId,
        data: Option<Vec<Arc<NodeData>>>,
        started_nodes: &mut HashSet<NodeId>,
        finished_nodes: &mut HashSet<NodeId>,
        queued_ids: &mut VecDeque<NodeId>,
    ) {
        println!("Finished node: {:?}", id);
        finished_nodes.insert(id);

        if let Some(x) = data {
            self.node_data.insert(id, x);
        }

        for edge in self.edges.iter() {
            if !started_nodes.contains(&edge.input_id) {
                queued_ids.push_back(*&edge.input_id);
                started_nodes.insert(*&edge.input_id);
            }
        }
    }

    // pub fn get_output(&self, id: NodeId) -> &NodeData {
    //     &self.node_data.get(&id).unwrap()
    // }

    pub fn get_output_u8(&self, id: NodeId) -> Vec<u8> {
        self.node_data
            .get(&id)
            .unwrap()
            .iter()
            .map(|node_data| &node_data.value)
            .flatten()
            .map(|x| (x * 255.).min(255.) as u8)
            .collect()
    }

    fn new_id(&mut self) -> NodeId {
        loop {
            let id: NodeId = NodeId(rand::random());
            if !self.nodes.contains_key(&id) {
                return id;
            }
        }
    }

    fn get_root_ids(&self) -> Vec<NodeId> {
        // self.reversed_edges()
        //     .iter()
        //     .filter(|(_, v)| v.is_empty())
        //     .map(|(k, _)| *k)
        //     .collect()

        self.nodes
            .keys()
            .filter(|node_id| {
                self.edges
                    .iter()
                    .map(|edge| edge.output_id)
                    .collect::<Vec<NodeId>>()
                    .contains(node_id)
            }).map(|node_id| *node_id)
            .collect::<Vec<NodeId>>()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Slot(usize);

impl Slot {
    fn as_usize(&self) -> usize {
        self.0
    }
}

enum Side {
    Input,
    Output,
}

#[derive(Debug, PartialEq)]
pub enum NodeType {
    Input,
    Output,
    Add,
    Multiply,
}

#[derive(Debug)]
struct Node(NodeType);

impl Node {
    pub fn process(&self, input: &[Arc<NodeData>]) -> Option<Vec<Arc<NodeData>>> {
        println!(
            "self.capacity(Side::Input): {:?}",
            self.capacity(Side::Input)
        );
        println!("input.len(): {:?}", input.len());
        assert!(input.len() <= self.capacity(Side::Input));

        let output: Vec<Arc<NodeData>> = match self.0 {
            NodeType::Input => return None,
            NodeType::Output => Self::output(input),
            NodeType::Add => Self::add(&input[0], &input[1]),
            NodeType::Multiply => Self::multiply(&input[0], &input[1]),
        };

        assert!(output.len() <= self.capacity(Side::Output));
        Some(output)
    }

    pub fn capacity(&self, side: Side) -> usize {
        match side {
            Input => match self.0 {
                NodeType::Input => 0,
                NodeType::Output => 4,
                NodeType::Add => 2,
                NodeType::Multiply => 2,
            },
            Output => match self.0 {
                NodeType::Input => 1,
                NodeType::Output => 4,
                NodeType::Add => 1,
                NodeType::Multiply => 1,
            },
        }
    }

    fn output(inputs: &[Arc<NodeData>]) -> Vec<Arc<NodeData>> {
        let mut outputs: Vec<Arc<NodeData>> = Vec::with_capacity(inputs.len());

        for input in inputs {
            outputs.push(Arc::clone(input));
        }

        outputs
    }

    fn add(input_0: &NodeData, input_1: &NodeData) -> Vec<Arc<NodeData>> {
        let data: Vec<ChannelPixel> = input_0
            .value
            .iter()
            .zip(&input_1.value)
            .map(|(x, y)| x + y)
            .collect();
        vec![Arc::new(NodeData::with_content(
            Slot(0),
            input_0.width,
            input_0.height,
            &data,
        ))]
    }

    fn multiply(input_0: &NodeData, input_1: &NodeData) -> Vec<Arc<NodeData>> {
        let data: Vec<ChannelPixel> = input_0
            .value
            .iter()
            .zip(&input_1.value)
            .map(|(x, y)| x * y)
            .collect();
        vec![Arc::new(NodeData::with_content(
            Slot(0),
            input_0.width,
            input_0.height,
            &data,
        ))]
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

        let image_0 = image::open(&Path::new(&"data/image_1.png")).unwrap();
        // let image_1 = image::open(&Path::new(&"data/image_2.png")).unwrap();
        // let image_2 = image::open(&Path::new(&"data/heart_256.png"))
        //     .unwrap();
        // let image_3 = image::open(&Path::new(&"data/heart_256.png"))
        //     .unwrap();

        // for id in nodes {
        //     match image::save_buffer(
        //         &Path::new(&format!("out/{:?}.png", id)),
        //         &image::GrayImage::from_vec(256, 256, tex_pro.get_output_u8(id)).unwrap(),
        //         256,
        //         256,
        //         image::ColorType::Gray(8),
        //     ) {
        //         Ok(_) => (),
        //         Err(e) => println!("Error when writing buffer: {:?}", e),
        //     };
        // }

        let input_node = tex_pro.add_input_node(image_0);
        let output_node = tex_pro.add_node_with_id(NodeType::Output, NodeId(12));
        // input_nodes.append(&mut tex_pro.add_input_node(image_1));
        // let node_4 = tex_pro.add_node_with_id(NodeType::Add, NodeId(4));
        // let node_5 = tex_pro.add_node(NodeType::Add);
        // let node_6 = tex_pro.add_node(NodeType::Multiply);
        // let node_7 = tex_pro.add_node(NodeType::Add);

        tex_pro.connect(input_node, output_node, Slot(0), Slot(0));
        tex_pro.connect(input_node, output_node, Slot(1), Slot(1));
        tex_pro.connect(input_node, output_node, Slot(2), Slot(2));
        tex_pro.connect(input_node, output_node, Slot(3), Slot(3));

        // tex_pro.connect(input_node, node_4, Slot(0), Slot(0));
        // tex_pro.connect(input_node, node_4, Slot(0), Slot(1));
        // tex_pro.connect(input_nodes[3], node_4, Slot(0), Slot(0));
        // tex_pro.connect(node_1, node_4);
        // tex_pro.connect(node_1, node_5);
        // tex_pro.connect(node_2, node_5);
        // tex_pro.connect(node_5, node_6);
        // tex_pro.connect(node_4, node_6);
        // tex_pro.connect(node_6, node_7);
        // tex_pro.connect(node_3, node_7);

        tex_pro.process();

        image::save_buffer(
            &Path::new(&"out/output.png"),
            &image::GrayImage::from_vec(256, 256, tex_pro.get_output_u8(output_node)).unwrap(),
            256,
            256,
            image::ColorType::Gray(8),
        ).unwrap();
        // image::save_buffer(
        //     &Path::new(&"out/chan_r.png"),
        //     &image::GrayImage::from_vec(256, 256, tex_pro.get_output_u8(input_node)).unwrap(),
        //     256,
        //     256,
        //     image::ColorType::Gray(8),
        // ).unwrap();
        // image::save_buffer(
        //     &Path::new(&"out/chan_a.png"),
        //     &image::GrayImage::from_vec(256, 256, tex_pro.get_output_u8(input_node)).unwrap(),
        //     256,
        //     256,
        //     image::ColorType::Gray(8),
        // ).unwrap();

        // image::save_buffer(
        //     &Path::new(&"out/node_0.png"),
        //     &image::GrayImage::from_vec(256, 256, tex_pro.get_output_u8(node_0)).unwrap(),
        //     256,
        //     256,
        //     image::ColorType::Gray(8),
        // ).unwrap();
        // image::save_buffer(
        //     &Path::new(&"out/node_1.png"),
        //     &image::GrayImage::from_vec(256, 256, tex_pro.get_output_u8(node_1)).unwrap(),
        //     256,
        //     256,
        //     image::ColorType::Gray(8),
        // ).unwrap();
        // image::save_buffer(
        //     &Path::new(&"out/node_2.png"),
        //     &image::GrayImage::from_vec(256, 256, tex_pro.get_output_u8(node_2)).unwrap(),
        //     256,
        //     256,
        //     image::ColorType::Gray(8),
        // ).unwrap();
        // image::save_buffer(
        //     &Path::new(&"out/node_3.png"),
        //     &image::GrayImage::from_vec(256, 256, tex_pro.get_output_u8(node_3)).unwrap(),
        //     256,
        //     256,
        //     image::ColorType::Gray(8),
        // ).unwrap();
        // image::save_buffer(
        //     &Path::new(&"out/node_4.png"),
        //     &image::GrayImage::from_vec(256, 256, tex_pro.get_output_u8(node_4)).unwrap(),
        //     256,
        //     256,
        //     image::ColorType::Gray(8),
        // ).unwrap();
        // image::save_buffer(
        //     &Path::new(&"out/node_5.png"),
        //     &image::GrayImage::from_vec(256, 256, tex_pro.get_output_u8(node_5)).unwrap(),
        //     256,
        //     256,
        //     image::ColorType::Gray(8),
        // ).unwrap();
        // image::save_buffer(
        //     &Path::new(&"out/node_6.png"),
        //     &image::GrayImage::from_vec(256, 256, tex_pro.get_output_u8(node_6)).unwrap(),
        //     256,
        //     256,
        //     image::ColorType::Gray(8),
        // ).unwrap();
        // image::save_buffer(
        //     &Path::new(&"out/node_7.png"),
        //     &image::GrayImage::from_vec(256, 256, tex_pro.get_output_u8(node_7)).unwrap(),
        //     256,
        //     256,
        //     image::ColorType::Gray(8),
        // ).unwrap();
    }
}
