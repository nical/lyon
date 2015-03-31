use std::rc::Rc;
use std::default::Default;
use std::slice;
use std::ops;

use vodk_id::*;
use vodk_id::id_vector::IdVector;

type DataTypeList = Vec<DataTypeId>;

#[derive(Copy, PartialEq, Debug)]
enum DataType {
    Generic(u32),
    Type(DataTypeId),
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
    generics: Vec<Option<DataTypeId>>,
    node_type: NodeTypeId,
    valid: bool,
}

#[derive(Copy, Debug)]
struct Connection {
    port: u16,
    other_node: NodeId,
    other_port: u16,
}

struct TypeSystem {
    descriptors: IdVector<NodeTypeId, NodeDescriptor>,
}

struct Graph {
    nodes: IdVector<NodeId, Node>,
    type_system: Rc<TypeSystem>,
}

type PortIndex = u16;
type PortID = u16; // TODO

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Node_;
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct NodeType_;
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct DataType_;

pub type NodeHandle = u16;
pub type NodeTypeHandle = u16;
pub type DataTypeHandle = u32;

pub type NodeId = Id<Node_, NodeHandle>;
pub type NodeTypeId = Id<NodeType_, NodeTypeHandle>;
pub type DataTypeId = Id<DataType_, DataTypeHandle>;

pub fn node_id(index: NodeHandle) -> NodeId { NodeId::new(index) }
pub fn node_type_id(index: NodeTypeHandle) -> NodeTypeId { NodeTypeId::new(index) }
pub fn data_type_id(index: DataTypeHandle) -> DataTypeId { DataTypeId::new(index) }

impl Graph {
    pub fn new(type_system: Rc<TypeSystem>) -> Graph {
        Graph {
            nodes: IdVector::new(),
            type_system: type_system,
        }
    }

    pub fn connect(&mut self, n1: NodeId, p1: PortIndex, n2: NodeId, p2: PortIndex) -> bool {
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
            let mut node1 = &mut self.nodes[n1];
            node1.outputs.push(Connection {
                port: p1,
                other_node: n2,
                other_port: p2
            });
            node1.outputs.sort_by(|a,b|{a.port.cmp(&b.port)});
            match self.type_system.get(node1.node_type).outputs[p1 as usize].data_type {
                DataType::Generic(g) => { node1.generics[g as usize] = Some(c_type); }
                _ => {}
            }
        }
        {
            let mut node2 = &mut self.nodes[n2];
            node2.inputs.push(Connection {
                port: p2,
                other_node: n1,
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

    pub fn are_connected(&self, n1: NodeId, p1: PortIndex, n2: NodeId, p2: PortIndex) -> bool {
        if self.nodes.len() <= n1.handle as usize || self.nodes.len() <= n2.handle as usize {
            return false;
        }

        let node1 = &self.nodes[n1];
        let node2 = &self.nodes[n2];

        let mut connected1 = false;
        for p in node1.outputs.iter() {
            if p.port == p1 && p.other_node == n2 && p.other_port == p2 {
                connected1 = true;
            }
        }

        let mut connected2 = false;
        for p in node2.inputs.iter() {
            if p.port == p2 && p.other_node == n1 && p.other_port == p1 {
                connected2 = true;
            }
        }

        assert_eq!(connected1, connected2);

        return connected1;
    }

    fn get_node<'l>(&'l self, id: NodeId) -> &'l Node {
        return &self.nodes[id];
    }

    pub fn can_connect(&self, n1: NodeId, p1: PortIndex, n2: NodeId, p2: PortIndex) -> bool {
        if !self.contains(n1) || !self.contains(n2) {
            return false;
        }
        return self.type_system.can_connect_instances(
            self.get_node(n1), p1,
            self.get_node(n2), p2
        ).is_some();
    }

    pub fn disconnect_input(&mut self, n: NodeId, p: PortID) {
        if !self.nodes[n].valid {
            return;
        }
        // look for the connections in n's inputs
        let mut i = 0;
        loop {
            if i >= self.nodes[n].inputs.len() {
                break;
            }
            let inputs_i = self.nodes[n].inputs[i];
            if inputs_i.port == p {
                let input_node = inputs_i.other_node;
                {
                    let outputs = &mut self.nodes[input_node].outputs;
                    // look for the corresponding connection in the othe node's outputs
                    let mut j = 0;
                    loop {
                        if outputs[j].other_node == n && outputs[j].other_port == p {
                            outputs.remove(j);
                            break;
                        }
                        j += 1;
                    }
                }
                self.nodes[n].inputs.remove(i);
            } else {
                i += 1;
            }
        }
    }

    pub fn disconnect_output(&mut self, n: NodeId, p: PortID) {
        if !self.nodes[n].valid {
            return;
        }
        // look for the connections in n's outputs
        let mut i = 0;
        loop {
            if i >= self.nodes[n].outputs.len() {
                break;
            }
            let outputs_i = self.nodes[n].outputs[i];
            if outputs_i.port == p {
                let output_node = outputs_i.other_node;
                {
                    let inputs = &mut self.nodes[output_node].inputs;
                    // look for the corresponding connection in the othe node's outputs
                    let mut j = 0;
                    loop {
                        if inputs[j].other_node == n
                            && inputs[j].other_port == p {
                            inputs.remove(j);
                            break;
                        }
                        j += 1;
                    }
                }
                self.nodes[n].outputs.remove(i);
            } else {
                i += 1;
            }
        }
    }

    pub fn add(&mut self, id: NodeTypeId) -> NodeId {
        let mut i = 0;
        for ref n in self.nodes.iter() {
            if !n.valid { break; }
            i += 1;
        }
        return if i == self.nodes.len() {
            self.nodes.push(Node::new(id))
        } else {
            *self.nodes.at_index_mut(i) = Node::new(id);
            node_id(i as u16)
        };
    }

    pub fn remove(&mut self, id: NodeId) {
        if !self.contains(id) {
            return;
        }
        loop {
            if self.nodes[id].inputs.len() > 0 {
                self.disconnect_input(id, 0);
            } else {
                break;
            }
        }
        loop {
            if self.nodes[id].outputs.len() > 0 {
                self.disconnect_output(id, 0);
            } else {
                break;
            }
        }
        self.nodes[id].valid = false;
    }

    pub fn contains(&self, id: NodeId) -> bool {
        if self.nodes.len() <= id.handle as usize {
            return false;
        }
        return self.nodes[id].valid;
    }
}

//impl ops::Index<NodeId> for Graph {
//    type Output = Node;
//    fn index<'l>(&'l self, id: &NodeId) -> &'l Node { &self.nodes[*id] }
//}
//
//impl ops::IndexMut<NodeId> for Graph {
//    fn index_mut<'l>(&'l mut self, id: &NodeId) -> &'l mut Node { &mut self.nodes[*id] }
//}


impl Node {
    fn new(t: NodeTypeId) -> Node {
        Node {
            inputs: Vec::new(),
            outputs: Vec::new(),
            generics: Vec::new(),
            node_type: t,
            valid: true,
        }
    }
}

impl TypeSystem {
    pub fn new() -> TypeSystem {
        TypeSystem {
            descriptors: IdVector::new(),
        }
    }

    pub fn add(&mut self, desc: NodeDescriptor) -> NodeTypeId {
        if !desc.is_valid() { panic!("Invalid node descriptor"); }
        return self.descriptors.push(desc);
    }

    pub fn get<'l>(&'l self, type_id: NodeTypeId) -> &'l NodeDescriptor {
        return &self.descriptors[type_id];
    }

    pub fn can_connect_types(
        &self,
        o_node: NodeTypeId, o_port: PortIndex,
        i_node: NodeTypeId, i_port: PortIndex
    ) -> bool {
        let o_types = self[o_node].get_output_types(o_port);
        let i_types = self[i_node].get_input_types(i_port);
        return intersect_types(i_types, o_types).len() > 0;
    }

    pub fn can_connect_instances(
        &self,
        o_node: &Node, o_port: PortIndex,
        i_node: &Node, i_port: PortIndex
    ) -> Option<DataTypeId> {
        let o_desc = &self[o_node.node_type];
        let i_desc = &self[i_node.node_type];

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
    ) -> &'l [DataTypeId] {
        if desc.inputs.len() <= port as usize {
            return &[];
        }
        match desc.inputs[port as usize].data_type {
            DataType::Type(ref t) => { return slice::ref_slice(t); }
            DataType::Generic(g) => {
                match node.generics[g as usize] {
                    Some(ref t) => { return slice::ref_slice(t); }
                    None => {
                        return &desc.generics[g as usize][..];
                    }
                }
            }
        }
    }

    fn get_output_types<'l>(
        &self, node: &'l Node,
        desc: &'l NodeDescriptor,
        port: PortIndex
    ) -> &'l [DataTypeId] {
        if desc.outputs.len() <= port as usize {
            return &[];
        }
        match desc.outputs[port as usize].data_type {
            DataType::Type(ref t) => { return slice::ref_slice(t); }
            DataType::Generic(g) => {
                match node.generics[g as usize] {
                    Some(ref t) => { return slice::ref_slice(t); }
                    None => {
                        return &desc.generics[g as usize][..];
                    }
                }
            }
        }
    }
}

impl ops::Index<NodeTypeId> for TypeSystem {
    type Output = NodeDescriptor;
    fn index<'l>(&'l self, id: &NodeTypeId) -> &'l NodeDescriptor { &self.descriptors[*id] }
}

impl ops::IndexMut<NodeTypeId> for TypeSystem {
    fn index_mut<'l>(&'l mut self, id: &NodeTypeId) -> &'l mut NodeDescriptor { &mut self.descriptors[*id] }
}

impl NodeDescriptor {

    fn get_input_types<'l>(&'l self, port: PortIndex) -> &'l [DataTypeId] {
        if port as usize >= self.inputs.len() {
            return &[];
        }
        match self.inputs[port as usize].data_type {
            DataType::Type(ref t) => { return slice::ref_slice(t); },
            DataType::Generic(g) => { return &self.generics[g as usize][..]; }
        }
    }

    fn get_output_types<'l>(&'l self, port: PortIndex) -> &'l [DataTypeId] {
        if port as usize >= self.outputs.len() {
            return &[];
        }
        match self.outputs[port as usize].data_type {
            DataType::Type(ref t) => { return slice::ref_slice(t); },
            DataType::Generic(g) => { return &self.generics[g as usize][..]; }
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

#[cfg(test)]
mod tests {
    use super::{
        Graph, NodeDescriptor,
        TypeSystem, PortDescriptor, DataType, data_type_id,
    };
    use std::rc::Rc;

    #[test]
    fn simple_graph() {
        let mut types = TypeSystem::new();

        let INT = data_type_id(0);
        let FLOAT = data_type_id(1);

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

fn intersect_types(a: &[DataTypeId], b:&[DataTypeId]) -> Vec<DataTypeId> {
    let mut result: Vec<DataTypeId> = Vec::new();
    for i in a.iter() {
        for j in b.iter() {
            if *i == *j {
                result.push(*i);
            }
        }
    }
    return result;
}