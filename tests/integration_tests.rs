use std::path::Path;
use texture_processor::{
    dag::TextureProcessor,
    node::{Node, NodeType},
    node_graph::{NodeGraph, SlotId, NodeId},
};

#[test]
fn input_output() {
    let mut tex_pro = TextureProcessor::new();

    let input_node = tex_pro.node_graph.add_node(Node::new(NodeType::Read("data/image_2.png".to_string()))).unwrap();
    let output_node = tex_pro.node_graph.add_node(Node::new(NodeType::OutputRgba)).unwrap();

    tex_pro.node_graph
        .connect(input_node, output_node, SlotId(0), SlotId(0))
        .unwrap();
    tex_pro.node_graph
        .connect(input_node, output_node, SlotId(1), SlotId(1))
        .unwrap();
    tex_pro.node_graph
        .connect(input_node, output_node, SlotId(2), SlotId(2))
        .unwrap();
    tex_pro.node_graph
        .connect(input_node, output_node, SlotId(3), SlotId(3))
        .unwrap();

    tex_pro.process();

    image::save_buffer(
        &Path::new(&"out/input_output.png"),
        &image::RgbaImage::from_vec(256, 256, tex_pro.get_output_rgba(output_node).unwrap())
            .unwrap(),
        256,
        256,
        image::ColorType::RGBA(8),
    )
    .unwrap();
}

#[test]
fn input_output_2() {
    let tex_pro_compare = input_output_2_internal();

    for _ in 0..30 {
        let tex_pro = input_output_2_internal();

        for node_data_cmp in &tex_pro_compare.node_datas {
            assert!(tex_pro.node_datas.iter().any(|node_data| *node_data == *node_data_cmp));
        }
    }
}

fn input_output_2_internal() -> TextureProcessor {
    let mut tex_pro = TextureProcessor::new();

    let input_node_1 = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::Read("data/px_1.png".to_string()))).unwrap();
    let input_node_2 = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::Read("data/px_1.png".to_string()))).unwrap();
    let output_node = tex_pro.node_graph.add_node(Node::new(NodeType::OutputRgba)).unwrap();

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

    let red_node = tex_pro.node_graph.add_node(Node::new(NodeType::Value(0.2))).unwrap();
    let green_node = tex_pro.node_graph.add_node(Node::new(NodeType::Value(0.5))).unwrap();
    let blue_node = tex_pro.node_graph.add_node(Node::new(NodeType::Value(0.7))).unwrap();
    let alpha_node = tex_pro.node_graph.add_node(Node::new(NodeType::Value(1.))).unwrap();

    let output_node = tex_pro.node_graph.add_node(Node::new(NodeType::OutputRgba)).unwrap();

    tex_pro.node_graph
        .connect(red_node, output_node, SlotId(0), SlotId(0))
        .unwrap();
    tex_pro.node_graph
        .connect(green_node, output_node, SlotId(0), SlotId(1))
        .unwrap();
    tex_pro.node_graph
        .connect(blue_node, output_node, SlotId(0), SlotId(2))
        .unwrap();
    tex_pro.node_graph
        .connect(alpha_node, output_node, SlotId(0), SlotId(3))
        .unwrap();

    tex_pro.process();

    image::save_buffer(
        &Path::new(&"out/value_node.png"),
        &image::RgbaImage::from_vec(1, 1, tex_pro.get_output_rgba(output_node).unwrap())
            .unwrap(),
        1,
        1,
        image::ColorType::RGBA(8),
    )
    .unwrap();
}

// #[test]
// fn add_node() {
//     let mut tex_pro = TextureProcessor::new();

//     let input_node = tex_pro.node_graph.add_node(Node::new(NodeType::Read("data/image_2.png".to_string()))).unwrap();
//     let add_node = tex_pro.node_graph.add_node(Node::new(NodeType::Add)).unwrap();
//     let output_node = tex_pro.node_graph.add_node(Node::new(NodeType::OutputRgba)).unwrap();

//     tex_pro.node_graph
//         .connect(input_node, add_node, SlotId(0), SlotId(0))
//         .unwrap();

//     tex_pro.node_graph
//         .connect(add_node, output_node, SlotId(0), SlotId(0))
//         .unwrap();
//     tex_pro.node_graph
//         .connect(add_node, output_node, SlotId(0), SlotId(1))
//         .unwrap();
//     tex_pro.node_graph
//         .connect(add_node, output_node, SlotId(0), SlotId(2))
//         .unwrap();
//     tex_pro.node_graph
//         .connect(add_node, output_node, SlotId(0), SlotId(3))
//         .unwrap();

//     tex_pro.process();

//     image::save_buffer(
//         &Path::new(&"out/input_output.png"),
//         &image::RgbaImage::from_vec(256, 256, tex_pro.get_output_rgba(output_node).unwrap())
//             .unwrap(),
//         256,
//         256,
//         image::ColorType::RGBA(8),
//     )
//     .unwrap();
// }

#[test]
fn graph_node_rgba() {
    // Nested graph
    let mut nested_graph = NodeGraph::new();

    let nested_input_node = nested_graph.add_external_input_rgba(vec![SlotId(0), SlotId(1), SlotId(2), SlotId(3)]).unwrap();
    let nested_output_node = nested_graph.add_external_output_rgba(vec![SlotId(0), SlotId(1), SlotId(2), SlotId(3)]).unwrap();

    nested_graph.connect(nested_input_node, nested_output_node, SlotId(0), SlotId(0)).unwrap();
    nested_graph.connect(nested_input_node, nested_output_node, SlotId(1), SlotId(1)).unwrap();
    nested_graph.connect(nested_input_node, nested_output_node, SlotId(2), SlotId(2)).unwrap();
    nested_graph.connect(nested_input_node, nested_output_node, SlotId(3), SlotId(3)).unwrap();


    // Texture Processor
    let mut tex_pro = TextureProcessor::new();

    let input_node = tex_pro.node_graph.add_node_with_id(Node::new(NodeType::Read("data/image_2.png".to_string())), NodeId(1)).unwrap();
    let graph_node = tex_pro.node_graph.add_node_with_id(Node::new(NodeType::Graph(nested_graph)), NodeId(2)).unwrap();
    let output_node = tex_pro.node_graph.add_node_with_id(Node::new(NodeType::OutputRgba), NodeId(3)).unwrap();

    tex_pro.node_graph.connect(input_node, graph_node, SlotId(0), SlotId(0)).unwrap();
    tex_pro.node_graph.connect(input_node, graph_node, SlotId(1), SlotId(1)).unwrap();
    tex_pro.node_graph.connect(input_node, graph_node, SlotId(2), SlotId(2)).unwrap();
    tex_pro.node_graph.connect(input_node, graph_node, SlotId(3), SlotId(3)).unwrap();

    tex_pro.node_graph.connect(graph_node, output_node, SlotId(0), SlotId(0)).unwrap();
    tex_pro.node_graph.connect(graph_node, output_node, SlotId(1), SlotId(1)).unwrap();
    tex_pro.node_graph.connect(graph_node, output_node, SlotId(2), SlotId(2)).unwrap();
    tex_pro.node_graph.connect(graph_node, output_node, SlotId(3), SlotId(3)).unwrap();

    tex_pro.process();

    // Output
    image::save_buffer(
        &Path::new(&"out/graph_node_rgba.png"),
        &image::RgbaImage::from_vec(256, 256, tex_pro.get_output_rgba(output_node).unwrap())
            .unwrap(),
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
    let nested_output_node = nested_graph.add_node_with_id(Node::new(NodeType::OutputGray), NodeId(10)).unwrap();

    nested_graph.connect(nested_input_node, nested_output_node, SlotId(0), SlotId(0)).unwrap();


    // Texture Processor
    let mut tex_pro = TextureProcessor::new();

    let input_node = tex_pro.node_graph.add_node_with_id(Node::new(NodeType::Read("data/image_2.png".to_string())), NodeId(1)).unwrap();
    let graph_node = tex_pro.node_graph.add_node_with_id(Node::new(NodeType::Graph(nested_graph)), NodeId(2)).unwrap();
    let output_node = tex_pro.node_graph.add_node_with_id(Node::new(NodeType::OutputRgba), NodeId(3)).unwrap();

    tex_pro.node_graph.connect(input_node, graph_node, SlotId(0), SlotId(0)).unwrap();

    tex_pro.node_graph.connect(graph_node, output_node, SlotId(0), SlotId(0)).unwrap();
    tex_pro.node_graph.connect(graph_node, output_node, SlotId(0), SlotId(1)).unwrap();
    tex_pro.node_graph.connect(graph_node, output_node, SlotId(0), SlotId(2)).unwrap();
    tex_pro.node_graph.connect(graph_node, output_node, SlotId(0), SlotId(3)).unwrap();

    tex_pro.process();

    // Output
    image::save_buffer(
        &Path::new(&"out/graph_node_gray.png"),
        &image::RgbaImage::from_vec(256, 256, tex_pro.get_output_rgba(output_node).unwrap())
            .unwrap(),
        256,
        256,
        image::ColorType::RGBA(8),
    )
    .unwrap();
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
//         &image::RgbaImage::from_vec(256, 256, tex_pro.get_output_rgba(output_node).unwrap())
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

//     let input_image_1 = tex_pro.add_node(Node::new(NodeType::Read("data/image_1.png".to_string())));
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
//         tex_pro.add_node(Node::new(NodeType::Read("data/heart_256.png".to_string())));
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
//         tex_pro.add_node(Node::new(NodeType::Read("data/heart_128.png".to_string())));
//     let input_image_1 = tex_pro.add_node(Node::new(NodeType::Read("data/image_1.png".to_string())));
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
//         tex_pro.add_node(Node::new(NodeType::Read("data/heart_256.png".to_string())));
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

//     let input_image_1 = tex_pro.add_node(Node::new(NodeType::Read("data/image_1.png".to_string())));
//     let input_white = tex_pro.add_node(Node::new(NodeType::Read("data/white.png".to_string())));
//     let add_node = tex_pro.add_node(Node::new(NodeType::Add));
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

//     let input_image_1 = tex_pro.add_node(Node::new(NodeType::Read("data/image_1.png".to_string())));
//     let input_white = tex_pro.add_node(Node::new(NodeType::Read("data/white.png".to_string())));
//     let multiply_node = tex_pro.add_node(Node::new(NodeType::Multiply));
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
