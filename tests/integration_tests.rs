use kanter_core::{
    node::{EmbeddedNodeDataId, MixType, Node, NodeType, ResizeFilter, ResizePolicy},
    node_graph::{NodeGraph, SlotId},
    slot_data::Size,
    texture_processor::TextureProcessor,
};
use ntest::timeout;
use std::{fs::create_dir, path::Path, sync::Arc};

const OUT_DIR: &str = "out";
const IMAGE_1: &str = "data/image_1.png";
const IMAGE_2: &str = "data/image_2.png";
const HEART_128: &str = "data/heart_128.png";
const HEART_256: &str = "data/heart_256.png";
const HEART_WIDE: &str = "data/heart_wide.png";
const HEART_TALL: &str = "data/heart_tall.png";
const HEART_110: &str = "data/heart_110.png";
const CLOUDS: &str = "data/clouds.png";

fn ensure_out_dir() {
    match create_dir(Path::new(OUT_DIR)) {
        _ => (),
    };
}

fn images_equal<P: AsRef<Path>, Q: AsRef<Path>>(path_1: P, path_2: Q) -> bool {
    let image_1 = image::open(path_1).unwrap();
    let raw_pixels_1 = image_1.raw_pixels();
    let image_2 = image::open(path_2).unwrap();
    let raw_pixels_2 = image_2.raw_pixels();

    raw_pixels_1.iter().eq(raw_pixels_2.iter())
}

#[test]
#[timeout(20000)]
fn input_output() {
    const SIZE: u32 = 256;
    const PATH_IN: &str = IMAGE_2;
    const PATH_OUT: &str = &"out/input_output.png";

    let tex_pro = TextureProcessor::new();

    let input_node = tex_pro
        .add_node(Node::new(NodeType::Image(PATH_IN.clone().into())))
        .unwrap();
    let output_node = tex_pro.add_node(Node::new(NodeType::OutputRgba)).unwrap();

    for i in 0..4 {
        tex_pro
            .connect(input_node, output_node, SlotId(i), SlotId(i))
            .unwrap();
    }

    ensure_out_dir();
    image::save_buffer(
        &Path::new(&PATH_OUT),
        &image::RgbaImage::from_vec(
            SIZE,
            SIZE,
            tex_pro.get_output_rgba(output_node, SlotId(0)).unwrap(),
        )
        .unwrap(),
        SIZE,
        SIZE,
        image::ColorType::RGBA(8),
    )
    .unwrap();

    assert!(images_equal(PATH_IN, PATH_OUT));
}

#[test]
#[timeout(20000)]
fn request_empty_buffer() {
    let tex_pro = TextureProcessor::new();

    let mix_node = tex_pro
        .add_node(Node::new(NodeType::Mix(MixType::default())))
        .unwrap();
    let output_node = tex_pro.add_node(Node::new(NodeType::OutputRgba)).unwrap();

    tex_pro
        .connect(mix_node, output_node, SlotId(0), SlotId(0))
        .unwrap();

    #[allow(unused_variables)]
    let nothing = tex_pro.get_output_rgba(output_node, SlotId(0)).unwrap();
}

#[test]
fn input_output_intercept() {
    const SIZE: u32 = 256;
    const SIZE_LARGE: u32 = 200;
    const SIZE_SMALL: u32 = 128;
    const PATH_IN: &str = IMAGE_2;
    const PATH_OUT_INTERCEPT: &str = &"out/input_output_intercept.png";
    const PATH_OUT: &str = &"out/input_output_intercept_out.png";

    let tex_pro = TextureProcessor::new();

    let input_node = tex_pro
        .add_node(Node::new(NodeType::Image(PATH_IN.clone().into())))
        .unwrap();
    let resize_node_1 = tex_pro
        .add_node(
            Node::new(NodeType::OutputRgba)
                .resize_filter(ResizeFilter::Lanczos3)
                .resize_policy(ResizePolicy::SpecificSize(Size::new(
                    SIZE_SMALL, SIZE_SMALL,
                ))),
        )
        .unwrap();
    let resize_node_2 = tex_pro
        .add_node(
            Node::new(NodeType::OutputRgba)
                .resize_filter(ResizeFilter::Lanczos3)
                .resize_policy(ResizePolicy::SpecificSize(Size::new(
                    SIZE_LARGE, SIZE_LARGE,
                ))),
        )
        .unwrap();
    let resize_node_3 = tex_pro
        .add_node(
            Node::new(NodeType::OutputRgba)
                .resize_filter(ResizeFilter::Lanczos3)
                .resize_policy(ResizePolicy::SpecificSize(Size::new(SIZE, SIZE))),
        )
        .unwrap();
    let output_node = tex_pro.add_node(Node::new(NodeType::OutputRgba)).unwrap();

    for i in 0..4 {
        tex_pro
            .connect(input_node, resize_node_1, SlotId(i), SlotId(i))
            .unwrap();
        tex_pro
            .connect(resize_node_1, resize_node_2, SlotId(i), SlotId(i))
            .unwrap();
        tex_pro
            .connect(resize_node_2, resize_node_3, SlotId(i), SlotId(i))
            .unwrap();
        tex_pro
            .connect(resize_node_3, output_node, SlotId(i), SlotId(i))
            .unwrap();
    }

    let mut intercepted = false;
    loop {
        if !intercepted {
            if let Ok(buffer) = tex_pro.try_get_output_rgba(resize_node_1, SlotId(0)) {
                ensure_out_dir();
                image::save_buffer(
                    &Path::new(&PATH_OUT_INTERCEPT),
                    &image::RgbaImage::from_vec(SIZE_SMALL, SIZE_SMALL, buffer).unwrap(),
                    SIZE_SMALL,
                    SIZE_SMALL,
                    image::ColorType::RGBA(8),
                )
                .unwrap();
                intercepted = true;
            }
        }

        if let Ok(buffer) = tex_pro.try_get_output_rgba(output_node, SlotId(0)) {
            ensure_out_dir();
            image::save_buffer(
                &Path::new(&PATH_OUT),
                &image::RgbaImage::from_vec(SIZE, SIZE, buffer).unwrap(),
                SIZE,
                SIZE,
                image::ColorType::RGBA(8),
            )
            .unwrap();

            break;
        }
    }
}

#[test]
#[timeout(20000)]
fn mix_node_single_input() {
    const SIZE: u32 = 256;
    let path_in = IMAGE_2.to_string();
    const PATH_OUT: &str = &"out/mix_node_single_input.png";
    const PATH_CMP: &str = &"data/test_compare/mix_node_single_input.png";

    let tex_pro = TextureProcessor::new();

    let value_node = tex_pro
        .add_node(Node::new(NodeType::Image(path_in.clone().into())))
        .unwrap();
    let mix_node = tex_pro
        .add_node(Node::new(NodeType::Mix(MixType::Add)))
        .unwrap();
    let output_node = tex_pro.add_node(Node::new(NodeType::OutputGray)).unwrap();

    tex_pro
        .connect(value_node, mix_node, SlotId(0), SlotId(0))
        .unwrap();
    tex_pro
        .connect(mix_node, output_node, SlotId(0), SlotId(0))
        .unwrap();

    let output = tex_pro.get_output_rgba(output_node, SlotId(0)).unwrap();

    ensure_out_dir();
    image::save_buffer(
        &Path::new(&PATH_OUT),
        &image::RgbaImage::from_vec(SIZE, SIZE, output).unwrap(),
        SIZE,
        SIZE,
        image::ColorType::RGBA(8),
    )
    .unwrap();

    assert!(images_equal(PATH_CMP, PATH_OUT));
}

#[test]
#[timeout(20000)]
fn mix_node_single_input_2() {
    const SIZE: u32 = 256;
    let path_in = IMAGE_2.to_string();
    const PATH_OUT: &str = &"out/mix_node_single_input_2.png";
    const PATH_CMP: &str = &"data/test_compare/mix_node_single_input_2.png";

    let tex_pro = TextureProcessor::new();

    let value_node = tex_pro
        .add_node(Node::new(NodeType::Image(path_in.clone().into())))
        .unwrap();
    let mix_node = tex_pro
        .add_node(Node::new(NodeType::Mix(MixType::Subtract)))
        .unwrap();
    let output_node = tex_pro.add_node(Node::new(NodeType::OutputGray)).unwrap();

    tex_pro
        .connect(value_node, mix_node, SlotId(0), SlotId(1))
        .unwrap();
    tex_pro
        .connect(mix_node, output_node, SlotId(0), SlotId(0))
        .unwrap();

    let output = tex_pro.get_output_rgba(output_node, SlotId(0)).unwrap();

    ensure_out_dir();
    image::save_buffer(
        &Path::new(&PATH_OUT),
        &image::RgbaImage::from_vec(SIZE, SIZE, output).unwrap(),
        SIZE,
        SIZE,
        image::ColorType::RGBA(8),
    )
    .unwrap();

    assert!(images_equal(PATH_CMP, PATH_OUT));
}

#[test]
#[timeout(20000)]
fn unconnected() {
    let tex_pro = TextureProcessor::new();

    tex_pro.add_node(Node::new(NodeType::OutputRgba)).unwrap();
}

#[test]
#[timeout(20000)]
fn embedded_node_data() {
    let path_in = IMAGE_1.to_string();
    let path_out = "out/embedded_node_data.png".to_string();

    let tex_pro_1 = TextureProcessor::new();

    let tp1_input_node = tex_pro_1
        .add_node(Node::new(NodeType::Image(path_in.clone().into())))
        .unwrap();
    let tp1_output_node = tex_pro_1.add_node(Node::new(NodeType::OutputRgba)).unwrap();

    for i in 0..4 {
        tex_pro_1
            .connect(tp1_input_node, tp1_output_node, SlotId(i), SlotId(i))
            .unwrap();
    }

    let node_data = tex_pro_1.node_slot_data(tp1_output_node).unwrap();

    // Second graph
    let tex_pro_2 = TextureProcessor::new();

    let tp2_output_node = tex_pro_2.add_node(Node::new(NodeType::OutputRgba)).unwrap();

    for i in 0..4 {
        let end_id = tex_pro_2
            .embed_slot_data_with_id(Arc::clone(&node_data[i]), EmbeddedNodeDataId(i as u32))
            .unwrap();

        let input = tex_pro_2
            .add_node(Node::new(NodeType::NodeData(end_id)))
            .unwrap();

        tex_pro_2
            .connect(input, tp2_output_node, SlotId(0), SlotId(i as u32))
            .unwrap();
    }

    ensure_out_dir();
    image::save_buffer(
        &Path::new(&path_out),
        &image::RgbaImage::from_vec(
            256,
            256,
            tex_pro_2
                .get_output_rgba(tp2_output_node, SlotId(0))
                .unwrap(),
        )
        .unwrap(),
        256,
        256,
        image::ColorType::RGBA(8),
    )
    .unwrap();

    assert!(images_equal(path_in, path_out));
}

#[test]
#[timeout(20000)]
fn repeat_process() {
    let tex_pro = TextureProcessor::new();

    let input_node = tex_pro
        .add_node(Node::new(NodeType::Image("data/image_1.png".into())))
        .unwrap();
    let output_node = tex_pro.add_node(Node::new(NodeType::OutputRgba)).unwrap();

    tex_pro
        .connect(input_node, output_node, SlotId(0), SlotId(0))
        .unwrap();
}

#[test]
#[timeout(20000)]
fn mix_images() {
    let path_in_1 = IMAGE_1.to_string();
    let path_in_2 = IMAGE_2.to_string();
    let path_out = "out/mix_images.png".to_string();
    let path_compare = "data/test_compare/mix_images.png".to_string();

    let tex_pro = TextureProcessor::new();

    let input_1 = tex_pro
        .add_node(Node::new(NodeType::Image(path_in_1.into())))
        .unwrap();
    let input_2 = tex_pro
        .add_node(Node::new(NodeType::Image(path_in_2.into())))
        .unwrap();
    let output_node = tex_pro.add_node(Node::new(NodeType::OutputRgba)).unwrap();

    tex_pro
        .connect(input_1, output_node, SlotId(3), SlotId(0))
        .unwrap();
    tex_pro
        .connect(input_1, output_node, SlotId(1), SlotId(1))
        .unwrap();
    tex_pro
        .connect(input_2, output_node, SlotId(2), SlotId(2))
        .unwrap();
    tex_pro
        .connect(input_2, output_node, SlotId(3), SlotId(3))
        .unwrap();

    ensure_out_dir();
    image::save_buffer(
        &Path::new(&path_out),
        &image::RgbaImage::from_vec(
            256,
            256,
            tex_pro.get_output_rgba(output_node, SlotId(0)).unwrap(),
        )
        .unwrap(),
        256,
        256,
        image::ColorType::RGBA(8),
    )
    .unwrap();

    assert!(images_equal(path_out, path_compare))
}

#[test]
#[timeout(20000)]
fn irregular_sizes() {
    const PATH_OUT: &str = &"out/irregular_sizes.png";
    const PATH_CMP: &str = &"data/test_compare/irregular_sizes.png";

    let tex_pro = TextureProcessor::new();

    let input_1 = tex_pro
        .add_node(Node::new(NodeType::Image(HEART_128.into())))
        .unwrap();
    let input_2 = tex_pro
        .add_node(Node::new(NodeType::Image(HEART_110.into())))
        .unwrap();
    let output_node = tex_pro.add_node(Node::new(NodeType::OutputRgba)).unwrap();

    tex_pro
        .connect(input_1, output_node, SlotId(0), SlotId(0))
        .unwrap();
    tex_pro
        .connect(input_2, output_node, SlotId(0), SlotId(1))
        .unwrap();

    let size = tex_pro
        .await_slot_data_size(output_node, SlotId(0))
        .unwrap();

    ensure_out_dir();
    image::save_buffer(
        &Path::new(PATH_OUT),
        &image::RgbaImage::from_vec(
            size.width,
            size.height,
            tex_pro.get_output_rgba(output_node, SlotId(0)).unwrap(),
        )
        .unwrap(),
        size.width,
        size.height,
        image::ColorType::RGBA(8),
    )
    .unwrap();

    assert!(images_equal(PATH_OUT, PATH_CMP));
}

#[test]
#[timeout(20000)]
fn unconnected_node() {
    let tex_pro = TextureProcessor::new();

    let input_1 = tex_pro.add_node(Node::new(NodeType::Value(0.0))).unwrap();
    tex_pro.add_node(Node::new(NodeType::Value(0.0))).unwrap();
    let output_node = tex_pro.add_node(Node::new(NodeType::OutputGray)).unwrap();

    tex_pro
        .connect(input_1, output_node, SlotId(0), SlotId(0))
        .unwrap();
}

#[test]
#[timeout(20000)]
fn resize_rgba() {
    const SIZE: u32 = 256;
    const IN_PATH: &str = &"data/image_2.png";
    const OUT_PATH: &str = &"out/resize_rgba.png";
    let tex_pro = TextureProcessor::new();

    let n_in = tex_pro
        .add_node(Node::new(NodeType::Image(IN_PATH.into())))
        .unwrap();

    let n_out = tex_pro
        .add_node(
            Node::new(NodeType::OutputRgba)
                .resize_policy(ResizePolicy::SpecificSize(Size::new(SIZE, SIZE))),
        )
        .unwrap();

    for i in 0..4 {
        tex_pro
            .connect(n_in, n_out, SlotId(i as u32), SlotId(i as u32))
            .unwrap();
    }

    ensure_out_dir();
    image::save_buffer(
        &Path::new(OUT_PATH),
        &image::RgbaImage::from_vec(
            SIZE,
            SIZE,
            tex_pro.get_output_rgba(n_out, SlotId(0)).unwrap(),
        )
        .unwrap(),
        SIZE,
        SIZE,
        image::ColorType::RGBA(8),
    )
    .unwrap();

    assert!(images_equal(OUT_PATH, IN_PATH));
}

// #[test]
// #[timeout(20000)]
// fn input_output_2() {
//     let tex_pro_compare = input_output_2_internal();

//     for _ in 0..30 {
//         let tex_pro = input_output_2_internal();

//         for node_data_cmp in &tex_pro_compare.slot_datas() {
//             assert!(tex_pro
//                 .slot_datas()
//                 .iter()
//                 .any(|node_data| *node_data == *node_data_cmp));
//         }
//     }
// }

fn input_output_2_internal() -> TextureProcessor {
    let tex_pro = TextureProcessor::new();

    let input_node_1 = tex_pro
        .add_node(Node::new(NodeType::Image("data/px_1.png".into())))
        .unwrap();
    let input_node_2 = tex_pro
        .add_node(Node::new(NodeType::Image("data/px_1.png".into())))
        .unwrap();
    let output_node = tex_pro.add_node(Node::new(NodeType::OutputRgba)).unwrap();

    tex_pro
        .connect(input_node_2, output_node, SlotId(2), SlotId(2))
        .unwrap();
    tex_pro
        .connect(input_node_1, output_node, SlotId(0), SlotId(0))
        .unwrap();
    tex_pro
        .connect(input_node_1, output_node, SlotId(1), SlotId(1))
        .unwrap();
    tex_pro
        .connect(input_node_2, output_node, SlotId(3), SlotId(3))
        .unwrap();

    tex_pro
}

#[test]
#[timeout(20000)]
fn remove_node() {
    let tex_pro = TextureProcessor::new();

    let value_node = tex_pro.add_node(Node::new(NodeType::Value(0.))).unwrap();

    tex_pro.remove_node(value_node).unwrap();

    assert_eq!(tex_pro.node_ids().len(), 0);
}

#[test]
fn connect_invalid_slot() {
    let tex_pro = TextureProcessor::new();

    let value_node = tex_pro.add_node(Node::new(NodeType::Value(0.))).unwrap();

    let output_node = tex_pro.add_node(Node::new(NodeType::OutputRgba)).unwrap();

    assert!(tex_pro
        .connect(value_node, output_node, SlotId(0), SlotId(0))
        .is_ok());
    assert!(tex_pro
        .connect(value_node, output_node, SlotId(0), SlotId(1))
        .is_ok());
    assert!(tex_pro
        .connect(value_node, output_node, SlotId(0), SlotId(2))
        .is_ok());
    assert!(tex_pro
        .connect(value_node, output_node, SlotId(0), SlotId(3))
        .is_ok());
    assert!(tex_pro
        .connect(value_node, output_node, SlotId(0), SlotId(4))
        .is_err());
    assert!(tex_pro
        .connect(value_node, output_node, SlotId(1), SlotId(0))
        .is_err());
}

#[test]
#[timeout(20000)]
fn value_node() {
    const PATH_OUT: &str = &"out/value_node.png";
    const PATH_CMP: &str = &"data/test_compare/value_node.png";

    let tex_pro = TextureProcessor::new();

    let red_node = tex_pro.add_node(Node::new(NodeType::Value(0.))).unwrap();
    let green_node = tex_pro.add_node(Node::new(NodeType::Value(0.33))).unwrap();
    let blue_node = tex_pro.add_node(Node::new(NodeType::Value(0.66))).unwrap();
    let alpha_node = tex_pro.add_node(Node::new(NodeType::Value(1.))).unwrap();

    let output_node = tex_pro.add_node(Node::new(NodeType::OutputRgba)).unwrap();

    let node_ids = [red_node, green_node, blue_node, alpha_node];
    for i in 0..4 {
        tex_pro
            .connect(node_ids[i], output_node, SlotId(0), SlotId(i as u32))
            .unwrap();
    }

    ensure_out_dir();
    image::save_buffer(
        &Path::new(PATH_OUT),
        &image::RgbaImage::from_vec(
            1,
            1,
            tex_pro.get_output_rgba(output_node, SlotId(0)).unwrap(),
        )
        .unwrap(),
        1,
        1,
        image::ColorType::RGBA(8),
    )
    .unwrap();

    assert!(images_equal(PATH_OUT, PATH_CMP));
}

#[test]
#[timeout(20000)]
fn shuffle_channels() {
    const PATH_OUT: &str = &"out/shuffle_channels.png";
    const PATH_CMP: &str = &"data/test_compare/shuffle_channels.png";

    let tex_pro = TextureProcessor::new();

    let image_node = tex_pro
        .add_node(Node::new(NodeType::Image(IMAGE_2.into())))
        .unwrap();
    let output_node = tex_pro.add_node(Node::new(NodeType::OutputRgba)).unwrap();

    let output_slots = [SlotId(3), SlotId(1), SlotId(2), SlotId(0)];
    for i in 0..4 {
        tex_pro
            .connect(image_node, output_node, SlotId(i), output_slots[i as usize])
            .unwrap();
    }

    ensure_out_dir();
    let size = 256;
    image::save_buffer(
        &Path::new(PATH_OUT),
        &image::RgbaImage::from_vec(
            size,
            size,
            tex_pro.get_output_rgba(output_node, SlotId(0)).unwrap(),
        )
        .unwrap(),
        size,
        size,
        image::ColorType::RGBA(8),
    )
    .unwrap();

    assert!(images_equal(PATH_OUT, PATH_CMP));
}

#[test]
#[timeout(20000)]
fn resize_policy_most_pixels() {
    let tex_pro = TextureProcessor::new();

    let node_128 = tex_pro
        .add_node(Node::new(NodeType::Image(HEART_128.into())))
        .unwrap();
    let node_256 = tex_pro
        .add_node(Node::new(NodeType::Image(HEART_256.into())))
        .unwrap();
    let output = tex_pro
        .add_node(Node::new(NodeType::OutputRgba).resize_policy(ResizePolicy::MostPixels))
        .unwrap();

    tex_pro
        .connect(node_128, output, SlotId(0), SlotId(0))
        .unwrap();
    tex_pro
        .connect(node_256, output, SlotId(0), SlotId(1))
        .unwrap();

    assert!(
        tex_pro.node_slot_datas(output).unwrap()[0].size
            == tex_pro.node_slot_datas(output).unwrap()[1].size
    );
}

#[test]
#[timeout(20000)]
fn resize_policy_least_pixels() {
    let tex_pro = TextureProcessor::new();

    let node_128 = tex_pro
        .add_node(Node::new(NodeType::Image(HEART_128.into())))
        .unwrap();
    let node_256 = tex_pro
        .add_node(Node::new(NodeType::Image(HEART_256.into())))
        .unwrap();

    let mut passthrough_node = Node::new(NodeType::OutputRgba);
    passthrough_node.resize_policy = ResizePolicy::LeastPixels;
    let passthrough_node = tex_pro.add_node(passthrough_node).unwrap();
    let output_128 = tex_pro.add_node(Node::new(NodeType::OutputGray)).unwrap();
    let output_256 = tex_pro.add_node(Node::new(NodeType::OutputGray)).unwrap();

    tex_pro
        .connect(node_128, passthrough_node, SlotId(0), SlotId(0))
        .unwrap();
    tex_pro
        .connect(node_256, passthrough_node, SlotId(1), SlotId(1))
        .unwrap();

    tex_pro
        .connect(passthrough_node, output_128, SlotId(0), SlotId(0))
        .unwrap();
    tex_pro
        .connect(passthrough_node, output_256, SlotId(1), SlotId(0))
        .unwrap();

    assert!(
        tex_pro.await_slot_data_size(output_256, SlotId(0)).unwrap()
            == tex_pro.await_slot_data_size(node_128, SlotId(0)).unwrap()
    );
}

#[test]
#[timeout(20000)]
fn resize_policy_largest_axes() {
    let tex_pro = TextureProcessor::new();

    let node_256x128 = tex_pro
        .add_node(Node::new(NodeType::Image(HEART_WIDE.into())))
        .unwrap();
    let node_128x256 = tex_pro
        .add_node(Node::new(NodeType::Image(HEART_TALL.into())))
        .unwrap();
    let output = tex_pro
        .add_node(Node::new(NodeType::OutputRgba).resize_policy(ResizePolicy::LargestAxes))
        .unwrap();

    tex_pro
        .connect(node_256x128, output, SlotId(0), SlotId(0))
        .unwrap();
    tex_pro
        .connect(node_128x256, output, SlotId(1), SlotId(1))
        .unwrap();

    let target_size = Size::new(
        tex_pro.node_slot_datas(node_256x128).unwrap()[0].size.width,
        tex_pro.node_slot_datas(node_128x256).unwrap()[0]
            .size
            .height,
    );

    assert_eq!(
        tex_pro.node_slot_datas(output).unwrap()[0].size,
        target_size
    );
    assert_eq!(
        tex_pro.node_slot_datas(output).unwrap()[1].size,
        target_size
    );
}

#[test]
#[timeout(20000)]
fn add_node() {
    const PATH_OUT: &str = &"out/add_node.png";
    const PATH_CMP: &str = &"data/test_compare/add_node.png";

    let tex_pro = TextureProcessor::new();

    let image_node = tex_pro
        .add_node(Node::new(NodeType::Image(IMAGE_2.into())))
        .unwrap();
    let white_node = tex_pro.add_node(Node::new(NodeType::Value(1.))).unwrap();
    let add_node = tex_pro
        .add_node(Node::new(NodeType::Mix(MixType::Add)))
        .unwrap();
    let output_node = tex_pro.add_node(Node::new(NodeType::OutputRgba)).unwrap();

    tex_pro
        .connect(image_node, add_node, SlotId(0), SlotId(0))
        .unwrap();
    tex_pro
        .connect(image_node, add_node, SlotId(1), SlotId(1))
        .unwrap();

    tex_pro
        .connect(add_node, output_node, SlotId(0), SlotId(0))
        .unwrap();
    tex_pro
        .connect(add_node, output_node, SlotId(0), SlotId(1))
        .unwrap();
    tex_pro
        .connect(add_node, output_node, SlotId(0), SlotId(2))
        .unwrap();
    tex_pro
        .connect(white_node, output_node, SlotId(0), SlotId(3))
        .unwrap();

    ensure_out_dir();
    let size = 256;
    image::save_buffer(
        &Path::new(PATH_OUT),
        &image::RgbaImage::from_vec(
            size,
            size,
            tex_pro.get_output_rgba(output_node, SlotId(0)).unwrap(),
        )
        .unwrap(),
        size,
        size,
        image::ColorType::RGBA(8),
    )
    .unwrap();

    assert!(images_equal(PATH_OUT, PATH_CMP));
}

#[test]
#[timeout(20000)]
fn subtract_node() {
    const PATH_OUT: &str = &"out/subtract_node.png";
    const PATH_CMP: &str = &"data/test_compare/subtract_node.png";

    let tex_pro = TextureProcessor::new();

    let image_node = tex_pro
        .add_node(Node::new(NodeType::Image(IMAGE_2.into())))
        .unwrap();
    let subtract_node = tex_pro
        .add_node(Node::new(NodeType::Mix(MixType::Subtract)))
        .unwrap();
    let output_node = tex_pro.add_node(Node::new(NodeType::OutputGray)).unwrap();

    tex_pro
        .connect(image_node, subtract_node, SlotId(0), SlotId(0))
        .unwrap();
    tex_pro
        .connect(image_node, subtract_node, SlotId(1), SlotId(1))
        .unwrap();

    tex_pro
        .connect(subtract_node, output_node, SlotId(0), SlotId(0))
        .unwrap();

    ensure_out_dir();
    let size = 256;
    image::save_buffer(
        &Path::new(PATH_OUT),
        &image::RgbaImage::from_vec(
            size,
            size,
            tex_pro.get_output_rgba(output_node, SlotId(0)).unwrap(),
        )
        .unwrap(),
        size,
        size,
        image::ColorType::RGBA(8),
    )
    .unwrap();

    assert!(images_equal(PATH_OUT, PATH_CMP));
}

#[test]
#[timeout(20000)]
fn subtract_node_several() {
    const PATH_OUT: &str = &"out/subtract_node_several.png";
    const PATH_CMP: &str = &"data/test_compare/subtract_node_several.png";

    let tex_pro = TextureProcessor::new();

    let image_node = tex_pro
        .add_node(Node::new(NodeType::Image(IMAGE_2.into())))
        .unwrap();
    let subtract_node = tex_pro
        .add_node(Node::new(NodeType::Mix(MixType::Subtract)))
        .unwrap();
    let output_node = tex_pro.add_node(Node::new(NodeType::OutputGray)).unwrap();

    tex_pro
        .connect(image_node, subtract_node, SlotId(0), SlotId(0))
        .unwrap();
    tex_pro
        .connect(image_node, subtract_node, SlotId(1), SlotId(1))
        .unwrap();
    tex_pro
        .connect(image_node, subtract_node, SlotId(2), SlotId(2))
        .unwrap();

    tex_pro
        .connect(subtract_node, output_node, SlotId(0), SlotId(0))
        .unwrap();

    ensure_out_dir();
    let size = 256;
    image::save_buffer(
        &Path::new(PATH_OUT),
        &image::RgbaImage::from_vec(
            size,
            size,
            tex_pro.get_output_rgba(output_node, SlotId(0)).unwrap(),
        )
        .unwrap(),
        size,
        size,
        image::ColorType::RGBA(8),
    )
    .unwrap();

    assert!(images_equal(PATH_OUT, PATH_CMP));
}

#[test]
#[timeout(20000)]
fn invert_graph_node() {
    const PATH_OUT: &str = &"out/invert_graph_node.png";
    const PATH_CMP: &str = &"data/test_compare/invert_graph_node.png";
    // Nested invert graph
    let mut invert_graph = NodeGraph::new();

    let white_node_nested = invert_graph
        .add_node(Node::new(NodeType::Value(1.)))
        .unwrap();
    let nested_input_node = invert_graph.add_external_input_gray(SlotId(0)).unwrap();
    let subtract_node = invert_graph
        .add_node(Node::new(NodeType::Mix(MixType::Subtract)))
        .unwrap();
    let nested_output_node = invert_graph.add_external_output_gray(SlotId(0)).unwrap();

    invert_graph
        .connect(white_node_nested, subtract_node, SlotId(0), SlotId(0))
        .unwrap();
    invert_graph
        .connect(nested_input_node, subtract_node, SlotId(0), SlotId(1))
        .unwrap();

    invert_graph
        .connect(subtract_node, nested_output_node, SlotId(0), SlotId(0))
        .unwrap();

    // Main graph
    let tex_pro = TextureProcessor::new();

    let image_node = tex_pro
        .add_node(Node::new(NodeType::Image(HEART_256.into())))
        .unwrap();
    let white_node = tex_pro.add_node(Node::new(NodeType::Value(1.))).unwrap();
    let invert_graph_node = tex_pro
        .add_node(Node::new(NodeType::Graph(invert_graph)))
        .unwrap();
    let output_node = tex_pro.add_node(Node::new(NodeType::OutputRgba)).unwrap();

    tex_pro
        .connect(image_node, invert_graph_node, SlotId(0), SlotId(0))
        .unwrap();

    tex_pro
        .connect(invert_graph_node, output_node, SlotId(0), SlotId(0))
        .unwrap();
    tex_pro
        .connect(invert_graph_node, output_node, SlotId(0), SlotId(1))
        .unwrap();
    tex_pro
        .connect(invert_graph_node, output_node, SlotId(0), SlotId(2))
        .unwrap();
    tex_pro
        .connect(white_node, output_node, SlotId(0), SlotId(3))
        .unwrap();

    ensure_out_dir();
    let size = 256;
    image::save_buffer(
        &Path::new(PATH_OUT),
        &image::RgbaImage::from_vec(
            size,
            size,
            tex_pro.get_output_rgba(output_node, SlotId(0)).unwrap(),
        )
        .unwrap(),
        size,
        size,
        image::ColorType::RGBA(8),
    )
    .unwrap();

    assert!(images_equal(PATH_OUT, PATH_CMP));
}

#[test]
#[timeout(20000)]
fn invert_graph_node_export() {
    // Nested invert graph
    let mut invert_graph = NodeGraph::new();

    let white_node_nested = invert_graph
        .add_node(Node::new(NodeType::Value(1.)))
        .unwrap();
    let nested_input_node = invert_graph.add_external_input_gray(SlotId(0)).unwrap();
    let subtract_node = invert_graph
        .add_node(Node::new(NodeType::Mix(MixType::Subtract)))
        .unwrap();
    let nested_output_node = invert_graph.add_external_output_gray(SlotId(0)).unwrap();

    invert_graph
        .connect(white_node_nested, subtract_node, SlotId(0), SlotId(0))
        .unwrap();
    invert_graph
        .connect(nested_input_node, subtract_node, SlotId(0), SlotId(1))
        .unwrap();

    invert_graph
        .connect(subtract_node, nested_output_node, SlotId(0), SlotId(0))
        .unwrap();

    invert_graph
        .export_json("out/invert_graph.json".into())
        .unwrap();
}

#[test]
#[timeout(20000)]
fn invert_graph_node_import() {
    const PATH_OUT: &str = &"out/invert_graph_node_import.png";
    const PATH_CMP: &str = &"data/test_compare/invert_graph_node_import.png";

    // Nested invert graph
    let invert_graph = NodeGraph::from_path("data/invert_graph.json".into()).unwrap();

    // Main graph
    let tex_pro = TextureProcessor::new();

    let image_node = tex_pro
        .add_node(Node::new(NodeType::Image("data/heart_256.png".into())))
        .unwrap();
    let white_node = tex_pro.add_node(Node::new(NodeType::Value(1.))).unwrap();
    let invert_graph_node = tex_pro
        .add_node(Node::new(NodeType::Graph(invert_graph)))
        .unwrap();
    let output_node = tex_pro.add_node(Node::new(NodeType::OutputRgba)).unwrap();

    tex_pro
        .connect(image_node, invert_graph_node, SlotId(0), SlotId(0))
        .unwrap();

    tex_pro
        .connect(invert_graph_node, output_node, SlotId(0), SlotId(0))
        .unwrap();
    tex_pro
        .connect(invert_graph_node, output_node, SlotId(0), SlotId(1))
        .unwrap();
    tex_pro
        .connect(invert_graph_node, output_node, SlotId(0), SlotId(2))
        .unwrap();
    tex_pro
        .connect(white_node, output_node, SlotId(0), SlotId(3))
        .unwrap();

    ensure_out_dir();
    let size = 256;
    image::save_buffer(
        &Path::new(PATH_OUT),
        &image::RgbaImage::from_vec(
            size,
            size,
            tex_pro.get_output_rgba(output_node, SlotId(0)).unwrap(),
        )
        .unwrap(),
        size,
        size,
        image::ColorType::RGBA(8),
    )
    .unwrap();

    assert!(images_equal(PATH_OUT, PATH_CMP));
}

#[test]
#[timeout(20000)]
fn graph_node_rgba() {
    const PATH_OUT: &str = &"out/graph_node_rgba.png";
    const PATH_CMP: &str = &"data/test_compare/graph_node_rgba.png";

    // Nested graph
    let mut nested_graph = NodeGraph::new();

    let nested_input_node = nested_graph
        .add_external_input_rgba(vec![SlotId(0), SlotId(1), SlotId(2), SlotId(3)])
        .unwrap();
    let nested_output_node = nested_graph
        .add_external_output_rgba(vec![SlotId(0), SlotId(1), SlotId(2), SlotId(3)])
        .unwrap();

    nested_graph
        .connect(nested_input_node, nested_output_node, SlotId(0), SlotId(0))
        .unwrap();
    nested_graph
        .connect(nested_input_node, nested_output_node, SlotId(1), SlotId(1))
        .unwrap();
    nested_graph
        .connect(nested_input_node, nested_output_node, SlotId(2), SlotId(2))
        .unwrap();
    nested_graph
        .connect(nested_input_node, nested_output_node, SlotId(3), SlotId(3))
        .unwrap();

    // Texture Processor
    let tex_pro = TextureProcessor::new();

    let input_node = tex_pro
        .add_node(Node::new(NodeType::Image(IMAGE_2.into())))
        .unwrap();
    let graph_node = tex_pro
        .add_node(Node::new(NodeType::Graph(nested_graph)))
        .unwrap();
    let output_node = tex_pro.add_node(Node::new(NodeType::OutputRgba)).unwrap();

    tex_pro
        .connect(input_node, graph_node, SlotId(0), SlotId(0))
        .unwrap();
    tex_pro
        .connect(input_node, graph_node, SlotId(1), SlotId(1))
        .unwrap();
    tex_pro
        .connect(input_node, graph_node, SlotId(2), SlotId(2))
        .unwrap();
    tex_pro
        .connect(input_node, graph_node, SlotId(3), SlotId(3))
        .unwrap();

    tex_pro
        .connect(graph_node, output_node, SlotId(0), SlotId(0))
        .unwrap();
    tex_pro
        .connect(graph_node, output_node, SlotId(1), SlotId(1))
        .unwrap();
    tex_pro
        .connect(graph_node, output_node, SlotId(2), SlotId(2))
        .unwrap();
    tex_pro
        .connect(graph_node, output_node, SlotId(3), SlotId(3))
        .unwrap();

    ensure_out_dir();
    // Output
    image::save_buffer(
        &Path::new(PATH_OUT),
        &image::RgbaImage::from_vec(
            256,
            256,
            tex_pro.get_output_rgba(output_node, SlotId(0)).unwrap(),
        )
        .unwrap(),
        256,
        256,
        image::ColorType::RGBA(8),
    )
    .unwrap();

    assert!(images_equal(PATH_OUT, PATH_CMP));
}

/// Grayscale passthrough node.
#[test]
#[timeout(20000)]
fn graph_node_gray() {
    const PATH_OUT: &str = &"out/graph_node_gray.png";
    const PATH_CMP: &str = &"data/test_compare/graph_node_gray.png";

    // Nested graph
    let mut nested_graph = NodeGraph::new();

    let nested_input_node = nested_graph.add_external_input_gray(SlotId(0)).unwrap();
    let nested_output_node = nested_graph
        .add_node(Node::new(NodeType::OutputGray))
        .unwrap();

    nested_graph
        .connect(nested_input_node, nested_output_node, SlotId(0), SlotId(0))
        .unwrap();

    // Texture Processor
    let tex_pro = TextureProcessor::new();

    let input_node = tex_pro
        .add_node(Node::new(NodeType::Image(IMAGE_2.into())))
        .unwrap();
    let graph_node = tex_pro
        .add_node(Node::new(NodeType::Graph(nested_graph)))
        .unwrap();
    let output_node = tex_pro.add_node(Node::new(NodeType::OutputRgba)).unwrap();

    tex_pro
        .connect(input_node, graph_node, SlotId(0), SlotId(0))
        .unwrap();

    tex_pro
        .connect(graph_node, output_node, SlotId(0), SlotId(0))
        .unwrap();
    tex_pro
        .connect(graph_node, output_node, SlotId(0), SlotId(1))
        .unwrap();
    tex_pro
        .connect(graph_node, output_node, SlotId(0), SlotId(2))
        .unwrap();
    tex_pro
        .connect(graph_node, output_node, SlotId(0), SlotId(3))
        .unwrap();

    ensure_out_dir();
    // Output
    image::save_buffer(
        &Path::new(PATH_OUT),
        &image::RgbaImage::from_vec(
            256,
            256,
            tex_pro.get_output_rgba(output_node, SlotId(0)).unwrap(),
        )
        .unwrap(),
        256,
        256,
        image::ColorType::RGBA(8),
    )
    .unwrap();

    assert!(images_equal(PATH_OUT, PATH_CMP));
}

#[test]
#[timeout(20000)]
fn height_to_normal_node() {
    const PATH_OUT: &str = &"out/height_to_normal_node.png";
    const PATH_CMP: &str = &"data/test_compare/height_to_normal_node.png";

    // Texture Processor
    let tex_pro = TextureProcessor::new();

    let input_node = tex_pro
        .add_node(Node::new(NodeType::Image(CLOUDS.into())))
        .unwrap();
    let h2n_node = tex_pro
        .add_node(Node::new(NodeType::HeightToNormal))
        .unwrap();
    let output_node = tex_pro.add_node(Node::new(NodeType::OutputRgba)).unwrap();

    tex_pro
        .connect(input_node, h2n_node, SlotId(0), SlotId(0))
        .unwrap();

    tex_pro
        .connect(h2n_node, output_node, SlotId(0), SlotId(0))
        .unwrap();
    tex_pro
        .connect(h2n_node, output_node, SlotId(1), SlotId(1))
        .unwrap();
    tex_pro
        .connect(h2n_node, output_node, SlotId(2), SlotId(2))
        .unwrap();

    ensure_out_dir();
    // Output
    image::save_buffer(
        &Path::new(PATH_OUT),
        &image::RgbaImage::from_vec(
            256,
            256,
            tex_pro.get_output_rgba(output_node, SlotId(0)).unwrap(),
        )
        .unwrap(),
        256,
        256,
        image::ColorType::RGBA(8),
    )
    .unwrap();

    assert!(images_equal(PATH_OUT, PATH_CMP));
}

#[test]
#[timeout(20000)]
fn multiply_node() {
    const PATH_OUT: &str = &"out/multiply_node.png";
    const PATH_CMP: &str = &"data/test_compare/multiply_node.png";

    let tex_pro = TextureProcessor::new();

    let image_node = tex_pro
        .add_node(Node::new(NodeType::Image(IMAGE_1.into())))
        .unwrap();
    let multiply_node = tex_pro
        .add_node(Node::new(NodeType::Mix(MixType::Multiply)))
        .unwrap();
    let output_node = tex_pro.add_node(Node::new(NodeType::OutputGray)).unwrap();

    tex_pro
        .connect(image_node, multiply_node, SlotId(0), SlotId(0))
        .unwrap();
    tex_pro
        .connect(image_node, multiply_node, SlotId(3), SlotId(1))
        .unwrap();

    tex_pro
        .connect(multiply_node, output_node, SlotId(0), SlotId(0))
        .unwrap();

    ensure_out_dir();
    let size = 256;
    image::save_buffer(
        &Path::new(PATH_OUT),
        &image::RgbaImage::from_vec(
            size,
            size,
            tex_pro.get_output_rgba(output_node, SlotId(0)).unwrap(),
        )
        .unwrap(),
        size,
        size,
        image::ColorType::RGBA(8),
    )
    .unwrap();

    assert!(images_equal(PATH_OUT, PATH_CMP));
}

#[test]
#[timeout(20000)]
fn divide_node() {
    const PATH_OUT: &str = &"out/divide_node.png";
    const PATH_CMP: &str = &"data/test_compare/divide_node.png";

    let tex_pro = TextureProcessor::new();

    let image_node = tex_pro
        .add_node(Node::new(NodeType::Image(IMAGE_1.into())))
        .unwrap();
    let divide_node = tex_pro
        .add_node(Node::new(NodeType::Mix(MixType::Divide)))
        .unwrap();
    let output_node = tex_pro.add_node(Node::new(NodeType::OutputGray)).unwrap();

    tex_pro
        .connect(image_node, divide_node, SlotId(0), SlotId(0))
        .unwrap();
    tex_pro
        .connect(image_node, divide_node, SlotId(3), SlotId(1))
        .unwrap();

    tex_pro
        .connect(divide_node, output_node, SlotId(0), SlotId(0))
        .unwrap();

    ensure_out_dir();
    let size = 256;
    image::save_buffer(
        &Path::new(PATH_OUT),
        &image::RgbaImage::from_vec(
            size,
            size,
            tex_pro.get_output_rgba(output_node, SlotId(0)).unwrap(),
        )
        .unwrap(),
        size,
        size,
        image::ColorType::RGBA(8),
    )
    .unwrap();

    assert!(images_equal(PATH_OUT, PATH_CMP));
}
