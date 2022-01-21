use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use crate::{node_graph::SlotId, slot_data::SlotData};

use super::Node;

pub(crate) fn slot_data_with_name(
    slot_datas: &[Arc<SlotData>],
    node: &Node,
    name: &str,
) -> Option<Arc<SlotData>> {
    slot_data_with_slot_id(
        slot_datas,
        node.input_slot_with_name(name.into()).unwrap().slot_id,
    )
}

pub(crate) fn slot_data_with_slot_id(
    slot_datas: &[Arc<SlotData>],
    slot_id: SlotId,
) -> Option<Arc<SlotData>> {
    slot_datas
        .iter()
        .find(|slot_data| slot_data.slot_id == slot_id)
        .map(Arc::clone)
}

pub(crate) trait Sampling {
    fn wrapping_sample_add(self, right_side: Self, max: Self) -> Self;
    fn wrapping_sample_subtract(self, right_side: Self, max: Self) -> Self;
    fn coordinate_to_fraction(self, size: Self) -> f32;
}

/// Note that these functions are not checking for values outside of bounds, so they will not
/// do what's right if they get the wrong data.
impl Sampling for u32 {
    fn wrapping_sample_add(self, right_side: Self, max: Self) -> Self {
        let mut new_value = self;

        new_value += right_side;

        while new_value > max - 1 {
            new_value -= max;
        }

        new_value
    }

    fn wrapping_sample_subtract(self, right_side: Self, max: Self) -> Self {
        let mut new_value = self;

        while new_value < right_side {
            new_value += max;
        }

        new_value - right_side
    }

    fn coordinate_to_fraction(self, size: Self) -> f32 {
        self as f32 / size as f32
    }
}

pub(crate) fn cancelling(a: &Arc<AtomicBool>, b: &Arc<AtomicBool>) -> bool {
    a.load(Ordering::Relaxed) || b.load(Ordering::Relaxed)
}
