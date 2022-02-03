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
        if self.priority.swap(val, Ordering::SeqCst) != val {
            self.touched.store(true, Ordering::SeqCst)
        }
    }

    pub fn propagated_priority(&self) -> i8 {
        self.propagated_priority.load(Ordering::SeqCst)
    }

    pub fn priority(&self) -> i8 {
        self.priority.load(Ordering::SeqCst)
    }

    fn untouch(&self) {
        self.touched.store(false, Ordering::SeqCst)
    }

    pub fn touch(&self) {
        self.touched.store(true, Ordering::SeqCst)
    }

    fn set_max_prio(
        &self,
        priority_propagator: &PriorityPropagator,
        node_graph: &NodeGraph,
        node_id: NodeId,
    ) -> i8 {
        let max_child_prio = node_graph
            .get_children(node_id)
            .iter()
            .flatten()
            .map(|node_id| {
                priority_propagator
                    .prio_of_node_id(*node_id)
                    .unwrap()
                    .1
                    .propagated_priority()
            })
            .max()
            .unwrap_or(i8::MIN);
        let prio = max_child_prio.max(self.priority());
        self.propagated_priority.store(prio, Ordering::SeqCst);
        prio
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

        for (node_id, priority) in self
            .priorities
            .iter()
            .filter(|(_, priority)| priority.touched.load(Ordering::SeqCst))
            .rev()
        {
            let new_prio = priority.set_max_prio(self, node_graph, *node_id);
            priority.untouch();
            match new_prio.cmp(&priority.priority()) {
                std::cmp::Ordering::Less => self.propagate_priority(*node_id, priority, node_graph),
                std::cmp::Ordering::Equal => (),
                std::cmp::Ordering::Greater => {
                    priority.set_max_prio(self, node_graph, *node_id);
                    self.propagate_priority(*node_id, priority, node_graph);
                }
            }
        }
    }

    fn sort_by_priority(priorities: &mut Vec<(NodeId, Arc<Priority>)>) {
        priorities.sort_unstable_by(|a, b| {
            a.1.priority
                .load(Ordering::SeqCst)
                .cmp(&b.1.priority.load(Ordering::SeqCst))
        });
    }

    fn prio_of_node_id(&self, node_id: NodeId) -> Option<&(NodeId, Arc<Priority>)> {
        self.priorities.iter().find(|(nid, _)| *nid == node_id)
    }

    fn propagate_priority(
        &self,
        this_node_id: NodeId,
        this_prio: &Arc<Priority>,
        node_graph: &NodeGraph,
    ) {
        let this_propagated_priority = this_prio.propagated_priority();

        for parent in node_graph.get_parents(this_node_id) {
            if let Some((parent_node_id, parent_prio)) = self.prio_of_node_id(parent) {
                match parent_prio
                    .propagated_priority
                    .fetch_max(this_propagated_priority, Ordering::SeqCst)
                    .cmp(&this_propagated_priority)
                {
                    std::cmp::Ordering::Less => {
                        self.propagate_priority(*parent_node_id, parent_prio, node_graph);
                    }
                    std::cmp::Ordering::Equal => {}
                    std::cmp::Ordering::Greater => {
                        parent_prio.set_max_prio(self, node_graph, *parent_node_id);
                        self.propagate_priority(*parent_node_id, parent_prio, node_graph);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{atomic::Ordering, Arc};

    use crate::{
        node::{mix::MixType, node_type::NodeType, Node},
        node_graph::{NodeGraph, NodeId, SlotId},
    };

    use super::{Priority, PriorityPropagator};

    #[test]
    fn propagate_priority() {
        let mut node_graph = NodeGraph::new();
        let mut priority_propagator = PriorityPropagator::new();

        let node_1_prio = 3;
        let node_2_prio = -10;
        let node_3_prio = 8;
        let node_4_prio = 5;
        let node_5_prio = 0;

        let node_1 = add_node_with_prio(&mut node_graph, &mut priority_propagator, node_1_prio);
        let node_2 = add_node_with_prio(&mut node_graph, &mut priority_propagator, node_2_prio);
        let node_3 = add_node_with_prio(&mut node_graph, &mut priority_propagator, node_3_prio);
        let node_4 = add_node_with_prio(&mut node_graph, &mut priority_propagator, node_4_prio);
        let node_5 = add_node_with_prio(&mut node_graph, &mut priority_propagator, node_5_prio);

        node_graph
            .connect(node_1, node_2, SlotId(0), SlotId(0))
            .unwrap();
        node_graph
            .connect(node_2, node_4, SlotId(0), SlotId(0))
            .unwrap();
        node_graph
            .connect(node_3, node_4, SlotId(0), SlotId(1))
            .unwrap();
        node_graph
            .connect(node_4, node_5, SlotId(0), SlotId(0))
            .unwrap();

        // This is what the DAG looks like
        //
        //  1───2───┐
        //          4───5
        //      3───┘
        //

        priority_propagator.update(&node_graph);

        assert_priority(
            node_3,
            node_3_prio,
            priority_propagator.priorities.pop().unwrap(),
        );
        assert_priority(
            node_4,
            node_4_prio,
            priority_propagator.priorities.pop().unwrap(),
        );
        assert_priority(
            node_1,
            node_4_prio,
            priority_propagator.priorities.pop().unwrap(),
        );
        assert_priority(
            node_5,
            node_5_prio,
            priority_propagator.priorities.pop().unwrap(),
        );
        assert_priority(
            node_2,
            node_4_prio,
            priority_propagator.priorities.pop().unwrap(),
        );
    }

    fn assert_priority(
        expected_node_id: NodeId,
        expected_prio: i8,
        (node_id, prio): (NodeId, Arc<Priority>),
    ) {
        assert_eq!(node_id, expected_node_id);
        assert_eq!(prio.propagated_priority(), expected_prio);
        assert!(!prio.touched.load(Ordering::SeqCst));
    }

    fn add_node_with_prio(
        node_graph: &mut NodeGraph,
        priority_propagator: &mut PriorityPropagator,
        val: i8,
    ) -> NodeId {
        let node_id = node_graph
            .add_node(Node::new(NodeType::Mix(MixType::default())))
            .unwrap();
        let prio = node_graph.node(node_id).unwrap().priority;
        prio.set_priority(val);
        priority_propagator.push_priority(node_id, Arc::clone(&prio));

        node_id
    }
}
