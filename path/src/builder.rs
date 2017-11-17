//! # Lyon path builder
//!
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
//! // The resulting path contains only MoveTo, LineTo and Close events.
//! let path = builder.build();
//! ```
//!

use core::{PathEvent, FlattenedEvent, SvgEvent, ArcFlags};
use core::math::*;
use bezier::{CubicBezierSegment, QuadraticBezierSegment, SvgArc};
use bezier;

/// The most basic path building interface. Does not handle any kind of curve.
pub trait FlatPathBuilder: ::std::marker::Sized {
    /// The type of object that is created by this builder.
    type PathType;

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

    /// Builds a path object and resets the builder so that it can be used again.
    fn build(self) -> Self::PathType;

    /// Builds a path object and resets the builder so that it can be used again.
    fn build_and_reset(&mut self) -> Self::PathType;

    fn current_position(&self) -> Point;

    fn flat_event(&mut self, event: FlattenedEvent) {
        match event {
            FlattenedEvent::MoveTo(to) => {
                self.move_to(to);
            }
            FlattenedEvent::LineTo(to) => {
                self.line_to(to);
            }
            FlattenedEvent::Close => {
                self.close();
            }
        }
    }

    /// Returns a builder that approximates all curves with sequences of line segments.
    fn flattened(self, tolerance: f32) -> FlatteningBuilder<Self> {
        FlatteningBuilder::new(self, tolerance)
    }
}

/// The main path building interface. More elaborate interfaces are built on top
/// of the provided primitives.
pub trait PathBuilder: FlatPathBuilder {
    fn quadratic_bezier_to(&mut self, ctrl: Point, to: Point);
    fn cubic_bezier_to(&mut self, ctrl1: Point, ctrl2: Point, to: Point);

    fn path_event(&mut self, event: PathEvent) {
        match event {
            PathEvent::MoveTo(to) => {
                self.move_to(to);
            }
            PathEvent::LineTo(to) => {
                self.line_to(to);
            }
            PathEvent::QuadraticTo(ctrl, to) => {
                self.quadratic_bezier_to(ctrl, to);
            }
            PathEvent::CubicTo(ctrl1, ctrl2, to) => {
                self.cubic_bezier_to(ctrl1, ctrl2, to);
            }
            PathEvent::Close => {
                self.close();
            }
        }
    }

    /// Returns a builder that support svg commands.
    fn with_svg(self) -> SvgPathBuilder<Self> { SvgPathBuilder::new(self) }
}

/// A path building interface that tries to stay close to SVG's path specification.
/// https://svgwg.org/specs/paths/
pub trait SvgBuilder: PathBuilder {
    fn relative_move_to(&mut self, to: Vec2);
    fn relative_line_to(&mut self, to: Vec2);
    fn relative_quadratic_bezier_to(&mut self, ctrl: Vec2, to: Vec2);
    fn relative_cubic_bezier_to(&mut self, ctrl1: Vec2, ctrl2: Vec2, to: Vec2);
    fn smooth_cubic_bezier_to(&mut self, ctrl2: Point, to: Point);
    fn smooth_relative_cubic_bezier_to(&mut self, ctrl2: Vec2, to: Vec2);
    fn smooth_quadratic_bezier_to(&mut self, to: Point);
    fn smooth_relative_quadratic_bezier_to(&mut self, to: Vec2);
    fn horizontal_line_to(&mut self, x: f32);
    fn relative_horizontal_line_to(&mut self, dx: f32);
    fn vertical_line_to(&mut self, y: f32);
    fn relative_vertical_line_to(&mut self, dy: f32);
    // TODO: Would it be better to use an api closer to cairo/skia for arcs?
    fn arc_to(&mut self, radii: Vec2, x_rotation: Radians<f32>, flags: ArcFlags, to: Point);
    fn relative_arc_to(
        &mut self,
        radii: Vec2,
        x_rotation: Radians<f32>,
        flags: ArcFlags,
        to: Vec2,
    );

    fn svg_event(&mut self, event: SvgEvent) {
        match event {
            SvgEvent::MoveTo(to) => {
                self.move_to(to);
            }
            SvgEvent::LineTo(to) => {
                self.line_to(to);
            }
            SvgEvent::QuadraticTo(ctrl, to) => {
                self.quadratic_bezier_to(ctrl, to);
            }
            SvgEvent::CubicTo(ctrl1, ctrl2, to) => {
                self.cubic_bezier_to(ctrl1, ctrl2, to);
            }
            SvgEvent::Close => {
                self.close();
            }

            SvgEvent::ArcTo(radii, x_rotation, flags, to) => {
                self.arc_to(radii, x_rotation, flags, to);
            }
            SvgEvent::RelativeArcTo(radii, x_rotation, flags, to) => {
                self.relative_arc_to(radii, x_rotation, flags, to);
            }

            SvgEvent::RelativeMoveTo(to) => {
                self.relative_move_to(to);
            }
            SvgEvent::RelativeLineTo(to) => {
                self.relative_line_to(to);
            }
            SvgEvent::RelativeQuadraticTo(ctrl, to) => {
                self.relative_quadratic_bezier_to(ctrl, to);
            }
            SvgEvent::RelativeCubicTo(ctrl1, ctrl2, to) => {
                self.relative_cubic_bezier_to(ctrl1, ctrl2, to);
            }

            SvgEvent::HorizontalLineTo(x) => {
                self.horizontal_line_to(x);
            }
            SvgEvent::VerticalLineTo(y) => {
                self.vertical_line_to(y);
            }
            SvgEvent::RelativeHorizontalLineTo(x) => {
                self.relative_horizontal_line_to(x);
            }
            SvgEvent::RelativeVerticalLineTo(y) => {
                self.relative_vertical_line_to(y);
            }

            SvgEvent::SmoothQuadraticTo(to) => {
                self.smooth_quadratic_bezier_to(to);
            }
            SvgEvent::SmoothCubicTo(ctrl2, to) => {
                self.smooth_cubic_bezier_to(ctrl2, to);
            }
            SvgEvent::SmoothRelativeQuadraticTo(to) => {
                self.smooth_relative_quadratic_bezier_to(to);
            }
            SvgEvent::SmoothRelativeCubicTo(ctrl2, to) => {
                self.smooth_relative_cubic_bezier_to(ctrl2, to);
            }
        }
    }
}

/// Build a path from a simple list of points.
pub trait PolygonBuilder {
    fn polygon(&mut self, points: &[Point]);
}

/// Implements the Svg building interface on top of a PathBuilder.
pub struct SvgPathBuilder<Builder: PathBuilder> {
    builder: Builder,
    last_ctrl: Point,
}

impl<Builder: PathBuilder> SvgPathBuilder<Builder> {
    pub fn new(builder: Builder) -> SvgPathBuilder<Builder> {
        SvgPathBuilder {
            builder: builder,
            last_ctrl: point(0.0, 0.0),
        }
    }
}

impl<Builder: PathBuilder> FlatPathBuilder for SvgPathBuilder<Builder> {
    type PathType = Builder::PathType;

    fn move_to(&mut self, to: Point) {
        self.last_ctrl = to;
        self.builder.move_to(to);
    }

    fn line_to(&mut self, to: Point) {
        self.last_ctrl = self.current_position();
        self.builder.line_to(to);
    }

    fn close(&mut self) {
        self.last_ctrl = point(0.0, 0.0);
        self.builder.close()
    }

    fn current_position(&self) -> Point { self.builder.current_position() }

    fn build(self) -> Builder::PathType { self.builder.build() }

    fn build_and_reset(&mut self) -> Builder::PathType { self.builder.build_and_reset() }
}

impl<Builder: PathBuilder> PathBuilder for SvgPathBuilder<Builder> {
    fn quadratic_bezier_to(&mut self, ctrl: Point, to: Point) {
        self.last_ctrl = ctrl;
        self.builder.quadratic_bezier_to(ctrl, to);
    }

    fn cubic_bezier_to(&mut self, ctrl1: Point, ctrl2: Point, to: Point) {
        self.last_ctrl = ctrl2;
        self.builder.cubic_bezier_to(ctrl1, ctrl2, to);
    }
}

impl<Builder: PathBuilder> SvgBuilder for SvgPathBuilder<Builder> {
    fn relative_move_to(&mut self, to: Vec2) {
        let offset = self.builder.current_position();
        self.move_to(offset + to);
    }

    fn relative_line_to(&mut self, to: Vec2) {
        let offset = self.builder.current_position();
        self.line_to(offset + to);
    }

    fn relative_quadratic_bezier_to(&mut self, ctrl: Vec2, to: Vec2) {
        let offset = self.builder.current_position();
        self.quadratic_bezier_to(offset + ctrl, offset + to);
    }

    fn relative_cubic_bezier_to(&mut self, ctrl1: Vec2, ctrl2: Vec2, to: Vec2) {
        let offset = self.builder.current_position();
        self.cubic_bezier_to(offset + ctrl1, offset + ctrl2, offset + to);
    }

    fn smooth_cubic_bezier_to(&mut self, ctrl2: Point, to: Point) {
        let ctrl = self.builder.current_position() +
            (self.builder.current_position() - self.last_ctrl);
        self.cubic_bezier_to(ctrl, ctrl2, to);
    }

    fn smooth_relative_cubic_bezier_to(&mut self, ctrl2: Vec2, to: Vec2) {
        let ctrl = self.builder.current_position() - self.last_ctrl;
        self.relative_cubic_bezier_to(ctrl, ctrl2, to);
    }

    fn smooth_quadratic_bezier_to(&mut self, to: Point) {
        let ctrl = self.builder.current_position() +
            (self.builder.current_position() - self.last_ctrl);
        self.quadratic_bezier_to(ctrl, to);
    }

    fn smooth_relative_quadratic_bezier_to(&mut self, to: Vec2) {
        let ctrl = self.builder.current_position() - self.last_ctrl;
        self.relative_quadratic_bezier_to(ctrl, to);
    }

    fn horizontal_line_to(&mut self, x: f32) {
        let y = self.builder.current_position().y;
        self.line_to(point(x, y));
    }

    fn relative_horizontal_line_to(&mut self, dx: f32) {
        let p = self.builder.current_position();
        self.line_to(point(p.x + dx, p.y));
    }

    fn vertical_line_to(&mut self, y: f32) {
        let x = self.builder.current_position().x;
        self.line_to(point(x, y));
    }

    fn relative_vertical_line_to(&mut self, dy: f32) {
        let p = self.builder.current_position();
        self.line_to(point(p.x, p.y + dy));
    }

    fn arc_to(&mut self, radii: Vec2, x_rotation: Radians<f32>, flags: ArcFlags, to: Point) {
        SvgArc {
            from: self.current_position(),
            to: to,
            radii: radii,
            x_rotation: x_rotation,
            flags: bezier::ArcFlags {
                large_arc: flags.large_arc,
                sweep: flags.sweep,
            },
        }.to_quadratic_beziers(&mut|ctrl, to|{
            self.quadratic_bezier_to(ctrl, to);
        })
    }

    fn relative_arc_to(
        &mut self,
        radii: Vec2,
        x_rotation: Radians<f32>,
        flags: ArcFlags,
        to: Vec2,
    ) {
        let offset = self.builder.current_position();
        self.arc_to(radii, x_rotation, flags, offset + to);
    }
}

/// Generates flattened paths
pub struct FlatteningBuilder<Builder> {
    builder: Builder,
    tolerance: f32,
}

impl<Builder: FlatPathBuilder> FlatPathBuilder for FlatteningBuilder<Builder> {
    type PathType = Builder::PathType;

    fn move_to(&mut self, to: Point) { self.builder.move_to(to); }

    fn line_to(&mut self, to: Point) { self.builder.line_to(to); }

    fn close(&mut self) { self.builder.close() }

    fn current_position(&self) -> Point { self.builder.current_position() }

    fn build(self) -> Builder::PathType { self.builder.build() }

    fn build_and_reset(&mut self) -> Builder::PathType { self.builder.build_and_reset() }
}

impl<Builder: FlatPathBuilder> PathBuilder for FlatteningBuilder<Builder> {
    fn quadratic_bezier_to(&mut self, ctrl: Point, to: Point) {
        QuadraticBezierSegment {
            from: self.current_position(),
            ctrl: ctrl,
            to: to,
        }.flattened_for_each(self.tolerance, &mut |point| { self.line_to(point); });
    }

    fn cubic_bezier_to(&mut self, ctrl1: Point, ctrl2: Point, to: Point) {
        CubicBezierSegment {
            from: self.current_position(),
            ctrl1: ctrl1,
            ctrl2: ctrl2,
            to: to,
        }.flattened_for_each(self.tolerance, &mut |point| { self.line_to(point); });
    }
}

impl<Builder: FlatPathBuilder> FlatteningBuilder<Builder> {
    pub fn new(builder: Builder, tolerance: f32) -> FlatteningBuilder<Builder> {
        FlatteningBuilder {
            builder: builder,
            tolerance: tolerance,
        }
    }

    pub fn set_tolerance(&mut self, tolerance: f32) { self.tolerance = tolerance }
}

impl<Builder: FlatPathBuilder> PolygonBuilder for Builder {
    fn polygon(&mut self, points: &[Point]) {
        assert!(!points.is_empty());

        self.move_to(points[0]);
        for p in points[1..].iter() {
            self.line_to(*p);
        }
        self.close();
    }
}
