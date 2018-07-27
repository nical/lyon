use math::*;
use geom::{Line, LineSegment};
use std::cmp::PartialOrd;
use advanced_path::*;

#[derive(Debug)]
struct IntersectingEdge {
    intersection: Point,
    id: EdgeId,
    d: f32,
    t: f32,
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
    ) -> Vec<SubPathId> {
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
                    self.intersecting_edges.push(IntersectingEdge {
                        intersection,
                        id: edge_id,
                        d: v.dot(intersection - segment.from),
                        t,
                    });
                }
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
    ) -> Vec<SubPathId> {
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
                    self.intersecting_edges.push(IntersectingEdge {
                        intersection,
                        id: edge_id,
                        d: v.dot(intersection - line.point),
                        t,
                    });
                }
            }
        });

        self.split(line, path)
    }

    fn split(&mut self, line: &Line<f32>, path: &mut AdvancedPath) -> Vec<SubPathId> {
        // Sort the intersecting edges along the segment.
        self.intersecting_edges.sort_by(|a, b| { a.d.partial_cmp(&b.d).unwrap() });

        let mut new_sub_paths = Vec::new();

        let mut edge_in = None;
        for i in 0..self.intersecting_edges.len() {
            let e = &self.intersecting_edges[i];
            if e.t == 0.0 {
                let prev = path.edge_from(path.previous_edge_id(e.id));
                let next = path.edge_from(path.next_edge_id(e.id));
                let d1 = signed_pseudo_distance(line, &path[prev]);
                let d2 = signed_pseudo_distance(line, &path[next]);
                let same_side = d1.signum() == d2.signum();
                match (same_side, edge_in) {
                    (true, Some(e_in)) => {
                        // .\   /.
                        // ..\ /..
                        // ---x---
                        //
                        // Inside of the shape.
                        // Connect both left and right.
                        if let Some(sub_path) = path.connect_edges(e_in, e.id) {
                            new_sub_paths.push(sub_path);
                        }

                        edge_in = Some(path.previous_edge_id(e.id));
                    }
                    (true, None) => {
                        //  \.../
                        //   \./
                        // ---x---
                        //
                        // Outside of the shape, nothing to do.
                    }
                    (false, Some(e_in)) => {
                        // ..\
                        // ---x---
                        // ../
                        if let Some(sub_path) = path.connect_edges(e_in, e.id) {
                            new_sub_paths.push(sub_path);
                        }
                        edge_in = None;
                    }
                    (false, None) => {
                        //   \....
                        // ---x---
                        //   /....
                        edge_in = Some(path.previous_edge_id(e.id));
                    }
                }
            } else {
                path.split_edge(e.id, e.intersection);
                if let Some(e_in) = edge_in {
                    // ..\
                    // ---\---
                    // ....\
                    let e_out = path.next_edge_id(e.id);
                    if let Some(sub_path) = path.connect_edges(e_in, e_out) {
                        new_sub_paths.push(sub_path);
                    }
                    edge_in = None;
                } else {
                    //   \....
                    // ---\---
                    //     \..
                    edge_in = Some(e.id);
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

#[test]
fn split_with_segment_1() {
    use geom::euclid::approxeq::ApproxEq;
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
    let sp2 = new_sub_paths[0];

    let events1: Vec<PathEvent> = path.sub_path_edges(sp).path_iter().collect();
    let events2: Vec<PathEvent> = path.sub_path_edges(sp2).path_iter().collect();

    if let PathEvent::MoveTo(p) = events1[0] {
        assert!(p.approx_eq(&point(0.0, 0.5)))
    } else {
        panic!("unexpected event {:?}", events1[1]);
    }
    if let PathEvent::LineTo(p) = events1[1] {
        assert!(p.approx_eq(&point(1.0, 0.5)))
    } else {
        panic!("unexpected event {:?}", events1[1]);
    }
    assert_eq!(events1[2], PathEvent::LineTo(point(1.0, 1.0)));
    assert_eq!(events1[3], PathEvent::LineTo(point(0.0, 1.0)));
    assert_eq!(events1[4], PathEvent::Close);
    assert_eq!(events1.len(), 5);


    if let PathEvent::MoveTo(p) = events2[0] {
        assert!(p.approx_eq(&point(1.0, 0.5)))
    } else {
        panic!("unexpected event {:?}", events1[1]);
    }
    if let PathEvent::LineTo(p) = events2[1] {
        assert!(p.approx_eq(&point(0.0, 0.5)))
    } else {
        panic!("unexpected event {:?}", events1[1]);
    }
    assert_eq!(events2[2], PathEvent::LineTo(point(0.0, 0.0)));
    assert_eq!(events2[3], PathEvent::LineTo(point(1.0, 0.0)));
    assert_eq!(events2[4], PathEvent::Close);
    assert_eq!(events2.len(), 5);
}

#[test]
fn split_with_segment_2() {
    use geom::euclid::approxeq::ApproxEq;
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
    let sp2 = new_sub_paths[0];
    let sp3 = new_sub_paths[1];

    let events1: Vec<PathEvent> = path.sub_path_edges(sp).path_iter().collect();

    assert_eq!(events1[0], PathEvent::MoveTo(point(2.0, 2.0)));
    assert_eq!(events1[1], PathEvent::LineTo(point(3.0, 2.0)));
    assert_eq!(events1[2], PathEvent::LineTo(point(3.0, 3.0)));
    assert_eq!(events1[3], PathEvent::LineTo(point(2.0, 3.0)));
    assert_eq!(events1[4], PathEvent::Close);

    let events2: Vec<PathEvent> = path.sub_path_edges(sp2).path_iter().collect();

    assert_eq!(events2[0], PathEvent::MoveTo(point(1.0, 2.0)));
    if let PathEvent::LineTo(p) = events2[1] {
        assert!(p.approx_eq(&point(0.0, 2.0)))
    } else {
        panic!("unexpected event {:?}", events2[1]);
    }
    assert_eq!(events2[2], PathEvent::LineTo(point(0.0, 0.0)));
    assert_eq!(events2[3], PathEvent::LineTo(point(3.0, 0.0)));
    assert_eq!(events2[4], PathEvent::LineTo(point(3.0, 2.0)));
    assert_eq!(events2[5], PathEvent::LineTo(point(2.0, 2.0)));
    assert_eq!(events2[6], PathEvent::LineTo(point(2.0, 1.0)));
    assert_eq!(events2[7], PathEvent::LineTo(point(1.0, 1.0)));
    assert_eq!(events2[8], PathEvent::Close);

    let events3: Vec<PathEvent> = path.sub_path_edges(sp3).path_iter().collect();

    assert_eq!(events3[0], PathEvent::MoveTo(point(3.0, 2.0)));
    assert_eq!(events3[1], PathEvent::LineTo(point(2.0, 2.0)));
    assert_eq!(events3[2], PathEvent::LineTo(point(2.0, 1.0)));
    assert_eq!(events3[3], PathEvent::LineTo(point(1.0, 1.0)));
    assert_eq!(events3[4], PathEvent::LineTo(point(1.0, 2.0)));
    if let PathEvent::LineTo(p) = events3[5] {
        assert!(p.approx_eq(&point(0.0, 2.0)))
    } else {
        panic!("unexpected event {:?}", events2[1]);
    }
    assert_eq!(events3[6], PathEvent::LineTo(point(0.0, 0.0)));
    assert_eq!(events3[7], PathEvent::LineTo(point(3.0, 0.0)));
    assert_eq!(events3[8], PathEvent::Close);
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

    assert_eq!(events1[0], PathEvent::MoveTo(point(0.0, 0.0)));
    assert_eq!(events1[1], PathEvent::LineTo(point(1.0, 2.0)));
    assert_eq!(events1[2], PathEvent::LineTo(point(0.0, 2.0)));
    assert_eq!(events1[3], PathEvent::Close);

    let sp2 = new_sub_paths[0];
    let events2: Vec<PathEvent> = path.sub_path_edges(sp2).path_iter().collect();

    assert_eq!(events2[0], PathEvent::MoveTo(point(1.0, 2.0)));
    assert_eq!(events2[1], PathEvent::LineTo(point(0.0, 0.0)));
    assert_eq!(events2[2], PathEvent::LineTo(point(2.0, 0.0)));
    assert_eq!(events2[3], PathEvent::LineTo(point(2.0, 2.0)));
    assert_eq!(events2[4], PathEvent::Close);
}

#[test]
#[ignore]
fn split_with_segment_4() {
    use path::PathEvent;

    //  ________
    // |        |
    //-+--+--+--+-
    // |  |  |  |
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
            from: point(-1.0, 1.0),
            to: point(4.0, 1.0),
        },
    );

    assert_eq!(new_sub_paths.len(), 2);
    let sp2 = new_sub_paths[0];
    let sp3 = new_sub_paths[1];

    let events1: Vec<PathEvent> = path.sub_path_edges(sp).path_iter().collect();
    let events2: Vec<PathEvent> = path.sub_path_edges(sp2).path_iter().collect();
    let events3: Vec<PathEvent> = path.sub_path_edges(sp3).path_iter().collect();

    println!("\n{:?}\n", events1);
    println!("\n{:?}\n", events2);
    println!("\n{:?}\n", events3);
}
