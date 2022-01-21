use std::sync::{Arc, RwLock};

use crate::{
    error::Result,
    node_graph::{NodeId, SlotId},
    slot_data::SlotData,
    slot_image::{Buffer, SlotImage},
    transient_buffer::{TransientBuffer, TransientBufferContainer},
};

use super::Node;

fn black_pixel_buffer() -> Arc<TransientBufferContainer> {
    Arc::new(TransientBufferContainer::new(Arc::new(RwLock::new(
        TransientBuffer::new(Box::new(Buffer::from_raw(1, 1, vec![0.0]).unwrap())),
    ))))
}

fn default_output(node_id: NodeId) -> Vec<Arc<SlotData>> {
    vec![
        Arc::new(SlotData::new(
            node_id,
            SlotId(0),
            SlotImage::Gray(black_pixel_buffer()),
        )),
        Arc::new(SlotData::new(
            node_id,
            SlotId(1),
            SlotImage::Gray(black_pixel_buffer()),
        )),
        Arc::new(SlotData::new(
            node_id,
            SlotId(2),
            SlotImage::Gray(black_pixel_buffer()),
        )),
        Arc::new(SlotData::new(
            node_id,
            SlotId(3),
            SlotImage::Gray(black_pixel_buffer()),
        )),
    ]
}

pub(crate) fn process(slot_datas: &[Arc<SlotData>], node: &Node) -> Result<Vec<Arc<SlotData>>> {
    if let Some(slot_data) = slot_datas.get(0) {
        if let SlotImage::Rgba(buf) = &slot_data.image {
            Ok(vec![
                Arc::new(SlotData::new(
                    node.node_id,
                    SlotId(0),
                    SlotImage::Gray(Arc::clone(&buf[0])),
                )),
                Arc::new(SlotData::new(
                    node.node_id,
                    SlotId(1),
                    SlotImage::Gray(Arc::clone(&buf[1])),
                )),
                Arc::new(SlotData::new(
                    node.node_id,
                    SlotId(2),
                    SlotImage::Gray(Arc::clone(&buf[2])),
                )),
                Arc::new(SlotData::new(
                    node.node_id,
                    SlotId(3),
                    SlotImage::Gray(Arc::clone(&buf[3])),
                )),
            ])
        } else {
            Ok(default_output(node.node_id))
        }
    } else {
        Ok(default_output(node.node_id))
    }
}
