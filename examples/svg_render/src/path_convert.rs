use std::slice;

use lyon::math::Point;
use lyon::path::PathEvent;
use lyon::path::iterator::PathIter;
use lyon::geom::{LineSegment, CubicBezierSegment};

use usvg::{Path, PathSegment};

fn point(x: &f64, y: &f64) -> Point {
    Point::new((*x) as f32, (*y) as f32)
}

pub struct PathConv<'a>(SegmentIter<'a>);

// Alias for the iterator returned by usvg::Path::iter()
type SegmentIter<'a> = slice::Iter<'a, PathSegment>;

pub struct PathConvIter<'a> {
    iter: slice::Iter<'a, PathSegment>,
    prev: Point,
    first: Point,
}

// Provide a function which gives back a PathIter which is compatible with
// tessellators, so we don't have to implement the PathIterator trait
impl<'a> PathConv<'a> {
    pub fn path_iter(self) -> PathIter<PathConvIter<'a>> {
        PathIter::new(PathConvIter {
            iter: self.0,
            prev: Point::new(0.0, 0.0),
            first: Point::new(0.0, 0.0),
        })
    }
}

impl<'l> Iterator for PathConvIter<'l> {
    type Item = PathEvent;
    fn next(&mut self) -> Option<PathEvent> {
        match self.iter.next() {
            Some(PathSegment::MoveTo { x, y }) => {
                self.prev = point(x, y);
                self.first = self.prev;
                Some(PathEvent::MoveTo(self.prev))
            }
            Some(PathSegment::LineTo { x, y }) => {
                let from = self.prev;
                self.prev = point(x, y);
                Some(PathEvent::Line(LineSegment { from, to: self.prev }))
            }
            Some(PathSegment::CurveTo { x1, y1, x2, y2, x, y, }) => {
                let from = self.prev;
                self.prev = point(x, y);
                Some(PathEvent::Cubic(CubicBezierSegment {
                    from,
                    ctrl1: point(x1, y1),
                    ctrl2: point(x2, y2),
                    to: self.prev,
                }))
            }
            Some(PathSegment::ClosePath) => {
                self.prev = self.first;
                Some(PathEvent::Close(LineSegment {
                    from: self.prev,
                    to: self.first,
                }))
            }
            None => None,
        }
    }
}

pub fn convert_path<'a>(p: &'a Path) -> PathConv<'a> {
    PathConv(p.segments.iter())
}
