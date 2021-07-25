use crate::{error::*, node_graph::*};
use image::{ImageBuffer, Luma};
use serde::{Deserialize, Serialize};
use std::{fmt, fs::File, mem, sync::Arc};

#[derive(Debug)]
pub(crate) enum SlotImageCache {
    Ram(SlotImage),
    Storage((Size, bool, File)), // The bool is if it's an Rgba SlotImage, otherwise it's a Gray SlotImage.
}

impl From<SlotImage> for SlotImageCache {
    fn from(slot_image: SlotImage) -> Self {
        Self::Ram(slot_image)
    }
}

impl SlotImageCache {
    // TODO: If self is Storage, this functions should turn it into Ram.
    pub(crate) fn get(&mut self) -> SlotImage {
        unimplemented!()
        // match Self {
        //     Self::Ram(slot_image) -> slot_image,
        //     Self::Storage((size, rgba, file)) -> {
        //         let mut buffer = Vec::<u8>::new();

        //         if rgba {
        //             // file.
        //         } else {
        //             file.read_to_end(&mut buffer);
        //             Arc::new(Box::new(Buffer::from_raw(size.x, size.y, ).unwrap())),
                    
        //         }
        //     }
        // }
    }

    pub(crate) fn into_type(&self, rgba: bool) -> Self {
        Self::Ram(self.get().into_type(rgba))
    }

    pub(crate) fn is_rgba(&self) -> bool {
        match self {
            Self::Ram(slot_image) => slot_image.is_rgba(),
            Self::Storage((_, is_rgba, _)) => *is_rgba,
        }
    }

    pub(crate) fn size(&self) -> Size {
        match self {
            Self::Ram(slot_image) => slot_image.size(),
            Self::Storage((size, _, _)) => *size,
        }
    }
}

#[derive(Debug, Clone)]
pub enum SlotImage {
    Gray(Arc<BoxBuffer>),
    Rgba([Arc<BoxBuffer>; 4]),
}

impl PartialEq for SlotImage {
    fn eq(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

impl SlotImage {
    pub fn from_value(size: Size, value: ChannelPixel, rgba: bool) -> Self {
        if rgba {
            Self::Rgba([
                Arc::new(Box::new(
                    Buffer::from_raw(size.width, size.height, vec![value; size.pixel_count()])
                        .unwrap(),
                )),
                Arc::new(Box::new(
                    Buffer::from_raw(size.width, size.height, vec![value; size.pixel_count()])
                        .unwrap(),
                )),
                Arc::new(Box::new(
                    Buffer::from_raw(size.width, size.height, vec![value; size.pixel_count()])
                        .unwrap(),
                )),
                Arc::new(Box::new(
                    Buffer::from_raw(size.width, size.height, vec![1.0; size.pixel_count()])
                        .unwrap(),
                )),
            ])
        } else {
            Self::Gray(Arc::new(Box::new(
                Buffer::from_raw(size.width, size.height, vec![value; size.pixel_count()]).unwrap(),
            )))
        }
    }

    pub fn from_buffers_rgba(buffers: &mut [Buffer]) -> Result<Self> {
        if buffers.len() != 4 {
            return Err(TexProError::InvalidBufferCount);
        }

        let mut buffers = buffers.to_vec();
        buffers.reverse();

        Ok(Self::Rgba([
            Arc::new(Box::new(buffers.pop().unwrap())),
            Arc::new(Box::new(buffers.pop().unwrap())),
            Arc::new(Box::new(buffers.pop().unwrap())),
            Arc::new(Box::new(buffers.pop().unwrap())),
        ]))
    }

    pub fn from_buffers_rgb(buffers: &mut [Buffer]) -> Result<Self> {
        if buffers.len() != 3 {
            return Err(TexProError::InvalidBufferCount);
        }

        let (width, height) = (buffers[0].width(), buffers[0].height());
        let mut buffers = buffers.to_vec();

        buffers
            .push(Buffer::from_raw(width, height, vec![1.0; (width * height) as usize]).unwrap());

        Self::from_buffers_rgba(&mut buffers)
    }

    pub fn size(&self) -> Size {
        match self {
            Self::Gray(buf) => Size::new(buf.width(), buf.height()),
            Self::Rgba(bufs) => Size::new(bufs[0].width(), bufs[0].height()),
        }
    }

    pub fn is_rgba(&self) -> bool {
        mem::discriminant(self)
            == mem::discriminant(&Self::Rgba([
                Arc::new(Box::new(Buffer::from_raw(0, 0, Vec::new()).unwrap())),
                Arc::new(Box::new(Buffer::from_raw(0, 0, Vec::new()).unwrap())),
                Arc::new(Box::new(Buffer::from_raw(0, 0, Vec::new()).unwrap())),
                Arc::new(Box::new(Buffer::from_raw(0, 0, Vec::new()).unwrap())),
            ]))
    }
}

#[derive(Debug, Clone)]
pub struct SlotData {
    pub node_id: NodeId,
    pub slot_id: SlotId,
    pub size: Size,
    pub image: Arc<SlotImageCache>,
}

pub type Buffer = ImageBuffer<Luma<ChannelPixel>, Vec<ChannelPixel>>;
pub type BoxBuffer = Box<Buffer>;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

impl fmt::Display for Size {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}x{}", self.width, self.height)
    }
}

impl Size {
    pub fn new(width: u32, height: u32) -> Self {
        Size { width, height }
    }

    pub fn pixel_count(self) -> usize {
        (self.width * self.height) as usize
    }
}

pub type ChannelPixel = f32;

// impl PartialEq for SlotData {
//     fn eq(&self, other: &Self) -> bool {
//         self.size == other.size
//             && self
//                 .data
//                 .pixels()
//                 .zip(other.data.pixels())
//                 .all(|(p1, p2)| p1 == p2)
//     }
// }

// impl Eq for SlotData {}

impl SlotData {
    pub fn new(node_id: NodeId, slot_id: SlotId, size: Size, image: Arc<SlotImageCache>) -> Self {
        Self {
            node_id,
            slot_id,
            size,
            image,
        }
    }

    pub fn from_slot_image(node_id: NodeId, slot_id: SlotId, size: Size, image: SlotImage) -> Self {
        Self {
            node_id,
            slot_id,
            size,
            image: Arc::new(image.into()),
        }
    }

    pub fn from_value(size: Size, value: ChannelPixel, rgba: bool) -> Self {
        Self::from_slot_image(
            NodeId(0),
            SlotId(0),
            size,
            SlotImage::from_value(size, value, rgba),
        )
    }
}

trait SrgbColorSpace {
    fn linear_to_srgb(self) -> f32;
    fn srgb_to_linear(self) -> f32;
}

// source: https://entropymine.com/imageworsener/srgbformula/
impl SrgbColorSpace for f32 {
    fn linear_to_srgb(self) -> f32 {
        if self <= 0.0 {
            return self;
        }

        if self <= 0.0031308 {
            self * 12.92 // linear falloff in dark values
        } else {
            (1.055 * self.powf(1.0 / 2.4)) - 0.055 // gamma curve in other area
        }
    }

    fn srgb_to_linear(self) -> f32 {
        if self <= 0.0 {
            return self;
        }
        if self <= 0.04045 {
            self / 12.92 // linear falloff in dark values
        } else {
            ((self + 0.055) / 1.055).powf(2.4) // gamma curve in other area
        }
    }
}

impl SlotImage {
    #[inline]
    fn f32_to_u8(value: f32) -> u8 {
        ((value.clamp(0.0, 1.0) * 255.).min(255.)) as u8
    }

    pub fn to_u8(&self) -> Vec<u8> {
        match self {
            Self::Gray(buf) => buf
                .pixels()
                .map(|x| {
                    let value = Self::f32_to_u8(x[0]);
                    vec![value, value, value, 255]
                })
                .flatten()
                .collect(),
            Self::Rgba(bufs) => bufs[0]
                .pixels()
                .zip(bufs[1].pixels())
                .zip(bufs[2].pixels())
                .zip(bufs[3].pixels())
                .map(|(((r, g), b), a)| vec![r, g, b, a].into_iter())
                .flatten()
                .map(|x| Self::f32_to_u8(x[0]))
                .collect(),
        }
    }

    pub fn to_u8_srgb(&self) -> Vec<u8> {
        #[inline]
        fn f32_to_u8_srgb(value: f32) -> u8 {
            ((value.clamp(0.0, 1.0).srgb_to_linear() * 255.).min(255.)) as u8
        }

        match self {
            Self::Gray(buf) => buf
                .pixels()
                .map(|x| {
                    let value = f32_to_u8_srgb(x[0]);
                    vec![value, value, value, 255]
                })
                .flatten()
                .collect(),
            Self::Rgba(bufs) => bufs[0]
                .pixels()
                .zip(bufs[1].pixels())
                .zip(bufs[2].pixels())
                .zip(bufs[3].pixels())
                .map(|(((r, g), b), a)| {
                    vec![
                        f32_to_u8_srgb(r.data[0]),
                        f32_to_u8_srgb(g.data[0]),
                        f32_to_u8_srgb(b.data[0]),
                        Self::f32_to_u8(a.data[0]),
                    ]
                })
                .flatten()
                .collect(),
        }
    }

    /// Converts to and from grayscale and rgba.
    pub fn into_type(self, rgba: bool) -> Self {
        if self.is_rgba() == rgba {
            return self;
        }

        let (width, height) = (self.size().width, self.size().height);

        match self {
            Self::Gray(buf) => Self::Rgba([
                Arc::clone(&buf),
                Arc::clone(&buf),
                buf,
                Arc::new(Box::new(
                    Buffer::from_raw(width, height, vec![1.0; (width * height) as usize]).unwrap(),
                )),
            ]),
            Self::Rgba(bufs) => Self::Gray(Arc::new(Box::new(Buffer::from_fn(
                width,
                height,
                |x, y| {
                    Luma([(bufs[0].get_pixel(x, y).data[0]
                        + bufs[1].get_pixel(x, y).data[0]
                        + bufs[2].get_pixel(x, y).data[0])
                        / 3.])
                },
            )))),
        }
    }
}
