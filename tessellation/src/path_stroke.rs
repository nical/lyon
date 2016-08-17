//! Tessellation routines for path stroke operations.
//!
//! The current implementation is pretty bad and does not deal with overlap in an svg-compliant
//! way.

use math::*;
use lyon_core::{ FlattenedEvent };
//use lyon_path_builder::{ PrimitiveBuilder };
use geometry_builder::{ VertexId, GeometryBuilder, Count, };
use math_utils::{ tangent, directed_angle, directed_angle2, line_intersection, };

pub type StrokeResult = Result<Count, ()>;

pub struct StrokeTessellator {}

impl StrokeTessellator {
    pub fn new() -> StrokeTessellator { StrokeTessellator {} }

    pub fn tessellate<Input, Output>(&mut self, input: Input, options: &StrokeOptions, builder: &mut Output) -> StrokeResult
    where Input: Iterator<Item=FlattenedEvent>, Output: GeometryBuilder<Point> {
        let zero = Point::new(0.0, 0.0);
        return StrokingContext {
            first: zero,
            second: zero,
            previous: zero,
            current: zero,
            previous_a_id: VertexId(0),
            previous_b_id: VertexId(0),
            second_a_id: VertexId(0),
            second_b_id: VertexId(0),
            nth: 0,
            stroke_width: options.stroke_width,
            output: builder
        }.tessellate(input);
    }
}

struct StrokingContext<'l, Output:'l> {
    first: Point,
    previous: Point,
    current: Point,
    second: Point,
    previous_a_id: VertexId,
    previous_b_id: VertexId,
    second_a_id: VertexId,
    second_b_id: VertexId,
    nth: u32,
    stroke_width: f32,
    output: &'l mut Output,
}

impl<'l, Output:'l + GeometryBuilder<Point>> StrokingContext<'l, Output> {

    fn tessellate<Input>(&mut self, input: Input) -> StrokeResult
    where Input: Iterator<Item=FlattenedEvent> {
        self.output.begin_geometry();

        self.nth = 0;
        for evt in input {
            match evt {
                FlattenedEvent::LineTo(to) => { self.line_to(to); }
                FlattenedEvent::MoveTo(to) => { self.move_to(to); }
                FlattenedEvent::Close => { self.close(); }
            }
        }

        return Ok(self.output.end_geometry());
    }

    pub fn move_to(&mut self, to: Point) {
        // TODO: implement line caps!
        self.first = to;
        self.current = to;
        self.nth = 0;
    }

    pub fn line_to(&mut self, to: Point) {
        let width = self.stroke_width;
        self.edge_to(to, width);
    }

    pub fn close(&mut self) {
        let width = self.stroke_width;
        let first = self.first;
        self.edge_to(first, width);
        if self.nth > 1 {
            let second = self.second;
            self.edge_to(second, width);
            self.output.add_triangle(self.previous_b_id, self.previous_a_id, self.second_b_id);
            self.output.add_triangle(self.previous_a_id, self.second_a_id, self.second_b_id);
        }
        self.nth = 0;
        self.current = self.first;
    }

    fn edge_to(&mut self, to: Point, width: f32) {
        if self.current == to {
            return;
        }
        if self.nth == 0 {
            // We don't have enough information to compute a and b yet.
            self.previous = self.first;
            self.current = to;
            self.nth += 1;
            return;
        }
        let (a, b, c_opt) = get_angle_info(self.previous, self.current, to, width);
        let a_id = self.output.add_vertex(a);
        let b_id = self.output.add_vertex(b);
        let (c, c_id) = if let Some(c) = c_opt { (c, self.output.add_vertex(c)) } else { (b, b_id) };

        if self.nth > 1 {
            self.output.add_triangle(self.previous_b_id, self.previous_a_id, b_id);
            self.output.add_triangle(self.previous_a_id, a_id, b_id);
        }

        self.previous = self.current;
        self.previous_a_id = a_id;
        self.previous_b_id = c_id;
        self.current = to;

        if self.nth == 1 {
            self.second = self.previous;
            self.second_a_id = a_id;
            self.second_b_id = c_id;
        }

        if c_opt.is_some() {
            self.tessellate_angle(a, a_id, b, b_id, c, c_id);
        }

        self.nth += 1;
    }

    fn tessellate_angle(&mut self, _a: Point, a_id: VertexId, _b: Point, b_id: VertexId, _c: Point, c_id: VertexId) {
        // TODO: Properly support all types of angles.
        self.output.add_triangle(b_id, a_id, c_id);
    }
}

fn get_angle_info(previous: Point, current: Point, next: Point, width: f32) -> (Point, Point, Option<Point>) {
    let amount = width * 0.5;
    let n1 = tangent(current - previous) * amount;
    let n2 = tangent(next - current) * amount;

    // Segment P1-->PX
    let pn1  = previous + n1; // prev extruded along the tangent n1
    let pn1x = current + n1; // px extruded along the tangent n1
    // Segment PX-->P2
    let pn2  = next + n2;
    let pn2x = current + n2;

    let inter = match line_intersection(pn1, pn1x, pn2x, pn2) {
        Some(v) => { v }
        None => {
            if (n1 - n2).square_length() < 0.000001 {
                pn1x
            } else {
                // TODO: the angle is very narrow, use rounded corner instead
                //panic!("Not implemented yet");
                println!("!! narrow angle at {:?} {:?} {:?} | {:?} {:?} {:?}",
                    current, directed_angle(n1, n2), directed_angle2(current, previous, next),
                    previous, current, next,
                );
                current + (current - previous) * amount / (current - previous).length()
            }
        }
    };
    let a = current + current - inter;
    return (inter, a, None);
}

/// Line cap as defined by the SVG specification.
///
/// See: https://svgwg.org/specs/strokes/#StrokeLinecapProperty
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum LineCap {
    Butt,
    Round,
    Square
}

/// Line join as defined by the SVG specification.
///
/// See: https://svgwg.org/specs/strokes/#StrokeLinejoinProperty
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum LineJoin {
    Miter,
    MiterClip,
    Round,
    Bevel,
    Arcs,
}

/// Parameters for the tessellator.
///
/// Not used yet (only one configuration supported).
pub struct StrokeOptions {
    /// Thickness of the stroke.
    pub stroke_width: f32,

    /// See the SVG secification.
    pub line_cap: LineCap,

    /// See the SVG secification.
    pub line_join: LineJoin,

    /// See the SVG secification.
    pub miter_limit: f32,

    /// Maximum allowed distance to the path when building an approximation.
    pub tolerance: f32,

    /// The number of tesselator units per world unit.
    ///
    /// As the tesselator is internally using integer coordinates, this parameter defines
    /// the precision and range of the tesselator.
    pub unit_scale: f32,

    /// An anti-aliasing trick extruding a 1-px wide strip around the edges with
    /// a gradient to smooth the edges.
    ///
    /// Not implemented yet!
    pub vertex_aa: bool,

    // To be able to add fields without making it a breaking change, add an empty private field
    // which makes it impossible to create a StrokeOptions without the calling constructor.
    _private: (),
}

impl StrokeOptions {
    pub fn stroke_width(stroke_width: f32) -> StrokeOptions {
        StrokeOptions {
            stroke_width: stroke_width,
            line_cap: LineCap::Butt,
            line_join: LineJoin::Miter,
            miter_limit: 10.0,
            tolerance: 0.1,
            unit_scale: 1000.0,
            vertex_aa: false,
            _private: (),
        }
    }

    pub fn default() -> StrokeOptions { StrokeOptions::stroke_width(1.0) }

    pub fn with_tolerance(mut self, tolerance: f32) -> StrokeOptions {
        self.tolerance = tolerance;
        return self;
    }

    pub fn with_line_cap(mut self, cap: LineCap) -> StrokeOptions {
        self.line_cap = cap;
        return self;
    }

    pub fn with_line_join(mut self, join: LineJoin) -> StrokeOptions {
        self.line_join = join;
        return self;
    }

    pub fn with_miter_limit(mut self, limit: f32) -> StrokeOptions {
        self.miter_limit = limit;
        return self;
    }

    pub fn with_stroke_width(mut self, width: f32) -> StrokeOptions {
        self.stroke_width = width;
        return self;
    }

    pub fn with_unit_scale(mut self, scale: f32) -> StrokeOptions {
        self.unit_scale = scale;
        return self;
    }

    pub fn with_vertex_aa(mut self) -> StrokeOptions {
        self.vertex_aa = true;
        return self;
    }
}
