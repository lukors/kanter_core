use std::{path::Path, sync::Arc};

use crate::{
    error::Result, node_graph::SlotId, shared::read_slot_image, slot_data::SlotData,
    slot_image::SlotImage,
};

use super::{pixel_buffer, Node};

pub(crate) fn process(node: &Node, path: &Path) -> Result<Vec<Arc<SlotData>>> {
    let slot_image = match read_slot_image(path) {
        Ok(slot_image) => slot_image,
        Err(_) => SlotImage::Rgba([
            pixel_buffer(1.0),
            pixel_buffer(0.0),
            pixel_buffer(1.0),
            pixel_buffer(1.0),
        ]),
    };

    Ok(vec![Arc::new(SlotData::new(
        node.node_id,
        SlotId(0),
        slot_image,
    ))])
}
