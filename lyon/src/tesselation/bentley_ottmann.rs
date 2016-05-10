#![allow(dead_code)]
#![allow(unused_variables)]

use tesselation::{ VertexId, VertexSlice };
use tesselation::polygon::*;
use tesselation::vectors::{ Position2D, /*Vec2, vec2_sub, vec2_len*/ };
use tesselation::sweep_line;
use tesselation::sweep_line::{ EventType };

use vodk_math::{ Vec2, vec2, fuzzy_eq };

struct BentleyOttmannAlgorithm {
    intersections: Intersections<ComplexPointId>,
    sweep_line: sweep_line::SweepLineLR,
}

impl BentleyOttmannAlgorithm {
    fn test_edge<V: Position2D>(&mut self, origin: ComplexPointId, polygon: ComplexPolygonSlice, vertices: VertexSlice<V>) {
        let a = vertices[polygon.vertex(origin)].position();
        let b = vertices[polygon.next_vertex(origin)].position();
        let v1 = b - a;
        if v1.fuzzy_eq(vec2(0.0, 0.0)) {
            return;
        }
        for sl_edge in self.sweep_line.as_slice() {
            let ea = sl_edge.from;
            let eb = sl_edge.to;
            let v2 = eb - ea;
            if v2.fuzzy_eq(vec2(0.0, 0.0)) {
                continue;
            }

            let v1_cross_v2 = v1.cross(v2);
            if fuzzy_eq(v1_cross_v2, 0.0) {
                continue;
            }
            // (q − p) x r / (r x s)
            let t = (a - ea).cross(v1) / v1_cross_v2;
            let intersection = a + (v1 * t);

            self.intersections.add_intersection(origin, sl_edge.key, intersection);
        }
    }
}

impl<V: Position2D> sweep_line::Algorithm<V> for BentleyOttmannAlgorithm {
    type Success = ();
    type Error = ();

    fn begin(&mut self, polygon: ComplexPolygonSlice, vertices: VertexSlice<V>) -> Result<(), ()> {
        return self.sweep_line.begin(polygon, vertices);
    }

    fn end(&mut self, polygon: ComplexPolygonSlice, vertices: VertexSlice<V>) -> Result<(), ()> {
        return self.sweep_line.end(polygon, vertices);
    }

    fn on_event(&mut self,
        event: &sweep_line::Event,
        event_type: EventType,
        polygon: ComplexPolygonSlice,
        vertices: VertexSlice<V>
    ) -> Result<(), ()> {

        match event_type {
            EventType::Start | EventType::Split => {
                self.test_edge(event.current, polygon, vertices);
                self.test_edge(event.previous, polygon, vertices);
            }
            EventType::Right => {
                self.test_edge(event.current, polygon, vertices);
            }
            EventType::Left => {
                self.test_edge(event.previous, polygon, vertices);
            }
            EventType::End | EventType::Merge => {
                // nothing to do
            }
        }
        return self.sweep_line.on_event(event, event_type, polygon, vertices);
    }
}

pub fn compute_segment_intersection(a1: Vec2, b1: Vec2, a2: Vec2, b2: Vec2) -> Option<Vec2> {
    let v1 = b1 - a1;
    let v2 = b2 - a2;
    if v2.fuzzy_eq(vec2(0.0, 0.0)) {
        return None;
    }

    let v1_cross_v2 = v1.cross(v2);
    let a2_a1_cross_v1 = (a2 - a1).cross(v1);

    if v1_cross_v2 == 0.0 {
        if a2_a1_cross_v1 == 0.0 {

            let v1_sqr_len = v1.square_length();
            // check if a2 is between a1 and b1
            let v1_dot_a2a1 = v1.dot(&(a2-a1));
            if v1_dot_a2a1 > 0.0 && v1_dot_a2a1 < v1_sqr_len { return Some(a2); }

            // check if b2 is between a1 and b1
            let v1_dot_b2a1 = v1.dot(&(b2-a1));
            if v1_dot_b2a1 > 0.0 && v1_dot_b2a1 < v1_sqr_len { return Some(b2); }

            let v2_sqr_len = v2.square_length();
            // check if a1 is between a2 and b2
            let v2_dot_a1a2 = v2.dot(&(a1-a2));
            if v2_dot_a1a2 > 0.0 && v2_dot_a1a2 < v2_sqr_len { return Some(a1); }

            // check if b1 is between a2 and b2
            let v2_dot_b1a2 = v2.dot(&(b1-a2));
            if v2_dot_b1a2 > 0.0 && v2_dot_b1a2 < v2_sqr_len { return Some(b1); }

            return None;
        }

        return None;
    }

    let t = a2_a1_cross_v1 / v1_cross_v2;
    let u = (a2 - a1).cross(v2) / v1_cross_v2;

    if t > 0.0 && t < 1.0 && u > 0.0 && u < 1.0 {
        return Some(a1 + (v1 * t));
    }

    return None;
}

#[test]
fn test_segment_intersection() {

    assert!(compute_segment_intersection(
        vec2(0.0, -2.0), vec2(-5.0, 2.0),
        vec2(-5.0, 0.0), vec2(-11.0, 5.0)
    ).is_none());

    let i = compute_segment_intersection(
        vec2(0.0, 0.0), vec2(1.0, 1.0),
        vec2(0.0, 1.0), vec2(1.0, 0.0)
    ).unwrap();
    println!(" intersection: {:?}", i);
    assert!(i.fuzzy_eq(vec2(0.5, 0.5)));

    assert!(compute_segment_intersection(
        vec2(0.0, 0.0), vec2(0.0, 1.0),
        vec2(1.0, 0.0), vec2(1.0, 1.0)
    ).is_none());

    assert!(compute_segment_intersection(
        vec2(0.0, 0.0), vec2(1.0, 0.0),
        vec2(2.0, 0.0), vec2(3.0, 0.0)
    ).is_none());

    assert!(compute_segment_intersection(
        vec2(0.0, 0.0), vec2(2.0, 0.0),
        vec2(1.0, 0.0), vec2(3.0, 0.0)
    ).is_some());

    assert!(compute_segment_intersection(
        vec2(3.0, 0.0), vec2(1.0, 0.0),
        vec2(2.0, 0.0), vec2(4.0, 0.0)
    ).is_some());

    assert!(compute_segment_intersection(
        vec2(2.0, 0.0), vec2(4.0, 0.0),
        vec2(3.0, 0.0), vec2(1.0, 0.0)
    ).is_some());

    assert!(compute_segment_intersection(
        vec2(1.0, 0.0), vec2(4.0, 0.0),
        vec2(2.0, 0.0), vec2(3.0, 0.0)
    ).is_some());

    assert!(compute_segment_intersection(
        vec2(2.0, 0.0), vec2(3.0, 0.0),
        vec2(1.0, 0.0), vec2(4.0, 0.0)
    ).is_some());

    assert!(compute_segment_intersection(
        vec2(0.0, 0.0), vec2(1.0, 0.0),
        vec2(0.0, 1.0), vec2(1.0, 1.0)
    ).is_none());
}

struct Intersection<PointId> {
    from: PointId,
    to: PointId,
    inter: Vec2,
    processed_face: bool,
    processed_opposite_face: bool,
}

pub struct Intersections<PointId> {
    intersections: Vec<Intersection<PointId>>
}

impl<PointId: ::std::fmt::Debug> Intersections<PointId> {
    pub fn new() -> Intersections<PointId> {
        Intersections {
            intersections: Vec::with_capacity(4),
        }
    }

    pub fn add_intersection(&mut self, from: PointId, to: PointId, inter: Vec2) {
        self.intersections.push(Intersection{
            from: from, to: to, inter: inter,
            processed_face: false, processed_opposite_face: false,
        });
    }

    pub fn is_empty(&self) -> bool { self.intersections.is_empty() }

    pub fn clear_flags(&mut self) {
        for inter in &mut self.intersections {
            inter.processed_face = false;
            inter.processed_opposite_face = false;
        }
    }
}

pub fn apply_intersections<Poly: AbstractPolygonSlice, V: Position2D>(
    polygon: Poly,
    vertices: VertexSlice<V>,
    intersections: &mut Intersections<Poly::PointId>,
    output: &mut Vec<Polygon>
) -> Result<(), ()> {
    Err(())
}

fn gen_polygon<Poly: AbstractPolygonSlice, V: Position2D>(
    polygon: Poly,
    vertices: VertexSlice<V>,
    intersections: &mut Intersections<Poly::PointId>,
    first_point: Poly::PointId,
    second_point: Poly::PointId,
) -> Result<Polygon, ()> {
    panic!("TODO");
/*
    let mut new_poly = Polygon::new();
    let mut prev = first_point;
    let mut it = second_point;
    let mut loop_counter = 0;
    loop {
        //println!("\n\n ------ point {:?}", it);
        new_poly.push_vertex(polygon.vertex(it));

        let mut selected = None;
        let mut shortest_dist = ::std::f32::NAN;

        for i in 0..intersections.intersections.len() {
            let intersect = &intersections.intersections[i];
            let to = if intersect.from == it { intersect.to }
                     else if intersect.to == it { intersect.from }
                     else { continue; };
            let from_v = vertices[polygon.vertex(it)].position();
            let to_v = vertices[polygon.vertex(to)].position();
            let inter_v = vertices[intersect.inter].position();
            let dist = vec2_len(vec2_sub(from_v, inter_v));
            if shortest_dist.is_nan() || dist < shortest_dist {
                shortest_dist = dist;
                selected = Some(i);
            }
        }

        if let Some(idx) = selected {
        }

        if it == second_point && prev == first_point {
            // back to where we began, the work is done.
            break;
        }

        loop_counter += 1;
        if loop_counter > polygon.num_vertices() * 2 {
            return Err(Error);
        }
    }

    Err(Error)
*/
}
