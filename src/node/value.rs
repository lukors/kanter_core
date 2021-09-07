use std::sync::{Arc, RwLock};

use image::ImageBuffer;

use crate::{
    node_graph::SlotId,
    slot_data::SlotData,
    slot_image::SlotImage,
    transient_buffer::{TransientBuffer, TransientBufferContainer},
};

use super::Node;

pub(crate) fn process(node: &Node, value: f32) -> Vec<Arc<SlotData>> {
    let (width, height) = (1, 1);

    vec![Arc::new(SlotData::new(
        node.node_id,
        SlotId(0),
        SlotImage::Gray(Arc::new(TransientBufferContainer::new(Arc::new(
            RwLock::new(TransientBuffer::new(Box::new(
                ImageBuffer::from_raw(width, height, vec![value]).unwrap(),
            ))),
        )))),
    ))]
}
