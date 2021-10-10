use crate::{
    error::{Result, TexProError},
    node::Side,
    node_graph::{NodeId, SlotId},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Serialize)]
pub struct Edge {
    pub output_id: NodeId,
    pub input_id: NodeId,
    pub output_slot: SlotId,
    pub input_slot: SlotId,
}

impl Edge {
    pub fn new(
        output_id: NodeId,
        input_id: NodeId,
        output_slot: SlotId,
        input_slot: SlotId,
    ) -> Self {
        Self {
            output_id,
            input_id,
            output_slot,
            input_slot,
        }
    }

    pub fn from_arbitrary(
        a_node: NodeId,
        a_side: Side,
        a_slot: SlotId,
        b_node: NodeId,
        b_side: Side,
        b_slot: SlotId,
    ) -> Result<Self> {
        if a_node == b_node || a_side == b_side {
            return Err(TexProError::Generic);
        }

        Ok(match a_side {
            Side::Input => Self {
                output_id: b_node,
                input_id: a_node,
                output_slot: b_slot,
                input_slot: a_slot,
            },
            Side::Output => Self {
                output_id: a_node,
                input_id: b_node,
                output_slot: a_slot,
                input_slot: b_slot,
            },
        })
    }

    pub fn output_id(&self) -> NodeId {
        self.output_id
    }

    pub fn input_id(&self) -> NodeId {
        self.input_id
    }

    pub fn output_slot(&self) -> SlotId {
        self.output_slot
    }

    pub fn input_slot(&self) -> SlotId {
        self.input_slot
    }
}
