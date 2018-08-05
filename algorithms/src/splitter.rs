use math::*;
use geom::{Line, LineSegment};
use std::cmp::PartialOrd;
use advanced_path::*;
use std::mem;

#[derive(Debug)]
struct IntersectingEdge {
    intersection: Point,
    d: f32,
    id: EdgeId,
    split_edge: bool,
    positive: bool,
}

/// A context object that can split the sub paths of `AdvancedPath`.
pub struct Splitter {
    intersecting_edges: Vec<IntersectingEdge>,
}

impl Splitter {
    /// Constructor.
    pub fn new() -> Self {
        Splitter { intersecting_edges: Vec::new(), }
    }

    /// Split the selected portions of a path using a line segment.
    ///
    /// Returns the ids of the sub paths that were created in the process.
    pub fn split_with_segment(
        &mut self,
        path: &mut AdvancedPath,
        selection: &dyn SubPathSelection,
        segment: &LineSegment<f32>
    ) -> SubPathIdRange {
        let line = segment.to_line();
        self.intersecting_edges.clear();

        let v = segment.to_vector();

        // Find the edges that intersect the segment.
        path.for_each_edge_id(selection, &mut|path, _sub_path, edge_id| {
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

        self.split(&line, path)
    }

    /// Split the selected portions of a path using a line.
    ///
    /// Returns the ids of the sub paths that were created in the process.
    pub fn split_with_line(
        &mut self,
        path: &mut AdvancedPath,
        selection: &dyn SubPathSelection,
        line: &Line<f32>
    ) -> SubPathIdRange {
        self.intersecting_edges.clear();

        let v = line.vector;

        // Find the edges that intersect the segment.
        path.for_each_edge_id(selection, &mut|path, _sub_path, edge_id| {
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

        self.split(line, path)
    }

    fn split(&mut self, line: &Line<f32>, path: &mut AdvancedPath) -> SubPathIdRange {
        // Sort the intersecting edges along the segment.
        self.intersecting_edges.sort_by(|a, b| { a.d.partial_cmp(&b.d).unwrap() });

        let start_index = path.sub_path_ids().end;
        let mut new_sub_paths = SubPathIdRange::new(start_index..start_index);

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

                let mut last_side = 0.0;
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
                        last_side = d1.signum();
                    }
                    (Ty::OverlapAfter, _) => {
                        //
                        // ---x====---
                        //   / . . . .
                        edge_in = None;
                        last_side = -d1.signum();
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

        new_sub_paths
    }
}

fn signed_pseudo_distance(line: &Line<f32>, p: &Point) -> f32 {
    let v1 = line.point.to_vector();
    let v2 = v1 + line.vector;
    line.vector.cross(p.to_vector()) + v1.cross(v2)
}

#[cfg(test)]
use path::PathEvent;

#[cfg(test)]
fn compare_path_events(actual: &[PathEvent], expected: &[PathEvent]) {
    use geom::euclid::approxeq::ApproxEq;

    if actual.len() != expected.len() {
        panic!("error: lengths don't match\nexpected {:?}\ngot: {:?}", expected, actual);
    }
    for i in 0..expected.len() {
        let ok = match (actual[i], expected[i]) {
            (PathEvent::MoveTo(p1), PathEvent::MoveTo(p2)) => p1.approx_eq(&p2),
            (PathEvent::LineTo(p1), PathEvent::LineTo(p2)) => p1.approx_eq(&p2),
            (PathEvent::QuadraticTo(p1, ctrl1), PathEvent::QuadraticTo(p2, ctrl2)) => {
                p1.approx_eq(&p2) && ctrl1.approx_eq(&ctrl2)
            }
            (PathEvent::Close, PathEvent::Close) => true,
            _ => false,
        };

        if !ok {
            panic!("error:\nexpected {:?}\ngot: {:?}", expected, actual);
        }
    }
}

#[test]
fn split_with_segment_1() {
    use path::PathEvent;

    let mut path = AdvancedPath::new();
    let sp = path.add_polyline(
        &[
            point(0.0, 0.0),
            point(1.0, 0.0),
            point(1.0, 1.0),
            point(0.0, 1.0),
        ],
        true,
    );

    let mut splitter = Splitter::new();
    let new_sub_paths = splitter.split_with_segment(
        &mut path,
        &AllSubPaths,
        &LineSegment {
            from: point(-1.0, 0.5),
            to: point(2.0, 0.5),
        },
    );

    assert_eq!(new_sub_paths.len(), 1);
    let sp2 = new_sub_paths.nth(0);

    let events1: Vec<PathEvent> = path.sub_path_edges(sp).path_iter().collect();
    let events2: Vec<PathEvent> = path.sub_path_edges(sp2).path_iter().collect();

    compare_path_events(&events1, &[
        PathEvent::MoveTo(point(0.0, 0.5)),
        PathEvent::LineTo(point(1.0, 0.5)),
        PathEvent::LineTo(point(1.0, 1.0)),
        PathEvent::LineTo(point(0.0, 1.0)),
        PathEvent::Close,
    ]);

    compare_path_events(&events2, &[
        PathEvent::MoveTo(point(1.0, 0.5)),
        PathEvent::LineTo(point(0.0, 0.5)),
        PathEvent::LineTo(point(0.0, 0.0)),
        PathEvent::LineTo(point(1.0, 0.0)),
        PathEvent::Close,
    ]);
}

#[test]
fn split_with_segment_2() {
    use path::PathEvent;

    //  ________
    // |   __   |
    // |  |  |  |
    //------------
    // |__|  |__|
    //

    let mut path = AdvancedPath::new();
    let sp = path.add_polyline(
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
        true,
    );

    let mut splitter = Splitter::new();
    let new_sub_paths = splitter.split_with_segment(
        &mut path,
        &AllSubPaths,
        &LineSegment {
            from: point(-1.0, 2.0),
            to: point(4.0, 2.0),
        },
    );

    assert_eq!(new_sub_paths.len(), 2);
    let sp2 = new_sub_paths.nth(0);
    let sp3 = new_sub_paths.nth(1);

    let events1: Vec<PathEvent> = path.sub_path_edges(sp).path_iter().collect();

    compare_path_events(&events1, &[
        PathEvent::MoveTo(point(0.0, 2.0)),
        PathEvent::LineTo(point(1.0, 2.0)),
        PathEvent::LineTo(point(1.0, 3.0)),
        PathEvent::LineTo(point(0.0, 3.0)),
        PathEvent::Close,
    ]);

    let events2: Vec<PathEvent> = path.sub_path_edges(sp2).path_iter().collect();

    compare_path_events(&events2, &[
        PathEvent::MoveTo(point(2.0, 2.0)),
        PathEvent::LineTo(point(3.0, 2.0)),
        PathEvent::LineTo(point(3.0, 3.0)),
        PathEvent::LineTo(point(2.0, 3.0)),
        PathEvent::Close,
    ]);

    let events3: Vec<PathEvent> = path.sub_path_edges(sp3).path_iter().collect();

    compare_path_events(&events3, &[
        PathEvent::MoveTo(point(3.0, 2.0)),
        PathEvent::LineTo(point(2.0, 2.0)),
        PathEvent::LineTo(point(2.0, 1.0)),
        PathEvent::LineTo(point(1.0, 1.0)),
        PathEvent::LineTo(point(1.0, 2.0)),
        PathEvent::LineTo(point(0.0, 2.0)),
        PathEvent::LineTo(point(0.0, 0.0)),
        PathEvent::LineTo(point(3.0, 0.0)),
        PathEvent::Close,
    ]);
}

#[test]
fn split_with_segment_3() {
    use path::PathEvent;

    //  \____
    //  |\   |
    //  |_\__|
    //     \

    let mut path = AdvancedPath::new();
    let sp = path.add_polyline(
        &[
            point(0.0, 0.0),
            point(2.0, 0.0),
            point(2.0, 2.0),
            point(0.0, 2.0),
        ],
        true,
    );



    let mut splitter = Splitter::new();
    let new_sub_paths = splitter.split_with_segment(
        &mut path,
        &AllSubPaths,
        &LineSegment {
            from: point(-1.0, -2.0),
            to: point(2.0, 4.0),
        },
    );


    let events1: Vec<PathEvent> = path.sub_path_edges(sp).path_iter().collect();

    compare_path_events(&events1, &[
        PathEvent::MoveTo(point(0.0, 0.0)),
        PathEvent::LineTo(point(1.0, 2.0)),
        PathEvent::LineTo(point(0.0, 2.0)),
        PathEvent::Close,
    ]);

    let sp2 = new_sub_paths.nth(0);
    let events2: Vec<PathEvent> = path.sub_path_edges(sp2).path_iter().collect();

    compare_path_events(&events2, &[
        PathEvent::MoveTo(point(1.0, 2.0)),
        PathEvent::LineTo(point(0.0, 0.0)),
        PathEvent::LineTo(point(2.0, 0.0)),
        PathEvent::LineTo(point(2.0, 2.0)),
        PathEvent::Close,
    ]);
}

#[test]
fn split_with_segment_4() {
    use path::PathEvent;

    //  ________
    // |        |
    //-+--+-----+-
    // |__|
    //

    let mut path = AdvancedPath::new();
    let sp = path.add_polyline(
        &[
            point(0.0, 0.0),
            point(3.0, 0.0),
            point(3.0, 1.0),
            point(1.0, 1.0),
            point(1.0, 2.0),
            point(0.0, 2.0),
        ],
        true,
    );

    let mut splitter = Splitter::new();
    let new_sub_paths = splitter.split_with_segment(
        &mut path,
        &AllSubPaths,
        &LineSegment {
            from: point(-1.0, 1.0),
            to: point(4.0, 1.0),
        },
    );

    assert_eq!(new_sub_paths.len(), 1);

    let events1: Vec<PathEvent> = path.sub_path_edges(sp).path_iter().collect();

    compare_path_events(&events1, &[
        PathEvent::MoveTo(point(0.0, 1.0)),
        PathEvent::LineTo(point(1.0, 1.0)),
        PathEvent::LineTo(point(1.0, 2.0)),
        PathEvent::LineTo(point(0.0, 2.0)),
        PathEvent::Close,
    ]);

    let sp2 = new_sub_paths.nth(0);
    let events2: Vec<PathEvent> = path.sub_path_edges(sp2).path_iter().collect();

    compare_path_events(&events2, &[
        PathEvent::MoveTo(point(1.0, 1.0)),
        PathEvent::LineTo(point(0.0, 1.0)),
        PathEvent::LineTo(point(0.0, 0.0)),
        PathEvent::LineTo(point(3.0, 0.0)),
        PathEvent::LineTo(point(3.0, 1.0)),
        PathEvent::Close,
    ]);
}

#[test]
fn split_with_segment_5() {
    use path::PathEvent;

    //  ________
    // |        |
    //-+-----+--+-
    //       |__|
    //

    let mut path = AdvancedPath::new();
    let sp = path.add_polyline(
        &[
            point(0.0, 0.0),
            point(3.0, 0.0),
            point(3.0, 2.0),
            point(2.0, 2.0),
            point(2.0, 1.0),
            point(0.0, 1.0),
        ],
        true,
    );

    let mut splitter = Splitter::new();
    let new_sub_paths = splitter.split_with_segment(
        &mut path,
        &AllSubPaths,
        &LineSegment {
            from: point(-1.0, 1.0),
            to: point(4.0, 1.0),
        },
    );

    assert_eq!(new_sub_paths.len(), 1);

    let events1: Vec<PathEvent> = path.sub_path_edges(sp).path_iter().collect();
    let sp2 = new_sub_paths.nth(0);
    let events2: Vec<PathEvent> = path.sub_path_edges(sp2).path_iter().collect();

    compare_path_events(&events1, &[
        PathEvent::MoveTo(point(2.0, 1.0)),
        PathEvent::LineTo(point(3.0, 1.0)),
        PathEvent::LineTo(point(3.0, 2.0)),
        PathEvent::LineTo(point(2.0, 2.0)),
        PathEvent::Close
    ]);

    compare_path_events(&events2, &[
        PathEvent::MoveTo(point(3.0, 1.0)),
        PathEvent::LineTo(point(2.0, 1.0)),
        PathEvent::LineTo(point(0.0, 1.0)),
        PathEvent::LineTo(point(0.0, 0.0)),
        PathEvent::LineTo(point(3.0, 0.0)),
        PathEvent::Close,
    ]);
}

#[test]
fn split_with_segment_6() {
    use path::PathEvent;

    //  ________
    // |        |
    //-+--+--+--+-
    // |__|  |__|
    //

    let mut path = AdvancedPath::new();
    let sp = path.add_polyline(
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
        true,
    );

    let mut splitter = Splitter::new();
    let new_sub_paths = splitter.split_with_segment(
        &mut path,
        &AllSubPaths,
        &LineSegment {
            from: point(-1.0, 1.0),
            to: point(4.0, 1.0),
        },
    );

    assert_eq!(new_sub_paths.len(), 2);

    let events1: Vec<PathEvent> = path.sub_path_edges(sp).path_iter().collect();

    compare_path_events(&events1, &[
        PathEvent::MoveTo(point(0.0, 1.0)),
        PathEvent::LineTo(point(1.0, 1.0)),
        PathEvent::LineTo(point(1.0, 2.0)),
        PathEvent::LineTo(point(0.0, 2.0)),
        PathEvent::Close,
    ]);

    let sp2 = new_sub_paths.nth(0);
    let events2: Vec<PathEvent> = path.sub_path_edges(sp2).path_iter().collect();

    compare_path_events(&events2, &[
        PathEvent::MoveTo(point(2.0, 1.0)),
        PathEvent::LineTo(point(3.0, 1.0)),
        PathEvent::LineTo(point(3.0, 2.0)),
        PathEvent::LineTo(point(2.0, 2.0)),
        PathEvent::Close,
    ]);

    let sp3 = new_sub_paths.nth(1);
    let events3: Vec<PathEvent> = path.sub_path_edges(sp3).path_iter().collect();

    compare_path_events(&events3, &[
        PathEvent::MoveTo(point(3.0, 1.0)),
        PathEvent::LineTo(point(2.0, 1.0)),
        PathEvent::LineTo(point(1.0, 1.0)),
        PathEvent::LineTo(point(0.0, 1.0)),
        PathEvent::LineTo(point(0.0, 0.0)),
        PathEvent::LineTo(point(3.0, 0.0)),
        PathEvent::Close,
    ]);
}
