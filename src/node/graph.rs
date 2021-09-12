use std::sync::{Arc, RwLock};

use crate::{
    error::Result,
    live_graph::LiveGraph,
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
    tex_pro: &Arc<TextureProcessor>,
) -> Result<Vec<Arc<SlotData>>> {
    let mut output: Vec<Arc<SlotData>> = Vec::new();
    let mut live_graph = LiveGraph::new(Arc::clone(&tex_pro.add_buffer_queue));
    live_graph.set_node_graph((*graph).clone());

    // Insert `SlotData`s into the graph TexPro.
    for slot_data in slot_datas {
        live_graph.add_input_slot_data(Arc::new(SlotData::new(
            NodeId(slot_data.slot_id.0),
            SlotId(0),
            slot_data.image.clone(),
        )));
    }

    let live_graph = Arc::new(RwLock::new(live_graph));
    tex_pro.add_live_graph(Arc::clone(&live_graph))?;

    // Fill the output vector with `SlotData`.
    let output_node_ids = live_graph.read()?.output_ids();
    for output_node_id in output_node_ids {
        let live_graph = LiveGraph::await_clean_read(&live_graph, output_node_id)?;
        for slot_data in live_graph.node_slot_datas(output_node_id)? {
            let output_node_data = SlotData::new(
                node.node_id,
                SlotId(output_node_id.0),
                slot_data.image.clone(),
            );
            output.push(Arc::new(output_node_data));
        }
    }

    Ok(output)
}
