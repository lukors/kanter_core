use std::sync::Arc;

use crate::{node_graph::SlotId, slot_data::SlotData};

use super::Node;

pub(crate) fn process(node_datas: &[Arc<SlotData>], node: &Node) -> Vec<Arc<SlotData>> {
    if let Some(slot_data) = node_datas.get(0) {
        let mut slot_data = (**slot_data).clone();
        slot_data.node_id = node.node_id;
        slot_data.slot_id = SlotId(0);

        vec![Arc::new(slot_data)]
    } else {
        Vec::new()
    }
}
