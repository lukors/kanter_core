use std::{
    fmt,
    sync::{Arc, RwLock},
};

use crate::{
    error::Result,
    node::process_shared::slot_data_with_name,
    node_graph::SlotId,
    slot_data::{Buffer, Size, SlotData, SlotImage},
    transient_buffer::{TransientBuffer, TransientBufferContainer},
};

use super::Node;

use image::{ImageBuffer, Luma};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Copy, Clone, PartialEq)]
pub enum MixType {
    Add,
    Subtract,
    Multiply,
    Divide,
    Pow,
}

impl Default for MixType {
    fn default() -> Self {
        Self::Add
    }
}

impl fmt::Display for MixType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Add => "Add",
                Self::Subtract => "Subtract",
                Self::Multiply => "Multiply",
                Self::Divide => "Divide",
                Self::Pow => "Power",
            }
        )
    }
}

pub(crate) fn process(
    slot_datas: &[Arc<SlotData>],
    node: &Node,
    mix_type: MixType,
) -> Result<Vec<Arc<SlotData>>> {
    let (image_left, image_right): (SlotImage, SlotImage) = {
        if let Some(slot_data_left) = slot_data_with_name(&slot_datas, &node, "left") {
            let is_rgba = slot_data_left.image.is_rgba();

            let image_right = {
                if let Some(slot_data) = slot_data_with_name(&slot_datas, &node, "right") {
                    slot_data.image.as_type(is_rgba)?
                } else {
                    SlotImage::from_value(slot_data_left.size, 0.0, is_rgba)
                }
            };

            (slot_data_left.image.clone(), image_right)
        } else if let Some(slot_data_right) = slot_data_with_name(&slot_datas, &node, "right") {
            let image_left =
                SlotImage::from_value(slot_data_right.size, 0.0, slot_data_right.image.is_rgba())
                    .into();

            (image_left, slot_data_right.image.clone())
        } else {
            return Ok(Vec::new());
        }
    };

    let size = image_left.size()?;

    let slot_image: SlotImage = match (image_left, image_right) {
        (SlotImage::Gray(left), SlotImage::Gray(right)) => {
            let (mut left, mut right) = (
                left.transient_buffer().write()?,
                right.transient_buffer().write()?,
            );
            let (left, right) = (left.buffer()?, right.buffer()?);

            SlotImage::Gray(match mix_type {
                MixType::Add => process_add_gray(left, right, size),
                MixType::Subtract => process_subtract_gray(left, right, size),
                MixType::Multiply => process_multiply_gray(left, right, size),
                MixType::Divide => process_divide_gray(left, right, size),
                MixType::Pow => process_pow_gray(left, right, size),
            })
        }
        (SlotImage::Rgba(left), SlotImage::Rgba(right)) => {
            let (mut left, mut right) = (
                left.iter()
                    .map(|tbc| tbc.transient_buffer().write().expect("Poisoned lock"))
                    .collect::<Vec<_>>(),
                right
                    .iter()
                    .map(|tbc| tbc.transient_buffer().write().expect("Poisoned lock"))
                    .collect::<Vec<_>>(),
            );
            let (left, right) = (
                left.iter_mut()
                    .map(|tbc| tbc.buffer().expect("Poisoned lock"))
                    .collect::<Vec<_>>(),
                right
                    .iter_mut()
                    .map(|tbc| tbc.buffer().expect("Poisoned lock"))
                    .collect::<Vec<_>>(),
            );

            SlotImage::Rgba(match mix_type {
                MixType::Add => process_add_rgba(&left, &right, size),
                MixType::Subtract => process_subtract_rgba(&left, &right, size),
                MixType::Multiply => process_multiply_rgba(&left, &right, size),
                MixType::Divide => process_divide_rgba(&left, &right, size),
                MixType::Pow => process_pow_rgba(&left, &right, size),
            })
        }
        _ => return Ok(Vec::new()),
    };

    Ok(vec![Arc::new(SlotData::new(
        node.node_id,
        SlotId(0),
        size,
        slot_image,
    ))])
}

fn process_add_gray(left: &Buffer, right: &Buffer, size: Size) -> Arc<TransientBufferContainer> {
    Arc::new(TransientBufferContainer::new(RwLock::new(
        TransientBuffer::new(Box::new(ImageBuffer::from_fn(
            size.width,
            size.height,
            |x, y| Luma([left.get_pixel(x, y).data[0] + right.get_pixel(x, y).data[0]]),
        ))),
    )))
}

fn process_subtract_gray(
    left: &Buffer,
    right: &Buffer,
    size: Size,
) -> Arc<TransientBufferContainer> {
    Arc::new(TransientBufferContainer::new(RwLock::new(
        TransientBuffer::new(Box::new(ImageBuffer::from_fn(
            size.width,
            size.height,
            |x, y| Luma([left.get_pixel(x, y).data[0] - right.get_pixel(x, y).data[0]]),
        ))),
    )))
}

fn process_multiply_gray(
    left: &Buffer,
    right: &Buffer,
    size: Size,
) -> Arc<TransientBufferContainer> {
    Arc::new(TransientBufferContainer::new(RwLock::new(
        TransientBuffer::new(Box::new(ImageBuffer::from_fn(
            size.width,
            size.height,
            |x, y| Luma([left.get_pixel(x, y).data[0] * right.get_pixel(x, y).data[0]]),
        ))),
    )))
}

fn process_divide_gray(left: &Buffer, right: &Buffer, size: Size) -> Arc<TransientBufferContainer> {
    Arc::new(TransientBufferContainer::new(RwLock::new(
        TransientBuffer::new(Box::new(ImageBuffer::from_fn(
            size.width,
            size.height,
            |x, y| Luma([left.get_pixel(x, y).data[0] / right.get_pixel(x, y).data[0]]),
        ))),
    )))
}

fn process_pow_gray(left: &Buffer, right: &Buffer, size: Size) -> Arc<TransientBufferContainer> {
    Arc::new(TransientBufferContainer::new(RwLock::new(
        TransientBuffer::new(Box::new(ImageBuffer::from_fn(
            size.width,
            size.height,
            |x, y| Luma([left.get_pixel(x, y).data[0].powf(right.get_pixel(x, y).data[0])]),
        ))),
    )))
}

fn process_add_rgba(
    left: &[&Buffer],
    right: &[&Buffer],
    size: Size,
) -> [Arc<TransientBufferContainer>; 4] {
    [
        process_add_gray(&left[0], &right[0], size),
        process_add_gray(&left[1], &right[1], size),
        process_add_gray(&left[2], &right[2], size),
        Arc::new(TransientBufferContainer::new(RwLock::new(
            TransientBuffer::new(Box::new(
                Buffer::from_raw(
                    size.width,
                    size.height,
                    vec![1.0; (size.width * size.height) as usize],
                )
                .unwrap(),
            )),
        ))),
    ]
}

fn process_subtract_rgba(
    left: &[&Buffer],
    right: &[&Buffer],
    size: Size,
) -> [Arc<TransientBufferContainer>; 4] {
    [
        process_subtract_gray(&left[0], &right[0], size),
        process_subtract_gray(&left[1], &right[1], size),
        process_subtract_gray(&left[2], &right[2], size),
        Arc::new(TransientBufferContainer::new(RwLock::new(
            TransientBuffer::new(Box::new(
                Buffer::from_raw(
                    size.width,
                    size.height,
                    vec![1.0; (size.width * size.height) as usize],
                )
                .unwrap(),
            )),
        ))),
    ]
}

fn process_multiply_rgba(
    left: &[&Buffer],
    right: &[&Buffer],
    size: Size,
) -> [Arc<TransientBufferContainer>; 4] {
    [
        process_multiply_gray(&left[0], &right[0], size),
        process_multiply_gray(&left[1], &right[1], size),
        process_multiply_gray(&left[2], &right[2], size),
        Arc::new(TransientBufferContainer::new(RwLock::new(
            TransientBuffer::new(Box::new(
                Buffer::from_raw(
                    size.width,
                    size.height,
                    vec![1.0; (size.width * size.height) as usize],
                )
                .unwrap(),
            )),
        ))),
    ]
}

fn process_divide_rgba(
    left: &[&Buffer],
    right: &[&Buffer],
    size: Size,
) -> [Arc<TransientBufferContainer>; 4] {
    [
        process_divide_gray(&left[0], &right[0], size),
        process_divide_gray(&left[1], &right[1], size),
        process_divide_gray(&left[2], &right[2], size),
        Arc::new(TransientBufferContainer::new(RwLock::new(
            TransientBuffer::new(Box::new(
                Buffer::from_raw(
                    size.width,
                    size.height,
                    vec![1.0; (size.width * size.height) as usize],
                )
                .unwrap(),
            )),
        ))),
    ]
}

fn process_pow_rgba(
    left: &[&Buffer],
    right: &[&Buffer],
    size: Size,
) -> [Arc<TransientBufferContainer>; 4] {
    [
        process_pow_gray(&left[0], &right[0], size),
        process_pow_gray(&left[1], &right[1], size),
        process_pow_gray(&left[2], &right[2], size),
        Arc::new(TransientBufferContainer::new(RwLock::new(
            TransientBuffer::new(Box::new(
                Buffer::from_raw(
                    size.width,
                    size.height,
                    vec![1.0; (size.width * size.height) as usize],
                )
                .unwrap(),
            )),
        ))),
    ]
}
