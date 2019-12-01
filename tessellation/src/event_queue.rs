use crate::geom::{QuadraticBezierSegment, CubicBezierSegment};
use crate::math::{Point, point};
use crate::path;
use crate::path::{PathEvent, IdEvent, EndpointId, PositionStore};
use crate::fill::{is_after, compare_positions};

use std::{f32, u32, usize};
use std::cmp::Ordering;
use std::ops::Range;
use std::mem::swap;

pub type TessEventId = u32;

pub const INVALID_EVENT_ID: TessEventId = u32::MAX;

pub struct Event {
    pub next_sibling: TessEventId,
    pub next_event: TessEventId,
    pub position: Point,
}

#[derive(Clone, Debug)]
pub struct EdgeData {
    pub to: Point,
    pub range: std::ops::Range<f32>,
    pub winding: i16,
    pub is_edge: bool,
    pub evt_id: path::EventId,
    pub from_id: EndpointId,
    pub to_id: EndpointId,
}

pub struct EventQueue {
    pub events: Vec<Event>,
    pub edge_data: Vec<EdgeData>,
    pub first: TessEventId,
    pub sorted: bool,
}

impl EventQueue {
    pub fn new() -> Self {
        EventQueue {
            events: Vec::new(),
            edge_data: Vec::new(),
            first: 0,
            sorted: false,
        }
    }

    pub fn with_capacity(cap: usize) -> Self {
        EventQueue {
            events: Vec::with_capacity(cap),
            edge_data: Vec::with_capacity(cap),
            first: 0,
            sorted: false,
        }
    }

    pub fn reset(&mut self) {
        self.events.clear();
        self.edge_data.clear();
        self.first = 0;
        self.sorted = false;
    }

    pub fn from_path(tolerance: f32, path: impl Iterator<Item = PathEvent>) -> Self {
        let (min, max) = path.size_hint();
        let capacity = max.unwrap_or(min);
        let mut builder = EventQueueBuilder::with_capacity(capacity);
        builder.set_path(tolerance, path);

        builder.build()
    }

    pub fn from_path_with_ids(
        tolerance: f32,
        path: impl Iterator<Item = IdEvent>,
        positions: &impl PositionStore,
    ) -> Self {
        let (min, max) = path.size_hint();
        let capacity = max.unwrap_or(min);
        let mut builder = EventQueueBuilder::with_capacity(capacity);
        builder.set_path_with_ids(tolerance, path, positions);

        builder.build()
    }

    pub fn reserve(&mut self, n: usize) {
        self.events.reserve(n);
    }

    pub fn push(&mut self, position: Point) {
        if self.sorted {
            self.push_sorted(position);
        } else {
            self.push_unsorted(position);
        }
    }

    pub(crate) fn into_builder(mut self) -> EventQueueBuilder {
        self.reset();
        EventQueueBuilder {
            queue: self,
            current: point(f32::NAN, f32::NAN),
            prev: point(f32::NAN, f32::NAN),
            second: point(f32::NAN, f32::NAN),
            nth: 0,
            prev_evt_is_edge: false,
            tolerance: 0.1,
            prev_endpoint_id: EndpointId(std::u32::MAX),
        }
    }

    fn push_unsorted(&mut self, position: Point) {
        self.events.push(Event {
            position,
            next_sibling: INVALID_EVENT_ID,
            next_event: INVALID_EVENT_ID,
        });
    }

    fn push_sorted(&mut self, position: Point) {
        self.events.push(Event {
            position,
            next_sibling: u32::MAX,
            next_event: u32::MAX,
        });

        let id = (self.events.len() - 1) as u32;
        let mut current = self.first_id();
        let mut prev = current;
        while self.valid_id(current) {
            let evt_pos = self.events[current as usize].position;
            match compare_positions(position, evt_pos) {
                Ordering::Greater => {}
                Ordering::Equal => {
                    // Add to sibling list.
                    let mut current_sibling = current;
                    let mut next_sibling = self.next_sibling_id(current);
                    while self.valid_id(next_sibling) {
                        current_sibling = next_sibling;
                        next_sibling = self.next_sibling_id(current_sibling);
                    }
                    self.events[current_sibling as usize].next_sibling = id;
                    return;
                }
                Ordering::Less => {
                    if prev != current {
                        // Insert between `prev` and `current`.
                        self.events[prev as usize].next_event = id;
                    } else {
                        // It's the first element.
                        self.first = id;
                    }
                    self.events[id as usize].next_event = current;
                    return;
                }
            }

            prev = current;
            current = self.next_id(current);
        }

        // Append at the end.
        self.events[prev as usize].next_event = id;
    }

    // Could start searching at the tessellator's current event id.
    pub fn insert_sorted(&mut self, position: Point, data: EdgeData, after: TessEventId) -> TessEventId {
        debug_assert!(self.sorted);
        debug_assert!(is_after(data.to, position));

        let idx = self.events.len() as TessEventId;
        self.events.push(Event {
            position,
            next_sibling: INVALID_EVENT_ID,
            next_event: INVALID_EVENT_ID,
        });
        self.edge_data.push(data);

        let mut prev = after;
        let mut current = after;
        while self.valid_id(current) {
            let pos = self.events[current as usize].position;

            if pos == position {
                debug_assert!(prev != current);
                self.events[idx as usize].next_sibling = self.events[current as usize].next_sibling;
                self.events[current as usize].next_sibling = idx;
                break;
            } else if is_after(pos, position) {
                self.events[prev as usize].next_event = idx;
                self.events[idx as usize].next_event = current;
                break;
            }

            prev = current;
            current = self.next_id(current);
        }

        idx
    }

    pub fn insert_sibling(&mut self, sibling: TessEventId, position: Point, data: EdgeData) {
        let idx = self.events.len() as TessEventId;
        let next_sibling = self.events[sibling as usize].next_sibling;

        self.events.push(Event {
            position,
            next_event: INVALID_EVENT_ID,
            next_sibling,
        });

        self.edge_data.push(data);

        self.events[sibling as usize].next_sibling = idx;
    }

    pub fn clear(&mut self) {
        self.events.clear();
        self.first = 0;
        self.sorted = false;
    }

    pub fn first_id(&self) -> TessEventId { self.first }

    pub fn next_id(&self, id: TessEventId) -> TessEventId { self.events[id as usize].next_event }

    pub fn next_sibling_id(&self, id: TessEventId) -> TessEventId { self.events[id as usize].next_sibling }

    pub fn valid_id(&self, id: TessEventId) -> bool { (id as usize) < self.events.len() }

    pub fn position(&self, id: TessEventId) -> Point { self.events[id as usize].position }

    fn sort(&mut self) {
        self.sorted = true;

        if self.events.is_empty() {
            return;
        }

        let range = 0..self.events.len();
        self.first = self.merge_sort(range);
    }

    /// Merge sort with two twists:
    /// - Events at the same position are grouped into a "sibling" list.
    /// - We take advantage of having events stored contiguously in a vector
    ///   by recursively splitting ranges of the array instead of traversing
    ///   the lists to find a split point.
    fn merge_sort(&mut self, range: Range<usize>) -> TessEventId {
        let split = (range.start + range.end) / 2;

        if split == range.start {
            return range.start as TessEventId;
        }

        let a_head = self.merge_sort(range.start..split);
        let b_head = self.merge_sort(split..range.end);

        self.merge(a_head, b_head)
    }

    fn merge(&mut self, a: TessEventId, b: TessEventId) -> TessEventId {
        if a == INVALID_EVENT_ID {
            return b;
        }
        if b == INVALID_EVENT_ID {
            return a;
        }

        debug_assert!(a != b);

        return match compare_positions(self.events[a as usize].position, self.events[b as usize].position) {
            Ordering::Less => {
                let a_next = self.events[a as usize].next_event;
                self.events[a as usize].next_event = self.merge(a_next, b);

                a
            }
            Ordering::Greater => {
                let b_next = self.events[b as usize].next_event;
                self.events[b as usize].next_event = self.merge(a, b_next);

                b
            }
            Ordering::Equal => {
                // Add b to a's sibling list.
                let a_sib = self.find_last_sibling(a) as usize;
                self.events[a_sib].next_sibling = b;

                let b_next = self.events[b as usize].next_event;
                self.merge(a, b_next)
            }
        };
    }

    fn find_last_sibling(&self, id: TessEventId) -> TessEventId {
        let mut current_sibling = id;
        let mut next_sibling = self.next_sibling_id(id);
        while self.valid_id(next_sibling) {
            current_sibling = next_sibling;
            next_sibling = self.next_sibling_id(current_sibling);
        }

        current_sibling
    }

    #[cfg(debug_assertions)]
    fn log(&self) {
        let mut iter_count = self.events.len() * self.events.len();

        println!("--");
        let mut current = self.first;
        while (current as usize) < self.events.len() {
            assert!(iter_count > 0);
            iter_count -= 1;

            print!("[");
            let mut current_sibling = current;
            while (current_sibling as usize) < self.events.len() {
                print!("{:?},", self.events[current_sibling as usize].position);
                current_sibling = self.events[current_sibling as usize].next_sibling;
            }
            print!("]  ");
            current = self.events[current as usize].next_event;
        }
        println!("\n--");
    }

    fn assert_sorted(&self) {
        let mut current = self.first;
        let mut pos = point(f32::MIN, f32::MIN);
        let mut n = 0;
        while self.valid_id(current) {
            assert!(is_after(self.events[current as usize].position, pos));
            pos = self.events[current as usize].position;
            let mut current_sibling = current;
            while self.valid_id(current_sibling) {
                n += 1;
                assert_eq!(self.events[current_sibling as usize].position, pos);
                current_sibling = self.next_sibling_id(current_sibling);
            }
            current = self.next_id(current);
        }
        assert_eq!(n, self.events.len());
    }
}

pub(crate) struct EventQueueBuilder {
    current: Point,
    prev: Point,
    second: Point,
    nth: u32,
    queue: EventQueue,
    prev_evt_is_edge: bool,
    tolerance: f32,
    prev_endpoint_id: EndpointId,
}

impl EventQueueBuilder {
    pub fn new() -> Self {
        EventQueue::new().into_builder()
    }

    pub fn with_capacity(cap: usize) -> Self {
        EventQueue::with_capacity(cap).into_builder()
    }

    pub fn build(mut self) -> EventQueue {
        debug_assert!(!self.prev_evt_is_edge);

        self.queue.sort();

        self.queue
    }

    pub fn set_path(&mut self, tolerance: f32, path: impl Iterator<Item=PathEvent>) {
        self.tolerance = tolerance;
        let mut evt_id = path::EventId(0);
        let mut endpoint_id = EndpointId(0);
        for evt in path {
            match evt {
                PathEvent::Begin { at } => {
                    self.begin(at);
                }
                PathEvent::Line { to, .. } => {
                    self.line_segment(to, endpoint_id, evt_id, 0.0, 1.0);
                }
                PathEvent::Quadratic { ctrl, to, .. } => {
                    self.quadratic_bezier_segment(
                        ctrl,
                        to,
                        endpoint_id,
                        evt_id,
                    );
                }
                PathEvent::Cubic { ctrl1, ctrl2, to, .. } => {
                    self.cubic_bezier_segment(
                        ctrl1,
                        ctrl2,
                        to,
                        endpoint_id,
                        evt_id,
                    );
                }
                PathEvent::End { first, .. } => {
                    self.end(first, endpoint_id, evt_id);
                }
            }

            evt_id.0 += 1;
            self.prev_endpoint_id = endpoint_id;
            endpoint_id.0 += 1;
        }

        // Should finish with an end event.
        debug_assert!(!self.prev_evt_is_edge);
    }

    pub fn set_path_with_ids(
        &mut self,
        tolerance: f32,
        path_events: impl Iterator<Item=IdEvent>,
        points: &impl PositionStore,
    ) {
        self.tolerance = tolerance;
        for evt in path_events {
            match evt {
                IdEvent::Begin { at } => {
                    self.begin(points.endpoint_position(at));
                    self.prev_endpoint_id = at;
                }
                IdEvent::Line { to, edge, .. } => {
                    self.line_segment(
                        points.endpoint_position(to), to,
                        edge,
                        0.0, 1.0,
                    );
                    self.prev_endpoint_id = to;
                }
                IdEvent::Quadratic { ctrl, to, edge, .. } => {
                    self.quadratic_bezier_segment(
                        points.ctrl_point_position(ctrl),
                        points.endpoint_position(to),
                        to,
                        edge,
                    );
                    self.prev_endpoint_id = to;
                }
                IdEvent::Cubic { ctrl1, ctrl2, to, edge, .. } => {
                    self.cubic_bezier_segment(
                        points.ctrl_point_position(ctrl1),
                        points.ctrl_point_position(ctrl2),
                        points.endpoint_position(to),
                        to,
                        edge,
                    );
                    self.prev_endpoint_id = to;
                }
                IdEvent::End { first, edge, .. } => {
                    self.end(points.endpoint_position(first), first, edge);
                    self.prev_endpoint_id = first;
                }
            }
        }

        // Should finish with an end event.
        debug_assert!(!self.prev_evt_is_edge);
    }

    fn vertex_event(&mut self, at: Point, endpoint_id: EndpointId, evt_id: path::EventId) {
        self.queue.push(at);
        self.queue.edge_data.push(EdgeData {
            to: point(f32::NAN, f32::NAN),
            range: 0.0..0.0,
            winding: 0,
            is_edge: false,
            evt_id,
            from_id: endpoint_id,
            to_id: endpoint_id,
        });
    }

    fn vertex_event_on_curve(
        &mut self,
        at: Point,
        t: f32,
        from_id: EndpointId,
        to_id: EndpointId,
        evt_id: path::EventId,
    ) {
        self.queue.push(at);
        self.queue.edge_data.push(EdgeData {
            to: point(f32::NAN, f32::NAN),
            range: t..t,
            winding: 0,
            is_edge: false,
            evt_id,
            from_id,
            to_id,
        });
    }

    fn end(&mut self, first: Point, first_endpoint_id: EndpointId, evt_id: path::EventId) {
        if self.nth == 0 {
            return;
        }

        // Unless we are already back to the first point, we need to
        // to insert an edge.
        if self.current != first {
            self.line_segment(first, first_endpoint_id, evt_id, 0.0, 1.0);
        }

        // Since we can only check for the need of a vertex event when
        // we have a previous edge, we skipped it for the first edge
        // and have to do it now.
        if is_after(first, self.prev) && is_after(first, self.second) {
            self.vertex_event(first, first_endpoint_id, evt_id);
        }

        self.prev_evt_is_edge = false;

        self.nth = 0;
    }

    fn begin(&mut self, to: Point) {
        debug_assert!(!self.prev_evt_is_edge);

        self.nth = 0;
        self.current = to;
    }

    fn add_edge(
        &mut self,
        from: Point,
        to: Point,
        mut winding: i16,
        evt_id: path::EventId,
        from_id: EndpointId,
        to_id: EndpointId,
        mut t0: f32,
        mut t1: f32,
    ) {
        let mut evt_pos = from;
        let mut evt_to = to;
        if is_after(evt_pos, to) {
            evt_to = evt_pos;
            evt_pos = to;
            swap(&mut t0, &mut t1);
            //swap(&mut from_id, &mut to_id);
            winding *= -1;
        }

        self.queue.push(evt_pos);
        self.queue.edge_data.push(EdgeData {
            to: evt_to,
            range: t0..t1,
            winding,
            is_edge: true,
            evt_id,
            from_id,
            to_id,
        });

        self.nth += 1;
        self.prev_evt_is_edge = true;
    }

    fn line_segment(
        &mut self,
        to: Point,
        to_id: EndpointId,
        evt_id: path::EventId,
        t0: f32, t1: f32,
    ) {
        debug_assert!(evt_id != path::EventId::INVALID);

        let from = self.current;
        if from == to {
            return;
        }

        if is_after(from, to) {
            if self.nth > 0 && is_after(from, self.prev) {
                self.vertex_event(from, self.prev_endpoint_id, evt_id);
            }
        }

        if self.nth == 0 {
            self.second = to;
        }

        self.add_edge(
            from, to,
            1,
            evt_id,
            self.prev_endpoint_id,
            to_id,
            t0, t1
        );

        self.prev = self.current;
        self.prev_endpoint_id = to_id;
        self.current = to;
    }

    fn quadratic_bezier_segment(
        &mut self,
        ctrl: Point,
        to: Point,
        to_id: EndpointId,
        evt_id: path::EventId,
    ) {
        // Swap the curve so that it always goes downwards. This way if two
        // paths share the same edge with different windings, the flattening will
        // play out the same way, which avoid cracks.

        // We have to put special care into properly tracking the previous and second
        // points as if we hadn't swapped.

        let original = QuadraticBezierSegment {
            from: self.current,
            ctrl,
            to,
        };

        let needs_swap = is_after(original.from, original.to);

        let mut segment = original;
        let mut winding = 1;
        if needs_swap {
            swap(&mut segment.from, &mut segment.to);
            winding = -1;
        }

        let mut t0 = 0.0;
        let mut prev = segment.from;
        let mut from = segment.from;
        let mut first = None;
        let is_first_edge = self.nth == 0;
        segment.for_each_flattened_with_t(self.tolerance, &mut|to, t1| {
            if first == None {
                first = Some(to)
                // We can't call vertex(prev, from, to) in the first iteration
                // because if we flipped the curve, we don't have a proper value for
                // the previous vertex yet.
                // We'll handle it after the loop.
            } else if is_after(from, to) && is_after(from, prev) {
                self.vertex_event_on_curve(
                    from,
                    t0,
                    self.prev_endpoint_id,
                    to_id,
                    evt_id,
                );
            }

            self.add_edge(
                from, to,
                winding,
                evt_id,
                self.prev_endpoint_id,
                to_id,
                t0, t1,
            );

            t0 = t1;
            prev = from;
            from = to;
        });

        let first = first.unwrap();
        let (second, previous) = if needs_swap { (prev, first) } else { (first, prev) };

        if is_first_edge {
            self.second = second;
        } else if is_after(original.from, self.prev) && is_after(original.from, second) {
            // Handle the first vertex we took out of the loop above.
            // The missing vertex is always the origin of the edge (before the flip).
            self.vertex_event(original.from, self.prev_endpoint_id, evt_id);
        }

        self.prev = previous;
        self.current = original.to;
    }

    fn cubic_bezier_segment(
        &mut self,
        ctrl1: Point,
        ctrl2: Point,
        to: Point,
        to_id: EndpointId,
        evt_id: path::EventId,
    ) {
        // Swap the curve so that it always goes downwards. This way if two
        // paths share the same edge with different windings, the flattening will
        // play out the same way, which avoid cracks.

        // We have to put special care into properly tracking the previous and second
        // points as if we hadn't swapped.

        let original = CubicBezierSegment {
            from: self.current,
            ctrl1,
            ctrl2,
            to,
        };

        let needs_swap = is_after(original.from, original.to);

        let mut segment = original;
        let mut winding = 1;
        if needs_swap {
            swap(&mut segment.from, &mut segment.to);
            swap(&mut segment.ctrl1, &mut segment.ctrl2);
            winding = -1;
        }

        let mut t0 = 0.0;
        let mut prev = segment.from;
        let mut from = segment.from;
        let mut first = None;
        let is_first_edge = self.nth == 0;
        segment.for_each_flattened_with_t(self.tolerance, &mut|to, t1| {
            if first == None {
                first = Some(to)
                // We can't call vertex(prev, from, to) in the first iteration
                // because if we flipped the curve, we don't have a proper value for
                // the previous vertex yet.
                // We'll handle it after the loop.
            } else if is_after(from, to) && is_after(from, prev) {
                self.vertex_event_on_curve(
                    from,
                    t0,
                    self.prev_endpoint_id,
                    to_id,
                    evt_id,
                );
            }

            self.add_edge(
                from, to,
                winding,
                evt_id,
                self.prev_endpoint_id,
                to_id,
                t0, t1,
            );

            t0 = t1;
            prev = from;
            from = to;
        });

        let first = first.unwrap();
        let (second, previous) = if needs_swap { (prev, first) } else { (first, prev) };

        if is_first_edge {
            self.second = second;
        } else if is_after(original.from, self.prev) && is_after(original.from, second) {
            // Handle the first vertex we took out of the loop above.
            // The missing vertex is always the origin of the edge (before the flip).
            self.vertex_event(original.from, self.prev_endpoint_id, evt_id);
        }

        self.prev = previous;
        self.current = original.to;
    }
}

#[test]
fn test_event_queue_sort_1() {
    let mut queue = EventQueue::new();
    queue.push(point(0.0, 0.0));
    queue.push(point(4.0, 0.0));
    queue.push(point(2.0, 0.0));
    queue.push(point(3.0, 0.0));
    queue.push(point(4.0, 0.0));
    queue.push(point(0.0, 0.0));
    queue.push(point(6.0, 0.0));

    queue.sort();
    queue.assert_sorted();
}

#[test]
fn test_event_queue_sort_2() {
    let mut queue = EventQueue::new();
    queue.push(point(0.0, 0.0));
    queue.push(point(0.0, 0.0));
    queue.push(point(0.0, 0.0));
    queue.push(point(0.0, 0.0));

    queue.sort();
    queue.assert_sorted();
}

#[test]
fn test_event_queue_sort_3() {
    let mut queue = EventQueue::new();
    queue.push(point(0.0, 0.0));
    queue.push(point(1.0, 0.0));
    queue.push(point(2.0, 0.0));
    queue.push(point(3.0, 0.0));
    queue.push(point(4.0, 0.0));
    queue.push(point(5.0, 0.0));

    queue.sort();
    queue.assert_sorted();
}

#[test]
fn test_event_queue_sort_4() {
    let mut queue = EventQueue::new();
    queue.push(point(5.0, 0.0));
    queue.push(point(4.0, 0.0));
    queue.push(point(3.0, 0.0));
    queue.push(point(2.0, 0.0));
    queue.push(point(1.0, 0.0));
    queue.push(point(0.0, 0.0));

    queue.sort();
    queue.assert_sorted();
}

#[test]
fn test_event_queue_sort_5() {
    let mut queue = EventQueue::new();
    queue.push(point(5.0, 0.0));
    queue.push(point(5.0, 0.0));
    queue.push(point(4.0, 0.0));
    queue.push(point(4.0, 0.0));
    queue.push(point(3.0, 0.0));
    queue.push(point(3.0, 0.0));
    queue.push(point(2.0, 0.0));
    queue.push(point(2.0, 0.0));
    queue.push(point(1.0, 0.0));
    queue.push(point(1.0, 0.0));
    queue.push(point(0.0, 0.0));
    queue.push(point(0.0, 0.0));

    queue.sort();
    queue.assert_sorted();
}

#[test]
fn test_event_queue_push_sorted() {
    let mut queue = EventQueue::new();
    queue.push(point(5.0, 0.0));
    queue.push(point(4.0, 0.0));
    queue.push(point(3.0, 0.0));
    queue.push(point(2.0, 0.0));
    queue.push(point(1.0, 0.0));
    queue.push(point(0.0, 0.0));

    queue.sort();
    queue.push_sorted(point(1.5, 0.0));
    queue.assert_sorted();

    queue.push_sorted(point(2.5, 0.0));
    queue.assert_sorted();

    queue.push_sorted(point(2.5, 0.0));
    queue.assert_sorted();

    queue.push_sorted(point(6.5, 0.0));
    queue.assert_sorted();
}

#[test]
fn test_logo() {
    use crate::path::{Path, builder::Build};

    let mut path = Path::builder().with_svg();

    crate::extra::rust_logo::build_logo_path(&mut path);
    let path = path.build();

    crate::extra::debugging::find_reduced_test_case(
        path.as_slice(),
        &|path: Path| {
            let _ = EventQueue::from_path(0.05, path.iter());
            true
        },
    );
}