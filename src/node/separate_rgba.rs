use std::sync::Arc;

use crate::{
    node_graph::SlotId,
    slot_data::{SlotData, SlotImage},
};

use super::Node;

pub(crate) fn process(slot_datas: &[Arc<SlotData>], node: &Node) -> Vec<Arc<SlotData>> {
    if let Some(slot_data) = slot_datas.get(0) {
        if let SlotImage::Rgba(buf) = &*slot_data.image {
            let size = slot_datas[0].size;
            vec![
                Arc::new(SlotData::new(
                    node.node_id,
                    SlotId(0),
                    size,
                    Arc::new(SlotImage::Gray(Arc::clone(&buf[0]))),
                )),
                Arc::new(SlotData::new(
                    node.node_id,
                    SlotId(1),
                    size,
                    Arc::new(SlotImage::Gray(Arc::clone(&buf[1]))),
                )),
                Arc::new(SlotData::new(
                    node.node_id,
                    SlotId(2),
                    size,
                    Arc::new(SlotImage::Gray(Arc::clone(&buf[2]))),
                )),
                Arc::new(SlotData::new(
                    node.node_id,
                    SlotId(3),
                    size,
                    Arc::new(SlotImage::Gray(Arc::clone(&buf[3]))),
                )),
            ]
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    }
}
