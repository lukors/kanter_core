use crate::error::{Result, TexProError};
use crate::{node::*, slot_data::*};
use image::{imageops, DynamicImage, GenericImageView, ImageBuffer};
use std::{
    cmp::{max, min},
    path::Path,
    sync::Arc,
    u32,
};

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

pub fn has_dup<T: PartialEq>(slice: &[T]) -> bool {
    for i in 1..slice.len() {
        if slice[i..].contains(&slice[i - 1]) {
            return true;
        }
    }
    false
}

pub fn channels_to_rgba(channels: &[Arc<Buffer>]) -> Result<Vec<u8>> {
    if channels.len() != 4 {
        return Err(TexProError::InvalidBufferCount);
    }

    fn clamp_float(input: f32) -> f32 {
        if input < 0. {
            0.
        } else if input > 1. {
            1.
        } else {
            input
        }
    }

    Ok(channels[0]
        .pixels()
        .zip(channels[1].pixels())
        .zip(channels[2].pixels())
        .zip(channels[3].pixels())
        .map(|(((r, g), b), a)| vec![r, g, b, a].into_iter())
        .flatten()
        .map(|x| (clamp_float(x[0]) * 255.).min(255.) as u8)
        .collect())
}

pub fn deconstruct_image(image: &DynamicImage) -> Vec<Buffer> {
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

    for (i, item) in pixel_vecs
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
        .map(|p_vec| {
            Box::new(
                ImageBuffer::from_raw(width, height, p_vec)
                    .expect("A bug in the deconstruct_image function caused a crash"),
            )
        })
        .collect()
}

pub fn resize_buffers(
    node_datas: &[Arc<SlotData>],
    policy: ResizePolicy,
    filter: ResizeFilter,
) -> Result<Vec<Arc<SlotData>>> {
    if node_datas.is_empty() {
        return Ok(node_datas.into());
    }

    let size = match policy {
        ResizePolicy::MostPixels => node_datas
            .iter()
            .max_by(|a, b| a.size.pixel_count().cmp(&b.size.pixel_count()))
            .map(|node_data| node_data.size)
            .unwrap(),
        ResizePolicy::LeastPixels => node_datas
            .iter()
            .min_by(|a, b| a.size.pixel_count().cmp(&b.size.pixel_count()))
            .map(|node_data| node_data.size)
            .unwrap(),
        ResizePolicy::LargestAxes => node_datas.iter().fold(Size::new(0, 0), |a, b| {
            Size::new(max(a.width, b.size.width), max(a.height, b.size.height))
        }),
        ResizePolicy::SmallestAxes => node_datas
            .iter()
            .fold(Size::new(u32::MAX, u32::MAX), |a, b| {
                Size::new(min(a.width, b.size.width), min(a.height, b.size.height))
            }),
        ResizePolicy::SpecificSlot(slot_id) => {
            node_datas
                .iter()
                .find(|node_data| node_data.slot_id == slot_id)
                .expect("Couldn't find a buffer with the given `NodeId` while resizing")
                .size
        }
        ResizePolicy::SpecificSize(size) => size,
    };

    let output: Vec<Arc<SlotData>> = node_datas
        .iter()
        .map(|ref node_data| {
            if node_data.size != size {
                // Needs to be resized
                let resized_buffer: Arc<Buffer> = Arc::new(Box::new(imageops::resize(
                    &**node_data.buffer,
                    size.width,
                    size.height,
                    filter.into(),
                )));
                Arc::new(SlotData::new(
                    node_data.node_id,
                    node_data.slot_id,
                    size,
                    Arc::clone(&resized_buffer),
                ))
            } else {
                // Does not need to be resized
                Arc::clone(node_data)
            }
        })
        .collect();

    Ok(output)
}

pub fn read_image<P: AsRef<Path>>(path: P) -> Result<Vec<Buffer>> {
    let image = image::open(path)?;
    let buffers = deconstruct_image(&image);

    Ok(buffers)
}
