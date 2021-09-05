use std::sync::Arc;

use crate::{
    error::Result,
    node_graph::{NodeGraph, SlotId},
    slot_data::SlotData,
    texture_processor::TextureProcessor,
};

use super::{Node, NodeId};

/// Executes the node graph contained in the node.
pub(crate) fn process(
    slot_datas: &[Arc<SlotData>],
    node: &Node,
    graph: &NodeGraph,
) -> Result<Vec<Arc<SlotData>>> {
    let mut output: Vec<Arc<SlotData>> = Vec::new();
    let tex_pro = TextureProcessor::new();
    tex_pro.set_node_graph((*graph).clone())?;

    // Insert `SlotData`s into the graph TexPro.
    for slot_data in slot_datas {
        tex_pro.input_slot_datas_push(Arc::new(SlotData::new(
            NodeId(slot_data.slot_id.0),
            SlotId(0),
            slot_data.size,
            slot_data.image.clone(),
        )));
    }

    // Fill the output vector with `SlotData`.
    for output_node_id in tex_pro.output_ids() {
        for slot_data in tex_pro.node_slot_datas(output_node_id)? {
            let output_node_data = SlotData::new(
                node.node_id,
                SlotId(output_node_id.0),
                slot_data.size,
                slot_data.image.clone(),
            );
            output.push(Arc::new(output_node_data));
        }
    }

    Ok(output)
}
