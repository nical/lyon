use std::comm;
use math::units::world;
use libc::funcs::c95::stdlib::{malloc, free};
use std::mem;
use std::sync::Arc;
use std::ty::Unsafe;
use core::atomics;
use core::clone::Clone;
use std::fmt;
use gfx::renderer;

/// Copy-on-write container.
/// Nodes are stored in a vector and refered to using an id equivalent to their
/// position in the vector.
/// When a node is removed, it is not deallocated right away: instead its index
/// is added to the list of available slots, and the allocated node will be 
/// recycled next time a node is created.
pub struct CopyOnWriteArcVector<T> {
    data: Vec<UnsafeArc<T>>,
    free_slots: Vec<u16>,
    next_node_gen: u32,
}

pub struct UnsafeArcInner<T> {
    ref_count: atomics::AtomicInt,
    data: T,
    gen: u32,
}

struct UnsafeArc<T> {
    ptr: *mut UnsafeArcInner<T>
}

#[deriving(PartialEq, Clone)]
pub struct Id<T> {
    handle: u32,
    gen: u32,
}


// placeholder
type SceneGraphData = u32;

pub struct SceneGraph<T> {
    nodes: CopyOnWriteArcVector<SceneGraphInner<T>>,
    // TODO - this might not be the best representation as we will want to query
    // the changes per Id<T> (ideally without having walking the change list)
    change_list: Vec<(Id<T>, ChangeFlags)>,
    transaction_id: u64,
}

#[deriving(Clone)]
pub struct SceneGraphInner<T> {
    children: Vec<Id<SceneGraphInner<T>>>,
    parent: Id<SceneGraphInner<T>>,
    payload: T,
}

#[deriving(Clone)]
pub struct Node {
    todo: u32,
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

impl<T: Clone> UnsafeArcInner<T> {
    fn new(data: T, gen: u32) -> UnsafeArcInner<T> {
        let n = UnsafeArcInner {
            ref_count: atomics::AtomicInt::new(1),
            data: data,
            gen: gen,
        };
        return n;
    }
}

impl<T: Clone> UnsafeArc<T> {
    fn new(data: T, gen: u32) -> UnsafeArc<T> {
        unsafe {
            let result = UnsafeArc {
                ptr: mem::transmute(malloc(mem::size_of::<UnsafeArcInner<T>>() as u64))
            };
            (*result.ptr) = UnsafeArcInner::new(data, gen);
            return result;
        }
    }

    fn deep_clone(&self) -> UnsafeArc<T> {
        unsafe {
            let result = UnsafeArc {
                ptr: mem::transmute(malloc(mem::size_of::<UnsafeArcInner<T>>() as u64))
            };
            (*result.ptr) = UnsafeArcInner::new(
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

    #[inline]
    pub fn ref_count(&self) -> int {
        unsafe {
            return (*self.ptr).ref_count.load(atomics::SeqCst);
        }
    }

    #[inline]
    fn inner<'l>(&'l self) -> &'l UnsafeArcInner<T> {
        unsafe {
            return &'l (*self.ptr);
        }
    }
}

impl<T: Clone> Clone for UnsafeArc<T> {
    fn clone(&self) -> UnsafeArc<T> {
        unsafe {
            self.add_ref();
            UnsafeArc { ptr: self.ptr }
        }
    }
}

#[unsafe_destructor]
impl<T: Clone> Drop for UnsafeArc<T> {
    fn drop(&mut self) {
        unsafe {
            self.release_ref();
        }
    }
}


impl<T: Clone> SceneGraph<T> {
    pub fn new() -> SceneGraph<T> {
        SceneGraph {
            nodes: CopyOnWriteArcVector::new(),
            change_list: Vec::new(),
            transaction_id: 0,
        }
    }

    pub fn snapshot(&mut self) -> SceneGraph<T> {
        unsafe {
            let mut clone: SceneGraph<T> = SceneGraph {
                nodes: self.nodes.snapshot(),
                change_list: Vec::new(),
                transaction_id: self.transaction_id,
            };
            mem::swap(&mut self.change_list, &mut clone.change_list);
            self.transaction_id += 1;
            return clone;
        }
    }

    pub fn add_child(&mut self, parent: Id<SceneGraphInner<T>>, val: T) -> Id<SceneGraphInner<T>> {
        let inner: SceneGraphInner<T> = SceneGraphInner {
            parent: parent,
            children: Vec::new(),
            payload: val,
        };
        let id = self.nodes.add(inner);
        self.nodes.get_mut(parent).children.push(id);
        return id;
    }

    pub fn remove(&mut self, id: Id<SceneGraphInner<T>>) {
        return self.nodes.remove(id);
    }

    pub fn get<'l>(&'l self, id: Id<SceneGraphInner<T>>) -> &'l T {
        return &'l self.nodes.get(id).payload;
    }

    pub fn get_mut<'l>(&'l mut self, id: Id<SceneGraphInner<T>>) -> &'l mut T {
        return &'l mut self.nodes.get_mut(id).payload;
    }

    pub fn set_root(&mut self, val: T) -> Id<SceneGraphInner<T>> {
        let inner: SceneGraphInner<T> = SceneGraphInner {
            parent: Id { handle: 0, gen: 0},
            children: Vec::new(),
            payload: val,
        };
        if self.len() == 0 {
            return self.nodes.add(inner);
        }
        *self.nodes.unchecked_get_mut(0) = inner;
        return Id { handle: 0, gen: self.nodes.get_gen(0) };
    }

    pub fn len(&self) -> uint {
        return self.nodes.len();
    }
}

// TODO: iterator, clear
impl<T: Clone> CopyOnWriteArcVector<T> {
    pub fn new() -> CopyOnWriteArcVector<T> {
        CopyOnWriteArcVector {
            data: Vec::new(),
            free_slots: Vec::new(),
            next_node_gen: 0,
        }
    }

    pub fn add(&mut self, data: T) -> Id<T> {
        unsafe {
            let new_node = UnsafeArc::new(data, self.next_node_gen);
            let node_id;
            if !self.free_slots.is_empty() {
                // Re-use availale slot...
                *self.data.get_mut(self.free_slots.len()-1) = new_node;
                let idx = self.free_slots.pop().unwrap();
                node_id = Id {
                    handle: idx as u32,
                    gen: self.next_node_gen,
                };
            } else {
                // ...or push to the end of the vector.
                self.data.push(new_node);
                node_id = Id {
                    handle: self.data.len() as u32 - 1,
                    gen: self.next_node_gen,
                };
            }
            self.next_node_gen += 1;
            return node_id;
        }
    }

    // Adds this node to the list of available slots.
    // The node will be actually removed whenever we add another node in its place.
    pub fn remove(&mut self, node: Id<T>)
    {
        if self.free_slots.contains(&(node.handle as u16)) {
            fail!("Removed node {} twice!", node.handle);
        }
        self.free_slots.push(node.handle as u16);
    }

    pub fn get<'l>(&'l self, node: Id<T>) -> &'l T {
        if node.gen != self.get_gen(node.handle) { fail!("invalid handle"); }
        return self.unchecked_get(node.handle);
    }

    fn unchecked_get<'l>(&'l self, index: u32) -> &'l T {
        unsafe {
            &'l (*self.data.get(index as uint).ptr).data
        }
    }

    pub fn get_mut<'l>(&'l mut self, node: Id<T>) -> &'l mut T {
        if node.gen != self.get_gen(node.handle) { fail!("invalid handle"); }
        return self.unchecked_get_mut(node.handle);
    }

    fn unchecked_get_mut<'l>(&'l mut self, index: u32) -> &'l mut T {
        unsafe {
            // If we are holding the only ref, no need to copy.
            if self.data.get_mut(index as uint).ref_count() == 1 {
                return &'l mut (*self.data.get_mut(index as uint).ptr).data;
            }
            // "copy on write" scenario.
            let mut node_ref = self.data.get_mut(index as uint);
            *node_ref = node_ref.deep_clone();
            return &'l mut (*node_ref.ptr).data;
        }
    }

    pub fn len(&self) -> uint {
        return self.data.len() - self.free_slots.len();
    }

    pub fn snapshot(&mut self) -> CopyOnWriteArcVector<T> {
        unsafe {
            let mut clone: CopyOnWriteArcVector<T> = CopyOnWriteArcVector {
                data: self.data.clone(),
                free_slots: self.free_slots.clone(),
                next_node_gen: self.next_node_gen + 1000,
            };
            return clone;
        }
    }

    pub fn get_gen(&self, index: u32) -> u32 {
        return self.data.get(index as uint).inner().gen;
    }
}

impl<T> fmt::Show for SceneGraph<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "SceneGraph(transaction_id: {})", self.transaction_id);
        Ok(())
    }
}

pub fn test_cow_scene_graph() {

    let (to_renderer, from_advance): (Sender<RendererMsg>, Receiver<RendererMsg>) = comm::channel();
    let (to_advance, from_render): (Sender<RendererReply>, Receiver<RendererReply>) = comm::channel();

    spawn(proc() {

        loop {
            let mut msg = from_advance.recv();
            match msg {
                Transaction(scene_graph) => {
                    let transaction_id = scene_graph.transaction_id;
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
    let n1 = scene_graph.set_root(1);
    let n2 = scene_graph.add_child(n1, 2);
    let n3 = scene_graph.add_child(n2, 3);

    loop {
        to_renderer.send(Transaction(scene_graph.snapshot()));
        *scene_graph.get_mut(n1) += 1;
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

