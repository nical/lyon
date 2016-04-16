use std::cmp::{Ordering, PartialOrd};
use std::f32::consts::PI;

use tesselation::{ VertexId, VertexSlice };
use tesselation::polygon::*;
use tesselation::vectors::Position2D;

use vodk_math::{ Vector2D, Vec2 };

#[cfg(test)]
use vodk_math::{ vec2 };

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum EventType {
    Start,
    End,
    Split,
    Merge,
    Left,
    Right,
}

/// Trait implemented by sweep line algorithms.
pub trait Algorithm<Vertex: Position2D> {
    type Success;
    type Error;

    fn begin(&mut self,
        _polygon: ComplexPolygonSlice,
        _vertices: VertexSlice<Vertex>
    ) -> Result<(), Self::Error> { Ok(()) }

    fn end(&mut self,
        polygon: ComplexPolygonSlice,
        vertices: VertexSlice<Vertex>
    ) -> Result<Self::Success, Self::Error>;

    fn on_event(&mut self,
        event: &Event,
        event_type: EventType,
        polygon: ComplexPolygonSlice,
        vertices: VertexSlice<Vertex>
    ) -> Result<(), Self::Error>;
}

/// run the algorithm passed in parameters on pre-sorted events for a given polygon.
pub fn rum_algorithm<Vertex: Position2D,  Algo: Algorithm<Vertex>>(
    polygon: ComplexPolygonSlice,
    vertices: VertexSlice<Vertex>,
    events: SortedEventSlice,
    algorithm: &mut Algo
) -> Result<Algo::Success, Algo::Error> {
    try!{ algorithm.begin(polygon, vertices) }

    for &evt in events.events {
        let prev = polygon.previous(evt);
        let next = polygon.next(evt);
        let event = Event {
            current: evt,
            previous: prev,
            next: next,
            current_position: vertices[polygon.vertex(evt)].position(),
            previous_position: vertices[polygon.vertex(prev)].position(),
            next_position: vertices[polygon.vertex(next)].position(),
        };

        let evt_type = compute_event_type(
            event.previous_position,
            event.current_position,
            event.next_position
        );

        try!{ algorithm.on_event(&event, evt_type, polygon, vertices) };
    }

    return algorithm.end(polygon, vertices);
}

pub fn compute_event_type(prev: Vec2, current: Vec2, next: Vec2) -> EventType {
    // assuming clockwise vertex_positions winding order
    let interrior_angle = (prev - current).directed_angle(next - current);

    // If the interrior angle is exactly 0 we'll have degenerate (invisible 0-area) triangles
    // which is yucks but we can live with it for the sake of being robust against degenerate
    // inputs. So special-case them so that they don't get considered as Merge ot Split vertices
    // otherwise there can be no monotone decomposition of a shape where all points are on the
    // same line.

    if is_below(current, prev) && is_below(current, next) {
        if interrior_angle < PI && interrior_angle != 0.0 {
            return EventType::Merge;
        } else {
            return EventType::End;
        }
    }

    if !is_below(current, prev) && !is_below(current, next) {
        if interrior_angle < PI && interrior_angle != 0.0 {
            return EventType::Split;
        } else {
            return EventType::Start;
        }
    }

    if prev.y == next.y {
        return if prev.x < next.x { EventType::Right } else { EventType::Left };
    }
    return if prev.y < next.y { EventType::Right } else { EventType::Left };
}

#[derive(Copy, Clone)]
pub struct SortedEventSlice<'l> {
    pub events: &'l[ComplexPointId]
}

/// Contains the events of a polygon and provide access to them, sorted from top to bottom
/// (assuming y points downwards).
pub struct EventVector {
    events: Vec<ComplexPointId>
}

impl EventVector {
    pub fn new() -> EventVector {
        EventVector { events: Vec::new() }
    }

    pub fn from_polygon<Vertex: Position2D>(
        polygon: ComplexPolygonSlice,
        vertices: VertexSlice<Vertex>,
    ) -> EventVector {
        let mut ev = EventVector {
            events: Vec::with_capacity(polygon.num_vertices())
        };
        ev.set_polygon(polygon, vertices);
        return ev;
    }

    pub fn set_polygon<Vertex: Position2D>(&mut self,
        polygon: ComplexPolygonSlice,
        vertices: VertexSlice<Vertex>,
    ) {
        self.events.clear();
        for sub_poly in polygon.polygon_ids() {
            self.events.extend(polygon.point_ids(sub_poly));
        }
        debug_assert!(self.events.len() == polygon.num_vertices());

        self.events.sort_by(|a, b| {
            let va = vertices[polygon.vertex(*a)].position();
            let vb = vertices[polygon.vertex(*b)].position();
            if va.y > vb.y { return Ordering::Greater; }
            if va.y < vb.y { return Ordering::Less; }
            if va.x > vb.x { return Ordering::Greater; }
            if va.x < vb.x { return Ordering::Less; }
            return Ordering::Equal;
        });
    }

    pub fn as_slice(&self) -> SortedEventSlice {
        SortedEventSlice { events: &self.events[..] }
    }

    pub fn clear(&mut self) {
        self.events.clear();
    }
}

/// Defines an ordering between two points
pub fn is_below(a: Vec2, b: Vec2) -> bool { a.y > b.y || (a.y == b.y && a.x > b.x) }
pub fn is_right_of(a: Vec2, b: Vec2) -> bool { a.x > b.x || (a.x == b.x && a.y > b.y) }


#[derive(Debug)]
pub struct SweepLineEdge {
    pub from: Vec2,
    pub to: Vec2,
    pub key: ComplexPointId,
    pub helper: Option<(ComplexPointId, EventType)>,
}

pub struct SweepLine {
    edges: Vec<SweepLineEdge>,
    current_position: Vec2,
}

impl SweepLine {
    pub fn new() -> SweepLine {
        SweepLine {
            edges: Vec::with_capacity(64),
            current_position: Vec2::new(0.0, 0.0),
        }
    }

    pub fn set_current_position(&mut self, pos: Vec2) {
        self.current_position = pos;
    }

    pub fn add(&mut self, e: SweepLineEdge) {
        self.edges.push(e);
        // sort from left to right (increasing x values)
        let y = self.current_position.y;
        self.edges.sort_by(|a, b| {
            let xa = intersect_segment_with_horizontal(a.from, a.to, y);
            let xb = intersect_segment_with_horizontal(b.from, b.to, y);

            if xa == xb {
                let ma = (a.from.x + a.to.x) / 2.0;
                let mb = (b.from.x + b.to.x) / 2.0;
                return ma.partial_cmp(&mb).unwrap();
            }

            return xa.partial_cmp(&xb).unwrap();
        });
    }

    pub fn remove(&mut self, key: ComplexPointId) {
        self.edges.retain(|item|{ item.key != key });
    }

    // Search the sweep status to find the edge directly to the right of the current vertex.
    pub fn find_right_of_current_position(&self) -> Option<ComplexPointId> {
        for e in &self.edges {
            let x = intersect_segment_with_horizontal(e.from, e.to, self.current_position.y);
            if x >= self.current_position.x {
                return Some(e.key);
            }
        }

        return None;
    }

    // Search the sweep status to find the edge directly to the right of the current vertex.
    pub fn find_index_right_of_current_position(&self) -> Option<usize> {
        let mut i: usize = 0;
        for e in &self.edges {
            let x = intersect_segment_with_horizontal(e.from, e.to, self.current_position.y);
            if x >= self.current_position.x {
                return Some(i);
            }
            i += 1;
        }

        return None;
    }

    pub fn find(&self, edge: ComplexPointId) -> Option<usize> {
        let mut i: usize = 0;
        for e in &self.edges {
            if e.key == edge {
                return Some(i);
            }
            i += 1;
        }
        return None;
    }

    pub fn set_helper(&mut self, index: usize, e: ComplexPointId, evt_type: EventType) {
        self.edges[index].helper = Some((e, evt_type));
    }

    pub fn get_helper(&mut self, index: usize) -> Option<(ComplexPointId, EventType)> {
        self.edges[index].helper
    }

    pub fn as_slice(&self) -> &[SweepLineEdge] { &self.edges[..] }
    pub fn as_mut_slice(&mut self) -> &mut[SweepLineEdge] { &mut self.edges[..] }
}

pub struct Event {
    pub current_position: Vec2,
    pub previous_position: Vec2,
    pub next_position: Vec2,
    pub current: ComplexPointId,
    pub previous: ComplexPointId,
    pub next: ComplexPointId,
}

/// A sweep line that records edges on the left and right side of the geometry
pub struct SweepLineLR {
    sweep_line: SweepLine,
}

impl SweepLineLR {
    pub fn new() -> SweepLineLR { SweepLineLR { sweep_line: SweepLine::new() }}

    pub fn as_slice(&self) -> &[SweepLineEdge] { self.sweep_line.as_slice() }
}

/// A sweep line that records edges on the left side of the geometry only
pub struct SweepLineL {
    sweep_line: SweepLine,
}

impl SweepLineL {
    pub fn new() -> SweepLineL { SweepLineL { sweep_line: SweepLine::new() }}

    pub fn as_slice(&self) -> &[SweepLineEdge] { self.sweep_line.as_slice() }
}

impl<Vertex: Position2D> Algorithm<Vertex> for SweepLineLR {
    type Success = ();
    type Error = ();

    fn end(&mut self,
        _polygon: ComplexPolygonSlice,
        _vertices: VertexSlice<Vertex>
    ) -> Result<(), ()> {
        Ok(())
    }

    fn on_event(&mut self,
        event: &Event,
        event_type: EventType,
        _polygon: ComplexPolygonSlice,
        _vertices: VertexSlice<Vertex>
    ) -> Result<(), ()> {
        let edge = SweepLineEdge {
            key: event.current,
            from: event.current_position,
            to: event.next_position,
            helper: None, // TODO
        };
        let prev_edge = SweepLineEdge {
            key: event.previous,
            from: event.previous_position,
            to: event.current_position,
            helper: None, // TODO
        };
        self.sweep_line.current_position = event.current_position;
        match event_type {
            EventType::Start | EventType::Split => {
                self.sweep_line.add(edge);
                self.sweep_line.add(prev_edge);
            }
            EventType::End | EventType::Merge => {
                self.sweep_line.remove(event.current);
                self.sweep_line.remove(event.previous);
            }
            EventType::Right => {
                self.sweep_line.remove(event.previous);
                self.sweep_line.add(edge);
            }
            EventType::Left => {
                self.sweep_line.remove(event.current);
                self.sweep_line.add(prev_edge);
            }
        }

        return Ok(());
    }
}

impl<Vertex: Position2D> Algorithm<Vertex> for SweepLineL {
    type Success = ();
    type Error = ();

    fn end(&mut self,
        _polygon: ComplexPolygonSlice,
        _vertices: VertexSlice<Vertex>
    ) -> Result<(), ()> {
        Ok(())
    }

    fn on_event(&mut self,
        event: &Event,
        event_type: EventType,
        _polygon: ComplexPolygonSlice,
        _vertices: VertexSlice<Vertex>
    ) -> Result<(), ()> {
        let edge = SweepLineEdge {
            key: event.current,
            from: event.current_position,
            to: event.next_position,
            helper: None, // TODO
        };
        self.sweep_line.current_position = event.current_position;
        match event_type {
            EventType::Start => {
                self.sweep_line.add(edge);
            }
            EventType::End => {
                self.sweep_line.remove(event.previous);
            }
            EventType::Split => {
                self.sweep_line.add(edge);
            }
            EventType::Merge => {
                self.sweep_line.remove(event.previous);
            }
            EventType::Right => {
                self.sweep_line.add(edge);
                self.sweep_line.remove(event.previous);
            }
            EventType::Left => {}
        }

        return Ok(());
    }
}

/// Combine two algorithms and run them in lock step in the same sweep.
pub struct Combined<Algo1, Algo2> {
    first: Algo1,
    second: Algo2,
}

impl<
    Vertex: Position2D,
    Algo1: Algorithm<Vertex>,
    Algo2: Algorithm<Vertex>
> Algorithm<Vertex> for Combined<Algo1, Algo2> {
    type Success = (Algo1::Success, Algo2::Success);
    type Error = (Option<Algo1::Error>, Option<Algo2::Error>);

    fn begin(&mut self,
        polygon: ComplexPolygonSlice,
        vertices: VertexSlice<Vertex>
    ) -> Result<(), Self::Error> {
        let r1 = self.first.begin(polygon, vertices);
        let r2 = self.second.begin(polygon, vertices);

        return match (r1, r2) {
            (Ok(_), Ok(_)) => { Ok(()) }
            (Err(e1), Err(e2)) => { Err((Some(e1), Some(e2))) }
            (Ok(_), Err(e2)) => { Err((None, Some(e2))) }
            (Err(e1), Ok(_)) => { Err((Some(e1), None)) }
        };
    }

    fn end(&mut self,
        polygon: ComplexPolygonSlice,
        vertices: VertexSlice<Vertex>
    ) -> Result<(Algo1::Success, Algo2::Success), Self::Error> {
        let r1 = self.first.end(polygon, vertices);
        let r2 = self.second.end(polygon, vertices);
        return match (r1, r2) {
            (Ok(o1), Ok(o2)) => { Ok((o1, o2)) }
            (Err(e1), Err(e2)) => { Err((Some(e1), Some(e2))) }
            (Ok(_), Err(e2)) => { Err((None, Some(e2))) }
            (Err(e1), Ok(_)) => { Err((Some(e1), None)) }
        };
    }

    fn on_event(&mut self,
        event: &Event,
        event_type: EventType,
        polygon: ComplexPolygonSlice,
        vertices: VertexSlice<Vertex>
    ) -> Result<(), Self::Error> {
        let r1 = self.first.on_event(event, event_type, polygon, vertices);
        let r2 = self.second.on_event(event, event_type, polygon, vertices);
        return match (r1, r2) {
            (Ok(_), Ok(_)) => { Ok(()) }
            (Err(e1), Err(e2)) => { Err((Some(e1), Some(e2))) }
            (Ok(_), Err(e2)) => { Err((None, Some(e2))) }
            (Err(e1), Ok(_)) => { Err((Some(e1), None)) }
        };
    }
}

pub fn intersect_segment_with_horizontal<U>(a: Vector2D<U>, b: Vector2D<U>, y: f32) -> f32 {
    let vx = b.x - a.x;
    let vy = b.y - a.y;
    if vy == 0.0 {
        // If the segment is horizontal, pick the biggest x value (the right-most point).
        // That's an arbitrary decision that serves the purpose of y-monotone decomposition
        return a.x.max(b.x);
    }
    return a.x + (y - a.y) * vx / vy;
}


#[cfg(test)]
fn assert_almost_eq(a: f32, b:f32) {
    if (a - b).abs() < 0.0001 { return; }
    println!("expected {} and {} to be equal", a, b);
    panic!();
}

#[test]
fn test_intersect_segment_horizontal() {
    assert_almost_eq(intersect_segment_with_horizontal(vec2(0.0, 0.0), vec2(0.0, 2.0), 1.0), 0.0);
    assert_almost_eq(intersect_segment_with_horizontal(vec2(0.0, 2.0), vec2(2.0, 0.0), 1.0), 1.0);
    assert_almost_eq(intersect_segment_with_horizontal(vec2(0.0, 1.0), vec2(3.0, 0.0), 0.0), 3.0);
}

#[test]
fn test_sweep_line_lr() {

    struct SweepLineTestLR {
        sweep: SweepLineLR,
        advance: usize,
    }

    impl Algorithm<Vec2> for SweepLineTestLR {
        type Success = ();
        type Error = ();

        fn begin(&mut self,
            polygon: ComplexPolygonSlice,
            vertices: VertexSlice<Vec2>
        ) -> Result<(), ()> {
            return self.sweep.begin(polygon, vertices);
        }

        fn end(&mut self,
            polygon: ComplexPolygonSlice,
            vertices: VertexSlice<Vec2>
        ) -> Result<(), ()> {
            return self.sweep.end(polygon, vertices);
        }

        fn on_event(&mut self,
            event: &Event,
            evt_type: EventType,
            polygon: ComplexPolygonSlice,
            vertices: VertexSlice<Vec2>
        ) -> Result<(), ()> {
            let expected: &[(u16, EventType, &[u16])] = &[
                (7,  EventType::Start,  &[]),
                (5,  EventType::Start,  &[6, 7]),
                (9,  EventType::Start,  &[4, 5, 6, 7]),
                (4,  EventType::Left,   &[4, 5, 6, 7, 8, 9]),
                (8,  EventType::Merge,  &[3, 5, 6, 7, 8, 9]),
                (6,  EventType::Merge,  &[3, 5, 6, 9]),
                (10, EventType::Right,  &[3, 9]),
                (2,  EventType::Split,  &[3, 10]),
                (11, EventType::Right,  &[3, 2, 1, 10]),
                (3,  EventType::End,    &[3, 2, 1, 11]),
                (12, EventType::Right,  &[1, 11]),
                (1,  EventType::Left,   &[1, 12]),
                (0,  EventType::End,    &[0, 12]),
            ];

            let (expected_evt, expected_type, expected_sl) = expected[self.advance];

            println!("  -- evt {}", event.current.point.handle);
            assert_eq!(event.current.point.handle, expected_evt);
            assert_eq!(evt_type, expected_type);

            {
                let sl = self.sweep.sweep_line.as_slice();
                for i in 0..sl.len() {
                    assert_eq!(sl[i].key.point.handle, expected_sl[i]);
                }
            }

            self.advance += 1;
            return self.sweep.on_event(event, evt_type, polygon, vertices);
        }
    }

    let positions = &[
        vec2(3.0, 10.0),//  0 -> 12
        vec2(4.0, 9.0), //  1 -> 11
        vec2(3.0, 5.0), //  2 ->  7
        vec2(2.0, 7.0), //  3 ->  9
        vec2(0.0, 3.0), //  4 ->  3
        vec2(1.0, 1.0), //  5 ->  1
        vec2(2.0, 4.0), //  6 ->  5
        vec2(3.0, 0.0), //  7 ->  0
        vec2(4.0, 3.0), //  8 ->  4
        vec2(5.0, 2.0), //  9 ->  2
        vec2(6.0, 4.0), // 10 ->  6
        vec2(5.0, 6.0), // 11 ->  8
        vec2(6.0, 8.0), // 12 -> 10
    ];

    let vertices: VertexSlice<Vec2> = VertexSlice::new(positions);
    let polygon = Polygon::from_vertices(vertices.ids()).into_complex_polygon();
    let mut events = EventVector::new();
    events.set_polygon(polygon.as_slice(), vertices);

    rum_algorithm(
        polygon.as_slice(),
        vertices, events.as_slice(),
        &mut SweepLineTestLR {
            sweep: SweepLineLR::new(),
            advance: 0
        }
    ).unwrap();
}


