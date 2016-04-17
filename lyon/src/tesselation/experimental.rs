use std::f32::consts::PI;
use std::cmp::{Ordering, PartialOrd};
use std::mem::swap;

use tesselation::{ vertex_id, VertexId, VertexSlice };
use tesselation::path::*;
use tesselation::vectors::Position2D;
use tesselation::sweep_line::{ EventType, is_below, intersect_segment_with_horizontal };

use vodk_math::{ Vector2D, Vec2, vec2 };

pub struct Event {
    pub current_position: Vec2,
    pub previous_position: Vec2,
    pub next_position: Vec2,
    pub current: ComplexVertexId,
    pub previous: ComplexVertexId,
    pub next: ComplexVertexId,
}

#[derive(Copy, Clone)]
pub struct SortedEventSlice<'l> {
    pub events: &'l[ComplexVertexId]
}

/// Contains the events of a path and provide access to them, sorted from top to bottom
/// (assuming y points downwards).
pub struct EventVector {
    events: Vec<ComplexVertexId>
}

impl EventVector {
    pub fn new() -> EventVector {
        EventVector { events: Vec::new() }
    }

    pub fn from_path(
        path: ComplexPathSlice,
    ) -> EventVector {
        let mut ev = EventVector {
            events: Vec::with_capacity(path.vertices().len())
        };
        ev.set_path(path);
        return ev;
    }

    pub fn set_path(&mut self,
        path: ComplexPathSlice,
    ) {
        self.events.clear();
        for sub_path in path.path_ids() {
            self.events.extend(path.vertex_ids(sub_path));
        }
        debug_assert!(self.events.len() == path.vertices().len());

        self.events.sort_by(|a, b| {
            let va = path.vertex(*a).position();
            let vb = path.vertex(*b).position();
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

pub enum BaseEventType {
    Regular,
    Up,
    Down,
}

pub struct Tesselator<'l> {
    path: ComplexPathSlice<'l>,
    sweep_line: SweepLine,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum Side { Left, Right }

impl Side {
    pub fn opposite(self) -> Side {
        match self {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
        }
    }
}

#[derive(Copy, Clone, Debug)]
struct Edge {
    upper_position: Vec2,
    lower_position: Vec2,
    upper: ComplexVertexId,
    lower: Option<ComplexVertexId>,
}

struct Span {
    left: Edge,
    right: Edge,
    last_side: Option<Side>,
    helper: Option<(ComplexVertexId, EventType)>,
    montone_tesselator: MonotoneTesselator,
}

impl Span {
    fn begin(l: Edge, r: Edge) -> Span {
        Span {
            left: l, right: r, last_side: None,
            helper: Some((l.upper, EventType::Start)),
            montone_tesselator: MonotoneTesselator::begin(l.upper_position, l.upper.vertex_id),
        }
    }

    fn vertex(&mut self, pos: Vec2, id: ComplexVertexId, side: Side) {
        let mut e = match side {
            Side::Left => { &mut self.left }
            Side::Right => { &mut self.right }
        };

        e.upper = e.lower.unwrap();
        e.upper_position = e.lower_position;
        e.lower = Some(id);
        e.lower_position = pos;
        self.last_side = Some(Side::Left);

        self.montone_tesselator.vertex(pos, id.vertex_id, side);
    }

    fn end(&mut self, pos: Vec2, id: ComplexVertexId) {
        self.montone_tesselator.end(pos, id.vertex_id);
    }
}

struct SweepLine {
    spans: Vec<Span>
}

impl<'l> Tesselator<'l> {
    pub fn new(path: ComplexPathSlice<'l>) -> Tesselator<'l> {
        Tesselator {
            path: path,
            sweep_line: SweepLine {
                spans: Vec::with_capacity(16),
            },
        }
    }

    pub fn tesselate(&mut self, sorted_events: SortedEventSlice<'l>) -> Result<(), ()>{

        for &e in sorted_events.events {
            let p = self.path.previous(e);
            let n = self.path.next(e);
            let evt = Event {
                current_position: self.path.vertex(e).position(),
                previous_position: self.path.vertex(p).position(),
                next_position: self.path.vertex(n).position(),
                current: e,
                previous: p,
                next: n
            };
            self.on_event(&evt);
        }

        return Err(());
    }

    fn on_event(&mut self, event: &Event) -> Result<(), ()> {
        let (x, y) = event.current_position.tuple();
        let base_evt_type = compute_base_event_type(
            event.previous_position, event.current_position, event.next_position
        );

        let angle = (event.previous_position - event.current_position).directed_angle(
            event.next_position - event.current_position
        );

        match base_evt_type {
            BaseEventType::Regular => {
                let next_below = is_below(event.next_position, event.current_position);
                for mut span in &mut self.sweep_line.spans {

                    let side = if span.left.lower == Some(event.current) { Side::Left }
                               else if span.right.lower == Some(event.current) { Side::Right }
                               else { continue; };

                    let (point, pos) = if next_below { (event.next, event.next_position) }
                                       else { (event.previous, event.previous_position) };

                    span.vertex(pos, point, side);
                }
            }
            BaseEventType::Up => {
                let mut inside = false;
                let mut span_index = 0;
                for span in &self.sweep_line.spans {
                    let lx = intersect_segment_with_horizontal(
                        span.left.upper_position,
                        span.left.lower_position,
                        y
                    );
                    if lx > x {
                        // inside = false
                        break;
                    }
                    let rx = intersect_segment_with_horizontal(
                        span.right.upper_position,
                        span.right.lower_position,
                        y
                    );
                    if rx > x {
                        inside = true;
                        break;
                    }
                    span_index += 1;
                }

                let mut l = Edge {
                    upper: event.current,
                    lower: Some(event.previous),
                    upper_position: event.current_position,
                    lower_position: event.previous_position,
                };
                let mut r = Edge {
                    upper: event.current,
                    lower: Some(event.next),
                    upper_position: event.current_position,
                    lower_position: event.next_position,
                };

                if angle < PI / 2.0 {
                    swap(&mut l, &mut r);
                }

                if inside {
                    // Split event.

                    // TODO: connect something

                    // |     :     |
                    // |    / \    |
                    // ll  l   r  rr
                    let mut rr = self.sweep_line.spans[span_index].right;

                    self.sweep_line.spans[span_index+1].right = l;

                    // TODO: that's a bit more complicated.
                    //
                    //   |    \ /    |
                    //   |     :     |
                    //   |    / \    |
                    //
                    // In the above case we don't insert a new span
                } else {
                    // Start event.

                    self.sweep_line.spans.insert(span_index, Span::begin(l, r));
                }
            }
            BaseEventType::Down => {
                let mut is_end = false;
                let mut index = 0;
                // look for the two edges in the sweep line that contain event.current
                // if they are both on the same span -> end event else -> merge
                for span in &mut self.sweep_line.spans {
                    if span.right.lower == Some(event.current) {
                        is_end = span.left.lower == Some(event.current);
                        break;
                    }
                    index += 1;
                }

                if is_end {
                    // End event
                    self.sweep_line.spans[index].end(event.current_position, event.current);
                    self.sweep_line.spans.remove(index);
                } else {
                    // Merge event
                }
            }
        }

        return Err(());
    }

    fn on_start_event(&mut self, event: &Event) -> Result<(), ()> {
        Err(())
    }

    fn on_split_event(&mut self, event: &Event) -> Result<(), ()> {
        Err(())
    }
}

pub fn compute_base_event_type(prev: Vec2, current: Vec2, next: Vec2) -> BaseEventType {
    let interrior_angle = (prev - current).directed_angle(next - current);

    if is_below(current, prev) && is_below(current, next) {
        if interrior_angle < PI && interrior_angle != 0.0 {
            return BaseEventType::Down;
        }
    }

    if !is_below(current, prev) && !is_below(current, next) {
        if interrior_angle < PI && interrior_angle != 0.0 {
            return BaseEventType::Up;
        }
    }

    return BaseEventType::Regular;
}

/// helper class that generates a triangulation from a sequence of vertices describing a monotone
/// polygon.
struct MonotoneTesselator {
    stack: Vec<MonotoneVertex>,
    previous: MonotoneVertex,
    triangles: Vec<u16>,
}

#[derive(Copy, Clone, Debug)]
struct MonotoneVertex {
    pos: Vec2,
    id: VertexId,
    side: Side,
}

impl MonotoneTesselator {
    pub fn begin(pos: Vec2, id: VertexId) -> MonotoneTesselator {
        let first = MonotoneVertex { pos: pos, id: id, side: Side::Left };

        let mut tess = MonotoneTesselator {
            stack: Vec::with_capacity(32),
            triangles: Vec::with_capacity(32),
            previous: first,
        };

        tess.stack.push(first);

        return tess;
    }

    pub fn vertex(&mut self, pos: Vec2, id: VertexId, side: Side) {
        let mut current = MonotoneVertex{ pos: pos, id: id, side: side };

        assert!(is_below(current.pos, self.previous.pos));

        let changed_side = current.side != self.previous.side;
        let winding_fixup = current.side == Side::Right;

        if !self.stack.is_empty() && changed_side {
            for i in 0..(self.stack.len() - 1) {
                let mut a = self.stack[i];
                let mut b = self.stack[i+1];
                let c = current;
                if winding_fixup {
                    swap(&mut a, &mut b);
                }
                self.push_triangle(&a, &b, &c);
            }
            self.stack.clear();
            self.stack.push(self.previous);
            self.stack.push(current);
        } else {
            let mut last_popped = self.stack.pop();
            loop {
                if self.stack.is_empty() {
                    break;
                }

                let mut a = *self.stack.last().unwrap();
                let mut b = last_popped.unwrap();
                let c = current;

                if winding_fixup {
                    swap(&mut a, &mut b);
                }

                if (c.pos - b.pos).directed_angle(c.pos - a.pos) > PI {
                    self.push_triangle(&a, &b, &c);

                    last_popped = self.stack.pop();
                } else {
                    break;
                }
            }
            if let Some(item) = last_popped {
                self.stack.push(item);
            }
            self.stack.push(current);
        }

        self.previous = current;
    }

    pub fn end(&mut self, pos: Vec2, id: VertexId) {
        let side = self.previous.side.opposite();
        self.vertex(pos, id, side);
    }

    fn push_triangle(&mut self, a: &MonotoneVertex, b: &MonotoneVertex, c: &MonotoneVertex) {
        println!(" -- triangle {} {} {}", a.id.handle, b.id.handle, c.id.handle);
        self.triangles.push(a.id.handle);
        self.triangles.push(b.id.handle);
        self.triangles.push(c.id.handle);
    }
}

#[test]
fn test_monotone_tess() {
    {
        let mut tess = MonotoneTesselator::begin(vec2(0.0, 0.0), vertex_id(0));
        tess.vertex(vec2(1.0, 1.0), vertex_id(1), Side::Right);
        tess.vertex(vec2(-1.5, 2.0), vertex_id(2), Side::Left);
        tess.vertex(vec2(-1.0, 3.0), vertex_id(3), Side::Left);
        tess.vertex(vec2(1.0, 4.0), vertex_id(4), Side::Right);
        tess.end(vec2(0.0, 5.0), vertex_id(5));
    }
    {
        let mut tess = MonotoneTesselator::begin(vec2(0.0, 0.0), vertex_id(0));
        tess.vertex(vec2(-1.0, 1.0), vertex_id(1), Side::Left);
        tess.end(vec2(1.0, 2.0), vertex_id(2));
    }
}