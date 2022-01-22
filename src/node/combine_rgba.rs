use std::sync::{Arc, RwLock};

use crate::{
    error::Result,
    node::process_shared::slot_data_with_name,
    node_graph::SlotId,
    slot_data::{Size, SlotData},
    slot_image::{Buffer, SlotImage},
    transient_buffer::{TransientBuffer, TransientBufferContainer},
};

use super::Node;

pub(crate) fn process(slot_datas: &[Arc<SlotData>], node: &Node) -> Result<Vec<Arc<SlotData>>> {
    fn rgba_slot_data_to_buffer(
        slot_data: &Option<Arc<SlotData>>,
        buffer_default: &Arc<TransientBufferContainer>,
    ) -> Arc<TransientBufferContainer> {
        if let Some(slot_data) = slot_data {
            if let SlotImage::Gray(buf) = &slot_data.image {
                Arc::clone(buf)
            } else {
                panic!("It shouldn't be possible to connect an RGBA image into this slot");
            }
        } else {
            Arc::clone(buffer_default)
        }
    }

    fn buffer_default(
        existing_buffer: &mut Option<Arc<TransientBufferContainer>>,
        size: Size,
        alpha: bool,
    ) -> Arc<TransientBufferContainer> {
        let value = if alpha {
            1.0
        } else {
            if let Some(buffer) = existing_buffer {
                return Arc::clone(buffer);
            }

            0.0
        };

        let new_buffer = Arc::new(TransientBufferContainer::new(Arc::new(RwLock::new(
            TransientBuffer::new(Box::new(
                Buffer::from_raw(
                    size.width,
                    size.height,
                    vec![value; (size.width * size.height) as usize],
                )
                .unwrap(),
            )),
        ))));

        *existing_buffer = Some(Arc::clone(&new_buffer));
        new_buffer
    }

    let mut arc_buffer_default: Option<Arc<TransientBufferContainer>> = None;

    let size = if let Some(slot_data) = slot_datas.get(0) {
        slot_data.size()?
    } else {
        Size::new(1, 1)
    };

    let slot_datas = [
        slot_data_with_name(slot_datas, node, "red"),
        slot_data_with_name(slot_datas, node, "green"),
        slot_data_with_name(slot_datas, node, "blue"),
        slot_data_with_name(slot_datas, node, "alpha"),
    ];

    Ok(vec![Arc::new(SlotData::new(
        node.node_id,
        SlotId(0),
        SlotImage::Rgba([
            rgba_slot_data_to_buffer(
                &slot_datas[0],
                &buffer_default(&mut arc_buffer_default, size, false),
            ),
            rgba_slot_data_to_buffer(
                &slot_datas[1],
                &buffer_default(&mut arc_buffer_default, size, false),
            ),
            rgba_slot_data_to_buffer(
                &slot_datas[2],
                &buffer_default(&mut arc_buffer_default, size, false),
            ),
            rgba_slot_data_to_buffer(
                &slot_datas[3],
                &buffer_default(&mut arc_buffer_default, size, true),
            ),
        ]),
    ))])
}
