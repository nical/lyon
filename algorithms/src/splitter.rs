/// Split paths with a line or line segment.

use crate::math::*;
use crate::geom::{Line, LineSegment};
use crate::advanced_path::*;
use crate::path::*;
use crate::path::iterator::PathIterator;
use crate::path::builder::PathBuilder;
use std::cmp::PartialOrd;
use std::mem;

#[derive(Debug)]
struct IntersectingEdge {
    intersection: Point,
    d: f32,
    id: EdgeId,
    split_edge: bool,
    positive: bool,
}

/// A context object that can split paths.
///
/// When splitting several paths, reusing the `Splitter` object can improve performance
/// by reusing memory allocations.
pub struct Splitter {
    intersecting_edges: Vec<IntersectingEdge>,
    point_buffer: Vec<Point>,
    flattening_tolerance: f32,
}

impl Splitter {
    /// Constructor.
    pub fn new() -> Self {
        Splitter {
            intersecting_edges: Vec::new(),
            point_buffer: Vec::new(),
            flattening_tolerance: 0.1,
        }
    }

    /// Sets the flattening tolerance that will be used to approximate curves
    /// if any.
    pub fn set_flattening_tolerance(&mut self, tolerance: f32) {
        self.flattening_tolerance = tolerance;
    }

    /// Splits a path using a line segment.
    ///
    /// Returns two `Path` objects, the first one being on the positive
    /// side of the line, and the other one on the negative side.
    ///
    /// "positive" and "negative" in this context refer to the sign of the
    /// cross product between a vector going from the splitting line to the
    /// path and the vector of the splitting line.
    ///
    /// Curves are flattened.
    pub fn split_with_segment<'l, P>(&mut self, path: P, segment: &LineSegment<f32>) -> (Path, Path)
    where P: Into<PathSlice<'l>> {
        self.split_with_segment_impl(path.into(), segment)
    }

    fn split_with_segment_impl(
        &mut self,
        path_slice: PathSlice,
        segment: &LineSegment<f32>
    ) -> (Path, Path) {
        let line = segment.to_line();
        self.intersecting_edges.clear();

        let mut path = AdvancedPath::new();
        self.to_advanced_path(path_slice, &mut path);

        let v = segment.to_vector();

        // Find the edges that intersect the segment.
        path.for_each_edge_id(&AllSubPaths, &mut|path, _sub_path, edge_id| {
            let edge = path.edge(edge_id);
            let edge_segment = LineSegment {
                from: path[edge.from],
                to: path[edge.to],
            };
            if let Some((t, _)) = edge_segment.intersection_t(&segment) {
                if t < 1.0 {
                    let intersection = edge_segment.sample(t);
                    let prev_vertex = path[path.edge_from(path.previous_edge_id(edge_id))];
                    let positive = (prev_vertex - intersection).dot(v) <= (edge_segment.to - intersection).dot(v);
                    self.intersecting_edges.push(IntersectingEdge {
                        intersection,
                        id: edge_id,
                        d: v.dot(intersection - segment.from),
                        split_edge: t > 0.0,
                        positive,
                    });
                }
            } else if segment.contains_segment(&edge_segment) {
                let positive = edge_segment.to_vector().dot(segment.to_vector()) > 0.0;
                let intersection = edge_segment.from;
                self.intersecting_edges.push(IntersectingEdge {
                    intersection,
                    id: edge_id,
                    d: v.dot(intersection - segment.from),
                    split_edge: false,
                    positive,
                });
            }
        });

        self.split(&line, &mut path)
    }

    /// Splits a path using a line.
    ///
    /// Returns two `Path` objects, the first one being on the positive
    /// side of the line, and the other one on the negative side.
    ///
    /// "positive" and "negative" in this context refer to the sign of the
    /// cross product between a vector going from the splitting line to the
    /// path and the vector of the splitting line.
    ///
    /// Curves are flattened.
    pub fn split_with_line<'l, P>(&mut self, path: P, line: &Line<f32>) -> (Path, Path)
    where P: Into<PathSlice<'l>> {
        self.split_with_line_impl(path.into(), line)
    }

    fn split_with_line_impl(
        &mut self,
        path_slice: PathSlice,
        line: &Line<f32>
    ) -> (Path, Path) {
        self.intersecting_edges.clear();
        let mut path = AdvancedPath::new();
        self.to_advanced_path(path_slice, &mut path);

        let v = line.vector;

        // Find the edges that intersect the segment.
        path.for_each_edge_id(&AllSubPaths, &mut|path, _sub_path, edge_id| {
            let edge = path.edge(edge_id);
            let edge_segment = LineSegment {
                from: path[edge.from],
                to: path[edge.to],
            };

            if let Some(t) = edge_segment.line_intersection_t(line) {
                if t < 1.0 {
                    let intersection = edge_segment.sample(t);
                    let prev_vertex = path[path.edge_from(path.previous_edge_id(edge_id))];
                    let positive = (prev_vertex - intersection).dot(v) <= (edge_segment.to - intersection).dot(v);
                    self.intersecting_edges.push(IntersectingEdge {
                        intersection,
                        id: edge_id,
                        d: v.dot(intersection - line.point),
                        split_edge: t > 0.0 && t < 1.0,
                        positive,
                    });
                }
            } else if edge_segment.overlaps_line(line) {
                let positive = edge_segment.to_vector().dot(line.vector) > 0.0;
                let intersection = edge_segment.from;
                self.intersecting_edges.push(IntersectingEdge {
                    intersection,
                    id: edge_id,
                    d: v.dot(intersection - line.point),
                    split_edge: false,
                    positive,
                });
            }
        });

        self.split(line, &mut path)
    }

    fn split(&mut self, line: &Line<f32>, path: &mut AdvancedPath) -> (Path, Path) {
        // Sort the intersecting edges along the segment.
        self.intersecting_edges.sort_by(|a, b| { a.d.partial_cmp(&b.d).unwrap() });

        let start_index = path.sub_path_ids().end;
        let mut new_sub_paths = SubPathIdRange::new(start_index..start_index);

        let mut last_side = 0.0;
        let mut edge_in = None;
        for i in 0..self.intersecting_edges.len() {
            let e = &self.intersecting_edges[i];
            if e.split_edge {
                // The common case.

                path.split_edge(e.id, e.intersection);
                if let Some(e_in) = edge_in {
                    // ..\
                    // ---\---
                    // ....\
                    let e_out = path.next_edge_id(e.id);
                    if let Some(sub_path) = path.connect_edges(e_in, e_out) {
                        debug_assert!(sub_path.handle == new_sub_paths.end);
                        new_sub_paths.end += 1;
                    }
                    edge_in = None;
                } else {
                    //   \....
                    // ---\---
                    //     \..
                    edge_in = Some(e.id);
                }
            } else {
                // The uncommon and ugly cases.

                #[derive(Debug)]
                enum Ty {
                    SameSide,
                    DifferentSide,
                    OverlapBefore,
                    OverlapAfter,
                }

                let prev = path.edge_from(path.previous_edge_id(e.id));
                let next = path.edge_from(path.next_edge_id(e.id));
                let prev_point = path[prev];
                let next_point = path[next];
                let mut d1 = signed_pseudo_distance(line, &prev_point);
                let mut d2 = signed_pseudo_distance(line, &next_point);

                if d1 == 0.0 && d2 == 0.0 {
                    continue;
                }

                if !e.positive {
                    mem::swap(&mut d1, &mut d2);
                }

                let configuration = if d1 == 0.0 {
                    Ty::OverlapBefore
                } else if d2 == 0.0 {
                    Ty::OverlapAfter
                } else if d1.signum() == d2.signum() {
                    Ty::SameSide
                } else {
                    Ty::DifferentSide
                };

                match (configuration, edge_in) {
                    (Ty::SameSide, Some(e_in)) => {
                        // .\   /.
                        // ..\ /..
                        // ---x---
                        //
                        // Inside of the shape.
                        // Connect both left and right.
                        if let Some(sub_path) = path.connect_edges(e_in, e.id) {
                            debug_assert!(sub_path.handle == new_sub_paths.end);
                            new_sub_paths.end += 1;
                        }
                        edge_in = Some(path.previous_edge_id(e.id));
                    }
                    (Ty::SameSide, None) => {
                        //  \.../
                        //   \./
                        // ---x---
                        //
                        // Outside of the shape, nothing to do.
                    }
                    (Ty::DifferentSide, Some(e_in)) => {
                        // ..\
                        // ---x---
                        // ../
                        if let Some(sub_path) = path.connect_edges(e_in, e.id) {
                            debug_assert!(sub_path.handle == new_sub_paths.end);
                            new_sub_paths.end += 1;
                        }
                        edge_in = None;
                    }
                    (Ty::DifferentSide, None) => {
                        //   \....
                        // ---x---
                        //   /....
                        edge_in = Some(path.previous_edge_id(e.id));
                    }
                    (Ty::OverlapAfter, Some(e_in)) => {
                        // . . . . . .
                        // ---x====---
                        // . /
                        if let Some(sub_path) = path.connect_edges(e_in, e.id) {
                            debug_assert!(sub_path.handle == new_sub_paths.end);
                            new_sub_paths.end += 1;
                        }
                        edge_in = None;
                        last_side = -d1.signum();
                    }
                    (Ty::OverlapAfter, _) => {
                        //
                        // ---x====---
                        //   / . . . .
                        edge_in = None;
                        last_side = d1.signum();
                    }
                    (Ty::OverlapBefore, _) => {
                        // . . . . . .
                        // ---====x---
                        //         \ .
                        //
                        // or
                        //
                        // ---====x---
                        // . . . . \
                        //
                        let side = d2.signum();
                        if side == last_side {
                            // Transitioning out
                            edge_in = None;
                        } else {
                            // Transitioning in
                            edge_in = Some(path.previous_edge_id(e.id));
                        }
                    }
                }
            }
        }

        from_advanced_path(&path, line)
    }

    fn to_advanced_path(&mut self, path: PathSlice, adv: &mut AdvancedPath) {
        self.point_buffer.clear();
        for evt in path.iter().flattened(self.flattening_tolerance) {
            match evt {
                PathEvent::Begin { at } => {
                    debug_assert!(self.point_buffer.is_empty());
                    self.point_buffer.clear();
                    self.point_buffer.push(at)
                }
                PathEvent::Line { to, .. } => {
                    self.point_buffer.push(to)
                }
                PathEvent::End { close, .. } => {
                    if self.point_buffer.len() > 2 {
                        adv.add_polyline(&self.point_buffer, close);
                    }
                    self.point_buffer.clear();
                }
                _ => { panic!(); }
            }
        }
    }
}

fn from_advanced_path(adv: &AdvancedPath, line: &Line<f32>)-> (Path, Path) {
    let mut p1 = Path::builder();
    let mut p2 = Path::builder();
    adv.for_each_sub_path_id(&AllSubPaths, &mut|adv, id| {
        let edges = adv.sub_path_edges(id);

        // Figure out which side of the line the edge loop is on.
        let mut e2 = edges.clone();
        let mut center = point(0.0, 0.0);
        let mut div = 0.0;
        loop {
            center += adv[adv.edge_from(e2.current())].to_vector();
            div += 1.0;

            if !e2.move_forward() {
                break;
            }
        }

        center /= div;

        let is_p1 = (center - line.point).cross(line.vector) >= 0.0;

        let path = if is_p1 { &mut p1 } else { &mut p2 };

        for evt in edges.path_iter() {
            path.path_event(evt);
        }
    });

    (p1.build(), p2.build())
}

fn signed_pseudo_distance(line: &Line<f32>, p: &Point) -> f32 {
    let v1 = line.point.to_vector();
    let v2 = v1 + line.vector;
    line.vector.cross(p.to_vector()) + v1.cross(v2)
}

#[cfg(test)]
use crate::path::PathEvent;

#[cfg(test)]
fn compare_path_events(actual: &[PathEvent<Point, Point>], expected: &[PathEvent<Point, Point>]) {
    use crate::geom::euclid::approxeq::ApproxEq;

    if actual.len() != expected.len() {
        panic!("error: lengths don't match\nexpected {:?}\ngot: {:?}", expected, actual);
    }
    for i in 0..expected.len() {
        let ok = match (actual[i], expected[i]) {
            (
                PathEvent::Begin{ at: p1 },
                PathEvent::Begin { at: p2 }
            ) => p1.approx_eq(&p2),
            (
                PathEvent::End { last: l1, first: f1, close: c1 },
                PathEvent::End { last: l2, first: f2, close: c2 }
            ) => c1 == c2 && l1.approx_eq(&l2) && f1.approx_eq(&f2),
            (
                PathEvent::Line { from: f1, to: t1 },
                PathEvent::Line { from: f2, to: t2 }
            ) => f1.approx_eq(&f2) && t1.approx_eq(&t2),
            (
                PathEvent::Quadratic { from: f1, ctrl: c1, to: t1 },
                PathEvent::Quadratic { from: f2, ctrl: c2, to: t2 }
            ) => f1.approx_eq(&f2) && c1.approx_eq(&c2) && t1.approx_eq(&t2),
            (
                PathEvent::Cubic { from: f1, ctrl1: c11, ctrl2: c21, to: t1 },
                PathEvent::Cubic { from: f2, ctrl1: c12, ctrl2: c22, to: t2 }
            ) => f1.approx_eq(&f2) && c11.approx_eq(&c12) && c21.approx_eq(&c22) && t1.approx_eq(&t2),
            _ => false
        };

        if !ok {
            panic!("error:\nexpected {:?}\ngot: {:?}", expected, actual);
        }
    }
}

#[test]
fn split_with_segment_1() {
    use crate::path::PathEvent;

    let mut path = Path::builder();
    path.polygon(
        &[
            point(0.0, 0.0),
            point(1.0, 0.0),
            point(1.0, 1.0),
            point(0.0, 1.0),
        ],
    );

    let mut splitter = Splitter::new();
    let (p1, p2) = splitter.split_with_segment(
        path.build().as_slice(),
        &LineSegment {
            from: point(-1.0, 0.5),
            to: point(2.0, 0.5),
        },
    );

    let events1: Vec<PathEvent<Point, Point>> = p1.iter().collect();
    let events2: Vec<PathEvent<Point, Point>> = p2.iter().collect();

    compare_path_events(&events1, &[
        PathEvent::Begin { at: point(1.0, 0.5) },
        PathEvent::Line { from: point(1.0, 0.5), to: point(0.0, 0.5) },
        PathEvent::Line { from: point(0.0, 0.5), to: point(0.0, 0.0) },
        PathEvent::Line { from: point(0.0, 0.0), to: point(1.0, 0.0) },
        PathEvent::End { last: point(1.0, 0.0), first: point(1.0, 0.5), close: true },
    ]);

    compare_path_events(&events2, &[
        PathEvent::Begin { at: point(0.0, 0.5) },
        PathEvent::Line { from: point(0.0, 0.5), to: point(1.0, 0.5) },
        PathEvent::Line { from: point(1.0, 0.5), to: point(1.0, 1.0) },
        PathEvent::Line { from: point(1.0, 1.0), to: point(0.0, 1.0) },
        PathEvent::End { last: point(0.0, 1.0), first: point(0.0, 0.5), close: true },
    ]);
}

#[test]
fn split_with_segment_2() {
    use crate::path::PathEvent;

    //  ________
    // |   __   |
    // |  |  |  |
    //------------
    // |__|  |__|
    //

    let mut path = Path::builder();
    path.polygon(
        &[
            point(0.0, 0.0),
            point(3.0, 0.0),
            point(3.0, 3.0),
            point(2.0, 3.0),
            point(2.0, 1.0),
            point(1.0, 1.0),
            point(1.0, 3.0),
            point(0.0, 3.0),
        ],
    );

    let mut splitter = Splitter::new();
    let (p1, p2) = splitter.split_with_segment(
        path.build().as_slice(),
        &LineSegment {
            from: point(-1.0, 2.0),
            to: point(4.0, 2.0),
        },
    );

    let events1: Vec<PathEvent<Point, Point>> = p1.iter().collect();
    let events2: Vec<PathEvent<Point, Point>> = p2.iter().collect();

    compare_path_events(&events1, &[
        PathEvent::Begin { at: point(3.0, 2.0) },
        PathEvent::Line { from: point(3.0, 2.0), to: point(2.0, 2.0) },
        PathEvent::Line { from: point(2.0, 2.0), to: point(2.0, 1.0) },
        PathEvent::Line { from: point(2.0, 1.0), to: point(1.0, 1.0) },
        PathEvent::Line { from: point(1.0, 1.0), to: point(1.0, 2.0) },
        PathEvent::Line { from: point(1.0, 2.0), to: point(0.0, 2.0) },
        PathEvent::Line { from: point(0.0, 2.0), to: point(0.0, 0.0) },
        PathEvent::Line { from: point(0.0, 0.0), to: point(3.0, 0.0) },
        PathEvent::End { last: point(3.0, 0.0), first: point(3.0, 2.0), close: true },
    ]);

    compare_path_events(&events2, &[
        PathEvent::Begin { at: point(0.0, 2.0) },
        PathEvent::Line { from: point(0.0, 2.0), to: point(1.0, 2.0) },
        PathEvent::Line { from: point(1.0, 2.0), to: point(1.0, 3.0) },
        PathEvent::Line { from: point(1.0, 3.0), to: point(0.0, 3.0) },
        PathEvent::End { last: point(0.0, 3.0), first: point(0.0, 2.0), close: true },
        PathEvent::Begin { at: point(2.0, 2.0) },
        PathEvent::Line { from: point(2.0, 2.0), to: point(3.0, 2.0) },
        PathEvent::Line { from: point(3.0, 2.0), to: point(3.0, 3.0) },
        PathEvent::Line { from: point(3.0, 3.0), to: point(2.0, 3.0) },
        PathEvent::End { last: point(2.0, 3.0), first: point(2.0, 2.0), close: true },
    ]);
}

#[test]
fn split_with_segment_3() {
    use crate::path::PathEvent;

    //  \____
    //  |\   |
    //  |_\__|
    //     \

    let mut path = Path::builder();
    path.polygon(
        &[
            point(0.0, 0.0),
            point(2.0, 0.0),
            point(2.0, 2.0),
            point(0.0, 2.0),
        ],
    );



    let mut splitter = Splitter::new();
    let (p1, p2) = splitter.split_with_segment(
        path.build().as_slice(),
        &LineSegment {
            from: point(-1.0, -2.0),
            to: point(2.0, 4.0),
        },
    );


    let events1: Vec<PathEvent<Point, Point>> = p1.iter().collect();
    let events2: Vec<PathEvent<Point, Point>> = p2.iter().collect();

    compare_path_events(&events1, &[
        PathEvent::Begin { at: point(1.0, 2.0) },
        PathEvent::Line { from: point(1.0, 2.0), to: point(0.0, 0.0) },
        PathEvent::Line { from: point(0.0, 0.0), to: point(2.0, 0.0) },
        PathEvent::Line { from: point(2.0, 0.0), to: point(2.0, 2.0) },
        PathEvent::End { last: point(2.0, 2.0), first: point(1.0, 2.0), close: true },
    ]);

    compare_path_events(&events2, &[
        PathEvent::Begin { at: point(0.0, 0.0) },
        PathEvent::Line { from: point(0.0, 0.0), to: point(1.0, 2.0) },
        PathEvent::Line { from: point(1.0, 2.0), to: point(0.0, 2.0) },
        PathEvent::End { last: point(0.0, 2.0), first: point(0.0, 0.0), close: true },
    ]);
}


#[test]
fn split_with_segment_4() {
    use crate::path::PathEvent;

    //  ________
    // |        |
    //-+--+-----+-
    // |__|
    //

    let mut path = Path::builder();
    path.polygon(
        &[
            point(0.0, 0.0),
            point(3.0, 0.0),
            point(3.0, 1.0),
            point(1.0, 1.0),
            point(1.0, 2.0),
            point(0.0, 2.0),
        ],
    );

    let mut splitter = Splitter::new();
    let (p1, p2) = splitter.split_with_segment(
        path.build().as_slice(),
        &LineSegment {
            from: point(-1.0, 1.0),
            to: point(4.0, 1.0),
        },
    );

    let events1: Vec<PathEvent<Point, Point>> = p1.iter().collect();
    let events2: Vec<PathEvent<Point, Point>> = p2.iter().collect();

    compare_path_events(&events1, &[
        PathEvent::Begin { at: point(1.0, 1.0) },
        PathEvent::Line { from: point(1.0, 1.0), to: point(0.0, 1.0) },
        PathEvent::Line { from: point(0.0, 1.0), to: point(0.0, 0.0) },
        PathEvent::Line { from: point(0.0, 0.0), to: point(3.0, 0.0) },
        PathEvent::Line { from: point(3.0, 0.0), to: point(3.0, 1.0) },
        PathEvent::End { last: point(3.0, 1.0), first: point(1.0, 1.0), close: true },
    ]);

    compare_path_events(&events2, &[
        PathEvent::Begin { at: point(0.0, 1.0) },
        PathEvent::Line { from: point(0.0, 1.0), to: point(1.0, 1.0) },
        PathEvent::Line { from: point(1.0, 1.0), to: point(1.0, 2.0) },
        PathEvent::Line { from: point(1.0, 2.0), to: point(0.0, 2.0) },
        PathEvent::End { last: point(0.0, 2.0), first: point(0.0, 1.0), close: true },
    ]);
}

#[test]
fn split_with_segment_5() {
    use crate::path::PathEvent;

    //  ________
    // |        |
    //-+-----+--+-
    //       |__|
    //

    let mut path = Path::builder();
    path.polygon(
        &[
            point(0.0, 0.0),
            point(3.0, 0.0),
            point(3.0, 2.0),
            point(2.0, 2.0),
            point(2.0, 1.0),
            point(0.0, 1.0),
        ],
    );

    let mut splitter = Splitter::new();
    let (p1, p2) = splitter.split_with_segment(
        path.build().as_slice(),
        &LineSegment {
            from: point(-1.0, 1.0),
            to: point(4.0, 1.0),
        },
    );

    let events1: Vec<PathEvent<Point, Point>> = p1.iter().collect();
    let events2: Vec<PathEvent<Point, Point>> = p2.iter().collect();


    compare_path_events(&events1, &[
        PathEvent::Begin { at: point(3.0, 1.0) },
        PathEvent::Line { from: point(3.0, 1.0), to: point(2.0, 1.0) },
        PathEvent::Line { from: point(2.0, 1.0), to: point(0.0, 1.0) },
        PathEvent::Line { from: point(0.0, 1.0), to: point(0.0, 0.0) },
        PathEvent::Line { from: point(0.0, 0.0), to: point(3.0, 0.0) },
        PathEvent::End { last: point(3.0, 0.0), first: point(3.0, 1.0), close: true },
    ]);

    compare_path_events(&events2, &[
        PathEvent::Begin { at: point(2.0, 1.0) },
        PathEvent::Line { from: point(2.0, 1.0), to: point(3.0, 1.0) },
        PathEvent::Line { from: point(3.0, 1.0), to: point(3.0, 2.0) },
        PathEvent::Line { from: point(3.0, 2.0), to: point(2.0, 2.0) },
        PathEvent::End { last: point(2.0, 2.0), first: point(2.0, 1.0), close: true },
    ]);
}

#[test]
fn split_with_segment_6() {
    use crate::path::PathEvent;

    //  ________
    // |        |
    //-+--+--+--+-
    // |__|  |__|
    //

    let mut path = Path::builder();
    path.polygon(
        &[
            point(0.0, 0.0),
            point(3.0, 0.0),
            point(3.0, 2.0),
            point(2.0, 2.0),
            //point(2.5, 2.0),
            point(2.0, 1.0),
            point(1.0, 1.0),
            point(1.0, 2.0),
            point(0.0, 2.0),
        ],
    );

    let mut splitter = Splitter::new();
    let (p1, p2) = splitter.split_with_segment(
        path.build().as_slice(),
        &LineSegment {
            from: point(-1.0, 1.0),
            to: point(4.0, 1.0),
        },
    );

    let events1: Vec<PathEvent<Point, Point>> = p1.iter().collect();
    let events2: Vec<PathEvent<Point, Point>> = p2.iter().collect();

    compare_path_events(&events1, &[
        PathEvent::Begin { at: point(3.0, 1.0) },
        PathEvent::Line { from: point(3.0, 1.0), to: point(2.0, 1.0) },
        PathEvent::Line { from: point(2.0, 1.0), to: point(1.0, 1.0) },
        PathEvent::Line { from: point(1.0, 1.0), to: point(0.0, 1.0) },
        PathEvent::Line { from: point(0.0, 1.0), to: point(0.0, 0.0) },
        PathEvent::Line { from: point(0.0, 0.0), to: point(3.0, 0.0) },
        PathEvent::End { last: point(3.0, 0.0), first: point(3.0, 1.0), close: true },
    ]);

    compare_path_events(&events2, &[
        PathEvent::Begin { at: point(0.0, 1.0) },
        PathEvent::Line { from: point(0.0, 1.0), to: point(1.0, 1.0) },
        PathEvent::Line { from: point(1.0, 1.0), to: point(1.0, 2.0) },
        PathEvent::Line { from: point(1.0, 2.0), to: point(0.0, 2.0) },
        PathEvent::End { last: point(0.0, 2.0), first: point(0.0, 1.0), close: true },
        PathEvent::Begin { at: point(2.0, 1.0) },
        PathEvent::Line { from: point(2.0, 1.0), to: point(3.0, 1.0) },
        PathEvent::Line { from: point(3.0, 1.0), to: point(3.0, 2.0) },
        PathEvent::Line { from: point(3.0, 2.0), to: point(2.0, 2.0) },
        PathEvent::End { last: point(2.0, 2.0), first: point(2.0, 1.0), close: true },
    ]);
}
