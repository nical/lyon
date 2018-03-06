use std::iter;
use std::slice;

use lyon::path::PathEvent;
use lyon::path::iterator::PathIter;
use lyon::geom::euclid::{TypedPoint2D, UnknownUnit};

use resvg::tree::{Path, PathSegment};

fn point(x: f64, y: f64) -> TypedPoint2D<f32, UnknownUnit> {
    TypedPoint2D::new(x as f32, y as f32)
}

// map resvg::tree::PathSegment to lyon::path::PathEvent
fn as_event(ps: &PathSegment) -> PathEvent {
    match *ps {
        PathSegment::MoveTo { x, y } => PathEvent::MoveTo(point(x, y)),
        PathSegment::LineTo { x, y } => PathEvent::LineTo(point(x, y)),
        PathSegment::CurveTo {
            x1,
            y1,
            x2,
            y2,
            x,
            y,
        } => PathEvent::CubicTo(point(x1, y1), point(x2, y2), point(x, y)),
        PathSegment::ClosePath => PathEvent::Close,
    }
}

pub struct PathConv<'a>(SegmentIter<'a>);

// alias for the iterator returned by resvg::tree::Path::iter()
type SegmentIter<'a> = slice::Iter<'a, PathSegment>;

// alias for our `interface` iterator
type PathConvIter<'a> = iter::Map<SegmentIter<'a>, fn(&PathSegment) -> PathEvent>;

// provide a function which gives back a PathIter which is compatible with
// tesselators, so we don't have to implement the PathIterator trait
impl<'a> PathConv<'a> {
    pub fn path_iter(self) -> PathIter<PathConvIter<'a>> {
        PathIter::new(self.0.map(as_event))
    }
}

pub fn convert_path<'a>(p: &'a Path) -> PathConv<'a> {
    PathConv(p.segments.iter())
}
