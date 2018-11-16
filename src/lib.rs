// TODO:
// - Add support for all ResizePolicy variants
// - Add a resize node, though nodes are able to output a different size than their input.
// - Implement same features as Channel Shuffle 1 & 2.
// - Implement CLI.
// - Make randomly generated test to try finding corner cases.
// - Make benchmark tests.
// - Optimize away the double-allocation when resizing an image before it's processed.
// - Make each node save the resized versions of their inputs,
//   and use them if they are still relevant.

extern crate image;
extern crate rand;

use image::{imageops, DynamicImage, FilterType, GenericImageView, ImageBuffer, Luma};
use std::{
    cmp::max,
    collections::{HashMap, HashSet, VecDeque},
    path::Path,
    sync::{mpsc, Arc},
    thread,
};

#[derive(Default)]
pub struct TextureProcessor {
    nodes: HashMap<NodeId, Arc<Node>>,
    node_data: HashMap<NodeId, NodeData>,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct NodeId(u32);


#[derive(Debug, Clone)]
struct DetachedBuffer {
    id: Option<NodeId>,
    slot: Slot,
    size: Size,
    buffer: Arc<Buffer>,
}

#[derive(Debug, Copy, Clone, PartialEq)]
struct Size {
    width: u32,
    height: u32,
}

impl Size {
    fn new(width: u32, height: u32) -> Self {
        Size { width, height }
    }

    fn pixel_count(self) -> u32 {
        self.width * self.height
    }
}

#[derive(Debug)]
struct NodeData {
    size: Size,
    buffers: HashMap<Slot, Arc<Buffer>>,
}

impl NodeData {
    fn new(size: Size) -> Self {
        Self {
            size,
            buffers: HashMap::new(),
        }
    }
}

type ChannelPixel = f32;

impl TextureProcessor {
        pub fn new() -> Self {
            Self {
                nodes: HashMap::new(),
                node_data: HashMap::new(),
                edges: Vec::new(),
            }
        }

        fn add_node_internal(&mut self, node: Node, id: NodeId) -> NodeId {
        self.nodes.insert(id, Arc::new(node));
        id
    }

    pub fn add_node(&mut self, node: Node) -> NodeId {
        if node.node_type == NodeType::Input {
            panic!("Use the `add_input_node` function when adding an input node");
        }
        let id = self.new_id();
        self.add_node_internal(node, id)
    }

    pub fn add_node_with_id(&mut self, node: Node, id: NodeId) -> NodeId {
        self.add_node_internal(node, id)
    }

    pub fn add_input_node(&mut self, image: &DynamicImage) -> NodeId {
        let id = self.new_id();

        self.add_node_internal(
            Node {
                node_type: NodeType::Input,
                resize_policy: None,
                filter_type: None,
            },
            id,
        );

        let mut wrapped_buffers = HashMap::new();
        for (id, buffer) in deconstruct_image(&image).into_iter().enumerate() {
            wrapped_buffers.insert(Slot(id), Arc::new(buffer));
        }

        self.node_data.insert(
            id,
            NodeData {
                size: Size::new(image.width(), image.height()),
                buffers: wrapped_buffers,
            },
        );

        id
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
                    for (slot, data_vec) in node_data.buffers.iter() {
                        if *slot == edge.output_slot
                            && *id == edge.output_id
                            && current_id == edge.input_id
                        {
                            input_data.push(DetachedBuffer {
                                id: Some(*id),
                                slot: *slot,
                                size: node_data.size,
                                buffer: Arc::clone(data_vec),
                            });
                            relevant_edges.push(edge.clone());
                        }
                    }
                }
            }

            let current_node = Arc::clone(&self.nodes[&current_id]);
            let send = send.clone();

            thread::spawn(move || {
                let buffers = current_node.process(&mut input_data, &relevant_edges);

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
                self.node_data.insert(id, NodeData::new(buffers[0].size));
                for buffer in buffers {
                    self.node_data
                        .get_mut(&id)
                        .unwrap()
                        .buffers
                        .insert(buffer.slot, buffer.buffer);
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

    pub fn get_output_rgba(&self, id: NodeId) -> Vec<u8> {
        let buffers = &self.node_data[&id].buffers;

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
            let id: NodeId = NodeId(rand::random());
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
            }).cloned()
            .collect::<Vec<NodeId>>()
    }
}

fn channels_to_rgba(channels: &[&Buffer]) -> Vec<u8> {
    if channels.len() != 4 {
        panic!("The number of channels when converting to an RGBA image needs to be 4");
    }

    channels[0]
        .pixels()
        .zip(channels[1].pixels())
        .zip(channels[2].pixels())
        .zip(channels[3].pixels())
        .map(|(((r, g), b), a)| vec![r, g, b, a].into_iter())
        .flatten()
        .map(|x| (x[0] * 255.).min(255.) as u8)
        .collect()
}

fn deconstruct_image(image: &DynamicImage) -> Vec<Buffer> {
    let raw_pixels = image.raw_pixels();
    let (width, height) = (image.width(), image.height());
    let pixel_count = (width * height) as usize;
    let channel_count = raw_pixels.len() / pixel_count;
    let max_channel_count = 4;
    let mut pixel_vecs: Vec<Vec<f32>> = Vec::with_capacity(max_channel_count);

    for _ in 0..max_channel_count {
        pixel_vecs.push(Vec::with_capacity(pixel_count));
    }

    let mut current_channel = 0;

    for component in raw_pixels {
        pixel_vecs[current_channel].push(ChannelPixel::from(component) / 255.);
        current_channel = (current_channel + 1) % channel_count;
    }

    for (i, mut item) in pixel_vecs
        .iter_mut()
        .enumerate()
        .take(max_channel_count)
        .skip(channel_count)
    {
        *item = match i {
            3 => vec![1.; pixel_count],
            _ => vec![0.; pixel_count],
        }
    }

    pixel_vecs
        .into_iter()
        .map(|p_vec| ImageBuffer::from_raw(width, height, p_vec).unwrap())
        .collect()
}

#[derive(Copy, Clone, Debug, PartialEq)]
enum ResizePolicy {
    MostPixels,
    LeastPixels,
    LargestAxes,
    SmallestAxes,
    SpecificNode(NodeId),
    SpecificSize(Size),
}

fn resize_buffers(
    buffers: &mut [DetachedBuffer],
    policy: Option<ResizePolicy>,
    filter: Option<FilterType>,
) {
    if buffers.len() < 2 {
        return;
    }

    let policy = policy.unwrap_or(ResizePolicy::MostPixels);
    let filter = filter.unwrap_or(FilterType::CatmullRom);

    let size = match policy {
        ResizePolicy::MostPixels => buffers
            .iter()
            .max_by(|a, b| a.size.pixel_count().cmp(&b.size.pixel_count()))
            .map(|buffer| buffer.size)
            .unwrap(),
        ResizePolicy::LeastPixels => buffers
            .iter()
            .min_by(|a, b| a.size.pixel_count().cmp(&b.size.pixel_count()))
            .map(|buffer| buffer.size)
            .unwrap(),
        ResizePolicy::LargestAxes => buffers
            .iter()
            .fold(Size::new(0, 0), |a, b| {
                Size::new(
                    max(a.width, b.size.width),
                    max(a.height, b.size.height),
                )
            }),
        _ => unimplemented!(),
    };

    buffers
        .iter_mut()
        .filter(|ref buffer| buffer.size != size)
        .for_each(|ref mut buffer| {
            buffer.buffer = Arc::new(imageops::resize(
                &*buffer.buffer,
                size.width,
                size.height,
                filter,
            ));
            buffer.size = size;
        });
}

#[derive(Clone, Copy, Debug, PartialEq, Ord, PartialOrd, Eq, Hash)]
pub struct Slot(pub usize);

impl Slot {
    fn as_usize(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy)]
pub enum Side {
    Input,
    Output,
}

#[derive(Debug, PartialEq)]
pub enum NodeType {
    Input,
    Output,
    Read(String),
    Write(String),
    Invert,
    Add,
    Multiply,
}

pub struct Node {
    node_type: NodeType,
    resize_policy: Option<ResizePolicy>,
    filter_type: Option<FilterType>,
}

type Buffer = ImageBuffer<Luma<ChannelPixel>, Vec<ChannelPixel>>;

impl Node {
    pub fn new(node_type: NodeType) -> Self {
        Self {
            node_type,
            resize_policy: None,
            filter_type: None,
        }
    }

    fn process(&self, input: &mut [DetachedBuffer], edges: &[Edge]) -> Vec<DetachedBuffer> {
        assert!(input.len() <= self.capacity(Side::Input));
        assert_eq!(edges.len(), input.len());

        resize_buffers(input, self.resize_policy, self.filter_type);

        let mut sorted_input: Vec<Option<DetachedBuffer>> = vec![None; input.len()];
        for detached_buffer in input {
            for edge in edges.iter() {
                if detached_buffer.id == Some(edge.output_id)
                    && detached_buffer.slot == edge.output_slot
                {
                    sorted_input[edge.input_slot.as_usize()] = Some(detached_buffer.clone());
                }
            }
        }

        let sorted_input: Vec<DetachedBuffer> = sorted_input
            .into_iter()
            .map(|buffer| buffer.expect("No NodeData found when expected."))
            .collect();

        let output: Vec<DetachedBuffer> = match self.node_type {
            NodeType::Input => Vec::new(),
            NodeType::Output => Self::output(&sorted_input),
            NodeType::Read(ref path) => Self::read(path),
            NodeType::Write(ref path) => Self::write(&sorted_input, path),
            NodeType::Invert => Self::invert(&sorted_input),
            NodeType::Add => Self::add(&sorted_input[0], &sorted_input[1]), // TODO: These should take the entire vector and not two arguments
            NodeType::Multiply => Self::multiply(&sorted_input[0], &sorted_input[1]),
        };

        assert!(output.len() <= self.capacity(Side::Output));
        output
    }

    fn capacity(&self, side: Side) -> usize {
        match side {
            Side::Input => match self.node_type {
                NodeType::Input => 0,
                NodeType::Output => 4,
                NodeType::Read(_) => 0,
                NodeType::Write(_) => 4,
                NodeType::Invert => 1,
                NodeType::Add => 2,
                NodeType::Multiply => 2,
            },
            Side::Output => match self.node_type {
                NodeType::Input => 4,
                NodeType::Output => 4,
                NodeType::Read(_) => 4,
                NodeType::Write(_) => 0,
                NodeType::Invert => 1,
                NodeType::Add => 1,
                NodeType::Multiply => 1,
            },
        }
    }

    fn output(inputs: &[DetachedBuffer]) -> Vec<DetachedBuffer> {
        let mut outputs: Vec<DetachedBuffer> = Vec::with_capacity(inputs.len());

        for (slot, _input) in inputs.iter().enumerate() {
            outputs.push(DetachedBuffer {
                id: None,
                slot: Slot(slot),
                size: inputs[slot].size,
                buffer: Arc::clone(&inputs[slot].buffer),
            });
        }

        outputs
    }

    fn read(path: &str) -> Vec<DetachedBuffer> {
        let mut output = Vec::new();

        let image = image::open(&Path::new(path)).unwrap();
        let buffers = deconstruct_image(&image);

        for (channel, buffer) in buffers.into_iter().enumerate() {
            output.push(DetachedBuffer {
                id: None,
                slot: Slot(channel),
                size: Size::new(image.width(), image.height()),
                buffer: Arc::new(buffer),
            });
        }

        output
    }

    fn write(inputs: &[DetachedBuffer], path: &str) -> Vec<DetachedBuffer> {
        let channel_vec: Vec<&Buffer> = inputs.iter().map(|node_data| &*node_data.buffer).collect();
        let (width, height) = (inputs[0].size.width, inputs[0].size.height);

        image::save_buffer(
            &Path::new(path),
            &image::RgbaImage::from_vec(width, height, channels_to_rgba(&channel_vec)).unwrap(),
            width,
            height,
            image::ColorType::RGBA(8),
        ).unwrap();

        Vec::new()
    }

    fn invert(input: &[DetachedBuffer]) -> Vec<DetachedBuffer> {
        let input = &input[0];
        // let buffer: Buffer = input.buffer.iter().map(|value| (value * -1.) + 1.).collect();
        let (width, height) = (input.size.width, input.size.height);
        let buffer: Buffer = ImageBuffer::from_fn(width, height, |x, y| {
            Luma([(input.buffer.get_pixel(x, y).data[0] * -1.) + 1.])
        });

        vec![DetachedBuffer {
            id: None,
            slot: Slot(0),
            size: input.size,
            buffer: Arc::new(buffer),
        }]
    }

    fn add(input_0: &DetachedBuffer, input_1: &DetachedBuffer) -> Vec<DetachedBuffer> {
        let (width, height) = (input_0.size.width, input_1.size.height);

        let buffer: Buffer = ImageBuffer::from_fn(width, height, |x, y| {
            Luma([input_0.buffer.get_pixel(x, y).data[0] + input_1.buffer.get_pixel(x, y).data[0]])
        });

        vec![DetachedBuffer {
            id: None,
            slot: Slot(0),
            size: input_0.size,
            buffer: Arc::new(buffer),
        }]
    }

    fn multiply(input_0: &DetachedBuffer, input_1: &DetachedBuffer) -> Vec<DetachedBuffer> {
        let (width, height) = (input_0.size.width, input_1.size.height);

        let buffer: Buffer = ImageBuffer::from_fn(width, height, |x, y| {
            Luma([input_0.buffer.get_pixel(x, y).data[0] * input_1.buffer.get_pixel(x, y).data[0]])
        });

        vec![DetachedBuffer {
            id: None,
            slot: Slot(0),
            size: input_0.size,
            buffer: Arc::new(buffer),
        }]
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