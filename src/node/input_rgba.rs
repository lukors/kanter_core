use std::sync::Arc;

use crate::{node_graph::SlotId, slot_data::SlotData};

use super::Node;

pub(crate) fn process(node: &Node, input_node_datas: &[Arc<SlotData>]) -> Vec<Arc<SlotData>> {
    let mut output = (*input_node_datas[0]).clone();
    output.slot_id = SlotId(0);
    output.node_id = node.node_id;

    vec![Arc::new(output)]
}
