use std::slice;

use lyon::math::Point;
use lyon::path::PathEvent;

use usvg;

// This module implements some glue between usvg and lyon.
// PathConvIter translate usvg's path data structure into an
// iterator of lyon's PathEvent.

fn point(x: &f64, y: &f64) -> Point {
    Point::new((*x) as f32, (*y) as f32)
}

pub struct PathConvIter<'a> {
    iter: slice::Iter<'a, usvg::PathSegment>,
    prev: Point,
    first: Point,
    needs_end: bool,
    deferred_moveto: Option<Point>,
}

impl<'l> Iterator for PathConvIter<'l> {
    type Item = PathEvent<Point, Point>;
    fn next(&mut self) -> Option<PathEvent<Point, Point>> {
        if let Some(at) = self.deferred_moveto.take() {
            return Some(PathEvent::Begin { at });
        }
        match self.iter.next() {
            Some(usvg::PathSegment::MoveTo { x, y }) => {
                let prev = self.prev;
                self.prev = point(x, y);
                if self.needs_end {
                    self.deferred_moveto = Some(point(x, y));
                    Some(PathEvent::End { last: prev, first: self.first, close: false })
                } else {
                    self.needs_end = true;
                    self.first = point(x, y);
                    Some(PathEvent::Begin { at: self.prev })
                }
            }
            Some(usvg::PathSegment::LineTo { x, y }) => {
                self.needs_end = true;
                let from = self.prev;
                self.prev = point(x, y);
                Some(PathEvent::Line { from, to: self.prev })
            }
            Some(usvg::PathSegment::CurveTo { x1, y1, x2, y2, x, y, }) => {
                self.needs_end = true;
                let from = self.prev;
                self.prev = point(x, y);
                Some(PathEvent::Cubic {
                    from,
                    ctrl1: point(x1, y1),
                    ctrl2: point(x2, y2),
                    to: self.prev,
                })
            }
            Some(usvg::PathSegment::ClosePath) => {
                self.needs_end = false;
                self.prev = self.first;
                Some(PathEvent::End {
                    last: self.prev,
                    first: self.first,
                    close: true,
                })
            }
            None => None,
        }
    }
}

pub fn convert_path<'a>(p: &'a usvg::Path) -> PathConvIter<'a> {
    PathConvIter {
        iter: p.segments.iter(),
        first: Point::new(0.0, 0.0),
        prev: Point::new(0.0, 0.0),
        deferred_moveto: None,
        needs_end: false,
    }
}
