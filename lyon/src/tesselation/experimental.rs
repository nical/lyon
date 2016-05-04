use std::f32::consts::PI;
use std::cmp::{ Ordering };
use std::mem::swap;

use tesselation::{ vertex_id, VertexId };
use tesselation::path::*;
use tesselation::vectors::Position2D;
use tesselation::sweep_line::{ is_below, intersect_segment_with_horizontal };
use tesselation::vertex_builder::{ VertexBufferBuilder, VertexBuffers, simple_vertex_builder };
use tesselation::bentley_ottmann::compute_segment_intersection;

use vodk_math::{ Vec2, vec2 };

pub struct Event {
    pub current_position: Vec2,
    pub previous_position: Vec2,
    pub next_position: Vec2,
    pub current: ComplexVertexId,
    pub previous: ComplexVertexId,
    pub next: ComplexVertexId,
}

pub struct Intersection {
    position: Vec2,
    a_down_pos: Vec2,
    b_down_pos: Vec2,
    //a_up: ComplexVertexId,
    a_down: ComplexVertexId,
    //b_up: ComplexVertexId,
    b_down: ComplexVertexId,
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

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum Side { Left, Right }

impl Side {
    pub fn opposite(self) -> Side {
        match self {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
        }
    }

    pub fn is_left(self) -> bool { self == Side::Left }

    pub fn is_right(self) -> bool { self == Side::Right }
}

#[derive(Copy, Clone, Debug)]
struct Edge {
    upper_position: Vec2,
    lower_position: Vec2,
    upper: ComplexVertexId,
    lower: Option<ComplexVertexId>,
    lower2: Option<ComplexVertexId>,
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

    fn end(&mut self, pos: Vec2, id: ComplexVertexId) {
        self.monotone_tesselator.end(pos, id.vertex_id);
    }
}

struct SweepLine {
    spans: Vec<Span>
}

pub struct Tesselator<'l, Output: VertexBufferBuilder<Vec2>+'l> {
    path: ComplexPathSlice<'l>,
    sweep_line: SweepLine,
    intersections: Vec<Intersection>,
    next_new_vertex: ComplexVertexId,
    output: &'l mut Output,
}

impl<'l, Output: VertexBufferBuilder<Vec2>> Tesselator<'l, Output> {
    pub fn new(path: ComplexPathSlice<'l>, output: &'l mut Output) -> Tesselator<'l, Output> {
        Tesselator {
            path: path,
            sweep_line: SweepLine {
                spans: Vec::with_capacity(16),
            },
            intersections: Vec::new(),
            next_new_vertex: ComplexVertexId {
                vertex_id: vertex_id(0),
                path_id: path_id(path.num_sub_paths() as u16),
            },
            output: output,
        }
    }

    pub fn tesselate(&mut self, sorted_events: SortedEventSlice<'l>) {

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

            for inter in &self.intersections {
                if is_below(evt.current_position, inter.position) {
                    println!(" Should process intersection at {:?} ", inter.position);
                }
            }
            self.on_event(&evt);
        }
    }

    fn on_event(&mut self, event: &Event) {
        println!(" ------ Event {:?}", event.current);

        let below_prev = is_below(event.current_position, event.previous_position);
        let below_next = is_below(event.current_position, event.next_position);

        if below_prev && below_next {
            return self.on_down_event(event);
        }

        if !below_prev && !below_next {
            return self.on_up_event(event);
        }

        return self.on_regular_event(event);
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

        if side.is_left() {
            //println!(" ++++++ Left event {}", event.current.vertex_id.handle);

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
            //println!(" ++++++ Right event {}", event.current.vertex_id.handle);
        }

        self.insert_sweep_line_edge(
            span_index, side,
            event.current_position, event.current,
            next_pos, next,
        );
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
            lower2: Some(event.previous),
            upper_position: event.current_position,
            lower_position: event.previous_position,
        };
        let mut r = Edge {
            upper: event.current,
            lower: Some(event.next),
            lower2: Some(event.next),
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
            //println!(" ++++++ Start event {}", event.current.vertex_id.handle);
            // Start event.

            // TODO: use a function that checks intersections for l and r
            self.sweep_line.spans.insert(span_index, Span::begin(l, r));
        }
    }

    fn on_split_event(&mut self, event: &Event, span_index: usize, l: &Edge, r: &Edge) {
        //println!(" ++++++ Split event {}", event.current.vertex_id.handle);
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
                lower2: Some(event.current),
                lower_position: event.current_position,
            };

            self.sweep_line.spans.insert(span_index, Span::begin(ll, r2));

            self.insert_sweep_line_edge(span_index, Side::Right,
                event.current_position, event.current,
                l.lower_position, l.lower.unwrap(),
            );
            self.insert_sweep_line_edge(span_index+1, Side::Left,
                event.current_position, event.current,
                r.lower_position, r.lower.unwrap(),
            );
        }
    }

    fn find_span_down(&self, event: &Event) -> (usize, bool) {
        let mut span_index = 0;
        for span in &self.sweep_line.spans {

            //println!(" ** search {} left: {:?} | right: {:?}", event.current.vertex_id.handle,
            //  span.left.lower, span.right.lower);

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
        //println!(" ++++++ End event {}", event.current.vertex_id.handle);

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
        //println!(" ++++++ Merge event {}", event.current.vertex_id.handle);
        assert!(span_index < self.sweep_line.spans.len()-1);

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

    fn insert_sweep_line_edge(&mut self,
        span_index: usize, side: Side,
        up_pos: Vec2, up_id: ComplexVertexId,
        down_pos: Vec2, down_id: ComplexVertexId,
    ) {
        //println!(" -- insert_sweep_line_edge");
        let mut idx = 0;
        for span in &self.sweep_line.spans {

            if !span.left.has_merge_vertex() && (idx != span_index || side.is_right()) {
                if let Some(intersection) = compute_segment_intersection(
                    span.left.upper_position, span.left.lower_position,
                    up_pos, down_pos,
                ) {
                    println!(" -- found an intersection at {:?}", intersection);
                    let intersection = Intersection {
                        position: intersection,
                        a_down_pos: down_pos,
                        b_down_pos: span.left.lower_position,
                        a_down: down_id,
                        b_down: span.left.lower.unwrap(),
                    };

                    self.intersections.push(intersection);
                    self.intersections.sort_by(|a, b| {
                        let va = a.position;
                        let vb = b.position;
                        if va.y > vb.y { return Ordering::Greater; }
                        if va.y < vb.y { return Ordering::Less; }
                        if va.x > vb.x { return Ordering::Greater; }
                        if va.x < vb.x { return Ordering::Less; }
                        return Ordering::Equal;
                    });
                }
            }

            if !span.right.has_merge_vertex() && (idx != span_index || side.is_left()) {
                if let Some(intersection) = compute_segment_intersection(
                    span.right.upper_position, span.right.lower_position,
                    up_pos, down_pos,
                ) {
                    println!(" -- found an intersection at {:?}", intersection);
                    let intersection = Intersection {
                        position: intersection,
                        a_down_pos: down_pos,
                        b_down_pos: span.right.lower_position,
                        a_down: down_id,
                        b_down: span.right.lower.unwrap(),
                    };
                }
            }

            idx += 1;
        }
        self.sweep_line.spans[span_index].vertex(up_pos, up_id, down_pos, down_id, side);
    }

    fn gen_vertex_id(&mut self) -> ComplexVertexId {
        let ret = self.next_new_vertex;
        self.next_new_vertex = ComplexVertexId {
            vertex_id: vertex_id(self.next_new_vertex.vertex_id.handle + 1),
            path_id: self.next_new_vertex.path_id,
        };
        return ret;
    }

    fn on_itersection_event(&mut self, intersection: &Intersection) {
        let mut span_index = 0;
        let mut side = Side::Left;
        let mut is_a = false;
        for span in &self.sweep_line.spans {
            if span.left.lower2 == Some(intersection.a_down) {
                side = Side::Left;
                is_a = true;
                break;
            }
            if span.left.lower2 == Some(intersection.b_down) {
                side = Side::Left;
                is_a = false;
                break;
            }

            if span.right.lower2 == Some(intersection.a_down) {
                side = Side::Right;
                is_a = true;
                break;
            }
            if span.right.lower2 == Some(intersection.b_down) {
                side = Side::Right;
                is_a = false;
                break;
            }

            span_index += 1;
        }

        if side.is_left() { // TODO this is wrong
/*
*/ 
            let vertex_id = self.gen_vertex_id();
            self.on_regular_event(&Event{
                current_position: intersection.position,
                next_position: if is_a { intersection.a_down_pos } else { intersection.b_down_pos },
                next: if is_a { intersection.a_down } else { intersection.b_down },
                previous_position: intersection.position + vec2(0.0, -1.0), // >_<
                previous: intersection.a_down, // should not matter
                current: vertex_id,
            });
        }
    }

    fn end_span(&mut self, span_index: usize, event: &Event) {
        self.sweep_line.spans[span_index].end(event.current_position, event.current);
        self.sweep_line.spans[span_index].monotone_tesselator.flush(self.output);
        self.sweep_line.spans.remove(span_index);
    }
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
        let current = MonotoneVertex{ pos: pos, id: id, side: side };
        let right_side = current.side == Side::Right;

        assert!(is_below(current.pos, self.previous.pos));
        assert!(!self.stack.is_empty());

        let changed_side = current.side != self.previous.side;

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
        //println!(" #### triangle {} {} {}", a.id.handle, b.id.handle, c.id.handle);

        if (c.pos - b.pos).directed_angle(a.pos - b.pos) <= PI {
            self.triangles.push((a.id.handle, b.id.handle, c.id.handle));
        } else {
            self.triangles.push((b.id.handle, a.id.handle, c.id.handle));
        }
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

pub fn tesselate_fill<'l, Output: VertexBufferBuilder<Vec2>>(
    path: ComplexPathSlice<'l>,
    output: &mut Output
) -> Result<(), ()> {
    output.begin_geometry();

    for v in path.vertices().as_slice() {
        output.push_vertex(v.position());
    }

    let events = EventVector::from_path(path);
    let mut tess = Tesselator::new(path, output);
    tess.tesselate(events.as_slice());

    return Ok(());
}

#[cfg(test)]
fn test_path(path: ComplexPathSlice, expected_triangle_count: Option<usize>) {
    let mut buffers: VertexBuffers<Vec2> = VertexBuffers::new();
    {
        let mut vertex_builder = simple_vertex_builder(&mut buffers);
        let events = EventVector::from_path(path);
        let mut tess = Tesselator::new(path, &mut vertex_builder);
        tess.tesselate(events.as_slice());
    }
    if let Some(num_triangles) = expected_triangle_count {
        assert_eq!(buffers.indices.len()/3, num_triangles);
    }
}

#[cfg(test)]
fn test_path_with_rotations(path: ComplexPath, step: f32, expected_triangle_count: Option<usize>) {
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

        angle += step;
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
    test_path(path.as_slice(), Some(4));
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
    test_path_with_rotations(path, 0.001, Some(3));
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
    test_path_with_rotations(path, 0.001, Some(4));
}

#[test]
fn test_tesselator_simple_aligned() {
    let mut path = ComplexPath::new();
    PathBuilder::begin(&mut path, vec2(0.0, 0.0)).flattened()
        .line_to(vec2(1.0, 0.0))
        .line_to(vec2(2.0, 0.0))
        .line_to(vec2(2.0, 1.0))
        .line_to(vec2(2.0, 2.0))
        .line_to(vec2(1.0, 2.0))
        .line_to(vec2(0.0, 2.0))
        .line_to(vec2(0.0, 1.0))
        .close();
    test_path_with_rotations(path, 0.001, Some(6));
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
    test_path_with_rotations(path, 0.001, Some(4));
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
    test_path_with_rotations(path, 0.001, Some(10));
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

    test_path_with_rotations(path, 0.001, Some(6));
}

#[test]
fn test_tesselator_degenerate_empty() {
    test_path(ComplexPath::new().as_slice(), Some(0));
}

#[test]
fn test_tesselator_degenerate_same_position() {
    let mut path = ComplexPath::new();
    PathBuilder::begin(&mut path, vec2(0.0, 0.0)).flattened()
        .line_to(vec2(0.0, 0.0))
        .line_to(vec2(0.0, 0.0))
        .line_to(vec2(0.0, 0.0))
        .line_to(vec2(0.0, 0.0))
        .line_to(vec2(0.0, 0.0))
        .close();
    test_path_with_rotations(path, 0.001, None);
}

#[test]
fn test_tesselator_auto_intersection() {
    //  x.___
    //   \   'x
    //    \ /
    //     o  <-- intersection!
    //    / \
    //  x.___\
    //       'x
    let mut path = ComplexPath::new();
    PathBuilder::begin(&mut path, vec2(0.0, 0.0)).flattened()
        .line_to(vec2(2.0, 1.0))
        .line_to(vec2(0.0, 2.0))
        .line_to(vec2(2.0, 3.0))
        .close();
    test_path(path.as_slice(), None);
    panic!();
}

#[test]
#[ignore]
fn test_tesselator_rust_logo() {
    let mut path = ComplexPath::new();

    PathBuilder::begin(&mut path, vec2(122.631, 69.716)).flattened()
        .relative_line_to(vec2(-4.394, -2.72))
        .relative_cubic_bezier_to(vec2(-0.037, -0.428), vec2(-0.079, -0.855), vec2(-0.125, -1.28))
        .relative_line_to(vec2(3.776, -3.522))
        .relative_cubic_bezier_to(vec2(0.384, -0.358), vec2(0.556, -0.888), vec2(0.452, -1.401))
        .relative_cubic_bezier_to(vec2(-0.101, -0.515), vec2(-0.462, -0.939), vec2(-0.953, -1.122))
        .relative_line_to(vec2(-4.827, -1.805))
        .relative_cubic_bezier_to(vec2(-0.121, -0.418), vec2(-0.248, -0.833), vec2(-0.378, -1.246))
        .relative_line_to(vec2(3.011, -4.182))
        .relative_cubic_bezier_to(vec2(0.307, -0.425), vec2(0.37, -0.978), vec2(0.17, -1.463))
        .relative_cubic_bezier_to(vec2(-0.2, -0.483), vec2(-0.637, -0.829), vec2(-1.154, -0.914))
        .relative_line_to(vec2(-5.09, -0.828))
        .relative_cubic_bezier_to(vec2(-0.198, -0.386), vec2(-0.404, -0.766), vec2(-0.612, -1.143))
        .relative_line_to(vec2(2.139, -4.695))
        .relative_cubic_bezier_to(vec2(0.219, -0.478), vec2(0.174, -1.034), vec2(-0.118, -1.468))
        .relative_cubic_bezier_to(vec2(-0.291, -0.436), vec2(-0.784, -0.691), vec2(-1.31, -0.671))
        .relative_line_to(vec2(-5.166, 0.18))
        .relative_cubic_bezier_to(vec2(-0.267, -0.334), vec2(-0.539, -0.665), vec2(-0.816, -0.99))
        .relative_line_to(vec2(1.187, -5.032))
        .relative_cubic_bezier_to(vec2(0.12, -0.511), vec2(-0.031, -1.046), vec2(-0.403, -1.417))
        .relative_cubic_bezier_to(vec2(-0.369, -0.37), vec2(-0.905, -0.523), vec2(-1.416, -0.403))
        .relative_line_to(vec2(-5.031, 1.186))
        .relative_cubic_bezier_to(vec2(-0.326, -0.276), vec2(-0.657, -0.549), vec2(-0.992, -0.816))
        .relative_line_to(vec2(0.181, -5.166))
        .relative_cubic_bezier_to(vec2(0.02, -0.523), vec2(-0.235, -1.02), vec2(-0.671, -1.31))
        .relative_cubic_bezier_to(vec2(-0.437, -0.292), vec2(-0.99, -0.336), vec2(-1.467, -0.119))
        .relative_line_to(vec2(-4.694, 2.14))
        .relative_cubic_bezier_to(vec2(-0.379, -0.208), vec2(-0.759, -0.414), vec2(-1.143, -0.613))
        .relative_line_to(vec2(-0.83, -5.091))
        .relative_cubic_bezier_to(vec2(-0.084, -0.516), vec2(-0.43, -0.954), vec2(-0.914, -1.154))
        .relative_cubic_bezier_to(vec2(-0.483, -0.201), vec2(-1.037, -0.136), vec2(-1.462, 0.17))
        .relative_line_to(vec2(-4.185, 3.011))
        .relative_cubic_bezier_to(vec2(-0.412, -0.131), vec2(-0.826, -0.257), vec2(-1.244, -0.377))
        .relative_line_to(vec2(-1.805, -4.828))
        .relative_cubic_bezier_to(vec2(-0.183, -0.492), vec2(-0.607, -0.853), vec2(-1.122, -0.955))
        .relative_cubic_bezier_to(vec2(-0.514, -0.101), vec2(-1.043, 0.07), vec2(-1.4, 0.452))
        .relative_line_to(vec2(-3.522, 3.779))
        .relative_cubic_bezier_to(vec2(-0.425, -0.047), vec2(-0.853, -0.09), vec2(-1.28, -0.125))
        .relative_line_to(vec2(-2.72, -4.395))
        .relative_cubic_bezier_to(vec2(-0.275, -0.445), vec2(-0.762, -0.716), vec2(-1.286, -0.716))
        .relative_cubic_bezier_to_s(vec2(-1.011, 0.271), vec2(-1.285, 0.716))
        .relative_line_to(vec2(-2.72, 4.395))
        .relative_cubic_bezier_to(vec2(-0.428, 0.035), vec2(-0.856, 0.078), vec2(-1.281, 0.125))
        .relative_line_to(vec2(-3.523, -3.779))
        .relative_cubic_bezier_to(vec2(-0.357, -0.382), vec2(-0.887, -0.553), vec2(-1.4, -0.452))
        .relative_cubic_bezier_to(vec2(-0.515, 0.103), vec2(-0.939, 0.463), vec2(-1.122, 0.955))
        .relative_line_to(vec2(-1.805, 4.828))
        .relative_cubic_bezier_to(vec2(-0.418, 0.12), vec2(-0.832, 0.247), vec2(-1.245, 0.377))
        .relative_line_to(vec2(-4.184, -3.011))
        .relative_cubic_bezier_to(vec2(-0.425, -0.307), vec2(-0.979, -0.372), vec2(-1.463, -0.17))
        .relative_cubic_bezier_to(vec2(-0.483, 0.2), vec2(-0.83, 0.638), vec2(-0.914, 1.154))
        .relative_line_to(vec2(-0.83, 5.091))
        .relative_cubic_bezier_to(vec2(-0.384, 0.199), vec2(-0.764, 0.404), vec2(-1.143, 0.613))
        .relative_line_to(vec2(-4.694, -2.14))
        .relative_cubic_bezier_to(vec2(-0.477, -0.218), vec2(-1.033, -0.173), vec2(-1.467, 0.119))
        .relative_cubic_bezier_to(vec2(-0.436, 0.29), vec2(-0.691, 0.787), vec2(-0.671, 1.31))
        .relative_line_to(vec2(0.18, 5.166))
        .relative_cubic_bezier_to(vec2(-0.334, 0.267), vec2(-0.665, 0.54), vec2(-0.992, 0.816))
        .relative_line_to(vec2(-5.031, -1.186))
        .relative_cubic_bezier_to(vec2(-0.511, -0.119), vec2(-1.047, 0.033), vec2(-1.417, 0.403))
        .relative_cubic_bezier_to(vec2(-0.372, 0.371), vec2(-0.523, 0.906), vec2(-0.403, 1.417))
        .relative_line_to(vec2(1.185, 5.032))
        .relative_cubic_bezier_to(vec2(-0.275, 0.326), vec2(-0.547, 0.656), vec2(-0.814, 0.99))
        .relative_line_to(vec2(-5.166, -0.18))
        .relative_cubic_bezier_to(vec2(-0.521, -0.015), vec2(-1.019, 0.235), vec2(-1.31, 0.671))
        .relative_cubic_bezier_to(vec2(-0.292, 0.434), vec2(-0.336, 0.99), vec2(-0.119, 1.468))
        .relative_line_to(vec2(2.14, 4.695))
        .relative_cubic_bezier_to(vec2(-0.208, 0.377), vec2(-0.414, 0.757), vec2(-0.613, 1.143))
        .relative_line_to(vec2(-5.09, 0.828))
        .relative_cubic_bezier_to(vec2(-0.517, 0.084), vec2(-0.953, 0.43), vec2(-1.154, 0.914))
        .relative_cubic_bezier_to(vec2(-0.2, 0.485), vec2(-0.135, 1.038), vec2(0.17, 1.463))
        .relative_line_to(vec2(3.011, 4.182))
        .relative_cubic_bezier_to(vec2(-0.131, 0.413), vec2(-0.258, 0.828), vec2(-0.378, 1.246))
        .relative_line_to(vec2(-4.828, 1.805))
        .relative_cubic_bezier_to(vec2(-0.49, 0.183), vec2(-0.851, 0.607), vec2(-0.953, 1.122))
        .relative_cubic_bezier_to(vec2(-0.102, 0.514), vec2(0.069, 1.043), vec2(0.452, 1.401))
        .relative_line_to(vec2(3.777, 3.522))
        .relative_cubic_bezier_to(vec2(-0.047, 0.425), vec2(-0.089, 0.853), vec2(-0.125, 1.28))
        .relative_line_to(vec2(-4.394, 2.72))
        .relative_cubic_bezier_to(vec2(-0.445, 0.275), vec2(-0.716, 0.761), vec2(-0.716, 1.286))
        .relative_cubic_bezier_to_s(vec2(0.271, 1.011), vec2(0.716, 1.285))
        .relative_line_to(vec2(4.394, 2.72))
        .relative_cubic_bezier_to(vec2(0.036, 0.428), vec2(0.078, 0.855), vec2(0.125, 1.28))
        .relative_line_to(vec2(-3.777, 3.523))
        .relative_cubic_bezier_to(vec2(-0.383, 0.357), vec2(-0.554, 0.887), vec2(-0.452, 1.4))
        .relative_cubic_bezier_to(vec2(0.102, 0.515), vec2(0.463, 0.938), vec2(0.953, 1.122))
        .relative_line_to(vec2(4.828, 1.805))
        .relative_cubic_bezier_to(vec2(0.12, 0.418), vec2(0.247, 0.833), vec2(0.378, 1.246))
        .relative_line_to(vec2(-3.011, 4.183))
        .relative_cubic_bezier_to(vec2(-0.306, 0.426), vec2(-0.371, 0.979), vec2(-0.17, 1.462))
        .relative_cubic_bezier_to(vec2(0.201, 0.485), vec2(0.638, 0.831), vec2(1.155, 0.914))
        .relative_line_to(vec2(5.089, 0.828))
        .relative_cubic_bezier_to(vec2(0.199, 0.386), vec2(0.403, 0.766), vec2(0.613, 1.145))
        .relative_line_to(vec2(-2.14, 4.693))
        .relative_cubic_bezier_to(vec2(-0.218, 0.477), vec2(-0.173, 1.032), vec2(0.119, 1.468))
        .relative_cubic_bezier_to(vec2(0.292, 0.437), vec2(0.789, 0.692), vec2(1.31, 0.671))
        .relative_line_to(vec2(5.164, -0.181))
        .relative_cubic_bezier_to(vec2(0.269, 0.336), vec2(0.54, 0.665), vec2(0.816, 0.992))
        .relative_line_to(vec2(-1.185, 5.033))
        .relative_cubic_bezier_to(vec2(-0.12, 0.51), vec2(0.031, 1.043), vec2(0.403, 1.414))
        .relative_cubic_bezier_to(vec2(0.369, 0.373), vec2(0.906, 0.522), vec2(1.417, 0.402))
        .relative_line_to(vec2(5.031, -1.185))
        .relative_cubic_bezier_to(vec2(0.327, 0.278), vec2(0.658, 0.548), vec2(0.992, 0.814))
        .relative_line_to(vec2(-0.18, 5.167))
        .relative_cubic_bezier_to(vec2(-0.02, 0.523), vec2(0.235, 1.019), vec2(0.671, 1.311))
        .relative_cubic_bezier_to(vec2(0.434, 0.291), vec2(0.99, 0.335), vec2(1.467, 0.117))
        .relative_line_to(vec2(4.694, -2.139))
        .relative_cubic_bezier_to(vec2(0.378, 0.21), vec2(0.758, 0.414), vec2(1.143, 0.613))
        .relative_line_to(vec2(0.83, 5.088))
        .relative_cubic_bezier_to(vec2(0.084, 0.518), vec2(0.43, 0.956), vec2(0.914, 1.155))
        .relative_cubic_bezier_to(vec2(0.483, 0.201), vec2(1.038, 0.136), vec2(1.463, -0.169))
        .relative_line_to(vec2(4.182, -3.013))
        .relative_cubic_bezier_to(vec2(0.413, 0.131), vec2(0.828, 0.259), vec2(1.246, 0.379))
        .relative_line_to(vec2(1.805, 4.826))
        .relative_cubic_bezier_to(vec2(0.183, 0.49), vec2(0.607, 0.853), vec2(1.122, 0.953))
        .relative_cubic_bezier_to(vec2(0.514, 0.104), vec2(1.043, -0.068), vec2(1.4, -0.452))
        .relative_line_to(vec2(3.523, -3.777))
        .relative_cubic_bezier_to(vec2(0.425, 0.049), vec2(0.853, 0.09), vec2(1.281, 0.128))
        .relative_line_to(vec2(2.72, 4.394))
        .relative_cubic_bezier_to(vec2(0.274, 0.443), vec2(0.761, 0.716), vec2(1.285, 0.716))
        .relative_cubic_bezier_to_s(vec2(1.011, -0.272), vec2(1.286, -0.716))
        .relative_line_to(vec2(2.72, -4.394))
        .relative_cubic_bezier_to(vec2(0.428, -0.038), vec2(0.855, -0.079), vec2(1.28, -0.128))
        .relative_line_to(vec2(3.522, 3.777))
        .relative_cubic_bezier_to(vec2(0.357, 0.384), vec2(0.887, 0.556), vec2(1.4, 0.452))
        .relative_cubic_bezier_to(vec2(0.515, -0.101), vec2(0.939, -0.463), vec2(1.122, -0.953))
        .relative_line_to(vec2(1.805, -4.826))
        .relative_cubic_bezier_to(vec2(0.418, -0.12), vec2(0.833, -0.248), vec2(1.246, -0.379))
        .relative_line_to(vec2(4.183, 3.013))
        .relative_cubic_bezier_to(vec2(0.425, 0.305), vec2(0.979, 0.37), vec2(1.462, 0.169))
        .relative_cubic_bezier_to(vec2(0.484, -0.199), vec2(0.83, -0.638), vec2(0.914, -1.155))
        .relative_line_to(vec2(0.83, -5.088))
        .relative_cubic_bezier_to(vec2(0.384, -0.199), vec2(0.764, -0.406), vec2(1.143, -0.613))
        .relative_line_to(vec2(4.694, 2.139))
        .relative_cubic_bezier_to(vec2(0.477, 0.218), vec2(1.032, 0.174), vec2(1.467, -0.117))
        .relative_cubic_bezier_to(vec2(0.436, -0.292), vec2(0.69, -0.787), vec2(0.671, -1.311))
        .relative_line_to(vec2(-0.18, -5.167))
        .relative_cubic_bezier_to(vec2(0.334, -0.267), vec2(0.665, -0.536), vec2(0.991, -0.814))
        .relative_line_to(vec2(5.031, 1.185))
        .relative_cubic_bezier_to(vec2(0.511, 0.12), vec2(1.047, -0.029), vec2(1.416, -0.402))
        .relative_cubic_bezier_to(vec2(0.372, -0.371), vec2(0.523, -0.904), vec2(0.403, -1.414))
        .relative_line_to(vec2(-1.185, -5.033))
        .relative_cubic_bezier_to(vec2(0.276, -0.327), vec2(0.548, -0.656), vec2(0.814, -0.992))
        .relative_line_to(vec2(5.166, 0.181))
        .relative_cubic_bezier_to(vec2(0.521, 0.021), vec2(1.019, -0.234), vec2(1.31, -0.671))
        .relative_cubic_bezier_to(vec2(0.292, -0.436), vec2(0.337, -0.991), vec2(0.118, -1.468))
        .relative_line_to(vec2(-2.139, -4.693))
        .relative_cubic_bezier_to(vec2(0.209, -0.379), vec2(0.414, -0.759), vec2(0.612, -1.145))
        .relative_line_to(vec2(5.09, -0.828))
        .relative_cubic_bezier_to(vec2(0.518, -0.083), vec2(0.954, -0.429), vec2(1.154, -0.914))
        .relative_cubic_bezier_to(vec2(0.2, -0.483), vec2(0.137, -1.036), vec2(-0.17, -1.462))
        .relative_line_to(vec2(-3.011, -4.183))
        .relative_cubic_bezier_to(vec2(0.13, -0.413), vec2(0.257, -0.828), vec2(0.378, -1.246))
        .relative_line_to(vec2(4.827, -1.805))
        .relative_cubic_bezier_to(vec2(0.491, -0.184), vec2(0.853, -0.607), vec2(0.953, -1.122))
        .relative_cubic_bezier_to(vec2(0.104, -0.514), vec2(-0.068, -1.043), vec2(-0.452, -1.4))
        .relative_line_to(vec2(-3.776, -3.523))
        .relative_cubic_bezier_to(vec2(0.046, -0.425), vec2(0.088, -0.853), vec2(0.125, -1.28))
        .relative_line_to(vec2(4.394, -2.72))
        .relative_cubic_bezier_to(vec2(0.445, -0.274), vec2(0.716, -0.761), vec2(0.716, -1.285))
        .cubic_bezier_to_s(vec2(123.076, 69.991), vec2(122.631, 69.716))
        .close();
    PathBuilder::begin(&mut path, vec2(93.222, 106.167)).flattened()
        .relative_cubic_bezier_to(vec2(-1.678, -0.362), vec2(-2.745, -2.016), vec2(-2.385, -3.699))
        .relative_cubic_bezier_to(vec2(0.359, -1.681), vec2(2.012, -2.751), vec2(3.689, -2.389))
        .relative_cubic_bezier_to(vec2(1.678, 0.359), vec2(2.747, 2.016), vec2(2.387, 3.696))
        .cubic_bezier_to_s(vec2(94.899, 106.526), vec2(93.222, 106.167))
        .close();
    PathBuilder::begin(&mut path, vec2(91.729, 96.069)).flattened()
        .relative_cubic_bezier_to(vec2(-1.531, -0.328), vec2(-3.037, 0.646), vec2(-3.365, 2.18))
        .relative_line_to(vec2(-1.56, 7.28))
        .relative_cubic_bezier_to(vec2(-4.814, 2.185), vec2(-10.16, 3.399), vec2(-15.79, 3.399))
        .relative_cubic_bezier_to(vec2(-5.759, 0.0), vec2(-11.221, -1.274), vec2(-16.121, -3.552))
        .relative_line_to(vec2(-1.559, -7.28))
        .relative_cubic_bezier_to(vec2(-0.328, -1.532), vec2(-1.834, -2.508), vec2(-3.364, -2.179))
        .relative_line_to(vec2(-6.427, 1.38))
        .relative_cubic_bezier_to(vec2(-1.193, -1.228), vec2(-2.303, -2.536), vec2(-3.323, -3.917))
        .relative_horizontal_line_to(31.272)
        .relative_cubic_bezier_to(vec2(0.354, 0.0), vec2(0.59, -0.064), vec2(0.59, -0.386))
        .vertical_line_to(81.932)
        .relative_cubic_bezier_to(vec2(0.0, -0.322), vec2(-0.236, -0.386), vec2(-0.59, -0.386))
        .relative_horizontal_line_to(-9.146)
        .relative_vertical_line_to(-7.012)
        .relative_horizontal_line_to(9.892)
        .relative_cubic_bezier_to(vec2(0.903, 0.0), vec2(4.828, 0.258), vec2(6.083, 5.275))
        .relative_cubic_bezier_to(vec2(0.393, 1.543), vec2(1.256, 6.562), vec2(1.846, 8.169))
        .relative_cubic_bezier_to(vec2(0.588, 1.802), vec2(2.982, 5.402), vec2(5.533, 5.402))
        .relative_horizontal_line_to(15.583)
        .relative_cubic_bezier_to(vec2(0.177, 0.0), vec2(0.366, -0.02), vec2(0.565, -0.056))
        .relative_cubic_bezier_to(vec2(-1.081, 1.469), vec2(-2.267, 2.859), vec2(-3.544, 4.158))
        .line_to(vec2(91.729, 96.069))
        .close();
    PathBuilder::begin(&mut path, vec2(48.477, 106.015)).flattened()
        .relative_cubic_bezier_to(vec2(-1.678, 0.362), vec2(-3.33, -0.708), vec2(-3.691, -2.389))
        .relative_cubic_bezier_to(vec2(-0.359, -1.684), vec2(0.708, -3.337), vec2(2.386, -3.699))
        .relative_cubic_bezier_to(vec2(1.678, -0.359), vec2(3.331, 0.711), vec2(3.691, 2.392))
        .cubic_bezier_to(vec2(51.222, 103.999), vec2(50.154, 105.655), vec2(48.477, 106.015))
        .close();
    PathBuilder::begin(&mut path, vec2(36.614, 57.91)).flattened()
        .relative_cubic_bezier_to(vec2(0.696, 1.571), vec2(-0.012, 3.412), vec2(-1.581, 4.107))
        .relative_cubic_bezier_to(vec2(-1.569, 0.697), vec2(-3.405, -0.012), vec2(-4.101, -1.584))
        .relative_cubic_bezier_to(vec2(-0.696, -1.572), vec2(0.012, -3.41), vec2(1.581, -4.107))
        .cubic_bezier_to(vec2(34.083, 55.63), vec2(35.918, 56.338), vec2(36.614, 57.91))
        .close();
    PathBuilder::begin(&mut path, vec2(32.968, 66.553)).flattened()
        .relative_line_to(vec2(6.695, -2.975))
        .relative_cubic_bezier_to(vec2(1.43, -0.635), vec2(2.076, -2.311), vec2(1.441, -3.744))
        .relative_line_to(vec2(-1.379, -3.118))
        .relative_horizontal_line_to(5.423)
        .vertical_line_to(81.16)
        .horizontal_line_to(34.207)
        .relative_cubic_bezier_to(vec2(-0.949, -3.336), vec2(-1.458, -6.857), vec2(-1.458, -10.496))
        .cubic_bezier_to(vec2(32.749, 69.275), vec2(32.824, 67.902), vec2(32.968, 66.553))
        .close();
    PathBuilder::begin(&mut path, vec2(62.348, 64.179)).flattened()
        .relative_vertical_line_to(-7.205)
        .relative_horizontal_line_to(12.914)
        .relative_cubic_bezier_to(vec2(0.667, 0.0), vec2(4.71, 0.771), vec2(4.71, 3.794))
        .relative_cubic_bezier_to(vec2(0.0, 2.51), vec2(-3.101, 3.41), vec2(-5.651, 3.41))
        //.horizontal_line_to(62.348) //TODO
        .close();
    PathBuilder::begin(&mut path, vec2(109.28, 70.664)).flattened()
        .relative_cubic_bezier_to(vec2(0.0, 0.956), vec2(-0.035, 1.902), vec2(-0.105, 2.841))
        .relative_horizontal_line_to(-3.926)
        .relative_cubic_bezier_to(vec2(-0.393, 0.0), vec2(-0.551, 0.258), vec2(-0.551, 0.643))
        .relative_vertical_line_to(1.803)
        .relative_cubic_bezier_to(vec2(0.0, 4.244), vec2(-2.393, 5.167), vec2(-4.49, 5.402))
        .relative_cubic_bezier_to(vec2(-1.997, 0.225), vec2(-4.211, -0.836), vec2(-4.484, -2.058))
        .relative_cubic_bezier_to(vec2(-1.178, -6.626), vec2(-3.141, -8.041), vec2(-6.241, -10.486))
        .relative_cubic_bezier_to(vec2(3.847, -2.443), vec2(7.85, -6.047), vec2(7.85, -10.871))
        .relative_cubic_bezier_to(vec2(0.0, -5.209), vec2(-3.571, -8.49), vec2(-6.005, -10.099))
        .relative_cubic_bezier_to(vec2(-3.415, -2.251), vec2(-7.196, -2.702), vec2(-8.216, -2.702))
        .horizontal_line_to(42.509)
        .relative_cubic_bezier_to(vec2(5.506, -6.145), vec2(12.968, -10.498), vec2(21.408, -12.082))
        .relative_line_to(vec2(4.786, 5.021))
        .relative_cubic_bezier_to(vec2(1.082, 1.133), vec2(2.874, 1.175), vec2(4.006, 0.092))
        .relative_line_to(vec2(5.355, -5.122))
        .relative_cubic_bezier_to(vec2(11.221, 2.089), vec2(20.721, 9.074), vec2(26.196, 18.657))
        .relative_line_to(vec2(-3.666, 8.28))
        .relative_cubic_bezier_to(vec2(-0.633, 1.433), vec2(0.013, 3.109), vec2(1.442, 3.744))
        .relative_line_to(vec2(7.058, 3.135))
        .cubic_bezier_to(vec2(109.216, 68.115), vec2(109.28, 69.381), vec2(109.28, 70.664))
        .close();
    PathBuilder::begin(&mut path, vec2(68.705, 28.784)).flattened()
        .relative_cubic_bezier_to(vec2(1.24, -1.188), vec2(3.207, -1.141), vec2(4.394, 0.101))
        .relative_cubic_bezier_to(vec2(1.185, 1.245), vec2(1.14, 3.214), vec2(-0.103, 4.401))
        .relative_cubic_bezier_to(vec2(-1.24, 1.188), vec2(-3.207, 1.142), vec2(-4.394, -0.102))
        .cubic_bezier_to(vec2(67.418, 31.941), vec2(67.463, 29.972), vec2(68.705, 28.784))
        .close();
    PathBuilder::begin(&mut path, vec2(105.085, 58.061)).flattened()
        .relative_cubic_bezier_to(vec2(0.695, -1.571), vec2(2.531, -2.28), vec2(4.1, -1.583))
        .relative_cubic_bezier_to(vec2(1.569, 0.696), vec2(2.277, 2.536), vec2(1.581, 4.107))
        .relative_cubic_bezier_to(vec2(-0.695, 1.572), vec2(-2.531, 2.281), vec2(-4.101, 1.584))
        .cubic_bezier_to(vec2(105.098, 61.473), vec2(104.39, 59.634), vec2(105.085, 58.061))
        .close();

    test_path_with_rotations(path, 0.011, None);
}