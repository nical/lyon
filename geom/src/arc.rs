//! Elliptic arc related maths and tools.

use std::f32::*;
use std::f32;

use Line;
use math::{Point, point, Vector, vector, Rotation2D, Transform2D, Radians, Rect};
use utils::directed_angle;
use segment::{Segment, FlattenedForEach, FlatteningStep, BoundingRect};
use segment;

/// A flattening iterator for arc segments.
pub type Flattened = segment::Flattened<Arc>;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct SvgArc {
    pub from: Point,
    pub to: Point,
    pub radii: Vector,
    pub x_rotation: Radians,
    pub flags: ArcFlags,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Arc {
    pub center: Point,
    pub radii: Vector,
    pub start_angle: Radians,
    pub sweep_angle: Radians,
    pub x_rotation: Radians,
}

impl Arc {
    pub fn from_svg_arc(arc: &SvgArc) -> Arc {
        debug_assert!(!arc.from.x.is_nan());
        debug_assert!(!arc.from.y.is_nan());
        debug_assert!(!arc.to.x.is_nan());
        debug_assert!(!arc.to.y.is_nan());
        debug_assert!(!arc.radii.x.is_nan());
        debug_assert!(!arc.radii.y.is_nan());
        debug_assert!(!arc.x_rotation.get().is_nan());

        let rx = arc.radii.x;
        let ry = arc.radii.y;

        assert_ne!(arc.from, arc.to);
        assert_ne!(rx, 0.0);
        assert_ne!(ry, 0.0);

        let xr = arc.x_rotation.get() % (2.0 * consts::PI);
        let cos_phi = xr.cos();
        let sin_phi = xr.sin();
        let hd_x = (arc.from.x - arc.to.x) / 2.0;
        let hd_y = (arc.from.y - arc.to.y) / 2.0;
        let hs_x = (arc.from.x + arc.to.x) / 2.0;
        let hs_y = (arc.from.y + arc.to.y) / 2.0;
        // F6.5.1
        let p = Point::new(
            cos_phi * hd_x + sin_phi * hd_y,
            -sin_phi * hd_x + cos_phi * hd_y,
        );

        // TODO: sanitize radii

        let rxry = rx * ry;
        let rxpy = rx * p.y;
        let rypx = ry * p.x;
        let sum_of_sq = rxpy * rxpy + rypx * rypx;

        debug_assert_ne!(sum_of_sq, 0.0);

        let sign_coe = if arc.flags.large_arc == arc.flags.sweep {-1.0 } else { 1.0 };
        let coe = sign_coe * ((rxry * rxry - sum_of_sq) / sum_of_sq).abs().sqrt();

        let transformed_cx = coe * rxpy / ry;
        let transformed_cy = -coe * rypx / rx;

        // F6.5.3
        let center = point(
            cos_phi * transformed_cx - sin_phi * transformed_cy + hs_x,
            sin_phi * transformed_cx + cos_phi * transformed_cy + hs_y
        );

        let a = vector(
            (p.x - transformed_cx) / rx,
            (p.y - transformed_cy) / ry,
        );
        // TODO
        let b = -vector(
            (-p.x - transformed_cx) / rx,
            (-p.y - transformed_cy) / ry,
        );

        let start_angle = Radians::new(directed_angle(vector(1.0, 0.0), a));

        let sign_delta = if arc.flags.sweep { 1.0 } else { -1.0 };
        let sweep_angle = Radians::new(sign_delta * (directed_angle(a, b).abs() % (2.0 * consts::PI)));

        Arc {
            center: center,
            radii: arc.radii,
            start_angle: start_angle,
            sweep_angle: sweep_angle,
            x_rotation: arc.x_rotation
        }
    }

    pub fn to_svg_arc(&self) -> SvgArc {
        let from = self.sample(0.0);
        let to = self.sample(1.0);
        let flags = ArcFlags {
            sweep: f32::abs(self.sweep_angle.get()) >= consts::PI,
            large_arc: self.sweep_angle.get() >= 0.0,
        };
        SvgArc {
            from,
            to,
            radii: self.radii,
            x_rotation: self.x_rotation,
            flags,
        }
    }

    #[inline]
    pub fn to_quadratic_beziers<F: FnMut(Point, Point)>(&self, cb: &mut F) {
        arc_to_to_quadratic_beziers(self, cb);
    }

    /// Sample the curve at t (expecting t between 0 and 1).
    #[inline]
    pub fn sample(&self, t: f32) -> Point {
        let angle = self.get_angle(t);
        self.center + sample_ellipse(self.radii, self.x_rotation, angle).to_vector()
    }

    #[inline]
    pub fn x(&self, t: f32) -> f32 { self.sample(t).x }

    #[inline]
    pub fn y(&self, t: f32) -> f32 { self.sample(t).y }

    /// Sample the curve's tangent at t (expecting t between 0 and 1).
    #[inline]
    pub fn sample_tangent(&self, t: f32) -> Vector {
        self.tangent_at_angle(self.get_angle(t))
    }

    /// Sample the curve's angle at t (expecting t between 0 and 1).
    #[inline]
    pub fn get_angle(&self, t: f32) -> Radians {
        self.start_angle + Radians::new(self.sweep_angle.get() * t)
    }

    #[inline]
    pub fn end_angle(&self) -> Radians {
        self.start_angle + self.sweep_angle
    }

    #[inline]
    pub fn from(&self) -> Point {
        self.sample(0.0)
    }

    #[inline]
    pub fn to(&self) -> Point {
        self.sample(1.0)
    }

    /// Split this curve into two sub-curves.
    pub fn split(&self, t: f32) -> (Arc, Arc) {
        let split_angle = Radians::new(self.sweep_angle.get() * t);
        (
            Arc {
                center: self.center,
                radii: self.radii,
                start_angle: self.start_angle,
                sweep_angle: split_angle,
                x_rotation: self.x_rotation,
            },
            Arc {
                center: self.center,
                radii: self.radii,
                start_angle: self.start_angle + split_angle,
                sweep_angle: self.sweep_angle - split_angle,
                x_rotation: self.x_rotation,
            },
        )
    }

    /// Return the curve before the split point.
    pub fn before_split(&self, t: f32) -> Arc {
        let split_angle = Radians::new(self.sweep_angle.get() * t);
        Arc {
            center: self.center,
            radii: self.radii,
            start_angle: self.start_angle,
            sweep_angle: split_angle,
            x_rotation: self.x_rotation,
        }
    }

    /// Return the curve after the split point.
    pub fn after_split(&self, t: f32) -> Arc {
        let split_angle = Radians::new(self.sweep_angle.get() * t);
        Arc {
            center: self.center,
            radii: self.radii,
            start_angle: self.start_angle + split_angle,
            sweep_angle: self.sweep_angle - split_angle,
            x_rotation: self.x_rotation,
        }
    }

    /// Swap the direction of the segment.
    pub fn flip(&self) -> Self {
        let mut arc = *self;
        arc.start_angle = arc.start_angle + self.sweep_angle;
        arc.sweep_angle = -self.sweep_angle;

        arc
    }

    /// Iterates through the curve invoking a callback at each point.
    pub fn flattened_for_each<F: FnMut(Point)>(&self, tolerance: f32, call_back: &mut F) {
        <Self as FlattenedForEach>::flattened_for_each(self, tolerance, call_back);
    }

    /// Iterates through the curve invoking a callback at each point.
    pub fn flattening_step(&self, tolerance: f32) -> f32 {
        // Here we make the approximation that for small tolerance values we consider
        // the radius to be constant over each approximated segment.
        let r = (self.from() - self.center).length();
        let a = 2.0 * tolerance * r - tolerance * tolerance;
        f32::acos((a * a) / r)
    }

    /// Returns the flattened representation of the curve as an iterator, starting *after* the
    /// current point.
    pub fn flattened(&self, tolerance: f32) -> Flattened {
        Flattened::new(*self, tolerance)
    }

    /// Returns a conservative rectangle that contains the curve.
    pub fn bounding_rect(&self) -> Rect {
        Transform2D::create_rotation(self.x_rotation).transform_rect(
            &Rect::new(
                self.center - self.radii,
                self.radii.to_size() * 2.0
            )
        )
    }

    pub fn approximate_length(&self, tolerance: f32) -> f32 {
        segment::approximate_length_from_flattening(self, tolerance)
    }

    #[inline]
    fn tangent_at_angle(&self, angle: Radians) -> Vector {
        let a = angle.get();
        Rotation2D::new(self.x_rotation).transform_vector(
            &vector(-self.radii.x * a.sin(), self.radii.y * a.cos())
        )
    }
}

impl Into<Arc> for SvgArc {
    fn into(self) -> Arc { self.to_arc() }
}

impl SvgArc {
    pub fn to_arc(&self) -> Arc { Arc::from_svg_arc(self) }

    pub fn to_quadratic_beziers<F: FnMut(Point, Point)>(&self, cb: &mut F) {
        Arc::from_svg_arc(self).to_quadratic_beziers(cb);
    }
}

/// Flag parameters for arcs as described by the SVG specification.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ArcFlags {
    pub large_arc: bool,
    pub sweep: bool,
}

impl Default for ArcFlags {
    fn default() -> Self {
        ArcFlags {
            large_arc: false,
            sweep: false,
        }
    }
}

fn arc_to_to_quadratic_beziers<F: FnMut(Point, Point)>(
    arc: &Arc,
    call_back: &mut F,
) {
    let sweep_angle = arc.sweep_angle.get().abs().min(consts::PI * 2.0);

    let n_steps = (sweep_angle / consts::FRAC_PI_4).ceil();
    let step = sweep_angle / n_steps;

    for i in 0..(n_steps as i32) {
        let a1 = arc.start_angle.get() + step * (i as f32);
        let a2 = arc.start_angle.get() + step * ((i+1) as f32);

        let v1 = sample_ellipse(arc.radii, arc.x_rotation, Radians::new(a1)).to_vector();
        let v2 = sample_ellipse(arc.radii, arc.x_rotation, Radians::new(a2)).to_vector();
        let p1 = arc.center + v1;
        let p2 = arc.center + v2;
        let l1 = Line { point: p1, vector: arc.tangent_at_angle(Radians::new(a1)) };
        let l2 = Line { point: p2, vector: arc.tangent_at_angle(Radians::new(a2)) };
        let ctrl = l2.intersection(&l1).unwrap();

        call_back(ctrl, p2);
    }
}

fn sample_ellipse(radii: Vector, x_rotation: Radians, angle: Radians) -> Point {
    Rotation2D::new(x_rotation).transform_point(
        &point(radii.x * angle.get().cos(), radii.y * angle.get().sin())
    )
}

impl Segment for Arc {
    fn from(&self) -> Point { self.from() }
    fn to(&self) -> Point { self.to() }
    fn sample(&self, t: f32) -> Point { self.sample(t) }
    fn x(&self, t: f32) -> f32 { self.x(t) }
    fn y(&self, t: f32) -> f32 { self.y(t) }
    fn derivative(&self, t: f32) -> Vector { self.sample_tangent(t) }
    fn split(&self, t: f32) -> (Self, Self) { self.split(t) }
    fn before_split(&self, t: f32) -> Self { self.before_split(t) }
    fn after_split(&self, t: f32) -> Self { self.after_split(t) }
    fn flip(&self) -> Self { self.flip() }
    fn approximate_length(&self, tolerance: f32) -> f32 {
        self.approximate_length(tolerance)
    }
}

impl BoundingRect for Arc {
    fn bounding_rect(&self) -> Rect { self.bounding_rect() }
    fn fast_bounding_rect(&self) -> Rect { self.bounding_rect() }
}

impl FlatteningStep for Arc {
    fn flattening_step(&self, tolerance: f32) -> f32 {
        self.flattening_step(tolerance)
    }
}
