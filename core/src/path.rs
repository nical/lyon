//! Data structures to represent complex paths.
//!
//! This whole module will change at some point in order to implement a more
//! flexible and efficient Path data structure.

use super::{
    vertex_id, VertexId, VertexIdRange,
    VertexSlice, MutVertexSlice,
};

use vodk_math::{ Vec2, Rect, };

use sid::{ Id, IdRange, ToIndex };

#[derive(Debug)]
/// Phatom type marker for PathId.
pub struct Path_;
/// An Id that represents a sub-path in a certain path object.
pub type PathId = Id<Path_, u16>;
/// A contiguous range of PathIds.
pub type PathIdRange = IdRange<Path_, u16>;
pub fn path_id(idx: u16) -> PathId { PathId::new(idx) }

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum PointType {
    Normal,
    Control,
}

// TODO: Need a better representation for paths. It needs to:
//  * be compact
//  * allow quickly finding the previous and next command
//  * be stored in a contiguous buffer
//  * be extensible (add extra parameters to vertices)
//  * not store pointers
//  * be iterable but not necessarily random-accessible
#[derive(Copy, Clone, Debug)]
pub struct PointData {
    pub position: Vec2,
    pub point_type: PointType,
}

/// The data structure that represent a complex path.
///
/// This API is not stable yet. Both the data structure and the methods it exposes will
/// change, hopefully soon.
#[derive(Clone, Debug)]
pub struct Path {
    vertices: Vec<PointData>,
    sub_paths: Vec<PathInfo>,
}

impl Path {
    pub fn new() -> Path {
        Path { vertices: Vec::new(), sub_paths: Vec::new() }
    }

    pub fn from_vec(vertices: Vec<PointData>, sub_paths: Vec<PathInfo>) -> Path {
        Path {
            vertices: vertices,
            sub_paths: sub_paths,
        }
    }

    pub fn vertices(&self) -> VertexSlice<PointData> { VertexSlice::new(&self.vertices[..]) }

    pub fn mut_vertices(&mut self) -> MutVertexSlice<PointData> { MutVertexSlice::new(&mut self.vertices[..]) }

    pub fn num_vertices(&self) -> usize { self.as_slice().num_vertices() }

    pub fn sub_path(&self, id: PathId) -> SubPathSlice {
        SubPathSlice {
            vertices: VertexSlice::new(&self.vertices[..]),
            info: &self.sub_paths[id.handle.to_index()]
        }
    }

    pub fn path_ids(&self) -> PathIdRange {
        IdRange::new(0, self.sub_paths.len() as u16)
    }

    pub fn as_slice(&self) -> PathSlice {
        PathSlice {
            vertices: VertexSlice::new(&self.vertices[..]),
            sub_paths: &self.sub_paths[..],
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct PathVertexId {
    pub vertex_id: VertexId,
    pub path_id: PathId,
}

pub struct PathVertexIdRange {
    pub range: VertexIdRange,
    pub path_id: PathId,
}

impl Iterator for PathVertexIdRange {
    type Item = PathVertexId;
    fn next(&mut self) -> Option<PathVertexId> {
        return if let Some(next) = self.range.next() {
            Some(PathVertexId {
                vertex_id: next,
                path_id: self.path_id
            })
        } else {
            None
        };
    }
}

#[derive(Copy, Clone)]
pub struct PathSlice<'l> {
    vertices: VertexSlice<'l,PointData>,
    sub_paths: &'l[PathInfo],
}

impl<'l> PathSlice<'l> {

    pub fn vertices(&self) -> VertexSlice<PointData> { self.vertices }

    pub fn vertex_ids(&self, sub_path: PathId) -> PathVertexIdRange {
        PathVertexIdRange {
            range: self.sub_path(sub_path).vertex_ids(),
            path_id: sub_path,
        }
    }

    pub fn num_vertices(&self) -> usize { self.vertices.len() }

    pub fn num_sub_paths(&self) -> usize { self.sub_paths.len() }

    pub fn sub_path(&self, id: PathId) -> SubPathSlice {
        SubPathSlice {
            vertices: self.vertices,
            info: &self.sub_paths[id.handle.to_index()]
        }
    }

    pub fn path_ids(&self) -> PathIdRange {
        IdRange::new(0, self.sub_paths.len() as u16)
    }

    pub fn vertex(&self, id: PathVertexId) -> &PointData {
        &self.vertices[id.vertex_id]
    }

    pub fn next(&self, id: PathVertexId) -> PathVertexId {
        PathVertexId {
            path_id: id.path_id,
            vertex_id: self.sub_path(id.path_id).next(id.vertex_id),
        }
    }

    pub fn previous(&self, id: PathVertexId) -> PathVertexId {
        PathVertexId {
            path_id: id.path_id,
            vertex_id: self.sub_path(id.path_id).previous(id.vertex_id),
        }
    }
}

#[derive(Copy, Clone)]
pub struct SubPathSlice<'l> {
    vertices: VertexSlice<'l, PointData>,
    info: &'l PathInfo,
}

impl<'l> SubPathSlice<'l> {
    pub fn info(&self) -> &'l PathInfo { self.info }

    pub fn vertex(&self, id: VertexId) -> &PointData { &self.vertices[id] }

    pub fn first(&self) -> VertexId { self.info.range.first }

    pub fn last(&self) -> VertexId {
        vertex_id(self.info.range.first.handle + self.info.range.count - 1)
    }

    pub fn next(&self, id: VertexId) -> VertexId {
        let first = self.info.range.first.handle;
        let last = first + self.info.range.count - 1;
        debug_assert!(id.handle >= first);
        debug_assert!(id.handle <= last);
        return Id::new(if id.handle == last { first } else { id.handle + 1 });
    }

    pub fn previous(&self, id: VertexId) -> VertexId {
        let first = self.info.range.first.handle;
        let last = first + self.info.range.count - 1;
        debug_assert!(id.handle >= first);
        debug_assert!(id.handle <= last);
        return Id::new(if id.handle == first { last } else { id.handle - 1 });
    }

    pub fn next_vertex(&self, id: VertexId) -> &PointData {
        self.vertex(self.next(id))
    }

    pub fn previous_vertex(&self, id: VertexId) -> &PointData {
        self.vertex(self.previous(id))
    }

    pub fn vertex_ids(&self) -> VertexIdRange { self.info().range }

    pub fn num_vertices(&self) -> usize { self.vertices.len() }
}

/// Some metadata for sub paths
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct PathInfo {
    pub aabb: Rect,
    pub range: VertexIdRange,
    pub has_beziers: Option<bool>,
    pub is_closed: bool,
}

