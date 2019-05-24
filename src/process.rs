use image::{FilterType, ImageBuffer, Luma};
use std::{collections::HashMap, path::Path, sync::Arc};

use crate::{
    dag::*,
    error::Result,
    node::*,
    node_data::*,
    node_graph::*,
    shared::*,
};

// TODO: I want to make this function take a node and process it.
pub fn process_node(
    node: Arc<Node>,
    input_node_datas: Vec<Arc<NodeData>>,
    edges: &[Edge],
) -> Result<Vec<Arc<NodeData>>> {
    assert!(input_node_datas.len() <= node.capacity(Side::Input));
    assert_eq!(edges.len(), input_node_datas.len());

    resize_buffers(input_node_datas, node.resize_policy, node.filter_type)?;

    // NOTE: I believe this code is no longer needed because it used to be that I sent in buffers,
    // which meant I needed to sort them in the order they were supposed to be in for the
    // calculations to be correct before doing the calculations.
    
    // Now I send in `NodeData`s instead, which contain the node and slot they belong to, so there
    // should be no need for any sorting now.

    // let mut sorted_input: Vec<Arc<NodeData>> = Vec::new();
    // for node_data in input_node_datas {
    //     for edge in edges.iter() {
    //         if node_data.node_id == edge.output_id()
    //             && node_data.slot_id == edge.output_slot()
    //         {
    //             sorted_input[edge.input_slot().as_usize()] = Arc::clone(&node_data);
    //         }
    //     }
    // }

    // let sorted_input: Vec<NodeData> = sorted_input
    //     .into_iter()
    //     .map(|buffer| buffer.expect("No NodeData found when expected."))
    //     .collect();

    let output: Vec<Arc<NodeData>> = match node.node_type {
        NodeType::Input => Vec::new(),
        NodeType::Output => output(&input_node_datas),
        NodeType::Graph(ref node_graph) => graph(&input_node_datas, node_graph)?,
        NodeType::Read(ref path) => read(path)?,
        NodeType::Write(ref path) => write(&input_node_datas, path)?,
        NodeType::Invert => invert(&input_node_datas),
        NodeType::Add => add(input_node_datas[0], input_node_datas[1]), // TODO: These should take the entire vector and not two arguments
        NodeType::Multiply => multiply(&input_node_datas[0], &input_node_datas[1]),
    };

    assert!(output.len() <= node.capacity(Side::Output));
    Ok(output)
}

// TODO: Re-implement the deactivated node type process functions.

fn output(inputs: &[Arc<NodeData>]) -> Vec<Arc<NodeData>> {
    unimplemented!()
    // let mut outputs: Vec<NodeData> = Vec::with_capacity(inputs.len());

    // for (slot, _input) in inputs.iter().enumerate() {
    //     outputs.push(NodeData {
    //         id: None,
    //         slot: Slot(slot),
    //         size: inputs[slot].size,
    //         buffer: Arc::clone(&inputs[slot].buffer),
    //     });
    // }

    // outputs
}

fn graph(inputs: &[Arc<NodeData>], graph: &NodeGraph) -> Result<Vec<Arc<NodeData>>> {
    unimplemented!()
}

fn read(path: &str) -> Result<Vec<Arc<NodeData>>> {
    Ok(read_image(&Path::new(path))?)
}

fn write(inputs: &[Arc<NodeData>], path: &str) -> Result<Vec<Arc<NodeData>>> {
    let channel_vec: Vec<&Buffer> = inputs.iter().map(|node_data| &node_data.buffer).collect();
    let (width, height) = (inputs[0].size.width, inputs[0].size.height);

    image::save_buffer(
        &Path::new(path),
        &image::RgbaImage::from_vec(width, height, channels_to_rgba(&channel_vec)?).unwrap(),
        width,
        height,
        image::ColorType::RGBA(8),
    )
    .unwrap();

    Ok(Vec::new())
}

fn invert(input: &[Arc<NodeData>]) -> Vec<Arc<NodeData>> {
    unimplemented!()
    // let input = &input[0];
    // let (width, height) = (input.size.width, input.size.height);
    // let buffer: Buffer = ImageBuffer::from_fn(width, height, |x, y| {
    //     Luma([(input.buffer.get_pixel(x, y).data[0] * -1.) + 1.])
    // });

    // vec![NodeData {
    //     id: None,
    //     slot: Slot(0),
    //     size: input.size,
    //     buffer: Arc::new(buffer),
    // }]
}

fn add(input_0: Arc<NodeData>, input_1: Arc<NodeData>) -> Vec<Arc<NodeData>> {
    unimplemented!()
    // let (width, height) = (input_0.size.width, input_1.size.height);

    // let buffer: Buffer = ImageBuffer::from_fn(width, height, |x, y| {
    //     Luma([input_0.buffer.get_pixel(x, y).data[0] + input_1.buffer.get_pixel(x, y).data[0]])
    // });

    // vec![DetachedBuffer {
    //     id: None,
    //     slot: Slot(0),
    //     size: input_0.size,
    //     buffer: Arc::new(buffer),
    // }]
}

fn multiply(input_0: &Arc<NodeData>, input_1: &Arc<NodeData>) -> Vec<Arc<NodeData>> {
    unimplemented!()
    // let (width, height) = (input_0.size.width, input_1.size.height);

    // let buffer: Buffer = ImageBuffer::from_fn(width, height, |x, y| {
    //     Luma([input_0.buffer.get_pixel(x, y).data[0] * input_1.buffer.get_pixel(x, y).data[0]])
    // });

    // vec![DetachedBuffer {
    //     id: None,
    //     slot: Slot(0),
    //     size: input_0.size,
    //     buffer: Arc::new(buffer),
    // }]
}