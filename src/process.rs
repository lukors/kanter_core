use crate::{
    error::{Result, TexProError},
    node::*,
    node_graph::*,
    shared::*,
    slot_data::*,
    texture_processor::TextureProcessor,
};
use image::{ImageBuffer, Luma};
use nalgebra::{Cross, Norm, Vector3};
use std::{path::Path, sync::Arc};

pub fn process_node(
    node: Node,
    slot_datas: &[Arc<SlotData>],
    embedded_slot_datas: &[Arc<EmbeddedNodeData>],
    input_slot_datas: &[Arc<SlotData>],
    edges: &[Edge],
) -> Result<Vec<Arc<SlotData>>> {
    assert!(
        slot_datas.len() <= node.capacity(Side::Input),
        "slot_datas.len(): {:?}, node.capacity(Side::Input): {:?}",
        slot_datas.len(),
        node.capacity(Side::Input)
    );
    assert_eq!(edges.len(), slot_datas.len());

    // edges.sort_by(|a, b| a.input_slot.cmp(&b.input_slot));

    let node_datas: Vec<Arc<SlotData>> =
        resize_buffers(&slot_datas, node.resize_policy, node.resize_filter)?;

    let output: Vec<Arc<SlotData>> = match node.node_type {
        NodeType::InputRgba => input_rgba(&node, &input_slot_datas),
        NodeType::InputGray => input_gray(&node, &input_slot_datas),
        NodeType::OutputRgba => output_rgba(&node_datas, edges)?,
        NodeType::OutputGray => output_gray(&node_datas, edges, &node),
        NodeType::Graph(ref node_graph) => graph(&node_datas, &node, node_graph)?,
        NodeType::Image(ref path) => read(&node, path)?,
        NodeType::NodeData(embedded_node_data_id) => {
            image_buffer(&node, embedded_slot_datas, embedded_node_data_id)?
        }
        NodeType::Write(ref path) => write(&node_datas, path)?,
        NodeType::Value(val) => value(&node, val),
        NodeType::Mix(mix_type) => process_mix(&node_datas, &node, edges, mix_type),
        NodeType::HeightToNormal => process_height_to_normal(&node_datas, &node),
    };

    assert!(output.len() <= node.capacity(Side::Output));
    Ok(output)
}

fn input_gray(node: &Node, input_node_datas: &[Arc<SlotData>]) -> Vec<Arc<SlotData>> {
    if let Some(node_data) = input_node_datas
        .iter()
        .find(|nd| nd.node_id == node.node_id)
    {
        vec![Arc::clone(&node_data)]
    } else {
        Vec::new()
    }
}

fn input_rgba(node: &Node, input_node_datas: &[Arc<SlotData>]) -> Vec<Arc<SlotData>> {
    let mut new_node_datas: Vec<Arc<SlotData>> = Vec::with_capacity(node.capacity(Side::Input));

    for node_data in input_node_datas
        .iter()
        .filter(|nd| nd.node_id == node.node_id)
    {
        new_node_datas.push(Arc::clone(&node_data));
    }

    new_node_datas
}

fn image_buffer(
    node: &Node,
    embedded_node_datas: &[Arc<EmbeddedNodeData>],
    embedded_node_data_id: EmbeddedNodeDataId,
) -> Result<Vec<Arc<SlotData>>> {
    if let Some(enode_data) = embedded_node_datas
        .iter()
        .find(|end| end.node_data_id == embedded_node_data_id)
    {
        Ok(vec![Arc::new(SlotData::new(
            node.node_id,
            SlotId(0),
            enode_data.size,
            Arc::clone(&enode_data.buffer),
        ))])
    } else {
        Err(TexProError::NodeProcessing)
    }
}

/// Finds the `NodeData`s relevant for this `Node` and outputs them.
fn output_rgba(node_datas: &[Arc<SlotData>], edges: &[Edge]) -> Result<Vec<Arc<SlotData>>> {
    let mut new_node_datas: Vec<Arc<SlotData>> = Vec::with_capacity(4);

    for edge in edges {
        let node_data = node_datas
            .iter()
            .find(|node_data| {
                node_data.node_id == edge.output_id && node_data.slot_id == edge.output_slot
            })
            .ok_or(TexProError::NodeProcessing)?;

        let new_node_data = Arc::new(SlotData::new(
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
fn output_gray(inputs: &[Arc<SlotData>], edges: &[Edge], node: &Node) -> Vec<Arc<SlotData>> {
    let mut new_node_datas: Vec<Arc<SlotData>> = Vec::with_capacity(1);

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
fn graph(node_datas: &[Arc<SlotData>], node: &Node, graph: &NodeGraph) -> Result<Vec<Arc<SlotData>>> {
    let mut output: Vec<Arc<SlotData>> = Vec::new();
    let tex_pro = TextureProcessor::new();
    tex_pro.set_node_graph((*graph).clone());

    // Take the `NodeData`s that are fed into this node from the parent node and associate
    // them with the correct outputs on the input nodes in the child graph.
    for node_data in node_datas {
        let (target_node, target_slot) = tex_pro.input_mapping(node_data.slot_id)?;

        tex_pro.input_slot_datas_push(Arc::new(SlotData::new(
            target_node,
            target_slot,
            node_data.size,
            Arc::clone(&node_data.buffer),
        )));
    }

    // Fill the output vector with `SlotData`.
    for output_node_id in tex_pro.external_output_ids() {
        for slot_data in tex_pro.node_slot_datas(output_node_id)? {
            let output_node_data = SlotData::new(
                node.node_id,
                slot_data.slot_id,
                slot_data.size,
                Arc::clone(&slot_data.buffer),
            );
            output.push(Arc::new(output_node_data));
        }
    }

    Ok(output)
}

fn read(node: &Node, path: &Path) -> Result<Vec<Arc<SlotData>>> {
    let buffers = read_image(path)?;
    let size = Size {
        width: buffers[0].width(),
        height: buffers[0].height(),
    };

    let mut output: Vec<Arc<SlotData>> = Vec::with_capacity(4);
    for (channel, buffer) in buffers.into_iter().enumerate() {
        output.push(Arc::new(SlotData::new(
            node.node_id,
            SlotId(channel as u32),
            size,
            Arc::new(buffer),
        )));
    }

    Ok(output)
}

fn write(inputs: &[Arc<SlotData>], path: &Path) -> Result<Vec<Arc<SlotData>>> {
    let channel_vec: Vec<Arc<Buffer>> = inputs
        .iter()
        .map(|node_data| Arc::clone(&node_data.buffer))
        .collect();
    let (width, height) = (inputs[0].size.width, inputs[0].size.height);

    image::save_buffer(
        &path,
        &image::RgbaImage::from_vec(width, height, channels_to_rgba(&channel_vec)?).unwrap(),
        width,
        height,
        image::ColorType::RGBA(8),
    )
    .unwrap();

    Ok(Vec::new())
}

fn value(node: &Node, value: f32) -> Vec<Arc<SlotData>> {
    let (width, height) = (1, 1);

    vec![Arc::new(SlotData::new(
        node.node_id,
        SlotId(0),
        Size::new(width, height),
        Arc::new(Box::new(
            ImageBuffer::from_raw(width, height, vec![value]).unwrap(),
        )),
    ))]
}

// TODO: Look into optimizing this by sampling straight into the un-resized image instead of
// resizing the image before blending.
fn process_mix(
    node_datas: &[Arc<SlotData>],
    node: &Node,
    edges: &[Edge],
    mix_type: MixType,
) -> Vec<Arc<SlotData>> {
    if node_datas.is_empty() {
        return Vec::new();
    }

    let size = node_datas[0].size;

    let buffer = match mix_type {
        MixType::Add => process_add(&node_datas, size),
        MixType::Subtract => process_subtract(&node_datas, size, edges),
        MixType::Multiply => process_multiply(&node_datas, size),
        MixType::Divide => process_divide(&node_datas, size, edges),
    };

    vec![Arc::new(SlotData::new(
        node.node_id,
        SlotId(0),
        size,
        buffer,
    ))]
}

fn process_add(node_datas: &[Arc<SlotData>], size: Size) -> Arc<Buffer> {
    Arc::new(Box::new(ImageBuffer::from_fn(
        size.width,
        size.height,
        |x, y| {
            Luma([node_datas
                .iter()
                .map(|nd| nd.buffer.get_pixel(x, y).data[0])
                .sum()])
        },
    )))
}

fn process_subtract(node_datas: &[Arc<SlotData>], size: Size, edges: &[Edge]) -> Arc<Buffer> {
    let mut node_datas = node_datas.to_vec();

    let node_data_first = split_first_node_data(&mut node_datas, size, edges);

    Arc::new(Box::new(ImageBuffer::from_fn(
        size.width,
        size.height,
        |x, y| {
            let value = node_data_first.buffer.get_pixel(x, y).data[0]
                - node_datas
                    .iter()
                    .map(|nd| nd.buffer.get_pixel(x, y).data[0])
                    .sum::<ChannelPixel>();
            Luma([value])
        },
    )))
}

fn split_first_node_data(
    node_datas: &mut Vec<Arc<SlotData>>,
    size: Size,
    edges: &[Edge],
) -> Arc<SlotData> {
    let first_index = first_node_data_index(node_datas, edges);

    // Return the first `NodeData` if there is one. Otherwise return a new `NodeData` filled with black.
    if let Some(index) = first_index {
        node_datas.remove(index)
    } else {
        Arc::new(SlotData::new(
            NodeId(0),
            SlotId(0),
            size,
            Arc::new(Box::new(
                ImageBuffer::from_raw(
                    size.width,
                    size.height,
                    vec![0.; (size.width * size.height) as usize],
                )
                .unwrap(),
            )),
        ))
    }
}

fn process_multiply(node_datas: &[Arc<SlotData>], size: Size) -> Arc<Buffer> {
    Arc::new(Box::new(ImageBuffer::from_fn(
        size.width,
        size.height,
        |x, y| {
            Luma([node_datas
                .iter()
                .map(|nd| nd.buffer.get_pixel(x, y).data[0])
                .product()])
        },
    )))
}

fn process_divide(node_datas: &[Arc<SlotData>], size: Size, edges: &[Edge]) -> Arc<Buffer> {
    let mut node_datas = node_datas.to_vec();

    let node_data_first = split_first_node_data(&mut node_datas, size, edges);

    Arc::new(Box::new(ImageBuffer::from_fn(
        size.width,
        size.height,
        |x, y| {
            Luma([node_data_first.buffer.get_pixel(x, y).data[0]
                / node_datas
                    .iter()
                    .map(|nd| nd.buffer.get_pixel(x, y).data[0])
                    .sum::<ChannelPixel>()])
        },
    )))
}

/// Returns the position of the `NodeData` connected to the first input slot along with the
/// index it was found at.
fn first_node_data_index(node_datas: &[Arc<SlotData>], edges: &[Edge]) -> Option<usize> {
    assert!(
        edges
            .iter()
            .all(|edge| edge.input_id() == edges[0].input_id()),
        "All edges must be connected to the same node"
    );
    {
        let mut input_slots = edges
            .iter()
            .map(|edge| edge.input_slot().0)
            .collect::<Vec<u32>>();
        input_slots.sort_unstable();
        assert!(
            !has_dulicates(&input_slots),
            "A slot cannot have more than one input"
        );
    }
    let edge = edges.iter().find(|edge| edge.input_slot == SlotId(0))?;

    node_datas.iter().position(|node_data| {
        node_data.node_id == edge.output_id && node_data.slot_id == edge.output_slot
    })
}

/// Checks if the input slice has any duplicates.
/// Note: The slice has to be sorted.
fn has_dulicates<T: PartialEq>(slice: &[T]) -> bool {
    for i in 1..slice.len() {
        if slice[i] == slice[i - 1] {
            return true;
        }
    }
    false
}

/// Orders node datas so they are in the same order as the input slots for order-dependent processing.
// fn order_node_datas(node_datas: &[Arc<NodeData>], edges: &[Edge]) -> Vec<Arc<NodeData>> {
//     let mut edges = edges.to_vec();
//     edges.sort_by(|a, b| a.input_slot().partial_cmp(&b.input_slot()).unwrap());

//     edges
//         .iter()
//         .map(|edge| {
//             Arc::clone(
//                 node_datas
//                     .iter()
//                     .find(|nd| nd.slot_id == edge.output_slot() && nd.node_id == edge.output_id())
//                     .unwrap(),
//             )
//         })
//         .collect::<Vec<Arc<NodeData>>>()
// }

fn process_height_to_normal(node_datas: &[Arc<SlotData>], node: &Node) -> Vec<Arc<SlotData>> {
    let channel_count = 3;
    let heightmap = &node_datas[0].buffer;
    let (width, height) = (heightmap.width(), heightmap.height());
    let pixel_distance_x = 1. / width as f32;
    let pixel_distance_y = 1. / height as f32;

    let mut output_buffers: Vec<Buffer> =
        vec![Box::new(ImageBuffer::new(width, height)); channel_count];

    for (x, y, px) in heightmap.enumerate_pixels() {
        let sample_up = heightmap.get_pixel(x, y.wrapping_sample_subtract(1, height))[0];
        let sample_left = heightmap.get_pixel(x.wrapping_sample_subtract(1, width), y)[0];

        let tangent = Vector3::new(pixel_distance_x, 0., px[0] - sample_left).normalize();
        let bitangent = Vector3::new(0., pixel_distance_y, sample_up - px[0]).normalize();
        let normal = tangent.cross(&bitangent).normalize();

        for (i, buffer) in output_buffers.iter_mut().enumerate() {
            buffer.put_pixel(x, y, Luma([normal[i] * 0.5 + 0.5]));
        }
    }

    let mut output_node_datas = Vec::with_capacity(channel_count);
    for (i, buffer) in output_buffers.into_iter().enumerate() {
        output_node_datas.push(Arc::new(SlotData::new(
            node.node_id,
            SlotId(i as u32),
            Size::new(heightmap.width(), heightmap.height()),
            Arc::new(buffer),
        )));
    }

    output_node_datas
}

trait Sampling {
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
