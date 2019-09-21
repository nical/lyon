//! Elliptic arc related maths and tools.

use std::ops::Range;
use std::mem::swap;

use crate::Line;
use crate::scalar::{Scalar, Float, cast};
use crate::generic_math::{Point, point, Vector, vector, Rotation2D, Transform2D, Angle, Rect};
use crate::segment::{Segment, FlatteningStep, BoundingRect};
use crate::segment;
use crate::QuadraticBezierSegment;
use crate::CubicBezierSegment;

/// A flattening iterator for arc segments.
pub type Flattened<S> = segment::Flattened<S, Arc<S>>;

/// An elliptic arc curve segment using the SVG's end-point notation.
#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct SvgArc<S> {
    pub from: Point<S>,
    pub to: Point<S>,
    pub radii: Vector<S>,
    pub x_rotation: Angle<S>,
    pub flags: ArcFlags,
}

/// An elliptic arc curve segment.
#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct Arc<S> {
    pub center: Point<S>,
    pub radii: Vector<S>,
    pub start_angle: Angle<S>,
    pub sweep_angle: Angle<S>,
    pub x_rotation: Angle<S>,
}

impl<S: Scalar> Arc<S> {
    /// Create simple circle.
    pub fn circle(center: Point<S>, radius: S) -> Self {
        Arc {
            center,
            radii: vector(radius, radius),
            start_angle: Angle::zero(),
            sweep_angle: Angle::two_pi(),
            x_rotation: Angle::zero(),
        }
    }

    /// Convert from the SVG arc notation.
    pub fn from_svg_arc(arc: &SvgArc<S>) -> Arc<S> {
        debug_assert!(!arc.from.x.is_nan());
        debug_assert!(!arc.from.y.is_nan());
        debug_assert!(!arc.to.x.is_nan());
        debug_assert!(!arc.to.y.is_nan());
        debug_assert!(!arc.radii.x.is_nan());
        debug_assert!(!arc.radii.y.is_nan());
        debug_assert!(!arc.x_rotation.get().is_nan());
        // The SVG spec specifies what we should do if one of the two
        // radii is zero and not the other, but it's better to handle
        // this out of arc code and generate a line_to instead of an arc.
        assert!(!arc.is_straight_line());

        let mut rx = S::abs(arc.radii.x);
        let mut ry = S::abs(arc.radii.y);

        let xr = arc.x_rotation.get() % (S::TWO * S::PI());
        let cos_phi = Float::cos(xr);
        let sin_phi = Float::sin(xr);
        let hd_x = (arc.from.x - arc.to.x) / S::TWO;
        let hd_y = (arc.from.y - arc.to.y) / S::TWO;
        let hs_x = (arc.from.x + arc.to.x) / S::TWO;
        let hs_y = (arc.from.y + arc.to.y) / S::TWO;

        // F6.5.1
        let p = Point::new(
            cos_phi * hd_x + sin_phi * hd_y,
            -sin_phi * hd_x + cos_phi * hd_y,
        );

        // Sanitize the radii.
        // If rf > 1 it means the radii are too small for the arc to
        // possibly connect the end points. In this situation we scale
        // them up according to the formula provided by the SVG spec.

        // F6.6.2
        let rf = p.x * p.x / (rx * rx) + p.y * p.y / (ry * ry);
        if rf > S::ONE {
            let scale = S::sqrt(rf);
            rx *= scale;
            ry *= scale;
        }

        let rxry = rx * ry;
        let rxpy = rx * p.y;
        let rypx = ry * p.x;
        let sum_of_sq = rxpy * rxpy + rypx * rypx;

        debug_assert_ne!(sum_of_sq, S::ZERO);

        // F6.5.2
        let sign_coe = if arc.flags.large_arc == arc.flags.sweep {-S::ONE } else { S::ONE };
        let coe = sign_coe * S::sqrt(S::abs((rxry * rxry - sum_of_sq) / sum_of_sq));
        let transformed_cx = coe * rxpy / ry;
        let transformed_cy = -coe * rypx / rx;

        // F6.5.3
        let center = point(
            cos_phi * transformed_cx - sin_phi * transformed_cy + hs_x,
            sin_phi * transformed_cx + cos_phi * transformed_cy + hs_y
        );

        let start_v: Vector<S> = vector(
            (p.x - transformed_cx) / rx,
            (p.y - transformed_cy) / ry,
        );
        let end_v: Vector<S> = vector(
            (-p.x - transformed_cx) / rx,
            (-p.y - transformed_cy) / ry,
        );

        let two_pi = S::TWO * S::PI();

        let start_angle = start_v.angle_from_x_axis();

        let mut sweep_angle = (end_v.angle_from_x_axis() - start_angle).radians % two_pi;

        if arc.flags.sweep && sweep_angle < S::ZERO {
            sweep_angle += two_pi;
        } else if !arc.flags.sweep && sweep_angle > S::ZERO {
            sweep_angle -= two_pi;
        }

        Arc {
            center,
            radii: vector(rx, ry),
            start_angle,
            sweep_angle: Angle::radians(sweep_angle),
            x_rotation: arc.x_rotation
        }
    }

    /// Convert to the SVG arc notation.
    pub fn to_svg_arc(&self) -> SvgArc<S> {
        let from = self.sample(S::ZERO);
        let to = self.sample(S::ONE);
        let flags = ArcFlags {
            sweep: S::abs(self.sweep_angle.get()) >= S::PI(),
            large_arc: self.sweep_angle.get() >= S::ZERO,
        };
        SvgArc {
            from,
            to,
            radii: self.radii,
            x_rotation: self.x_rotation,
            flags,
        }
    }

    /// Approximate the arc with a sequence of quadratic bézier curves.
    #[inline]
    pub fn for_each_quadratic_bezier<F>(&self, cb: &mut F)
    where
        F: FnMut(&QuadraticBezierSegment<S>)
    {
        arc_to_quadratic_beziers(self, cb);
    }

    /// Approximate the arc with a sequence of cubic bézier curves.
    #[inline]
    pub fn for_each_cubic_bezier<F>(&self, cb: &mut F)
    where
        F: FnMut(&CubicBezierSegment<S>)
    {
        arc_to_cubic_beziers(self, cb);
    }

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

    /// Sample the curve's angle at t (expecting t between 0 and 1).
    #[inline]
    pub fn get_angle(&self, t: S) -> Angle<S> {
        self.start_angle + Angle::radians(self.sweep_angle.get() * t)
    }

    #[inline]
    pub fn end_angle(&self) -> Angle<S> {
        self.start_angle + self.sweep_angle
    }

    #[inline]
    pub fn from(&self) -> Point<S> {
        self.sample(S::ZERO)
    }

    #[inline]
    pub fn to(&self) -> Point<S> {
        self.sample(S::ONE)
    }

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

    /// Approximates the arc with a sequence of line segments.
    pub fn for_each_flattened<F>(&self, tolerance: S, call_back: &mut F)
    where
        F: FnMut(Point<S>)
    {
        crate::segment::for_each_flattened(self, tolerance, call_back);
    }

    /// Iterates through the curve invoking a callback at each point.
    pub fn for_each_flattened_with_t<F>(&self, tolerance: S, call_back: &mut F)
    where
        F: FnMut(Point<S>, S)
    {
        crate::segment::for_each_flattened_with_t(self, tolerance, call_back);
    }

    /// Finds the interval of the beginning of the curve that can be approximated with a
    /// line segment.
    pub fn flattening_step(&self, tolerance: S) -> S {
        // cos(theta) = (r - tolerance) / r
        // angle = 2 * theta
        // s = angle / sweep

        // Here we make the approximation that for small tolerance values we consider
        // the radius to be constant over each approximated segment.
        let r = (self.from() - self.center).length();
        let a = S::TWO * S::acos((r - tolerance) / r);
        let result = S::min(a / self.sweep_angle.radians, S::ONE);

        if result < S::EPSILON {
            return S::ONE;
        }

        result
    }

    /// Returns the flattened representation of the curve as an iterator, starting *after* the
    /// current point.
    pub fn flattened(&self, tolerance: S) -> Flattened<S> {
        Flattened::new(*self, tolerance)
    }

    /// Returns a conservative rectangle that contains the curve.
    pub fn fast_bounding_rect(&self) -> Rect<S> {
        Transform2D::create_rotation(self.x_rotation).transform_rect(
            &Rect::new(
                self.center - self.radii,
                self.radii.to_size() * S::TWO
            )
        )
    }

    /// Returns a conservative rectangle that contains the curve.
    pub fn bounding_rect(&self) -> Rect<S> {
        let from = self.from();
        let to = self.to();
        let mut min = Point::min(from, to);
        let mut max = Point::max(from, to);
        self.for_each_local_x_extremum_t(&mut|t| {
            let p = self.sample(t);
            min.x = S::min(min.x, p.x);
            max.x = S::max(max.x, p.x);
        });
        self.for_each_local_y_extremum_t(&mut|t| {
            let p = self.sample(t);
            min.y = S::min(min.y, p.y);
            max.y = S::max(max.y, p.y);
        });

        Rect {
            origin: min,
            size: (max - min).to_size(),
        }
    }

    pub fn for_each_local_x_extremum_t<F>(&self, cb: &mut F)
    where F: FnMut(S) {
        let rx = self.radii.x;
        let ry = self.radii.y;
        let a1 = Angle::radians(-S::atan(ry * Float::tan(self.x_rotation.radians) / rx));
        let a2 = Angle::pi() + a1;

        self.for_each_extremum_inner(a1, a2, cb);
    }

    pub fn for_each_local_y_extremum_t<F>(&self, cb: &mut F)
    where F: FnMut(S) {
        let rx = self.radii.x;
        let ry = self.radii.y;
        let a1 = Angle::radians(S::atan(ry / (Float::tan(self.x_rotation.radians) * rx)));
        let a2 = Angle::pi() + a1;

        self.for_each_extremum_inner(a1, a2, cb);
    }

    fn for_each_extremum_inner<F>(&self, a1: Angle<S>, a2: Angle<S>, cb: &mut F)
    where F: FnMut(S) {
        let sweep = self.sweep_angle.radians;
        let abs_sweep = S::abs(sweep);
        let sign = S::signum(sweep);

        let mut a1 = (a1 - self.start_angle).positive().radians;
        let mut a2 = (a2 - self.start_angle).positive().radians;
        if a1 * sign > a2 * sign {
            swap(&mut a1, &mut a2);
        }

        let two_pi = S::TWO * S::PI();
        if sweep >= S::ZERO {
            if a1 < abs_sweep {
                cb(a1 / abs_sweep);
            }
            if a2 < abs_sweep {
                cb(a2 / abs_sweep);
            }
        } else  {
            if a1 > two_pi - abs_sweep {
                cb(a1 / abs_sweep);
            }
            if a2 > two_pi - abs_sweep {
                cb(a2 / abs_sweep);
            }
        }
    }

    pub fn bounding_range_x(&self) -> (S, S) {
        let r = self.bounding_rect();
        (r.min_x(), r.max_x())
    }

    pub fn bounding_range_y(&self) -> (S, S) {
        let r = self.bounding_rect();
        (r.min_y(), r.max_y())
    }

    pub fn fast_bounding_range_x(&self) -> (S, S) {
        let r = self.fast_bounding_rect();
        (r.min_x(), r.max_x())
    }

    pub fn fast_bounding_range_y(&self) -> (S, S) {
        let r = self.fast_bounding_rect();
        (r.min_y(), r.max_y())
    }

    pub fn approximate_length(&self, tolerance: S) -> S {
        let mut from = self.from();
        let mut len = S::ZERO;
        self.for_each_flattened(tolerance, &mut|to| {
            len = len + (to - from).length();
            from = to;
        });

        len
    }

    #[inline]
    fn tangent_at_angle(&self, angle: Angle<S>) -> Vector<S> {
        let a = angle.get();
        Rotation2D::new(self.x_rotation).transform_vector(
            vector(-self.radii.x * Float::sin(a), self.radii.y * Float::cos(a))
        )
    }
}

impl<S: Scalar> Into<Arc<S>> for SvgArc<S> {
    fn into(self) -> Arc<S> { self.to_arc() }
}

impl<S: Scalar> SvgArc<S> {
    /// Converts this arc from endpoints to center notation.
    pub fn to_arc(&self) -> Arc<S> { Arc::from_svg_arc(self) }

    /// Per SVG spec, this arc should be rendered as a line_to segment.
    ///
    /// Do not convert an `SvgArc` into an `arc` if this returns true.
    pub fn is_straight_line(&self) -> bool {
        S::abs(self.radii.x) <= S::EPSILON
            || S::abs(self.radii.y) <= S::EPSILON
            || self.from == self.to
    }

    /// Approximates the arc with a sequence of quadratic bézier segments.
    pub fn for_each_quadratic_bezier<F>(&self, cb: &mut F)
    where
        F: FnMut(&QuadraticBezierSegment<S>)
    {
        if self.is_straight_line() {
            cb(&QuadraticBezierSegment{
                from: self.from,
                ctrl: self.from,
                to: self.to,
            });
            return;
        }

        Arc::from_svg_arc(self).for_each_quadratic_bezier(cb);
    }

    /// Approximates the arc with a sequence of cubic bézier segments.
    pub fn for_each_cubic_bezier<F>(&self, cb: &mut F)
    where
        F: FnMut(&CubicBezierSegment<S>)
    {
        if self.is_straight_line() {
            cb(&CubicBezierSegment {
                from: self.from,
                ctrl1: self.from,
                ctrl2: self.to,
                to: self.to,
            });
            return;
        }

        Arc::from_svg_arc(self).for_each_cubic_bezier(cb);
    }

    /// Approximates the arc with a sequence of line segments.
    pub fn for_each_flattened<F: FnMut(Point<S>)>(&self, tolerance: S, cb: &mut F) {
        if self.is_straight_line() {
            cb(self.to);
            return;
        }

        Arc::from_svg_arc(self).for_each_flattened(tolerance, cb);
    }
}

/// Flag parameters for arcs as described by the SVG specification.
#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
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

fn arc_to_quadratic_beziers<S, F>(
    arc: &Arc<S>,
    callback: &mut F,
)
where
    S: Scalar,
    F: FnMut(&QuadraticBezierSegment<S>)
{
    let sign = arc.sweep_angle.get().signum();
    let sweep_angle = S::abs(arc.sweep_angle.get()).min(S::PI() * S::TWO);

    let n_steps = S::ceil(sweep_angle / S::FRAC_PI_4());
    let step = Angle::radians(sweep_angle / n_steps * sign);

    for i in 0..cast::<S, i32>(n_steps).unwrap() {
        let a1 = arc.start_angle + step * cast(i).unwrap();
        let a2 = arc.start_angle + step * cast(i+1).unwrap();

        let v1 = sample_ellipse(arc.radii, arc.x_rotation, a1).to_vector();
        let v2 = sample_ellipse(arc.radii, arc.x_rotation, a2).to_vector();
        let from = arc.center + v1;
        let to = arc.center + v2;
        let l1 = Line { point: from, vector: arc.tangent_at_angle(a1) };
        let l2 = Line { point: to, vector: arc.tangent_at_angle(a2) };
        let ctrl = l2.intersection(&l1).unwrap_or(from);

        callback(&QuadraticBezierSegment { from , ctrl, to });
    }
}

fn arc_to_cubic_beziers<S, F>(
    arc: &Arc<S>,
    callback: &mut F,
)
where
    S: Scalar,
    F: FnMut(&CubicBezierSegment<S>)
{
    let sign = arc.sweep_angle.get().signum();
    let sweep_angle = S::abs(arc.sweep_angle.get()).min(S::PI() * S::TWO);

    let n_steps = S::ceil(sweep_angle / S::FRAC_PI_2());
    let step = Angle::radians(sweep_angle / n_steps * sign);

    for i in 0..cast::<S, i32>(n_steps).unwrap() {
        let a1 = arc.start_angle + step * cast(i).unwrap();
        let a2 = arc.start_angle + step * cast(i+1).unwrap();

        let v1 = sample_ellipse(arc.radii, arc.x_rotation, a1).to_vector();
        let v2 = sample_ellipse(arc.radii, arc.x_rotation, a2).to_vector();
        let from = arc.center + v1;
        let to = arc.center + v2;

        // From http://www.spaceroots.org/documents/ellipse/elliptical-arc.pdf
        // Note that the parameterization used by Arc (see sample_ellipse for
        // example) is the same as the eta-parameterization used at the link.
        let delta_a = a2 - a1;
        let tan_da = Float::tan(delta_a.get() * S::HALF);
        let alpha_sqrt = S::sqrt(S::FOUR + S::THREE * tan_da * tan_da);
        let alpha = Float::sin(delta_a.get()) * (alpha_sqrt - S::ONE) / S::THREE;
        let ctrl1 = from + arc.tangent_at_angle(a1) * alpha;
        let ctrl2 = to - arc.tangent_at_angle(a2) * alpha;

        callback(&CubicBezierSegment { from , ctrl1, ctrl2, to });
    }
}

fn sample_ellipse<S: Scalar>(radii: Vector<S>, x_rotation: Angle<S>, angle: Angle<S>) -> Point<S> {
    Rotation2D::new(x_rotation).transform_point(
        point(radii.x * Float::cos(angle.get()), radii.y * Float::sin(angle.get()))
    )
}

impl<S: Scalar> Segment for Arc<S> {
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

impl<S: Scalar> BoundingRect for Arc<S> {
    type Scalar = S;
    fn bounding_rect(&self) -> Rect<S> { self.bounding_rect() }
    fn fast_bounding_rect(&self) -> Rect<S> { self.fast_bounding_rect() }
    fn bounding_range_x(&self) -> (S, S) { self.bounding_range_x() }
    fn bounding_range_y(&self) -> (S, S) { self.bounding_range_y() }
    fn fast_bounding_range_x(&self) -> (S, S) { self.fast_bounding_range_x() }
    fn fast_bounding_range_y(&self) -> (S, S) { self.fast_bounding_range_y() }
}

impl<S: Scalar> FlatteningStep for Arc<S> {
    fn flattening_step(&self, tolerance: S) -> S {
        self.flattening_step(tolerance)
    }
}

#[test]
fn test_from_svg_arc() {
    use euclid::approxeq::ApproxEq;
    use crate::math::vector;

    let flags = ArcFlags { large_arc: false, sweep: false };

    test_endpoints(&SvgArc {
        from: point(0.0, -10.0),
        to: point(10.0, 0.0),
        radii: vector(10.0, 10.0),
        x_rotation: Angle::radians(0.0),
        flags,
    });

    test_endpoints(&SvgArc {
        from: point(0.0, -10.0),
        to: point(10.0, 0.0),
        radii: vector(100.0, 10.0),
        x_rotation: Angle::radians(0.0),
        flags,
    });

    test_endpoints(&SvgArc {
        from: point(0.0, -10.0),
        to: point(10.0, 0.0),
        radii: vector(10.0, 30.0),
        x_rotation: Angle::radians(1.0),
        flags,
    });

    test_endpoints(&SvgArc {
        from: point(5.0, -10.0),
        to: point(5.0, 5.0),
        radii: vector(10.0, 30.0),
        x_rotation: Angle::radians(-2.0),
        flags,
    });

    // This arc has invalid radii (too small to connect the two endpoints),
    // but the conversion needs to be able to cope with that.
    test_endpoints(&SvgArc {
        from: point(0.0, 0.0),
        to: point(80.0, 60.0),
        radii: vector(40.0, 40.0),
        x_rotation: Angle::radians(0.0),
        flags,
    });

    fn test_endpoints(svg_arc: &SvgArc<f64>) {
        do_test_endpoints(&SvgArc {
            flags: ArcFlags {
                large_arc: false,
                sweep: false,
            },
            ..svg_arc.clone()
        });

        do_test_endpoints(&SvgArc {
            flags: ArcFlags {
                large_arc: true,
                sweep: false,
            },
            ..svg_arc.clone()
        });

        do_test_endpoints(&SvgArc {
            flags: ArcFlags {
                large_arc: false,
                sweep: true,
            },
            ..svg_arc.clone()
        });

        do_test_endpoints(&SvgArc {
            flags: ArcFlags {
                large_arc: true,
                sweep: true,
            },
            ..svg_arc.clone()
        });
    }

    fn do_test_endpoints(svg_arc: &SvgArc<f64>) {
        let eps = point(0.01, 0.01);
        let arc = svg_arc.to_arc();
        assert!(arc.from().approx_eq_eps(&svg_arc.from, &eps),
            "unexpected arc.from: {:?} == {:?}, flags: {:?}",
            arc.from(), svg_arc.from, svg_arc.flags,
        );
        assert!(arc.to().approx_eq_eps(&svg_arc.to, &eps),
            "unexpected arc.from: {:?} == {:?}, flags: {:?}",
            arc.to(), svg_arc.to, svg_arc.flags,
        );
    }
}

#[test]
fn test_to_quadratics_and_cubics() {
    use euclid::approxeq::ApproxEq;

    fn do_test(arc: &Arc<f32>, expected_quadratic_count: u32, expected_cubic_count: u32) {
        let last = arc.to();
        {
            let mut prev = arc.from();
            let mut count = 0;
            arc.for_each_quadratic_bezier(&mut |c| {
                assert!(c.from.approx_eq(&prev));
                prev = c.to;
                count += 1;
            });
            assert!(prev.approx_eq(&last));
            assert_eq!(count, expected_quadratic_count);
        }
        {
            let mut prev = arc.from();
            let mut count = 0;
            arc.for_each_cubic_bezier(&mut|c| {
                assert!(c.from.approx_eq(&prev));
                prev = c.to;
                count += 1;
            });
            assert!(prev.approx_eq(&last));
            assert_eq!(count, expected_cubic_count);
        }
    }

    do_test(
        &Arc {
            center: point(2.0, 3.0),
            radii: vector(10.0, 3.0),
            start_angle: Angle::radians(0.1),
            sweep_angle: Angle::radians(3.0),
            x_rotation: Angle::radians(0.5),
        },
        4,
        2
    );

    do_test(
        &Arc {
            center: point(4.0, 5.0),
            radii: vector(3.0, 5.0),
            start_angle: Angle::radians(2.0),
            sweep_angle: Angle::radians(-3.0),
            x_rotation: Angle::radians(1.3),
        },
        4,
        2
    );

    do_test(
        &Arc {
            center: point(0.0, 0.0),
            radii: vector(100.0, 0.01),
            start_angle: Angle::radians(-1.0),
            sweep_angle: Angle::radians(0.1),
            x_rotation: Angle::radians(0.3),
        },
        1,
        1
    );

    do_test(
        &Arc {
            center: point(0.0, 0.0),
            radii: vector(1.0, 1.0),
            start_angle: Angle::radians(3.0),
            sweep_angle: Angle::radians(-0.1),
            x_rotation: Angle::radians(-0.3),
        },
        1,
        1
    );
}

#[test]
fn test_bounding_rect() {
    use euclid::approxeq::ApproxEq;
    use crate::math::rect;

    fn approx_eq(r1: Rect<f32>, r2: Rect<f32>) -> bool {
        if !r1.min_x().approx_eq(&r2.min_x()) ||
           !r1.max_x().approx_eq(&r2.max_x()) ||
           !r1.min_y().approx_eq(&r2.min_y()) ||
           !r1.max_y().approx_eq(&r2.max_y()) {
            println!("\n   left: {:?}\n   right: {:?}", r1, r2);
            return false;
        }

        true
    }

    let r = Arc {
        center: point(0.0, 0.0),
        radii: vector(1.0, 1.0),
        start_angle: Angle::radians(0.0),
        sweep_angle: Angle::pi(),
        x_rotation: Angle::zero(),
    }.bounding_rect();
    assert!(approx_eq(r, rect(-1.0, 0.0, 2.0, 1.0)));

    let r = Arc {
        center: point(0.0, 0.0),
        radii: vector(1.0, 1.0),
        start_angle: Angle::radians(0.0),
        sweep_angle: Angle::pi(),
        x_rotation: Angle::pi(),
    }.bounding_rect();
    assert!(approx_eq(r, rect(-1.0, -1.0, 2.0, 1.0)));

    let r = Arc {
        center: point(0.0, 0.0),
        radii: vector(2.0, 1.0),
        start_angle: Angle::radians(0.0),
        sweep_angle: Angle::pi(),
        x_rotation: Angle::pi() * 0.5,
    }.bounding_rect();
    assert!(approx_eq(r, rect(-1.0, -2.0, 1.0, 4.0)));

    let r = Arc {
        center: point(1.0, 1.0),
        radii: vector(1.0, 1.0),
        start_angle: Angle::pi(),
        sweep_angle: Angle::pi(),
        x_rotation: -Angle::pi() * 0.25,
    }.bounding_rect();
    assert!(approx_eq(r, rect(0.0, 0.0, 1.707107, 1.707107)));

    let mut angle = Angle::zero();
    for _ in 0..10 {
        println!("angle: {:?}", angle);
        let r = Arc {
            center: point(0.0, 0.0),
            radii: vector(4.0, 4.0),
            start_angle: angle,
            sweep_angle: Angle::pi() * 2.0,
            x_rotation: Angle::pi() * 0.25,
        }.bounding_rect();
        assert!(approx_eq(r, rect(-4.0, -4.0, 8.0, 8.0)));
        angle += Angle::pi() * 2.0 / 10.0;
    }

    let mut angle = Angle::zero();
    for _ in 0..10 {
        println!("angle: {:?}", angle);
        let r = Arc {
            center: point(0.0, 0.0),
            radii: vector(4.0, 4.0),
            start_angle: Angle::zero(),
            sweep_angle: Angle::pi() * 2.0,
            x_rotation: angle,
        }.bounding_rect();
        assert!(approx_eq(r, rect(-4.0, -4.0, 8.0, 8.0)));
        angle += Angle::pi() * 2.0 / 10.0;
    }
}

#[test]
fn negative_flattening_step() {
    // These parameters were running into a precision issue which led the
    // flattening step to never converge towards 1 and cause an infinite loop.

    let arc = Arc {
        center: point(-100.0, -150.0),
        radii: vector(50.0, 50.0),
        start_angle: Angle::radians(0.982944787),
        sweep_angle: Angle::radians(-898.0),
        x_rotation: Angle::zero(),
    };

    arc.for_each_flattened(0.100000001, &mut|_|{});
}
