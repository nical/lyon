use math::*;
use geom::LineSegment;
use std::cmp::PartialOrd;
use advanced_path::*;

#[derive(Debug)]
struct IntersectingEdge {
    intersection: Point,
    id: EdgeId,
    d: f32,
}

pub struct Splitter {
    intersecting_edges: Vec<IntersectingEdge>,
}

impl Splitter {
    pub fn new() -> Self {
        Splitter { intersecting_edges: Vec::new(), }
    }

    pub fn segment_split(
        &mut self,
        path: &mut AdvancedPath,
        selection: &SubPathSelection,
        segment: &LineSegment<f32>
    ) -> Vec<SubPathId> {
        self.intersecting_edges.clear();

        let v = segment.to_vector();

        // Find the edges that intersect the segment.
        path.for_each_edge_id(selection, &mut|path, _sub_path, edge_id| {
            let edge = path.edge(edge_id);
            let edge_segment = LineSegment {
                from: path[edge.from],
                to: path[edge.to],
            };

            if let Some(intersection) = segment.intersection(&edge_segment) {
                self.intersecting_edges.push(IntersectingEdge {
                    intersection,
                    id: edge_id,
                    d: v.dot(intersection - segment.from),
                });
            }
        });

        // Sort the intersecting edges along the segment.
        self.intersecting_edges.sort_by(|a, b| { a.d.partial_cmp(&b.d).unwrap() });

        // Split the intersecting edges by pair to avoid splitting an edge if
        // we aren't going to be able to connect it.
        for i in 0..(self.intersecting_edges.len() / 2) {
            let e1 = &self.intersecting_edges[i];
            let e2 = &self.intersecting_edges[i+1];
            path.split_edge(e1.id, e1.intersection);
            path.split_edge(e2.id, e2.intersection);
        }

        // Connect the split edges.
        let mut new_sub_paths = Vec::new();
        for i in 0..(self.intersecting_edges.len() / 2) {
            let i = i * 2;
            let e1 = self.intersecting_edges[i].id;
            let e2 = path.next_edge_id(self.intersecting_edges[i+1].id);

            if let Some(sub_path) = path.connect_edges(e1, e2) {
                new_sub_paths.push(sub_path);
            }
        }

        new_sub_paths
    }
}

#[test]
fn segment_split() {
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

    let mut edge_id = None;
    for id in path.sub_path_edge_id_loop(sp) {
        if path[path.edge(id).from] == point(1.0, 0.0) {
            edge_id = Some(id);
        }
    }

    let mut splitter = Splitter::new();
    let new_sub_paths = splitter.segment_split(
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
