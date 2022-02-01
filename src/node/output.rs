use std::sync::Arc;

use crate::{
    node::{node_type::NodeType, pixel_buffer},
    node_graph::SlotId,
    slot_data::SlotData,
    slot_image::SlotImage,
};

use super::Node;

pub(crate) fn process(node_datas: &[Arc<SlotData>], node: &Node) -> Vec<Arc<SlotData>> {
    if let Some(slot_data) = node_datas.get(0) {
        let mut slot_data = (**slot_data).clone();
        slot_data.node_id = node.node_id;
        slot_data.slot_id = SlotId(0);

        vec![Arc::new(slot_data)]
    } else {
        let slot_image = match node.node_type {
            NodeType::OutputRgba(..) => SlotImage::Rgba([
                pixel_buffer(0.0),
                pixel_buffer(0.0),
                pixel_buffer(0.0),
                pixel_buffer(1.0),
            ]),
            NodeType::OutputGray(..) => SlotImage::Gray(pixel_buffer(0.0)),
            _ => panic!("it should only be able to be `OutputRgba` or `OutputGray`"),
        };

        vec![Arc::new(SlotData::new(node.node_id, SlotId(0), slot_image))]
    }
}
