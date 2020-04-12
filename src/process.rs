use crate::{
    dag::*,
    error::{Result, TexProError},
    node::*,
    node_data::*,
    node_graph::*,
    shared::*,
};
use image::{imageops::resize, FilterType, ImageBuffer, Luma};
use std::{path::Path, sync::Arc};

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

    let output: Vec<Arc<NodeData>> = match node.node_type {
        NodeType::InputRgba => Vec::new(),
        NodeType::InputGray => Vec::new(),
        NodeType::OutputRgba => output_rgba(&input_node_datas, edges)?,
        NodeType::OutputGray => output_gray(&input_node_datas, edges, &node),
        NodeType::Graph(ref node_graph) => graph(&input_node_datas, &node, node_graph),
        NodeType::Read(ref path) => read(Arc::clone(&node), path)?,
        NodeType::Write(ref path) => write(&input_node_datas, path)?,
        NodeType::Value(val) => value(Arc::clone(&node), val),
        NodeType::Resize(resize_policy, filter_type) => process_resize(
            &input_node_datas,
            Arc::clone(&node),
            resize_policy,
            filter_type,
        )?,
        NodeType::Add => process_add(&input_node_datas, Arc::clone(&node))?,
        NodeType::Subtract => process_subtract(&input_node_datas, Arc::clone(&node), edges)?,
        NodeType::Invert => invert(&input_node_datas),
        NodeType::Multiply => multiply(&input_node_datas[0], &input_node_datas[1]),
    };

    assert!(output.len() <= node.capacity(Side::Output));
    Ok(output)
}

/// Finds the `NodeData`s relevant for this `Node` and outputs them.
fn output_rgba(node_datas: &[Arc<NodeData>], edges: &[Edge]) -> Result<Vec<Arc<NodeData>>> {
    let mut new_node_datas: Vec<Arc<NodeData>> = Vec::with_capacity(4);

    for edge in edges {
        let node_data = node_datas
            .iter()
            .find(|node_data| {
                node_data.node_id == edge.output_id && node_data.slot_id == edge.output_slot
            })
            .ok_or(TexProError::NodeProcessing)?;

        let new_node_data = Arc::new(NodeData::new(
            edge.input_id,
            edge.input_slot,
            node_data.size,
            Arc::clone(&node_data.buffer),
        ));

        new_node_datas.push(new_node_data);
    }

    // assert_eq!(new_node_datas.len(), 4);

    Ok(new_node_datas)
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

    new_node_datas
}

/// Executes the node graph contained in the node.
fn graph(inputs: &[Arc<NodeData>], node: &Arc<Node>, graph: &NodeGraph) -> Vec<Arc<NodeData>> {
    let mut output: Vec<Arc<NodeData>> = Vec::new();
    let mut tex_pro = TextureProcessor::new();
    tex_pro.node_graph = (*graph).clone();

    // Take the `NodeData`s that are fed into this node from the external node and associate
    // them with the correct outputs on the input nodes in the child graph.
    for node_data in inputs {
        let (target_node, target_slot) =
            tex_pro.node_graph.input_mapping(node_data.slot_id).unwrap();

        tex_pro.node_datas.push(Arc::new(NodeData::new(
            target_node,
            target_slot,
            node_data.size,
            Arc::clone(&node_data.buffer),
        )));
    }

    tex_pro.process();

    // Fill the output vector with `NodeData`.
    for output_node_id in tex_pro.node_graph.external_output_ids() {
        for node_data in tex_pro.node_datas(output_node_id) {
            let output_node_data = NodeData::new(
                node.node_id,
                node_data.slot_id,
                node_data.size,
                Arc::clone(&node_data.buffer),
            );
            output.push(Arc::new(output_node_data));
        }
    }

    output
}

fn read(node: Arc<Node>, path: &str) -> Result<Vec<Arc<NodeData>>> {
    let buffers = read_image(&Path::new(path))?;
    let size = Size {
        width: buffers[0].width(),
        height: buffers[0].height(),
    };

    let mut output: Vec<Arc<NodeData>> = Vec::with_capacity(4);
    for (channel, buffer) in buffers.into_iter().enumerate() {
        output.push(Arc::new(NodeData::new(
            node.node_id,
            SlotId(channel as u32),
            size,
            Arc::new(buffer),
        )));
    }

    Ok(output)
}

fn write(inputs: &[Arc<NodeData>], path: &str) -> Result<Vec<Arc<NodeData>>> {
    let channel_vec: Vec<Arc<Buffer>> = inputs
        .iter()
        .map(|node_data| Arc::clone(&node_data.buffer))
        .collect();
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

fn value(node: Arc<Node>, value: f32) -> Vec<Arc<NodeData>> {
    let (width, height) = (1, 1);

    vec![Arc::new(NodeData::new(
        node.node_id,
        SlotId(0),
        Size::new(width, height),
        Arc::new(Box::new(
            ImageBuffer::from_raw(width, height, vec![value]).unwrap(),
        )),
    ))]
}

// The different `ResizePolicy`s need tests.
fn resize_only(
    node_datas: &[Arc<NodeData>],
    resize_policy: Option<ResizePolicy>,
    filter_type: Option<FilterType>,
) -> Result<Vec<Arc<NodeData>>> {
    let size: Size = match resize_policy.unwrap_or_default() {
        ResizePolicy::MostPixels => node_datas
            .iter()
            .map(|node_data| node_data.size)
            .max_by(|size_1, size_2| (size_1.pixel_count()).cmp(&size_2.pixel_count())),
        ResizePolicy::LeastPixels => node_datas
            .iter()
            .map(|node_data| node_data.size)
            .min_by(|size_1, size_2| (size_1.pixel_count()).cmp(&size_2.pixel_count())),
        ResizePolicy::LargestAxes => Some(Size::new(
            node_datas
                .iter()
                .map(|node_data| node_data.size.width)
                .max_by(|width_1, width_2| width_1.cmp(&width_2))
                .unwrap(),
            node_datas
                .iter()
                .map(|node_data| node_data.size.height)
                .max_by(|height_1, height_2| height_1.cmp(&height_2))
                .unwrap(),
        )),
        ResizePolicy::SmallestAxes => Some(Size::new(
            node_datas
                .iter()
                .map(|node_data| node_data.size.width)
                .min_by(|width_1, width_2| width_1.cmp(&width_2))
                .unwrap(),
            node_datas
                .iter()
                .map(|node_data| node_data.size.height)
                .min_by(|height_1, height_2| height_1.cmp(&height_2))
                .unwrap(),
        )),
        ResizePolicy::SpecificSlot(slot_id) => node_datas
            .iter()
            .find(|node_data| node_data.slot_id == slot_id)
            .map(|node_data| node_data.size),
        ResizePolicy::SpecificSize(size) => Some(size),
    }
    .ok_or(TexProError::NodeProcessing)?;

    let filter_type = filter_type.unwrap_or(FilterType::Triangle);

    let mut output_node_datas: Vec<Arc<NodeData>> = Vec::new();

    for node_data in node_datas {
        if node_data.size == size {
            output_node_datas.push(Arc::new(NodeData::new(
                node_data.node_id,
                node_data.slot_id,
                size,
                Arc::clone(&node_data.buffer),
            )));
        } else {
            output_node_datas.push(Arc::new(NodeData::new(
                node_data.node_id,
                node_data.slot_id,
                size,
                Arc::new(Box::new(resize(
                    &**node_data.buffer,
                    size.width,
                    size.height,
                    filter_type,
                ))),
            )));
        }
    }

    Ok(output_node_datas)
}

fn process_resize(
    node_datas: &[Arc<NodeData>],
    node: Arc<Node>,
    resize_policy: Option<ResizePolicy>,
    filter_type: Option<FilterType>,
) -> Result<Vec<Arc<NodeData>>> {
    let node_datas = resize_only(node_datas, resize_policy, filter_type)?;

    let mut output_node_datas: Vec<Arc<NodeData>> = Vec::new();
    for node_data in node_datas {
        output_node_datas.push(Arc::new(NodeData::new(
            node.node_id,
            node_data.slot_id,
            node_data.size,
            Arc::clone(&node_data.buffer),
        )));
    }

    Ok(output_node_datas)
}

// TODO: Look into optimizing this by sampling straight into the un-resized image instead of
// resizing the image before adding.
fn process_add(node_datas: &[Arc<NodeData>], node: Arc<Node>) -> Result<Vec<Arc<NodeData>>> {
    if node_datas.len() != 2 {
        return Err(TexProError::InvalidBufferCount);
    }
    let node_datas = process_resize(&node_datas, Arc::clone(&node), None, None)?;
    let size = node_datas[0].size;

    let buffer: Arc<Buffer> = Arc::new(Box::new(ImageBuffer::from_fn(
        size.width,
        size.height,
        |x, y| {
            Luma([node_datas[0].buffer.get_pixel(x, y).data[0]
                + node_datas[1].buffer.get_pixel(x, y).data[0]])
        },
    )));

    let node_data = Arc::new(NodeData::new(node.node_id, SlotId(0), size, buffer));

    Ok(vec![node_data])
}

// TODO: Look into optimizing this by sampling straight into the un-resized image instead of
// resizing the image before adding.
fn process_subtract(
    node_datas: &[Arc<NodeData>],
    node: Arc<Node>,
    edges: &[Edge],
) -> Result<Vec<Arc<NodeData>>> {
    if node_datas.len() != 2 {
        return Err(TexProError::InvalidBufferCount);
    }
    let node_datas = resize_only(&node_datas, None, None)?;
    let size = node_datas[0].size;

    let left_side_edge = edges
        .iter()
        .find(|edge| edge.input_slot == SlotId(0))
        .ok_or(TexProError::NodeProcessing)?;

    let left_side_node_data = Arc::clone(
        node_datas
            .iter()
            .find(|node_data| {
                node_data.node_id == left_side_edge.output_id
                    && node_data.slot_id == left_side_edge.output_slot
            })
            .ok_or(TexProError::NodeProcessing)?,
    );

    let right_side_node_data = Arc::clone(
        node_datas
            .iter()
            .find(|node_data| **node_data != left_side_node_data)
            .ok_or(TexProError::NodeProcessing)?,
    );

    let buffer: Arc<Buffer> = Arc::new(Box::new(ImageBuffer::from_fn(
        size.width,
        size.height,
        |x, y| {
            let left_side_pixel = left_side_node_data.buffer.get_pixel(x, y).data[0];
            let right_side_pixel = right_side_node_data.buffer.get_pixel(x, y).data[0];
            Luma([left_side_pixel - right_side_pixel])
        },
    )));

    let node_data = Arc::new(NodeData::new(node.node_id, SlotId(0), size, buffer));
    Ok(vec![node_data])
}

fn invert(_input: &[Arc<NodeData>]) -> Vec<Arc<NodeData>> {
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

fn multiply(_input_0: &Arc<NodeData>, _input_1: &Arc<NodeData>) -> Vec<Arc<NodeData>> {
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
