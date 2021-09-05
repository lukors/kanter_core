use std::sync::{Arc, RwLock};

use crate::{
    node_graph::SlotId,
    slot_data::{Buffer, SlotData, SlotImage},
    transient_buffer::{TransientBuffer, TransientBufferContainer},
};

use super::Node;

pub(crate) fn process(slot_datas: &[Arc<SlotData>], node: &Node) -> Vec<Arc<SlotData>> {
    fn rgba_slot_data_to_buffer(
        slot_data: Option<&Arc<SlotData>>,
        buffer_default: &Arc<TransientBufferContainer>,
    ) -> Arc<TransientBufferContainer> {
        if let Some(slot_data) = slot_data {
            if let SlotImage::Gray(buf) = &slot_data.image {
                Arc::clone(&buf)
            } else {
                panic!("It shouldn't be possible to connect an RGBA image into this slot");
                // Arc::clone(&buffer_default)
            }
        } else {
            Arc::clone(&buffer_default)
        }
    }

    if let Some(slot_data) = slot_datas.get(0) {
        let size = slot_data.size;

        let buffer_default = Arc::new(TransientBufferContainer::new(RwLock::new(
            TransientBuffer::new(Box::new(
                Buffer::from_raw(
                    size.width,
                    size.height,
                    vec![1.0; (size.width * size.height) as usize],
                )
                .unwrap(),
            )),
        )));

        vec![Arc::new(SlotData::new(
            node.node_id,
            SlotId(0),
            size,
            SlotImage::Rgba([
                rgba_slot_data_to_buffer(slot_datas.get(0), &buffer_default),
                rgba_slot_data_to_buffer(slot_datas.get(1), &buffer_default),
                rgba_slot_data_to_buffer(slot_datas.get(2), &buffer_default),
                rgba_slot_data_to_buffer(slot_datas.get(3), &buffer_default),
            ]),
        ))]
    } else {
        Vec::new()
    }
}
