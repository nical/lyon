use std::f32::consts::PI;
use std::cmp::{Ordering, PartialOrd};
use std::mem::swap;

use tesselation::{ vertex_id, VertexId, VertexSlice };
use tesselation::path::*;
use tesselation::vectors::Position2D;
use tesselation::sweep_line::{ EventType, is_below, intersect_segment_with_horizontal };
use tesselation::vertex_builder::{ VertexBufferBuilder };
use tesselation::vertex_builder::{ VertexBuffers, simple_vertex_builder, };

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
        debug_assert_eq!(self.events.len(), path.vertices().len());

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

impl Edge {
    fn has_merge_vertex(&self) -> bool { self.lower.is_none() }
}

struct Span {
    left: Edge,
    right: Edge,
    monotone_tesselator: MonotoneTesselator,
}

impl Span {
    fn begin(l: Edge, r: Edge) -> Span {
        Span {
            left: l, right: r,
            monotone_tesselator: MonotoneTesselator::begin(l.upper_position, l.upper.vertex_id),
        }
    }

    fn vertex(&mut self,
        pos: Vec2, id: ComplexVertexId,
        next_pos: Vec2, next_id: ComplexVertexId,
        side: Side
    ) {
        self.set_upper_vertex(pos, id, side);
        self.set_lower_vertex(next_pos, next_id, side);
    }

    fn merge_vertex(&mut self, pos: Vec2, id: ComplexVertexId, side: Side) {
        self.set_upper_vertex(pos, id, side);
        self.set_no_lower_vertex(side);
    }

    fn set_upper_vertex(&mut self, pos: Vec2, id: ComplexVertexId, side: Side) {
        {
            let mut edge = self.mut_edge(side);

            //if let Some(n) = edge.lower {
            //    assert_eq!(n, id);
            //}

            edge.upper = id;
            edge.upper_position = pos;
        }

        self.monotone_tesselator.vertex(pos, id.vertex_id, side);
    }

    fn set_lower_vertex(&mut self, pos: Vec2, id: ComplexVertexId, side: Side) {
       let mut edge = self.mut_edge(side);

        edge.lower = Some(id);
        edge.lower_position = pos;
    }

    fn set_no_lower_vertex(&mut self, side: Side) {
        self.mut_edge(side).lower = None;
    }

    fn edge(&self, side: Side) -> &Edge {
        return match side {
            Side::Left => { &self.left }
            Side::Right => { &self.right }
        };
    }

    fn mut_edge(&mut self, side: Side) -> &mut Edge {
        return match side {
            Side::Left => { &mut self.left }
            Side::Right => { &mut self.right }
        };
    }

    fn end(&mut self,
        pos: Vec2, id: ComplexVertexId,
    ) {
        self.monotone_tesselator.end(pos, id.vertex_id);
    }
}

struct SweepLine {
    spans: Vec<Span>
}

pub struct Tesselator<'l, Output: VertexBufferBuilder<Vec2>+'l> {
    path: ComplexPathSlice<'l>,
    sweep_line: SweepLine,
    output: &'l mut Output,
}

impl<'l, Output: VertexBufferBuilder<Vec2>> Tesselator<'l, Output> {
    pub fn new(path: ComplexPathSlice<'l>, output: &'l mut Output) -> Tesselator<'l, Output> {
        Tesselator {
            path: path,
            sweep_line: SweepLine {
                spans: Vec::with_capacity(16),
            },
            output: output,
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
        println!(" ------ Event {:?}", event.current);
        let base_evt_type = compute_base_event_type(
            event.previous_position, event.current_position, event.next_position
        );

        match base_evt_type {
            BaseEventType::Regular => { self.on_regular_event(event); }
            BaseEventType::Down => { self.on_down_event(event); }
            BaseEventType::Up => { self.on_up_event(event); }
        }

        return Ok(());
    }

    fn find_span_regular(&self, event: &Event) -> (usize, Side) {
        let mut span_index = 0;
        for span in &self.sweep_line.spans {
            if span.left.lower == Some(event.current) {
                return (span_index, Side::Left);
            } else if span.right.lower == Some(event.current) {
                return (span_index, Side::Right);
            }
            span_index += 1;
        }
        // unreachable
        panic!();
    }

    fn on_regular_event(&mut self, event: &Event) {
        let (span_index, side) = self.find_span_regular(event);

        let next_below = is_below(event.next_position, event.current_position);
        let (next, next_pos) = if next_below { (event.next, event.next_position) }
                               else { (event.previous, event.previous_position) };

        if side == Side::Left {
            println!(" ++++++ Left event {}", event.current.vertex_id.handle);

            if self.sweep_line.spans[span_index].right.has_merge_vertex() {
                //     \ /
                //  \   x   <-- merge vertex
                //   \ :
                // ll x   <-- current vertex
                //     \r
                self.sweep_line.spans[span_index+1].set_lower_vertex(event.current_position, event.current, Side::Left);
                self.end_span(span_index, event);
            }

        } else {
            println!(" ++++++ Right event {}", event.current.vertex_id.handle);
        }

        self.sweep_line.spans[span_index].vertex(
            event.current_position, event.current,
            next_pos, next,
            side
        );
    }

    fn on_split_event(&mut self, event: &Event, span_index: usize, l: &Edge, r: &Edge) {
        println!(" ++++++ Split event {}", event.current.vertex_id.handle);
        //println!("left {:?} | right {:?}", l.lower.unwrap().vertex_id.handle, r.lower.unwrap().vertex_id.handle);

        // look whether the span shares a merge vertex with the previous one
        if self.sweep_line.spans[span_index].left.has_merge_vertex() {
            let left_span = span_index-1;
            let right_span = span_index;
            //            \ /
            //             x   <-- merge vertex
            //  left_span  :  righ_span
            //             x   <-- current split vertex
            //           l/ \r
            self.sweep_line.spans[left_span].vertex(
                event.current_position, event.current,
                l.lower_position, l.lower.unwrap(),
                Side::Right,
            );
            self.sweep_line.spans[right_span].vertex(
                event.current_position, event.current,
                r.lower_position, r.lower.unwrap(),
                Side::Left,
            );
        } else {
            //      /
            //     x
            //    / :r2
            // ll/   x   <-- current split vertex
            //     l/ \r
            let ll = self.sweep_line.spans[span_index].left;
            let r2 = Edge {
                upper: ll.upper,
                upper_position: ll.upper_position,
                lower: Some(event.current),
                lower_position: event.current_position,
            };
            self.sweep_line.spans.insert(span_index, Span::begin(ll, r2));
            self.sweep_line.spans[span_index].vertex(
                event.current_position, event.current,
                l.lower_position, l.lower.unwrap(),
                Side::Right,
            );
            self.sweep_line.spans[span_index+1].vertex(
                event.current_position, event.current,
                r.lower_position, r.lower.unwrap(),
                Side::Left,
            );
        }
    }

    fn find_span_up(&self, event: &Event) -> (usize, bool) {
        let (x, y) = event.current_position.tuple();
        let mut span_index = 0;
        for span in &self.sweep_line.spans {
            if span.left.lower.is_some() {
                let lx = intersect_segment_with_horizontal(
                    span.left.upper_position,
                    span.left.lower_position,
                    y
                );
                if lx > x {
                    return (span_index, false); // outside
                }
            }
            if span.right.lower.is_some() {
                let rx = intersect_segment_with_horizontal(
                    span.right.upper_position,
                    span.right.lower_position,
                    y
                );
                if rx > x {
                    return (span_index, true); // inside
                }
            }
            span_index += 1;
        }

        return (span_index, false);
    }

    fn on_up_event(&mut self, event: &Event) {
        let (span_index, is_inside) = self.find_span_up(event);

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

        let angle = (event.previous_position - event.current_position).directed_angle(
            event.next_position - event.current_position
        );

        if angle < PI {
            swap(&mut l, &mut r);
        }

        if is_inside {
            self.on_split_event(event, span_index, &l, &r)
        } else {
            println!(" ++++++ Start event {}", event.current.vertex_id.handle);
            // Start event.
            self.sweep_line.spans.insert(span_index, Span::begin(l, r));
        }
    }


    fn find_span_down(&self, event: &Event) -> (usize, bool) {
        let mut span_index = 0;
        for span in &self.sweep_line.spans {

            println!(" ** search {} left: {:?} | right: {:?}", event.current.vertex_id.handle,
                span.left.lower, span.right.lower);

            if span.left.lower == Some(event.current) {
                return (span_index, true);
            }

            if span.right.lower == Some(event.current) {
                return (span_index, false);
            }

            span_index += 1;
        }
        // unreachable
        panic!();
    }

    fn on_down_event(&mut self, event: &Event) {
        let (span_index, is_end) = self.find_span_down(event);

        assert!(span_index < self.sweep_line.spans.len());

        if is_end {
            self.on_end_event(event, span_index);
        } else {
            self.on_merge_event(event, span_index);
        }
    }

    fn on_end_event(&mut self, event: &Event, span_index: usize) {
        println!(" ++++++ End event {}", event.current.vertex_id.handle);

        if self.sweep_line.spans[span_index].right.has_merge_vertex() {
            //   \ /
            //  \ x   <-- merge vertex
            //   \:/
            //    x   <-- current vertex
            self.end_span(span_index, event);
        }

        self.end_span(span_index, event);
    }

    fn on_merge_event(&mut self, event: &Event, span_index: usize) {
        println!(" ++++++ Merge event {}", event.current.vertex_id.handle);
        assert!(span_index < self.sweep_line.spans.len()-1);

        // TODO: do we actually need this one?
        //if self.sweep_line.spans[span_index].left.has_merge_vertex() {
        //    //  \ / \
        //    //   x-. \ /  <-- merge vertex
        //    //      '-x    <-- current merge vertex
        //    self.sweep_line.spans[span_index-1].set_lower_vertex(event.current_position, event.current, Side::Right);
        //    self.end_span(span_index+1, event);
        //}

        if self.sweep_line.spans[span_index].right.has_merge_vertex() {
            //     / \ /
            //  \ / .-x    <-- merge vertex
            //   x-'      <-- current merge vertex
            self.sweep_line.spans[span_index+2].set_lower_vertex(event.current_position, event.current, Side::Left);
            self.end_span(span_index+1, event);
        }

        debug_assert!(self.sweep_line.spans[span_index+1].left.lower == Some(event.current));

        self.sweep_line.spans[span_index].merge_vertex(
            event.current_position, event.current, Side::Right
        );
        self.sweep_line.spans[span_index+1].merge_vertex(
            event.current_position, event.current, Side::Left
        );
    }

    fn end_span(&mut self, span_index: usize, event: &Event) {
        self.sweep_line.spans[span_index].end(event.current_position, event.current);
        self.sweep_line.spans[span_index].monotone_tesselator.flush(self.output);
        self.sweep_line.spans.remove(span_index);
    }
}

pub fn compute_base_event_type(prev: Vec2, current: Vec2, next: Vec2) -> BaseEventType {
    let interrior_angle = (prev - current).directed_angle(next - current);

    let below_prev = is_below(current, prev);
    let below_next = is_below(current, next);

    if below_prev && below_next {
        return BaseEventType::Down;
    }

    if !below_prev && !below_next {
        return BaseEventType::Up;
    }

    return BaseEventType::Regular;
}

/// helper class that generates a triangulation from a sequence of vertices describing a monotone
/// polygon.
struct MonotoneTesselator {
    stack: Vec<MonotoneVertex>,
    previous: MonotoneVertex,
    triangles: Vec<(u16, u16, u16)>,
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
            stack: Vec::with_capacity(16),
            triangles: Vec::with_capacity(128),
            previous: first,
        };

        tess.stack.push(first);

        return tess;
    }

    pub fn vertex(&mut self, pos: Vec2, id: VertexId, side: Side) {
        let mut current = MonotoneVertex{ pos: pos, id: id, side: side };

        assert!(is_below(current.pos, self.previous.pos));
        assert!(!self.stack.is_empty());

        let changed_side = current.side != self.previous.side;
        let right_side = current.side == Side::Right;

        if changed_side {
            for i in 0..(self.stack.len() - 1) {
                let mut a = self.stack[i];
                let mut b = self.stack[i+1];

                if right_side {
                    swap(&mut a, &mut b);
                }

                self.push_triangle(&a, &b, &current);
            }
            self.stack.clear();
            self.stack.push(self.previous);
        } else {
            let mut last_popped = self.stack.pop();
            while !self.stack.is_empty() {
                let mut a = last_popped.unwrap();
                let mut b = *self.stack.last().unwrap();

                if right_side {
                    swap(&mut a, &mut b);
                }

                if (current.pos - b.pos).directed_angle(a.pos - b.pos) <= PI {
                    self.push_triangle(&a, &b, &current);
                    last_popped = self.stack.pop();
                } else {
                    break;
                }
            }
            if let Some(item) = last_popped {
                self.stack.push(item);
            }
        }

        self.stack.push(current);
        self.previous = current;
    }

    pub fn end(&mut self, pos: Vec2, id: VertexId) {
        let side = self.previous.side.opposite();
        self.vertex(pos, id, side);
        self.stack.clear();
    }

    fn push_triangle(&mut self, a: &MonotoneVertex, b: &MonotoneVertex, c: &MonotoneVertex) {
        println!(" #### triangle {} {} {}", a.id.handle, b.id.handle, c.id.handle);

        debug_assert!((c.pos - b.pos).directed_angle(a.pos - b.pos) <= PI);

        self.triangles.push((a.id.handle, b.id.handle, c.id.handle));
    }

    fn flush<Output: VertexBufferBuilder<Vec2>>(&mut self, output: &mut Output) {
        for &(a, b, c) in &self.triangles {
            output.push_indices(a, b, c);
        }
        self.triangles.clear();
    }
}

#[test]
fn test_monotone_tess() {
    println!(" ------------ ");
    {
        let mut tess = MonotoneTesselator::begin(vec2(0.0, 0.0), vertex_id(0));
        tess.vertex(vec2(-1.0, 1.0), vertex_id(1), Side::Left);
        tess.end(vec2(1.0, 2.0), vertex_id(2));
        assert_eq!(tess.triangles.len(), 1);
    }
    println!(" ------------ ");
    {
        let mut tess = MonotoneTesselator::begin(vec2(0.0, 0.0), vertex_id(0));
        tess.vertex(vec2(1.0, 1.0), vertex_id(1), Side::Right);
        tess.vertex(vec2(-1.5, 2.0), vertex_id(2), Side::Left);
        tess.vertex(vec2(-1.0, 3.0), vertex_id(3), Side::Left);
        tess.vertex(vec2(1.0, 4.0), vertex_id(4), Side::Right);
        tess.end(vec2(0.0, 5.0), vertex_id(5));
        assert_eq!(tess.triangles.len(), 4);
    }
    println!(" ------------ ");
    {
        let mut tess = MonotoneTesselator::begin(vec2(0.0, 0.0), vertex_id(0));
        tess.vertex(vec2(1.0, 1.0), vertex_id(1), Side::Right);
        tess.vertex(vec2(3.0, 2.0), vertex_id(2), Side::Right);
        tess.vertex(vec2(1.0, 3.0), vertex_id(3), Side::Right);
        tess.vertex(vec2(1.0, 4.0), vertex_id(4), Side::Right);
        tess.vertex(vec2(4.0, 5.0), vertex_id(5), Side::Right);
        tess.end(vec2(0.0, 6.0), vertex_id(6));
        assert_eq!(tess.triangles.len(), 5);
    }
    println!(" ------------ ");
    {
        let mut tess = MonotoneTesselator::begin(vec2(0.0, 0.0), vertex_id(0));
        tess.vertex(vec2(-1.0, 1.0), vertex_id(1), Side::Left);
        tess.vertex(vec2(-3.0, 2.0), vertex_id(2), Side::Left);
        tess.vertex(vec2(-1.0, 3.0), vertex_id(3), Side::Left);
        tess.vertex(vec2(-1.0, 4.0), vertex_id(4), Side::Left);
        tess.vertex(vec2(-4.0, 5.0), vertex_id(5), Side::Left);
        tess.end(vec2(0.0, 6.0), vertex_id(6));
        assert_eq!(tess.triangles.len(), 5);
    }
    println!(" ------------ ");
}

#[cfg(test)]
fn test_path(path: ComplexPathSlice, expected_triangle_count: usize) {
    let mut buffers: VertexBuffers<Vec2> = VertexBuffers::new();
    {
        let mut vertex_builder = simple_vertex_builder(&mut buffers);
        let events = EventVector::from_path(path);
        let mut tess = Tesselator::new(path, &mut vertex_builder);
        tess.tesselate(events.as_slice());
    }
    assert_eq!(buffers.indices.len()/3, expected_triangle_count);
}

#[cfg(test)]
fn test_path_with_rotations(mut path: ComplexPath, expected_triangle_count: usize) {
    let mut angle = 0.0;

    while angle < PI * 2.0 {
        let mut tranformed_path = path.clone();
        let cos = angle.cos();
        let sin = angle.sin();
        for v in tranformed_path.mut_vertices() {
            let (x, y) = (v.position.x, v.position.y);
            v.position.x = x*cos + y*sin;
            v.position.y = y*cos - x*sin;
        }
        println!("\n\n ==================== angle = {}", angle);
        test_path(tranformed_path.as_slice(), expected_triangle_count);

        angle += 0.001;
    }
}

#[test]
fn test_tesselator_simple_monotone() {
    let mut path = ComplexPath::new();
    PathBuilder::begin(&mut path, vec2(0.0, 0.0)).flattened()
        .line_to(vec2(-1.0, 1.0))
        .line_to(vec2(-3.0, 2.0))
        .line_to(vec2(-1.0, 3.0))
        .line_to(vec2(-4.0, 5.0))
        .line_to(vec2( 0.0, 6.0))
        .close();
    test_path(path.as_slice(), 4);
}

#[test]
fn test_tesselator_simple_split() {
    let mut path = ComplexPath::new();
    PathBuilder::begin(&mut path, vec2(0.0, 0.0)).flattened()
        .line_to(vec2(2.0, 1.0))
        .line_to(vec2(2.0, 3.0))
        .line_to(vec2(1.0, 2.0))
        .line_to(vec2(0.0, 3.0))
        .close();
    test_path_with_rotations(path, 3);
}

#[test]
fn test_tesselator_simple_merge_split() {
    let mut path = ComplexPath::new();
    PathBuilder::begin(&mut path, vec2(0.0, 0.0)).flattened()
        .line_to(vec2(1.0, 1.0))
        .line_to(vec2(2.0, 0.0))
        .line_to(vec2(2.0, 3.0))
        .line_to(vec2(1.0, 2.0))
        .line_to(vec2(0.0, 3.0))
        .close();
    test_path_with_rotations(path, 4);
}

#[test]
fn test_tesselator_simple_1() {
    let mut path = ComplexPath::new();
    PathBuilder::begin(&mut path, vec2(0.0, 0.0)).flattened()
        .line_to(vec2(1.0, 1.0))
        .line_to(vec2(2.0, 0.0))
        .line_to(vec2(1.0, 3.0))
        .line_to(vec2(0.5, 4.0))
        .line_to(vec2(0.0, 3.0))
        .close();
    test_path_with_rotations(path, 4);
}

#[test]
fn test_tesselator_simple_2() {
    let mut path = ComplexPath::new();
    PathBuilder::begin(&mut path, vec2(0.0, 0.0)).flattened()
        .line_to(vec2(1.0, 0.0))
        .line_to(vec2(2.0, 0.0))
        .line_to(vec2(3.0, 0.0))
        .line_to(vec2(3.0, 1.0))
        .line_to(vec2(3.0, 2.0))
        .line_to(vec2(3.0, 3.0))
        .line_to(vec2(2.0, 3.0))
        .line_to(vec2(1.0, 3.0))
        .line_to(vec2(0.0, 3.0))
        .line_to(vec2(0.0, 2.0))
        .line_to(vec2(0.0, 1.0))
        .close();
    test_path_with_rotations(path, 10);
}

#[test]
fn test_tesselator_hole_1() {
    let mut path = ComplexPath::new();
    PathBuilder::begin(&mut path, vec2(-11.0, 5.0)).flattened()
        .line_to(vec2(0.0, -5.0))
        .line_to(vec2(10.0, 5.0))
        .close();
    PathBuilder::begin(&mut path, vec2(-5.0, 2.0)).flattened()
        .line_to(vec2(0.0, -2.0))
        .line_to(vec2(4.0, 2.0))
        .close();

    test_path_with_rotations(path, 6);
}
