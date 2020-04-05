//! Tools to build path objects from a sequence of imperative commands.
//!
//! ## Examples
//!
//! The following example shows the Builder struct from the
//! [lyon_path](https://docs.rs/lyon_path/*/lyon_path) crate using the
//! [PathBuilder](trait.PathBuilder.html) interface.
//!
//! ```ignore
//! let mut builder = Path::builder();
//!
//! builder.move_to(point(0.0, 0.0));
//! builder.line_to(point(1.0, 0.0));
//! builder.quadratic_bezier_to(point(2.0, 0.0), point(2.0, 1.0));
//! builder.cubic_bezier_to(point(2.0, 2.0), point(0.0, 2.0), point(0.0, 0.0));
//! builder.close();
//!
//! let path = builder.build();
//! ```
//!
//! To build a path that approximates curves with a sequence of line segments, use a
//! flattened builder:
//!
//! ```ignore
//! let tolerance = 0.05;// maximum distance between a curve and its approximation.
//! let mut builder = Path::builder().flattened(tolerance);
//!
//! builder.move_to(point(0.0, 0.0));
//! builder.quadratic_bezier_to(point(1.0, 0.0), point(1.0, 1.0));
//! builder.close();
//!
//! // The resulting path contains only Begin, Line and End events.
//! let path = builder.build();
//! ```
//!

use crate::events::PathEvent;
use crate::geom::{Arc, CubicBezierSegment, QuadraticBezierSegment, LineSegment};
use crate::math::*;
use crate::svg::SvgBuilder;
use crate::polygon::PolygonSlice;
use std::marker::Sized;

use crate::EndpointId;

pub trait Build {
    /// The type of object that is created by this builder.
    type PathType;

    /// Builds a path object and resets the builder so that it can be used again.
    fn build(self) -> Self::PathType;
}

/// The main path building interface. More elaborate interfaces are built on top
/// of the provided primitives.
pub trait PathBuilder {
    /// Sets the current position in preparation for the next sub-path.
    /// If the current sub-path contains edges, this ends the sub-path without closing it.
    fn begin(&mut self, to: Point) -> EndpointId;

    /// Adds a line segment to the current sub-path and set the current position.
    fn line_to(&mut self, to: Point) -> EndpointId;

    /// Ends the current sub path.
    fn end(&mut self, close: bool);

    /// Closes the current sub path.
    ///
    /// Shorthand for `builder.end(true)`.
    fn close(&mut self) {
        self.end(true)
    }

    fn quadratic_bezier_to(&mut self, ctrl: Point, to: Point) -> EndpointId;

    fn cubic_bezier_to(&mut self, ctrl1: Point, ctrl2: Point, to: Point) -> EndpointId;

    fn reserve(&mut self, _endpoints: usize, _ctrl_points: usize) {}

    fn path_event(&mut self, event: PathEvent) {
        match event {
            PathEvent::Begin { at } => {
                self.begin(at);
            }
            PathEvent::Line { to, .. } => {
                self.line_to(to);
            }
            PathEvent::Quadratic { ctrl, to, .. } => {
                self.quadratic_bezier_to(ctrl, to);
            }
            PathEvent::Cubic { ctrl1, ctrl2, to, .. } => {
                self.cubic_bezier_to(ctrl1, ctrl2, to);
            }
            PathEvent::End { close, .. } => {
                self.end(close);
            }
        }
    }

    fn add_polygon(&mut self, polygon: PolygonSlice<Point>) {
        if polygon.points.is_empty() {
            return;
        }

        self.reserve(polygon.points.len(), 0);

        self.begin(polygon.points[0]);
        for p in &polygon.points[1..] {
            self.line_to(*p);
        }

        self.end(polygon.closed);
    }

    fn add_point(&mut self, at: Point) -> EndpointId {
        let id = self.begin(at);
        self.end(false);

        id
    }

    fn add_line_segment(&mut self, line: &LineSegment<f32>) -> (EndpointId, EndpointId) {
        let a = self.begin(line.from);
        let b = self.line_to(line.to);
        self.end(false);

        (a, b)
    }

    /// Returns a builder that approximates all curves with sequences of line segments.
    fn flattened(self, tolerance: f32) -> Flattened<Self>
    where
        Self: Sized,
    {
        Flattened::new(self, tolerance)
    }

    /// Returns a builder that support svg commands.
    fn with_svg(self) -> SvgBuilder<Self>
    where
        Self: Sized,
    {
        SvgBuilder::new(self)
    }
}

#[doc(hidden)]
pub fn build_polygon<Builder: PathBuilder>(builder: &mut Builder, points: &[Point]) {
    if points.len() < 2 {
        return;
    }

    builder.begin(points[0]);
    for p in &points[1..] {
        builder.line_to(*p);
    }
    builder.close();
}

/// Generates flattened paths
pub struct Flattened<Builder> {
    builder: Builder,
    current_position: Point,
    tolerance: f32,
}

impl<Builder: Build> Build for Flattened<Builder> {
    type PathType = Builder::PathType;

    fn build(self) -> Builder::PathType {
        self.builder.build()
    }
}

pub fn build_arc_as_quadratic_beziers(
    current_position: Point,
    center: Point,
    radii: Vector,
    sweep_angle: Angle,
    x_rotation: Angle,
    builder: &mut impl PathBuilder,
) {
    let start_angle = (current_position - center).angle_from_x_axis() - x_rotation;
    let arc = Arc {
        center,
        radii,
        start_angle,
        sweep_angle,
        x_rotation,
    };

    let arc_start = arc.from();
    if (arc_start - current_position).square_length() < 0.01 {
        // TODO: if there is no point on the current sub-path we should do a
        // move_to instead, but we don't have the information here.
        builder.line_to(arc_start);
    }

    arc.for_each_quadratic_bezier(&mut |curve| {
        builder.quadratic_bezier_to(curve.ctrl, curve.to);
    });
}

pub fn flatten_arc(
    tolerance: f32,
    current_position: Point,
    center: Point,
    radii: Vector,
    sweep_angle: Angle,
    x_rotation: Angle,
    builder: &mut impl PathBuilder,
) {
    let start_angle = (current_position - center).angle_from_x_axis() - x_rotation;
    let arc = Arc {
        center,
        radii,
        start_angle,
        sweep_angle,
        x_rotation,
    };

    let arc_start = arc.from();
    if (arc_start - current_position).square_length() < 0.01 {
        // TODO: if there is no point on the current sub-path we should do a
        // move_to instead, but we don't have the information here.
        builder.line_to(arc_start);
    }

    arc.for_each_flattened(tolerance, &mut |to| {
        builder.line_to(to);
    });
}

pub fn flatten_quadratic_bezier(
    tolerance: f32,
    from: Point,
    ctrl: Point,
    to: Point,
    builder: &mut impl PathBuilder,
) -> EndpointId {
    let curve = QuadraticBezierSegment { from, ctrl, to, };
    let mut id = EndpointId::INVALID;
    curve.for_each_flattened(tolerance, &mut |point| {
        id = builder.line_to(point);
    });

    id
}

pub fn flatten_cubic_bezier(
    tolerance: f32,
    from: Point,
    ctrl1: Point,
    ctrl2: Point,
    to: Point,
    builder: &mut impl PathBuilder,
) -> EndpointId {
    let curve = CubicBezierSegment { from, ctrl1, ctrl2, to };
    let mut id = EndpointId::INVALID;
    curve.for_each_flattened(tolerance, &mut |point| {
        id = builder.line_to(point);
    });

    id
}

impl<Builder: PathBuilder> PathBuilder for Flattened<Builder> {

    fn begin(&mut self, at: Point) -> EndpointId {
        self.current_position = at;
        self.builder.begin(at)
    }

    fn end(&mut self, close: bool) {
        self.builder.end(close)
    }

    fn line_to(&mut self, to: Point) -> EndpointId {
        self.current_position = to;
        self.builder.line_to(to) 
    }

    fn quadratic_bezier_to(&mut self, ctrl: Point, to: Point) -> EndpointId {
        self.current_position = to;
        flatten_quadratic_bezier(self.tolerance, self.current_position, ctrl, to, self)
    }

    fn cubic_bezier_to(&mut self, ctrl1: Point, ctrl2: Point, to: Point) -> EndpointId {
        self.current_position = to;
        flatten_cubic_bezier(self.tolerance, self.current_position, ctrl1, ctrl2, to, self)
    }
}

impl<Builder: PathBuilder> Flattened<Builder> {
    pub fn new(builder: Builder, tolerance: f32) -> Flattened<Builder> {
        Flattened {
            builder,
            current_position: point(0.0, 0.0),
            tolerance,
        }
    }

    pub fn set_tolerance(&mut self, tolerance: f32) {
        self.tolerance = tolerance
    }
}
