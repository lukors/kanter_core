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

    dbg!(&edges);

    let output: Vec<Arc<NodeData>> = match node.node_type {
        NodeType::InputRgba => input_rgba(&input_node_datas, &node),
        NodeType::InputGray => input_gray(&input_node_datas, &node),
        NodeType::OutputRgba => output_rgba(&input_node_datas, edges, &node),
        NodeType::OutputGray => output_gray(&input_node_datas, edges, &node),
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
fn output_rgba(inputs: &[Arc<NodeData>], edges: &[Edge], node: &Arc<Node>) -> Vec<Arc<NodeData>> {
    let mut new_node_datas: Vec<Arc<NodeData>> = Vec::with_capacity(4);

    // Find a `NodeData` in `inputs` that matches the current `Edge`.
    for edge in edges {
        // Clone the `NodeData` when you find the right one. We don't want to clone the
        // `Arc<NodeData>`, because we want to make an entirely new `NodeData` which we can then
        // modify and put in a new `Arc<NodeData>` and return from the function.
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
    // dbg!(inputs.len());
    // dbg!(edges.len());

    // dbg!(&edges);
    // dbg!(&inputs);
    assert_eq!(new_node_datas.len(), 4);

    new_node_datas
}

/// Finds the `NodeData` relevant for this `Node` and outputs them.
fn output_gray(inputs: &[Arc<NodeData>], edges: &[Edge], node: &Arc<Node>) -> Vec<Arc<NodeData>> {
    let mut new_node_datas: Vec<Arc<NodeData>> = Vec::with_capacity(1);

    // Find a `NodeData` in `inputs` that matches the current `Edge`.
    for edge in edges {
        // Clone the `NodeData` in the `Arc<NodeData>` when we find the right one. We don't want to
        // clone the `Arc<NodeData>`, because we want to make an entirely new `NodeData` which we
        // can then modify and put in the `Vec<Arc<NodeData>>` and return from the function.
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

    assert_eq!(new_node_datas.len(), 1);

    // let mut new_node_datas: Vec<NodeData> = inputs.iter().map(|node_data| (**node_data).clone()).collect();
    // for new_node_data in &mut new_node_datas {
    //     new_node_data.node_id = new_node_id;
    //     new_node_data.slot_id = edges.iter().

    // }

    new_node_datas
}

/// If there is no `NodeData` associated with this node, just send an empty `Vec`, otherwise send a
/// `Vec` with the associated `NodeData`.
fn input_gray(inputs: &[Arc<NodeData>], node: &Node) -> Vec<Arc<NodeData>> {
    if inputs.len() > 0 {
        vec![Arc::clone(&inputs[0])]
    } else {
        Vec::new()
    }

    // unimplemented!();
    // Vec::new()
}

/// If there is no `NodeData` associated with this node, just send an empty `Vec`, otherwise send a
/// `Vec` with the associated `NodeData`.
fn input_rgba(inputs: &[Arc<NodeData>], node: &Node) -> Vec<Arc<NodeData>> {
    unimplemented!();
    Vec::new()
}

/// Executes the node graph contained in the node.
fn graph(inputs: &[Arc<NodeData>], edges: &[Edge], node: &Arc<Node>, graph: &NodeGraph) -> Vec<Arc<NodeData>> {
    let mut output: Vec<Arc<NodeData>> = Vec::new();
    let mut tex_pro = TextureProcessor::new();
    tex_pro.node_graph = (*graph).clone();


    // Take the `NodeData`s that are fed into this node from the external node and associate
    // them with the correct outputs on the input nodes in the child graph.
    for node_data in inputs {
        let (target_node, target_slot) = tex_pro.node_graph.input_mapping(node_data.slot_id).unwrap();

        tex_pro.node_datas.push(
            Arc::new(
                NodeData::new(target_node, target_slot, node_data.size, Arc::clone(&node_data.buffer))
                )
            );
    }

    tex_pro.process();

    // Fill the output vector with `NodeData`.
    for output_id in tex_pro.node_graph.external_output_ids() {
        // Remapping the node id from the nested graph to the parent graph so the node data ends up
        // in the right place when the parent graph tries to access it. Then return it.
        
        println!("GAG AG OAGO AOG");
        let new_node_data = NodeData::new(
            node.node_id,
            tex_pro.node_datas(output_id)[0].slot_id,
            tex_pro.node_datas(output_id)[0].size,
            Arc::clone(&tex_pro.node_datas(output_id)[0].buffer)
        );
        output.push(Arc::new(new_node_data));
        // output.push(Arc::clone(&tex_pro.node_datas(output_id)[0]));
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
