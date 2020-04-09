use crate::math::*;
use crate::builder::*;
use crate::path;
use crate::{EndpointId, PathSlice};

use std::fmt;

#[derive(Clone, Debug)]
struct PathDescriptor {
    points: (u32, u32),
    verbs: (u32, u32),
    num_attributes: u32,
}

/// An object that stores multiple paths contiguously.
#[derive(Clone)]
pub struct PathBuffer {
    points: Vec<Point>,
    verbs: Vec<path::Verb>,
    paths: Vec<PathDescriptor>,
}

/// Refers to a path in a `PathBuffer`.
#[derive(Copy, Clone, Debug)]
pub struct PathId(pub u32);

impl PathId {
    #[inline]
    pub fn to_usize(&self) -> usize { self.0 as usize }
}

impl PathBuffer {
    pub fn new() -> Self {
        PathBuffer {
            points: Vec::new(),
            verbs: Vec::new(),
            paths: Vec::new(),
        }
    }

    pub fn with_capacity(endpoints: usize, ctrl_points: usize, paths: usize) -> Self {
        let mut buffer = PathBuffer::new();
        buffer.reserve(endpoints, ctrl_points, paths);

        buffer
    }

    #[inline]
    pub fn as_slice(&self) -> PathBufferSlice {
        PathBufferSlice {
            points: &self.points,
            verbs: &self.verbs,
            paths: &self.paths,
        }
    }

    #[inline]
    pub fn get(&self, id: PathId) -> PathSlice {
        let desc = &self.paths[id.0 as usize];
        PathSlice {
            points: &self.points[desc.points.0 as usize..desc.points.1 as usize],
            verbs: &self.verbs[desc.verbs.0 as usize..desc.verbs.1 as usize],
            num_attributes: desc.num_attributes as usize,
        }
    }

    #[inline]
    pub fn ids(&self) -> PathIdRange {
        PathIdRange {
            range: 0..self.paths.len() as u32,
        }
    }

    #[inline]
    pub fn builder(&mut self) -> Builder {
        Builder::new(self)
    }

    #[inline]
    pub fn clear(&mut self) {
        self.points.clear();
        self.verbs.clear();
    }

    #[inline]
    pub fn reserve(&mut self, endpoints: usize, ctrl_points: usize, paths: usize) {
        self.points.reserve(endpoints + ctrl_points);
        self.verbs.reserve(endpoints);
        self.paths.reserve(paths);
    }
}

impl fmt::Debug for PathBuffer {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        self.as_slice().fmt(formatter)
    }
}

/// A view on a `PathBuffer`.
pub struct PathBufferSlice<'l> {
    points: &'l [Point],
    verbs: &'l [path::Verb],
    paths: &'l [PathDescriptor],
}

impl<'l> PathBufferSlice<'l> {
    #[inline]
    pub fn get(&self, id: PathId) -> PathSlice {
        let desc = &self.paths[id.0 as usize];
        PathSlice {
            points: &self.points[desc.points.0 as usize..desc.points.1 as usize],
            verbs: &self.verbs[desc.verbs.0 as usize..desc.verbs.1 as usize],
            num_attributes: desc.num_attributes as usize,
        }
    }

    #[inline]
    pub fn ids(&self) -> PathIdRange {
        PathIdRange {
            range: 0..self.paths.len() as u32,
        }
    }
}

impl<'l> fmt::Debug for PathBufferSlice<'l> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "PathBuffer {{ paths: {:?}, points: {:?}, verbs: {:?}, ",
            self.paths.len(),
            self.points.len(),
            self.verbs.len(),
        )?;
        for path in self.ids() {
            write!(formatter, "#{:?}: ", path.0)?;
            self.get(path).fmt(formatter)?;
            write!(formatter, ", ")?;
        }
        write!(formatter, " }}")
    }
}

/// A Builder that appends a path to an existing PathBuffer.
///
/// Implements the `PathBuilder` trait.
pub struct Builder<'l> {
    buffer: &'l mut PathBuffer,
    builder: path::Builder,
    points_start: u32,
    verbs_start: u32,
}

impl<'l> Builder<'l> {
    #[inline]
    fn new(buffer: &'l mut PathBuffer) -> Self {
        let points = std::mem::replace(&mut buffer.points, Vec::new());
        let verbs = std::mem::replace(&mut buffer.verbs, Vec::new());
        let points_start = points.len() as u32;
        let verbs_start = verbs.len() as u32;
        Builder {
            buffer,
            builder: path::Builder {
                points,
                verbs,
            },
            points_start,
            verbs_start,
        }
    }

    #[inline]
    pub fn with_attributes(self, num_attributes: usize) -> BuilderWithAttributes<'l> {
        assert_eq!(self.builder.verbs.len(), self.verbs_start as usize);

        BuilderWithAttributes {
            buffer: self.buffer,
            builder: path::BuilderWithAttributes {
                builder: self.builder,
                num_attributes,
            },
            points_start: self.points_start,
            verbs_start: self.verbs_start,
        }
    }

    #[inline]
    pub fn build(mut self) -> PathId {
        let points_end = self.builder.points.len() as u32;
        let verbs_end = self.builder.verbs.len() as u32;
        std::mem::swap(&mut self.builder.points, &mut self.buffer.points);
        std::mem::swap(&mut self.builder.verbs, &mut self.buffer.verbs);

        let id = PathId(self.buffer.paths.len() as u32);
        self.buffer.paths.push(PathDescriptor {
            points: (self.points_start, points_end),
            verbs: (self.verbs_start, verbs_end),
            num_attributes: 0,
        });

        id
    }

    #[inline]
    fn adjust_id(&self, mut id: EndpointId) -> EndpointId {
        id.0 -= self.points_start;

        id
    }

    #[inline]
    pub fn begin(&mut self, at: Point) -> EndpointId {
        let id = self.builder.begin(at);
        self.adjust_id(id)
    }

    #[inline]
    pub fn end(&mut self, close: bool) {
        self.builder.end(close)
    }

    #[inline]
    pub fn line_to(&mut self, to: Point) -> EndpointId {
        let id = self.builder.line_to(to);
        self.adjust_id(id)
    }

    #[inline]
    pub fn quadratic_bezier_to(&mut self, ctrl: Point, to: Point) -> EndpointId {
        let id = self.builder.quadratic_bezier_to(ctrl, to);
        self.adjust_id(id)
    }

    #[inline]
    pub fn cubic_bezier_to(&mut self, ctrl1: Point, ctrl2: Point, to: Point) -> EndpointId {
        let id = self.builder.cubic_bezier_to(ctrl1, ctrl2, to);
        self.adjust_id(id)
    }

    #[inline]
    pub fn reserve(&mut self, endpoints: usize, ctrl_points: usize) {
        self.builder.reserve(endpoints, ctrl_points);
    }
}

impl<'l> PathBuilder for Builder<'l> {
    #[inline]
    fn begin(&mut self, at: Point) -> EndpointId {
        self.begin(at)
    }

    #[inline]
    fn end(&mut self, close: bool) {
        self.end(close);
    }

    #[inline]
    fn line_to(&mut self, to: Point) -> EndpointId {
        self.line_to(to)
    }

    #[inline]
    fn quadratic_bezier_to(&mut self, ctrl: Point, to: Point) -> EndpointId {
        self.quadratic_bezier_to(ctrl, to)
    }

    #[inline]
    fn cubic_bezier_to(&mut self, ctrl1: Point, ctrl2: Point, to: Point) -> EndpointId {
        self.cubic_bezier_to(ctrl1, ctrl2, to)
    }

    #[inline]
    fn reserve(&mut self, endpoints: usize, ctrl_points: usize) {
        self.reserve(endpoints, ctrl_points);
    }
}

pub struct PathIdRange {
    range: std::ops::Range<u32>,
}

impl Iterator for PathIdRange {
    type Item = PathId;
    #[inline]
    fn next(&mut self) -> Option<PathId> {
        self.range.next().map(|idx| PathId(idx))
    }
}


/// A Builder that appends a path to an existing PathBuffer, with custom attributes.
pub struct BuilderWithAttributes<'l> {
    buffer: &'l mut PathBuffer,
    builder: path::BuilderWithAttributes,
    points_start: u32,
    verbs_start: u32,
}

impl<'l> BuilderWithAttributes<'l> {
    #[inline]
    pub fn new(buffer: &'l mut PathBuffer, num_attributes: usize) -> Self {
        let points = std::mem::replace(&mut buffer.points, Vec::new());
        let verbs = std::mem::replace(&mut buffer.verbs, Vec::new());
        let points_start = points.len() as u32;
        let verbs_start = verbs.len() as u32;
        BuilderWithAttributes {
            buffer,
            builder: path::BuilderWithAttributes {
                builder: path::Builder {
                    points,
                    verbs,
                },
                num_attributes,
            },
            points_start,
            verbs_start,
        }
    }

    #[inline]
    pub fn build(mut self) -> PathId {
        let points_end = self.builder.builder.points.len() as u32;
        let verbs_end = self.builder.builder.verbs.len() as u32;
        std::mem::swap(&mut self.builder.builder.points, &mut self.buffer.points);
        std::mem::swap(&mut self.builder.builder.verbs, &mut self.buffer.verbs);

        let id = PathId(self.buffer.paths.len() as u32);
        self.buffer.paths.push(PathDescriptor {
            points: (self.points_start, points_end),
            verbs: (self.verbs_start, verbs_end),
            num_attributes: 0,
        });

        id
    }

    #[inline]
    fn adjust_id(&self, mut id: EndpointId) -> EndpointId {
        id.0 -= self.points_start;

        id
    }

    #[inline]
    pub fn begin(&mut self, at: Point, attributes: &[f32]) -> EndpointId {
        let id = self.builder.begin(at, attributes);
        self.adjust_id(id)
    }

    #[inline]
    pub fn end(&mut self, close: bool) {
        self.builder.end(close)
    }

    #[inline]
    pub fn line_to(&mut self, to: Point, attributes: &[f32]) -> EndpointId {
        let id = self.builder.line_to(to, attributes);
        self.adjust_id(id)
    }

    #[inline]
    pub fn quadratic_bezier_to(&mut self, ctrl: Point, to: Point, attributes: &[f32]) -> EndpointId {
        let id = self.builder.quadratic_bezier_to(ctrl, to, attributes);
        self.adjust_id(id)
    }

    #[inline]
    pub fn cubic_bezier_to(&mut self, ctrl1: Point, ctrl2: Point, to: Point, attributes: &[f32]) -> EndpointId {
        let id = self.builder.cubic_bezier_to(ctrl1, ctrl2, to, attributes);
        self.adjust_id(id)
    }

    #[inline]
    pub fn reserve(&mut self, endpoints: usize, ctrl_points: usize) {
        self.builder.reserve(endpoints, ctrl_points);
    }
}

#[test]
fn simple() {
  use crate::PathEvent;

  let mut buffer = PathBuffer::new();

  let mut builder = buffer.builder();
  builder.begin(point(0.0, 0.0));
  builder.line_to(point(10.0, 0.0));
  builder.line_to(point(10.0, 10.0));
  builder.line_to(point(0.0, 10.0));
  builder.end(true);

  let p1 = builder.build();

  let mut builder = buffer.builder();
  builder.begin(point(0.0, 0.0));
  builder.line_to(point(20.0, 0.0));
  builder.line_to(point(20.0, 20.0));
  builder.line_to(point(0.0, 20.0));
  builder.end(false);

  let p2 = builder.build();

  let mut iter = buffer.get(p1).iter();
  assert_eq!(iter.next(), Some(PathEvent::Begin { at: point(0.0, 0.0) }));
  assert_eq!(iter.next(), Some(PathEvent::Line { from: point(0.0, 0.0), to: point(10.0, 0.0) }));
  assert_eq!(iter.next(), Some(PathEvent::Line { from: point(10.0, 0.0), to: point(10.0, 10.0) }));
  assert_eq!(iter.next(), Some(PathEvent::Line { from: point(10.0, 10.0), to: point(0.0, 10.0) }));
  assert_eq!(iter.next(), Some(PathEvent::End { last: point(0.0, 10.0), first: point(0.0, 0.0), close: true }));
  assert_eq!(iter.next(), None);

  let mut iter = buffer.get(p2).iter();
  assert_eq!(iter.next(), Some(PathEvent::Begin { at: point(0.0, 0.0) }));
  assert_eq!(iter.next(), Some(PathEvent::Line { from: point(0.0, 0.0), to: point(20.0, 0.0) }));
  assert_eq!(iter.next(), Some(PathEvent::Line { from: point(20.0, 0.0), to: point(20.0, 20.0) }));
  assert_eq!(iter.next(), Some(PathEvent::Line { from: point(20.0, 20.0), to: point(0.0, 20.0) }));
  assert_eq!(iter.next(), Some(PathEvent::End { last: point(0.0, 20.0), first: point(0.0, 0.0), close: false }));
  assert_eq!(iter.next(), None);
}
