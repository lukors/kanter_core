use crate::{
    error::*,
    slot_data::{ChannelPixel, Size, SrgbColorSpace},
    transient_buffer::{TransientBuffer, TransientBufferContainer},
};
use image::{ImageBuffer, Luma};
use std::{
    mem,
    sync::{Arc, RwLock},
};

pub type Buffer = ImageBuffer<Luma<ChannelPixel>, Vec<ChannelPixel>>;
pub type BoxBuffer = Box<Buffer>;

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
                Arc::new(TransientBufferContainer::new(Arc::new(RwLock::new(
                    TransientBuffer::new(Box::new(
                        Buffer::from_raw(size.width, size.height, vec![value; size.pixel_count()])
                            .unwrap(),
                    )),
                )))),
                Arc::new(TransientBufferContainer::new(Arc::new(RwLock::new(
                    TransientBuffer::new(Box::new(
                        Buffer::from_raw(size.width, size.height, vec![value; size.pixel_count()])
                            .unwrap(),
                    )),
                )))),
                Arc::new(TransientBufferContainer::new(Arc::new(RwLock::new(
                    TransientBuffer::new(Box::new(
                        Buffer::from_raw(size.width, size.height, vec![value; size.pixel_count()])
                            .unwrap(),
                    )),
                )))),
                Arc::new(TransientBufferContainer::new(Arc::new(RwLock::new(
                    TransientBuffer::new(Box::new(
                        Buffer::from_raw(size.width, size.height, vec![1.0; size.pixel_count()])
                            .unwrap(),
                    )),
                )))),
            ])
        } else {
            Self::Gray(Arc::new(TransientBufferContainer::new(Arc::new(
                RwLock::new(TransientBuffer::new(Box::new(
                    Buffer::from_raw(size.width, size.height, vec![value; size.pixel_count()])
                        .unwrap(),
                ))),
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
            Arc::new(TransientBufferContainer::new(Arc::new(RwLock::new(
                TransientBuffer::new(Box::new(buffers.pop().unwrap())),
            )))),
            Arc::new(TransientBufferContainer::new(Arc::new(RwLock::new(
                TransientBuffer::new(Box::new(buffers.pop().unwrap())),
            )))),
            Arc::new(TransientBufferContainer::new(Arc::new(RwLock::new(
                TransientBuffer::new(Box::new(buffers.pop().unwrap())),
            )))),
            Arc::new(TransientBufferContainer::new(Arc::new(RwLock::new(
                TransientBuffer::new(Box::new(buffers.pop().unwrap())),
            )))),
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

    pub fn from_self(&self) -> Self {
        match self {
            Self::Gray(buf) => Self::Gray(Arc::new(buf.from_self())),
            Self::Rgba(bufs) => Self::Rgba([
                Arc::new(bufs[0].from_self()),
                Arc::new(bufs[1].from_self()),
                Arc::new(bufs[2].from_self()),
                Arc::new(bufs[3].from_self()),
            ]),
        }
    }

    pub fn size(&self) -> Result<Size> {
        Ok(match self {
            Self::Gray(buf) => Size::new(buf.size().width, buf.size().height),
            Self::Rgba(bufs) => Size::new(bufs[0].size().width, bufs[0].size().height),
        })
    }

    pub fn is_rgba(&self) -> bool {
        mem::discriminant(self)
            == mem::discriminant(&Self::Rgba([
                Arc::new(TransientBufferContainer::new(Arc::new(RwLock::new(
                    TransientBuffer::new(Box::new(Buffer::from_raw(0, 0, Vec::new()).unwrap())),
                )))),
                Arc::new(TransientBufferContainer::new(Arc::new(RwLock::new(
                    TransientBuffer::new(Box::new(Buffer::from_raw(0, 0, Vec::new()).unwrap())),
                )))),
                Arc::new(TransientBufferContainer::new(Arc::new(RwLock::new(
                    TransientBuffer::new(Box::new(Buffer::from_raw(0, 0, Vec::new()).unwrap())),
                )))),
                Arc::new(TransientBufferContainer::new(Arc::new(RwLock::new(
                    TransientBuffer::new(Box::new(Buffer::from_raw(0, 0, Vec::new()).unwrap())),
                )))),
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
                .buffer()
                .pixels()
                .map(|x| {
                    let value = Self::f32_to_u8(x[0]);
                    vec![value, value, value, 255]
                })
                .flatten()
                .collect(),
            Self::Rgba(bufs) => bufs[0]
                .transient_buffer()
                .buffer()
                .pixels()
                .zip(bufs[1].transient_buffer().buffer().pixels())
                .zip(bufs[2].transient_buffer().buffer().pixels())
                .zip(bufs[3].transient_buffer().buffer().pixels())
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
                .buffer()
                .pixels()
                .map(|x| {
                    let value = f32_to_u8_srgb(x[0]);
                    vec![value, value, value, 255]
                })
                .flatten()
                .collect(),
            Self::Rgba(bufs) => bufs[0]
                .transient_buffer()
                .buffer()
                .pixels()
                .zip(bufs[1].transient_buffer().buffer().pixels())
                .zip(bufs[2].transient_buffer().buffer().pixels())
                .zip(bufs[3].transient_buffer().buffer().pixels())
                .map(|(((r, g), b), a)| {
                    vec![
                        f32_to_u8_srgb(r.0[0]),
                        f32_to_u8_srgb(g.0[0]),
                        f32_to_u8_srgb(b.0[0]),
                        Self::f32_to_u8(a.0[0]),
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
                Arc::clone(buf),
                Arc::clone(buf),
                Arc::clone(buf),
                Arc::new(TransientBufferContainer::new(Arc::new(RwLock::new(
                    TransientBuffer::new(Box::new(
                        Buffer::from_raw(width, height, vec![1.0; (width * height) as usize])
                            .unwrap(),
                    )),
                )))),
            ]),
            Self::Rgba(bufs) => {
                let (buf_r, buf_g, buf_b) = (
                    bufs[0].transient_buffer(),
                    bufs[1].transient_buffer(),
                    bufs[2].transient_buffer(),
                );
                let (buf_r, buf_g, buf_b) = (buf_r.buffer(), buf_g.buffer(), buf_b.buffer());

                Self::Gray(Arc::new(TransientBufferContainer::new(Arc::new(
                    RwLock::new(TransientBuffer::new(Box::new(Buffer::from_fn(
                        width,
                        height,
                        |x, y| {
                            Luma([(buf_r.get_pixel(x, y).0[0]
                                + buf_g.get_pixel(x, y).0[0]
                                + buf_b.get_pixel(x, y).0[0])
                                / 3.])
                        },
                    )))),
                ))))
            }
        })
    }

    pub fn bufs(&self) -> Vec<Arc<TransientBufferContainer>> {
        match self {
            Self::Gray(buf) => vec![Arc::clone(buf)],
            Self::Rgba(bufs) => bufs.to_vec(),
        }
    }
}
