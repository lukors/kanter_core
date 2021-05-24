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
    assert_eq!(
        edges.len(),
        slot_datas.len(),
        "NodeType: {:?}",
        node.node_type
    );

    // Slot datas resized, sorted by input `SlotId` and given the `SlotId` they belong in.
    let slot_datas = {
        let mut edges = edges.to_vec();
        edges.sort_unstable_by(|a, b| a.input_slot.cmp(&b.input_slot));

        let slot_datas: Vec<Arc<SlotData>> =
            resize_buffers(&slot_datas, &edges, node.resize_policy, node.resize_filter)?;

        assign_slot_ids(&slot_datas, &edges)
    };

    let output: Vec<Arc<SlotData>> = match node.node_type {
        NodeType::InputRgba(_) => input_rgba(&node, &input_slot_datas),
        NodeType::InputGray(_) => input_gray(&node, &input_slot_datas),
        NodeType::OutputRgba(_) | NodeType::OutputGray(_) => output(&slot_datas, &node),
        NodeType::Graph(ref node_graph) => process_graph(&slot_datas, &node, node_graph)?,
        NodeType::Image(ref path) => image(&node, path)?,
        NodeType::Embedded(embedded_node_data_id) => {
            image_buffer(&node, embedded_slot_datas, embedded_node_data_id)?
        }
        NodeType::Write(ref path) => write(&slot_datas, path)?,
        NodeType::Value(val) => value(&node, val),
        NodeType::Mix(mix_type) => process_mix(&slot_datas, &node, mix_type),
        NodeType::HeightToNormal => unimplemented!(), // process_height_to_normal(&node_datas, &node),
        NodeType::SplitRgba => split_rgba(&slot_datas, &node),
        NodeType::MergeRgba => merge_rgba(&slot_datas, &node),
    };

    Ok(output)
}

fn assign_slot_ids(slot_datas: &Vec<Arc<SlotData>>, edges: &[Edge]) -> Vec<Arc<SlotData>> {
    edges
        .iter()
        .map(|edge| {
            let slot_data = slot_datas
                .iter()
                .find(|slot_data| {
                    edge.output_slot == slot_data.slot_id && edge.output_id == slot_data.node_id
                })
                .unwrap();
            Arc::new(SlotData::new(
                edge.input_id,
                edge.input_slot,
                slot_data.size,
                Arc::clone(&slot_data.image),
            ))
        })
        .collect::<Vec<Arc<SlotData>>>()
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
    let mut output = (*input_node_datas[0]).clone();
    output.slot_id = SlotId(0);
    output.node_id = node.node_id;

    vec![Arc::new(output)]
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
fn process_graph(
    slot_datas: &[Arc<SlotData>],
    node: &Node,
    graph: &NodeGraph,
) -> Result<Vec<Arc<SlotData>>> {
    let mut output: Vec<Arc<SlotData>> = Vec::new();
    let tex_pro = TextureProcessor::new();
    tex_pro.set_node_graph((*graph).clone())?;

    // Insert `SlotData`s into the graph TexPro.
    for slot_data in slot_datas {
        tex_pro.input_slot_datas_push(Arc::new(SlotData::new(
            NodeId(slot_data.slot_id.0),
            SlotId(0),
            slot_data.size,
            Arc::clone(&slot_data.image),
        )));
    }

    // Fill the output vector with `SlotData`.
    for output_node_id in tex_pro.external_output_ids() {
        for slot_data in tex_pro.node_slot_datas(output_node_id)? {
            let output_node_data = SlotData::new(
                node.node_id,
                SlotId(output_node_id.0),
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

    dbg!(node.input_slot_with_name("left".into()).unwrap().slot_id);
    dbg!(node.input_slot_with_name("right".into()).unwrap().slot_id);
    dbg!(slot_datas.len());

    let size = slot_datas[0].size;
    let is_rgba = Arc::clone(&slot_datas[0].image).is_rgba();

    let image_left = Arc::clone(
        &slot_data_with_name(
            &slot_datas,
            &node,
            "left"
        )
        .unwrap_or_else(|| Arc::new(SlotData::from_value(size, 0.0, is_rgba)))
        .image,
    );

    let image_right = Arc::clone(
        &slot_data_with_name(
            &slot_datas,
            &node,
            "right"
        )
        .unwrap_or_else(|| Arc::new(SlotData::from_value(size, 0.0, is_rgba)))
        .image,
    );

    if image_left.is_rgba() != image_right.is_rgba() {
        return Vec::new();
    }

    let slot_image: SlotImage = match (&*image_left, &*image_right) {
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

fn slot_data_with_name(slot_datas: &[Arc<SlotData>], node: &Node, name: &str) -> Option<Arc<SlotData>> {
    slot_data_with_slot_id(
        &slot_datas,
        node.input_slot_with_name(name.into()).unwrap().slot_id,
    )
}

fn slot_data_with_slot_id(slot_datas: &[Arc<SlotData>], slot_id: SlotId) -> Option<Arc<SlotData>> {
    if let Some(slot_data) = slot_datas
        .iter()
        .find(|slot_data| slot_data.slot_id == slot_id)
    {
        Some(Arc::clone(slot_data))
    } else {
        None
    }
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
