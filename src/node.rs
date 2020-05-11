use crate::{node_data::*, node_graph::*};
use image::{imageops, FilterType, ImageBuffer};
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    mem,
    sync::Arc,
};

#[derive(Copy, Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum ResizePolicy {
    MostPixels,
    LeastPixels,
    LargestAxes,
    SmallestAxes,
    SpecificSlot(SlotId),
    SpecificSize(Size),
}

impl Default for ResizePolicy {
    fn default() -> Self {
        ResizePolicy::MostPixels
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Deserialize, Serialize)]
pub enum ResizeFilter {
    Nearest,
    Triangle,
    CatmullRom,
    Gaussian,
    Lanczos3,
}

impl From<FilterType> for ResizeFilter {
    fn from(filter_type: FilterType) -> Self {
        match filter_type {
            FilterType::Nearest => ResizeFilter::Nearest,
            FilterType::Triangle => ResizeFilter::Triangle,
            FilterType::CatmullRom => ResizeFilter::CatmullRom,
            FilterType::Gaussian => ResizeFilter::Gaussian,
            FilterType::Lanczos3 => ResizeFilter::Lanczos3,
        }
    }
}

impl Into<FilterType> for ResizeFilter {
    fn into(self) -> FilterType {
        match self {
            ResizeFilter::Nearest => FilterType::Nearest,
            ResizeFilter::Triangle => FilterType::Triangle,
            ResizeFilter::CatmullRom => FilterType::CatmullRom,
            ResizeFilter::Gaussian => FilterType::Gaussian,
            ResizeFilter::Lanczos3 => FilterType::Lanczos3,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum Side {
    Input,
    Output,
}

#[derive(Clone, Debug)]
pub struct Rgba {
    pub r: Option<Arc<Buffer>>,
    pub g: Option<Arc<Buffer>>,
    pub b: Option<Arc<Buffer>>,
    pub a: Option<Arc<Buffer>>,
}

impl Rgba {
    pub fn from_buffers(buffers: Vec<Buffer>) -> Self {
        if buffers.len() != 4 {
            panic!("Tried creating an RGBA buffer with less than 4 inputs");
        }

        Self {
            r: Some(Arc::new(buffers[0])),
            g: Some(Arc::new(buffers[1])),
            b: Some(Arc::new(buffers[2])),
            a: Some(Arc::new(buffers[3])),
        }
    }
}

#[derive(Clone, Debug)]
pub enum BufferEnum {
    Gray(Option<Arc<Buffer>>),
    Rgba(Rgba),
}

impl BufferEnum {
    pub fn from_type(buffer_enum: BufferEnum) -> Self {
        match buffer_enum {
            
    }

    pub fn from_buffers(buffers: Vec<Buffer>) -> Self {
        match buffers.len() {
            1 => Self::Gray(Some(Arc::new(buffers[0]))),
            4 => Self::Rgba(Rgba::from_buffers(buffers)),
            _ => panic!("Tried creating a `BufferEnum` with {:?} channels, needs to be 1 or 4", buffers.len()),
        }
    }

    pub fn from_gray(buffer: Buffer) -> Self {
        Self::Gray(Some(Arc::new(buffer)))
    }

    pub fn resized(&self, size: Size, filter_type: FilterType) -> Self {
        let output = 

        imageops::resize(
            &node_data.buffer,
            size.width,
            size.height,
            filter_type.into(),
        )

        for buffer in self.buffers() {

        }

    }

    pub fn width(&self) -> Option<u32> {
        if let Some(size) = self.size() {
            Some(size.width)
        } else {
            None
        }
    }

    pub fn height(&self) -> Option<u32> {
        if let Some(size) = self.size() {
            Some(size.height)
        } else {
            None
        }
    }

    pub fn size(&self) -> Option<Size> {
        if let Some(buffer) = self.buffers_option().iter().find(|b| b.is_some()) {
            let buffer = buffer.unwrap();
            Some(Size::new(buffer.width(), buffer.height()))
        } else {
            None
        }
    }

    pub fn eq_enum(&self, other: &Self) -> bool {
        mem::discriminant(self) == mem::discriminant(other)
    }

    pub fn buffers_option(&self) -> Vec<Option<Arc<Buffer>>> {
        match self {
            Self::Gray(b) => vec![b.clone()],
            Self::Rgba(b) => vec![b.r.clone(), b.g.clone(), b.b.clone(), b.a.clone()],
        }
    }

    pub fn buffers(&self) -> Vec<Arc<Buffer>> {
        match self {
            Self::Gray(option_buffer) => {
                let buffer = if let Some(buffer) = option_buffer {
                    Arc::clone(buffer)
                } else {
                    Arc::new(Box::new(ImageBuffer::from_raw(1, 1, vec![0.; 1]).expect("Unable to create `ImageBuffer`")))
                };

                vec![buffer]
            }
            Self::Rgba(b) => {
                let (width, height, size) = {
                    let mut result = (1, 1, 1);

                    for option_buffer in self.buffers_option().iter() {
                        if let Some(buffer) = option_buffer {
                            let (width, height) = (buffer.width(), buffer.height());
                            let size = (width * height) as usize;

                            result.0 = width;
                            result.1 = height;
                            result.2 = size;

                            break
                        }
                    }

                    result
                };

                let mut output = Vec::new();

                for (i, buffer) in self.buffers_option().iter().enumerate() {

                    output.push(
                        if let Some(buffer) = buffer{

                            Arc::clone(buffer)
                        } else {
                            let value = if i == 3 { // If it's the alpha channel
                                1.
                            } else {
                                0.
                            };

                            Arc::new(Box::new(ImageBuffer::from_raw(width, height, vec![value; size]).expect("Unable to create `ImageBuffer`")))
                        }
                    )
                }

                output
            }
        }
    }

    pub fn buffers_rgba(&self) -> Vec<Arc<Buffer>> {
        let mut buffers = self.buffers();

        if buffers.len() != 4 {
            let size = self.size().expect("Was unable to get width");

            buffers.push(Arc::clone(&buffers[0]));
            buffers.push(Arc::clone(&buffers[0]));
            buffers.push(Arc::new(Box::new(ImageBuffer::from_raw(size.width, size.height, vec![1.; 1]).expect("Unable to create `ImageBuffer`"))));
        }

        buffers
    }
}

#[derive(Deserialize, Serialize, Clone)]
pub enum NodeType {
    InputGray,
    InputRgba,
    OutputGray,
    OutputRgba,
    Graph(NodeGraph),
    Image(String),
    Write(String),
    Value(f32),
    Resize(Option<ResizePolicy>, Option<ResizeFilter>),
    Mix(MixType),
    HeightToNormal,
}

#[derive(Deserialize, Serialize, Copy, Clone, PartialEq)]
pub enum MixType {
    Add,
    Subtract,
    Multiply,
    Divide,
}

impl Default for MixType {
    fn default() -> Self {
        Self::Add
    }
}

impl PartialEq for NodeType {
    fn eq(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

impl fmt::Debug for NodeType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            NodeType::InputGray => write!(f, "InputGray"),
            NodeType::InputRgba => write!(f, "InputRgba"),
            NodeType::OutputGray => write!(f, "OutputGray"),
            NodeType::OutputRgba => write!(f, "OutputRgba"),
            NodeType::Graph(_) => write!(f, "Graph"),
            NodeType::Image(_) => write!(f, "Image"),
            NodeType::Write(_) => write!(f, "Write"),
            NodeType::Value(_) => write!(f, "Value"),
            NodeType::Resize(_, _) => write!(f, "Resize"),
            NodeType::Mix(_) => write!(f, "Mix"),
            NodeType::HeightToNormal => write!(f, "HeightToNormal"),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Node {
    pub node_id: NodeId,
    pub node_type: NodeType,
    pub resize_policy: Option<ResizePolicy>,
    pub filter_type: Option<ResizeFilter>,
}

impl Node {
    pub fn new(node_type: NodeType) -> Self {
        Self {
            node_id: NodeId(0),
            node_type,
            resize_policy: None,
            filter_type: None,
        }
    }

    pub fn capacity(&self, side: Side) -> usize {
        match side {
            Side::Input => match self.node_type {
                NodeType::InputGray => 1,
                NodeType::InputRgba => 0,
                NodeType::OutputGray => 1,
                NodeType::OutputRgba => 4,
                NodeType::Graph(ref graph) => graph.input_count(),
                NodeType::Image(_) => 0,
                NodeType::Write(_) => 4,
                NodeType::Value(_) => 0,
                NodeType::Resize(_, _) => 2,
                NodeType::Mix(_) => 2,
                NodeType::HeightToNormal => 1,
            },
            Side::Output => match self.node_type {
                NodeType::InputGray => 1,
                NodeType::InputRgba => 4,
                NodeType::OutputGray => 1,
                NodeType::OutputRgba => 4,
                NodeType::Graph(ref graph) => graph.output_count(),
                NodeType::Image(_) => 4,
                NodeType::Write(_) => 0,
                NodeType::Value(_) => 1,
                NodeType::Resize(_, _) => 2,
                NodeType::Mix(_) => 1,
                NodeType::HeightToNormal => 3,
            },
        }
    }
}
