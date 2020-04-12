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
use crate::geom::{
    Arc, ArcFlags, SvgArc,
    CubicBezierSegment, QuadraticBezierSegment, LineSegment,
    traits::Transformation
};
use crate::math::*;
use crate::polygon::Polygon;
use crate::path::Verb;
use crate::EndpointId;

use std::marker::Sized;

/// The base path building interface.
///
/// Unlike `SvgPathBuilder`, this interface strictly requires sub-paths to be manually
/// started and ended (See the `begin` and `end` methods).
/// All positions are provided in absolute coordinates.
///
/// The goal of this interface is to abstract over simple and fast implementations that
/// do not deal with corner cases such as adding segments without starting a sub-path.
///
/// More elaborate interfaces are built on top of the provided primitives. In particular,
/// the `SvgPathBuilder` trait providing more permissive and richer interface is
/// automatically implemented via the `WithSvg` adapter (See the `with_svg` method).
pub trait PathBuilder {
    /// Starts a new sub-path at a given position.
    ///
    /// There must be no sub-path in progress when this method is called.
    /// `at` becomes the current position of the sub-path.
    fn begin(&mut self, at: Point) -> EndpointId;

    /// Ends the current sub path.
    ///
    /// A sub-path must be in progress when this method is called.
    /// After this method is called, there is no sub-path in progress until
    /// `begin` is called again.
    fn end(&mut self, close: bool);

    /// Closes the current sub path.
    ///
    /// Shorthand for `builder.end(true)`.
    fn close(&mut self) {
        self.end(true)
    }

    /// Adds a line segment to the current sub-path.
    ///
    /// A sub-path must be in progress when this method is called.
    fn line_to(&mut self, to: Point) -> EndpointId;

    /// Adds a quadratic bézier curve to the current sub-path.
    ///
    /// A sub-path must be in progress when this method is called.
    fn quadratic_bezier_to(&mut self, ctrl: Point, to: Point) -> EndpointId;

    /// Adds a cubic bézier curve to the current sub-path.
    ///
    /// A sub-path must be in progress when this method is called.
    fn cubic_bezier_to(&mut self, ctrl1: Point, ctrl2: Point, to: Point) -> EndpointId;

    /// Hints at the builder that a certain number of endpoints and control
    /// points will be added.
    ///
    /// The Builder implementation may use this information to pre-allocate
    /// memory as an optimization.
    fn reserve(&mut self, _endpoints: usize, _ctrl_points: usize) {}

    /// Applies the provided path event.
    ///
    /// By default this calls one of `begin`, `end`, `line`, `quadratic_bezier_segment`,
    /// or `cubic_bezier_segment` according to the path event.
    ///
    /// The requirements for each method apply to the corresponding event.
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

    /// Adds a sub-path from a polygon.
    ///
    /// There must be no sub-path in progress when this method is called.
    /// No sub-path is in progress after the method is called.
    fn add_polygon(&mut self, polygon: Polygon<Point>) {
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

    /// Adds a sub-path containing a single point.
    ///
    /// There must be no sub-path in progress when this method is called.
    /// No sub-path is in progress after the method is called.
    fn add_point(&mut self, at: Point) -> EndpointId {
        let id = self.begin(at);
        self.end(false);

        id
    }

    /// Adds a sub-path containing a single line segment.
    ///
    /// There must be no sub-path in progress when this method is called.
    /// No sub-path is in progress after the method is called.
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

    /// Returns a builder that applies the given transformation to all positions.
    fn transformed<Transform>(self, transform: Transform) -> Transformed<Self, Transform>
    where
        Self: Sized,
        Transform: Transformation<f32>
    {
        Transformed::new(self, transform)
    }

    /// Returns a builder that support SVG commands.
    ///
    /// This must be called before starting to add any sub-path.
    fn with_svg(self) -> WithSvg<Self>
    where
        Self: Sized,
    {
        WithSvg::new(self)
    }
}

/// A path building interface that tries to stay close to SVG's path specification.
/// https://svgwg.org/specs/paths/
///
/// Some of the wording in the documentation of this trait is borrowed from the SVG
/// specification.
///
/// Unlike `PathBuilder`, implementations of this trait are expected to deal with
/// various corners cases such as adding segments without starting a sub-path.
pub trait SvgPathBuilder {
    /// Start a new sub-path at the given position.
    ///
    /// Corresponding SVG command: `M`.
    ///
    /// This command establishes a new initial point and a new current point. The effect
    /// is as if the "pen" were lifted and moved to a new location.
    /// If a sub-path is in progress, it is ended without being closed.
    fn move_to(&mut self, to: Point);

    /// ends the current sub-path by connecting it back to its initial point.
    ///
    /// Corresponding SVG command: `Z`.
    ///
    /// A straight line is drawn from the current point to the initial point of the
    /// current sub-path.
    /// The current position is set to the initial position of the sub-path that was
    /// closed.
    fn close(&mut self);

    /// Adds a line segment to the current sub-path.
    ///
    /// Corresponding SVG command: `L`.
    ///
    /// The segment starts at the builder's current position.
    /// If this is the very first command of the path (the builder therefore does not
    /// have a current position), the `line_to` command is replaced with a `move_to(to)`.
    fn line_to(&mut self, to: Point);

    /// Adds a quadratic bézier segment to the current sub-path.
    ///
    /// Corresponding SVG command: `Q`.
    ///
    /// The segment starts at the builder's current position.
    /// If this is the very first command of the path (the builder therefore does not
    /// have a current position), the `quadratic_bezier_to` command is replaced with
    /// a `move_to(to)`.
    fn quadratic_bezier_to(&mut self, ctrl: Point, to: Point);

    /// Adds a cubic bézier segment to the current sub-path.
    ///
    /// Corresponding SVG command: `C`.
    ///
    /// The segment starts at the builder's current position.
    /// If this is the very first command of the path (the builder therefore does not
    /// have a current position), the `cubic_bezier_to` command is replaced with
    /// a `move_to(to)`.
    fn cubic_bezier_to(&mut self, ctrl1: Point, ctrl2: Point, to: Point);

    /// Equivalent to `move_to` in relative coordinates.
    ///
    /// Corresponding SVG command: `m`.
    ///
    /// the provided coordinates are offsets relative to the current position of
    /// the builder.
    fn relative_move_to(&mut self, to: Vector);

    /// Equivalent to `line_to` in relative coordinates.
    ///
    /// Corresponding SVG command: `l`.
    ///
    /// the provided coordinates are offsets relative to the current position of
    /// the builder.
    fn relative_line_to(&mut self, to: Vector);

    /// Equivalent to `quadratic_bezier_to` in relative coordinates.
    ///
    /// Corresponding SVG command: `q`.
    ///
    /// the provided coordinates are offsets relative to the current position of
    /// the builder.
    fn relative_quadratic_bezier_to(&mut self, ctrl: Vector, to: Vector);

    /// Equivalent to `cubic_bezier_to` in relative coordinates.
    ///
    /// the provided coordinates are offsets relative to the current position of
    /// the builder.
    fn relative_cubic_bezier_to(&mut self, ctrl1: Vector, ctrl2: Vector, to: Vector);

    /// Equivalent to `cubic_bezier_to` with implicit first control point.
    ///
    /// Corresponding SVG command: `S`.
    ///
    /// The first control point is assumed to be the reflection of the second
    /// control point on the previous command relative to the current point.
    /// If there is no previous command or if the previous command was not a
    /// cubic bézier segment, the first control point is coincident with
    /// the current position.
    fn smooth_cubic_bezier_to(&mut self, ctrl2: Point, to: Point);

    /// Equivalent to `smooth_cubic_bezier_to` in relative coordinates.
    ///
    /// Corresponding SVG command: `s`.
    ///
    /// the provided coordinates are offsets relative to the current position of
    /// the builder.
    fn smooth_relative_cubic_bezier_to(&mut self, ctrl2: Vector, to: Vector);

    /// Equivalent to `quadratic_bezier_to` with implicit control point.
    ///
    /// Corresponding SVG command: `T`.
    ///
    /// The control point is assumed to be the reflection of the control
    /// point on the previous command relative to the current point.
    /// If there is no previous command or if the previous command was not a
    /// quadratic bézier segment, a line segment is added instead.
    fn smooth_quadratic_bezier_to(&mut self, to: Point);

    /// Equivalent to `smooth_quadratic_bezier_to` in relative coordinates.
    ///
    /// Corresponding SVG command: `t`.
    ///
    /// the provided coordinates are offsets relative to the current position of
    /// the builder.
    fn smooth_relative_quadratic_bezier_to(&mut self, to: Vector);

    /// Adds an horizontal line segment.
    ///
    /// Corresponding SVG command: `L`.
    ///
    /// Equivalent to `line_to`, using the y coordinate of the current position.
    fn horizontal_line_to(&mut self, x: f32);

    /// Adds an horizontal line segment in relative coordinates.
    ///
    /// Corresponding SVG command: `l`.
    ///
    /// Equivalent to `line_to`, using the y coordinate of the current position.
    /// `dx` is the horizontal offset relative to the current position.
    fn relative_horizontal_line_to(&mut self, dx: f32);

    /// Adds a vertical line segment.
    ///
    /// Corresponding SVG command: `V`.
    ///
    /// Equivalent to `line_to`, using the x coordinate of the current position.
    fn vertical_line_to(&mut self, y: f32);

    /// Adds a vertical line segment in relative coordinates.
    ///
    /// Corresponding SVG command: `v`.
    ///
    /// Equivalent to `line_to`, using the y coordinate of the current position.
    /// `dy` is the horizontal offset relative to the current position.
    fn relative_vertical_line_to(&mut self, dy: f32);

    /// Adds an elliptical arc.
    ///
    /// Corresponding SVG command: `A`.
    ///
    /// The arc starts at the current point and ends at `to`.
    /// The size and orientation of the ellipse are defined by `radii` and an `x_rotation`,
    /// which indicates how the ellipse as a whole is rotated relative to the current coordinate
    /// system. The center of the ellipse is calculated automatically to satisfy the constraints
    /// imposed by the other parameters. the arc `flags` contribute to the automatic calculations
    /// and help determine how the arc is built.
    fn arc_to(&mut self, radii: Vector, x_rotation: Angle, flags: ArcFlags, to: Point);

    /// Equivalent to `arc_to` in relative coordinates.
    ///
    /// Corresponding SVG command: `a`.
    ///
    /// the provided `to` coordinates are offsets relative to the current position of
    /// the builder.
    fn relative_arc_to(&mut self, radii: Vector, x_rotation: Angle, flags: ArcFlags, to: Vector);

    /// Hints at the builder that a certain number of endpoints and control
    /// points will be added.
    ///
    /// The Builder implementation may use this information to pre-allocate
    /// memory as an optimization.
    fn reserve(&mut self, _endpoints: usize, _ctrl_points: usize) {}
}

/// Builds a path.
///
/// This trait is separate from `PathBuilder` and `SvgPathBuilder` to allow them to
/// be used as trait object (which isn't when a method returns an associated type).
pub trait Build {
    /// The type of object that is created by this builder.
    type PathType;

    /// Builds a path object and resets the builder so that it can be used again.
    fn build(self) -> Self::PathType;
}


/// A Builder that approximates curves with successions of line segments.
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

    fn reserve(&mut self, endpoints: usize, ctrl_points: usize) {
        self.builder.reserve(endpoints, ctrl_points);
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

/// Builds a path with a transformation applied.
pub struct Transformed<Builder, Transform> {
    builder: Builder,
    transform: Transform,
}

impl<Builder, Transform> Transformed<Builder, Transform> {
    #[inline]
    pub fn new(builder: Builder, transform: Transform) -> Self {
        Transformed {
            builder,
            transform,
        }
    }

    #[inline]
    pub fn set_transform(&mut self, transform: Transform) {
        self.transform = transform;
    }
}

impl<Builder: Build, Transform> Build for Transformed<Builder, Transform> {
    type PathType = Builder::PathType;

    #[inline]
    fn build(self) -> Builder::PathType {
        self.builder.build()
    }
}

impl<Builder, Transform> PathBuilder for Transformed<Builder, Transform>
where
    Builder: PathBuilder,
    Transform: Transformation<f32>,
{
    #[inline]
    fn begin(&mut self, at: Point) -> EndpointId {
        self.builder.begin(self.transform.transform_point(at))
    }

    #[inline]
    fn end(&mut self, close: bool) {
        self.builder.end(close)
    }

    #[inline]
    fn line_to(&mut self, to: Point) -> EndpointId {
        self.builder.line_to(self.transform.transform_point(to))
    }

    #[inline]
    fn quadratic_bezier_to(&mut self, ctrl: Point, to: Point) -> EndpointId {
        self.builder.quadratic_bezier_to(
            self.transform.transform_point(ctrl),
            self.transform.transform_point(to),
        )
    }

    #[inline]
    fn cubic_bezier_to(&mut self, ctrl1: Point, ctrl2: Point, to: Point) -> EndpointId {
        self.builder.cubic_bezier_to(
            self.transform.transform_point(ctrl1),
            self.transform.transform_point(ctrl2),
            self.transform.transform_point(to),
        )
    }

    #[inline]
    fn reserve(&mut self, endpoints: usize, ctrl_points: usize) {
        self.builder.reserve(endpoints, ctrl_points);
    }
}

/// Implements an SVG-like building interface on top of a PathBuilder.
pub struct WithSvg<Builder: PathBuilder> {
    builder: Builder,

    first_position: Point,
    current_position: Point,
    last_ctrl: Point,
    last_cmd: Verb,
    need_moveto: bool,
    is_empty: bool,
}

impl<Builder: PathBuilder> WithSvg<Builder> {

    pub fn new(builder: Builder) -> Self {
        WithSvg {
            builder,
            first_position: point(0.0, 0.0),
            current_position: point(0.0, 0.0),
            last_ctrl: point(0.0, 0.0),
            need_moveto: true,
            is_empty: true,
            last_cmd: Verb::End,
        }
    }

    pub fn flattened(self, tolerance: f32) -> WithSvg<Flattened<Builder>> {
        WithSvg::new(Flattened::new(self.builder, tolerance))
    }

    pub fn transformed<Transform>(self, transform: Transform) -> WithSvg<Transformed<Builder, Transform>>
    where
        Transform: Transformation<f32>
    {
        WithSvg::new(Transformed::new(self.builder, transform))
    }

    pub fn move_to(&mut self, to: Point) -> EndpointId {
        self.end_if_needed();

        let id = self.builder.begin(to);

        self.is_empty = false;
        self.need_moveto = false;
        self.first_position = to;
        self.current_position = to;
        self.last_cmd = Verb::Begin;

        id
    }

    pub fn line_to(&mut self, to: Point) -> EndpointId {
        if let Some(id) = self.begin_if_needed(&to) {
            return id;
        }

        self.current_position = to;
        self.last_cmd = Verb::LineTo;

        self.builder.line_to(to)
    }

    pub fn close(&mut self) {
        if self.need_moveto {
            return;
        }

        // Relative path ops tend to accumulate small floating point imprecisions
        // which results in the last segment ending almost but not quite at the
        // start of the sub-path, causing a new edge to be inserted which often
        // intersects with the first or last edge. This can affect algorithms that
        // Don't handle self-intersecting paths.
        // Deal with this by snapping the last point if it is very close to the
        // start of the sub path.
        //
        // TODO
        // if let Some(p) = self.builder.points.last_mut() {
        //     let d = (*p - self.first_position).abs();
        //     if d.x + d.y < 0.0001 {
        //         *p = self.first_position;
        //     }
        // }

        self.current_position = self.first_position;
        self.need_moveto = true;
        self.last_cmd = Verb::Close;

        self.builder.close();
    }

    pub fn quadratic_bezier_to(&mut self, ctrl: Point, to: Point) -> EndpointId {
        if let Some(id) = self.begin_if_needed(&to) {
            return id;
        }

        self.current_position = to;
        self.last_cmd = Verb::QuadraticTo;
        self.last_ctrl = ctrl;

        self.builder.quadratic_bezier_to(ctrl, to)
    }

    pub fn cubic_bezier_to(&mut self, ctrl1: Point, ctrl2: Point, to: Point) -> EndpointId {
        if let Some(id) = self.begin_if_needed(&to) {
            return id;
        }

        self.current_position = to;
        self.last_cmd = Verb::CubicTo;
        self.last_ctrl = ctrl2;

        self.builder.cubic_bezier_to(ctrl1, ctrl2, to)
    }

    pub fn arc(&mut self, center: Point, radii: Vector, sweep_angle: Angle, x_rotation: Angle) {
        nan_check(center);
        nan_check(radii.to_point());
        debug_assert!(!sweep_angle.get().is_nan());
        debug_assert!(!x_rotation.get().is_nan());

        let start_angle = (self.current_position - center).angle_from_x_axis() - x_rotation;
        let arc = Arc {
            start_angle,
            center,
            radii,
            sweep_angle,
            x_rotation,
        };

        // If the current position is not on the arc, move or line to the beginning of the
        // arc.
        let arc_start = arc.from();
        if self.need_moveto {
            self.move_to(arc_start);
        } else if (arc_start - self.current_position).square_length() < 0.01 {
            self.builder.line_to(arc_start);
        }

        arc.for_each_quadratic_bezier(&mut |curve| {
            self.builder.quadratic_bezier_to(curve.ctrl, curve.to);
        });

        self.last_ctrl = self.current_position;
    }

    /// Ensures the current sub-path has a moveto command.
    ///
    /// Returns an ID if the command should be skipped and the ID returned instead.
    #[inline(always)]
    fn begin_if_needed(&mut self, default: &Point) -> Option<EndpointId> {
        if self.need_moveto {
            return self.insert_move_to(default)
        }

        None
    }

    #[inline(never)]
    fn insert_move_to(&mut self, default: &Point) -> Option<EndpointId> {
        if self.is_empty {
            return Some(self.move_to(*default))
        }

        self.move_to(self.first_position);

        None
    }

    fn end_if_needed(&mut self) {
        if (self.last_cmd as u8) <= (Verb::Begin as u8) {
            self.builder.end(false);
        }
    }

    pub fn current_position(&self) -> Point {
        self.current_position
    }

    pub fn reserve(&mut self, endpoints: usize, ctrl_points: usize) {
        self.builder.reserve(endpoints, ctrl_points);
    }

    fn get_smooth_cubic_ctrl(&self) -> Point {
        match self.last_cmd {
            Verb::CubicTo => self.current_position + (self.current_position - self.last_ctrl),
            _ => self.current_position,
        }
    }

    fn get_smooth_quadratic_ctrl(&self) -> Point {
        match self.last_cmd {
            Verb::QuadraticTo => self.current_position + (self.current_position - self.last_ctrl),
            _ => self.current_position,
        }
    }

    fn relative_to_absolute(&self, v: Vector) -> Point {
        self.current_position + v
    }
}

impl<Builder: PathBuilder + Build> Build for WithSvg<Builder> {
    type PathType = Builder::PathType;

    fn build(mut self) -> Builder::PathType {
        self.end_if_needed();
        self.builder.build()
    }
}

impl<Builder: PathBuilder> SvgPathBuilder for WithSvg<Builder> {
    fn move_to(&mut self, to: Point) {
        self.move_to(to);
    }

    fn line_to(&mut self, to: Point) {
        self.line_to(to);
    }

    fn quadratic_bezier_to(&mut self, ctrl: Point, to: Point) {
        self.quadratic_bezier_to(ctrl, to);
    }

    fn cubic_bezier_to(&mut self, ctrl1: Point, ctrl2: Point, to: Point) {
        self.cubic_bezier_to(ctrl1, ctrl2, to);
    }

    fn close(&mut self) {
        self.close();
    }

    fn relative_move_to(&mut self, to: Vector) {
        let to = self.relative_to_absolute(to);
        self.move_to(to);
    }

    fn relative_line_to(&mut self, to: Vector) {
        let to = self.relative_to_absolute(to);
        self.line_to(to);
    }

    fn relative_quadratic_bezier_to(&mut self, ctrl: Vector, to: Vector) {
        let ctrl = self.relative_to_absolute(ctrl);
        let to = self.relative_to_absolute(to);
        self.builder.quadratic_bezier_to(ctrl, to);
    }

    fn relative_cubic_bezier_to(&mut self, ctrl1: Vector, ctrl2: Vector, to: Vector) {
        let to = self.relative_to_absolute(to);
        let ctrl1 = self.relative_to_absolute(ctrl1);
        let ctrl2 = self.relative_to_absolute(ctrl2);
        self.cubic_bezier_to(ctrl1, ctrl2, to);
    }

    fn smooth_cubic_bezier_to(&mut self, ctrl2: Point, to: Point) {
        let ctrl1 = self.get_smooth_cubic_ctrl();
        self.cubic_bezier_to(ctrl1, ctrl2, to);
    }

    fn smooth_relative_cubic_bezier_to(&mut self, ctrl2: Vector, to: Vector) {
        let ctrl1 = self.get_smooth_cubic_ctrl();
        let ctrl2 = self.relative_to_absolute(ctrl2);
        let to = self.relative_to_absolute(to);
        self.cubic_bezier_to(ctrl1, ctrl2, to);
    }

    fn smooth_quadratic_bezier_to(&mut self, to: Point) {
        let ctrl = self.get_smooth_quadratic_ctrl();
        self.quadratic_bezier_to(ctrl, to);
    }

    fn smooth_relative_quadratic_bezier_to(&mut self, to: Vector) {
        let ctrl = self.get_smooth_quadratic_ctrl();
        let to = self.relative_to_absolute(to);
        self.quadratic_bezier_to(ctrl, to);
    }

    fn horizontal_line_to(&mut self, x: f32) {
        let y = self.current_position.y;
        self.line_to(point(x, y));
    }

    fn relative_horizontal_line_to(&mut self, dx: f32) {
        let p = self.current_position;
        self.line_to(point(p.x + dx, p.y));
    }

    fn vertical_line_to(&mut self, y: f32) {
        let x = self.current_position.x;
        self.line_to(point(x, y));
    }

    fn relative_vertical_line_to(&mut self, dy: f32) {
        let p = self.current_position;
        self.line_to(point(p.x, p.y + dy));
    }

    fn arc_to(&mut self, radii: Vector, x_rotation: Angle, flags: ArcFlags, to: Point) {
        let arc = SvgArc {
            from: self.current_position,
            to,
            radii,
            x_rotation,
            flags: ArcFlags {
                large_arc: flags.large_arc,
                sweep: flags.sweep,
            },
        }.to_arc();

        self.arc(arc.center, arc.radii, arc.sweep_angle, arc.x_rotation);
    }

    fn relative_arc_to(&mut self, radii: Vector, x_rotation: Angle, flags: ArcFlags, to: Vector) {
        let to = self.relative_to_absolute(to);
        self.arc_to(radii, x_rotation, flags, to);
    }

    fn reserve(&mut self, endpoints: usize, ctrl_points: usize) {
        self.builder.reserve(endpoints, ctrl_points);
    }
}

// TODO: not sure whether we want to expose this.
#[doc(hidden)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct DebugValidator {
    #[cfg(debug_assertions)]
    in_subpath: bool,
}

impl DebugValidator {
    #[inline(always)]
    pub fn new() -> Self {
        DebugValidator {
            #[cfg(debug_assertions)]
            in_subpath: false,
        }
    }

    #[inline(always)]
    pub fn begin(&mut self) {
        #[cfg(debug_assertions)] {
            assert!(!self.in_subpath);
            self.in_subpath = true;
        }
    }

    #[inline(always)]
    pub fn end(&mut self) {
        #[cfg(debug_assertions)] {
            assert!(self.in_subpath);
            self.in_subpath = false;
        }
    }

    #[inline(always)]
    pub fn edge(&self) {
        #[cfg(debug_assertions)] {
            assert!(self.in_subpath);
        }
    }

    #[inline(always)]
    pub fn build(&self) {
        #[cfg(debug_assertions)] {
            assert!(!self.in_subpath);
        }
    }
}

#[doc(hidden)]
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

#[doc(hidden)]
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

#[inline]
fn nan_check(p: Point) {
    debug_assert!(p.x.is_finite());
    debug_assert!(p.y.is_finite());
}

#[test]
fn extended_builder_line_to_after_close() {
    use crate::Path;
    use crate::PathEvent;

    let mut p = Path::extended_builder();
    p.line_to(point(1.0, 0.0));
    p.close();
    p.line_to(point(2.0, 0.0));

    let path = p.build();
    let mut it = path.iter();
    assert_eq!(
        it.next(),
        Some(PathEvent::Begin {
            at: point(1.0, 0.0)
        })
    );
    assert_eq!(
        it.next(),
        Some(PathEvent::End {
            last: point(1.0, 0.0),
            first: point(1.0, 0.0),
            close: true
        })
    );
    assert_eq!(
        it.next(),
        Some(PathEvent::Begin {
            at: point(1.0, 0.0)
        })
    );
    assert_eq!(
        it.next(),
        Some(PathEvent::Line {
            from: point(1.0, 0.0),
            to: point(2.0, 0.0)
        })
    );
    assert_eq!(
        it.next(),
        Some(PathEvent::End {
            last: point(2.0, 0.0),
            first: point(1.0, 0.0),
            close: false
        })
    );
    assert_eq!(it.next(), None);
}
