//! Elliptic arc related maths and tools.

use std::ops::Range;

use Line;
use scalar::{Float, FloatExt, FloatConst, Trig, ApproxEq, cast};
use generic_math::{Point, point, Vector, vector, Rotation2D, Transform2D, Angle, Rect};
use utils::directed_angle;
use segment::{Segment, FlattenedForEach, FlatteningStep, BoundingRect};
use segment;

/// A flattening iterator for arc segments.
pub type Flattened<S> = segment::Flattened<S, Arc<S>>;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct SvgArc<S: Float> {
    pub from: Point<S>,
    pub to: Point<S>,
    pub radii: Vector<S>,
    pub x_rotation: Angle<S>,
    pub flags: ArcFlags,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Arc<S: Float> {
    pub center: Point<S>,
    pub radii: Vector<S>,
    pub start_angle: Angle<S>,
    pub sweep_angle: Angle<S>,
    pub x_rotation: Angle<S>,
}

impl<S: Float + FloatConst + Trig + ::std::fmt::Debug> Arc<S> {
    pub fn from_svg_arc(arc: &SvgArc<S>) -> Arc<S> {
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
        assert_ne!(rx, S::zero());
        assert_ne!(ry, S::zero());

        let xr = arc.x_rotation.get() % (S::c(2.0) * S::PI());
        let cos_phi = Float::cos(xr);
        let sin_phi = Float::sin(xr);
        let hd_x = (arc.from.x - arc.to.x) / S::c(2.0);
        let hd_y = (arc.from.y - arc.to.y) / S::c(2.0);
        let hs_x = (arc.from.x + arc.to.x) / S::c(2.0);
        let hs_y = (arc.from.y + arc.to.y) / S::c(2.0);
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

        debug_assert_ne!(sum_of_sq, S::zero());

        let sign_coe = if arc.flags.large_arc == arc.flags.sweep {-S::one() } else { S::one() };
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

        let start_angle = Angle::radians(directed_angle(vector(S::one(), S::zero()), a));

        let sign_delta = if arc.flags.sweep { S::one() } else { -S::one() };
        let sweep_angle = Angle::radians(sign_delta * (directed_angle(a, b).abs() % (S::c(2.0) * S::PI())));

        Arc {
            center: center,
            radii: arc.radii,
            start_angle: start_angle,
            sweep_angle: sweep_angle,
            x_rotation: arc.x_rotation
        }
    }
}

impl<S: Float + FloatConst + ApproxEq<S>> Arc<S> {
    pub fn to_svg_arc(&self) -> SvgArc<S> {
        let from = self.sample(S::zero());
        let to = self.sample(S::one());
        let flags = ArcFlags {
            sweep: S::abs(self.sweep_angle.get()) >= S::PI(),
            large_arc: self.sweep_angle.get() >= S::zero(),
        };
        SvgArc {
            from,
            to,
            radii: self.radii,
            x_rotation: self.x_rotation,
            flags,
        }
    }
}

impl<S: Float + FloatConst + ApproxEq<S>> Arc<S> {
    #[inline]
    pub fn to_quadratic_beziers<F: FnMut(Point<S>, Point<S>)>(&self, cb: &mut F) {
        arc_to_to_quadratic_beziers(self, cb);
    }
}

impl<S: Float + ApproxEq<S>> Arc<S> {
    /// Sample the curve at t (expecting t between 0 and 1).
    #[inline]
    pub fn sample(&self, t: S) -> Point<S> {
        let angle = self.get_angle(t);
        self.center + sample_ellipse(self.radii, self.x_rotation, angle).to_vector()
    }

    #[inline]
    pub fn x(&self, t: S) -> S { self.sample(t).x }

    #[inline]
    pub fn y(&self, t: S) -> S { self.sample(t).y }

    /// Sample the curve's tangent at t (expecting t between 0 and 1).
    #[inline]
    pub fn sample_tangent(&self, t: S) -> Vector<S> {
        self.tangent_at_angle(self.get_angle(t))
    }
}

impl<S: Float> Arc<S> {
    /// Sample the curve's angle at t (expecting t between 0 and 1).
    #[inline]
    pub fn get_angle(&self, t: S) -> Angle<S> {
        self.start_angle + Angle::radians(self.sweep_angle.get() * t)
    }

    #[inline]
    pub fn end_angle(&self) -> Angle<S> {
        self.start_angle + self.sweep_angle
    }
}

impl<S: Float + ApproxEq<S>> Arc<S> {
    #[inline]
    pub fn from(&self) -> Point<S> {
        self.sample(S::zero())
    }

    #[inline]
    pub fn to(&self) -> Point<S> {
        self.sample(S::one())
    }
}

impl<S: Float> Arc<S> {
    /// Return the sub-curve inside a given range of t.
    ///
    /// This is equivalent splitting at the range's end points.
    pub fn split_range(&self, t_range: Range<S>) -> Self {
        let angle_1 = Angle::radians(self.sweep_angle.get() * t_range.start);
        let angle_2 = Angle::radians(self.sweep_angle.get() * t_range.end);

        Arc {
            center: self.center,
            radii: self.radii,
            start_angle: self.start_angle + angle_1,
            sweep_angle: angle_2 - angle_1,
            x_rotation: self.x_rotation,
        }
    }

    /// Split this curve into two sub-curves.
    pub fn split(&self, t: S) -> (Arc<S>, Arc<S>) {
        let split_angle = Angle::radians(self.sweep_angle.get() * t);
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
    pub fn before_split(&self, t: S) -> Arc<S> {
        let split_angle = Angle::radians(self.sweep_angle.get() * t);
        Arc {
            center: self.center,
            radii: self.radii,
            start_angle: self.start_angle,
            sweep_angle: split_angle,
            x_rotation: self.x_rotation,
        }
    }

    /// Return the curve after the split point.
    pub fn after_split(&self, t: S) -> Arc<S> {
        let split_angle = Angle::radians(self.sweep_angle.get() * t);
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
}

impl<S: Float + ApproxEq<S>> Arc<S> {
    /// Iterates through the curve invoking a callback at each point.
    pub fn flattened_for_each<F: FnMut(Point<S>)>(&self, tolerance: S, call_back: &mut F) {
        <Self as FlattenedForEach>::flattened_for_each(self, tolerance, call_back);
    }

    /// Iterates through the curve invoking a callback at each point.
    pub fn flattening_step(&self, tolerance: S) -> S {
        // Here we make the approximation that for small tolerance values we consider
        // the radius to be constant over each approximated segment.
        let r = (self.from() - self.center).length();
        let a = S::c(2.0) * tolerance * r - tolerance * tolerance;
        S::acos((a * a) / r)
    }

    /// Returns the flattened representation of the curve as an iterator, starting *after* the
    /// current point.
    pub fn flattened(&self, tolerance: S) -> Flattened<S> {
        Flattened::new(*self, tolerance)
    }
}

impl<S: Float + Trig> Arc<S> {
    /// Returns a conservative rectangle that contains the curve.
    pub fn bounding_rect(&self) -> Rect<S> {
        Transform2D::create_rotation(self.x_rotation).transform_rect(
            &Rect::new(
                self.center - self.radii,
                self.radii.to_size() * S::c(2.0)
            )
        )
    }

    pub fn bounding_range_x(&self) -> (S, S) {
        let r = self.bounding_rect();
        (r.min_x(), r.max_x())
    }

    pub fn bounding_range_y(&self) -> (S, S) {
        let r = self.bounding_rect();
        (r.min_y(), r.max_y())
    }
}

impl<S: Float + ApproxEq<S>> Arc<S> {
    pub fn approximate_length(&self, tolerance: S) -> S {
        segment::approximate_length_from_flattening(self, tolerance)
    }

    #[inline]
    fn tangent_at_angle(&self, angle: Angle<S>) -> Vector<S> {
        let a = angle.get();
        Rotation2D::new(self.x_rotation).transform_vector(
            &vector(-self.radii.x * Float::sin(a), self.radii.y * Float::cos(a))
        )
    }
}

impl<S: Float + FloatConst + Trig + ApproxEq<S> + ::std::fmt::Debug> Into<Arc<S>> for SvgArc<S> {
    fn into(self) -> Arc<S> { self.to_arc() }
}

impl<S: Float + FloatConst + Trig + ApproxEq<S> + ::std::fmt::Debug> SvgArc<S> {
    pub fn to_arc(&self) -> Arc<S> { Arc::from_svg_arc(self) }

    pub fn to_quadratic_beziers<F: FnMut(Point<S>, Point<S>)>(&self, cb: &mut F) {
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

fn arc_to_to_quadratic_beziers<S: Float + FloatConst + ApproxEq<S>, F: FnMut(Point<S>, Point<S>)>(
    arc: &Arc<S>,
    call_back: &mut F,
) {
    let sweep_angle = arc.sweep_angle.get().abs().min(S::PI() * S::c(2.0));

    let n_steps = (sweep_angle / S::FRAC_PI_4()).ceil();
    let step = sweep_angle / n_steps;

    for i in 0..cast::<S, i32>(n_steps).unwrap() {
        let a1 = arc.start_angle.get() + step * cast(i).unwrap();
        let a2 = arc.start_angle.get() + step * cast(i+1).unwrap();

        let v1 = sample_ellipse(arc.radii, arc.x_rotation, Angle::radians(a1)).to_vector();
        let v2 = sample_ellipse(arc.radii, arc.x_rotation, Angle::radians(a2)).to_vector();
        let p1 = arc.center + v1;
        let p2 = arc.center + v2;
        let l1 = Line { point: p1, vector: arc.tangent_at_angle(Angle::radians(a1)) };
        let l2 = Line { point: p2, vector: arc.tangent_at_angle(Angle::radians(a2)) };
        let ctrl = l2.intersection(&l1).unwrap();

        call_back(ctrl, p2);
    }
}

fn sample_ellipse<S: Float + ApproxEq<S>>(radii: Vector<S>, x_rotation: Angle<S>, angle: Angle<S>) -> Point<S> {
    Rotation2D::new(x_rotation).transform_point(
        &point(radii.x * angle.get().cos(), radii.y * angle.get().sin())
    )
}

impl<S: Float + ApproxEq<S>> Segment for Arc<S> {
    type Scalar = S;
    fn from(&self) -> Point<S> { self.from() }
    fn to(&self) -> Point<S> { self.to() }
    fn sample(&self, t: S) -> Point<S> { self.sample(t) }
    fn x(&self, t: S) -> S { self.x(t) }
    fn y(&self, t: S) -> S { self.y(t) }
    fn derivative(&self, t: S) -> Vector<S> { self.sample_tangent(t) }
    fn split_range(&self, t_range: Range<S>) -> Self { self.split_range(t_range) }
    fn split(&self, t: S) -> (Self, Self) { self.split(t) }
    fn before_split(&self, t: S) -> Self { self.before_split(t) }
    fn after_split(&self, t: S) -> Self { self.after_split(t) }
    fn flip(&self) -> Self { self.flip() }
    fn approximate_length(&self, tolerance: S) -> S {
        self.approximate_length(tolerance)
    }
}

impl<S: Float + Trig> BoundingRect for Arc<S> {
    type Scalar = S;
    fn bounding_rect(&self) -> Rect<S> { self.bounding_rect() }
    fn fast_bounding_rect(&self) -> Rect<S> { self.bounding_rect() }
    fn bounding_range_x(&self) -> (S, S) { self.bounding_range_x() }
    fn bounding_range_y(&self) -> (S, S) { self.bounding_range_y() }
    fn fast_bounding_range_x(&self) -> (S, S) { self.bounding_range_x() }
    fn fast_bounding_range_y(&self) -> (S, S) { self.bounding_range_y() }
}

impl<S: Float + ApproxEq<S>> FlatteningStep for Arc<S> {
    fn flattening_step(&self, tolerance: S) -> S {
        self.flattening_step(tolerance)
    }
}
