//! Tools to build path objects from a sequence of imperative commands.
//!
//! ## Examples
//!
//! The following example shows the Builder struct from the
//! [lyon_path](https://docs.rs/lyon_path/*/lyon_path) crate using the
//! [FlatPathBuilder](traits.FlatPathBuilder.html) interface.
//!
//! ```ignore
//! use lyon_path::Path;
//! use lyon_core::math::{point};
//! use lyon_path::builder::*;
//!
//! // Create a builder object to build the path.
//! let mut builder = Path::builder();
//!
//! // Build a simple path using the FlatPathBuilder interface.
//! builder.move_to(point(0.0, 0.0));
//! builder.line_to(point(1.0, 2.0));
//! builder.line_to(point(2.0, 0.0));
//! builder.line_to(point(1.0, 1.0));
//! builder.close();
//!
//! // Finish building and create the actual path object.
//! let path = builder.build();
//! ```
//!
//! The next example uses the [PathBuilder](traits.PathBuilder.html) trait, which adds
//! some simple curves to the [FlatPathBuilder](traits.FlatPathBuilder.html) trait.
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
//! The [SvgBuilder](trait.SvgBuilder.html) Adds to [PathBuilder](traits.PathBuilder.html)
//! the rest of the [SVG path](https://svgwg.org/specs/paths/) commands.
//!
//! These SVG commands can approximated with the simpler set of commands supported by
//! [PathBuilder](traits.PathBuilder.html). Therefore it is possible to create an SvgBuilder
//! adapter on top of a PathBuilder using the with_svg method:
//!
//! ```ignore
//! let mut builder = Path::builder().with_svg();
//!
//! builder.move_to(point(0.0, 0.0));
//! builder.horizontal_line_to(1.0);
//! builder.relative_quadratic_bezier_to(point(1.0, 0.0), point(1.0, 1.0));
//! builder.smooth_relative_quadratic_bezier_to(point(-1.0, 1.0));
//!
//! let path = builder.build();
//! ```
//!
//! To build a path that approximates curves with a sequence of line segments, use the
//! flattened method:
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

use crate::math::*;
use crate::events::{PathEvent, FlattenedEvent};
use crate::geom::{CubicBezierSegment, QuadraticBezierSegment, SvgArc, Arc, ArcFlags};
use crate::path_state::PathState;
use std::marker::Sized;

pub trait Build {
    /// The type of object that is created by this builder.
    type PathType;

    /// Builds a path object and resets the builder so that it can be used again.
    fn build(self) -> Self::PathType;

    /// Builds a path object and resets the builder so that it can be used again.
    fn build_and_reset(&mut self) -> Self::PathType;
}

/// The most basic path building interface. Does not handle any kind of curve.
pub trait FlatPathBuilder {
    /// Sets the current position in preparation for the next sub-path.
    /// If the current sub-path contains edges, this ends the sub-path without closing it.
    fn move_to(&mut self, to: Point);

    /// Adds a line segment to the current sub-path and set the current position.
    fn line_to(&mut self, to: Point);

    /// Closes the current sub path and sets the current position to the first position of
    /// this the current sub-path.
    ///
    /// Subsequent commands will affect the next sub-path.
    fn close(&mut self);

    fn current_position(&self) -> Point;

    fn flat_event(&mut self, event: FlattenedEvent<Point>) {
        match event {
            FlattenedEvent::Begin { at } => {
                self.move_to(at);
            }
            FlattenedEvent::Line { to, .. } => {
                self.line_to(to);
            }
            FlattenedEvent::End { close: true, .. } => {
                self.close();
            }
            FlattenedEvent::End { close: false, .. } => {
            }
        }
    }

    /// Returns a builder that approximates all curves with sequences of line segments.
    fn flattened(self, tolerance: f32) -> FlatteningBuilder<Self> where Self: Sized {
        FlatteningBuilder::new(self, tolerance)
    }
}

/// The main path building interface. More elaborate interfaces are built on top
/// of the provided primitives.
pub trait PathBuilder: FlatPathBuilder {
    fn quadratic_bezier_to(&mut self, ctrl: Point, to: Point);
    fn cubic_bezier_to(&mut self, ctrl1: Point, ctrl2: Point, to: Point);
    fn arc(&mut self, center: Point, radii: Vector, sweep_angle: Angle, x_rotation: Angle);

    fn path_event(&mut self, event: PathEvent<Point, Point>) {
        match event {
            PathEvent::Begin { at } => {
                self.move_to(at);
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
            PathEvent::End { close : true, .. } => {
                self.close();
            }
            PathEvent::End { close : false, .. } => {
            }
        }
    }

    /// Returns a builder that support svg commands.
    fn with_svg(self) -> SvgPathBuilder<Self> where Self : Sized { SvgPathBuilder::new(self) }
}

/// A path building interface that tries to stay close to SVG's path specification.
/// https://svgwg.org/specs/paths/
pub trait SvgBuilder: PathBuilder {
    fn relative_move_to(&mut self, to: Vector);
    fn relative_line_to(&mut self, to: Vector);
    fn relative_quadratic_bezier_to(&mut self, ctrl: Vector, to: Vector);
    fn relative_cubic_bezier_to(&mut self, ctrl1: Vector, ctrl2: Vector, to: Vector);
    fn smooth_cubic_bezier_to(&mut self, ctrl2: Point, to: Point);
    fn smooth_relative_cubic_bezier_to(&mut self, ctrl2: Vector, to: Vector);
    fn smooth_quadratic_bezier_to(&mut self, to: Point);
    fn smooth_relative_quadratic_bezier_to(&mut self, to: Vector);
    fn horizontal_line_to(&mut self, x: f32);
    fn relative_horizontal_line_to(&mut self, dx: f32);
    fn vertical_line_to(&mut self, y: f32);
    fn relative_vertical_line_to(&mut self, dy: f32);
    fn arc_to(&mut self, radii: Vector, x_rotation: Angle, flags: ArcFlags, to: Point);
    fn relative_arc_to(
        &mut self,
        radii: Vector,
        x_rotation: Angle,
        flags: ArcFlags,
        to: Vector,
    );
}

/// Build a path from simple lists of points.
pub trait PolygonBuilder {
    /// Add a closed polygon.
    fn polygon(&mut self, points: &[Point]);
}

#[doc(hidden)]
pub fn build_polygon<Builder: FlatPathBuilder>(builder: &mut Builder, points: &[Point]) {
    if points.len() < 2 {
        return;
    }

    builder.move_to(points[0]);
    for p in &points[1..] {
        builder.line_to(*p);
    }
    builder.close();
}

/// Implements the Svg building interface on top of a PathBuilder.
pub struct SvgPathBuilder<Builder: PathBuilder> {
    builder: Builder,
    state: PathState,
}

impl<Builder: PathBuilder> SvgPathBuilder<Builder> {
    pub fn new(builder: Builder) -> SvgPathBuilder<Builder> {
        SvgPathBuilder {
            builder,
            state: PathState::new(),
        }
    }
}

impl<Builder: PathBuilder + Build> Build for SvgPathBuilder<Builder> {
    type PathType = Builder::PathType;

    fn build(self) -> Builder::PathType { self.builder.build() }

    fn build_and_reset(&mut self) -> Builder::PathType { self.builder.build_and_reset() }
}

impl<Builder: PathBuilder> FlatPathBuilder for SvgPathBuilder<Builder> {
    fn move_to(&mut self, to: Point) {
        self.state.move_to(to);
        self.builder.move_to(to);
    }

    fn line_to(&mut self, to: Point) {
        self.state.line_to(to);
        self.builder.line_to(to);
    }

    fn close(&mut self) {
        self.state.close();
        self.builder.close();
    }

    fn current_position(&self) -> Point { self.state.current_position() }
}

impl<Builder: PathBuilder> PathBuilder for SvgPathBuilder<Builder> {
    fn quadratic_bezier_to(&mut self, ctrl: Point, to: Point) {
        self.state.quadratic_bezier_to(ctrl, to);
        self.builder.quadratic_bezier_to(ctrl, to);
    }

    fn cubic_bezier_to(&mut self, ctrl1: Point, ctrl2: Point, to: Point) {
        self.state.cubic_bezier_to(ctrl1, ctrl2, to);
        self.builder.cubic_bezier_to(ctrl1, ctrl2, to);
    }

    fn arc(
        &mut self,
        center: Point,
        radii: Vector,
        sweep_angle: Angle,
        x_rotation: Angle
    ) {
        self.state.arc(center, radii, sweep_angle, x_rotation);
        self.builder.arc(center, radii, sweep_angle, x_rotation);
    }
}

impl<Builder: PathBuilder> SvgBuilder for SvgPathBuilder<Builder> {
    fn relative_move_to(&mut self, to: Vector) {
        self.state.relative_move_to(to);
        self.builder.move_to(self.state.current_position());
    }

    fn relative_line_to(&mut self, to: Vector) {
        self.state.relative_line_to(to);
        self.builder.line_to(self.state.current_position());
    }

    fn relative_quadratic_bezier_to(&mut self, ctrl: Vector, to: Vector) {
        let offset = self.state.current_position();
        self.state.relative_quadratic_bezier_to(ctrl, to);
        self.builder.quadratic_bezier_to(offset + ctrl, offset + to);
    }

    fn relative_cubic_bezier_to(&mut self, ctrl1: Vector, ctrl2: Vector, to: Vector) {
        let offset = self.state.current_position();
        self.state.relative_cubic_bezier_to(ctrl1, ctrl2, to);
        self.builder.cubic_bezier_to(offset + ctrl1, offset + ctrl2, offset + to);
    }

    fn smooth_cubic_bezier_to(&mut self, ctrl2: Point, to: Point) {
        let ctrl1 = self.state.get_smooth_cubic_ctrl();
        self.state.smooth_cubic_bezier_to(ctrl2, to);
        self.builder.cubic_bezier_to(ctrl1, ctrl2, to);
    }

    fn smooth_relative_cubic_bezier_to(&mut self, ctrl2: Vector, to: Vector) {
        let ctrl1 = self.state.get_smooth_cubic_ctrl();
        let offset = self.state.current_position();
        self.state.smooth_relative_cubic_bezier_to(ctrl2, to);
        self.builder.cubic_bezier_to(ctrl1, offset + ctrl2, offset + to);
    }

    fn smooth_quadratic_bezier_to(&mut self, to: Point) {
        let ctrl = self.state.get_smooth_quadratic_ctrl();
        self.state.smooth_quadratic_bezier_to(to);
        self.builder.quadratic_bezier_to(ctrl, to);
    }

    fn smooth_relative_quadratic_bezier_to(&mut self, to: Vector) {
        let ctrl = self.state.get_smooth_quadratic_ctrl();
        let offset = self.state.current_position();
        self.state.smooth_relative_quadratic_bezier_to(to);
        self.builder.quadratic_bezier_to(ctrl, offset + to);
    }

    fn horizontal_line_to(&mut self, x: f32) {
        self.state.horizontal_line_to(x);
        self.builder.line_to(self.state.current_position());
    }

    fn relative_horizontal_line_to(&mut self, dx: f32) {
        self.state.relative_horizontal_line_to(dx);
        self.builder.line_to(self.state.current_position());
    }

    fn vertical_line_to(&mut self, y: f32) {
        self.state.vertical_line_to(y);
        self.builder.line_to(self.state.current_position());
    }

    fn relative_vertical_line_to(&mut self, dy: f32) {
        self.state.relative_vertical_line_to(dy);
        self.builder.line_to(self.state.current_position());
    }

    fn arc_to(&mut self, radii: Vector, x_rotation: Angle, flags: ArcFlags, to: Point) {
        SvgArc {
            from: self.state.current_position(),
            to,
            radii,
            x_rotation,
            flags: ArcFlags {
                large_arc: flags.large_arc,
                sweep: flags.sweep,
            },
        }.for_each_quadratic_bezier(&mut|curve| {
            self.quadratic_bezier_to(curve.ctrl, curve.to);
        });
        self.state.arc_to(radii, x_rotation, flags, to);
    }

    fn relative_arc_to(
        &mut self,
        radii: Vector,
        x_rotation: Angle,
        flags: ArcFlags,
        to: Vector,
    ) {
        let offset = self.state.current_position();
        self.arc_to(radii, x_rotation, flags, offset + to);
    }
}

/// Generates flattened paths
pub struct FlatteningBuilder<Builder> {
    builder: Builder,
    tolerance: f32,
}

impl<Builder: Build> Build for FlatteningBuilder<Builder> {
    type PathType = Builder::PathType;

    fn build(self) -> Builder::PathType { self.builder.build() }

    fn build_and_reset(&mut self) -> Builder::PathType { self.builder.build_and_reset() }
}

impl<Builder: FlatPathBuilder> FlatPathBuilder for FlatteningBuilder<Builder> {
    fn move_to(&mut self, to: Point) { self.builder.move_to(to); }

    fn line_to(&mut self, to: Point) { self.builder.line_to(to); }

    fn close(&mut self) { self.builder.close() }

    fn current_position(&self) -> Point { self.builder.current_position() }
}

impl<Builder: FlatPathBuilder> PathBuilder for FlatteningBuilder<Builder> {
    fn quadratic_bezier_to(&mut self, ctrl: Point, to: Point) {
        QuadraticBezierSegment {
            from: self.current_position(),
            ctrl,
            to,
        }.for_each_flattened(self.tolerance, &mut |point| { self.line_to(point); });
    }

    fn cubic_bezier_to(&mut self, ctrl1: Point, ctrl2: Point, to: Point) {
        CubicBezierSegment {
            from: self.current_position(),
            ctrl1,
            ctrl2,
            to,
        }.for_each_flattened(self.tolerance, &mut |point| { self.line_to(point); });
    }

    fn arc(
        &mut self,
        center: Point,
        radii: Vector,
        sweep_angle: Angle,
        x_rotation: Angle
    ) {
        let start_angle = (self.current_position() - center).angle_from_x_axis() - x_rotation;
        Arc {
            center,
            radii,
            start_angle,
            sweep_angle,
            x_rotation,
        }.for_each_quadratic_bezier(&mut|curve| {
            self.quadratic_bezier_to(curve.ctrl, curve.to);
        });
    }
}

impl<Builder: FlatPathBuilder> FlatteningBuilder<Builder> {
    pub fn new(builder: Builder, tolerance: f32) -> FlatteningBuilder<Builder> {
        FlatteningBuilder {
            builder,
            tolerance,
        }
    }

    pub fn set_tolerance(&mut self, tolerance: f32) { self.tolerance = tolerance }
}
