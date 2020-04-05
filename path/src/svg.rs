//! A path building interface working with SVG commands.
//!
//! The `SvgPathBuilder` trait provides a richer set of commands than `PathBuilder`, is less
//! strict about explicitly beginning and ending sub paths. 
//!
//! ## Examples
//!
//! ```ignore
//! let mut builder = Path::svg_builder();
//!
//! builder.move_to(point(0.0, 0.0));
//! builder.horizontal_line_to(1.0);
//! builder.relative_quadratic_bezier_to(point(1.0, 0.0), point(1.0, 1.0));
//! builder.smooth_relative_quadratic_bezier_to(point(-1.0, 1.0));
//!
//! let path = builder.build();
//! ```


use crate::math::*;
use crate::geom::{Arc, ArcFlags, SvgArc};
use crate::builder::{PathBuilder, Flattened, Build};
use crate::EndpointId;
use crate::path::Verb;

/// A path building interface that tries to stay close to SVG's path specification.
/// https://svgwg.org/specs/paths/
pub trait SvgPathBuilder {
    fn move_to(&mut self, to: Point);
    fn line_to(&mut self, to: Point);
    fn quadratic_bezier_to(&mut self, ctrl: Point, to: Point);
    fn cubic_bezier_to(&mut self, ctrl1: Point, ctrl2: Point, to: Point);
    fn close(&mut self);

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
    fn relative_arc_to(&mut self, radii: Vector, x_rotation: Angle, flags: ArcFlags, to: Vector);
}

/// Implements the Svg building interface on top of a PathBuilder.
pub struct SvgBuilder<Builder: PathBuilder> {
    builder: Builder,

    first_position: Point,
    current_position: Point,
    last_ctrl: Point,
    last_cmd: Verb,
    need_moveto: bool,
    is_empty: bool,
}

impl<Builder: PathBuilder> SvgBuilder<Builder> {

    pub fn new(builder: Builder) -> Self {
        SvgBuilder {
            builder,
            first_position: point(0.0, 0.0),
            current_position: point(0.0, 0.0),
            last_ctrl: point(0.0, 0.0),
            need_moveto: true,
            is_empty: true,
            last_cmd: Verb::End,
        }
    }

    pub fn flattened(self, tolerance: f32) -> SvgBuilder<Flattened<Builder>> {
        SvgBuilder::new(Flattened::new(self.builder, tolerance))
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

impl<Builder: PathBuilder + Build> Build for SvgBuilder<Builder> {
    type PathType = Builder::PathType;

    fn build(mut self) -> Builder::PathType {
        self.end_if_needed();
        self.builder.build()
    }
}

impl<Builder: PathBuilder> SvgPathBuilder for SvgBuilder<Builder> {
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
}

#[inline]
fn nan_check(p: Point) {
    debug_assert!(p.x.is_finite());
    debug_assert!(p.y.is_finite());
}

#[test]
fn svg_builder_line_to_after_close() {
    use crate::Path;
    use crate::PathEvent;

    let mut p = Path::svg_builder();
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
