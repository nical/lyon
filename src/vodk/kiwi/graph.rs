use std::rc::Rc;
use std::default::Default;
use std::slice;

type DataTypeList = Vec<DataTypeID>;

#[derive(Copy, PartialEq, Debug)]
enum DataType {
    Generic(u32),
    Type(DataTypeID),
}

#[derive(Copy, Debug)]
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
    generics: Vec<Option<DataTypeID>>,
    node_type: NodeTypeID,
    valid: bool,
}

#[derive(Copy, Debug)]
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

#[derive(Copy, PartialEq, Clone, Debug)]
struct NodeID { handle: u16 }

#[derive(Copy, PartialEq, Clone, Debug)]
struct NodeTypeID { handle: i32 }

#[derive(Copy, PartialEq, Clone, Debug)]
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
        if self.are_connected(n1, p1, n2, p2) {
            return true;
        }
        let c_type = self.type_system.can_connect_instances(
            self.get_node(n1), p1,
            self.get_node(n2), p2
        );
        if c_type.is_none() {
            return false;
        }
        let c_type = c_type.unwrap();
        {
            let mut node1 = &mut self.nodes[n1.handle as usize];
            node1.outputs.push(Connection {
                port: p1,
                other_node: n2.handle,
                other_port: p2
            });
            node1.outputs.sort_by(|a,b|{a.port.cmp(&b.port)});
            match self.type_system.get(node1.node_type).outputs[p1 as usize].data_type {
                DataType::Generic(g) => { node1.generics[g as usize] = Some(c_type); }
                _ => {}
            }
        }
        {
            let mut node2 = &mut self.nodes[n2.handle as usize];
            node2.inputs.push(Connection {
                port: p2,
                other_node: n1.handle,
                other_port: p1,
            });
            node2.inputs.sort_by(|a,b|{a.port.cmp(&b.port)});
            match self.type_system.get(node2.node_type).inputs[p2 as usize].data_type {
                DataType::Generic(g) => { node2.generics[g as usize] = Some(c_type); }
                _ => {}
            }
        }
        assert!(self.are_connected(n1, p1, n2, p2));
        return true;
    }

    pub fn are_connected(&self, n1: NodeID, p1: PortIndex, n2: NodeID, p2: PortIndex) -> bool {
        if self.nodes.len() <= n1.handle as usize
            || self.nodes.len() <= n2.handle as usize {
            return false;
        }

        let node1 = &self.nodes[n1.handle as usize];
        let node2 = &self.nodes[n2.handle as usize];

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

    fn get_node<'l>(&'l self, id: NodeID) -> &'l Node {
        return &self.nodes[id.handle as usize];
    }

    pub fn can_connect(&self, n1: NodeID, p1: PortIndex, n2: NodeID, p2: PortIndex) -> bool {
        if !self.contains(n1) || !self.contains(n2) {
            return false;
        }
        return self.type_system.can_connect_instances(
            self.get_node(n1), p1,
            self.get_node(n2), p2
        ).is_some();
    }

    pub fn disconnect_input(&mut self, n: NodeID, p: PortID) {
        let n_handle = n.handle as usize;
        if !self.nodes[n_handle].valid {
            return;
        }
        // look for the connections in n's inputs
        let mut i = 0;
        loop {
            if i >= self.nodes[n_handle].inputs.len() {
                break;
            }
            let inputs_i = self.nodes[n_handle].inputs[i];
            if inputs_i.port == p {
                let input_node = inputs_i.other_node as usize;
                {
                    let outputs = &mut self.nodes[input_node].outputs;
                    // look for the corresponding connection in the othe node's outputs
                    let mut j = 0;
                    loop {
                        if outputs[j].other_node == n.handle
                            && outputs[j].other_port == p {
                            outputs.remove(j);
                            break;
                        }
                        j += 1;
                    }
                }
                self.nodes[n_handle].inputs.remove(i);
            } else {
                i += 1;
            }
        }
    }

    pub fn disconnect_output(&mut self, n: NodeID, p: PortID) {
        let n_handle = n.handle as usize;
        if !self.nodes[n_handle].valid {
            return;
        }
        // look for the connections in n's outputs
        let mut i = 0;
        loop {
            if i >= self.nodes[n_handle].outputs.len() {
                break;
            }
            let outputs_i = self.nodes[n_handle].outputs[i];
            if outputs_i.port == p {
                let output_node = outputs_i.other_node as usize;
                {
                    let inputs = &mut self.nodes[output_node].inputs;
                    // look for the corresponding connection in the othe node's outputs
                    let mut j = 0;
                    loop {
                        if inputs[j].other_node == n.handle
                            && inputs[j].other_port == p {
                            inputs.remove(j);
                            break;
                        }
                        j += 1;
                    }
                }
                self.nodes[n_handle].outputs.remove(i);
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
            self.nodes[i] = Node::new(id);
        }
        return NodeID { handle: i as u16 };
    }

    pub fn remove(&mut self, id: NodeID) {
        if !self.contains(id) {
            return;
        }
        loop {
            if self.nodes[id.handle as usize].inputs.len() > 0 {
                self.disconnect_input(id, 0);
            } else {
                break;
            }
        }
        loop {
            if self.nodes[id.handle as usize].outputs.len() > 0 {
                self.disconnect_output(id, 0);
            } else {
                break;
            }
        }
        self.nodes[id.handle as usize].valid = false;
    }

    pub fn contains(&self, id: NodeID) -> bool {
        if self.nodes.len() <= id.handle as usize {
            return false;
        }
        return self.nodes[id.handle as usize].valid;
    }
}

#[allow(dead_code)]
impl Node {
    fn new(t: NodeTypeID) -> Node {
        Node {
            inputs: Vec::new(),
            outputs: Vec::new(),
            generics: Vec::new(),
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
        if !desc.is_valid() { panic!("Invalid node descriptor"); }
        self.descriptors.push(desc);
        return NodeTypeID { handle: self.descriptors.len() as i32 - 1 };
    }

    pub fn get<'l>(&'l self, type_id: NodeTypeID) -> &'l NodeDescriptor {
        return &self.descriptors[type_id.handle as usize];
    }

    pub fn can_connect_types(
        &self,
        o_node: NodeTypeID, o_port: PortIndex,
        i_node: NodeTypeID, i_port: PortIndex
    ) -> bool {
        let o_types = self.get(o_node).get_output_types(o_port);
        let i_types = self.get(i_node).get_input_types(i_port);
        return intersect_types(i_types, o_types).len() > 0;
    }

    pub fn can_connect_instances(
        &self,
        o_node: &Node, o_port: PortIndex,
        i_node: &Node, i_port: PortIndex
    ) -> Option<DataTypeID> {
        let o_desc = self.get(o_node.node_type);
        let i_desc = self.get(i_node.node_type);

        let o_types = self.get_output_types(o_node, o_desc, o_port);
        let i_types = self.get_input_types(i_node, i_desc, i_port);

        let common = intersect_types(o_types, i_types);
        return if common.len() == 1 { Some(common[0]) }
               else { None };
    }

    fn get_input_types<'l>(
        &self, node: &'l Node,
        desc: &'l NodeDescriptor,
        port: PortIndex
    ) -> &'l [DataTypeID] {
        if desc.inputs.len() <= port as usize {
            return &[];
        }
        match desc.inputs[port as usize].data_type {
            DataType::Type(ref t) => { return slice::ref_slice(t); }
            DataType::Generic(g) => {
                match node.generics[g as usize] {
                    Some(ref t) => { return slice::ref_slice(t); }
                    None => {
                        return &desc.generics[g as usize][];
                    }
                }
            }
        }
    }

    fn get_output_types<'l>(
        &self, node: &'l Node,
        desc: &'l NodeDescriptor,
        port: PortIndex
    ) -> &'l [DataTypeID] {
        if desc.outputs.len() <= port as usize {
            return &[];
        }
        match desc.outputs[port as usize].data_type {
            DataType::Type(ref t) => { return slice::ref_slice(t); }
            DataType::Generic(g) => {
                match node.generics[g as usize] {
                    Some(ref t) => { return slice::ref_slice(t); }
                    None => {
                        return &desc.generics[g as usize][];
                    }
                }
            }
        }
    }
}

#[allow(dead_code)]
impl NodeDescriptor {

    fn get_input_types<'l>(&'l self, port: PortIndex) -> &'l [DataTypeID] {
        if port as usize >= self.inputs.len() {
            return &[];
        }
        match self.inputs[port as usize].data_type {
            DataType::Type(ref t) => { return slice::ref_slice(t); },
            DataType::Generic(g) => { return &self.generics[g as usize][]; }
        }
    }

    fn get_output_types<'l>(&'l self, port: PortIndex) -> &'l [DataTypeID] {
        if port as usize >= self.outputs.len() {
            return &[];
        }
        match self.outputs[port as usize].data_type {
            DataType::Type(ref t) => { return slice::ref_slice(t); },
            DataType::Generic(g) => { return &self.generics[g as usize][]; }
        }
    }

    fn is_valid(&self) -> bool {
        for input in self.inputs.iter() {
            match input.data_type {
                DataType::Generic(g) => {
                    if g as usize >= self.generics.len() {
                        return false;
                    }
                }
                _ => {}
            }
        }
        for output in self.outputs.iter() {
            match output.data_type {
                DataType::Generic(g) => {
                    if g as usize >= self.generics.len() {
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
        while self.data.len() <= id.handle as usize {
            self.data.push(Default::default());
        }
        self.data[id.handle as usize] = val;
    }

    pub fn get<'l>(&'l self, id: NodeID) -> &'l T {
        return &self.data[id.handle as usize];
    }

    pub fn get_mut<'l> (&'l mut self, id: NodeID) -> &'l mut T {
        while self.data.len() <= id.handle as usize {
            self.data.push(Default::default());
        }
        return &mut self.data[id.handle as usize];
    }

    pub fn erase(&mut self, id: NodeID) {
        if self.data.len() <= id.handle as usize {
            return;
        }

        self.data[id.handle as usize] = Default::default();
    }

    pub fn len(&self) -> usize { self.data.len() }

    pub fn clear(&mut self) { self.data.clear(); }
}

#[cfg(test)]
mod tests {
    use super::{
        Graph, NodeDescriptor, DataTypeID,
        TypeSystem, PortDescriptor, DataType,
    };
    use std::rc::Rc;

    #[test]
    fn simple_graph() {
        let mut types = TypeSystem::new();

        let INT = DataTypeID { handle: 0};
        let FLOAT = DataTypeID { handle: 1};

        let t1 = types.add(NodeDescriptor {
            generics: Vec::new(),
            inputs: vec![
                PortDescriptor { data_type: DataType::Type(INT) },
                PortDescriptor { data_type: DataType::Type(INT) },
            ],
            outputs: vec![
                PortDescriptor { data_type: DataType::Type(INT) },
                PortDescriptor { data_type: DataType::Type(FLOAT) },
            ],
        });

        let t2 = types.add(NodeDescriptor {
            generics: vec![vec![INT, FLOAT]],
            inputs: vec![
                PortDescriptor { data_type: DataType::Generic(0) },
                PortDescriptor { data_type: DataType::Generic(0) },
            ],
            outputs: vec![
                PortDescriptor { data_type: DataType::Generic(0) },
            ],
        });

        assert!(types.can_connect_types(t1, 0, t1, 0));
        assert!(!types.can_connect_types(t1, 1, t1, 0));
        assert!(types.can_connect_types(t2, 0, t2, 0));
        assert!(types.can_connect_types(t1, 0, t2, 0));
        assert!(types.can_connect_types(t1, 1, t2, 0));

        let mut g = Graph::new(Rc::new(types));

        let n1 = g.add(t1);
        let n2 = g.add(t1);
        let n3 = g.add(t1);
        let __ = g.add(t2);
        let __ = g.add(t2);

        assert!(!g.are_connected(n1, 0, n2, 0));
        assert!(!g.are_connected(n1, 0, n3, 0));

        assert!(g.connect(n1, 0, n2, 0));
        assert!(g.connect(n1, 0, n3, 0));

        assert!(g.are_connected(n1, 0, n2, 0));
        assert!(g.are_connected(n1, 0, n3, 0));

        g.disconnect_input(n2, 0);
        g.disconnect_input(n3, 0);

        assert!(!g.are_connected(n1, 0, n2, 0));
        assert!(!g.are_connected(n1, 1, n3, 0));

        assert!(g.connect(n1, 0, n2, 0));
        assert!(g.connect(n2, 0, n3, 1));

        assert!(g.are_connected(n1, 0, n2, 0));
        assert!(g.are_connected(n2, 0, n3, 1));

        assert!(!g.are_connected(n1, 0, n3, 0));
        // not connected, should do nothing
        g.disconnect_output(n1, 0);
        g.disconnect_output(n2, 0);

        assert!(!g.are_connected(n1, 0, n2, 0));
        assert!(!g.are_connected(n2, 0, n3, 1));

        assert!(!g.are_connected(n1, 0, n2, 0));
        assert!(!g.are_connected(n2, 1, n3, 1));
    }
}

fn intersect_types(a: &[DataTypeID], b:&[DataTypeID]) -> Vec<DataTypeID> {
    let mut result: Vec<DataTypeID> = Vec::new();
    for i in a.iter() {
        for j in b.iter() {
            if *i == *j {
                result.push(*i);
            }
        }
    }
    return result;
}