// TODO:
// Clean up the code, add error handling and so on
// Break inputs down into channels and process only channels
// Use only the simplest nodes possible that operate only on for instance two channels
// Panic when input/output rules are not followed
// Implement read and write nodes
// Implement tests
// Implement GUI

extern crate image;

use image::{ImageBuffer, RgbaImage};

use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::Path,
    sync::{mpsc, Arc},
    thread,
};

struct Dag {
    nodes: HashMap<NodeId, Arc<Node>>,
    node_data: HashMap<NodeId, Arc<NodeData>>,
    edges: HashMap<NodeId, Vec<NodeId>>,
    id_iterator: u32,
}

impl Dag {
    pub fn new() -> Self {
        Dag {
            nodes: HashMap::new(),
            node_data: HashMap::new(),
            edges: HashMap::new(),
            id_iterator: 0,
        }
    }

    pub fn add_node(&mut self, node: Node) -> NodeId {
        let id = self.new_id();
        self.nodes.insert(id, Arc::new(node));
        self.edges.insert(id, Vec::new());
        id
    }

    pub fn connect(&mut self, id_1: NodeId, id_2: NodeId) {
        if !self.nodes.contains_key(&id_1) || !self.nodes.contains_key(&id_2) {
            return;
        }

        self.edges
            .get_mut(&id_1)
            .map(|connections| connections.push(id_2));
    }

    fn reversed_edges(&self) -> HashMap<NodeId, Vec<NodeId>> {
        let mut reversed_edges: HashMap<NodeId, Vec<NodeId>> =
            HashMap::with_capacity(self.edges.len());

        for key in self.edges.keys() {
            reversed_edges.insert(*key, Vec::new());
        }

        for (id, target_ids) in self.edges.iter() {
            for target_id in target_ids {
                reversed_edges.entry(*target_id).and_modify(|e| e.push(*id));
            }
        }
        reversed_edges
    }

    pub fn process(&mut self) {
        struct ThreadMessage {
            node_id: NodeId,
            node_data: NodeData,
        }

        let reversed_edges: HashMap<NodeId, Vec<NodeId>> = self.reversed_edges();

        let (send, recv) = mpsc::channel::<ThreadMessage>();
        let mut finished_nodes: HashSet<NodeId> = HashSet::with_capacity(self.nodes.len());
        let mut started_nodes: HashSet<NodeId> = HashSet::with_capacity(self.nodes.len());

        let mut queued_ids: VecDeque<NodeId> = VecDeque::from(self.get_root_ids());
        for item in &queued_ids {
            started_nodes.insert(*item);
        }

        'outer: while finished_nodes.len() < self.nodes.len() {
            for message in recv.try_iter() {
                println!("Finished node: {:?}", message.node_id);
                finished_nodes.insert(message.node_id);
                self.node_data
                    .insert(message.node_id, Arc::new(message.node_data));

                for child_id in self.edges.get(&message.node_id).unwrap() {
                    if !started_nodes.contains(child_id) {
                        queued_ids.push_back(*child_id);
                        started_nodes.insert(*child_id);
                    }
                }
            }

            let current_id = match queued_ids.pop_front() {
                Some(id) => id,
                None => continue,
            };

            let parent_ids = reversed_edges.get(&current_id).unwrap();
            for id in parent_ids {
                if !finished_nodes.contains(id) {
                    queued_ids.push_back(current_id);
                    continue 'outer;
                }
            }
            println!("Started node: {:?}", current_id);

            let input_data: Vec<Arc<NodeData>> = parent_ids
                .iter()
                .map(|id| Arc::clone(self.node_data.get(id).unwrap()))
                .collect();
            let current_node = Arc::clone(self.nodes.get(&current_id).unwrap());
            let send = send.clone();

            thread::spawn(move || {
                let node_data = current_node.process(&input_data).unwrap();
                send.send(ThreadMessage {
                    node_id: current_id,
                    node_data,
                }).unwrap();
            });
        }
    }

    pub fn get_output(&self, id: NodeId) -> &NodeData {
        &self.node_data.get(&id).unwrap()
    }

    fn new_id(&mut self) -> NodeId {
        let id = NodeId(self.id_iterator);
        self.id_iterator += 1;
        id
    }

    fn get_root_ids(&self) -> Vec<NodeId> {
        self.reversed_edges()
            .iter()
            .filter(|(_, v)| v.is_empty())
            .map(|(k, _)| *k)
            .collect()
    }
}

pub enum NodeType {
    Input,
    Add,
    Multiply,
}

struct Node {
    node_type: NodeType,
}

struct NodeData {
    value: RgbaImage,
}

impl Node {
    pub fn new(node_type: NodeType) -> Self {
        Node { node_type }
    }

    pub fn process(&self, input: &[Arc<NodeData>]) -> Option<NodeData> {
        Some(NodeData {
            value: match self.node_type {
                NodeType::Input(ref x) => x.clone(),
                NodeType::Add => Self::add(&input[0], &input[1]),
                NodeType::Multiply => Self::multiply(&input[0], &input[1]),
            },
        })
    }

    fn add(input_0: &NodeData, input_1: &NodeData) -> RgbaImage {
        ImageBuffer::from_fn(256, 256, |x, y| {
            let mut channels: [u8; 4] = [0, 0, 0, 255];

            for i in 0..4 {
                channels[i] = input_0.value.get_pixel(x, y)[i]
                    .saturating_add(input_1.value.get_pixel(x, y)[i]);
            }

            image::Rgba(channels)
        })
    }

    fn multiply(input_0: &NodeData, input_1: &NodeData) -> RgbaImage {
        ImageBuffer::from_fn(256, 256, |x, y| {
            let mut channels: [u8; 4] = [0, 0, 0, 255];

            for i in 0..4 {
                channels[i] = ((input_0.value.get_pixel(x, y)[i] as f64 / 255.
                    * input_1.value.get_pixel(x, y)[i] as f64
                    / 255.)
                    * 255.) as u8;
            }

            image::Rgba(channels)
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
struct NodeId(u32);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integration_test() {
        let mut dag: Dag = Dag::new();

        let image_0 = image::open(&Path::new(&"data/image_1.png"))
            .unwrap()
            .to_rgba();
        let image_1 = image::open(&Path::new(&"data/image_2.png"))
            .unwrap()
            .to_rgba();
        let image_2 = image::open(&Path::new(&"data/heart_256.png"))
            .unwrap()
            .to_rgba();
        let image_3 = image::open(&Path::new(&"data/heart_256.png"))
            .unwrap()
            .to_rgba();

        let node_0 = dag.add_node(Node::new(NodeType::Input(image_0)));
        let node_1 = dag.add_node(Node::new(NodeType::Input(image_1)));
        let node_2 = dag.add_node(Node::new(NodeType::Input(image_2)));
        let node_3 = dag.add_node(Node::new(NodeType::Input(image_3)));
        let node_4 = dag.add_node(Node::new(NodeType::Add));
        let node_5 = dag.add_node(Node::new(NodeType::Add));
        let node_6 = dag.add_node(Node::new(NodeType::Multiply));
        let node_7 = dag.add_node(Node::new(NodeType::Add));

        dag.connect(node_0, node_4);
        dag.connect(node_1, node_4);
        dag.connect(node_1, node_5);
        dag.connect(node_2, node_5);
        dag.connect(node_5, node_6);
        dag.connect(node_4, node_6);
        dag.connect(node_6, node_7);
        dag.connect(node_3, node_7);

        dag.process();

        image::save_buffer(
            &Path::new(&"out/node_0.png"),
            &dag.get_output(node_0).value,
            256,
            256,
            image::ColorType::RGBA(8),
        ).unwrap();
        image::save_buffer(
            &Path::new(&"out/node_1.png"),
            &dag.get_output(node_1).value,
            256,
            256,
            image::ColorType::RGBA(8),
        ).unwrap();
        image::save_buffer(
            &Path::new(&"out/node_2.png"),
            &dag.get_output(node_2).value,
            256,
            256,
            image::ColorType::RGBA(8),
        ).unwrap();
        image::save_buffer(
            &Path::new(&"out/node_3.png"),
            &dag.get_output(node_3).value,
            256,
            256,
            image::ColorType::RGBA(8),
        ).unwrap();
        image::save_buffer(
            &Path::new(&"out/node_4.png"),
            &dag.get_output(node_4).value,
            256,
            256,
            image::ColorType::RGBA(8),
        ).unwrap();
        image::save_buffer(
            &Path::new(&"out/node_5.png"),
            &dag.get_output(node_5).value,
            256,
            256,
            image::ColorType::RGBA(8),
        ).unwrap();
        image::save_buffer(
            &Path::new(&"out/node_6.png"),
            &dag.get_output(node_6).value,
            256,
            256,
            image::ColorType::RGBA(8),
        ).unwrap();
        image::save_buffer(
            &Path::new(&"out/node_7.png"),
            &dag.get_output(node_7).value,
            256,
            256,
            image::ColorType::RGBA(8),
        ).unwrap();
    }
}
