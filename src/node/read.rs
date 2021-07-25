use std::{path::Path, sync::Arc};

use crate::{error::Result, node_graph::SlotId, shared::read_slot_image, slot_data::SlotData};

use super::Node;

pub(crate) fn process(node: &Node, path: &Path) -> Result<Vec<Arc<SlotData>>> {
    let slot_image = read_slot_image(path)?;
    Ok(vec![Arc::new(SlotData::new(
        node.node_id,
        SlotId(0),
        slot_image.size(),
        Arc::new(slot_image.into()),
    ))])
}
