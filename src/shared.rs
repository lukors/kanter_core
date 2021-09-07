use crate::{
    error::{Result, TexProError},
    node_graph::Edge,
    transient_buffer::{TransientBuffer, TransientBufferContainer},
};
use crate::{node::*, slot_data::*};
use image::{imageops, DynamicImage, GenericImageView, ImageBuffer};
use std::{
    cmp::{max, min},
    path::Path,
    sync::{Arc, RwLock},
    u32,
};

pub fn deconstruct_image(image: &DynamicImage) -> Vec<BoxBuffer> {
    let pixels = image.as_flat_samples_u8().unwrap().samples;
    let (width, height) = (image.width(), image.height());
    let pixel_count = (width * height) as usize;
    let channel_count = pixels.len() / pixel_count;
    let max_channel_count = 4;
    let mut pixel_vecs: Vec<Vec<f32>> = Vec::with_capacity(max_channel_count);

    for _ in 0..max_channel_count {
        pixel_vecs.push(Vec::with_capacity(pixel_count));
    }

    let mut current_channel = 0;

    for component in pixels {
        pixel_vecs[current_channel].push(ChannelPixel::from(*component) / 255.);
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

/// Finds out the size that a node will have.
///
/// Note: `edges` may only contain `Edge`s that connect to the inputs of the same node.
pub(crate) fn calculate_size(
    slot_datas: &[Arc<SlotData>],
    edges: &[Edge],
    policy: ResizePolicy,
) -> Size {
    assert!(edges
        .iter()
        .all(|edge| edges.first().unwrap().input_id == edge.input_id));

    match policy {
        ResizePolicy::MostPixels => {
            if slot_datas.is_empty() {
                Size::new(1, 1)
            } else {
                slot_datas
                    .iter()
                    .max_by(|a, b| {
                        a.size()
                            .unwrap()
                            .pixel_count()
                            .cmp(&b.size().unwrap().pixel_count())
                    })
                    .map(|node_data| node_data.size().unwrap())
                    .unwrap()
            }
        }
        ResizePolicy::LeastPixels => slot_datas
            .iter()
            .min_by(|a, b| {
                a.size()
                    .unwrap()
                    .pixel_count()
                    .cmp(&b.size().unwrap().pixel_count())
            })
            .map(|node_data| node_data.size().unwrap())
            .unwrap(),
        ResizePolicy::LargestAxes => slot_datas.iter().fold(Size::new(0, 0), |a, b| {
            Size::new(
                max(a.width, b.size().unwrap().width),
                max(a.height, b.size().unwrap().height),
            )
        }),
        ResizePolicy::SmallestAxes => {
            slot_datas
                .iter()
                .fold(Size::new(u32::MAX, u32::MAX), |a, b| {
                    Size::new(
                        min(a.width, b.size().unwrap().width),
                        min(a.height, b.size().unwrap().height),
                    )
                })
        }
        ResizePolicy::SpecificSlot(slot_id) => {
            let mut edges = edges.to_vec();
            edges.sort_unstable_by(|a, b| a.input_slot.cmp(&b.input_slot));

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
                    .size()
                    .unwrap()
            } else {
                // TODO: This should fall back to the size of the graph here. Graphs don't have a size
                // when this is written.
                Size::new(1, 1)
            }
        }
        ResizePolicy::SpecificSize(size) => size,
    }
}

pub(crate) fn resize_buffers(
    slot_datas: &[Arc<SlotData>],
    edges: &[Edge],
    policy: ResizePolicy,
    filter: ResizeFilter,
) -> Result<Vec<Arc<SlotData>>> {
    if slot_datas.is_empty() {
        return Ok(slot_datas.into());
    }
    let size = calculate_size(slot_datas, edges, policy);

    let output: Vec<Arc<SlotData>> = slot_datas
        .iter()
        .map(|ref slot_data| {
            if slot_data.size().unwrap() != size {
                let resized_image = match &slot_data.image {
                    SlotImage::Gray(buf) => {
                        SlotImage::Gray(Arc::new(TransientBufferContainer::new(Arc::new(
                            RwLock::new(TransientBuffer::new(Box::new(imageops::resize(
                                buf.transient_buffer().buffer(),
                                size.width,
                                size.height,
                                filter.into(),
                            )))),
                        ))))
                    }
                    SlotImage::Rgba(bufs) => SlotImage::Rgba([
                        Arc::new(TransientBufferContainer::new(Arc::new(RwLock::new(
                            TransientBuffer::new(Box::new(imageops::resize(
                                bufs[0].transient_buffer().buffer(),
                                size.width,
                                size.height,
                                filter.into(),
                            ))),
                        )))),
                        Arc::new(TransientBufferContainer::new(Arc::new(RwLock::new(
                            TransientBuffer::new(Box::new(imageops::resize(
                                bufs[1].transient_buffer().buffer(),
                                size.width,
                                size.height,
                                filter.into(),
                            ))),
                        )))),
                        Arc::new(TransientBufferContainer::new(Arc::new(RwLock::new(
                            TransientBuffer::new(Box::new(imageops::resize(
                                bufs[2].transient_buffer().buffer(),
                                size.width,
                                size.height,
                                filter.into(),
                            ))),
                        )))),
                        Arc::new(TransientBufferContainer::new(Arc::new(RwLock::new(
                            TransientBuffer::new(Box::new(imageops::resize(
                                bufs[3].transient_buffer().buffer(),
                                size.width,
                                size.height,
                                filter.into(),
                            ))),
                        )))),
                    ]),
                };

                Arc::new(SlotData::new(
                    slot_data.node_id,
                    slot_data.slot_id,
                    resized_image,
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
    ) -> Arc<TransientBufferContainer> {
        Arc::new(TransientBufferContainer::new(Arc::new(RwLock::new(
            TransientBuffer::new(
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
            ),
        ))))
    }

    let image = image::open(path)?;
    let mut buffers = deconstruct_image(&image);
    let width = buffers[0].width();
    let height = buffers[0].height();

    match buffers.len() {
        0 => Err(TexProError::InvalidBufferCount),
        1 => Ok(SlotImage::Gray(Arc::new(TransientBufferContainer::new(
            Arc::new(RwLock::new(TransientBuffer::new(buffers.pop().unwrap()))),
        )))),
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
