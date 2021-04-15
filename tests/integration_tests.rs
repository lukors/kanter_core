use kanter_core::{
    dag::TextureProcessor,
    node::{MixType, Node, NodeType, ResizePolicy},
    node_data::Size,
    node_graph::{NodeGraph, NodeId, SlotId},
};
use std::{fs::create_dir, path::Path};

const OUT_DIR: &str = "out";

fn ensure_out_dir() {
    match create_dir(Path::new(OUT_DIR)) {
        Ok(_) => (),
        Err(AlreadyExists) => (),
        Err(e) => panic!("Error when creating test dir: {:?}", e),
    };
}

#[test]
fn input_output() {
    let mut tex_pro = TextureProcessor::new();

    let input_node = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::Image("data/image_2.png".to_string())))
        .unwrap();
    let output_node = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::OutputRgba))
        .unwrap();

    tex_pro
        .node_graph
        .connect(input_node, output_node, SlotId(0), SlotId(0))
        .unwrap();
    tex_pro
        .node_graph
        .connect(input_node, output_node, SlotId(1), SlotId(1))
        .unwrap();
    tex_pro
        .node_graph
        .connect(input_node, output_node, SlotId(2), SlotId(2))
        .unwrap();
    tex_pro
        .node_graph
        .connect(input_node, output_node, SlotId(3), SlotId(3))
        .unwrap();

    tex_pro.process();

    ensure_out_dir();
    image::save_buffer(
        &Path::new(&"out/input_output.png"),
        &image::RgbaImage::from_vec(256, 256, tex_pro.get_output(output_node).unwrap()).unwrap(),
        256,
        256,
        image::ColorType::RGBA(8),
    )
    .unwrap();
}

#[test]
fn unconnected() {
    let mut tex_pro = TextureProcessor::new();

    tex_pro
        .node_graph
        .add_node(Node::new(NodeType::OutputRgba))
        .unwrap();
    
    tex_pro.process();
}

#[test]
fn repeat() {
    let mut tex_pro = TextureProcessor::new();

    let input_node = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::Image("data/image_1.png".to_string())))
        .unwrap();
    let output_node = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::OutputRgba))
        .unwrap();

    tex_pro
        .node_graph
        .connect(input_node, output_node, SlotId(0), SlotId(0))
        .unwrap();
    
    tex_pro.process();
    tex_pro.process();
    tex_pro.process();
    tex_pro.process();
    tex_pro.process();
}

#[test]
fn mix_images() {
    let mut tex_pro = TextureProcessor::new();

    let input_1 = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::Image("data/image_1.png".to_string())))
        .unwrap();
    let input_2 = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::Image("data/image_2.png".to_string())))
        .unwrap();
    let output_node = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::OutputRgba))
        .unwrap();

    tex_pro
        .node_graph
        .connect(input_1, output_node, SlotId(3), SlotId(0))
        .unwrap();
    tex_pro
        .node_graph
        .connect(input_1, output_node, SlotId(1), SlotId(1))
        .unwrap();
    tex_pro
        .node_graph
        .connect(input_2, output_node, SlotId(2), SlotId(2))
        .unwrap();
    tex_pro
        .node_graph
        .connect(input_2, output_node, SlotId(3), SlotId(3))
        .unwrap();

    tex_pro.process();

    ensure_out_dir();
    image::save_buffer(
        &Path::new(&"out/mix_images.png"),
        &image::RgbaImage::from_vec(256, 256, tex_pro.get_output(output_node).unwrap()).unwrap(),
        256,
        256,
        image::ColorType::RGBA(8),
    )
    .unwrap();
}

#[test]
fn resize_rgba() {
    const SIZE: u32 = 256;
    const IN_PATH: &str = &"data/image_2.png";
    let mut tex_pro = TextureProcessor::new();
    
    let n_in = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::Image(IN_PATH.to_string())))
        .unwrap();
    
    let n_resize_1 = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::Resize(
            Some(ResizePolicy::SpecificSize(Size::new(
                SIZE,
                SIZE,
            ))),
            None,
        )))
        .unwrap();
    let n_resize_2 = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::Resize(
            Some(ResizePolicy::SpecificSize(Size::new(
                SIZE,
                SIZE,
            ))),
            None,
        )))
        .unwrap();
    let n_resize_3 = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::Resize(
            Some(ResizePolicy::SpecificSize(Size::new(
                SIZE,
                SIZE,
            ))),
            None,
        )))
        .unwrap();
    let n_resize_4 = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::Resize(
            Some(ResizePolicy::SpecificSize(Size::new(
                SIZE,
                SIZE,
            ))),
            None,
        )))
        .unwrap();
    let n_out = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::OutputRgba))
        .unwrap();

    tex_pro
        .node_graph
        .connect(n_in, n_resize_1, SlotId(0), SlotId(0))
        .unwrap();
    tex_pro
        .node_graph
        .connect(n_in, n_resize_2, SlotId(1), SlotId(0))
        .unwrap();
    tex_pro
        .node_graph
        .connect(n_in, n_resize_3, SlotId(2), SlotId(0))
        .unwrap();
    tex_pro
        .node_graph
        .connect(n_in, n_resize_4, SlotId(3), SlotId(0))
        .unwrap();

    tex_pro
        .node_graph
        .connect(n_resize_1, n_out, SlotId(0), SlotId(0))
        .unwrap();
    tex_pro
        .node_graph
        .connect(n_resize_2, n_out, SlotId(0), SlotId(1))
        .unwrap();
    tex_pro
        .node_graph
        .connect(n_resize_3, n_out, SlotId(0), SlotId(2))
        .unwrap();
    tex_pro
        .node_graph
        .connect(n_resize_4, n_out, SlotId(0), SlotId(3))
        .unwrap();

    tex_pro.process();

    const OUT_PATH: &str = &"out/resize_rgba.png";
    ensure_out_dir();
    image::save_buffer(
        &Path::new(OUT_PATH),
        &image::RgbaImage::from_vec(
            SIZE,
            SIZE,
            tex_pro.get_output(n_out).unwrap(),
        )
        .unwrap(),
        SIZE,
        SIZE,
        image::ColorType::RGBA(8),
    )
    .unwrap();

    assert!(images_equal(OUT_PATH, IN_PATH));
}

fn images_equal<P: AsRef<Path>>(path_1: P, path_2: P) -> bool {
    let image_1 = image::open(path_1).unwrap();
    let raw_pixels_1 = image_1.raw_pixels();
    let image_2 = image::open(path_2).unwrap();
    let raw_pixels_2 = image_2.raw_pixels();

    raw_pixels_1.iter().eq(raw_pixels_2.iter())
}

#[test]
fn input_output_2() {
    let tex_pro_compare = input_output_2_internal();

    for _ in 0..30 {
        let tex_pro = input_output_2_internal();

        for node_data_cmp in &tex_pro_compare.node_datas {
            assert!(tex_pro
                .node_datas
                .iter()
                .any(|node_data| *node_data == *node_data_cmp));
        }
    }
}

fn input_output_2_internal() -> TextureProcessor {
    let mut tex_pro = TextureProcessor::new();

    let input_node_1 = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::Image("data/px_1.png".to_string())))
        .unwrap();
    let input_node_2 = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::Image("data/px_1.png".to_string())))
        .unwrap();
    let output_node = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::OutputRgba))
        .unwrap();

    tex_pro
        .node_graph
        .connect(input_node_2, output_node, SlotId(2), SlotId(2))
        .unwrap();
    tex_pro
        .node_graph
        .connect(input_node_1, output_node, SlotId(0), SlotId(0))
        .unwrap();
    tex_pro
        .node_graph
        .connect(input_node_1, output_node, SlotId(1), SlotId(1))
        .unwrap();
    tex_pro
        .node_graph
        .connect(input_node_2, output_node, SlotId(3), SlotId(3))
        .unwrap();

    tex_pro.process();

    tex_pro
}

#[test]
fn value_node() {
    let mut tex_pro = TextureProcessor::new();

    let red_node = tex_pro
        .node_graph
        .add_node_with_id(Node::new(NodeType::Value(0.)), NodeId(0))
        .unwrap();
    let green_node = tex_pro
        .node_graph
        .add_node_with_id(Node::new(NodeType::Value(0.33)), NodeId(1))
        .unwrap();
    let blue_node = tex_pro
        .node_graph
        .add_node_with_id(Node::new(NodeType::Value(0.66)), NodeId(2))
        .unwrap();
    let alpha_node = tex_pro
        .node_graph
        .add_node_with_id(Node::new(NodeType::Value(1.)), NodeId(3))
        .unwrap();

    let output_node = tex_pro
        .node_graph
        .add_node_with_id(Node::new(NodeType::OutputRgba), NodeId(5))
        .unwrap();

    tex_pro
        .node_graph
        .connect(red_node, output_node, SlotId(0), SlotId(0))
        .unwrap();
    tex_pro
        .node_graph
        .connect(green_node, output_node, SlotId(0), SlotId(1))
        .unwrap();
    tex_pro
        .node_graph
        .connect(blue_node, output_node, SlotId(0), SlotId(2))
        .unwrap();
    tex_pro
        .node_graph
        .connect(alpha_node, output_node, SlotId(0), SlotId(3))
        .unwrap();

    tex_pro.process();

    ensure_out_dir();
    image::save_buffer(
        &Path::new(&"out/value_node.png"),
        &image::RgbaImage::from_vec(1, 1, tex_pro.get_output(output_node).unwrap()).unwrap(),
        1,
        1,
        image::ColorType::RGBA(8),
    )
    .unwrap();
}

#[test]
fn resize_node() {
    let size = Size::new(256, 256);

    let mut tex_pro = TextureProcessor::new();

    let value_node = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::Value(0.5)))
        .unwrap();
    let resize_node = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::Resize(
            Some(ResizePolicy::SpecificSize(size)),
            None,
        )))
        .unwrap();
    let output_node = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::OutputRgba))
        .unwrap();

    tex_pro
        .node_graph
        .connect(value_node, resize_node, SlotId(0), SlotId(0))
        .unwrap();

    tex_pro
        .node_graph
        .connect(resize_node, output_node, SlotId(0), SlotId(0))
        .unwrap();
    tex_pro
        .node_graph
        .connect(resize_node, output_node, SlotId(0), SlotId(1))
        .unwrap();
    tex_pro
        .node_graph
        .connect(resize_node, output_node, SlotId(0), SlotId(2))
        .unwrap();
    tex_pro
        .node_graph
        .connect(resize_node, output_node, SlotId(0), SlotId(3))
        .unwrap();

    tex_pro.process();

    ensure_out_dir();
    image::save_buffer(
        &Path::new(&"out/resize_node.png"),
        &image::RgbaImage::from_vec(
            size.width,
            size.height,
            tex_pro.get_output(output_node).unwrap(),
        )
        .unwrap(),
        size.width,
        size.height,
        image::ColorType::RGBA(8),
    )
    .unwrap();
}

#[test]
fn shuffle_channels() {
    let mut tex_pro = TextureProcessor::new();

    let image_node = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::Image("data/image_2.png".to_string())))
        .unwrap();
    let output_node = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::OutputRgba))
        .unwrap();

    tex_pro
        .node_graph
        .connect(image_node, output_node, SlotId(0), SlotId(3))
        .unwrap();
    tex_pro
        .node_graph
        .connect(image_node, output_node, SlotId(1), SlotId(1))
        .unwrap();
    tex_pro
        .node_graph
        .connect(image_node, output_node, SlotId(2), SlotId(2))
        .unwrap();
    tex_pro
        .node_graph
        .connect(image_node, output_node, SlotId(3), SlotId(0))
        .unwrap();

    tex_pro.process();

    ensure_out_dir();
    let size = 256;
    image::save_buffer(
        &Path::new(&"out/shuffle_channels.png"),
        &image::RgbaImage::from_vec(size, size, tex_pro.get_output(output_node).unwrap()).unwrap(),
        size,
        size,
        image::ColorType::RGBA(8),
    )
    .unwrap();
}

#[test]
fn resize_policy_most_pixels() {
    let mut tex_pro = TextureProcessor::new();

    let node_128 = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::Image("data/heart_128.png".to_string())))
        .unwrap();
    let node_256 = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::Image("data/heart_256.png".to_string())))
        .unwrap();
    let resize_node = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::Resize(
            Some(ResizePolicy::MostPixels),
            None,
        )))
        .unwrap();
    let output_128 = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::OutputGray))
        .unwrap();
    let output_256 = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::OutputGray))
        .unwrap();

    tex_pro
        .node_graph
        .connect(node_128, resize_node, SlotId(0), SlotId(0))
        .unwrap();
    tex_pro
        .node_graph
        .connect(node_256, resize_node, SlotId(1), SlotId(1))
        .unwrap();

    tex_pro
        .node_graph
        .connect(resize_node, output_128, SlotId(0), SlotId(0))
        .unwrap();
    tex_pro
        .node_graph
        .connect(resize_node, output_256, SlotId(1), SlotId(0))
        .unwrap();

    tex_pro.process();

    assert!(tex_pro.node_datas(output_128)[0].size == tex_pro.node_datas(node_256)[0].size);
}

#[test]
fn resize_policy_least_pixels() {
    let mut tex_pro = TextureProcessor::new();

    let node_128 = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::Image("data/heart_128.png".to_string())))
        .unwrap();
    let node_256 = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::Image("data/heart_256.png".to_string())))
        .unwrap();
    let resize_node = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::Resize(
            Some(ResizePolicy::LeastPixels),
            None,
        )))
        .unwrap();
    let output_128 = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::OutputGray))
        .unwrap();
    let output_256 = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::OutputGray))
        .unwrap();

    tex_pro
        .node_graph
        .connect(node_128, resize_node, SlotId(0), SlotId(0))
        .unwrap();
    tex_pro
        .node_graph
        .connect(node_256, resize_node, SlotId(1), SlotId(1))
        .unwrap();

    tex_pro
        .node_graph
        .connect(resize_node, output_128, SlotId(0), SlotId(0))
        .unwrap();
    tex_pro
        .node_graph
        .connect(resize_node, output_256, SlotId(1), SlotId(0))
        .unwrap();

    tex_pro.process();

    assert!(tex_pro.node_datas(output_256)[0].size == tex_pro.node_datas(node_128)[0].size);
}

#[test]
fn resize_policy_largest_axes() {
    let mut tex_pro = TextureProcessor::new();

    let node_256x128 = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::Image("data/heart_wide.png".to_string())))
        .unwrap();
    let node_128x256 = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::Image("data/heart_tall.png".to_string())))
        .unwrap();
    let resize_node = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::Resize(
            Some(ResizePolicy::LargestAxes),
            None,
        )))
        .unwrap();
    let output_256x128 = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::OutputGray))
        .unwrap();
    let output_128x256 = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::OutputGray))
        .unwrap();

    tex_pro
        .node_graph
        .connect(node_256x128, resize_node, SlotId(0), SlotId(0))
        .unwrap();
    tex_pro
        .node_graph
        .connect(node_128x256, resize_node, SlotId(1), SlotId(1))
        .unwrap();

    tex_pro
        .node_graph
        .connect(resize_node, output_256x128, SlotId(0), SlotId(0))
        .unwrap();
    tex_pro
        .node_graph
        .connect(resize_node, output_128x256, SlotId(1), SlotId(0))
        .unwrap();

    tex_pro.process();

    let target_size = Size::new(
        tex_pro.node_datas(node_256x128)[0].size.width,
        tex_pro.node_datas(node_128x256)[0].size.height,
    );

    assert!(tex_pro.node_datas(output_128x256)[0].size == target_size);
    assert!(tex_pro.node_datas(output_256x128)[0].size == target_size);
}

#[test]
fn add_node() {
    let mut tex_pro = TextureProcessor::new();

    let image_node = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::Image("data/image_2.png".to_string())))
        .unwrap();
    let white_node = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::Value(1.)))
        .unwrap();
    let add_node = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::Mix(MixType::Add)))
        .unwrap();
    let output_node = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::OutputRgba))
        .unwrap();

    tex_pro
        .node_graph
        .connect(image_node, add_node, SlotId(0), SlotId(0))
        .unwrap();
    tex_pro
        .node_graph
        .connect(image_node, add_node, SlotId(1), SlotId(1))
        .unwrap();

    tex_pro
        .node_graph
        .connect(add_node, output_node, SlotId(0), SlotId(0))
        .unwrap();
    tex_pro
        .node_graph
        .connect(add_node, output_node, SlotId(0), SlotId(1))
        .unwrap();
    tex_pro
        .node_graph
        .connect(add_node, output_node, SlotId(0), SlotId(2))
        .unwrap();
    tex_pro
        .node_graph
        .connect(white_node, output_node, SlotId(0), SlotId(3))
        .unwrap();

    tex_pro.process();

    ensure_out_dir();
    let size = 256;
    image::save_buffer(
        &Path::new(&"out/add_node.png"),
        &image::RgbaImage::from_vec(size, size, tex_pro.get_output(output_node).unwrap()).unwrap(),
        size,
        size,
        image::ColorType::RGBA(8),
    )
    .unwrap();
}

#[test]
fn subtract_node() {
    let mut tex_pro = TextureProcessor::new();

    let image_node = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::Image("data/image_2.png".to_string())))
        .unwrap();
    let subtract_node = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::Mix(MixType::Subtract)))
        .unwrap();
    let output_node = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::OutputGray))
        .unwrap();

    tex_pro
        .node_graph
        .connect(image_node, subtract_node, SlotId(0), SlotId(0))
        .unwrap();
    tex_pro
        .node_graph
        .connect(image_node, subtract_node, SlotId(1), SlotId(1))
        .unwrap();

    tex_pro
        .node_graph
        .connect(subtract_node, output_node, SlotId(0), SlotId(0))
        .unwrap();

    tex_pro.process();

    ensure_out_dir();
    let size = 256;
    image::save_buffer(
        &Path::new(&"out/subtract_node.png"),
        &image::RgbaImage::from_vec(size, size, tex_pro.get_output(output_node).unwrap()).unwrap(),
        size,
        size,
        image::ColorType::RGBA(8),
    )
    .unwrap();
}

#[test]
fn invert_graph_node() {
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
    let mut tex_pro = TextureProcessor::new();

    let image_node = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::Image("data/heart_256.png".to_string())))
        .unwrap();
    let white_node = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::Value(1.)))
        .unwrap();
    let invert_graph_node = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::Graph(invert_graph)))
        .unwrap();
    let output_node = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::OutputRgba))
        .unwrap();

    tex_pro
        .node_graph
        .connect(image_node, invert_graph_node, SlotId(0), SlotId(0))
        .unwrap();

    tex_pro
        .node_graph
        .connect(invert_graph_node, output_node, SlotId(0), SlotId(0))
        .unwrap();
    tex_pro
        .node_graph
        .connect(invert_graph_node, output_node, SlotId(0), SlotId(1))
        .unwrap();
    tex_pro
        .node_graph
        .connect(invert_graph_node, output_node, SlotId(0), SlotId(2))
        .unwrap();
    tex_pro
        .node_graph
        .connect(white_node, output_node, SlotId(0), SlotId(3))
        .unwrap();

    tex_pro.process();

    ensure_out_dir();
    let size = 256;
    image::save_buffer(
        &Path::new(&"out/invert_graph_node.png"),
        &image::RgbaImage::from_vec(size, size, tex_pro.get_output(output_node).unwrap()).unwrap(),
        size,
        size,
        image::ColorType::RGBA(8),
    )
    .unwrap();
}

#[test]
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
        .export_json("out/invert_graph.json".to_string())
        .unwrap();
}

#[test]
fn invert_graph_node_import() {
    // Nested invert graph
    let invert_graph = NodeGraph::from_path("data/invert_graph.json".to_string()).unwrap();

    // Main graph
    let mut tex_pro = TextureProcessor::new();

    let image_node = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::Image("data/heart_256.png".to_string())))
        .unwrap();
    let white_node = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::Value(1.)))
        .unwrap();
    let invert_graph_node = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::Graph(invert_graph)))
        .unwrap();
    let output_node = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::OutputRgba))
        .unwrap();

    tex_pro
        .node_graph
        .connect(image_node, invert_graph_node, SlotId(0), SlotId(0))
        .unwrap();

    tex_pro
        .node_graph
        .connect(invert_graph_node, output_node, SlotId(0), SlotId(0))
        .unwrap();
    tex_pro
        .node_graph
        .connect(invert_graph_node, output_node, SlotId(0), SlotId(1))
        .unwrap();
    tex_pro
        .node_graph
        .connect(invert_graph_node, output_node, SlotId(0), SlotId(2))
        .unwrap();
    tex_pro
        .node_graph
        .connect(white_node, output_node, SlotId(0), SlotId(3))
        .unwrap();

    tex_pro.process();

    ensure_out_dir();
    let size = 256;
    image::save_buffer(
        &Path::new(&"out/invert_graph_node_import.png"),
        &image::RgbaImage::from_vec(size, size, tex_pro.get_output(output_node).unwrap()).unwrap(),
        size,
        size,
        image::ColorType::RGBA(8),
    )
    .unwrap();
}

#[test]
fn graph_node_rgba() {
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
    let mut tex_pro = TextureProcessor::new();

    let input_node = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::Image("data/image_2.png".to_string())))
        .unwrap();
    let graph_node = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::Graph(nested_graph)))
        .unwrap();
    let output_node = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::OutputRgba))
        .unwrap();

    tex_pro
        .node_graph
        .connect(input_node, graph_node, SlotId(0), SlotId(0))
        .unwrap();
    tex_pro
        .node_graph
        .connect(input_node, graph_node, SlotId(1), SlotId(1))
        .unwrap();
    tex_pro
        .node_graph
        .connect(input_node, graph_node, SlotId(2), SlotId(2))
        .unwrap();
    tex_pro
        .node_graph
        .connect(input_node, graph_node, SlotId(3), SlotId(3))
        .unwrap();

    tex_pro
        .node_graph
        .connect(graph_node, output_node, SlotId(0), SlotId(0))
        .unwrap();
    tex_pro
        .node_graph
        .connect(graph_node, output_node, SlotId(1), SlotId(1))
        .unwrap();
    tex_pro
        .node_graph
        .connect(graph_node, output_node, SlotId(2), SlotId(2))
        .unwrap();
    tex_pro
        .node_graph
        .connect(graph_node, output_node, SlotId(3), SlotId(3))
        .unwrap();

    tex_pro.process();

    ensure_out_dir();
    // Output
    image::save_buffer(
        &Path::new(&"out/graph_node_rgba.png"),
        &image::RgbaImage::from_vec(256, 256, tex_pro.get_output(output_node).unwrap()).unwrap(),
        256,
        256,
        image::ColorType::RGBA(8),
    )
    .unwrap();
}

#[test]
fn graph_node_gray() {
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
    let mut tex_pro = TextureProcessor::new();

    let input_node = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::Image("data/image_2.png".to_string())))
        .unwrap();
    let graph_node = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::Graph(nested_graph)))
        .unwrap();
    let output_node = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::OutputRgba))
        .unwrap();

    tex_pro
        .node_graph
        .connect(input_node, graph_node, SlotId(0), SlotId(0))
        .unwrap();

    tex_pro
        .node_graph
        .connect(graph_node, output_node, SlotId(0), SlotId(0))
        .unwrap();
    tex_pro
        .node_graph
        .connect(graph_node, output_node, SlotId(0), SlotId(1))
        .unwrap();
    tex_pro
        .node_graph
        .connect(graph_node, output_node, SlotId(0), SlotId(2))
        .unwrap();
    tex_pro
        .node_graph
        .connect(graph_node, output_node, SlotId(0), SlotId(3))
        .unwrap();

    tex_pro.process();

    ensure_out_dir();
    // Output
    image::save_buffer(
        &Path::new(&"out/graph_node_gray.png"),
        &image::RgbaImage::from_vec(256, 256, tex_pro.get_output(output_node).unwrap()).unwrap(),
        256,
        256,
        image::ColorType::RGBA(8),
    )
    .unwrap();
}

#[test]
fn height_to_normal_node() {
    // Texture Processor
    let mut tex_pro = TextureProcessor::new();

    let input_node = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::Image("data/clouds.png".to_string())))
        .unwrap();
    let h2n_node = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::HeightToNormal))
        .unwrap();
    let output_node = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::OutputRgba))
        .unwrap();

    tex_pro
        .node_graph
        .connect(input_node, h2n_node, SlotId(0), SlotId(0))
        .unwrap();

    tex_pro
        .node_graph
        .connect(h2n_node, output_node, SlotId(0), SlotId(0))
        .unwrap();
    tex_pro
        .node_graph
        .connect(h2n_node, output_node, SlotId(1), SlotId(1))
        .unwrap();
    tex_pro
        .node_graph
        .connect(h2n_node, output_node, SlotId(2), SlotId(2))
        .unwrap();

    tex_pro.process();

    ensure_out_dir();
    // Output
    image::save_buffer(
        &Path::new(&"out/height_to_normal_node.png"),
        &image::RgbaImage::from_vec(256, 256, tex_pro.get_output(output_node).unwrap()).unwrap(),
        256,
        256,
        image::ColorType::RGBA(8),
    )
    .unwrap();
}

#[test]
fn multiply_node() {
    let mut tex_pro = TextureProcessor::new();

    let image_node = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::Image("data/image_1.png".to_string())))
        .unwrap();
    let multiply_node = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::Mix(MixType::Multiply)))
        .unwrap();
    let output_node = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::OutputGray))
        .unwrap();

    tex_pro
        .node_graph
        .connect(image_node, multiply_node, SlotId(0), SlotId(0))
        .unwrap();
    tex_pro
        .node_graph
        .connect(image_node, multiply_node, SlotId(3), SlotId(1))
        .unwrap();

    tex_pro
        .node_graph
        .connect(multiply_node, output_node, SlotId(0), SlotId(0))
        .unwrap();

    tex_pro.process();

    ensure_out_dir();
    let size = 256;
    image::save_buffer(
        &Path::new(&"out/multiply_node.png"),
        &image::RgbaImage::from_vec(size, size, tex_pro.get_output(output_node).unwrap()).unwrap(),
        size,
        size,
        image::ColorType::RGBA(8),
    )
    .unwrap();
}

#[test]
fn divide_node() {
    let mut tex_pro = TextureProcessor::new();

    let image_node = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::Image("data/image_1.png".to_string())))
        .unwrap();
    let divide_node = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::Mix(MixType::Divide)))
        .unwrap();
    let output_node = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::OutputGray))
        .unwrap();

    tex_pro
        .node_graph
        .connect(image_node, divide_node, SlotId(0), SlotId(0))
        .unwrap();
    tex_pro
        .node_graph
        .connect(image_node, divide_node, SlotId(3), SlotId(1))
        .unwrap();

    tex_pro
        .node_graph
        .connect(divide_node, output_node, SlotId(0), SlotId(0))
        .unwrap();

    tex_pro.process();

    ensure_out_dir();
    let size = 256;
    image::save_buffer(
        &Path::new(&"out/divide_node.png"),
        &image::RgbaImage::from_vec(size, size, tex_pro.get_output(output_node).unwrap()).unwrap(),
        size,
        size,
        image::ColorType::RGBA(8),
    )
    .unwrap();
}

#[test]
fn change_mix_type() {
    let mut tex_pro = TextureProcessor::new();

    let mix_node = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::Mix(MixType::Add)))
        .unwrap();

    tex_pro.node_graph.set_mix_type(mix_node, MixType::Subtract);

    assert_eq!(
        tex_pro.node_graph.node_with_id(mix_node).unwrap().node_type,
        NodeType::Mix(MixType::Subtract)
    );
}

// #[test]
// fn input_output() {
//     let mut tex_pro = TextureProcessor::new();

//     let input_node = tex_pro.add_input_node(&image::open(&Path::new(&"data/image_2.png")).unwrap());
//     let output_node = tex_pro.add_node(Node::new(NodeType::Output));

//     tex_pro
//         .connect(input_node, output_node, Slot(0), Slot(0))
//         .unwrap();
//     tex_pro
//         .connect(input_node, output_node, Slot(1), Slot(1))
//         .unwrap();
//     tex_pro
//         .connect(input_node, output_node, Slot(2), Slot(2))
//         .unwrap();
//     tex_pro
//         .connect(input_node, output_node, Slot(3), Slot(3))
//         .unwrap();

//     tex_pro.process();

//     image::save_buffer(
//         &Path::new(&"out/input_output.png"),
//         &image::RgbaImage::from_vec(256, 256, tex_pro.get_output(output_node).unwrap())
//             .unwrap(),
//         256,
//         256,
//         image::ColorType::RGBA(8),
//     )
//     .unwrap();
// }

// #[test]
// fn read_write() {
//     let mut tex_pro = TextureProcessor::new();

//     let input_image_1 = tex_pro.add_node(Node::new(NodeType::Image("data/image_1.png".to_string())));
//     let write_node = tex_pro.add_node(Node::new(NodeType::Write("out/read_write.png".to_string())));

//     tex_pro
//         .connect(input_image_1, write_node, Slot(0), Slot(0))
//         .unwrap();
//     tex_pro
//         .connect(input_image_1, write_node, Slot(1), Slot(1))
//         .unwrap();
//     tex_pro
//         .connect(input_image_1, write_node, Slot(2), Slot(2))
//         .unwrap();
//     tex_pro
//         .connect(input_image_1, write_node, Slot(3), Slot(3))
//         .unwrap();

//     tex_pro.process();
// }

// #[test]
// fn shuffle() {
//     let mut tex_pro = TextureProcessor::new();

//     let input_heart_256 =
//         tex_pro.add_node(Node::new(NodeType::Image("data/heart_256.png".to_string())));
//     let write_node = tex_pro.add_node(Node::new(NodeType::Write("out/shuffle.png".to_string())));

//     tex_pro
//         .connect(input_heart_256, write_node, Slot(0), Slot(1))
//         .unwrap();
//     tex_pro
//         .connect(input_heart_256, write_node, Slot(1), Slot(2))
//         .unwrap();
//     tex_pro
//         .connect(input_heart_256, write_node, Slot(2), Slot(0))
//         .unwrap();
//     tex_pro
//         .connect(input_heart_256, write_node, Slot(3), Slot(3))
//         .unwrap();

//     tex_pro.process();
// }

// #[test]
// fn combine_different_sizes() {
//     let mut tex_pro = TextureProcessor::new();

//     let input_heart_256 =
//         tex_pro.add_node(Node::new(NodeType::Image("data/heart_128.png".to_string())));
//     let input_image_1 = tex_pro.add_node(Node::new(NodeType::Image("data/image_1.png".to_string())));
//     let write_node = tex_pro.add_node(Node::new(NodeType::Write(
//         "out/combine_different_sizes.png".to_string(),
//     )));

//     tex_pro
//         .connect(input_heart_256, write_node, Slot(0), Slot(1))
//         .unwrap();
//     tex_pro
//         .connect(input_heart_256, write_node, Slot(1), Slot(2))
//         .unwrap();
//     tex_pro
//         .connect(input_image_1, write_node, Slot(2), Slot(0))
//         .unwrap();
//     tex_pro
//         .connect(input_image_1, write_node, Slot(3), Slot(3))
//         .unwrap();

//     tex_pro.process();
// }

// #[test]
// fn invert() {
//     let mut tex_pro = TextureProcessor::new();

//     let input_heart_256 =
//         tex_pro.add_node(Node::new(NodeType::Image("data/heart_256.png".to_string())));
//     let invert_node = tex_pro.add_node(Node::new(NodeType::Invert));
//     let write_node = tex_pro.add_node(Node::new(NodeType::Write("out/invert.png".to_string())));

//     tex_pro
//         .connect(input_heart_256, invert_node, Slot(0), Slot(0))
//         .unwrap();

//     tex_pro
//         .connect(invert_node, write_node, Slot(0), Slot(0))
//         .unwrap();
//     tex_pro
//         .connect(input_heart_256, write_node, Slot(1), Slot(1))
//         .unwrap();
//     tex_pro
//         .connect(input_heart_256, write_node, Slot(2), Slot(2))
//         .unwrap();
//     tex_pro
//         .connect(input_heart_256, write_node, Slot(3), Slot(3))
//         .unwrap();

//     tex_pro.process();
// }

// #[test]
// fn add() {
//     let mut tex_pro = TextureProcessor::new();

//     let input_image_1 = tex_pro.add_node(Node::new(NodeType::Image("data/image_1.png".to_string())));
//     let input_white = tex_pro.add_node(Node::new(NodeType::Image("data/white.png".to_string())));
//     let add_node = tex_pro.add_node(Node::new(NodeType::Mix(MixType::Add)));
//     let write_node = tex_pro.add_node(Node::new(NodeType::Write("out/add.png".to_string())));

//     tex_pro
//         .connect(input_image_1, add_node, Slot(0), Slot(0))
//         .unwrap();
//     tex_pro
//         .connect(input_image_1, add_node, Slot(1), Slot(1))
//         .unwrap();

//     tex_pro
//         .connect(add_node, write_node, Slot(0), Slot(0))
//         .unwrap();
//     tex_pro
//         .connect(add_node, write_node, Slot(0), Slot(1))
//         .unwrap();
//     tex_pro
//         .connect(add_node, write_node, Slot(0), Slot(2))
//         .unwrap();
//     tex_pro
//         .connect(input_white, write_node, Slot(0), Slot(3))
//         .unwrap();

//     tex_pro.process();
// }

// #[test]
// fn multiply() {
//     let mut tex_pro = TextureProcessor::new();

//     let input_image_1 = tex_pro.add_node(Node::new(NodeType::Image("data/image_1.png".to_string())));
//     let input_white = tex_pro.add_node(Node::new(NodeType::Image("data/white.png".to_string())));
//     let multiply_node = tex_pro.add_node(Node::new(NodeType::Mix(MixType::Multiply)));
//     let write_node = tex_pro.add_node(Node::new(NodeType::Write("out/multiply.png".to_string())));

//     tex_pro
//         .connect(input_image_1, multiply_node, Slot(0), Slot(0))
//         .unwrap();
//     tex_pro
//         .connect(input_image_1, multiply_node, Slot(3), Slot(1))
//         .unwrap();

//     tex_pro
//         .connect(multiply_node, write_node, Slot(0), Slot(0))
//         .unwrap();
//     tex_pro
//         .connect(multiply_node, write_node, Slot(0), Slot(1))
//         .unwrap();
//     tex_pro
//         .connect(multiply_node, write_node, Slot(0), Slot(2))
//         .unwrap();
//     tex_pro
//         .connect(input_white, write_node, Slot(0), Slot(3))
//         .unwrap();

//     tex_pro.process();
// }
