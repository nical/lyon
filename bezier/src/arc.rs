//! Elliptic arc related maths and tools.

use std::f32::*;
use std::ops::Rem;

use {Point, point2, Vec2, vec2, Transform2D, Radians, Line};
use utils::{tangent, directed_angle};

pub struct SvgArc {
    pub from: Point,
    pub to: Point,
    pub radii: Vec2,
    pub x_rotation: Radians,
    pub flags: ArcFlags,
}

pub struct Arc {
    pub center: Point,
    pub radii: Vec2,
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
        let center = point2(
            cos_phi * transformed_cx - sin_phi * transformed_cy + hs_x,
            sin_phi * transformed_cx + cos_phi * transformed_cy + hs_y
        );

        let a = vec2(
            (p.x - transformed_cx) / rx,
            (p.y - transformed_cy) / ry,
        );
        // TODO
        let b = -vec2(
            (-p.x - transformed_cx) / rx,
            (-p.y - transformed_cy) / ry,
        );

        let start_angle = Radians::new(directed_angle(vec2(1.0, 0.0), a));

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

    pub fn to_quadratic_beziers<F: FnMut(Point, Point)>(&self, cb: &mut F) {
        arc_to_to_quadratic_beziers(self, cb);
    }

    pub fn sample(&self, t: f32) -> Point {
        let angle = Radians::new(self.sweep_angle.get() * t);
        self.center + sample_ellipse(self.radii, self.x_rotation, angle).to_vector()
    }
}

impl SvgArc {
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
    let sweep_angle = arc.sweep_angle.get().rem(consts::PI * 2.0);

    let n_steps = (sweep_angle.abs() / consts::FRAC_PI_4).ceil();
    let step = sweep_angle / n_steps;

    //println!(" -- arc_to_to_quadratic_beziers {:?} {:?} {:?} {:?} {:?}", center, start_angle, sweep_angle, radii, x_rotation);
    //println!(" -- n steps: {:?}, step: {:?}", n_steps, step);

    for i in 0..(n_steps as i32) {
        let a1 = arc.start_angle.get() + step * (i as f32);
        let a2 = arc.start_angle.get() + step * ((i+1) as f32);

        let v1 = sample_ellipse(arc.radii, arc.x_rotation, Radians::new(a1)).to_vector();
        let v2 = sample_ellipse(arc.radii, arc.x_rotation, Radians::new(a2)).to_vector();
        let p1 = arc.center + v1;
        let p2 = arc.center + v2;
        let l1 = Line { point: p1, vector: tangent(v1) };
        let l2 = Line { point: p2, vector: tangent(v2) };
        let ctrl = l1.intersection(&l2).unwrap();

        call_back(ctrl, p2);
    }
}

fn sample_ellipse(radii: Vec2, x_rotation: Radians, angle: Radians) -> Point {
    Transform2D::create_rotation(x_rotation).transform_point(
        &point2(radii.x * angle.get().cos(), radii.y * angle.get().sin())
    )
}
