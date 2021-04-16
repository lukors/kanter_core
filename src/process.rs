use crate::{
    dag::*,
    error::{Result, TexProError},
    node::*,
    node_data::*,
    node_graph::*,
    shared::*,
};
use image::{imageops::resize, ImageBuffer, Luma};
use nalgebra::{Cross, Norm, Vector3};
use std::{path::Path, sync::Arc};

pub fn process_node(
    node: Arc<Node>,
    input_node_datas: &[Arc<NodeData>],
    embedded_node_datas: &[Arc<EmbeddedNodeData>],
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
        NodeType::Image(ref path) => read(Arc::clone(&node), path)?,
        NodeType::NodeData(embedded_node_data_id) => {
            image_buffer(&node, embedded_node_datas, embedded_node_data_id)?
        }
        NodeType::Write(ref path) => write(&input_node_datas, path)?,
        NodeType::Value(val) => value(Arc::clone(&node), val),
        NodeType::Resize(resize_policy, filter_type) => process_resize(
            &input_node_datas,
            Arc::clone(&node),
            edges,
            resize_policy,
            filter_type,
        )?,
        NodeType::Mix(mix_type) => {
            process_blend(&input_node_datas, Arc::clone(&node), edges, mix_type)?
        }
        NodeType::HeightToNormal => process_height_to_normal(&input_node_datas, Arc::clone(&node)),
    };

    assert!(output.len() <= node.capacity(Side::Output));
    Ok(output)
}

fn image_buffer(
    node: &Arc<Node>,
    embedded_node_datas: &[Arc<EmbeddedNodeData>],
    embedded_node_data_id: EmbeddedNodeDataId,
) -> Result<Vec<Arc<NodeData>>> {
    if let Some(enode_data) = embedded_node_datas
        .iter()
        .find(|end| end.id == embedded_node_data_id)
    {
        Ok(vec![Arc::new(NodeData::new(
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
    filter_type: Option<ResizeFilter>,
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

    let filter_type = filter_type.unwrap_or(ResizeFilter::Triangle);

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
                    filter_type.into(),
                ))),
            )));
        }
    }

    Ok(output_node_datas)
}

fn process_resize(
    node_datas: &[Arc<NodeData>],
    node: Arc<Node>,
    edges: &[Edge],
    resize_policy: Option<ResizePolicy>,
    filter_type: Option<ResizeFilter>,
) -> Result<Vec<Arc<NodeData>>> {
    let mut output_node_datas: Vec<Arc<NodeData>> = Vec::new();
    let node_datas = resize_only(node_datas, resize_policy, filter_type)?;

    for edge in edges {
        let node_data = node_datas.iter().find(|&nd| nd.node_id == edge.output_id && nd.slot_id == edge.output_slot).expect("Could not find a fitting node_data while resizing");
        
        output_node_datas.push(Arc::new(NodeData::new(
            node.node_id,
            edge.input_slot,
            node_data.size,
            Arc::clone(&node_data.buffer),
        )));
    }

    Ok(output_node_datas)
}

// TODO: Look into optimizing this by sampling straight into the un-resized image instead of
// resizing the image before blending.
fn process_blend(
    node_datas: &[Arc<NodeData>],
    node: Arc<Node>,
    edges: &[Edge],
    mix_type: MixType,
) -> Result<Vec<Arc<NodeData>>> {
    if node_datas.len() != 2 {
        return Err(TexProError::InvalidBufferCount);
    }
    let node_datas = process_resize(&node_datas, Arc::clone(&node), edges, None, None)?;
    let size = node_datas[0].size;

    let buffer = match mix_type {
        MixType::Add => process_add(&node_datas, size),
        MixType::Subtract => process_subtract(&node_datas, size, edges)?,
        MixType::Multiply => process_multiply(&node_datas, size),
        MixType::Divide => process_divide(&node_datas, size, edges)?,
    };

    let node_data = Arc::new(NodeData::new(node.node_id, SlotId(0), size, buffer));

    Ok(vec![node_data])
}

fn process_add(node_datas: &[Arc<NodeData>], size: Size) -> Arc<Buffer> {
    Arc::new(Box::new(ImageBuffer::from_fn(
        size.width,
        size.height,
        |x, y| {
            Luma([node_datas[0].buffer.get_pixel(x, y).data[0]
                + node_datas[1].buffer.get_pixel(x, y).data[0]])
        },
    )))
}

fn process_subtract(
    node_datas: &[Arc<NodeData>],
    size: Size,
    edges: &[Edge],
) -> Result<Arc<Buffer>> {
    let node_datas_ordered = order_node_datas(node_datas, edges)?;

    Ok(Arc::new(Box::new(ImageBuffer::from_fn(
        size.width,
        size.height,
        |x, y| {
            let left_side_pixel = node_datas_ordered[0].buffer.get_pixel(x, y).data[0];
            let right_side_pixel = node_datas_ordered[1].buffer.get_pixel(x, y).data[0];
            Luma([left_side_pixel - right_side_pixel])
        },
    ))))
}

fn process_multiply(node_datas: &[Arc<NodeData>], size: Size) -> Arc<Buffer> {
    Arc::new(Box::new(ImageBuffer::from_fn(
        size.width,
        size.height,
        |x, y| {
            let left_side = node_datas[0].buffer.get_pixel(x, y).data[0];
            let right_side = node_datas[1].buffer.get_pixel(x, y).data[0];

            Luma([left_side * right_side])
        },
    )))
}

fn process_divide(node_datas: &[Arc<NodeData>], size: Size, edges: &[Edge]) -> Result<Arc<Buffer>> {
    let node_datas_ordered = order_node_datas(node_datas, edges)?;

    Ok(Arc::new(Box::new(ImageBuffer::from_fn(
        size.width,
        size.height,
        |x, y| {
            let left_side_pixel = node_datas_ordered[0].buffer.get_pixel(x, y).data[0];
            let right_side_pixel = node_datas_ordered[1].buffer.get_pixel(x, y).data[0];
            Luma([left_side_pixel / right_side_pixel])
        },
    ))))
}

/// Orders two node datas so they are in the correct order for order-dependent processing.
/// Todo: Allow an arbitrary number of inputs.
fn order_node_datas(node_datas: &[Arc<NodeData>], edges: &[Edge]) -> Result<Vec<Arc<NodeData>>> {
    if node_datas.len() != 2 {
        return Err(TexProError::InvalidBufferCount);
    }

    let left_side_edge = edges
        .iter()
        .find(|edge| edge.input_slot == SlotId(0))
        .ok_or(TexProError::NodeProcessing)?;

    let left_side_node_data = Arc::clone(
        node_datas
            .iter()
            .find(|node_data| {
                node_data.node_id == left_side_edge.input_id
                    && node_data.slot_id == left_side_edge.input_slot
            })
            .ok_or(TexProError::NodeProcessing)?,
    );

    let right_side_node_data = Arc::clone(
        node_datas
            .iter()
            .find(|node_data| **node_data != left_side_node_data)
            .ok_or(TexProError::NodeProcessing)?,
    );

    Ok(vec![left_side_node_data, right_side_node_data])
}

fn process_height_to_normal(node_datas: &[Arc<NodeData>], node: Arc<Node>) -> Vec<Arc<NodeData>> {
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
        output_node_datas.push(Arc::new(NodeData::new(
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
