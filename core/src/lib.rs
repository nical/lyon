extern crate sid;
extern crate sid_vec;
extern crate euclid;

pub mod math;
pub mod path;
pub mod path_builder;
pub mod path_iterator;
pub mod math_utils;
pub mod bezier;
pub mod flatten_cubic;
pub mod arc;

use math::{ Point, Vec2 };

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum SvgEvent {
    MoveTo(Point),
    RelativeMoveTo(Vec2),
    LineTo(Point),
    RelativeLineTo(Vec2),
    QuadraticTo(Point, Point),
    RelativeQuadraticTo(Vec2, Vec2),
    CubicTo(Point, Point, Point),
    RelativeCubicTo(Vec2, Vec2, Vec2),
    ArcTo(Vec2, Vec2, Vec2),
    HorizontalLineTo(f32),
    VerticalLineTo(f32),
    RelativeHorizontalLineTo(f32),
    RelativeVerticalLineTo(f32),
    Close,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum PrimitiveEvent {
    MoveTo(Point),
    LineTo(Point),
    QuadraticTo(Point, Point),
    CubicTo(Point, Point, Point),
    Close,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum FlattenedEvent {
    MoveTo(Point),
    LineTo(Point),
    Close,
}

impl SvgEvent {
    pub fn to_primitive(self, current: Point) -> PrimitiveEvent {
        return match self {
            SvgEvent::MoveTo(to) => { PrimitiveEvent::MoveTo(to) }
            SvgEvent::LineTo(to) => { PrimitiveEvent::LineTo(to) }
            SvgEvent::QuadraticTo(ctrl, to) => { PrimitiveEvent::QuadraticTo(ctrl, to) }
            SvgEvent::CubicTo(ctrl1, ctrl2, to) => { PrimitiveEvent::CubicTo(ctrl1, ctrl2, to) }
            SvgEvent::Close => { PrimitiveEvent::Close }
            SvgEvent::RelativeMoveTo(to) => { PrimitiveEvent::MoveTo(current + to) }
            SvgEvent::RelativeLineTo(to) => { PrimitiveEvent::LineTo(current + to) }
            SvgEvent::RelativeQuadraticTo(ctrl, to) => { PrimitiveEvent::QuadraticTo(current + ctrl, current + to) }
            SvgEvent::RelativeCubicTo(ctrl1, ctrl2, to) => { PrimitiveEvent::CubicTo(current + ctrl1, current + ctrl2, to) }
            SvgEvent::HorizontalLineTo(x) => { PrimitiveEvent::LineTo(Point::new(x, current.y)) }
            SvgEvent::VerticalLineTo(y) => { PrimitiveEvent::LineTo(Point::new(current.x, y)) }
            SvgEvent::RelativeHorizontalLineTo(x) => { PrimitiveEvent::LineTo(Point::new(current.x + x, current.y)) }
            SvgEvent::RelativeVerticalLineTo(y) => { PrimitiveEvent::LineTo(Point::new(current.x, current.y + y)) }
            // TODO arcs and smooth events
            _ => { unimplemented!() }
        };
    }
}

impl PrimitiveEvent {
    pub fn to_svg(self) -> SvgEvent {
        return match self {
            PrimitiveEvent::MoveTo(to) => { SvgEvent::MoveTo(to) }
            PrimitiveEvent::LineTo(to) => { SvgEvent::LineTo(to) }
            PrimitiveEvent::QuadraticTo(ctrl, to) => { SvgEvent::QuadraticTo(ctrl, to) }
            PrimitiveEvent::CubicTo(ctrl1, ctrl2, to) => { SvgEvent::CubicTo(ctrl1, ctrl2, to) }
            PrimitiveEvent::Close => { SvgEvent::Close }
        };
    }
}

use sid::{ Id, IdRange, };
use sid_vec::{ IdSlice, MutIdSlice, };

/// The integer type to index a vertex in a vertex buffer or path.
pub type Index = u16;

/// Phantom type marker for VertexId.
#[derive(Debug)]
pub struct Vertex_;

/// An Id that represents a vertex in a contiguous vertex buffer.
pub type VertexId = Id<Vertex_, Index>;

/// Create a VertexId from an u16 index.
#[inline]
pub fn vertex_id(index: Index) -> VertexId { VertexId::new(index) }

/// A range of VertexIds pointing to contiguous vertices.
pub type VertexIdRange = IdRange<Vertex_, Index>;

/// Create a VertexIdRange.
pub fn vertex_id_range(from: u16, to: u16) -> VertexIdRange {
    IdRange {
        first: Id::new(from),
        count: to - from,
    }
}

/// A slice of vertices indexed with VertexIds rather than usize offset.
pub type VertexSlice<'l, V> = IdSlice<'l, VertexId, V>;

/// A slice of mutable vertices indexed with VertexIds rather than usize offset.
pub type MutVertexSlice<'l, V> = MutIdSlice<'l, VertexId, V>;
