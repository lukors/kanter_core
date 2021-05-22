use crate::{
    error::{Result, TexProError},
    node_graph::Edge,
};
use crate::{node::*, slot_data::*};
use image::{imageops, DynamicImage, GenericImageView, ImageBuffer, Luma, RgbaImage};
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

pub fn deconstruct_image(image: &DynamicImage) -> Vec<BoxBuffer> {
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
    slot_datas: &[Arc<SlotData>],
    edges: &[Edge],
    policy: ResizePolicy,
    filter: ResizeFilter,
) -> Result<Vec<Arc<SlotData>>> {
    if slot_datas.is_empty() {
        return Ok(slot_datas.into());
    }

    let size = match policy {
        ResizePolicy::MostPixels => slot_datas
            .iter()
            .max_by(|a, b| a.size.pixel_count().cmp(&b.size.pixel_count()))
            .map(|node_data| node_data.size)
            .unwrap(),
        ResizePolicy::LeastPixels => slot_datas
            .iter()
            .min_by(|a, b| a.size.pixel_count().cmp(&b.size.pixel_count()))
            .map(|node_data| node_data.size)
            .unwrap(),
        ResizePolicy::LargestAxes => slot_datas.iter().fold(Size::new(0, 0), |a, b| {
            Size::new(max(a.width, b.size.width), max(a.height, b.size.height))
        }),
        ResizePolicy::SmallestAxes => slot_datas
            .iter()
            .fold(Size::new(u32::MAX, u32::MAX), |a, b| {
                Size::new(min(a.width, b.size.width), min(a.height, b.size.height))
            }),
        ResizePolicy::SpecificSlot(slot_id) => {
            let edge = edges
                .iter()
                .find(|edge| edge.input_slot == slot_id)
                .or_else(|| edges.first());

            if let Some(edge) = edge {
                slot_datas
                    .iter()
                    .find(|node_data| {
                        node_data.slot_id == edge.output_slot && node_data.node_id == edge.output_id
                    })
                    .expect("Couldn't find a buffer with the given `NodeId` while resizing")
                    .size
            } else {
                Size::new(1, 1)
            }
        }
        ResizePolicy::SpecificSize(size) => size,
    };

    let output: Vec<Arc<SlotData>> = slot_datas
        .iter()
        .map(|ref slot_data| {
            if slot_data.size != size {
                let resized_image =
                    match &*slot_data.image {
                        SlotImage::Gray(buf) => SlotImage::Gray(Arc::new(Box::new(
                            imageops::resize(&***buf, size.width, size.height, filter.into()),
                        ))),
                        SlotImage::Rgba(bufs) => SlotImage::Rgba([
                            Arc::new(Box::new(imageops::resize(
                                &**bufs[0],
                                size.width,
                                size.height,
                                filter.into(),
                            ))),
                            Arc::new(Box::new(imageops::resize(
                                &**bufs[1],
                                size.width,
                                size.height,
                                filter.into(),
                            ))),
                            Arc::new(Box::new(imageops::resize(
                                &**bufs[2],
                                size.width,
                                size.height,
                                filter.into(),
                            ))),
                            Arc::new(Box::new(imageops::resize(
                                &**bufs[3],
                                size.width,
                                size.height,
                                filter.into(),
                            ))),
                        ]),
                    };

                Arc::new(SlotData::new(
                    slot_data.node_id,
                    slot_data.slot_id,
                    size,
                    Arc::new(resized_image),
                ))
            } else {
                // Does not need to be resized
                Arc::clone(slot_data)
            }
        })
        .collect();

    Ok(output)
}

pub fn read_slot_image<P: AsRef<Path>>(path: P) -> Result<SlotImage> {
    fn pop_vec_to_arc_buffer(
        width: u32,
        height: u32,
        buffers: &mut Vec<BoxBuffer>,
        default: f32,
    ) -> Arc<BoxBuffer> {
        Arc::new(
            buffers
                .pop()
                .or_else(|| {
                    Some(Box::new(
                        ImageBuffer::from_raw(
                            width,
                            height,
                            vec![default; (width * height) as usize],
                        )
                        .unwrap(),
                    ))
                })
                .unwrap(),
        )
    }

    let image = image::open(path)?;
    let mut buffers = deconstruct_image(&image);
    let width = buffers[0].width();
    let height = buffers[0].height();

    match buffers.len() {
        0 => Err(TexProError::InvalidBufferCount),
        1 => Ok(SlotImage::Gray(Arc::new(buffers.pop().unwrap()))),
        _ => {
            let (a, b, g, r) = (
                pop_vec_to_arc_buffer(width, height, &mut buffers, 0.0),
                pop_vec_to_arc_buffer(width, height, &mut buffers, 0.0),
                pop_vec_to_arc_buffer(width, height, &mut buffers, 0.0),
                pop_vec_to_arc_buffer(width, height, &mut buffers, 1.0),
            );
            Ok(SlotImage::Rgba([r, g, b, a]))
        }
    }
}
