use std::sync::Arc;

use image::ImageBuffer;

use crate::{
    node_graph::SlotId,
    slot_data::{Size, SlotData, SlotImage},
};

use super::Node;

pub(crate) fn process(node: &Node, value: f32) -> Vec<Arc<SlotData>> {
    let (width, height) = (1, 1);

    vec![Arc::new(SlotData::new(
        node.node_id,
        SlotId(0),
        Size::new(width, height),
        Arc::new(SlotImage::Gray(Arc::new(Box::new(
            ImageBuffer::from_raw(width, height, vec![value]).unwrap(),
        )))),
    ))]
}
