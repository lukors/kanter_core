use std::sync::{Arc, RwLock};
extern crate num_cpus;

use crate::{
    error::Result,
    live_graph::{LiveGraph, NodeState},
    node_graph::NodeId,
    priority::Priority,
};

#[derive(Clone, Debug)]
pub(crate) struct ProcessPack {
    pub node_id: NodeId,
    pub priority: Arc<Priority>,
    pub live_graph: Arc<RwLock<LiveGraph>>,
}

pub(crate) struct ProcessPackManager {
    process_packs: Vec<ProcessPack>,
    pub max_count: usize,
}

impl ProcessPackManager {
    pub fn new() -> Self {
        Self {
            process_packs: Vec::new(),
            max_count: num_cpus::get(),
        }
    }

    /// Gets a vec of `ProcessPacks` and returns all the new `ProcessPacks` that fit within the
    /// `max_count` limit.
    pub fn update(&mut self, mut process_packs: Vec<ProcessPack>) -> Result<Vec<ProcessPack>> {
        let mut output_packs = Vec::new();
        self.remove_clean()?;
        Self::sort_by_priority(&mut self.process_packs);
        self.process_packs.truncate(self.max_count);

        Self::sort_by_priority(&mut process_packs);

        while !process_packs.is_empty() {
            let process_pack = process_packs.pop().expect("Unfailable");

            if self.process_packs.len() < self.max_count {
                self.insert_by_priority(process_pack.clone());
                output_packs.push(process_pack);
            } else if process_pack.priority.propagated_priority()
                > self
                    .process_packs
                    .first()
                    .expect("Unfailable")
                    .priority
                    .propagated_priority()
            {
                self.insert_by_priority(process_pack.clone());
                // todo: cancel the processing of the removed node.
                self.process_packs.remove(0);
                output_packs.push(process_pack);
            } else {
                break;
            }
        }

        Ok(output_packs)
    }

    fn remove_clean(&mut self) -> Result<()> {
        for i in (0..self.process_packs.len()).rev() {
            if self.process_packs[i]
                .live_graph
                .read()?
                .node_state(self.process_packs[i].node_id)?
                == NodeState::Clean
            {
                self.process_packs.remove(i);
            }
        }

        Ok(())
    }

    fn insert_by_priority(&mut self, process_pack: ProcessPack) {
        let pos = self
            .process_packs
            .binary_search_by(|pp| {
                pp.priority
                    .propagated_priority()
                    .cmp(&process_pack.priority.propagated_priority())
            })
            .unwrap_or_else(|e| e);
        self.process_packs.insert(pos, process_pack);
    }

    fn sort_by_priority(process_packs: &mut Vec<ProcessPack>) {
        process_packs.sort_unstable_by(|a, b| {
            a.priority
                .propagated_priority()
                .cmp(&b.priority.propagated_priority())
        });
    }

    pub fn process_packs(&self) -> &Vec<ProcessPack> {
        &self.process_packs
    }
}
