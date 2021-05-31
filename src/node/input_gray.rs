use std::sync::Arc;

use crate::slot_data::SlotData;

use super::Node;

pub(crate) fn process(node: &Node, input_node_datas: &[Arc<SlotData>]) -> Vec<Arc<SlotData>> {
    if let Some(node_data) = input_node_datas
        .iter()
        .find(|nd| nd.node_id == node.node_id)
    {
        vec![Arc::clone(&node_data)]
    } else {
        Vec::new()
    }
}
