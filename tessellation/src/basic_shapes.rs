#![allow(dead_code)]

//! Tessellation routines for simple shapes.

use geometry_builder::{GeometryBuilder, Count, VertexId};
use path_stroke::{StrokeTessellator, StrokeBuilder};
use path_fill::{FillOptions, FillTessellator, FillResult};
use math_utils::compute_normal;
use math::*;
use path_builder::BaseBuilder;
use path_iterator::FromPolyline;
use {FillVertex, StrokeVertex, StrokeOptions, Side};
use bezier::{Arc, Radians};

use std::f32::consts::PI;

/// Tessellate a triangle.
pub fn fill_triangle<Output: GeometryBuilder<FillVertex>>(
    v1: Point,
    v2: Point,
    v3: Point,
    output: &mut Output,
) -> Count {
    output.begin_geometry();

    let a = output.add_vertex(
        FillVertex {
            position: v1,
            normal: compute_normal(v1 - v3, v2 - v1),
        }
    );
    let b = output.add_vertex(
        FillVertex {
            position: v2,
            normal: compute_normal(v2 - v1, v3 - v2),
        }
    );
    let c = output.add_vertex(
        FillVertex {
            position: v3,
            normal: compute_normal(v3 - v2, v1 - v3),
        }
    );

    output.add_triangle(a, b, c);

    return output.end_geometry();
}

/// Tessellate the stroke of a triangle.
pub fn stroke_triangle<Output: GeometryBuilder<StrokeVertex>>(
    v1: Point,
    v2: Point,
    v3: Point,
    options: &StrokeOptions,
    output: &mut Output,
) -> Count {
    stroke_polyline([v1, v2, v3].iter().cloned(), true, options, output)
}


/// Tessellate a quad.
pub fn fill_quad<Output: GeometryBuilder<FillVertex>>(
    v1: Point,
    v2: Point,
    v3: Point,
    v4: Point,
    output: &mut Output,
) -> Count {
    output.begin_geometry();

    let a = output.add_vertex(
        FillVertex {
            position: v1,
            normal: compute_normal(v1 - v4, v2 - v1),
        }
    );
    let b = output.add_vertex(
        FillVertex {
            position: v2,
            normal: compute_normal(v2 - v1, v3 - v2),
        }
    );
    let c = output.add_vertex(
        FillVertex {
            position: v3,
            normal: compute_normal(v3 - v2, v4 - v3),
        }
    );
    let d = output.add_vertex(
        FillVertex {
            position: v4,
            normal: compute_normal(v4 - v3, v1 - v4),
        }
    );
    output.add_triangle(a, b, c);
    output.add_triangle(a, c, d);

    return output.end_geometry();
}

/// Tessellate the stroke of a quad.
pub fn stroke_quad<Output: GeometryBuilder<StrokeVertex>>(
    v1: Point,
    v2: Point,
    v3: Point,
    v4: Point,
    options: &StrokeOptions,
    output: &mut Output,
) -> Count {
    stroke_polyline([v1, v2, v3, v4].iter().cloned(), true, options, output)
}

/// Tessellate an axis-aligned rectangle.
pub fn fill_rectangle<Output: GeometryBuilder<FillVertex>>(
    rect: &Rect,
    output: &mut Output,
) -> Count {
    output.begin_geometry();

    let a = output.add_vertex(
        FillVertex {
            position: rect.origin,
            normal: vec2(-1.0, -1.0),
        }
    );
    let b = output.add_vertex(
        FillVertex {
            position: rect.top_right(),
            normal: vec2(1.0, -1.0),
        }
    );
    let c = output.add_vertex(
        FillVertex {
            position: rect.bottom_right(),
            normal: vec2(1.0, 1.0),
        }
    );
    let d = output.add_vertex(
        FillVertex {
            position: rect.bottom_left(),
            normal: vec2(-1.0, 1.0),
        }
    );
    output.add_triangle(a, b, c);
    output.add_triangle(a, c, d);

    return output.end_geometry();
}

/// Tessellate the stroke for an axis-aligne rectangle.
pub fn stroke_rectangle<Output: GeometryBuilder<StrokeVertex>>(
    rect: &Rect,
    output: &mut Output,
) -> Count {
    output.begin_geometry();

    let a1 = output.add_vertex(
        StrokeVertex {
            position: rect.origin,
            normal: -vec2(-1.0, -1.0),
            advancement: 0.0,
            side: Side::Right,
        }
    );
    let a2 = output.add_vertex(
        StrokeVertex {
            position: rect.origin,
            normal: vec2(-1.0, -1.0),
            advancement: 0.0,
            side: Side::Left,
        }
    );

    let b1 = output.add_vertex(
        StrokeVertex {
            position: rect.top_right(),
            normal: -vec2(1.0, -1.0),
            advancement: 0.0,
            side: Side::Right,
        }
    );
    let b2 = output.add_vertex(
        StrokeVertex {
            position: rect.top_right(),
            normal: vec2(1.0, -1.0),
            advancement: 0.0,
            side: Side::Left,
        }
    );

    let c1 = output.add_vertex(
        StrokeVertex {
            position: rect.bottom_right(),
            normal: -vec2(1.0, 1.0),
            advancement: 0.0,
            side: Side::Right,
        }
    );
    let c2 = output.add_vertex(
        StrokeVertex {
            position: rect.bottom_right(),
            normal: vec2(1.0, 1.0),
            advancement: 0.0,
            side: Side::Left,
        }
    );

    let d1 = output.add_vertex(
        StrokeVertex {
            position: rect.bottom_left(),
            normal: -vec2(1.0, 0.0),
            advancement: 0.0,
            side: Side::Right,
        }
    );
    let d2 = output.add_vertex(
        StrokeVertex {
            position: rect.bottom_left(),
            normal: vec2(1.0, 0.0),
            advancement: 0.0,
            side: Side::Left,
        }
    );

    output.add_triangle(a1, a2, b2);
    output.add_triangle(a2, b2, b1);
    output.add_triangle(b1, b2, c1);
    output.add_triangle(b2, c2, c1);
    output.add_triangle(c1, c2, d1);
    output.add_triangle(c2, d2, d1);
    output.add_triangle(d1, d2, a1);
    output.add_triangle(d2, a2, a1);

    return output.end_geometry();
}

/// The radius of each corner of a rounded rectangle.
pub struct BorderRadii {
    pub top_left: f32,
    pub top_right: f32,
    pub bottom_left: f32,
    pub bottom_right: f32,
}

impl BorderRadii {
    pub fn new(
        top_left: f32,
        top_right: f32,
        bottom_left: f32,
        bottom_right: f32,
    ) -> Self {
        BorderRadii {
            top_left: top_left.abs(),
            top_right: top_right.abs(),
            bottom_left: bottom_left.abs(),
            bottom_right: bottom_right.abs(),
        }
    }

    pub fn new_all_same(radius: f32) -> Self {
        let r = radius.abs();
        BorderRadii {
            top_left: r,
            top_right: r,
            bottom_left: r,
            bottom_right: r,
        }
    }
}

/// Tessellate an axis-aligned rounded rectangle.
pub fn fill_rounded_rectangle<Output: GeometryBuilder<FillVertex>>(
    rect: &Rect,
    radii: &BorderRadii,
    tolerance: f32,
    output: &mut Output,
) -> Count {
    output.begin_geometry();

    let w = rect.size.width;
    let h = rect.size.height;
    let x_min = rect.min_x();
    let y_min = rect.min_y();
    let x_max = rect.max_x();
    let y_max = rect.max_y();
    let min_wh = w.min(h);
    let mut tl = radii.top_left.abs().min(min_wh);
    let mut tr = radii.top_right.abs().min(min_wh);
    let mut bl = radii.bottom_left.abs().min(min_wh);
    let mut br = radii.bottom_right.abs().min(min_wh);

    // clamp border radii if they don't fit in the rectangle.
    if tl + tr > w {
        let x = (tl + tr - w) * 0.5;
        tl -= x;
        tr -= x;
    }
    if bl + br > w {
        let x = (bl + br - w) * 0.5;
        bl -= x;
        br -= x;
    }
    if tr + br > h {
        let x = (tr + br - h) * 0.5;
        tr -= x;
        br -= x;
    }
    if tl + bl > h {
        let x = (tl + bl - h) * 0.5;
        tl -= x;
        bl -= x;
    }

    // top
    let p1 = point(x_min + tl, y_min);
    let p2 = point(x_max - tr, y_min);

    // bottom
    let p6 = point(x_min + bl, y_max);
    let p5 = point(x_max - br, y_max);

    // left
    let p0 = point(x_min, y_min + tl);
    let p7 = point(x_min, y_max - bl);

    // right
    let p3 = point(x_max, y_min + tr);
    let p4 = point(x_max, y_max - br);

    let up = vec2(0.0, -1.0);
    let down = vec2(0.0, 1.0);
    let left = vec2(-1.0, 0.0);
    let right = vec2(1.0, 0.0);


    let v = [
        output.add_vertex(FillVertex { position: p0, normal: left }),
        output.add_vertex(FillVertex { position: p1, normal: up }),
        output.add_vertex(FillVertex { position: p2, normal: up }),
        output.add_vertex(FillVertex { position: p3, normal: right }),
        output.add_vertex(FillVertex { position: p4, normal: right }),
        output.add_vertex(FillVertex { position: p5, normal: down }),
        output.add_vertex(FillVertex { position: p6, normal: down }),
        output.add_vertex(FillVertex { position: p7, normal: left }),
    ];

    output.add_triangle(v[6], v[7], v[0]);
    output.add_triangle(v[6], v[0], v[1]);
    output.add_triangle(v[6], v[1], v[5]);
    output.add_triangle(v[5], v[1], v[2]);
    output.add_triangle(v[5], v[2], v[4]);
    output.add_triangle(v[4], v[2], v[3]);

    let radii = [tl, tr, br, bl];
    let angles = [
        (PI, 1.5 * PI),
        (1.5* PI, 2.0 * PI),
        (0.0, PI * 0.5),
        (PI * 0.5, PI),
    ];

    let centers = [
        point(p1.x, p0.y),
        point(p2.x, p3.y),
        point(p5.x, p4.y),
        point(p6.x, p7.y),
    ];

    for i in 0..4 {
        let radius = radii[i];
        if radius > 0.0 {

            let arc_len = 0.5 * PI * radius;

            let step = circle_flattening_step(radius, tolerance);
            let num_segments = (arc_len / step).ceil();

            let num_recursions = num_segments.log2() as u32;

            fill_border_radius(
                centers[i],
                angles[i],
                radius,
                v[i*2],
                v[i*2 + 1],
                num_recursions,
                output,
            );
        }
    }

    return output.end_geometry();
}

// recursively tessellate the rounded corners.
fn fill_border_radius<Output: GeometryBuilder<FillVertex>>(
    center: Point,
    angle: (f32, f32),
    radius: f32,
    va: VertexId,
    vb: VertexId,
    num_recursions: u32,
    output: &mut Output
) {
    if num_recursions == 0 {
        return;
    }

    let mid_angle = (angle.0 + angle.1) * 0.5;

    let normal = vec2(mid_angle.cos(), mid_angle.sin());
    let pos = center + normal * radius;

    let vertex = output.add_vertex(FillVertex {
        position: pos,
        normal: normal,
    });

    output.add_triangle(va, vertex, vb);

    fill_border_radius(
        center,
        (angle.0, mid_angle),
        radius,
        va,
        vertex,
        num_recursions - 1,
        output
    );
    fill_border_radius(
        center,
        (mid_angle, angle.1),
        radius,
        vertex,
        vb,
        num_recursions - 1,
        output
    );
}

/// Tessellate the stroke of an axis-aligned rounded rectangle.
pub fn stroke_rounded_rectangle<Output: GeometryBuilder<StrokeVertex>>(
    rect: &Rect,
    radii: &BorderRadii,
    options: &StrokeOptions,
    output: &mut Output,
) -> Count {
    output.begin_geometry();

    let w = rect.size.width;
    let h = rect.size.height;
    let x_min = rect.min_x();
    let y_min = rect.min_y();
    let x_max = rect.max_x();
    let y_max = rect.max_y();
    let min_wh = w.min(h);
    let mut tl = radii.top_left.abs().min(min_wh);
    let mut tr = radii.top_right.abs().min(min_wh);
    let mut bl = radii.bottom_left.abs().min(min_wh);
    let mut br = radii.bottom_right.abs().min(min_wh);

    // clamp border radii if they don't fit in the rectangle.
    if tl + tr > w {
        let x = (tl + tr - w) * 0.5;
        tl -= x;
        tr -= x;
    }
    if bl + br > w {
        let x = (bl + br - w) * 0.5;
        bl -= x;
        br -= x;
    }
    if tr + br > h {
        let x = (tr + br - h) * 0.5;
        tr -= x;
        br -= x;
    }
    if tl + bl > h {
        let x = (tl + bl - h) * 0.5;
        tl -= x;
        bl -= x;
    }

    // top
    let p1 = point(x_min + tl, y_min);
    let p2 = point(x_max - tr, y_min);

    // bottom
    let p6 = point(x_min + bl, y_max);
    let p5 = point(x_max - br, y_max);

    // left
    let p0 = point(x_min, y_min + tl);
    let p7 = point(x_min, y_max - bl);

    // right
    let p3 = point(x_max, y_min + tr);
    let p4 = point(x_max, y_max - br);

    let sides = &[
        [p1, p2],
        [p5, p6],
        [p3, p4],
        [p0, p7],
    ];

    let radii = [tl, tr, br, bl];
    let angles = [
        (PI, 1.5 * PI),
        (1.5* PI, 2.0 * PI),
        (0.0, PI * 0.5),
        (PI * 0.5, PI),
    ];

    let centers = [
        point(p1.x, p0.y),
        point(p2.x, p3.y),
        point(p5.x, p4.y),
        point(p6.x, p7.y),
    ];

    let mut nums = radii.iter().map(|&radius| {
        if radius > 0.0 {
            let arc_len = 0.5 * PI * radius;
            let step = circle_flattening_step(radius, options.tolerance);
            (arc_len / step).ceil() as u32  - 1
        } else {
            0
        }
    });

    { // output borrow scope start
        let mut builder = StrokeBuilder::new(options, output);
        builder.move_to(p7);
        for i in 0..4 {
            stroke_border_radius(
                centers[i],
                angles[i],
                radii[i],
                nums.next().unwrap(),
                &mut builder,
            );
            builder.line_to(sides[i][0]);
            builder.line_to(sides[i][1]);
        }
    } // output borrow scope end

    return output.end_geometry();
}

/// Tessellate a circle.
pub fn fill_circle<Output: GeometryBuilder<FillVertex>>(
    center: Point,
    radius: f32,
    tolerance: f32,
    output: &mut Output,
) -> Count {
    output.begin_geometry();

    let radius = radius.abs();
    if radius == 0.0 {
        return output.end_geometry();
    }

    let up = vec2(0.0, -1.0);
    let down = vec2(0.0, 1.0);
    let left = vec2(-1.0, 0.0);
    let right = vec2(1.0, 0.0);

    let v = [
        output.add_vertex(FillVertex {
            position: center + (left * radius),
            normal: left
        }),
        output.add_vertex(FillVertex {
            position: center + (up * radius),
            normal: up
        }),
        output.add_vertex(FillVertex {
            position: center + (right * radius),
            normal: right
        }),
        output.add_vertex(FillVertex {
            position: center + (down * radius),
            normal: down
        }),
    ];

    output.add_triangle(v[0], v[1], v[3]);
    output.add_triangle(v[1], v[2], v[3]);

    let angles = [
        (PI, 1.5 * PI),
        (1.5* PI, 2.0 * PI),
        (0.0, PI * 0.5),
        (PI * 0.5, PI),
    ];

    let arc_len = 0.5 * PI * radius;
    let step = circle_flattening_step(radius, tolerance);
    let num_segments = (arc_len / step).ceil();
    let num_recursions = num_segments.log2() as u32;

    for i in 0..4 {
        fill_border_radius(
            center,
            angles[i],
            radius,
            v[i],
            v[(i + 1) % 4],
            num_recursions,
            output,
        );
    }

    return output.end_geometry();
}

/// Tessellate the stroke of a circle.
pub fn stroke_circle<Output>(
    center: Point,
    radius: f32,
    options: &StrokeOptions,
    output: &mut Output
) -> Count
    where Output: GeometryBuilder<StrokeVertex>
{
    output.begin_geometry();

    let radius = radius.abs();
    if radius == 0.0 {
        return output.end_geometry();
    }

    let angle = (0.0, 2.0 * PI);
    let starting_point = center + vec2(1.0, 0.0) * radius;

    let arc_len = 2.0 * PI * radius;
    let step = circle_flattening_step(radius, options.tolerance);
    let num_points = (arc_len / step).ceil() as u32 - 1;

    { // output borrow scope start
        let mut builder = StrokeBuilder::new(options, output);
        builder.move_to(starting_point);
        stroke_border_radius(
            center,
            angle,
            radius,
            num_points,
            &mut builder,
        );
        builder.line_to(starting_point);
    } // output borrow scope end
    return output.end_geometry();
}

// tessellate the stroke of rounded corners using the inner points.
// assumming the builder started with move_to().
fn stroke_border_radius<Output: GeometryBuilder<StrokeVertex>>(
    center: Point,
    angle: (f32, f32),
    radius: f32,
    num_points: u32,
    builder: &mut StrokeBuilder<Output>,
) {
    let angle_size = (angle.0 - angle.1).abs();
    let starting_angle = angle.0.min(angle.1);

    let points = (1..num_points + 1).map(move |i| {
        let new_angle = i as f32 * (angle_size) / (num_points + 1) as f32 + starting_angle;
        let normal =
        vec2(new_angle.cos(),
        new_angle.sin());
        center + normal * radius
    });

    for point in points {
        builder.line_to(point)
    };

}

pub fn fill_ellipse<Output: GeometryBuilder<FillVertex>>(
    center: Point,
    radii: Vec2,
    x_rotation: Radians,
    tolerance: f32,
    output: &mut Output,
) -> Count {
    // TODO: This is far from optimal compared to the circle tessellation, but it
    // correctly takes the tolerance threshold into account which is harder to do
    // than with circles.

    let arc = Arc {
        center,
        radii,
        x_rotation,
        start_angle: Radians::new(0.0),
        sweep_angle: Radians::new(2.0 * PI-0.01),
    };

    use path_builder::{PathBuilder, FlatteningBuilder};
    use path_fill::EventsBuilder;

    let mut path = FlatteningBuilder::new(EventsBuilder::new(), tolerance).with_svg();

    path.move_to(arc.sample(0.0));
    arc.to_quadratic_beziers(&mut|ctrl, to| {
        path.quadratic_bezier_to(ctrl, to);
    });
    path.close();

    let events = path.build();

    return FillTessellator::new().tessellate_events(
        &events,
        &FillOptions::tolerance(tolerance).assume_no_intersections(),
        output,
    ).unwrap();
}

pub fn stroke_ellipse<Output: GeometryBuilder<StrokeVertex>>(
    center: Point,
    radii: Vec2,
    x_rotation: Radians,
    options: &StrokeOptions,
    output: &mut Output,
) -> Count {
    // TODO: This is far from optimal compared to the circle tessellation, but it
    // correctly takes the tolerance threshold into account which is harder to do
    // than with circles.

    let arc = Arc {
        center,
        radii,
        x_rotation,
        start_angle: Radians::new(0.0),
        sweep_angle: Radians::new(2.0 * PI-0.01),
    };

    use path_builder::{PathBuilder, FlatteningBuilder};
    use path_fill::EventsBuilder;

    output.begin_geometry();
    {
        let mut path = FlatteningBuilder::new(StrokeBuilder::new(options, output), options.tolerance).with_svg();

        path.move_to(arc.sample(0.0));
        arc.to_quadratic_beziers(&mut|ctrl, to| {
            path.quadratic_bezier_to(ctrl, to);
        });
        path.close();

        let _ = path.build();
    }

    return output.end_geometry();
}

/// Tessellate a convex shape that is discribed by an iterator of points.
///
/// The shape is assumed to be convex, calling this function with a concave
/// shape may produce incorrect results.
pub fn fill_convex_polyline<Iter, Output>(mut it: Iter, output: &mut Output) -> Count
where
    Iter: Iterator<Item = Point> + Clone,
    Output: GeometryBuilder<FillVertex>,
{
    // We insert 2nd point on line first in order to have the neighbours for normal calculation.
    let mut it1 = it.clone().cycle().skip(1);
    let mut it2 = it.clone().cycle().skip(2);

    output.begin_geometry();

    if let (Some(a1), Some(a2), Some(a3), Some(b2), Some(b3), Some(b4)) = (
        it.next(),it1.next(), it2.next(), it.next(), it1.next(), it2.next()
    ) {
        let mut a = output.add_vertex(
            FillVertex {
                position: a2,
                normal: compute_normal(a2 - a1, a3 - a2),
            }
        );
        let mut b = output.add_vertex(
            FillVertex {
                position: b3,
                normal: compute_normal(b3 - b2, b4 - b3),
            }
        );

        while let (Some(p1), Some(p2), Some(p3)) = (it.next(), it1.next(), it2.next()) {
            let c = output.add_vertex(
                FillVertex {
                    position: p2,
                    normal: compute_normal(p2 - p1, p3 - p2),
                }
            );

            output.add_triangle(a, b, c);

            a = b;
            b = c;
        }
    }

    return output.end_geometry();
}

/// Tessellate the stroke of a shape that is discribed by an iterator of points.
///
/// Convenient when tessellating a shape that is represented as a slice `&[Point]`.
pub fn stroke_polyline<Iter, Output>(
    it: Iter,
    is_closed: bool,
    options: &StrokeOptions,
    output: &mut Output
) -> Count
where
    Iter: Iterator<Item = Point>,
    Output: GeometryBuilder<StrokeVertex>,
{
    let mut tess = StrokeTessellator::new();

    return tess.tessellate_flattened_path(FromPolyline::new(is_closed, it), options, output);
}

/// Tessellate an arbitray shape that is discribed by an iterator of points.
pub fn fill_polyline<Iter, Output>(
    polyline: Iter,
    tessellator: &mut FillTessellator,
    options: &FillOptions,
    output: &mut Output
) -> FillResult
where
    Iter: Iterator<Item = Point>,
    Output: GeometryBuilder<FillVertex>,
{
    tessellator.tessellate_flattened_path(
        FromPolyline::closed(polyline),
        options,
        output
    )
}

// Returns the maximum length of individual line segments when approximating a
// circle.
//
// From pythagora's theorem:
// r² = (d/2)² + (r - t)²
// r² = d²/4 + r² + t² - 2 * e * r
// d² = 4 * (2 * t * r - t²)
// d = 2 * sqrt(2 * t * r - t²)
//
// With:
//  r: the radius
//  t: the tolerance threshold
//  d: the line segment length
pub(crate) fn circle_flattening_step(radius:f32, tolerance: f32) -> f32 {
    2.0 * (2.0 * tolerance * radius - tolerance * tolerance).sqrt()
}

