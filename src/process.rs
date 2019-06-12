use image::{FilterType, ImageBuffer, Luma};
use std::{collections::HashMap, path::Path, sync::Arc};

use crate::{dag::*, error::Result, node::*, node_data::*, node_graph::*, shared::*};

// TODO: I want to make this function take a node and process it.
pub fn process_node(
    node: Arc<Node>,
    input_node_datas: &[Arc<NodeData>],
    edges: &[Edge],
) -> Result<Vec<Arc<NodeData>>> {
    assert!(input_node_datas.len() <= node.capacity(Side::Input));
    assert_eq!(edges.len(), input_node_datas.len());

    let input_node_datas: Vec<Arc<NodeData>> =
        resize_buffers(&input_node_datas, node.resize_policy, node.filter_type)?;

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
        NodeType::Input => input(&input_node_datas, &node),
        NodeType::Output => output(&input_node_datas, edges, &node),
        NodeType::Graph(ref node_graph) => graph(&input_node_datas, edges, &node, node_graph),
        NodeType::Read(ref path) => read(Arc::clone(&node), path)?,
        NodeType::Write(ref path) => write(&input_node_datas, path)?,
        NodeType::Invert => invert(&input_node_datas),
        NodeType::Add => add(
            Arc::clone(&input_node_datas[0]),
            Arc::clone(&input_node_datas[1]),
        ), // TODO: These should take the entire vector and not two arguments
        NodeType::Multiply => multiply(&input_node_datas[0], &input_node_datas[1]),
    };

    assert!(output.len() <= node.capacity(Side::Output));
    Ok(output)
}

// TODO: Re-implement the deactivated node type process functions.

/// Finds the `NodeData`s relevant for this `Node` and outputs them.
fn output(inputs: &[Arc<NodeData>], edges: &[Edge], node: &Arc<Node>) -> Vec<Arc<NodeData>> {
    let mut new_node_datas: Vec<Arc<NodeData>> = Vec::new();

    for edge in edges {
        // Find a `NodeData` in `inputs` that matches the current `Edge`.
        let mut new_node_data = (**inputs
            .iter()
            .find(|node_data| {
                node.node_id == edge.input_id
                    && node_data.node_id == edge.output_id
                    && node_data.slot_id == edge.output_slot
            })
            .unwrap())
        .clone();

        new_node_data.node_id = node.node_id;
        new_node_data.slot_id = edge.input_slot;

        new_node_datas.push(Arc::new(new_node_data));
    }

    assert_eq!(new_node_datas.len(), 4);

    // let mut new_node_datas: Vec<NodeData> = inputs.iter().map(|node_data| (**node_data).clone()).collect();
    // for new_node_data in &mut new_node_datas {
    //     new_node_data.node_id = new_node_id;
    //     new_node_data.slot_id = edges.iter().

    // }

    new_node_datas
}

/// If there is no `NodeData` associated with this node, just send an empty `Vec`, otherwise send a
/// `Vec` with the associated `NodeData`.
fn input(inputs: &[Arc<NodeData>], node: &Node) -> Vec<Arc<NodeData>> {
    unimplemented!();
    Vec::new()
}

/// Executes the node graph contained in the node.
fn graph(inputs: &[Arc<NodeData>], edges: &[Edge], node: &Arc<Node>, graph: &NodeGraph) -> Vec<Arc<NodeData>> {
    let mut output: Vec<Arc<NodeData>> = Vec::new();

    let mut tex_pro = TextureProcessor::new();
    tex_pro.node_graph = (*graph).clone();

    // Put the relevant `NodeData` into the input nodes for this graph.
    for input_id in tex_pro.node_graph.input_ids() {
        // Get the output `NodeId` for the `NodeData` whose buffer should be given to this
        // `input_id`.
        let input_slot = tex_pro.node_graph.input_slot(input_id);
        let output_id: NodeId = edges.iter().find(|edge| edge.input_id == node.node_id && edge.input_slot == input_slot).unwrap().output_id;
        let output_data = inputs.iter().find(|node_data| node_data.node_id == output_id).unwrap();

        tex_pro.node_datas.push(
            Arc::new(
                NodeData::new(input_id, SlotId(0), output_data.size, Arc::clone(&output_data.buffer))
                )
            );
    }

    tex_pro.process();

    // Fill the output vector with `NodeData`.
    for output_id in tex_pro.node_graph.output_ids() {
        output.push(Arc::clone(&tex_pro.node_datas(output_id)[0]));
    }

    output
}

fn read(node: Arc<Node>, path: &str) -> Result<Vec<Arc<NodeData>>> {
    let buffers = read_image(&Path::new(path))?;
    let size = Size {
        width: buffers[0].width(),
        height: buffers[0].height(),
    };
    // Arc::new(NodeData::new(node.node_id, SlotId(0), size, buffer));

    let mut output: Vec<Arc<NodeData>> = Vec::with_capacity(4);
    for (channel, buffer) in buffers.into_iter().enumerate() {
        output.push(Arc::new(NodeData::new(
            node.node_id,
            SlotId(channel as u32),
            size,
            Arc::new(buffer),
        )));

        // output.push(NodeData::new(
        //     None,
        //     Slot(channel),
        //     Size::new(image.width, image.height),
        //     Arc::new(buffer),
        // ));
    }

    Ok(output)
}

fn write(inputs: &[Arc<NodeData>], path: &str) -> Result<Vec<Arc<NodeData>>> {
    let channel_vec: Vec<Arc<Buffer>> = inputs.iter().map(|node_data| Arc::clone(&node_data.buffer)).collect();
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