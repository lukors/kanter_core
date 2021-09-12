use std::{
    fmt::Debug,
    sync::{
        atomic::{AtomicBool, AtomicI8, Ordering},
        Arc,
    },
};

use crate::node_graph::{NodeGraph, NodeId};

#[derive(Debug)]
pub struct Priority {
    touched: AtomicBool,
    priority: AtomicI8,
    propagated_priority: AtomicI8,
}

impl Default for Priority {
    fn default() -> Self {
        Self {
            touched: true.into(),
            priority: 0.into(),
            propagated_priority: 0.into(),
        }
    }
}

impl Priority {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_priority(&self, val: i8) {
        if self.priority.swap(val, Ordering::Relaxed) != val {
            self.touched.store(true, Ordering::Relaxed)
        }
    }

    pub fn propagated_priority(&self) -> i8 {
        self.propagated_priority.load(Ordering::Relaxed)
    }

    pub fn priority(&self) -> i8 {
        self.priority.load(Ordering::Relaxed)
    }

    fn untouch(&self) {
        self.touched.store(false, Ordering::Relaxed)
    }
}

#[derive(Debug, Default)]
pub(crate) struct PriorityPropagator {
    priorities: Vec<(NodeId, Arc<Priority>)>,
}

impl PriorityPropagator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push_priority(&mut self, node_id: NodeId, priority: Arc<Priority>) {
        if self.priorities.iter().all(|(nid, _)| *nid != node_id) {
            self.priorities.push((node_id, priority));
        }
    }

    pub fn update(&mut self, node_graph: &NodeGraph) {
        for i in (0..self.priorities.len()).rev() {
            if Arc::strong_count(&self.priorities[i].1) == 1 {
                self.priorities.remove(i);
            }
        }

        Self::sort_by_priority(&mut self.priorities);

        while let Some(priority) = self
            .priorities
            .iter()
            .rev()
            .find(|(_, priority)| priority.touched.load(Ordering::Relaxed))
        {
            self.propagate_priority(priority, node_graph);
        }
    }

    fn sort_by_priority(priorities: &mut Vec<(NodeId, Arc<Priority>)>) {
        priorities.sort_unstable_by(|a, b| {
            a.1.priority
                .load(Ordering::Relaxed)
                .cmp(&b.1.priority.load(Ordering::Relaxed))
        });
    }

    fn propagate_priority(&self, current: &(NodeId, Arc<Priority>), node_graph: &NodeGraph) {
        current
            .1
            .propagated_priority
            .store(current.1.priority(), Ordering::Relaxed);
        current.1.untouch();

        for parent in node_graph.get_parents(current.0) {
            if let Some(parent) = self.priorities.iter().find(|(nid, _)| *nid == parent) {
                if parent.1.propagated_priority() < current.1.priority() {
                    self.propagate_priority(&(parent.0, Arc::clone(&current.1)), node_graph);
                }
            }
        }
    }
}
