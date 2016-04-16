use std::f32::consts::PI;
use std::cmp::{Ordering, PartialOrd};
use std::mem::swap;

use tesselation::{ VertexId, VertexSlice };
use tesselation::path::*;
use tesselation::vectors::Position2D;
use tesselation::sweep_line::{ EventType, is_below, intersect_segment_with_horizontal };

use vodk_math::{ Vector2D, Vec2 };

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

enum Side { Left, Right }

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
    helper: Option<ComplexVertexId>,
}

struct MonotoneEvent {
    position: Vec2,
    vertex_id: VertexId,
    side: Option<Side>,
}

struct MonotoneTesselator;

type SpanId = u32;

impl MonotoneTesselator {
    pub fn begin(&mut self, position: Vec2, vertex_id: VertexId) -> SpanId { 0 }
    pub fn vertex(&mut self, span_id: SpanId, position: Vec2, vertex_id: VertexId, side: Side) {}
    pub fn end(&mut self, span_id: SpanId, position: Vec2, vertex_id: VertexId) {}
}

impl Span {
    fn end(&mut self) {
    }

    fn set_left(&mut self, id: ComplexVertexId, pos: Vec2) {
        self.left.upper = self.left.lower.unwrap();
        self.left.upper_position = self.left.lower_position;
        self.left.lower = Some(id);
        self.left.lower_position = pos;
        self.last_side = Some(Side::Left);
    }

    fn set_right(&mut self, id: ComplexVertexId, pos: Vec2) {
        self.right.upper = self.left.lower.unwrap();
        self.right.upper_position = self.left.lower_position;
        self.right.lower = Some(id);
        self.right.lower_position = pos;
        self.last_side = Some(Side::Right);
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
                for mut chain in &mut self.sweep_line.spans {
                    if chain.left.lower == Some(event.current) {
                        if next_below {
                            chain.set_left(event.next, event.next_position);
                        } else {
                            chain.set_left(event.previous, event.previous_position);
                        }
                    } else if chain.right.lower == Some(event.current) {
                        if next_below {
                            chain.set_right(event.next, event.next_position);
                        } else {
                            chain.set_right(event.previous, event.previous_position);
                        }
                    }
                }
            }
            BaseEventType::Up => {
                let mut inside = false;
                let mut span_index = 0;
                for chain in &mut self.sweep_line.spans {
                    let lx = intersect_segment_with_horizontal(
                        chain.left.upper_position,
                        chain.left.lower_position,
                        y
                    );
                    if lx > x {
                        // inside = false
                        break;
                    }
                    let rx = intersect_segment_with_horizontal(
                        chain.right.upper_position,
                        chain.right.lower_position,
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

                    // |    /\    |
                    // ll  l  r  rr
                    let mut rr = self.sweep_line.spans[span_index].right;

                    self.sweep_line.spans[span_index+1].right = l;

                    self.sweep_line.spans.insert(span_index+1, Span {
                        left: r, right: rr, last_side: None, helper: None,
                    })
                } else {
                    // Start event.

                    self.sweep_line.spans.insert(span_index, Span {
                        left: l, right: r, last_side: None, helper: None,
                    })
                }
            }
            BaseEventType::Down => {
                // look for the two edges in the sweep line that contain event.current
                // if they are both on the same chain -> end event else -> merge
                for chain in &mut self.sweep_line.spans {
                    if chain.right.lower == Some(event.current) {
                        if chain.left.lower == Some(event.current) {
                            // End event
                        } else {
                            // Merge event
                        }
                        break;
                    }
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
