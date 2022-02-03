use std::{
    sync::{atomic::Ordering, mpsc, Arc, RwLock},
    thread,
    time::Duration,
};

use crate::{
    edge::Edge,
    error::{Result, TexProError},
    live_graph::{LiveGraph, NodeState},
    node::{embed::EmbeddedSlotData, node_type::process_node},
    node_graph::NodeId,
    process_pack::ProcessPack,
    slot_data::SlotData,
    texture_processor::TextureProcessor,
    transient_buffer::TransientBufferQueue,
};

struct ThreadMessage {
    node_id: NodeId,
    slot_datas: Result<Vec<Arc<SlotData>>>,
    live_graph: Arc<RwLock<LiveGraph>>,
}

pub(crate) fn process_loop(tex_pro: Arc<TextureProcessor>) {
    let (send, recv) = mpsc::channel::<ThreadMessage>();

    loop {
        if tex_pro.shutdown.load(Ordering::Relaxed) {
            return;
        }

        // Handle messages received from node processing threads.
        for message in recv.try_iter() {
            if let Some(live_graph) = tex_pro
                .live_graph()
                .read()
                .unwrap()
                .iter()
                .find(|live_graph| Arc::ptr_eq(live_graph, &message.live_graph))
            {
                let mut live_graph = live_graph.write().unwrap();

                let node_id = message.node_id;

                match message.slot_datas {
                    Ok(slot_datas) => {
                        for slot_data in &slot_datas {
                            TransientBufferQueue::add_slot_data(
                                &live_graph.add_buffer_queue,
                                slot_data,
                            );
                        }

                        live_graph.remove_nodes_data(node_id);
                        live_graph.slot_datas.append(&mut slot_datas.into());

                        if !live_graph.use_cache {
                            for parent in live_graph.node_graph.get_parents(node_id) {
                                if live_graph
                                    .node_graph
                                    .get_children(parent)
                                    .iter()
                                    .flatten()
                                    .all(|node_id| {
                                        matches![
                                            live_graph.node_state(*node_id).unwrap(),
                                            NodeState::Clean | NodeState::Processing
                                        ]
                                    })
                                {
                                    live_graph.remove_nodes_data(parent);
                                }
                            }
                        }

                        // At this point everything is done, the final thing before we mark it
                        // clean is to check if it's been cancelled or dirtied while we worked on
                        // it.
                        let mut not_clean = false;
                        if let Ok(node) = live_graph.node(node_id) {
                            if node.cancel.compare_exchange(
                                true,
                                false,
                                Ordering::SeqCst,
                                Ordering::Acquire,
                            ) == Ok(true)
                                || live_graph.node_state(node_id) == Ok(NodeState::ProcessingDirty)
                            {
                                not_clean = true;
                            } else {
                                let _ = live_graph.set_state(node_id, NodeState::Clean);
                            }
                        } else {
                            // Assuming the node has been removed.
                            not_clean = true;
                        }

                        if not_clean {
                            live_graph.remove_nodes_data(node_id);
                            let _ = live_graph.force_state(node_id, NodeState::Dirty);
                        }
                    }
                    Err(e) => match e {
                        TexProError::Canceled => {
                            if let Ok(node) = live_graph.node(node_id) {
                                let _ = live_graph.force_state(node_id, NodeState::Dirty);
                                node.cancel.store(false, Ordering::SeqCst);
                            }
                        }
                        _ => {
                            tex_pro.shutdown.store(true, Ordering::Relaxed);
                            panic!(
                                "Error when processing '{:?}' node with id '{}': {}",
                                live_graph.node_graph.node(node_id).unwrap().node_type,
                                node_id,
                                e
                            );
                        }
                    },
                }
            }
        }

        let mut process_packs: Vec<ProcessPack> = Vec::new();
        LiveGraph::drop_unused_live_graphs(&mut tex_pro.live_graphs.write().unwrap());

        for live_graph in tex_pro.live_graph().read().unwrap().iter() {
            let mut live_graph_write = live_graph.write().unwrap();

            let closest_processable = {
                // Get requested nodes
                let requested = if live_graph_write.auto_update {
                    live_graph_write
                        .node_states()
                        .iter()
                        .filter(|(_, node_state)| {
                            !matches!(
                                node_state,
                                NodeState::Processing
                                    | NodeState::ProcessingDirty
                                    | NodeState::Clean
                            )
                        })
                        .map(|(node_id, _)| *node_id)
                        .collect::<Vec<NodeId>>()
                } else {
                    live_graph_write
                        .node_states()
                        .iter()
                        .filter(|(_, node_state)| {
                            matches!(node_state, NodeState::Requested | NodeState::Prioritised)
                        })
                        .map(|(node_id, _)| *node_id)
                        .collect::<Vec<NodeId>>()
                };

                // Get the closest non-clean parents
                let mut closest_processable = Vec::new();
                for node_id in requested {
                    closest_processable
                        .append(&mut live_graph_write.get_closest_processable(node_id));
                }
                closest_processable.sort_unstable();
                closest_processable.dedup();
                closest_processable
            };

            for node_id in closest_processable {
                if let Ok(node) = live_graph_write.node(node_id) {
                    process_packs.push(ProcessPack {
                        node_id,
                        priority: Arc::clone(&node.priority),
                        live_graph: Arc::clone(live_graph),
                    });
                } else {
                    // Assuming the node has been deleted.
                    continue;
                }
            }

            live_graph_write.propagate_priorities();
        }

        let process_packs = {
            let mut process_pack_manager = tex_pro.process_pack_manager.write().unwrap();

            match process_pack_manager.update(process_packs) {
                Ok(process_packs) => process_packs,
                Err(e) => {
                    // All `InvalidNodeId` errors should already be handled in the function. If
                    // there is another error, it is unhandled.
                    println!("Unexpected error: {}", e);
                    tex_pro.shutdown.store(true, Ordering::Relaxed);
                    return;
                }
            }
        };

        'process: for process_pack in process_packs {
            let node_id = process_pack.node_id;

            let mut live_graph = process_pack.live_graph.write().unwrap();

            // We set it as processing before getting the list of edges to guarantee that no more
            // edges sneak in without us noticing.
            if let Ok(node_state) = live_graph.node_state_mut(node_id) {
                *node_state = NodeState::Processing;
            } else {
                continue;
            }

            let edges = live_graph
                .edges()
                .iter()
                .filter(|edge| edge.input_id == node_id)
                .copied()
                .collect::<Vec<Edge>>();

            // Ensure that all inputs are clean.
            for edge in &edges {
                let node_state = live_graph.node_state(edge.output_id);

                match node_state {
                    Ok(node_state) => {
                        if node_state != NodeState::Clean {
                            continue;
                        }
                    }
                    Err(e) => {
                        match e {
                            TexProError::InvalidNodeId => {
                                // Assuming the node has been deleted.
                                continue;
                            }
                            _ => {
                                // At time of writing there only the `InvalidNodeId` error can
                                // come from this function.
                                println!("unexpected error");
                                tex_pro.shutdown.store(true, Ordering::Relaxed);
                            }
                        }
                    }
                }
            }

            let node = live_graph.node_graph.node(node_id).unwrap();

            let embedded_node_datas: Vec<Arc<EmbeddedSlotData>> = live_graph
                .embedded_slot_datas()
                .iter()
                .map(Arc::clone)
                .collect();

            let input_node_datas: Vec<Arc<SlotData>> = live_graph
                .input_slot_datas()
                .iter()
                .map(Arc::clone)
                .collect();

            let input_data = {
                let mut input_data = Vec::new();
                for edge in &edges {
                    if let Ok(slot_data) = live_graph.slot_data(edge.output_id, edge.output_slot) {
                        input_data.push(Arc::clone(slot_data));
                    } else {
                        live_graph
                            .set_state(edge.output_id, NodeState::Dirty)
                            .unwrap();
                        live_graph.set_state(node_id, NodeState::Dirty).unwrap();
                        continue 'process;
                    }
                }
                input_data
            };

            assert_eq!(
                edges.len(),
                input_data.len(),
                "NodeType: {:?}",
                node.node_type
            );

            let tex_pro = Arc::clone(&tex_pro);
            let send = send.clone();
            let live_graph = Arc::clone(&process_pack.live_graph);

            thread::spawn(move || {
                let slot_datas: Result<Vec<Arc<SlotData>>> = process_node(
                    node,
                    &input_data,
                    &embedded_node_datas,
                    &input_node_datas,
                    &edges,
                    tex_pro,
                );

                match send.send(ThreadMessage {
                    node_id,
                    slot_datas,
                    live_graph,
                }) {
                    Ok(_) => (),
                    Err(e) => println!("{:?}", e),
                };
            });
        }

        // Sleeping to reduce CPU load.
        thread::sleep(Duration::from_millis(1));
    }
}
