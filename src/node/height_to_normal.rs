use std::sync::Arc;

use crate::{
    error::Result,
    node::process_shared::{slot_data_with_name, Sampling},
    node_graph::SlotId,
    slot_data::{Buffer, SlotData, SlotImage},
};

use super::Node;

use image::{ImageBuffer, Luma};
use nalgebra::{Cross, Norm, Vector3};

pub(crate) fn process(slot_datas: &[Arc<SlotData>], node: &Node) -> Result<Vec<Arc<SlotData>>> {
    let slot_data = if let Some(slot_data) = slot_data_with_name(slot_datas, node, "input") {
        slot_data
    } else {
        return Ok(Vec::new());
    };

    let size = slot_data.size()?;
    let (width, height) = (size.width, size.height);
    let pixel_distance_x = 1. / width as f32;
    let pixel_distance_y = 1. / height as f32;

    let mut buffer_normal: [Buffer; 3] = [
        ImageBuffer::new(width, height),
        ImageBuffer::new(width, height),
        ImageBuffer::new(width, height),
    ];

    {
        // let slot_image_cache = slot_data.image_cache();
        // let mut slot_image_cache = slot_image_cache.write().unwrap();
        let buffer_height = if let SlotImage::Gray(buf) = &slot_data.image {
            buf.transient_buffer().write()?.buffer()?;
            buf.transient_buffer().read()?
        } else {
            return Ok(Vec::new());
        };

        let buffer_height = buffer_height.buffer_read()?;

        for (x, y, px) in buffer_height.enumerate_pixels() {
            let sample_up = buffer_height.get_pixel(x, y.wrapping_sample_subtract(1, height))[0];
            let sample_left = buffer_height.get_pixel(x.wrapping_sample_subtract(1, width), y)[0];

            let tangent = Vector3::new(pixel_distance_x, 0., px[0] - sample_left).normalize();
            let bitangent = Vector3::new(0., pixel_distance_y, sample_up - px[0]).normalize();
            let normal = tangent.cross(&bitangent).normalize();

            for (i, buffer) in buffer_normal.iter_mut().enumerate() {
                buffer.put_pixel(x, y, Luma([normal[i] * 0.5 + 0.5]));
            }
        }
    }

    Ok(vec![Arc::new(SlotData::new(
        node.node_id,
        SlotId(0),
        SlotImage::from_buffers_rgb(&mut buffer_normal).unwrap(),
    ))])
}
