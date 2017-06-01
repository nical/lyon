//! Elliptic arc related maths and tools.

use std::f32::*;

use core::ArcFlags;
use core::math_utils::*;
use core::math::*;
use PathBuilder;

/// Build an svg arc by approximating it with cubic bezier curves.
///
/// Angles are expressed in radians.
pub fn arc_to_cubic_beziers<Builder: PathBuilder>(
    from: Point,
    to: Point,
    radii: Vec2,
    x_rotation: Radians<f32>,
    flags: ArcFlags,
    builder: &mut Builder,
) {
    if radii.x == 0.0 && radii.y == 0.0 {
        builder.line_to(to);
        return;
    }

    let x_axis_rotation = x_rotation.get() % (2.0 * consts::PI);

    // Middle point between start and end point
    let dx = (from.x - to.x) / 2.0;
    let dy = (from.y - to.y) / 2.0;
    let transformed_point = point(
        (x_axis_rotation.cos() * dx + x_axis_rotation.sin() * dy).round(),
        (-x_axis_rotation.sin() * dx + x_axis_rotation.cos() * dy).round(),
    );

    let scaled_radii = radii_to_scale(radii, transformed_point);
    let transformed_center = find_center(scaled_radii, transformed_point, flags);

    // Start, end and sweep angles
    let start_vector = ellipse_center_to_point(
        transformed_center,
        transformed_point,
        scaled_radii
    ).to_vector();
    let mut start_angle = angle_between(vec2(1.0, 0.0), start_vector);

    let end_vector = ellipse_center_to_point(
        transformed_center,
        point(-transformed_point.x, -transformed_point.y),
        scaled_radii,
    ).to_vector();
    let mut end_angle = angle_between(vec2(1.0, 0.0), end_vector);

    let mut sweep_angle = end_angle - start_angle;

    // Affect the flags value to get the right arc among the 4 possible
    if !flags.sweep && sweep_angle > 0.0 {
        sweep_angle = sweep_angle - 2.0 * consts::PI;
    } else if flags.sweep && sweep_angle < 0.0 {
        sweep_angle = sweep_angle + 2.0 * consts::PI;
    }
    sweep_angle %= 2.0 * consts::PI;

    // Break down the arc into smaller ones of maximum PI/2 angle from point to point
    while sweep_angle.abs() > consts::FRAC_PI_2 {
        // compute crossing-points
        end_angle = start_angle + sweep_angle.signum() * consts::FRAC_PI_2;

        let mut crossing_point =
            ellipse_point_from_angle(transformed_center, scaled_radii, end_angle);

        crossing_point = point(
            x_axis_rotation.cos() * crossing_point.x - x_axis_rotation.sin() * crossing_point.y +
                (from.x + to.x) / 2.0,
            x_axis_rotation.sin() * crossing_point.x + x_axis_rotation.cos() * crossing_point.y +
                (from.y + to.y) / 2.0,
        );

        sub_arc_to_cubic_beziers(
            builder.current_position(),
            crossing_point,
            start_angle,
            sweep_angle.signum() * consts::FRAC_PI_2,
            scaled_radii,
            x_axis_rotation,
            builder,
        );

        if sweep_angle > 0.0 {
            sweep_angle -= consts::FRAC_PI_2;
            start_angle += consts::FRAC_PI_2;
        } else {
            sweep_angle += consts::FRAC_PI_2;
            start_angle -= consts::FRAC_PI_2;
        }
    }

    sub_arc_to_cubic_beziers(
        builder.current_position(),
        to,
        start_angle,
        sweep_angle,
        scaled_radii,
        x_axis_rotation,
        builder,
    );
}

/// Draw an elliptical arc between two points with a sweep_angle <= Pi/2
/// current_position and to are points of the ellipse when aligned with the origin axis !!
/// which means that they are the rotation of the original starting and ending point
/// x_rotation_radian is in radian
fn sub_arc_to_cubic_beziers<Builder: PathBuilder>(
    from: Point,
    to: Point,
    start_angle: f32,
    sweep_angle: f32,
    radii: Vec2,
    x_rotation_radian: f32,
    builder: &mut Builder,
) {
    let alpha = sweep_angle.sin() *
        (((4.0 + 3.0 * (sweep_angle / 2.0).tan().powi(2)).sqrt() - 1.0) / 3.0);
    let end_angle = start_angle + sweep_angle;

    let ctrl_point_1 = point(
        (from.x + alpha * (-radii.x * x_rotation_radian.cos() * start_angle.sin() -
            radii.y * x_rotation_radian.sin() * start_angle.cos())).round(),
        (from.y + alpha * (-radii.x * x_rotation_radian.sin() * start_angle.sin() +
            radii.y * x_rotation_radian.cos() * start_angle.cos())).round(),
    );

    let ctrl_point_2 = point(
        (to.x - alpha * (-radii.x * x_rotation_radian.cos() * end_angle.sin() -
            radii.y * x_rotation_radian.sin() * end_angle.cos())).round(),
        (to.y - alpha * (-radii.x * x_rotation_radian.sin() * end_angle.sin() +
            radii.y * x_rotation_radian.cos() * end_angle.cos())).round(),
    );

    builder.cubic_bezier_to(ctrl_point_1, ctrl_point_2, to);
}

fn radii_to_scale(radii: Vec2, point: Point) -> Vec2 {
    let mut radii_to_scale = (point.x * point.x) / (radii.x * radii.x) +
        (point.y * point.y) / (radii.y * radii.y);
    if radii_to_scale > 1.0 {
        radii_to_scale = radii_to_scale.sqrt();
    } else {
        radii_to_scale = 1.0;
    }

    return vec2(radii_to_scale * radii.x.abs(), radii_to_scale * radii.y.abs());
}

fn find_center(radii: Vec2, p: Point, flags: ArcFlags) -> Point {
    let center_num = radii.x * radii.x * radii.y * radii.y - radii.x * radii.x * p.y * p.y -
        radii.y * radii.y * p.x * p.x;

    let center_denom = radii.x * radii.x * p.y * p.y +
        radii.y * radii.y * p.x * p.x;

    let mut center_coef = center_num / center_denom;
    if center_coef < 0.0 {
        center_coef = 0.0;
    }

    if flags.large_arc == flags.sweep {
        center_coef = -center_coef.sqrt();
    } else {
        center_coef = center_coef.sqrt();
    }

    return point(
        center_coef * radii.x * p.y / radii.y,
        center_coef * -radii.y * p.x / radii.x,
    );
}
