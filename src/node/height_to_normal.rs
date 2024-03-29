use std::sync::{atomic::AtomicBool, Arc};

use crate::{
    error::{Result, TexProError},
    node::process_shared::{cancelling, slot_data_with_name, Sampling},
    node_graph::SlotId,
    slot_data::SlotData,
    slot_image::{Buffer, SlotImage},
};

use super::Node;

use image::{ImageBuffer, Luma};
use nalgebra::Vector3;

pub(crate) fn process(
    shutdown: Arc<AtomicBool>,
    slot_datas: &[Arc<SlotData>],
    node: &Node,
) -> Result<Vec<Arc<SlotData>>> {
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
        let buffer_height = if let SlotImage::Gray(buf) = &slot_data.image {
            buf.transient_buffer()
        } else {
            return Ok(Vec::new());
        };
        let buffer_height = buffer_height.buffer();
        let buffer_iterator = buffer_height.enumerate_pixels();

        // This funciton is a temporary workaround. Rust-analyzer does not support const params
        // yet, so it shows a false positive error when using `Vector3::new()`, saying there are
        // too few parameters when there are not.
        fn vec3<T>(x: T, y: T, z: T) -> Vector3<T> {
            Vector3::new(x, y, z)
        }

        for (x, y, px) in buffer_iterator.take_while(|_| !cancelling(&node.cancel, &shutdown)) {
            let sample_up = buffer_height.get_pixel(x, y.wrapping_sample_subtract(1, height))[0];
            let sample_left = buffer_height.get_pixel(x.wrapping_sample_subtract(1, width), y)[0];

            let tangent = vec3(pixel_distance_x, 0., px[0] - sample_left).normalize();
            let bitangent = vec3(0., pixel_distance_y, sample_up - px[0]).normalize();
            let normal = tangent.cross(&bitangent).normalize();

            for (i, buffer) in buffer_normal.iter_mut().enumerate() {
                buffer.put_pixel(x, y, Luma([normal[i] * 0.5 + 0.5]));
            }
        }
    }

    if cancelling(&node.cancel, &shutdown) {
        Err(TexProError::Canceled)
    } else {
        Ok(vec![Arc::new(SlotData::new(
            node.node_id,
            SlotId(0),
            SlotImage::from_buffers_rgb(&mut buffer_normal).unwrap(),
        ))])
    }
}
