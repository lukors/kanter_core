use std::sync::{Arc, RwLock};

use crate::{
    engine::{Engine, NodeState},
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
    tex_pro: &Arc<TextureProcessor>,
) -> Result<Vec<Arc<SlotData>>> {
    let mut output: Vec<Arc<SlotData>> = Vec::new();
    // let tex_pro = TextureProcessor::default();
    let mut engine = Engine::new(Arc::clone(&tex_pro.add_buffer_queue));
    engine.set_node_graph((*graph).clone());

    // Insert `SlotData`s into the graph TexPro.
    for slot_data in slot_datas {
        engine.add_input_slot_data(Arc::new(SlotData::new(
            NodeId(slot_data.slot_id.0),
            SlotId(0),
            slot_data.image.clone(),
        )));
    }

    let engine = Arc::new(RwLock::new(engine));
    tex_pro.add_engine(Arc::clone(&engine))?;

    // Fill the output vector with `SlotData`.
    let output_node_ids = engine.read()?.output_ids();
    for output_node_id in output_node_ids {
        let engine = Engine::wait_for_state_read(&engine, output_node_id, NodeState::Clean)?;
        for slot_data in engine.node_slot_datas(output_node_id)? {
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
