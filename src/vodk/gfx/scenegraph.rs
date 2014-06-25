use std::comm;
use math::units::world;
use libc::funcs::c95::stdlib::{malloc, free};
use std::mem;
use core::atomics;
use core::clone::Clone;
use std::fmt;

/// Copy-on-write tree structure.
/// Nodes are stored in a vector and refered to using an id equivalent to their
/// position in the vector.
/// When a node is removed, it is not deallocated right away: instead its index
/// is added to the list of available slots, clained next time a node is created.
pub struct SceneGraph<T> {
    nodes: Vec<NodeRef<T>>,
    free_slots: Vec<u16>,
    // TODO - this might not be the best representation as we will want to query
    // the changes per NodeId (ideally without having walking the change list)
    change_list: Vec<(NodeId, ChangeFlags)>,
    transaction_id: u64,
    next_node_gen: u32,
}

#[deriving(Clone)]
pub struct NodeHeader {
    children: Vec<NodeId>,
    parent: NodeId,
}

pub struct Node<T> {
    ref_count: atomics::AtomicInt,
    header: NodeHeader,
    data: T,
    gen: u32,
}

struct NodeRef<T> {
    ptr: *mut Node<T>
}

#[deriving(PartialEq, Clone)]
pub struct NodeId {
    handle: u32, // index if the node
    gen: u32,    // exra pseudo-random number to avoid Id re-use issues
}

#[deriving(Show)]
pub enum RendererMsg {
    Transaction(SceneGraph<SceneGraphData>),
    Stop,
}

#[deriving(Show)]
pub enum RendererReply {
    AfterTransaction(u64),
    AfterStop,
}

/// To facilitate cache invalidation, keep track the type of modifications
/// that we do to nodes.
pub type ChangeFlags = u32;
pub static CHANGE_TRANSFORM: ChangeFlags = 1 << 0;
pub static CHANGE_PARENT:    ChangeFlags = 1 << 1;
pub static CHANGE_CHILDREN:  ChangeFlags = 1 << 2;
pub static CHANGE_CLIP:      ChangeFlags = 1 << 3;
pub static CHANGE_GEOM:      ChangeFlags = 1 << 4;
pub static CHANGE_OPACITY:   ChangeFlags = 1 << 5;
pub static CHANGE_CREATE:    ChangeFlags = 1 << 6;
pub static CHANGE_REMOVE:    ChangeFlags = 1 << 7;

impl NodeHeader {
    pub fn new(parent: NodeId) -> NodeHeader {
        NodeHeader {
            parent: parent,
            children: Vec::new(),
        }
    }
}

impl<T: Clone> Node<T> {
    fn new(header: NodeHeader, data: T, gen: u32) -> Node<T> {
        let n = Node {
            ref_count: atomics::AtomicInt::new(1),
            header: header,
            data: data,
            gen: gen,
        };
        return n;
    }
}

impl<T: Clone> NodeRef<T> {
    fn new(parent: NodeId, data: T, gen: u32) -> NodeRef<T> {
        unsafe {
            let result = NodeRef {
                ptr: mem::transmute(malloc(mem::size_of::<Node<T>>() as u64))
            };
            (*result.ptr) = Node::new(NodeHeader::new(parent), data, gen);
            return result;
        }
    }

    fn deep_clone(&self) -> NodeRef<T> {
        unsafe {
            let result = NodeRef {
                ptr: mem::transmute(malloc(mem::size_of::<Node<T>>() as u64))
            };
            (*result.ptr) = Node::new(
                (*self.ptr).header.clone(),
                (*self.ptr).data.clone(),
                (*self.ptr).gen
            );
            return result;
        }
    }

    #[inline]
    pub fn add_ref(& self) {
        unsafe {
            (*self.ptr).ref_count.fetch_add(1, atomics::SeqCst);
        }
    }
    #[inline]
    pub fn release_ref(&mut self) {
        unsafe {
            if (*self.ptr).ref_count.fetch_sub(1, atomics::Release) == 1 {
                atomics::fence(atomics::Acquire);
                free(mem::transmute(self.ptr));
            }
        }
    }
    pub fn ref_count(&self) -> int {
        unsafe {
            return (*self.ptr).ref_count.load(atomics::SeqCst);
        }
    }
}

impl<T: Clone> Clone for NodeRef<T> {
    fn clone(&self) -> NodeRef<T> {
        unsafe {
            self.add_ref();
            NodeRef { ptr: self.ptr }
        }
    }
}

#[unsafe_destructor]
impl<T: Clone> Drop for NodeRef<T> {
    fn drop(&mut self) {
        unsafe {
            self.release_ref();
        }
    }
}

impl<T: Clone> SceneGraph<T> {
    pub fn new() -> SceneGraph<T> {
        SceneGraph {
            nodes: Vec::new(),
            free_slots: Vec::new(),
            change_list: Vec::new(),
            transaction_id: 0,
            next_node_gen: 0,
        }
    }

    pub fn get_root_node(&self) -> NodeId { node_id(0) }

    pub fn add_child(&mut self, parent: NodeId, data: T) -> NodeId {
        unsafe {
            let new_node = NodeRef::new(parent, data, self.next_node_gen);
            let node_id;
            if !self.free_slots.is_empty() {
                // Re-use availale slot...
                *self.nodes.get_mut(self.free_slots.len()-1) = new_node;
                let idx = self.free_slots.pop().unwrap();
                node_id = NodeId {
                    handle: idx as u32,
                    gen: self.next_node_gen,
                };
            } else {
                // ...or push to the end of the vector.
                self.nodes.push(new_node);
                node_id = NodeId {
                    handle: self.nodes.len() as u32 - 1,
                    gen: self.next_node_gen,
                };
            }
            self.next_node_gen += 1;
            self.change_list.push((node_id, CHANGE_CREATE));
            return node_id;
        }
    }

    // Adds this node to the list of available slots.
    // the node will be actually removed whenever we add another node in its place.
    pub fn remove(&mut self, node: NodeId)
    {
        if self.free_slots.contains(&(node.handle as u16)) {
            fail!("Removed node {} twice!", node.handle);
        }
        self.free_slots.push(node.handle as u16);
        self.change_list.push((node, CHANGE_REMOVE));
    }

    pub fn get_data<'l>(&'l self, node: NodeId) -> &'l T {
        unsafe {
            &'l (*self.nodes.get(node.handle as uint).ptr).data
        }
    }

    pub fn get_mut_data<'l>(&'l mut self, node: NodeId, changes: ChangeFlags) -> &'l mut T {
        unsafe {
            self.change_list.push((node, changes));
            // If we are holding the only ref, no need to copy.
            if self.nodes.get_mut(node.handle as uint).ref_count() == 1 {
                return &'l mut (*self.nodes.get_mut(node.handle as uint).ptr).data;
            }
            // "copy on write" scenario.
            let mut node_ref = self.nodes.get_mut(node.handle as uint);
            *node_ref = node_ref.deep_clone();
            return &'l mut (*node_ref.ptr).data;
        }
    }

    pub fn snapshot(&mut self) -> SceneGraph<T> {
        unsafe {
            let mut clone: SceneGraph<T> = SceneGraph {
                nodes: self.nodes.clone(),
                free_slots: self.free_slots.clone(),
                change_list: Vec::new(),
                transaction_id: self.transaction_id,
                next_node_gen: self.next_node_gen + 1000,
            };
            mem::swap(&mut self.change_list, &mut clone.change_list);
            self.transaction_id += 1;
            return clone;
        }
    }
}

impl<T> fmt::Show for SceneGraph<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "SceneGraph(transaction: {})", self.transaction_id);
        Ok(())
    }
}

fn node_id(id: u32) -> NodeId { NodeId { handle: id, gen: 0 } }

fn consume<T>(byebye: T) {}

// placeholder
type SceneGraphData = u32;

pub fn test_cow_scene_graph() {

    let (to_renderer, from_advance): (Sender<RendererMsg>, Receiver<RendererMsg>) = comm::channel();
    let (to_advance, from_render): (Sender<RendererReply>, Receiver<RendererReply>) = comm::channel();

    spawn(proc() {

        loop {
            let mut msg = from_advance.recv();
            match msg {
                Transaction(scene_graph) => {
                    let transaction_id = scene_graph.transaction_id;
                    //consume(scene_graph);
                    to_advance.send(AfterTransaction(transaction_id));
                }
                Stop => {
                    break;
                } 
            }
        }

        println!("renderer task finished");
        to_advance.send(AfterStop);
    });

    let mut scene_graph: SceneGraph<SceneGraphData> = SceneGraph::new();
    let n1 = scene_graph.add_child(node_id(0), 1);
    let n2 = scene_graph.add_child(n1, 2);
    let n3 = scene_graph.add_child(n2, 3);

    loop {
        to_renderer.send(Transaction(scene_graph.snapshot()));
        *scene_graph.get_mut_data(node_id(0), CHANGE_GEOM) += 1;
        let _ = from_render.recv();
        if scene_graph.transaction_id > 100000 {
            break;
        }
    }

    to_renderer.send(Stop);
    match from_render.recv() {
        AfterStop => {}
        msg => fail!("Expected AfterStop message, got {}", msg),
    }
}
