use vismut_core::{
    live_graph::{LiveGraph, NodeState},
    node::{
        embed::EmbeddedSlotDataId, mix::MixType, node_type::NodeType, Node, ResizeFilter,
        ResizePolicy, Side,
    },
    node_graph::{NodeGraph, NodeId, SlotId},
    slot_data::Size,
    slot_image::SlotImage,
    texture_processor::TextureProcessor,
};
use ntest::timeout;
use std::{
    fs::create_dir,
    path::Path,
    sync::{atomic::Ordering, Arc, RwLock},
    thread,
    time::Duration,
};

const DIR_OUT: &str = "out";
const DIR_CMP: &str = &"data/test_compare";
const IMAGE_1: &str = "data/image_1.png";
const IMAGE_2: &str = "data/image_2.png";
const HEART_128: &str = "data/heart_128.png";
const HEART_256: &str = "data/heart_256.png";
const HEART_WIDE: &str = "data/heart_wide.png";
const HEART_TALL: &str = "data/heart_tall.png";
const HEART_110: &str = "data/heart_110.png";
const CLOUDS: &str = "data/clouds.png";

fn ensure_out_dir() {
    match create_dir(Path::new(DIR_OUT)) {
        _ => (),
    };
}

fn images_equal<P: AsRef<Path>, Q: AsRef<Path>>(path_1: P, path_2: Q) -> bool {
    let image_1 = image::open(path_1).unwrap();
    let raw_pixels_1 = image_1.as_flat_samples_u8().unwrap().samples;
    let image_2 = image::open(path_2).unwrap();
    let raw_pixels_2 = image_2.as_flat_samples_u8().unwrap().samples;

    raw_pixels_1.iter().eq(raw_pixels_2.iter())
}

fn tex_pro_new() -> Arc<TextureProcessor> {
    TextureProcessor::new(Arc::new(10_000_000.into()))
}

#[test]
#[timeout(20_000)]
fn input_output() {
    const SIZE: u32 = 256;
    const PATH_IN: &str = IMAGE_2;
    const PATH_OUT: &str = &"out/input_output.png";

    let tex_pro = tex_pro_new();
    let live_graph = tex_pro.new_live_graph().unwrap();
    let output_node = {
        let mut live_graph = live_graph.write().unwrap();
        let input_node = live_graph
            .add_node(Node::new(NodeType::Image(PATH_IN.clone().into())))
            .unwrap();
        let output_node = live_graph
            .add_node(Node::new(NodeType::OutputRgba("out".into())))
            .unwrap();

        live_graph
            .connect(input_node, output_node, SlotId(0), SlotId(0))
            .unwrap();

        output_node
    };

    ensure_out_dir();
    image::save_buffer(
        &Path::new(&PATH_OUT),
        &image::RgbaImage::from_vec(
            SIZE,
            SIZE,
            LiveGraph::await_clean_read(&live_graph, output_node)
                .unwrap()
                .buffer_rgba(output_node, SlotId(0))
                .unwrap(),
        )
        .unwrap(),
        SIZE,
        SIZE,
        image::ColorType::Rgba8,
    )
    .unwrap();

    assert!(images_equal(PATH_IN, PATH_OUT));
}

fn calculate_slot(live_graph: &Arc<RwLock<LiveGraph>>, node_id: NodeId, slot_id: SlotId) {
    for buf in LiveGraph::await_clean_read(live_graph, node_id)
        .unwrap()
        .slot_data(node_id, slot_id)
        .unwrap()
        .image
        .bufs()
    {
        let _ = buf.transient_buffer();
    }
}

#[test]
// #[timeout(1_000)]
fn deadlock() {
    let tex_pro = tex_pro_new();
    let live_graph = tex_pro.new_live_graph().unwrap();

    let mix_node_1 = {
        let mut live_graph = live_graph.write().unwrap();
        let value_node = live_graph
            .add_node(Node::new(NodeType::Value(0.0)))
            .unwrap();
        let mix_node_1 = live_graph
            .add_node(Node::new(NodeType::Mix(MixType::Add)))
            .unwrap();

        live_graph
            .connect(value_node, mix_node_1, SlotId(0), SlotId(0))
            .unwrap();
        live_graph
            .connect(value_node, mix_node_1, SlotId(0), SlotId(1))
            .unwrap();

        mix_node_1
    };

    LiveGraph::await_clean_read(&live_graph, mix_node_1)
        .unwrap()
        .slot_data(mix_node_1, SlotId(0))
        .unwrap();
}

#[test]
#[timeout(20_000)]
fn drive_cache() {
    const VAL: [f32; 4] = [0.0, 0.3, 0.7, 1.0];
    let tex_pro = tex_pro_new();
    // Setting the slot_data_ram_cap at 16 bytes should result in the Rgba8de getting written
    tex_pro.memory_threshold.store(16, Ordering::Relaxed);
    let live_graph = tex_pro.new_live_graph().unwrap();

    let (rgba_node, value_nodes, mix_node_1, mix_node_2) = {
        let mut live_graph = live_graph.write().unwrap();
        live_graph.use_cache = true;

        // Rgba8de should be 4 channels * 4 bytes = 16 bytes
        let rgba_node = live_graph
            .add_node(Node::new(NodeType::CombineRgba))
            .unwrap();

        // 4 value nodes should be 4 channels * 4 bytes = 16 bytes
        let mut value_nodes: Vec<NodeId> = Vec::new();
        for (i, val) in VAL.iter().enumerate() {
            let new_node = live_graph
                .add_node(Node::new(NodeType::Value(*val)))
                .unwrap();
            value_nodes.push(new_node);
            live_graph
                .connect(new_node, rgba_node, SlotId(0), SlotId(i as u32))
                .unwrap();
        }

        // 2 mix nodes should be 2 nodes * 4 channels * 4 bytes = 32 bytes
        let mix_node_1 = live_graph
            .add_node(Node::new(NodeType::Mix(MixType::Add)))
            .unwrap();
        let mix_node_2 = live_graph
            .add_node(Node::new(NodeType::Mix(MixType::Add)))
            .unwrap();

        live_graph
            .connect(rgba_node, mix_node_1, SlotId(0), SlotId(0))
            .unwrap();
        live_graph
            .connect(mix_node_1, mix_node_2, SlotId(0), SlotId(0))
            .unwrap();

        (rgba_node, value_nodes, mix_node_1, mix_node_2)
    };

    {
        let _ = LiveGraph::await_clean_read(&live_graph, mix_node_2).unwrap();
    }
    thread::sleep(Duration::from_millis(100));
    {
        // Assert that the right things are on drive and in RAM.
        calculate_slot(&live_graph, mix_node_2, SlotId(0));
        let live_graph = LiveGraph::await_clean_read(&live_graph, mix_node_2).unwrap();

        for node_id in &value_nodes {
            assert!(!live_graph.slot_in_memory(*node_id, SlotId(0)).unwrap());
        }

        assert!(!live_graph.slot_in_memory(rgba_node, SlotId(0)).unwrap());
        assert!(!live_graph.slot_in_memory(mix_node_1, SlotId(0)).unwrap());
        assert!(live_graph.slot_in_memory(mix_node_2, SlotId(0)).unwrap());
    }

    {
        let live_graph = live_graph.write().unwrap();
        if let SlotImage::Rgba(bufs) = &live_graph.slot_data(rgba_node, SlotId(0)).unwrap().image {
            let pixel = {
                let pixel = [
                    bufs[0].transient_buffer(),
                    bufs[1].transient_buffer(),
                    bufs[2].transient_buffer(),
                    bufs[3].transient_buffer(),
                ];

                [
                    pixel[0].buffer().pixels().next().unwrap().0[0],
                    pixel[1].buffer().pixels().next().unwrap().0[0],
                    pixel[2].buffer().pixels().next().unwrap().0[0],
                    pixel[3].buffer().pixels().next().unwrap().0[0],
                ]
            };

            assert_eq!(pixel, VAL);
        } else {
            panic!()
        }
    }

    // Test if the right thing happens when a slot_data on drive is retrieved...
    // Loads this slot_data into RAM.
    // calculate_slot(&live_graph, rgba_node, SlotId(0));
    {
        let live_graph = LiveGraph::await_clean_read(&live_graph, rgba_node).unwrap();

        thread::sleep(Duration::from_millis(100));

        for node_id in value_nodes {
            assert!(live_graph.slot_in_memory(node_id, SlotId(0)).unwrap());
        }

        assert!(live_graph.slot_in_memory(rgba_node, SlotId(0)).unwrap());
        assert!(!live_graph.slot_in_memory(mix_node_1, SlotId(0)).unwrap());
        assert!(!live_graph.slot_in_memory(mix_node_2, SlotId(0)).unwrap());
    }
}

#[test]
#[timeout(20_000)]
fn no_cache() {
    let tex_pro = tex_pro_new();
    let live_graph = tex_pro.new_live_graph().unwrap();

    let (value_node, output_node) = {
        let mut live_graph = live_graph.write().unwrap();
        let value_node = live_graph
            .add_node(Node::new(NodeType::Value(1.0)))
            .unwrap();
        let output_node = live_graph
            .add_node(Node::new(NodeType::OutputGray("out".into())))
            .unwrap();

        live_graph
            .connect(value_node, output_node, SlotId(0), SlotId(0))
            .unwrap();

        (value_node, output_node)
    };

    assert!(LiveGraph::await_clean_read(&live_graph, output_node)
        .unwrap()
        .slot_data(value_node, SlotId(0))
        .is_err());
}

#[test]
#[timeout(20_000)]
fn use_cache() {
    let tex_pro = tex_pro_new();
    let live_graph = tex_pro.new_live_graph().unwrap();

    let (value_node, output_node) = {
        let mut live_graph = live_graph.write().unwrap();

        let value_node = live_graph
            .add_node(Node::new(NodeType::Value(1.0)))
            .unwrap();
        let output_node = live_graph
            .add_node(Node::new(NodeType::OutputGray("out".into())))
            .unwrap();

        live_graph
            .connect(value_node, output_node, SlotId(0), SlotId(0))
            .unwrap();

        live_graph.use_cache = true;
        (value_node, output_node)
    };

    assert!(LiveGraph::await_clean_read(&live_graph, output_node)
        .unwrap()
        .slot_data(value_node, SlotId(0))
        .is_ok());
}

#[test]
#[timeout(20_000)]
fn request_empty_buffer() {
    let tex_pro = tex_pro_new();
    let live_graph = tex_pro.new_live_graph().unwrap();

    let output_node = {
        let mut live_graph = live_graph.write().unwrap();

        let mix_node = live_graph
            .add_node(Node::new(NodeType::Mix(MixType::default())))
            .unwrap();
        let output_node = live_graph
            .add_node(Node::new(NodeType::OutputRgba("out".into())))
            .unwrap();

        live_graph
            .connect(mix_node, output_node, SlotId(0), SlotId(0))
            .unwrap();
        output_node
    };

    let _ = LiveGraph::await_clean_read(&live_graph, output_node)
        .unwrap()
        .buffer_rgba(output_node, SlotId(0))
        .unwrap();
}

#[test]
#[timeout(20_000)]
fn input_output_intercept() {
    const SIZE: u32 = 30;
    const SIZE_LARGE: u32 = 20;
    const SIZE_SMALL: u32 = 10;
    const PATH_IN: &str = IMAGE_2;

    let tex_pro = tex_pro_new();
    let live_graph = tex_pro.new_live_graph().unwrap();

    let (resize_node_1, output_node) = {
        let mut live_graph = live_graph.write().unwrap();
        live_graph.auto_update = true;

        let input_node = live_graph
            .add_node(Node::new(NodeType::Image(PATH_IN.clone().into())))
            .unwrap();
        let resize_node_1 = live_graph
            .add_node(
                Node::new(NodeType::Mix(MixType::default()))
                    .resize_filter(ResizeFilter::Lanczos3)
                    .resize_policy(ResizePolicy::SpecificSize(Size::new(
                        SIZE_SMALL, SIZE_SMALL,
                    ))),
            )
            .unwrap();
        let resize_node_2 = live_graph
            .add_node(
                Node::new(NodeType::Mix(MixType::default()))
                    .resize_filter(ResizeFilter::Lanczos3)
                    .resize_policy(ResizePolicy::SpecificSize(Size::new(
                        SIZE_LARGE, SIZE_LARGE,
                    ))),
            )
            .unwrap();
        let resize_node_3 = live_graph
            .add_node(
                Node::new(NodeType::Mix(MixType::default()))
                    .resize_filter(ResizeFilter::Lanczos3)
                    .resize_policy(ResizePolicy::SpecificSize(Size::new(SIZE, SIZE))),
            )
            .unwrap();
        let output_node = live_graph
            .add_node(Node::new(NodeType::OutputRgba("out".into())))
            .unwrap();

        live_graph
            .connect(input_node, resize_node_1, SlotId(0), SlotId(0))
            .unwrap();
        live_graph
            .connect(resize_node_1, resize_node_2, SlotId(0), SlotId(0))
            .unwrap();
        live_graph
            .connect(resize_node_2, resize_node_3, SlotId(0), SlotId(0))
            .unwrap();
        live_graph
            .connect(resize_node_3, output_node, SlotId(0), SlotId(0))
            .unwrap();

        (resize_node_1, output_node)
    };

    let mut intercepted = false;
    loop {
        let live_graph = live_graph.read().unwrap();

        if live_graph.node_state(output_node).unwrap() == NodeState::Clean {
            break;
        } else if live_graph.node_state(resize_node_1).unwrap() == NodeState::Clean {
            intercepted = true;
            break;
        }
    }
    assert!(intercepted);
}

#[test]
#[timeout(20_000)]
fn priority() {
    assert!(!priority_internal(2, -1));
    assert!(priority_internal(1, 1));
    assert!(priority_internal(2, 1));
}

fn priority_internal(max_processing: usize, large_priority: i8) -> bool {
    const SIZE_LARGE: u32 = 400;
    const SIZE_SMALL: u32 = 400;

    let tex_pro = tex_pro_new();
    tex_pro.set_max_processing_nodes(max_processing).unwrap();

    let live_graph = tex_pro.new_live_graph().unwrap();

    let (resize_small_1, resize_small_2, resize_large) = {
        let mut live_graph = live_graph.write().unwrap();

        let value_node = live_graph
            .add_node(Node::new(NodeType::Value(0.5)))
            .unwrap();
        let resize_small_1 = live_graph
            .add_node(
                Node::new(NodeType::Mix(MixType::default()))
                    .resize_filter(ResizeFilter::Nearest)
                    .resize_policy(ResizePolicy::SpecificSize(Size::new(
                        SIZE_SMALL, SIZE_SMALL,
                    ))),
            )
            .unwrap();
        let resize_small_2 = live_graph
            .add_node(
                Node::new(NodeType::Mix(MixType::default()))
                    .resize_filter(ResizeFilter::Nearest)
                    .resize_policy(ResizePolicy::SpecificSize(Size::new(
                        SIZE_SMALL, SIZE_SMALL,
                    ))),
            )
            .unwrap();
        let resize_large = live_graph
            .add_node(
                Node::new(NodeType::Mix(MixType::default()))
                    .resize_filter(ResizeFilter::Nearest)
                    .resize_policy(ResizePolicy::SpecificSize(Size::new(
                        SIZE_LARGE, SIZE_LARGE,
                    ))),
            )
            .unwrap();
        live_graph
            .node(resize_large)
            .unwrap()
            .priority
            .set_priority(large_priority);

        live_graph
            .connect(value_node, resize_small_1, SlotId(0), SlotId(0))
            .unwrap();
        live_graph
            .connect(value_node, resize_large, SlotId(0), SlotId(0))
            .unwrap();
        live_graph
            .connect(value_node, resize_small_2, SlotId(0), SlotId(0))
            .unwrap();

        live_graph.auto_update = true;
        (resize_small_1, resize_small_2, resize_large)
    };

    // let mut large_done_first = false;
    let live_graph = LiveGraph::await_clean_read(&live_graph, resize_large).unwrap();

    if live_graph.node_state(resize_small_1).unwrap() == NodeState::Clean
        && live_graph.node_state(resize_small_2).unwrap() == NodeState::Clean
    {
        false
    } else {
        true
    }
}

#[test]
#[timeout(20_000)]
fn mix_node_single_input() {
    let tex_pro = tex_pro_new();
    let live_graph = tex_pro.new_live_graph().unwrap();

    let output_node = {
        let mut live_graph = live_graph.write().unwrap();

        let value_node = live_graph
            .add_node(Node::new(NodeType::Image(IMAGE_2.into())))
            .unwrap();
        let mix_node = live_graph
            .add_node(Node::new(NodeType::Mix(MixType::Add)))
            .unwrap();
        let output_node = live_graph
            .add_node(Node::new(NodeType::OutputGray("out".into())))
            .unwrap();

        live_graph
            .connect(value_node, mix_node, SlotId(0), SlotId(0))
            .unwrap();
        live_graph
            .connect(mix_node, output_node, SlotId(0), SlotId(0))
            .unwrap();
        output_node
    };

    save_and_compare(&live_graph, output_node, "mix_node_single_input.png");
}

#[test]
#[timeout(20_000)]
fn mix_node_single_input_2() {
    let tex_pro = tex_pro_new();
    let live_graph = tex_pro.new_live_graph().unwrap();

    let output_node = {
        let mut live_graph = live_graph.write().unwrap();
        let value_node = live_graph
            .add_node(Node::new(NodeType::Image(IMAGE_2.into())))
            .unwrap();
        let mix_node = live_graph
            .add_node(Node::new(NodeType::Mix(MixType::Subtract)))
            .unwrap();
        let output_node = live_graph
            .add_node(Node::new(NodeType::OutputGray("out".into())))
            .unwrap();

        live_graph
            .connect(value_node, mix_node, SlotId(0), SlotId(1))
            .unwrap();
        live_graph
            .connect(mix_node, output_node, SlotId(0), SlotId(0))
            .unwrap();
        output_node
    };

    save_and_compare(&live_graph, output_node, "mix_node_single_input_2.png");
}

#[test]
#[timeout(20_000)]
fn unconnected() {
    let tex_pro = tex_pro_new();
    let live_graph = tex_pro.new_live_graph().unwrap();
    let mut live_graph = live_graph.write().unwrap();

    live_graph
        .add_node(Node::new(NodeType::OutputRgba("out".into())))
        .unwrap();
}

#[test]
#[timeout(20_000)]
fn embedded_node_data() {
    let tex_pro = tex_pro_new();
    let live_graph_embed = tex_pro.new_live_graph().unwrap();

    let output_node_embed = {
        let mut live_graph = live_graph_embed.write().unwrap();
        let input_node = live_graph
            .add_node(Node::new(NodeType::Image(IMAGE_1.into())))
            .unwrap();
        let output_node = live_graph
            .add_node(Node::new(NodeType::OutputRgba("out".into())))
            .unwrap();

        live_graph
            .connect(input_node, output_node, SlotId(0), SlotId(0))
            .unwrap();
        output_node
    };

    let slot_data = Arc::clone(
        LiveGraph::await_clean_read(&live_graph_embed, output_node_embed)
            .unwrap()
            .slot_data(output_node_embed, SlotId(0))
            .unwrap(),
    );

    // Second graph
    let live_graph_out = tex_pro.new_live_graph().unwrap();

    let output_node_out = {
        let mut live_graph = live_graph_out.write().unwrap();
        let output_node = live_graph
            .add_node(Node::new(NodeType::OutputRgba("out".into())))
            .unwrap();

        let esd_id = live_graph
            .embed_slot_data_with_id(slot_data, EmbeddedSlotDataId(0))
            .unwrap();
        let input = live_graph
            .add_node(Node::new(NodeType::Embed(esd_id)))
            .unwrap();
        live_graph
            .connect(input, output_node, SlotId(0), SlotId(0))
            .unwrap();
        output_node
    };

    save_and_compare(&live_graph_out, output_node_out, "embedded_node_data.png");
}

#[test]
#[timeout(20_000)]
fn separate_node() {
    let tex_pro = tex_pro_new();
    let live_graph = tex_pro.new_live_graph().unwrap();

    let output_node = {
        let mut live_graph = live_graph.write().unwrap();
        let input_1 = live_graph
            .add_node(Node::new(NodeType::Image(IMAGE_1.into())))
            .unwrap();
        let separate_1 = live_graph
            .add_node(Node::new(NodeType::SeparateRgba))
            .unwrap();
        let input_2 = live_graph
            .add_node(Node::new(NodeType::Image(IMAGE_2.into())))
            .unwrap();
        let separate_2 = live_graph
            .add_node(Node::new(NodeType::SeparateRgba))
            .unwrap();
        let output_node = live_graph
            .add_node(Node::new(NodeType::OutputRgba("out".into())))
            .unwrap();
        let combine = live_graph
            .add_node(Node::new(NodeType::CombineRgba))
            .unwrap();

        live_graph
            .connect(input_1, separate_1, SlotId(0), SlotId(0))
            .unwrap();
        live_graph
            .connect(input_2, separate_2, SlotId(0), SlotId(0))
            .unwrap();

        live_graph
            .connect(separate_1, combine, SlotId(3), SlotId(0))
            .unwrap();
        live_graph
            .connect(separate_1, combine, SlotId(1), SlotId(1))
            .unwrap();
        live_graph
            .connect(separate_2, combine, SlotId(2), SlotId(2))
            .unwrap();
        live_graph
            .connect(separate_2, combine, SlotId(3), SlotId(3))
            .unwrap();

        live_graph
            .connect(combine, output_node, SlotId(0), SlotId(0))
            .unwrap();

        output_node
    };

    save_and_compare(&live_graph, output_node, "mix_images.png");
}

#[test]
#[timeout(20_000)]
fn irregular_sizes() {
    let tex_pro = tex_pro_new();
    let live_graph = tex_pro.new_live_graph().unwrap();

    let output_node = {
        let mut live_graph = live_graph.write().unwrap();
        let input_1 = live_graph
            .add_node(Node::new(NodeType::Image(HEART_128.into())))
            .unwrap();
        let input_2 = live_graph
            .add_node(Node::new(NodeType::Image(HEART_110.into())))
            .unwrap();
        let mix = live_graph
            .add_node(Node::new(NodeType::Mix(MixType::default())))
            .unwrap();
        let output_node = live_graph
            .add_node(Node::new(NodeType::OutputRgba("out".into())))
            .unwrap();

        live_graph
            .connect(input_1, mix, SlotId(0), SlotId(0))
            .unwrap();
        live_graph
            .connect(input_2, mix, SlotId(0), SlotId(1))
            .unwrap();
        live_graph
            .connect(mix, output_node, SlotId(0), SlotId(0))
            .unwrap();

        output_node
    };

    // Can not use the save_and_compare convenience function because this is slightly different.
    const PATH_OUT: &str = &"out/irregular_sizes.png";
    const PATH_CMP: &str = &"data/test_compare/irregular_sizes.png";

    let size = LiveGraph::await_clean_read(&live_graph, output_node)
        .unwrap()
        .slot_data_size(output_node, SlotId(0))
        .unwrap();

    ensure_out_dir();
    image::save_buffer(
        &Path::new(PATH_OUT),
        &image::RgbaImage::from_vec(
            size.width,
            size.height,
            LiveGraph::await_clean_read(&live_graph, output_node)
                .unwrap()
                .buffer_rgba(output_node, SlotId(0))
                .unwrap(),
        )
        .unwrap(),
        size.width,
        size.height,
        image::ColorType::Rgba8,
    )
    .unwrap();

    assert!(images_equal(PATH_OUT, PATH_CMP));
}

#[test]
#[timeout(20_000)]
fn unconnected_node() {
    let tex_pro = tex_pro_new();
    let live_graph = tex_pro.new_live_graph().unwrap();

    let output_node = {
        let mut live_graph = live_graph.write().unwrap();
        let input_1 = live_graph
            .add_node(Node::new(NodeType::Value(0.0)))
            .unwrap();
        live_graph
            .add_node(Node::new(NodeType::Value(0.0)))
            .unwrap();
        let output_node = live_graph
            .add_node(Node::new(NodeType::OutputGray("out".into())))
            .unwrap();

        live_graph
            .connect(input_1, output_node, SlotId(0), SlotId(0))
            .unwrap();
        live_graph.auto_update = true;
        output_node
    };

    thread::sleep(Duration::from_millis(1000));
    LiveGraph::await_clean_read(&live_graph, output_node)
        .unwrap()
        .buffer_rgba(output_node, SlotId(0))
        .unwrap();
}

#[test]
#[timeout(20_000)]
fn remove_node() {
    let tex_pro = tex_pro_new();
    let live_graph = tex_pro.new_live_graph().unwrap();

    {
        let mut live_graph = live_graph.write().unwrap();
        let value_node = live_graph.add_node(Node::new(NodeType::Value(0.))).unwrap();
        live_graph.remove_node(value_node).unwrap();
    }

    assert_eq!(live_graph.read().unwrap().node_ids().len(), 0);
}

#[test]
fn connect_invalid_slot() {
    let tex_pro = tex_pro_new();
    let live_graph = tex_pro.new_live_graph().unwrap();

    {
        let mut live_graph = live_graph.write().unwrap();
        let value_node = live_graph.add_node(Node::new(NodeType::Value(0.))).unwrap();

        let output_node = live_graph
            .add_node(Node::new(NodeType::Mix(MixType::default())))
            .unwrap();

        assert!(live_graph
            .connect(value_node, output_node, SlotId(0), SlotId(0))
            .is_ok());
        assert!(live_graph
            .connect(value_node, output_node, SlotId(0), SlotId(1))
            .is_ok());
        assert!(live_graph
            .connect(value_node, output_node, SlotId(0), SlotId(2))
            .is_err());
    }
}

#[test]
#[timeout(20_000)]
fn value_node() {
    let tex_pro = tex_pro_new();
    let live_graph = tex_pro.new_live_graph().unwrap();

    let combine_node = {
        let mut live_graph = live_graph.write().unwrap();
        let red_node = live_graph.add_node(Node::new(NodeType::Value(0.))).unwrap();
        let green_node = live_graph
            .add_node(Node::new(NodeType::Value(0.33)))
            .unwrap();
        let blue_node = live_graph
            .add_node(Node::new(NodeType::Value(0.66)))
            .unwrap();
        let alpha_node = live_graph.add_node(Node::new(NodeType::Value(1.))).unwrap();

        let combine_node = {
            let mut node = Node::new(NodeType::CombineRgba);
            node.resize_policy = ResizePolicy::SpecificSize(Size::new(256, 256));
            live_graph.add_node(node).unwrap()
        };

        let node_ids = [red_node, green_node, blue_node, alpha_node];
        for i in 0..4 {
            live_graph
                .connect(node_ids[i], combine_node, SlotId(0), SlotId(i as u32))
                .unwrap();
        }

        combine_node
    };

    save_and_compare(&live_graph, combine_node, "value_node.png");
}

fn resize_policy_test(
    resize_policy: ResizePolicy,
    img_path_1: &str,
    img_path_2: &str,
    expected_size: (u32, u32),
) {
    let tex_pro = tex_pro_new();
    let live_graph = tex_pro.new_live_graph().unwrap();

    let mix_node = {
        let mut live_graph = live_graph.write().unwrap();
        let image_node_1 = live_graph
            .add_node(Node::new(NodeType::Image(img_path_1.into())))
            .unwrap();
        let image_node_2 = live_graph
            .add_node(Node::new(NodeType::Image(img_path_2.into())))
            .unwrap();

        let mix_node = {
            let mut mix_node = Node::new(NodeType::Mix(MixType::default()));
            mix_node.resize_policy = resize_policy;
            live_graph.add_node(mix_node).unwrap()
        };

        live_graph
            .connect(image_node_1, mix_node, SlotId(0), SlotId(0))
            .unwrap();
        live_graph
            .connect(image_node_2, mix_node, SlotId(0), SlotId(1))
            .unwrap();

        mix_node
    };

    let actual_size = LiveGraph::await_clean_read(&live_graph, mix_node)
        .unwrap()
        .slot_data_size(mix_node, SlotId(0))
        .unwrap();
    let expected_size = Size::new(expected_size.0, expected_size.1);
    assert_eq!(
        actual_size, expected_size,
        "Actual size: {:?}, Expected size: {:?}",
        actual_size, expected_size
    );
}

#[test]
#[timeout(20_000)]
fn resize_policy_least_pixels() {
    resize_policy_test(ResizePolicy::LeastPixels, HEART_128, HEART_256, (128, 128));
}

#[test]
#[timeout(20_000)]
fn resize_policy_largest_axes() {
    resize_policy_test(
        ResizePolicy::LargestAxes,
        HEART_WIDE,
        HEART_TALL,
        (128, 128),
    );
}

#[test]
#[timeout(20_000)]
fn resize_policy_smallest_axes() {
    resize_policy_test(ResizePolicy::SmallestAxes, HEART_WIDE, HEART_TALL, (64, 64));
}

#[test]
#[timeout(20_000)]
fn resize_policy_most_pixels() {
    resize_policy_test(ResizePolicy::MostPixels, HEART_128, HEART_256, (256, 256));
}

#[test]
#[timeout(20_000)]
fn resize_policy_specific_size() {
    resize_policy_test(
        ResizePolicy::SpecificSize(Size::new(256, 256)),
        HEART_128,
        HEART_WIDE,
        (256, 256),
    );
}

#[test]
#[timeout(20_000)]
fn resize_policy_specific_slot() {
    resize_policy_test(
        ResizePolicy::SpecificSlot(SlotId(1)),
        HEART_128,
        HEART_WIDE,
        (128, 64),
    );
    resize_policy_test(
        ResizePolicy::SpecificSlot(SlotId(2)),
        HEART_128,
        HEART_WIDE,
        (128, 128),
    );
}

fn save_and_compare(live_graph: &Arc<RwLock<LiveGraph>>, node_id: NodeId, name: &str) {
    save_and_compare_size(live_graph, node_id, (256, 256), name);
}

fn save_and_compare_size(
    live_graph: &Arc<RwLock<LiveGraph>>,
    node_id: NodeId,
    size: (u32, u32),
    name: &str,
) {
    let (path_out, path_cmp) = build_paths(name);

    ensure_out_dir();
    let vec = LiveGraph::await_clean_read(live_graph, node_id)
        .unwrap()
        .buffer_rgba(node_id, SlotId(0))
        .unwrap();
    let vec_len = vec.len();
    let buf = &image::RgbaImage::from_vec(size.0, size.1, vec).expect(&format!(
        "Buffer was not big enough, \
        expected image size: {:?}, \
        number of pixels: {}, \
        Sqrt(number of pixels) = {}",
        size,
        vec_len,
        (vec_len as f32).sqrt()
    ));

    image::save_buffer(&path_out, buf, size.0, size.1, image::ColorType::Rgba8).unwrap();

    assert!(images_equal(path_out, path_cmp));
}

fn build_paths(name: &str) -> (String, String) {
    (
        format!("{}/{}", DIR_OUT, name),
        format!("{}/{}", DIR_CMP, name),
    )
}

#[test]
#[timeout(20_000)]
fn invert_graph_node() {
    // Nested invert graph
    let invert_graph = {
        let mut invert_graph = NodeGraph::new();

        let white_node_nested = invert_graph
            .add_node(Node::new(NodeType::Value(1.)))
            .unwrap();
        let nested_input_node = invert_graph
            .add_node(Node::new(NodeType::InputGray("in".into())))
            .unwrap();
        let subtract_node = invert_graph
            .add_node(Node::new(NodeType::Mix(MixType::Subtract)))
            .unwrap();
        let nested_output_node = invert_graph
            .add_node(Node::new(NodeType::OutputGray("out".into())))
            .unwrap();

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
    };

    let graph_node_input_slot_id = invert_graph.input_slot_id_with_name("in").unwrap();
    let graph_node_output_slot_id = invert_graph.output_slot_id_with_name("out").unwrap();

    // Main graph
    let tex_pro = tex_pro_new();
    let live_graph = tex_pro.new_live_graph().unwrap();

    let output_node = {
        let mut live_graph = live_graph.write().unwrap();
        let image_node = live_graph
            .add_node(Node::new(NodeType::Image(IMAGE_2.into())))
            .unwrap();
        let invert_graph_node = live_graph
            .add_node(Node::new(NodeType::Graph(invert_graph)))
            .unwrap();
        let separate_node = live_graph
            .add_node(Node::new(NodeType::SeparateRgba))
            .unwrap();
        let output_node = live_graph
            .add_node(Node::new(NodeType::OutputGray("out".into())))
            .unwrap();

        live_graph
            .connect(image_node, separate_node, SlotId(0), SlotId(0))
            .unwrap();
        live_graph
            .connect(
                separate_node,
                invert_graph_node,
                SlotId(0),
                graph_node_input_slot_id,
            )
            .unwrap();

        live_graph
            .connect(
                invert_graph_node,
                output_node,
                graph_node_output_slot_id,
                SlotId(0),
            )
            .unwrap();

        output_node
    };

    save_and_compare(&live_graph, output_node, "invert_graph_node.png");
}

#[test]
#[timeout(20_000)]
fn invert_graph_node_export() {
    // Nested invert graph
    let mut invert_graph = NodeGraph::new();

    let white_node_nested = invert_graph
        .add_node(Node::new(NodeType::Value(1.)))
        .unwrap();
    let nested_input_node = invert_graph
        .add_node(Node::new(NodeType::InputGray("in".into())))
        .unwrap();
    let subtract_node = invert_graph
        .add_node(Node::new(NodeType::Mix(MixType::Subtract)))
        .unwrap();
    let nested_output_node = invert_graph
        .add_node(Node::new(NodeType::OutputGray("out".into())))
        .unwrap();

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
#[timeout(20_000)]
fn invert_graph_node_import() {
    // Nested invert graph
    let invert_graph = NodeGraph::from_path("data/invert_graph.json".into()).unwrap();

    let graph_node_input_slot_id = invert_graph.input_slot_id_with_name("in").unwrap();
    let graph_node_output_slot_id = invert_graph.output_slot_id_with_name("out").unwrap();

    // Main graph
    let tex_pro = tex_pro_new();
    let live_graph = tex_pro.new_live_graph().unwrap();

    let output_node = {
        let mut live_graph = live_graph.write().unwrap();
        let image_node = live_graph
            .add_node(Node::new(NodeType::Image(IMAGE_2.into())))
            .unwrap();
        let separate_node = live_graph
            .add_node(Node::new(NodeType::SeparateRgba))
            .unwrap();
        let invert_graph_node = live_graph
            .add_node(Node::new(NodeType::Graph(invert_graph)))
            .unwrap();
        let output_node = live_graph
            .add_node(Node::new(NodeType::OutputGray("out".into())))
            .unwrap();

        live_graph
            .connect(image_node, separate_node, SlotId(0), SlotId(0))
            .unwrap();
        live_graph
            .connect(
                separate_node,
                invert_graph_node,
                SlotId(0),
                graph_node_input_slot_id,
            )
            .unwrap();
        live_graph
            .connect(
                invert_graph_node,
                output_node,
                graph_node_output_slot_id,
                SlotId(0),
            )
            .unwrap();

        output_node
    };

    save_and_compare(&live_graph, output_node, "invert_graph_node_import.png");
}

#[test]
#[timeout(1000)]
fn temp() {
    let tex_pro = tex_pro_new();

    let mut live_graph = LiveGraph::new(Arc::clone(&tex_pro.add_buffer_queue));
    live_graph.auto_update = true;
    live_graph.use_cache = true;
    let live_graph = Arc::new(RwLock::new(live_graph));

    tex_pro
        .push_live_graph(Arc::clone(&live_graph))
        .expect("Unable to add graph");

    let combine_node = {
        let mut live_graph = live_graph.write().unwrap();

        let value_node = live_graph
            .add_node(Node::new(NodeType::Value(0.5)))
            .unwrap();
        let combine_node = live_graph
            .add_node(Node::new(NodeType::CombineRgba))
            .unwrap();
        let separate_node = live_graph
            .add_node(Node::new(NodeType::SeparateRgba))
            .unwrap();

        live_graph
            .connect(combine_node, separate_node, SlotId(0), SlotId(0))
            .unwrap();
        thread::sleep(Duration::from_millis(100));
        live_graph
            .connect(value_node, combine_node, SlotId(0), SlotId(0))
            .unwrap();
        thread::sleep(Duration::from_millis(100));

        combine_node
    };

    LiveGraph::await_clean_read(&live_graph, combine_node)
        .unwrap()
        .slot_data_size(combine_node, SlotId(0))
        .unwrap();
}

#[test]
#[timeout(20_000)]
fn graph_node_rgba() {
    let nested_graph = {
        let mut nested_graph = NodeGraph::new();

        let nested_input_node = nested_graph
            .add_node(Node::new(NodeType::InputRgba("in".into())))
            .unwrap();
        let nested_output_node = nested_graph
            .add_node(Node::new(NodeType::OutputRgba("out".into())))
            .unwrap();

        nested_graph
            .connect(nested_input_node, nested_output_node, SlotId(0), SlotId(0))
            .unwrap();
        nested_graph
    };

    let graph_node_input_slot_id = nested_graph.input_slot_id_with_name("in").unwrap();
    let graph_node_output_slot_id = nested_graph.output_slot_id_with_name("out").unwrap();

    // Texture Processor
    let tex_pro = tex_pro_new();
    let live_graph = tex_pro.new_live_graph().unwrap();

    let output_node = {
        let mut live_graph = live_graph.write().unwrap();
        let input_node = live_graph
            .add_node(Node::new(NodeType::Image(IMAGE_2.into())))
            .unwrap();
        let graph_node = live_graph
            .add_node(Node::new(NodeType::Graph(nested_graph)))
            .unwrap();
        let output_node = live_graph
            .add_node(Node::new(NodeType::OutputRgba("out".into())))
            .unwrap();

        live_graph
            .connect(input_node, graph_node, SlotId(0), graph_node_input_slot_id)
            .unwrap();

        live_graph
            .connect(
                graph_node,
                output_node,
                graph_node_output_slot_id,
                SlotId(0),
            )
            .unwrap();
        output_node
    };

    save_and_compare(&live_graph, output_node, "graph_node_rgba.png");
}

/// Grayscale passthrough node.
#[test]
#[timeout(20_000)]
fn graph_node_gray() {
    let nested_graph = {
        let mut nested_graph = NodeGraph::new();

        let nested_input_node = nested_graph
            .add_node(Node::new(NodeType::InputGray("in".into())))
            .unwrap();
        let nested_output_node = nested_graph
            .add_node(Node::new(NodeType::OutputGray("out".into())))
            .unwrap();

        nested_graph
            .connect(nested_input_node, nested_output_node, SlotId(0), SlotId(0))
            .unwrap();
        nested_graph
    };

    let graph_node_input_slot_id = nested_graph.input_slot_id_with_name("in").unwrap();
    let graph_node_output_slot_id = nested_graph.output_slot_id_with_name("out").unwrap();

    let tex_pro = tex_pro_new();
    let live_graph = tex_pro.new_live_graph().unwrap();

    let output_node = {
        let mut live_graph = live_graph.write().unwrap();
        let input_node = live_graph
            .add_node(Node::new(NodeType::Image(IMAGE_2.into())))
            .unwrap();
        let separate_node = live_graph
            .add_node(Node::new(NodeType::SeparateRgba))
            .unwrap();
        let graph_node = live_graph
            .add_node(Node::new(NodeType::Graph(nested_graph)))
            .unwrap();
        let output_node = live_graph
            .add_node(Node::new(NodeType::OutputGray("out".into())))
            .unwrap();

        live_graph
            .connect(input_node, separate_node, SlotId(0), SlotId(0))
            .unwrap();
        live_graph
            .connect(
                separate_node,
                graph_node,
                SlotId(0),
                graph_node_input_slot_id,
            )
            .unwrap();

        live_graph
            .connect(
                graph_node,
                output_node,
                graph_node_output_slot_id,
                SlotId(0),
            )
            .unwrap();
        output_node
    };

    save_and_compare(&live_graph, output_node, "graph_node_gray.png");
}

#[test]
#[should_panic]
#[timeout(20_000)]
fn wrong_slot_type() {
    let tex_pro = tex_pro_new();
    let live_graph = tex_pro.new_live_graph().unwrap();

    let mut live_graph = live_graph.write().unwrap();
    let image_node = live_graph
        .add_node(Node::new(NodeType::Image(IMAGE_1.into())))
        .unwrap();
    let gray_node = live_graph
        .add_node(Node::new(NodeType::OutputGray("out".into())))
        .unwrap();
    live_graph
        .connect(image_node, gray_node, SlotId(0), SlotId(0))
        .unwrap();
}

#[test]
#[timeout(20_000)]
fn height_to_normal_node() {
    let tex_pro = tex_pro_new();
    let live_graph = tex_pro.new_live_graph().unwrap();

    let output_node = {
        let mut live_graph = live_graph.write().unwrap();
        let input_node = live_graph
            .add_node(Node::new(NodeType::Image(CLOUDS.into())))
            .unwrap();
        let separate_node = live_graph
            .add_node(Node::new(NodeType::SeparateRgba))
            .unwrap();
        let h2n_node = live_graph
            .add_node(Node::new(NodeType::HeightToNormal))
            .unwrap();
        let output_node = live_graph
            .add_node(Node::new(NodeType::OutputRgba("out".into())))
            .unwrap();

        live_graph
            .connect(input_node, separate_node, SlotId(0), SlotId(0))
            .unwrap();
        live_graph
            .connect(separate_node, h2n_node, SlotId(0), SlotId(0))
            .unwrap();
        live_graph
            .connect(h2n_node, output_node, SlotId(0), SlotId(0))
            .unwrap();

        output_node
    };

    save_and_compare(&live_graph, output_node, "height_to_normal_node.png");
}

#[test]
#[timeout(20_000)]
fn read_dirty_read() {
    const VALUE: f32 = 0.5;

    let tex_pro = tex_pro_new();
    let live_graph = tex_pro.new_live_graph().unwrap();

    let (val_node, combine_node) = {
        let mut live_graph = live_graph.write().unwrap();
        live_graph.use_cache = true;

        let val_node = live_graph
            .add_node(Node::new(NodeType::Value(VALUE)))
            .unwrap();
        let combine_node = live_graph
            .add_node(Node::new(NodeType::CombineRgba))
            .unwrap();

        live_graph
            .connect(val_node, combine_node, SlotId(0), SlotId(0))
            .unwrap();

        (val_node, combine_node)
    };

    fn verify_pixel(live_graph: &Arc<RwLock<LiveGraph>>, node_id: NodeId, identifier: String) {
        let pixels = {
            let live_graph = LiveGraph::await_clean_read(&live_graph, node_id).unwrap();
            let slot_data = live_graph.slot_data(node_id, SlotId(0)).unwrap();
            let pixels = slot_data.image.to_u8().unwrap();
            pixels
        };

        assert!(pixels == [127, 0, 0, 255,], "{}", identifier);
    }

    verify_pixel(&live_graph, combine_node, "Before dirty".into());

    {
        // Dirty
        let mut live_graph = live_graph.write().unwrap();
        live_graph
            .disconnect_slot(val_node, Side::Output, SlotId(0))
            .unwrap();
        live_graph
            .connect(val_node, combine_node, SlotId(0), SlotId(0))
            .unwrap();
    }

    verify_pixel(&live_graph, combine_node, "After dirty".into());
}

fn mix_node_test_gray(mix_type: MixType, name: &str) {
    let tex_pro = tex_pro_new();
    let live_graph = tex_pro.new_live_graph().unwrap();

    let output_node = {
        let mut live_graph = live_graph.write().unwrap();
        let image_node = live_graph
            .add_node(Node::new(NodeType::Image(IMAGE_2.into())))
            .unwrap();
        let separate_node = live_graph
            .add_node(Node::new(NodeType::SeparateRgba))
            .unwrap();
        let input_node = live_graph
            .add_node(Node::new(NodeType::Mix(mix_type)))
            .unwrap();
        let output_node = live_graph
            .add_node(Node::new(NodeType::OutputGray("out".into())))
            .unwrap();

        live_graph
            .connect(image_node, separate_node, SlotId(0), SlotId(0))
            .unwrap();
        live_graph
            .connect(separate_node, input_node, SlotId(0), SlotId(0))
            .unwrap();
        live_graph
            .connect(separate_node, input_node, SlotId(1), SlotId(1))
            .unwrap();

        live_graph
            .connect(input_node, output_node, SlotId(0), SlotId(0))
            .unwrap();
        output_node
    };

    save_and_compare(&live_graph, output_node, name);
}

fn mix_node_test_rgba(mix_type: MixType, name: &str) {
    let tex_pro = tex_pro_new();
    let live_graph = tex_pro.new_live_graph().unwrap();

    let output_node = {
        let mut live_graph = live_graph.write().unwrap();
        let image_node_1 = live_graph
            .add_node(Node::new(NodeType::Image(IMAGE_1.into())))
            .unwrap();
        let image_node_2 = live_graph
            .add_node(Node::new(NodeType::Image(IMAGE_2.into())))
            .unwrap();
        let multiply_node = live_graph
            .add_node(Node::new(NodeType::Mix(mix_type)))
            .unwrap();
        let output_node = live_graph
            .add_node(Node::new(NodeType::OutputRgba("out".into())))
            .unwrap();

        live_graph
            .connect(image_node_1, multiply_node, SlotId(0), SlotId(0))
            .unwrap();
        live_graph
            .connect(image_node_2, multiply_node, SlotId(0), SlotId(1))
            .unwrap();

        live_graph
            .connect(multiply_node, output_node, SlotId(0), SlotId(0))
            .unwrap();
        output_node
    };

    save_and_compare(&live_graph, output_node, name);
}

#[test]
#[timeout(20_000)]
fn add_node_gray() {
    mix_node_test_gray(MixType::Add, "add_node_gray.png");
}
#[test]
#[timeout(20_000)]
fn add_node_rgba() {
    mix_node_test_rgba(MixType::Add, "add_node_rgba.png");
}

#[test]
#[timeout(20_000)]
fn subtract_node_gray() {
    mix_node_test_gray(MixType::Subtract, "subtract_node_gray.png");
}
#[test]
#[timeout(20_000)]
fn subtract_node_rgba() {
    mix_node_test_rgba(MixType::Subtract, "subtract_node_rgba.png");
}

#[test]
#[timeout(20_000)]
fn multiply_node_gray() {
    mix_node_test_gray(MixType::Multiply, "multiply_node_gray.png");
}

#[test]
#[timeout(20_000)]
fn multiply_node_rgba() {
    mix_node_test_rgba(MixType::Multiply, "multiply_node_rgba.png");
}

#[test]
#[timeout(20_000)]
fn divide_node_gray() {
    mix_node_test_gray(MixType::Divide, "divide_node_gray.png");
}

#[test]
#[timeout(20_000)]
fn divide_node_rgba() {
    mix_node_test_rgba(MixType::Divide, "divide_node_rgba.png");
}

#[test]
#[timeout(20_000)]
fn pow_node_gray() {
    mix_node_test_gray(MixType::Pow, "pow_node_gray.png");
}

#[test]
#[timeout(20_000)]
fn pow_node_rgba() {
    mix_node_test_rgba(MixType::Pow, "pow_node_rgba.png");
}
