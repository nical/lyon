/*

use std::vec;

struct PortDescriptor {
    data_type: DataTypeID,
}

struct NodeDescriptor {
    inputs: ~[PortDescriptor],
    outputs: ~[PortDescriptor],
}

struct Node {
    inputs: Vec<Connection>,
    outputs: Vec<Connection>,
    node_type: NodeTypeID,
}

struct Connection {
    port: u16,
    other_node: u16,
    other_port: u16,
}

struct TypeSystem {
    descriptors: Vec<NodeDescriptor>,
}

struct Graph {
    nodes: Vec<Option<Node>>
}

type PortIndex = u16;
type PortID = u16; // TODO

struct NodeID { handle: u16 }
struct NodeTypeID { handle: i32 }
struct DataTypeID { handle: u32 }

impl Graph {
    pub fn new() -> Graph {
        Graph {
            nodes: Vec::new(),
        }
    }

    pub fn connect(&mut self, n1: NodeID, p1: PortIndex, n2: NodeID, p2: PortIndex) -> bool {
        if !self.can_connect(n1, p1, n2, p2) {
            return false;
        }
        if self.are_connected(n1, p1, n2, p2) {
            return true;
        }
        {
            let node1: &mut Node;
            match self.nodes.get_mut(n1.handle as uint) {
                &Some(ref mut n) => { node1 = n }
                &None => { return false; }
            }
            node1.outputs.push(Connection {
                port: p1,
                other_node: n2.handle,
                other_port: p2
            });
            node1.outputs.sort_by(|a,b|{a.port.cmp(&b.port)});
        }
        {
            let node2: &mut Node;
            match self.nodes.get_mut(n2.handle as uint) {
                &Some(ref mut n) => { node2 = n }
                &None => { return false; }
            }
            node2.inputs.push(Connection {
                port: p2,
                other_node: n1.handle,
                other_port: p1,
            });
            node2.inputs.sort_by(|a,b|{a.port.cmp(&b.port)});
        }
        return true;
    }

    pub fn are_connected(&self, n1: NodeID, p1: PortIndex, n2: NodeID, p2: PortIndex) -> bool {
        let node1: &Node;
        match self.nodes.get(n1.handle as uint) {
            &Some(ref n) => { node1 = n }
            &None => { return false; }
        }
        let node2: &Node;
        match self.nodes.get(n2.handle as uint) {
            &Some(ref n) => { node2 = n }
            &None => { return false; }
        }

        let mut connected1 = false;
        for p in node1.outputs.iter() {
            if p.port == p1 && p.other_node == n2.handle && p.other_port == p2 {
                connected1 = true;
            }
        }

        let mut connected2 = false;
        for p in node2.outputs.iter() {
            if p.port == p2 && p.other_node == n1.handle && p.other_port == p1 {
                connected2 = true;
            }
        }

        assert_eq!(connected1, connected2);

        return connected1;
    }

    pub fn can_connect(&self, n1: NodeID, p1: PortIndex, n2: NodeID, p2: PortIndex) -> bool {
        true
    }

    pub fn disconnect_input(&mut self, n: NodeID, p: PortID) {
        let inputs = match self.nodes.get_mut(n.handle as uint) {
            &Some(ref mut n) => { &mut n.inputs }
            &None => { return; }
        };
        // look for the connections in n's inputs
        let mut i = 0;
        loop {
            if inputs.get(i).port == p {
                let outputs = &mut self.nodes.get_mut(inputs.get(i).other_node as uint).unwrap().outputs;
                // look for the corresponding connection in the othe node's outputs
                let mut j = 0;
                loop {
                    if outputs.get(j).other_node == n.handle
                        && outputs.get(j).other_port == p {
                        outputs.remove(j);
                        break;
                    }
                    j += 1;
                }
                inputs.remove(i);
            } else {
                i += 1;
            }
        }
    }

    pub fn disconnect_output(&mut self, n: NodeID, p: PortID) {
    }

    pub fn add(&mut self, id: NodeTypeID) -> NodeID {
        let mut i = 0;
        for ref n in self.nodes.iter() {
            match **n {
                None => { break; }
                _ => {}
            }
            i += 1;
        }
        if i == self.nodes.len() {
            self.nodes.push(Some(Node {
                inputs: Vec::new(),
                outputs: Vec::new(),
                node_type: id,
            }));
        } else {
            *self.nodes.get_mut(i) = Some(Node {
                inputs: Vec::new(),
                outputs: Vec::new(),
                node_type: id,
            });
        }
        return NodeID { handle: i as u16 };
    }

    pub fn remove(&mut self, id: NodeID) {

    }
}

impl TypeSystem {
    pub fn new() -> TypeSystem {
        TypeSystem {
            descriptors: Vec::new(),
        }
    }

    pub fn add(&mut self, desc: NodeDescriptor) -> NodeTypeID {
        NodeTypeID { handle: 0 }
    }

    pub fn get<'l>(&'l self, type_id: NodeTypeID) -> &'l NodeDescriptor {
        return self.descriptors.get(type_id.handle as uint);
    }
}

mod tests {
    use super::{Node, Graph, NodeDescriptor, NodeID, TypeSystem, Connection, PortDescriptor};
    #[test]
    fn simple_graph() {
        let mut types = TypeSystem::new();
        let mut g = Graph::new();

        let t1 = types.add(NodeDescriptor{
            inputs: ~[
                PortDescriptor { data_type: 0 },
                PortDescriptor { data_type: 0 }
            ],
            outputs: ~[
                PortDescriptor { data_type: 0 }
            ],
        });

        let n1 = g.add(t1);
        let n2 = g.add(t1);
        let n3 = g.add(t1);

        assert!(!g.are_connected(n1, 0, n2, 0));
        assert!(!g.are_connected(n1, 0, n3, 0));

        assert!(g.connect(n1, 0, n2, 0));
        assert!(g.connect(n1, 1, n3, 0));

        assert!(g.are_connected(n1, 0, n2, 0));
        assert!(g.are_connected(n1, 0, n3, 0));

        g.disconnect_input(n2, 0);
        g.disconnect_input(n3, 0);

        assert!(!g.are_connected(n1, 0, n2, 0));
        assert!(!g.are_connected(n1, 0, n3, 0));
    }
}

*/