// TODO:
// Clean up the code, add error handling and so on
// Make the input node into a single node with 4 (or however many channels) outputs
// Implement read and write nodes
// Implement tests
// Implement GUI

extern crate image;
extern crate rand;

use image::{DynamicImage, GenericImageView};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::{mpsc, Arc},
    thread,
};

struct TextureProcessor {
    nodes: HashMap<NodeId, Arc<Node>>,
    node_data: HashMap<NodeId, Vec<Arc<NodeData>>>,
    edges: Vec<Edge>,
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
struct NodeData {
    slot: Slot,
    width: u32,
    height: u32,
    value: Vec<ChannelPixel>,
}

impl NodeData {
    fn new(slot: Slot, width: u32, height: u32) -> Self {
        Self {
            slot,
            width,
            height,
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

    pub fn add_input_node(&mut self, input: &DynamicImage) -> NodeId {
        let id = self.new_id();

        self.add_node_internal(NodeType::Input, id);
        self.node_data.insert(id, Self::deconstruct_image(&input));

        id
    }

    fn deconstruct_image(image: &DynamicImage) -> Vec<Arc<NodeData>> {
        let raw_pixels = image.raw_pixels();
        let (width, height) = (image.width(), image.height());
        let channel_count = raw_pixels.len() / (width as usize * height as usize);
        let mut node_data_vec = Vec::with_capacity(channel_count);

        for x in 0..channel_count {
            node_data_vec.push(NodeData::new(Slot(x), width, height));
        }

        let mut current_channel = 0;

        for component in raw_pixels {
            node_data_vec[current_channel]
                .value
                .push(f64::from(component) / 255.);
            current_channel = (current_channel + 1) % channel_count;
        }

        node_data_vec.into_iter().map(Arc::new).collect()
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
            println!("Started node: {:?}", current_id);

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
            let mut input_data: Vec<(NodeId, Arc<NodeData>)> = Vec::new();
            for (id, data_vec) in &self.node_data {
                if !relevant_ids.contains(&id) {
                    continue;
                }
                for edge in &self.edges {
                    for data in data_vec.iter() {
                        if data.slot == edge.output_slot
                            && *id == edge.output_id
                            && current_id == edge.input_id
                        {
                            input_data.push((*id, Arc::clone(data)));
                            relevant_edges.push(edge.clone());
                        }
                    }
                }
            }

            let current_node = Arc::clone(&self.nodes[&current_id]);
            let send = send.clone();

            thread::spawn(move || {
                let node_data = current_node.process(&input_data, &relevant_edges).unwrap();
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

        for edge in &self.edges {
            if !started_nodes.contains(&edge.input_id) {
                queued_ids.push_back(edge.input_id);
                started_nodes.insert(edge.input_id);
            }
        }
    }

    pub fn get_output_u8(&self, id: NodeId) -> Vec<u8> {
        self.node_data[&id]
            .iter()
            .map(|node_data| &node_data.value)
            .flatten()
            .map(|x| (x * 255.).min(255.) as u8)
            .collect()
    }

    pub fn get_output_rgba(&self, id: NodeId) -> Vec<u8> {
        let node_data_vec = &self.node_data[&id];

        let empty_vec = Vec::new();
        let mut sorted_value_vecs: Vec<&Vec<f64>> = Vec::with_capacity(4);
        sorted_value_vecs.push(&empty_vec);
        sorted_value_vecs.push(&empty_vec);
        sorted_value_vecs.push(&empty_vec);
        sorted_value_vecs.push(&empty_vec);

        for node_data in node_data_vec {
            match node_data.slot {
                Slot(0) => sorted_value_vecs[0] = &node_data.value,
                Slot(1) => sorted_value_vecs[1] = &node_data.value,
                Slot(2) => sorted_value_vecs[2] = &node_data.value,
                Slot(3) => sorted_value_vecs[3] = &node_data.value,
                _ => continue,
            }
        }

        for value_vec in &sorted_value_vecs {
            if value_vec.is_empty() {
                panic!("Too few channels when trying to output rgba image");
            }
        }

        sorted_value_vecs[0]
            .iter()
            .zip(sorted_value_vecs[1])
            .zip(sorted_value_vecs[2])
            .zip(sorted_value_vecs[3])
            .map(|(((r, g), b), a)| vec![r, g, b, a].into_iter())
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
        self.nodes
            .keys()
            .filter(|node_id| {
                self.edges
                    .iter()
                    .map(|edge| edge.output_id)
                    .any(|x| x == **node_id)
            }).cloned()
            .collect::<Vec<NodeId>>()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Ord, PartialOrd, Eq)]
struct Slot(usize);

impl Slot {
    fn as_usize(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy)]
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
    pub fn process(
        &self,
        input: &[(NodeId, Arc<NodeData>)],
        edges: &[Edge],
    ) -> Option<Vec<Arc<NodeData>>> {
        assert!(input.len() <= self.capacity(Side::Input));
        assert_eq!(edges.len(), input.len());

        let mut sorted_input: Vec<Option<Arc<NodeData>>> = vec![None; input.len()];
        for (data_id, node_data) in input {
            for edge in edges.iter() {
                if *data_id == edge.output_id && node_data.slot == edge.output_slot {
                    sorted_input[edge.input_slot.as_usize()] = Some(Arc::clone(node_data));
                }
            }
        }

        let sorted_input: Vec<Arc<NodeData>> = sorted_input
            .into_iter()
            .map(|node_data| node_data.expect("No NodeData found when expected."))
            .collect();

        let output: Vec<Arc<NodeData>> = match self.0 {
            NodeType::Input => return None,
            NodeType::Output => Self::output(&sorted_input),
            NodeType::Add => Self::add(&sorted_input[0], &sorted_input[1]),
            NodeType::Multiply => Self::multiply(&sorted_input[0], &sorted_input[1]),
        };

        assert!(output.len() <= self.capacity(Side::Output));
        Some(output)
    }

    pub fn capacity(&self, side: Side) -> usize {
        match side {
            Side::Input => match self.0 {
                NodeType::Input => 0,
                NodeType::Output => 4,
                NodeType::Add => 2,
                NodeType::Multiply => 2,
            },
            Side::Output => match self.0 {
                NodeType::Input => 1,
                NodeType::Output => 4,
                NodeType::Add => 1,
                NodeType::Multiply => 1,
            },
        }
    }

    fn output(inputs: &[Arc<NodeData>]) -> Vec<Arc<NodeData>> {
        let mut outputs: Vec<Arc<NodeData>> = Vec::with_capacity(inputs.len());

        for (slot, input) in inputs.iter().enumerate() {
            let mut node_data = (**input).clone();
            node_data.slot = Slot(slot);
            outputs.push(Arc::new(node_data));
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
    use std::path::Path;

    #[test]
    fn shuffle() {
        let mut tex_pro = TextureProcessor::new();

        let input_heart_256 =
            tex_pro.add_input_node(&image::open(&Path::new(&"data/heart_256.png")).unwrap());
        let output_node = tex_pro.add_node(NodeType::Output);

        tex_pro.connect(input_heart_256, output_node, Slot(0), Slot(2));
        tex_pro.connect(input_heart_256, output_node, Slot(1), Slot(0));
        tex_pro.connect(input_heart_256, output_node, Slot(3), Slot(1));
        tex_pro.connect(input_heart_256, output_node, Slot(0), Slot(3));

        tex_pro.process();

        image::save_buffer(
            &Path::new(&"out/shuffle.png"),
            &image::RgbaImage::from_vec(256, 256, tex_pro.get_output_rgba(output_node)).unwrap(),
            256,
            256,
            image::ColorType::RGBA(8),
        ).unwrap();
    }

    #[test]
    fn passthrough() {
        let mut tex_pro = TextureProcessor::new();

        let input_image_1 =
            tex_pro.add_input_node(&image::open(&Path::new(&"data/image_1.png")).unwrap());
        let output_node = tex_pro.add_node(NodeType::Output);

        tex_pro.connect(input_image_1, output_node, Slot(0), Slot(0));
        tex_pro.connect(input_image_1, output_node, Slot(1), Slot(1));
        tex_pro.connect(input_image_1, output_node, Slot(2), Slot(2));
        tex_pro.connect(input_image_1, output_node, Slot(3), Slot(3));

        tex_pro.process();

        image::save_buffer(
            &Path::new(&"out/passthrough.png"),
            &image::RgbaImage::from_vec(256, 256, tex_pro.get_output_rgba(output_node)).unwrap(),
            256,
            256,
            image::ColorType::RGBA(8),
        ).unwrap();
    }

    #[test]
    fn multiply() {
        let mut tex_pro = TextureProcessor::new();

        let input_image_1 =
            tex_pro.add_input_node(&image::open(&Path::new(&"data/image_1.png")).unwrap());
        let input_white =
            tex_pro.add_input_node(&image::open(&Path::new(&"data/white.png")).unwrap());
        let multiply_node = tex_pro.add_node(NodeType::Multiply);
        let output_node = tex_pro.add_node(NodeType::Output);

        tex_pro.connect(input_image_1, multiply_node, Slot(0), Slot(0));
        tex_pro.connect(input_image_1, multiply_node, Slot(3), Slot(1));
        tex_pro.connect(multiply_node, output_node, Slot(0), Slot(0));
        tex_pro.connect(multiply_node, output_node, Slot(0), Slot(1));
        tex_pro.connect(multiply_node, output_node, Slot(0), Slot(2));
        tex_pro.connect(input_white, output_node, Slot(0), Slot(3));

        tex_pro.process();

        image::save_buffer(
            &Path::new(&"out/multiply.png"),
            &image::RgbaImage::from_vec(256, 256, tex_pro.get_output_rgba(output_node)).unwrap(),
            256,
            256,
            image::ColorType::RGBA(8),
        ).unwrap();
    }

    #[test]
    fn add() {
        let mut tex_pro = TextureProcessor::new();

        let input_image_1 =
            tex_pro.add_input_node(&image::open(&Path::new(&"data/image_1.png")).unwrap());
        let input_white =
            tex_pro.add_input_node(&image::open(&Path::new(&"data/white.png")).unwrap());
        let add_node = tex_pro.add_node(NodeType::Add);
        let output_node = tex_pro.add_node(NodeType::Output);

        tex_pro.connect(input_image_1, add_node, Slot(0), Slot(0));
        tex_pro.connect(input_image_1, add_node, Slot(1), Slot(1));
        tex_pro.connect(add_node, output_node, Slot(0), Slot(0));
        tex_pro.connect(add_node, output_node, Slot(0), Slot(1));
        tex_pro.connect(add_node, output_node, Slot(0), Slot(2));
        tex_pro.connect(input_white, output_node, Slot(0), Slot(3));

        tex_pro.process();

        image::save_buffer(
            &Path::new(&"out/add.png"),
            &image::RgbaImage::from_vec(256, 256, tex_pro.get_output_rgba(output_node)).unwrap(),
            256,
            256,
            image::ColorType::RGBA(8),
        ).unwrap();
    }
}
