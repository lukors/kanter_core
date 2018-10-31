use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

#[derive(Debug)]
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

    // pub fn process_singlethread(&mut self) {
    //     let reversed_edges = self.reversed_edges();

    //     // TODO: Take out the root ids as part of the topological sort.
    //     // let mut sorted_ids = self.topological_sort();
    //     let queued_ids = self.topological_sort();

    //     for id in queued_ids {
    //         let parent_ids = reversed_edges.get(&id).unwrap();

    //         let new_data: NodeData = {
    //             let mut input_data: Vec<&NodeData> = Vec::new();
    //             for id in parent_ids {
    //                 input_data.push(self.node_data.get(&id).unwrap());
    //             }
    //             self.nodes.get_mut(&id).unwrap().process(&input_data).unwrap()
    //         };

    //         self.node_data.insert(id, Arc::new(new_data));
    //     }
    // }

    pub fn process(&mut self) {
        #[derive(Debug)]
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
                println!("Inserting processed node data: {:?}", message);
                finished_nodes.insert(message.node_id);
                self.node_data.insert(message.node_id, Arc::new(message.node_data));

                for child_id in self.edges.get(&message.node_id).unwrap() {
                    if !started_nodes.contains(child_id) {
                        println!("pusing onto quque: {:?}", child_id);
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

            let input_data: Vec<Arc<NodeData>> = parent_ids
                .iter()
                .map(|id| Arc::clone(self.node_data.get(id).unwrap()))
                .collect();
            let current_node = Arc::clone(self.nodes.get(&current_id).unwrap());
            let send = send.clone();

            thread::spawn(move || {
                let node_data = current_node.process(&input_data).unwrap();
                send.send(ThreadMessage{
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

    fn topological_sort(&self) -> Vec<NodeId> {
        let mut sorted_list = Vec::with_capacity(self.nodes.len());

        let mut all_ids: Vec<NodeId> = self.nodes.keys().map(|key| *key).collect();
        let mut mark_permanent: HashSet<NodeId> = HashSet::with_capacity(self.nodes.len());

        while let Some(id) = all_ids.pop() {
            if mark_permanent.contains(&id) {
                continue;
            }
            let mut mark_temporary: HashSet<NodeId> = HashSet::with_capacity(self.nodes.len());
            sorted_list.append(&mut self.visit(id, &mut mark_temporary, &mut mark_permanent));
        }

        sorted_list.reverse();
        sorted_list
    }

    fn visit(
        &self,
        id: NodeId,
        mark_temporary: &mut HashSet<NodeId>,
        mark_permanent: &mut HashSet<NodeId>,
    ) -> Vec<NodeId> {
        if mark_permanent.contains(&id) {
            return Vec::new();
        }
        if mark_temporary.contains(&id) {
            panic!("The graph has a cycle, so it's not a DAG")
        }
        mark_temporary.insert(id);
        let mut sorted_list = Vec::with_capacity(1);

        for input_id in self.get_input_edge_ids(id) {
            sorted_list.append(&mut self.visit(*input_id, mark_temporary, mark_permanent));
        }
        sorted_list.push(id);

        mark_permanent.insert(id);

        sorted_list
    }

    pub fn get_leaf_ids(&self) -> Vec<NodeId> {
        self.edges
            .iter()
            .filter(|(_, edges)| edges.is_empty())
            .map(|(key, _)| *key)
            .collect()

        // self.nodes
        //     .iter()
        //     .filter(|(_, node)| node.edges.is_empty())
        //     .map(|(key, _)| *key)
        //     .collect()
    }

    fn get_input_edge_ids(&self, id: NodeId) -> &Vec<NodeId> {
        self.edges
            .get(&id)
            .expect("Could not find the given `NodeId` key in the `edges` HashMap.")
    }

    fn get_root_ids(&self) -> Vec<NodeId> {
        // let mut root_ids: Vec<NodeId> = Vec::new();

        

        // root_ids

        self.reversed_edges().iter().filter(|(_, v)| v.is_empty()).map(|(k, _)| *k).collect()
    }
}

#[derive(Debug)]
pub enum NodeType {
    Input(f64),
    Add,
    Multiply,
}

#[derive(Debug)]
struct Node {
    node_type: NodeType,
}

#[derive(Debug)]
struct NodeData {
    value: f64,
}

impl Node {
    pub fn new(node_type: NodeType) -> Self {
        Node { node_type }
    }

    pub fn process(&self, input: &[Arc<NodeData>]) -> Option<NodeData> {
        thread::sleep(Duration::from_secs(1));
        Some(NodeData {
            value: match self.node_type {
                NodeType::Input(x) => x,
                NodeType::Add => input[0].value + input[1].value,
                NodeType::Multiply => input[0].value * input[1].value,
            },
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
struct NodeId(u32);

fn main() {
    let mut dag: Dag = Dag::new();

    let node_0 = dag.add_node(Node::new(NodeType::Input(5.)));
    let node_1 = dag.add_node(Node::new(NodeType::Input(2.)));
    let node_2 = dag.add_node(Node::new(NodeType::Input(3.)));
    let node_3 = dag.add_node(Node::new(NodeType::Input(4.)));
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

    // println!("leaf_ids: {:?}", dag.get_leaf_ids());
    // println!("root_ids: {:?}", dag.get_root_ids(&dag.get_leaf_ids()));

    dag.process();

    println!("{:?}", dag.get_output(node_0));
    println!("{:?}", dag.get_output(node_1));
    println!("{:?}", dag.get_output(node_2));
    println!("{:?}", dag.get_output(node_3));
    println!("{:?}", dag.get_output(node_4));
    println!("{:?}", dag.get_output(node_5));
    println!("{:?}", dag.get_output(node_6));

    // TODO:
    // Try using a DynamicImage
    // Turn into lib
    // Clean up the code, add error handling and so on
    // Add same features as ChannelShuffle 2 has
}
