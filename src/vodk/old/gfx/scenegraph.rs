use std::comm;
use std::mem;
use std::fmt;
use math::units::world;
use core::clone::Clone;
use base::containers::{Id, CwArcTable};
use gfx::renderer;

// placeholder
type SceneGraphData = u32;

pub struct SceneGraph<T> {
    nodes: CwArcTable<Node<T>>,
    // TODO - this might not be the best representation as we will want to query
    // the changes per Id<T> (ideally without having walking the change list)
    change_list: Vec<(Id<T>, ChangeFlags)>,
    transaction_id: u64,
}

#[derive(Clone)]
pub struct Node<T> {
    children: Vec<Id<Node<T>>>,
    parent: Id<Node<T>>,
    payload: T,
}

#[derive(Show)]
pub enum RendererMsg {
    Transaction(SceneGraph<SceneGraphData>),
    Stop,
}

#[derive(Show)]
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


impl<T: Clone> SceneGraph<T> {
    pub fn new() -> SceneGraph<T> {
        SceneGraph {
            nodes: CwArcTable::new(),
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

    pub fn add_child(&mut self, parent: Id<Node<T>>, val: T) -> Id<Node<T>> {
        let inner: Node<T> = Node {
            parent: parent,
            children: Vec::new(),
            payload: val,
        };
        let id = self.nodes.add(inner);
        self.nodes.get_mut(parent).children.push(id);
        return id;
    }

    pub fn remove(&mut self, id: Id<Node<T>>) {
        return self.nodes.remove(id);
    }

    pub fn get<'l>(&'l self, id: Id<Node<T>>) -> &'l T {
        return &'l self.nodes.get(id).payload;
    }

    pub fn get_mut<'l>(&'l mut self, id: Id<Node<T>>) -> &'l mut T {
        return &'l mut self.nodes.get_mut(id).payload;
    }

    pub fn set_root(&mut self, val: T) -> Id<Node<T>> {
        let inner: Node<T> = Node {
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

    pub fn len(&self) -> usize {
        return self.nodes.len();
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
        if scene_graph.transaction_id > 100 {
            break;
        }
    }

    to_renderer.send(Stop);
    match from_render.recv() {
        AfterStop => {}
        msg => fail!("Expected AfterStop message, got {}", msg),
    }
}

