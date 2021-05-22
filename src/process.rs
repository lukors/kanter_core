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
    assert_eq!(
        edges.len(),
        slot_datas.len(),
        "NodeType: {:?}",
        node.node_type
    );

    // Slot datas resized and sorted by input node id.
    let slot_datas = {
        let mut edges = edges.to_vec();
        edges.sort_unstable_by(|a, b| a.input_slot.cmp(&b.input_slot));

        let slot_datas: Vec<Arc<SlotData>> =
            resize_buffers(&slot_datas, &edges, node.resize_policy, node.resize_filter)?;

        edges
            .iter()
            .map(|edge| {
                slot_datas
                    .iter()
                    .find(|slot_data| {
                        edge.output_slot == slot_data.slot_id && edge.output_id == slot_data.node_id
                    })
                    .unwrap()
                    .clone()
            })
            .collect::<Vec<Arc<SlotData>>>()
    };

    let output: Vec<Arc<SlotData>> = match node.node_type {
        NodeType::InputRgba => input_rgba(&node, &input_slot_datas),
        NodeType::InputGray => input_gray(&node, &input_slot_datas),
        NodeType::OutputRgba | NodeType::OutputGray => output(&slot_datas, &node),
        NodeType::Graph(ref node_graph) => graph(&slot_datas, &node, node_graph)?,
        NodeType::Image(ref path) => image(&node, path)?,
        NodeType::NodeData(embedded_node_data_id) => {
            image_buffer(&node, embedded_slot_datas, embedded_node_data_id)?
        }
        NodeType::Write(ref path) => write(&slot_datas, path)?,
        NodeType::Value(val) => value(&node, val),
        NodeType::Mix(mix_type) => process_mix(&slot_datas, &node, mix_type),
        NodeType::HeightToNormal => unimplemented!(), // process_height_to_normal(&node_datas, &node),
        NodeType::SplitRgba => split_rgba(&slot_datas, &node),
        NodeType::MergeRgba => merge_rgba(&slot_datas, &node),
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
            Arc::clone(&enode_data.image),
        ))])
    } else {
        Err(TexProError::NodeProcessing)
    }
}

fn output(node_datas: &[Arc<SlotData>], node: &Node) -> Vec<Arc<SlotData>> {
    let mut output = (*node_datas[0]).clone();
    output.node_id = node.node_id;
    output.slot_id = SlotId(0);

    vec![Arc::new(output)]
}

/// Executes the node graph contained in the node.
fn graph(
    node_datas: &[Arc<SlotData>],
    node: &Node,
    graph: &NodeGraph,
) -> Result<Vec<Arc<SlotData>>> {
    let mut output: Vec<Arc<SlotData>> = Vec::new();
    let tex_pro = TextureProcessor::new();
    tex_pro.set_node_graph((*graph).clone())?;

    // Take the `NodeData`s that are fed into this node from the parent node and associate
    // them with the correct outputs on the input nodes in the child graph.
    for node_data in node_datas {
        let (target_node, target_slot) = tex_pro.input_mapping(node_data.slot_id)?;

        tex_pro.input_slot_datas_push(Arc::new(SlotData::new(
            target_node,
            target_slot,
            node_data.size,
            Arc::clone(&node_data.image),
        )));
    }

    // Fill the output vector with `SlotData`.
    for output_node_id in tex_pro.external_output_ids() {
        for slot_data in tex_pro.node_slot_datas(output_node_id)? {
            let output_node_data = SlotData::new(
                node.node_id,
                slot_data.slot_id,
                slot_data.size,
                Arc::clone(&slot_data.image),
            );
            output.push(Arc::new(output_node_data));
        }
    }

    Ok(output)
}

fn image(node: &Node, path: &Path) -> Result<Vec<Arc<SlotData>>> {
    let slot_image = read_slot_image(path)?;
    Ok(vec![Arc::new(SlotData::new(
        node.node_id,
        SlotId(0),
        slot_image.size(),
        Arc::new(slot_image),
    ))])
}

fn write(slot_datas: &[Arc<SlotData>], path: &Path) -> Result<Vec<Arc<SlotData>>> {
    if let Some(slot_data) = slot_datas.get(0) {
        let (width, height) = (slot_data.size.width, slot_data.size.height);

        image::save_buffer(
            &path,
            &image::RgbaImage::from_vec(width, height, slot_data.image.to_rgba()).unwrap(),
            width,
            height,
            image::ColorType::RGBA(8),
        )
        .unwrap();
    }

    Ok(Vec::new())
}

fn value(node: &Node, value: f32) -> Vec<Arc<SlotData>> {
    let (width, height) = (1, 1);

    vec![Arc::new(SlotData::new(
        node.node_id,
        SlotId(0),
        Size::new(width, height),
        Arc::new(SlotImage::Gray(Arc::new(Box::new(
            ImageBuffer::from_raw(width, height, vec![value]).unwrap(),
        )))),
    ))]
}

// TODO: Look into optimizing this by sampling straight into the un-resized image instead of
// resizing the image before blending.
fn process_mix(slot_datas: &[Arc<SlotData>], node: &Node, mix_type: MixType) -> Vec<Arc<SlotData>> {
    if slot_datas.is_empty() {
        return Vec::new();
    }

    let size = slot_datas[0].size;

    // Since the tests for rgba SlotDatas almost all work now, maybe its time to tackle handling
    // Slots as real things rather than amounts on each side of nodes.
    //
    // For this function I need to specifically know which slot is occupied to know what to do,
    // and it would be trivial and less error prone to do that if slots were named etc.
    //
    // I need to do that either way so might as well do it now.
    
    let slot_image: SlotImage = match (&*slot_datas[0].image, &*slot_datas[1].image) {
        (SlotImage::Gray(left), SlotImage::Gray(right)) => {
            SlotImage::Gray(Arc::new(Box::new(match mix_type {
                MixType::Add => process_add_gray(left, right, size),
                MixType::Subtract => process_subtract_gray(left, right, size),
                MixType::Multiply => process_multiply_gray(left, right, size),
                MixType::Divide => process_divide_gray(left, right, size),
                MixType::Pow => process_pow_gray(left, right, size),
            })))
        }
        (SlotImage::Rgba(left), SlotImage::Rgba(right)) => SlotImage::Rgba(match mix_type {
            MixType::Add => process_add_rgba(left, right, size),
            MixType::Subtract => process_subtract_rgba(left, right, size),
            MixType::Multiply => process_multiply_rgba(left, right, size),
            MixType::Divide => process_divide_rgba(left, right, size),
            MixType::Pow => process_pow_rgba(left, right, size),
        }),
        _ => return Vec::new(),
    };

    vec![Arc::new(SlotData::new(
        node.node_id,
        SlotId(0),
        size,
        Arc::new(slot_image),
    ))]
}

fn process_add_gray(left: &Arc<BoxBuffer>, right: &Arc<BoxBuffer>, size: Size) -> Buffer {
    ImageBuffer::from_fn(size.width, size.height, |x, y| {
        Luma([left.get_pixel(x, y).data[0] + right.get_pixel(x, y).data[0]])
    })
}

fn process_subtract_gray(left: &Arc<BoxBuffer>, right: &Arc<BoxBuffer>, size: Size) -> Buffer {
    ImageBuffer::from_fn(size.width, size.height, |x, y| {
        Luma([left.get_pixel(x, y).data[0] - right.get_pixel(x, y).data[0]])
    })
}

fn process_multiply_gray(left: &Arc<BoxBuffer>, right: &Arc<BoxBuffer>, size: Size) -> Buffer {
    ImageBuffer::from_fn(size.width, size.height, |x, y| {
        Luma([left.get_pixel(x, y).data[0] * right.get_pixel(x, y).data[0]])
    })
}

fn process_divide_gray(left: &Arc<BoxBuffer>, right: &Arc<BoxBuffer>, size: Size) -> Buffer {
    ImageBuffer::from_fn(size.width, size.height, |x, y| {
        Luma([left.get_pixel(x, y).data[0] / right.get_pixel(x, y).data[0]])
    })
}

fn process_pow_gray(left: &Arc<BoxBuffer>, right: &Arc<BoxBuffer>, size: Size) -> Buffer {
    ImageBuffer::from_fn(size.width, size.height, |x, y| {
        Luma([left.get_pixel(x, y).data[0].powf(right.get_pixel(x, y).data[0])])
    })
}

fn process_add_rgba(
    left: &[Arc<BoxBuffer>],
    right: &[Arc<BoxBuffer>],
    size: Size,
) -> [Arc<Box<Buffer>>; 4] {
    [
        Arc::new(Box::new(process_add_gray(&left[0], &right[0], size))),
        Arc::new(Box::new(process_add_gray(&left[1], &right[1], size))),
        Arc::new(Box::new(process_add_gray(&left[2], &right[2], size))),
        Arc::new(Box::new(
            Buffer::from_raw(
                size.width,
                size.height,
                vec![1.0; (size.width * size.height) as usize],
            )
            .unwrap(),
        )),
    ]
}

fn process_subtract_rgba(
    left: &[Arc<BoxBuffer>],
    right: &[Arc<BoxBuffer>],
    size: Size,
) -> [Arc<Box<Buffer>>; 4] {
    [
        Arc::new(Box::new(process_subtract_gray(&left[0], &right[0], size))),
        Arc::new(Box::new(process_subtract_gray(&left[1], &right[1], size))),
        Arc::new(Box::new(process_subtract_gray(&left[2], &right[2], size))),
        Arc::new(Box::new(
            Buffer::from_raw(
                size.width,
                size.height,
                vec![1.0; (size.width * size.height) as usize],
            )
            .unwrap(),
        )),
    ]
}

fn process_multiply_rgba(
    left: &[Arc<BoxBuffer>],
    right: &[Arc<BoxBuffer>],
    size: Size,
) -> [Arc<Box<Buffer>>; 4] {
    [
        Arc::new(Box::new(process_multiply_gray(&left[0], &right[0], size))),
        Arc::new(Box::new(process_multiply_gray(&left[1], &right[1], size))),
        Arc::new(Box::new(process_multiply_gray(&left[2], &right[2], size))),
        Arc::new(Box::new(
            Buffer::from_raw(
                size.width,
                size.height,
                vec![1.0; (size.width * size.height) as usize],
            )
            .unwrap(),
        )),
    ]
}

fn process_divide_rgba(
    left: &[Arc<BoxBuffer>],
    right: &[Arc<BoxBuffer>],
    size: Size,
) -> [Arc<Box<Buffer>>; 4] {
    [
        Arc::new(Box::new(process_divide_gray(&left[0], &right[0], size))),
        Arc::new(Box::new(process_divide_gray(&left[1], &right[1], size))),
        Arc::new(Box::new(process_divide_gray(&left[2], &right[2], size))),
        Arc::new(Box::new(
            Buffer::from_raw(
                size.width,
                size.height,
                vec![1.0; (size.width * size.height) as usize],
            )
            .unwrap(),
        )),
    ]
}

fn process_pow_rgba(
    left: &[Arc<BoxBuffer>],
    right: &[Arc<BoxBuffer>],
    size: Size,
) -> [Arc<Box<Buffer>>; 4] {
    [
        Arc::new(Box::new(process_pow_gray(&left[0], &right[0], size))),
        Arc::new(Box::new(process_pow_gray(&left[1], &right[1], size))),
        Arc::new(Box::new(process_pow_gray(&left[2], &right[2], size))),
        Arc::new(Box::new(
            Buffer::from_raw(
                size.width,
                size.height,
                vec![1.0; (size.width * size.height) as usize],
            )
            .unwrap(),
        )),
    ]
}

fn process_height_to_normal(node_datas: &[Arc<SlotData>], node: &Node) -> Vec<Arc<SlotData>> {
    unimplemented!()
    // let channel_count = 3;
    // let heightmap = &node_datas[0].image;
    // let (width, height) = (heightmap.width(), heightmap.height());
    // let pixel_distance_x = 1. / width as f32;
    // let pixel_distance_y = 1. / height as f32;

    // let mut output_buffers: Vec<BoxBuffer> =
    //     vec![Box::new(ImageBuffer::new(width, height)); channel_count];

    // for (x, y, px) in heightmap.enumerate_pixels() {
    //     let sample_up = heightmap.get_pixel(x, y.wrapping_sample_subtract(1, height))[0];
    //     let sample_left = heightmap.get_pixel(x.wrapping_sample_subtract(1, width), y)[0];

    //     let tangent = Vector3::new(pixel_distance_x, 0., px[0] - sample_left).normalize();
    //     let bitangent = Vector3::new(0., pixel_distance_y, sample_up - px[0]).normalize();
    //     let normal = tangent.cross(&bitangent).normalize();

    //     for (i, buffer) in output_buffers.iter_mut().enumerate() {
    //         buffer.put_pixel(x, y, Luma([normal[i] * 0.5 + 0.5]));
    //     }
    // }

    // let mut output_node_datas = Vec::with_capacity(channel_count);
    // for (i, buffer) in output_buffers.into_iter().enumerate() {
    //     output_node_datas.push(Arc::new(SlotData::new(
    //         node.node_id,
    //         SlotId(i as u32),
    //         Size::new(heightmap.width(), heightmap.height()),
    //         Arc::new(buffer),
    //     )));
    // }

    // output_node_datas
}

fn split_rgba(slot_datas: &[Arc<SlotData>], node: &Node) -> Vec<Arc<SlotData>> {
    if let SlotImage::Rgba(buf) = &*slot_datas[0].image {
        let size = slot_datas[0].size;
        vec![
            Arc::new(SlotData::new(
                node.node_id,
                SlotId(0),
                size,
                Arc::new(SlotImage::Gray(Arc::clone(&buf[0]))),
            )),
            Arc::new(SlotData::new(
                node.node_id,
                SlotId(1),
                size,
                Arc::new(SlotImage::Gray(Arc::clone(&buf[1]))),
            )),
            Arc::new(SlotData::new(
                node.node_id,
                SlotId(2),
                size,
                Arc::new(SlotImage::Gray(Arc::clone(&buf[2]))),
            )),
            Arc::new(SlotData::new(
                node.node_id,
                SlotId(3),
                size,
                Arc::new(SlotImage::Gray(Arc::clone(&buf[3]))),
            )),
        ]
    } else {
        Vec::new()
    }
}

fn merge_rgba(slot_datas: &[Arc<SlotData>], node: &Node) -> Vec<Arc<SlotData>> {
    fn rgba_slot_data_to_buffer(
        slot_data: Option<&Arc<SlotData>>,
        buffer_default: &Arc<Box<Buffer>>,
    ) -> Arc<Box<Buffer>> {
        if let Some(slot_data) = slot_data {
            if let SlotImage::Gray(buf) = &*slot_data.image {
                Arc::clone(&buf)
            } else {
                Arc::clone(&buffer_default)
            }
        } else {
            Arc::clone(&buffer_default)
        }
    }

    let size = slot_datas[0].size;

    let buffer_default = Arc::new(Box::new(
        Buffer::from_raw(
            size.width,
            size.height,
            vec![1.0; (size.width * size.height) as usize],
        )
        .unwrap(),
    ));

    vec![Arc::new(SlotData::new(
        node.node_id,
        SlotId(0),
        size,
        Arc::new(SlotImage::Rgba([
            rgba_slot_data_to_buffer(slot_datas.get(0), &buffer_default),
            rgba_slot_data_to_buffer(slot_datas.get(1), &buffer_default),
            rgba_slot_data_to_buffer(slot_datas.get(2), &buffer_default),
            rgba_slot_data_to_buffer(slot_datas.get(3), &buffer_default),
        ])),
    ))]
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
