extern crate image;
extern crate rand;

use self::image::{FilterType, GenericImageView, ImageBuffer, Luma};
use std::{
    collections::HashMap,
    path::Path,
    sync::Arc,
};

use shared::*;
use dag::*;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ResizePolicy {
    MostPixels,
    LeastPixels,
    LargestAxes,
    SmallestAxes,
    SpecificNode(NodeId),
    SpecificSize(Size),
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


#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Size {
    width: u32,
    height: u32,
}

impl Size {
    pub fn new(width: u32, height: u32) -> Self {
        Size { width, height }
    }

    pub fn pixel_count(self) -> u32 {
        self.width * self.height
    }

    pub fn width(self) -> u32 {
        self.width
    }

    pub fn height(self) -> u32 {
        self.height
    }
}

#[derive(Debug)]
pub struct NodeData {
    size: Size,
    buffers: HashMap<Slot, Arc<Buffer>>,
}

impl NodeData {
    pub fn new(size: Size) -> Self {
        Self {
            size,
            buffers: HashMap::new(),
        }
    }

    pub fn from_buffers(buffers: HashMap<Slot, Arc<Buffer>>) -> Self {
        if buffers.is_empty() {
            panic!("Attempted to create a `NodeData` with empty buffers.");
        }

        let (width, height) = buffers.values().next().unwrap().dimensions();
        for buffer in buffers.values() {
            if buffer.dimensions() != (width, height) {
                panic!("Attempted to create `NodeData` with differently sized buffers in slots.");
            }
        }

        Self {
            size: Size::new(width, height),
            buffers,
        }
    }

    pub fn get_buffers(&self) -> &HashMap<Slot, Arc<Buffer>> {
        &self.buffers
    }

    pub fn get_buffers_mut(&mut self) -> &mut HashMap<Slot, Arc<Buffer>> {
        &mut self.buffers
    }

    pub fn get_size(&self) -> Size {
        self.size
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct NodeId(u32);

impl NodeId {
    pub fn new(id: u32) -> Self {
        NodeId(id)
    }
}

#[derive(Debug, Clone)]
pub struct DetachedBuffer {
    id: Option<NodeId>,
    slot: Slot,
    size: Size,
    buffer: Arc<Buffer>,
}

impl DetachedBuffer {
    pub fn new(id: Option<NodeId>, slot: Slot, size: Size, buffer: Arc<Buffer>) -> Self {
        Self {
            id,
            slot,
            size,
            buffer,
        }
    }

    pub fn size(&self) -> Size {
        self.size
    }

    pub fn slot(&self) -> Slot {
        self.slot
    }

    pub fn buffer(&self) -> Arc<Buffer> {
        Arc::clone(&self.buffer)
    }

    pub fn set_buffer(&mut self, buffer: Buffer) {
        self.buffer = Arc::new(buffer);
    }

    pub fn set_size(&mut self, size: Size) {
        self.size = size
    }
}

pub type ChannelPixel = f32;

pub type Buffer = ImageBuffer<Luma<ChannelPixel>, Vec<ChannelPixel>>;

impl Node {
    pub fn new(node_type: NodeType) -> Self {
        Self {
            node_type,
            resize_policy: None,
            filter_type: None,
        }
    }

    pub fn get_type(&self) -> &NodeType {
        &self.node_type
    }

    pub fn process(&self, input: &mut [DetachedBuffer], edges: &[Edge]) -> Vec<DetachedBuffer> {
        assert!(input.len() <= self.capacity(Side::Input));
        assert_eq!(edges.len(), input.len());

        resize_buffers(input, self.resize_policy, self.filter_type);

        let mut sorted_input: Vec<Option<DetachedBuffer>> = vec![None; input.len()];
        for detached_buffer in input {
            for edge in edges.iter() {
                if detached_buffer.id == Some(edge.output_id())
                    && detached_buffer.slot == edge.output_slot()
                {
                    sorted_input[edge.input_slot().as_usize()] = Some(detached_buffer.clone());
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