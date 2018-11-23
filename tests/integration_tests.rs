extern crate image;
extern crate texture_processor;

use std::path::Path;
use texture_processor::{
    dag::TextureProcessor,
    node::{Node, NodeType, Slot},
};

#[test]
fn input_output() {
    let mut tex_pro = TextureProcessor::new();

    let input_node = tex_pro.add_input_node(&image::open(&Path::new(&"data/image_2.png")).unwrap());
    let output_node = tex_pro.add_node(Node::new(NodeType::Output));

    tex_pro.connect(input_node, output_node, Slot(0), Slot(0));
    tex_pro.connect(input_node, output_node, Slot(1), Slot(1));
    tex_pro.connect(input_node, output_node, Slot(2), Slot(2));
    tex_pro.connect(input_node, output_node, Slot(3), Slot(3));

    tex_pro.process();

    image::save_buffer(
        &Path::new(&"out/input_output.png"),
        &image::RgbaImage::from_vec(256, 256, tex_pro.get_output_rgba(output_node).unwrap()).unwrap(),
        256,
        256,
        image::ColorType::RGBA(8),
    ).unwrap();
}

#[test]
fn read_write() {
    let mut tex_pro = TextureProcessor::new();

    let input_image_1 = tex_pro.add_node(Node::new(NodeType::Read("data/image_1.png".to_string())));
    let write_node = tex_pro.add_node(Node::new(NodeType::Write("out/read_write.png".to_string())));

    tex_pro.connect(input_image_1, write_node, Slot(0), Slot(0));
    tex_pro.connect(input_image_1, write_node, Slot(1), Slot(1));
    tex_pro.connect(input_image_1, write_node, Slot(2), Slot(2));
    tex_pro.connect(input_image_1, write_node, Slot(3), Slot(3));

    tex_pro.process();
}

#[test]
fn shuffle() {
    let mut tex_pro = TextureProcessor::new();

    let input_heart_256 =
        tex_pro.add_node(Node::new(NodeType::Read("data/heart_256.png".to_string())));
    let write_node = tex_pro.add_node(Node::new(NodeType::Write("out/shuffle.png".to_string())));

    tex_pro.connect(input_heart_256, write_node, Slot(0), Slot(1));
    tex_pro.connect(input_heart_256, write_node, Slot(1), Slot(2));
    tex_pro.connect(input_heart_256, write_node, Slot(2), Slot(0));
    tex_pro.connect(input_heart_256, write_node, Slot(3), Slot(3));

    tex_pro.process();
}

#[test]
fn combine_different_sizes() {
    let mut tex_pro = TextureProcessor::new();

    let input_heart_256 =
        tex_pro.add_node(Node::new(NodeType::Read("data/heart_128.png".to_string())));
    let input_image_1 = tex_pro.add_node(Node::new(NodeType::Read("data/image_1.png".to_string())));
    let write_node = tex_pro.add_node(Node::new(NodeType::Write(
        "out/combine_different_sizes.png".to_string(),
    )));

    tex_pro.connect(input_heart_256, write_node, Slot(0), Slot(1));
    tex_pro.connect(input_heart_256, write_node, Slot(1), Slot(2));
    tex_pro.connect(input_image_1, write_node, Slot(2), Slot(0));
    tex_pro.connect(input_image_1, write_node, Slot(3), Slot(3));

    tex_pro.process();
}

#[test]
fn invert() {
    let mut tex_pro = TextureProcessor::new();

    let input_heart_256 =
        tex_pro.add_node(Node::new(NodeType::Read("data/heart_256.png".to_string())));
    let invert_node = tex_pro.add_node(Node::new(NodeType::Invert));
    let write_node = tex_pro.add_node(Node::new(NodeType::Write("out/invert.png".to_string())));

    tex_pro.connect(input_heart_256, invert_node, Slot(0), Slot(0));

    tex_pro.connect(invert_node, write_node, Slot(0), Slot(0));
    tex_pro.connect(input_heart_256, write_node, Slot(1), Slot(1));
    tex_pro.connect(input_heart_256, write_node, Slot(2), Slot(2));
    tex_pro.connect(input_heart_256, write_node, Slot(3), Slot(3));

    tex_pro.process();
}

#[test]
fn add() {
    let mut tex_pro = TextureProcessor::new();

    let input_image_1 = tex_pro.add_node(Node::new(NodeType::Read("data/image_1.png".to_string())));
    let input_white = tex_pro.add_node(Node::new(NodeType::Read("data/white.png".to_string())));
    let add_node = tex_pro.add_node(Node::new(NodeType::Add));
    let write_node = tex_pro.add_node(Node::new(NodeType::Write("out/add.png".to_string())));

    tex_pro.connect(input_image_1, add_node, Slot(0), Slot(0));
    tex_pro.connect(input_image_1, add_node, Slot(1), Slot(1));

    tex_pro.connect(add_node, write_node, Slot(0), Slot(0));
    tex_pro.connect(add_node, write_node, Slot(0), Slot(1));
    tex_pro.connect(add_node, write_node, Slot(0), Slot(2));
    tex_pro.connect(input_white, write_node, Slot(0), Slot(3));

    tex_pro.process();
}

#[test]
fn multiply() {
    let mut tex_pro = TextureProcessor::new();

    let input_image_1 = tex_pro.add_node(Node::new(NodeType::Read("data/image_1.png".to_string())));
    let input_white = tex_pro.add_node(Node::new(NodeType::Read("data/white.png".to_string())));
    let multiply_node = tex_pro.add_node(Node::new(NodeType::Multiply));
    let write_node = tex_pro.add_node(Node::new(NodeType::Write("out/multiply.png".to_string())));

    tex_pro.connect(input_image_1, multiply_node, Slot(0), Slot(0));
    tex_pro.connect(input_image_1, multiply_node, Slot(3), Slot(1));

    tex_pro.connect(multiply_node, write_node, Slot(0), Slot(0));
    tex_pro.connect(multiply_node, write_node, Slot(0), Slot(1));
    tex_pro.connect(multiply_node, write_node, Slot(0), Slot(2));
    tex_pro.connect(input_white, write_node, Slot(0), Slot(3));

    tex_pro.process();
}
