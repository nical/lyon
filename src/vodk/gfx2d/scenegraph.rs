use gfx2d::color::Rgba;
use math::units::world;
use math::units::texels;
use containers::copy_on_write;
use gfx2d::shapes;
use gpu;

pub type NodeFlags = u16;
pub static ANIMATED_NODE: NodeFlags     = 1;
pub static ANIM_SCROLL: NodeFlags       = 1 << 1 + 1;
pub static ANIM_OPACITY: NodeFlags      = 1 << 2 + 1;
pub static ANIM_TRANSFORM: NodeFlags    = 1 << 3 + 1;
pub static ANIM_GEOMETRY: NodeFlags     = 1 << 4 + 1;
pub static IS_OPAQUE: NodeFlags         = 1 << 5;
pub static FORCE_INTERMEDIATE_SURFACE: NodeFlags = 1 << 6;
pub static CLIP: NodeFlags              = 1 << 7;

/// Track the changes that were made to a given node to help with managing
/// caches.
pub type ChangeFlags = u16;
pub static CHANGE_SIBLING: ChangeFlags      = 1;
pub static CHANGE_PARENT: ChangeFlags       = 1 << 1;
pub static CHANGE_CHILD: ChangeFlags        = 1 << 2;
pub static CHANGE_TRANSFORM: ChangeFlags    = 1 << 3;
pub static CHANGE_SHAPE: ChangeFlags        = 1 << 4;
pub static CHANGE_MATERIAL: ChangeFlags     = 1 << 5;
pub static CHANGE_NODE_FLAGS: ChangeFlags   = 1 << 6;
pub static CHANGE_NEW: ChangeFlags          = 1 << 7;

#[deriving(Clone, Show)]
pub struct MaterialId { handle: u32 }
#[deriving(Clone, Show)]
pub struct TextureId { handle: u32 }
#[deriving(Clone, Show)]
pub struct MeshId { handle: u32 }

pub type NodeId = Option<copy_on_write::Id<Node>>;

#[deriving(Clone, Show)]
pub enum NodeMaterial {
    SolidColor(Rgba<u8>),
    LinearGradient(world::Vec2, Rgba<u8>, world::Vec2, Rgba<u8>),
    Texture(TextureId, texels::Rectangle),
    CustomMaterial(MaterialId),
    NoMaterial,
}

#[deriving(Clone, Show)]
pub enum NodeShape {
    CircleNode(shapes::Circle),
    EllipsisNode(shapes::Ellipsis),
    RectangleNode(world::Rectangle),
    RoundedRectangleNode(shapes::RoundedRectangle),
    MeshNode(MeshId),
    NoShape,
}

#[deriving(Clone, Show)]
pub struct Node {
    pub data: NodeData,
    pub parent: NodeId,
    pub first_child: NodeId,
    pub next_sibling: NodeId,
    pub changes: ChangeFlags,
}

#[deriving(Clone, Show)]
pub struct NodeData {
    pub transform: world::Mat4,
    pub shape: NodeShape,
    pub material: NodeMaterial,
    pub bounding_box: world::Rectangle,
    pub flags: NodeFlags,
}


pub struct SceneGraph {
    nodes: copy_on_write::CwArcTable<Node>,
    root: NodeId,
}

impl SceneGraph {

    pub fn get(&self, id: NodeId) -> &Node {
        assert!(id.is_some());
        return self.nodes.get(id.unwrap());
    }

    pub fn get_mut(&mut self, id: NodeId, changes: ChangeFlags) -> &mut Node {
        assert!(id.is_some());
        let node: &mut Node = self.nodes.get_mut(id.unwrap());
        node.changes |= changes;
        return node;
    }

    pub fn snapshot(&mut self) -> SceneGraph {
        SceneGraph {
            nodes: self.nodes.snapshot(),
            root: self.root,
        }
    }

    pub fn add_child(&mut self, parent: NodeId, data: NodeData) -> NodeId {
        let node = Node {
            data: data,
            changes: CHANGE_NEW,
            parent: parent,
            first_child: None,
            next_sibling: None,
        };
        let id = self.nodes.add(node);
        match parent {
            Some(parent_id) => {
                let first_sibling = self.nodes.get(parent_id).first_child;
                match first_sibling {
                    Some(sibling_id) => {
                        let last_sibling = self.get_last_sibling(Some(sibling_id));
                        self.get_mut(last_sibling, CHANGE_SIBLING).next_sibling = Some(id);
                    }
                    None => {
                        self.get_mut(parent, CHANGE_CHILD).first_child = Some(id);
                    }
                }
            }
            None => {
                assert!(self.root == None);
                self.root = Some(id);
            }
        }
        return Some(id);
    }

    pub fn add_after(&mut self, sibbling: NodeId, data: NodeData) -> NodeId {
        assert!(sibbling != self.root);
        assert!(sibbling.is_some());

        let parent = self.get(sibbling).parent;
        let next = self.get(sibbling).next_sibling;

        let node = Node {
            data: data,
            changes: CHANGE_NEW,
            parent: parent,
            first_child: None,
            next_sibling: next,
        };
        let id = self.nodes.add(node);

        self.get_mut(sibbling, CHANGE_SIBLING).next_sibling = Some(id);

        return Some(id);
    }

    pub fn remove(&mut self, id: NodeId) {
        assert!(id.is_some());
        self.nodes.remove(id.unwrap());
        // TODO: fix siblings, remove children, etc.
    }

    pub fn get_root(&self) -> NodeId { self.root }

    pub fn get_last_sibling(&self, id: NodeId) -> NodeId {
        assert!(id.is_some());
        let mut it = id;
        loop {
            let next = self.get(it).next_sibling;
            if next.is_none() {
                return it;
            }
            it = next
        }
    }

    pub fn get_first_sibling(&self, id: NodeId) -> NodeId {
        assert!(id.is_some());
        assert!(self.get(id).parent.is_some());

        return self.get(self.get(id).parent).first_child;
    }

    pub fn get_previous_sibling(&self, id: NodeId) -> NodeId {
        assert!(id.is_some());
        assert!(self.get(id).parent.is_some());
        let mut it = self.get_first_sibling(id);
        loop {
            let next = self.get(it).next_sibling;
            if next.is_none() || next == id {
                return it;
            }
            it = next
        }
    }

    pub fn num_nodes(&self) -> u32 { self.nodes.len() as u32 }
}

/*

for each node N and each batch B(N.geometry_root) {
    if N.shader != B.shader { try another batch }
    if N.aabb intersects B.region { try another batch }

}
*/

pub struct Range {
    pub first: u16,
    pub count: u16,
}

pub struct BufferRanges {
    pub vertices: Range,
    pub indices: Range,
    pub id: u32,
}

pub struct Batch {
    vertices: Vec<u8>,
    indices: Vec<u8>,
    ranges: Vec<BufferRanges>,
    regions: Vec<world::Vec2>,
    attributes: Vec<gpu::VertexAttribute>,
    stride: u32,
    geom: gpu::Geometry,
}

//impl Batch {
//    pub fn get_vertex_stream<'l, T>(
//        &'l mut self,
//        vertices: Range,
//        indices: Range
//    ) -> VertexStream<'l, T> {
//        //assert!(mem::size_of::<T>() as u32 == self.stride);
//
//        VertexStream {
//            vertices:
//            indices:
//        }
//    }
//}
