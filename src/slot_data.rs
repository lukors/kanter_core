use crate::{
    error::*,
    node_graph::*,
    transient_buffer::{TransientBuffer, TransientBufferContainer},
};
use image::{ImageBuffer, Luma};
use serde::{Deserialize, Serialize};
use std::{
    fmt::{self, Display},
    mem,
    sync::{Arc, RwLock},
};

// #[derive(Debug)]
// pub enum SlotImageCache {
//     Ram(SlotImage),
//     Storage((Size, bool, File)), // The bool is if it's an Rgba SlotImage, otherwise it's a Gray SlotImage.
// }

// impl From<SlotImage> for SlotImageCache {
//     fn from(slot_image: SlotImage) -> Self {
//         Self::Ram(slot_image)
//     }
// }

// impl Display for SlotImageCache {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         write!(f, "{}", self.is_in_ram())
//     }
// }

// impl SlotImageCache {
//     pub fn get(&mut self) -> &SlotImage {
//         match self {
//             Self::Ram(ref slot_image) => &slot_image,
//             Self::Storage((size, rgba, file)) => {
//                 let mut buffer_int: Vec<u8> = Vec::<u8>::new();
//                 file.seek(SeekFrom::Start(0)).unwrap();
//                 file.read_to_end(&mut buffer_int).unwrap();

//                 if *rgba {
//                     let component_count = buffer_int.len() / size_of::<ChannelPixel>();
//                     let pixel_count = component_count / 4;
//                     let mut buffers_f32: Vec<Vec<f32>> = vec![
//                         Vec::with_capacity(pixel_count),
//                         Vec::with_capacity(pixel_count),
//                         Vec::with_capacity(pixel_count),
//                         Vec::with_capacity(pixel_count),
//                     ];

//                     for i in 0..pixel_count {
//                         let loc = i * size_of::<ChannelPixel>();
//                         let bytes: [u8; 4] = [
//                             buffer_int[loc],
//                             buffer_int[loc + 1],
//                             buffer_int[loc + 2],
//                             buffer_int[loc + 3],
//                         ];
//                         let value = f32::from_ne_bytes(bytes);
//                         buffers_f32[3].push(value);
//                     }

//                     for i in pixel_count..pixel_count * 2 {
//                         let loc = i * size_of::<ChannelPixel>();
//                         let bytes: [u8; 4] = [
//                             buffer_int[loc],
//                             buffer_int[loc + 1],
//                             buffer_int[loc + 2],
//                             buffer_int[loc + 3],
//                         ];
//                         let value = f32::from_ne_bytes(bytes);
//                         buffers_f32[2].push(value);
//                     }

//                     for i in pixel_count * 2..pixel_count * 3 {
//                         let loc = i * size_of::<ChannelPixel>();
//                         let bytes: [u8; 4] = [
//                             buffer_int[loc],
//                             buffer_int[loc + 1],
//                             buffer_int[loc + 2],
//                             buffer_int[loc + 3],
//                         ];
//                         let value = f32::from_ne_bytes(bytes);
//                         buffers_f32[1].push(value);
//                     }

//                     for i in pixel_count * 3..pixel_count * 4 {
//                         let loc = i * size_of::<ChannelPixel>();
//                         let bytes: [u8; 4] = [
//                             buffer_int[loc],
//                             buffer_int[loc + 1],
//                             buffer_int[loc + 2],
//                             buffer_int[loc + 3],
//                         ];
//                         let value = f32::from_ne_bytes(bytes);
//                         buffers_f32[0].push(value);
//                     }

//                     *self = Self::Ram(SlotImage::Rgba([
//                         Arc::new(Box::new(
//                             Buffer::from_raw(size.width, size.height, buffers_f32.pop().unwrap())
//                                 .unwrap(),
//                         )),
//                         Arc::new(Box::new(
//                             Buffer::from_raw(size.width, size.height, buffers_f32.pop().unwrap())
//                                 .unwrap(),
//                         )),
//                         Arc::new(Box::new(
//                             Buffer::from_raw(size.width, size.height, buffers_f32.pop().unwrap())
//                                 .unwrap(),
//                         )),
//                         Arc::new(Box::new(
//                             Buffer::from_raw(size.width, size.height, buffers_f32.pop().unwrap())
//                                 .unwrap(),
//                         )),
//                     ]));

//                     if let Self::Ram(ref slot_image) = self {
//                         &slot_image
//                     } else {
//                         unreachable!() // Unreachable because self was just turned into a Self::Ram.
//                     }
//                 } else {
//                     let pixel_count = buffer_int.len() / size_of::<ChannelPixel>();
//                     let mut buffer_f32 = Vec::with_capacity(pixel_count);

//                     for i in 0..pixel_count {
//                         let loc = i * size_of::<ChannelPixel>();
//                         let bytes: [u8; 4] = [
//                             buffer_int[loc],
//                             buffer_int[loc + 1],
//                             buffer_int[loc + 2],
//                             buffer_int[loc + 3],
//                         ];
//                         let value = f32::from_ne_bytes(bytes);
//                         buffer_f32.push(value);
//                     }

//                     *self = Self::Ram(SlotImage::Gray(Arc::new(Box::new(
//                         Buffer::from_raw(size.width, size.height, buffer_f32).unwrap(),
//                     ))));

//                     if let Self::Ram(ref slot_image) = self {
//                         &slot_image
//                     } else {
//                         unreachable!() // Unreachable because self was just turned into a Self::Ram.
//                     }
//                 }
//             }
//         }
//     }

//     /// This function takes a path and a size, writes the file to the path and then converts itself
//     /// into a `Storage`.
//     pub(crate) fn store(&mut self, size: Size) -> Result<()> {
//         let mut file: File;
//         let rgba: bool;

//         if let Self::Ram(slot_image) = self {
//             file = tempfile()?;

//             match slot_image {
//                 SlotImage::Gray(buf) => {
//                     for pixel in buf.iter() {
//                         file.write(&pixel.to_ne_bytes())?;
//                     }
//                     rgba = false
//                 }
//                 SlotImage::Rgba(bufs) => {
//                     for buf in bufs {
//                         for pixel in buf.iter() {
//                             file.write(&pixel.to_ne_bytes())?;
//                         }
//                     }
//                     rgba = true
//                 }
//             }
//         } else {
//             return Ok(());
//         }

//         *self = Self::Storage((size, rgba, file));

//         Ok(())
//     }

//     pub fn is_in_ram(&self) -> bool {
//         match self {
//             Self::Ram(_) => true,
//             Self::Storage(_) => false,
//         }
//     }

//     pub fn channel_count(&self) -> usize {
//         if self.is_rgba() {
//             4
//         } else {
//             1
//         }
//     }

//     pub(crate) fn as_type(&mut self, rgba: bool) -> Self {
//         Self::Ram((*self.get()).clone().as_type(rgba))
//     }

//     pub(crate) fn is_rgba(&self) -> bool {
//         match self {
//             Self::Ram(slot_image) => slot_image.is_rgba(),
//             Self::Storage((_, is_rgba, _)) => *is_rgba,
//         }
//     }

//     pub(crate) fn size(&self) -> Size {
//         match self {
//             Self::Ram(slot_image) => slot_image.size(),
//             Self::Storage((size, _, _)) => *size,
//         }
//     }
// }

#[derive(Debug, Clone)]
pub enum SlotImage {
    Gray(Arc<TransientBufferContainer>),
    Rgba([Arc<TransientBufferContainer>; 4]),
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
                Arc::new(TransientBufferContainer::new(RwLock::new(
                    TransientBuffer::new(Box::new(
                        Buffer::from_raw(size.width, size.height, vec![value; size.pixel_count()])
                            .unwrap(),
                    )),
                ))),
                Arc::new(TransientBufferContainer::new(RwLock::new(
                    TransientBuffer::new(Box::new(
                        Buffer::from_raw(size.width, size.height, vec![value; size.pixel_count()])
                            .unwrap(),
                    )),
                ))),
                Arc::new(TransientBufferContainer::new(RwLock::new(
                    TransientBuffer::new(Box::new(
                        Buffer::from_raw(size.width, size.height, vec![value; size.pixel_count()])
                            .unwrap(),
                    )),
                ))),
                Arc::new(TransientBufferContainer::new(RwLock::new(
                    TransientBuffer::new(Box::new(
                        Buffer::from_raw(size.width, size.height, vec![1.0; size.pixel_count()])
                            .unwrap(),
                    )),
                ))),
            ])
        } else {
            Self::Gray(Arc::new(TransientBufferContainer::new(RwLock::new(
                TransientBuffer::new(Box::new(
                    Buffer::from_raw(size.width, size.height, vec![value; size.pixel_count()])
                        .unwrap(),
                )),
            ))))
        }
    }

    pub fn from_buffers_rgba(buffers: &mut [Buffer]) -> Result<Self> {
        if buffers.len() != 4 {
            return Err(TexProError::InvalidBufferCount);
        }

        let mut buffers = buffers.to_vec();
        buffers.reverse();

        Ok(Self::Rgba([
            Arc::new(TransientBufferContainer::new(RwLock::new(
                TransientBuffer::new(Box::new(buffers.pop().unwrap())),
            ))),
            Arc::new(TransientBufferContainer::new(RwLock::new(
                TransientBuffer::new(Box::new(buffers.pop().unwrap())),
            ))),
            Arc::new(TransientBufferContainer::new(RwLock::new(
                TransientBuffer::new(Box::new(buffers.pop().unwrap())),
            ))),
            Arc::new(TransientBufferContainer::new(RwLock::new(
                TransientBuffer::new(Box::new(buffers.pop().unwrap())),
            ))),
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

    pub fn size(&self) -> Result<Size> {
        Ok(match self {
            Self::Gray(buf) => Size::new(buf.size()?.width, buf.size()?.height),
            Self::Rgba(bufs) => Size::new(bufs[0].size()?.width, bufs[0].size()?.height),
        })
    }

    pub fn is_rgba(&self) -> bool {
        mem::discriminant(self)
            == mem::discriminant(&Self::Rgba([
                Arc::new(TransientBufferContainer::new(RwLock::new(
                    TransientBuffer::new(Box::new(Buffer::from_raw(0, 0, Vec::new()).unwrap())),
                ))),
                Arc::new(TransientBufferContainer::new(RwLock::new(
                    TransientBuffer::new(Box::new(Buffer::from_raw(0, 0, Vec::new()).unwrap())),
                ))),
                Arc::new(TransientBufferContainer::new(RwLock::new(
                    TransientBuffer::new(Box::new(Buffer::from_raw(0, 0, Vec::new()).unwrap())),
                ))),
                Arc::new(TransientBufferContainer::new(RwLock::new(
                    TransientBuffer::new(Box::new(Buffer::from_raw(0, 0, Vec::new()).unwrap())),
                ))),
            ]))
    }

    #[inline]
    fn f32_to_u8(value: f32) -> u8 {
        ((value.clamp(0.0, 1.0) * 255.).min(255.)) as u8
    }

    pub fn to_u8(&self) -> Result<Vec<u8>> {
        Ok(match self {
            Self::Gray(buf) => buf
                .transient_buffer()
                .write()?
                .buffer()?
                .pixels()
                .map(|x| {
                    let value = Self::f32_to_u8(x[0]);
                    vec![value, value, value, 255]
                })
                .flatten()
                .collect(),
            Self::Rgba(bufs) => bufs[0]
                .transient_buffer()
                .write()?
                .buffer()?
                .pixels()
                .zip(bufs[1].transient_buffer().write()?.buffer()?.pixels())
                .zip(bufs[2].transient_buffer().write()?.buffer()?.pixels())
                .zip(bufs[3].transient_buffer().write()?.buffer()?.pixels())
                .map(|(((r, g), b), a)| vec![r, g, b, a].into_iter())
                .flatten()
                .map(|x| Self::f32_to_u8(x[0]))
                .collect(),
        })
    }

    pub fn to_u8_srgb(&self) -> Result<Vec<u8>> {
        #[inline]
        fn f32_to_u8_srgb(value: f32) -> u8 {
            ((value.clamp(0.0, 1.0).srgb_to_linear() * 255.).min(255.)) as u8
        }

        Ok(match self {
            Self::Gray(buf) => buf
                .transient_buffer()
                .write()?
                .buffer()?
                .pixels()
                .map(|x| {
                    let value = f32_to_u8_srgb(x[0]);
                    vec![value, value, value, 255]
                })
                .flatten()
                .collect(),
            Self::Rgba(bufs) => bufs[0]
                .transient_buffer()
                .write()?
                .buffer()?
                .pixels()
                .zip(bufs[1].transient_buffer().write()?.buffer()?.pixels())
                .zip(bufs[2].transient_buffer().write()?.buffer()?.pixels())
                .zip(bufs[3].transient_buffer().write()?.buffer()?.pixels())
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
        })
    }

    /// Converts to and from grayscale and rgba.
    ///
    /// Note: This should probably be replaced by From implementations.
    pub fn as_type(&self, rgba: bool) -> Result<Self> {
        if self.is_rgba() == rgba {
            return Ok(self.clone());
        }

        let (width, height) = {
            let size = self.size()?;
            (size.width, size.height)
        };

        Ok(match self {
            Self::Gray(buf) => Self::Rgba([
                Arc::clone(&buf),
                Arc::clone(&buf),
                Arc::clone(&buf),
                Arc::new(TransientBufferContainer::new(RwLock::new(
                    TransientBuffer::new(Box::new(
                        Buffer::from_raw(width, height, vec![1.0; (width * height) as usize])
                            .unwrap(),
                    )),
                ))),
            ]),
            Self::Rgba(bufs) => {
                let (mut buf_r, mut buf_g, mut buf_b) = (
                    bufs[0].transient_buffer().write()?,
                    bufs[1].transient_buffer().write()?,
                    bufs[2].transient_buffer().write()?,
                );
                let (buf_r, buf_g, buf_b) = (buf_r.buffer()?, buf_g.buffer()?, buf_b.buffer()?);

                Self::Gray(Arc::new(TransientBufferContainer::new(RwLock::new(
                    TransientBuffer::new(Box::new(Buffer::from_fn(width, height, |x, y| {
                        Luma([(buf_r.get_pixel(x, y).data[0]
                            + buf_g.get_pixel(x, y).data[0]
                            + buf_b.get_pixel(x, y).data[0])
                            / 3.])
                    }))),
                ))))
            }
        })
    }

    pub fn bufs(&self) -> Vec<Arc<TransientBufferContainer>> {
        match self {
            Self::Gray(buf) => vec![Arc::clone(&buf)],
            Self::Rgba(bufs) => bufs.to_vec(),
        }
    }
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

impl From<(u32, u32)> for Size {
    fn from(other: (u32, u32)) -> Self {
        Size::new(other.0, other.1)
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

#[derive(Clone, Debug)]
pub struct SlotData {
    pub node_id: NodeId,
    pub slot_id: SlotId,
    pub size: Size, // Can be removed and taken from the SlotImage instead.
    pub image: SlotImage,
}

impl Display for SlotData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "NodeId: {}, SlotId: {}, Size: {}", //, Bytes: {}, In RAM: {}",
            self.node_id,
            self.slot_id,
            self.size,
            // self.bytes(),
            // self.image.read().unwrap()
        )
    }
}

impl SlotData {
    pub fn new(node_id: NodeId, slot_id: SlotId, size: Size, image: SlotImage) -> Self {
        Self {
            node_id,
            slot_id,
            size,
            image,
        }
    }

    // Stores the slot_data on drive.
    // pub(crate) fn store(&self) -> Result<()> {
    //     self.image_cache().write().unwrap().store(self.size)
    // }

    // pub fn image_cache(&self) -> Arc<RwLock<SlotImageCache>> {
    //     Arc::clone(&self.image)
    // }

    // pub fn from_slot_image(node_id: NodeId, slot_id: SlotId, size: Size, image: SlotImage) -> Self {
    //     Self {
    //         node_id,
    //         slot_id,
    //         size,
    //         image,
    //     }
    // }

    // pub fn from_value(size: Size, value: ChannelPixel, rgba: bool) -> Self {
    //     Self::new(
    //         NodeId(0),
    //         SlotId(0),
    //         size,
    //         SlotImage::from_value(size, value, rgba),
    //     )
    // }

    // pub fn channel_count(&self) -> usize {
    //     self.image.read().unwrap().channel_count()
    // }

    // pub fn bytes(&self) -> usize {
    //     self.size.pixel_count() * size_of::<ChannelPixel>() * self.channel_count()
    // }
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
