use std::{
    sync::{atomic::Ordering, mpsc, Arc, RwLock},
    thread,
    time::Duration,
};

use image::ImageBuffer;

use crate::{
    edge::Edge,
    error::Result,
    live_graph::{LiveGraph, NodeState},
    node::{embed::EmbeddedSlotData, node_type::process_node},
    node_graph::NodeId,
    process_pack::ProcessPack,
    slot_data::SlotData,
    slot_image::SlotImage,
    texture_processor::TextureProcessor,
    transient_buffer::{TransientBuffer, TransientBufferContainer, TransientBufferQueue},
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
                    }
                    Err(e) => {
                        tex_pro.shutdown.store(true, Ordering::Relaxed);
                        panic!(
                            "Error when processing '{:?}' node with id '{}': {}",
                            live_graph.node_graph.node(node_id).unwrap().node_type,
                            node_id,
                            e
                        );
                    }
                }

                if live_graph.set_state(node_id, NodeState::Clean).is_err() {
                    tex_pro.shutdown.store(true, Ordering::Relaxed);
                    return;
                }

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
            }
        }

        let mut process_packs: Vec<ProcessPack> = Vec::new();

        for live_graph in tex_pro.live_graph().read().unwrap().iter() {
            let closest_processable = {
                let live_graph = live_graph.read().unwrap();

                // Get requested nodes
                let requested = if live_graph.auto_update {
                    live_graph
                        .node_states()
                        .iter()
                        .filter(|(_, node_state)| {
                            !matches!(node_state, NodeState::Processing | NodeState::Clean)
                        })
                        .map(|(node_id, _)| *node_id)
                        .collect::<Vec<NodeId>>()
                } else {
                    live_graph
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
                    closest_processable.append(&mut live_graph.get_closest_processable(node_id));
                }
                closest_processable.sort_unstable();
                closest_processable.dedup();
                closest_processable
            };

            for node_id in closest_processable {
                process_packs.push(ProcessPack {
                    node_id,
                    priority: Arc::clone(
                        &live_graph.read().unwrap().node(node_id).unwrap().priority,
                    ),
                    live_graph: Arc::clone(live_graph),
                });
            }

            live_graph.write().unwrap().propagate_priorities();
        }

        let process_packs = tex_pro
            .process_pack_manager
            .write()
            .unwrap()
            .update(process_packs);

        for process_pack in process_packs {
            let node_id = process_pack.node_id;
            let mut live_graph = process_pack.live_graph.write().unwrap();

            *live_graph.node_state_mut(node_id).unwrap() = NodeState::Processing;

            let node = live_graph.node_graph.node(node_id).unwrap();

            let embedded_node_datas: Vec<Arc<EmbeddedSlotData>> = live_graph
                .embedded_slot_datas()
                .iter()
                .map(|end| Arc::clone(end))
                .collect();

            let input_node_datas: Vec<Arc<SlotData>> = live_graph
                .input_slot_datas()
                .iter()
                .map(|nd| Arc::clone(nd))
                .collect();

            let edges = live_graph
                .edges()
                .iter()
                .filter(|edge| edge.input_id == node_id)
                .copied()
                .collect::<Vec<Edge>>();

            let input_data = {
                edges
                    .iter()
                    .map(|edge| {
                        if let Ok(slot_data) =
                            live_graph.slot_data(edge.output_id, edge.output_slot)
                        {
                            Arc::clone(&slot_data)
                        } else {
                            Arc::new(SlotData::new(
                                edge.output_id,
                                edge.output_slot,
                                SlotImage::Gray(Arc::new(TransientBufferContainer::new(Arc::new(
                                    RwLock::new(TransientBuffer::new(Box::new(
                                        ImageBuffer::from_raw(1, 1, vec![0.0]).unwrap(),
                                    ))),
                                )))),
                            ))
                        }
                    })
                    .collect::<Vec<Arc<SlotData>>>()
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
