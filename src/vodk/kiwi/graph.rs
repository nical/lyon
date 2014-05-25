use std::vec;
use std::rc::Rc;
use std::default::Default;

type DataTypeList = Vec<DataTypeID>;

#[deriving(Eq, Show)]
enum DataType {
    Generic(u32),
    Type(DataTypeID),
}

#[deriving(Show)]
struct PortDescriptor {
    data_type: DataType,
}

struct NodeDescriptor {
    generics: Vec<DataTypeList>,
    inputs: Vec<PortDescriptor>,
    outputs: Vec<PortDescriptor>,
}

struct Node {
    inputs: Vec<Connection>,
    outputs: Vec<Connection>,
    node_type: NodeTypeID,
    valid: bool,
}

#[deriving(Show)]
struct Connection {
    port: u16,
    other_node: u16,
    other_port: u16,
}

struct TypeSystem {
    descriptors: Vec<NodeDescriptor>,
}

struct Graph {
    nodes: Vec<Node>,
    type_system: Rc<TypeSystem>,
}

type PortIndex = u16;
type PortID = u16; // TODO

#[deriving(Eq, Clone, Show)]
struct NodeID { handle: u16 }

#[deriving(Eq, Clone, Show)]
struct NodeTypeID { handle: i32 }

#[deriving(Eq, Clone, Show)]
struct DataTypeID { handle: u32 }

#[allow(dead_code)]
impl Graph {
    pub fn new(type_system: Rc<TypeSystem>) -> Graph {
        Graph {
            nodes: Vec::new(),
            type_system: type_system,
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
            let mut node1 = self.nodes.get_mut(n1.handle as uint);
            node1.outputs.push(Connection {
                port: p1,
                other_node: n2.handle,
                other_port: p2
            });
            node1.outputs.sort_by(|a,b|{a.port.cmp(&b.port)});
        }
        {
            let mut node2 = self.nodes.get_mut(n2.handle as uint);
            node2.inputs.push(Connection {
                port: p2,
                other_node: n1.handle,
                other_port: p1,
            });
            node2.inputs.sort_by(|a,b|{a.port.cmp(&b.port)});
        }
        assert!(self.are_connected(n1, p1, n2, p2));
        return true;
    }

    pub fn are_connected(&self, n1: NodeID, p1: PortIndex, n2: NodeID, p2: PortIndex) -> bool {
        if self.nodes.len() <= n1.handle as uint 
            || self.nodes.len() <= n2.handle as uint {
            return false;
        }

        let node1 = self.nodes.get(n1.handle as uint);
        let node2 = self.nodes.get(n2.handle as uint);

        let mut connected1 = false;
        for p in node1.outputs.iter() {
            if p.port == p1 && p.other_node == n2.handle && p.other_port == p2 {
                connected1 = true;
            }
        }

        let mut connected2 = false;
        for p in node2.inputs.iter() {
            if p.port == p2 && p.other_node == n1.handle && p.other_port == p1 {
                connected2 = true;
            }
        }

        assert_eq!(connected1, connected2);

        return connected1;
    }

    pub fn can_connect(&self, n1: NodeID, p1: PortIndex, n2: NodeID, p2: PortIndex) -> bool {
        if !self.contains(n1) || !self.contains(n2) {
            return false;
        }
        return self.type_system.can_connect(self, n1, p1, n2, p2);
    }

    pub fn disconnect_input(&mut self, n: NodeID, p: PortID) {
        let n_handle = n.handle as uint;
        if !self.nodes.get(n_handle).valid {
            return;
        }
        // look for the connections in n's inputs
        let mut i = 0;
        loop {
            if i >= self.nodes.get(n_handle).inputs.len() {
                break;
            }
            let inputs_i = *self.nodes.get(n_handle).inputs.get(i);
            if inputs_i.port == p {
                let input_node = inputs_i.other_node as uint;
                {
                    let outputs = &mut self.nodes.get_mut(input_node).outputs;
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
                }
                self.nodes.get_mut(n_handle).inputs.remove(i);
            } else {
                i += 1;
            }
        }
    }

    pub fn disconnect_output(&mut self, n: NodeID, p: PortID) {
        let n_handle = n.handle as uint;
        if !self.nodes.get(n_handle).valid {
            return;
        }
        // look for the connections in n's outputs
        let mut i = 0;
        loop {
            if i >= self.nodes.get(n_handle).outputs.len() {
                break;
            }
            let outputs_i = *self.nodes.get(n_handle).outputs.get(i);
            if outputs_i.port == p {
                let output_node = outputs_i.other_node as uint;
                {
                    let inputs = &mut self.nodes.get_mut(output_node).inputs;
                    // look for the corresponding connection in the othe node's outputs
                    let mut j = 0;
                    loop {
                        if inputs.get(j).other_node == n.handle
                            && inputs.get(j).other_port == p {
                            inputs.remove(j);
                            break;
                        }
                        j += 1;
                    }
                }
                self.nodes.get_mut(n_handle).outputs.remove(i);
            } else {
                i += 1;
            }
        }
    }

    pub fn add(&mut self, id: NodeTypeID) -> NodeID {
        let mut i = 0;
        for ref n in self.nodes.iter() {
            if !n.valid { break; }
            i += 1;
        }
        if i == self.nodes.len() {
            self.nodes.push(Node::new(id));
        } else {
            *self.nodes.get_mut(i) = Node::new(id);
        }
        return NodeID { handle: i as u16 };
    }

    pub fn remove(&mut self, id: NodeID) {
        if !self.contains(id) {
            return;
        }
        loop {
            if self.nodes.get(id.handle as uint).inputs.len() > 0 {
                self.disconnect_input(id, 0);
            } else {
                break;
            }
        }
        loop {
            if self.nodes.get(id.handle as uint).outputs.len() > 0 {
                self.disconnect_output(id, 0);
            } else {
                break;
            }
        }
        self.nodes.get_mut(id.handle as uint).valid = false;
    }

    pub fn contains(&self, id: NodeID) -> bool {
        if self.nodes.len() <= id.handle as uint {
            return false;
        }
        return self.nodes.get(id.handle as uint).valid;
    }
}

#[allow(dead_code)]
impl Node {
    fn new(t: NodeTypeID) -> Node {
        Node {
            inputs: Vec::new(),
            outputs: Vec::new(),
            node_type: t,
            valid: true,
        }
    }
}

#[allow(dead_code)]
impl TypeSystem {
    pub fn new() -> TypeSystem {
        TypeSystem {
            descriptors: Vec::new(),
        }
    }

    pub fn add(&mut self, desc: NodeDescriptor) -> NodeTypeID {
        if !desc.is_valid() { fail!("Invalid node descriptor"); }
        self.descriptors.push(desc);
        return NodeTypeID { handle: self.descriptors.len() as i32 - 1 };
    }

    pub fn get<'l>(&'l self, type_id: NodeTypeID) -> &'l NodeDescriptor {
        return self.descriptors.get(type_id.handle as uint);
    }

    pub fn can_connect(
        &self, graph: &Graph,
        n1: NodeID, p1: PortIndex,
        n2: NodeID, p2: PortIndex
    ) -> bool {
        let nt_1 = graph.nodes.get(n1.handle as uint).node_type;
        let nt_2 = graph.nodes.get(n2.handle as uint).node_type;
        let pt_1 = self.descriptors.get(nt_1.handle as uint).inputs.get(p1 as uint).data_type;
        let pt_2 = self.descriptors.get(nt_2.handle as uint).inputs.get(p2 as uint).data_type;
        return pt_1 == pt_2;
    }
}

#[allow(dead_code)]
impl NodeDescriptor {
    // TODO: return a slice
    fn get_input_types(&self, port: PortIndex) -> Vec<DataTypeID> {
        if port as uint >= self.inputs.len() {
            return Vec::new();
        }
        match self.inputs.get(port as uint).data_type {
            Type(t) => { return vec!(t); },
            Generic(g) => { return self.generics.get(g as uint).clone(); }
        }
    }

    // TODO: return a slice
    fn get_output_types(&self, port: PortIndex) -> Vec<DataTypeID> {
        if port as uint >= self.outputs.len() {
            return Vec::new();
        }
        match self.outputs.get(port as uint).data_type {
            Type(t) => { return vec!(t); },
            Generic(g) => { return self.generics.get(g as uint).clone(); }
        }
    }

    fn is_valid(&self) -> bool {
        for input in self.inputs.iter() {
            match input.data_type {
                Generic(g) => {
                    if g as uint <= self.generics.len() {
                        return false;
                    }
                }
                _ => {}
            }
        }
        for output in self.outputs.iter() {
            match output.data_type {
                Generic(g) => {
                    if g as uint <= self.generics.len() {
                        return false;
                    }
                }
                _ => {}
            }
        }
        return true;
    }
}

struct NodeAttributeVector<T> {
    data: Vec<T>
}

impl<T: Default> NodeAttributeVector<T> {

    pub fn new() -> NodeAttributeVector<T> {
        NodeAttributeVector {
            data: Vec::new()
        }
    }

    pub fn set(&mut self, id: NodeID, val: T) {
        while self.data.len() <= id.handle as uint {
            self.data.push(Default::default());
        }
        *self.data.get_mut(id.handle as uint) = val;
    }

    pub fn get<'l>(&'l self, id: NodeID) -> &'l T {
        return self.data.get(id.handle as uint);
    }

    pub fn get_mut<'l> (&'l mut self, id: NodeID) -> &'l mut T {
        while self.data.len() <= id.handle as uint {
            self.data.push(Default::default());
        }
        return self.data.get_mut(id.handle as uint);
    }

    pub fn erase(&mut self, id: NodeID) {
        if self.data.len() <= id.handle as uint {
            return;
        }

        *self.data.get_mut(id.handle as uint) = Default::default();
    }

    pub fn len(&self) -> uint { self.data.len() }

    pub fn clear(&mut self) { self.data.clear(); }
}

#[cfg(test)]
mod tests {
    use super::{
        Graph, NodeDescriptor, DataTypeID,
        TypeSystem, PortDescriptor, Type,
    };
    use std::rc::Rc;

    #[test]
    fn simple_graph() {
        let mut types = TypeSystem::new();

        let INT = DataTypeID{ handle: 0};
        //let FLOAT = DataTypeID{ handle: 1};

        let t1 = types.add(NodeDescriptor {
            generics: Vec::new(),
            inputs: vec!(
                PortDescriptor { data_type: Type(INT) },
                PortDescriptor { data_type: Type(INT) },
            ),
            outputs: vec!(
                PortDescriptor { data_type: Type(INT) },
            ),
        });

        let mut g = Graph::new(Rc::new(types));

        let n1 = g.add(t1);
        let n2 = g.add(t1);
        let n3 = g.add(t1);

        assert!(!g.are_connected(n1, 0, n2, 0));
        assert!(!g.are_connected(n1, 0, n3, 0));

        assert!(g.connect(n1, 0, n2, 0));
        assert!(g.connect(n1, 1, n3, 0));

        assert!(g.are_connected(n1, 0, n2, 0));
        assert!(g.are_connected(n1, 1, n3, 0));

        g.disconnect_input(n2, 0);
        g.disconnect_input(n3, 0);

        assert!(!g.are_connected(n1, 0, n2, 0));
        assert!(!g.are_connected(n1, 1, n3, 0));

        assert!(g.connect(n1, 0, n2, 0));
        assert!(g.connect(n2, 0, n3, 1));

        assert!(g.are_connected(n1, 0, n2, 0));
        assert!(g.are_connected(n2, 0, n3, 1));

        assert!(!g.are_connected(n1, 0, n3, 0));
        // not connected, shoud do nothing
        g.disconnect_output(n1, 0);
        g.disconnect_output(n2, 0);

        assert!(!g.are_connected(n1, 0, n2, 0));
        assert!(!g.are_connected(n2, 0, n3, 1));

        assert!(!g.are_connected(n1, 0, n2, 0));
        assert!(!g.are_connected(n2, 1, n3, 1));
    }
}
