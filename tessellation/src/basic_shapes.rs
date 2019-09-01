#![allow(dead_code)]

//! Tessellation routines for simple shapes.
//!
//! #Overview
//!
//! This module contains tessellators for specific shapes that can
//! benefit from having a specialized algorithm rather than using
//! the generic algorithm (for performance purposes or in some cases
//! just for convenience).
//!
//! See also the generic [fill](../struct.FillTessellator.html) and
//! [stroke](../struct.StrokeTessellator.html) tessellators.
//!
//! Some of these algorithms approximate the geometry based on a
//! tolerance threshold which sets the maximum allowed distance
//! between the theoretical curve and its approximation.
//!
//! This tolerance threshold is configured in the
//! [FillOptions](../struct.FillOptions.html) and
//! [StrokeOptions](../struct.StrokeOptions.html) parameters.
//!
//! More explanation about flattening and tolerance in the
//! [lyon_geom crate](https://docs.rs/lyon_geom/#flattening).

use crate::geometry_builder::{GeometryBuilder, GeometryBuilderError, VertexId};
use crate::path_stroke::{StrokeTessellator, StrokeBuilder};
use crate::math_utils::compute_normal;
use crate::geom::math::*;
use crate::geom::Arc;
use crate::path::builder::FlatPathBuilder;
use crate::path::iterator::FromPolyline;
use crate::{FillOptions, FillVertex, StrokeVertex, StrokeOptions, Side};
use crate::{FillTessellator, TessellationResult};

use std::f32::consts::PI;

fn bottom_left(rect: &Rect) -> Point {
    point(rect.min_x(), rect.max_y())
}

fn top_right(rect: &Rect) -> Point {
    point(rect.max_x(), rect.min_y())
}

fn bottom_right(rect: &Rect) -> Point {
    rect.max()
}

/// Tessellate a triangle.
pub fn fill_triangle(
    v1: Point,
    mut v2: Point,
    mut v3: Point,
    _options: &FillOptions,
    output: &mut dyn GeometryBuilder<FillVertex>,
) -> TessellationResult {
    output.begin_geometry();

    // Make sure the winding order is correct.
    if (v1 - v2).cross(v3 - v2) < 0.0 {
        ::std::mem::swap(&mut v2, &mut v3);
    }

    // Tangents
    let t31 = (v1 - v3).normalize();
    let t12 = (v2 - v1).normalize();
    let t23 = (v3 - v2).normalize();

    let a = output.add_vertex(
        FillVertex {
            position: v1,
            normal: compute_normal(t31, t12),
        }
    )?;
    let b = output.add_vertex(
        FillVertex {
            position: v2,
            normal: compute_normal(t12, t23),
        }
    )?;
    let c = output.add_vertex(
        FillVertex {
            position: v3,
            normal: compute_normal(t23, t31),
        }
    )?;

    output.add_triangle(a, b, c);

    Ok(output.end_geometry())
}

/// Tessellate the stroke for a triangle.
pub fn stroke_triangle(
    v1: Point,
    v2: Point,
    v3: Point,
    options: &StrokeOptions,
    output: &mut dyn GeometryBuilder<StrokeVertex>,
) -> TessellationResult {
    stroke_polyline([v1, v2, v3].iter().cloned(), true, options, output)
}


/// Tessellate a quad.
pub fn fill_quad(
    v1: Point,
    mut v2: Point,
    v3: Point,
    mut v4: Point,
    _options: &FillOptions,
    output: &mut dyn GeometryBuilder<FillVertex>,
) -> TessellationResult {
    output.begin_geometry();

    // Make sure the winding order is correct.
    if (v1 - v2).cross(v3 - v2) < 0.0 {
        ::std::mem::swap(&mut v2, &mut v4);
    }

    // Tangents
    let t12 = (v2 - v1).normalize();
    let t23 = (v3 - v2).normalize();
    let t34 = (v4 - v3).normalize();
    let t41 = (v1 - v4).normalize();

    let a = output.add_vertex(
        FillVertex {
            position: v1,
            normal: compute_normal(t41, t12),
        }
    )?;
    let b = output.add_vertex(
        FillVertex {
            position: v2,
            normal: compute_normal(t12, t23),
        }
    )?;
    let c = output.add_vertex(
        FillVertex {
            position: v3,
            normal: compute_normal(t23, t34),
        }
    )?;
    let d = output.add_vertex(
        FillVertex {
            position: v4,
            normal: compute_normal(t34, t41),
        }
    )?;
    output.add_triangle(a, b, c);
    output.add_triangle(a, c, d);

    Ok(output.end_geometry())
}

/// Tessellate the stroke for a quad.
pub fn stroke_quad(
    v1: Point,
    v2: Point,
    v3: Point,
    v4: Point,
    options: &StrokeOptions,
    output: &mut dyn GeometryBuilder<StrokeVertex>,
) -> TessellationResult {
    stroke_polyline([v1, v2, v3, v4].iter().cloned(), true, options, output)
}

/// Tessellate an axis-aligned rectangle.
pub fn fill_rectangle(
    rect: &Rect,
    _options: &FillOptions,
    output: &mut dyn GeometryBuilder<FillVertex>,
) -> TessellationResult {
    output.begin_geometry();

    let a = output.add_vertex(
        FillVertex {
            position: rect.origin,
            normal: vector(-1.0, -1.0),
        }
    )?;
    let b = output.add_vertex(
        FillVertex {
            position: bottom_left(&rect),
            normal: vector(-1.0, 1.0),
        }
    )?;
    let c = output.add_vertex(
        FillVertex {
            position: bottom_right(&rect),
            normal: vector(1.0, 1.0),
        }
    )?;
    let d = output.add_vertex(
        FillVertex {
            position: top_right(&rect),
            normal: vector(1.0, -1.0),
        }
    )?;
    output.add_triangle(a, b, c);
    output.add_triangle(a, c, d);

    Ok(output.end_geometry())
}

/// Tessellate the stroke for an axis-aligned rectangle.
pub fn stroke_rectangle(
    rect: &Rect,
    options: &StrokeOptions,
    output: &mut dyn GeometryBuilder<StrokeVertex>,
) -> TessellationResult {
    let line_width = options.line_width;
    if rect.size.width.abs() < line_width || rect.size.height < line_width {
        return stroke_thin_rectangle(rect, options, output)
    }

    stroke_quad(
        rect.origin,
        top_right(&rect),
        bottom_right(&rect),
        bottom_left(&rect),
        options,
        output
    )
}

// A fall-back that avoids off artifacts with zero-area rectangles as
// well as overlapping triangles if the rectangle is smaller than the
// line width in any dimension.
#[inline(never)]
fn stroke_thin_rectangle(
    rect: &Rect,
    options: &StrokeOptions,
    output: &mut dyn GeometryBuilder<StrokeVertex>,
) -> TessellationResult {
    let rect = if options.apply_line_width {
        let w = options.line_width * 0.5;
        rect.inflate(w, w)
    } else {
        *rect
    };

    output.begin_geometry();

    let a = output.add_vertex(
        StrokeVertex {
            position: rect.origin,
            normal: vector(-1.0, -1.0),
            advancement: 0.0,
            side: Side::Left,
        }
    )?;
    let b = output.add_vertex(
        StrokeVertex {
            position: bottom_left(&rect),
            normal: vector(-1.0, 1.0),
            advancement: 0.0,
            side: Side::Left,
        }
    )?;
    let c = output.add_vertex(
        StrokeVertex {
            position: bottom_right(&rect),
            normal: vector(1.0, 1.0),
            advancement: 1.0,
            side: Side::Right,
        }
    )?;
    let d = output.add_vertex(
        StrokeVertex {
            position: top_right(&rect),
            normal: vector(1.0, -1.0),
            advancement: 1.0,
            side: Side::Right,
        }
    )?;

    output.add_triangle(a, b, c);
    output.add_triangle(a, c, d);

    Ok(output.end_geometry())
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
pub fn fill_rounded_rectangle(
    rect: &Rect,
    radii: &BorderRadii,
    options: &FillOptions,
    output: &mut dyn GeometryBuilder<FillVertex>,
) -> TessellationResult {
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

    // right
    let p3 = point(x_max, y_min + tr);
    let p4 = point(x_max, y_max - br);

    // bottom
    let p6 = point(x_min + bl, y_max);
    let p5 = point(x_max - br, y_max);

    // left
    let p0 = point(x_min, y_min + tl);
    let p7 = point(x_min, y_max - bl);

    let up = vector(0.0, -1.0);
    let down = vector(0.0, 1.0);
    let left = vector(-1.0, 0.0);
    let right = vector(1.0, 0.0);


    let v = [
        output.add_vertex(FillVertex { position: p7, normal: left })?,
        output.add_vertex(FillVertex { position: p6, normal: down })?,
        output.add_vertex(FillVertex { position: p5, normal: down })?,
        output.add_vertex(FillVertex { position: p4, normal: right })?,
        output.add_vertex(FillVertex { position: p3, normal: right })?,
        output.add_vertex(FillVertex { position: p2, normal: up })?,
        output.add_vertex(FillVertex { position: p1, normal: up })?,
        output.add_vertex(FillVertex { position: p0, normal: left })?,
    ];

    output.add_triangle(v[6], v[7], v[0]);
    output.add_triangle(v[6], v[0], v[1]);
    output.add_triangle(v[6], v[1], v[5]);
    output.add_triangle(v[5], v[1], v[2]);
    output.add_triangle(v[5], v[2], v[4]);
    output.add_triangle(v[4], v[2], v[3]);

    let radii = [bl, br, tr, tl];
    let angles = [
        (PI * 0.5, PI),
        (0.0, PI * 0.5),
        (1.5* PI, 2.0 * PI),
        (PI, 1.5 * PI),
    ];

    let centers = [
        point(p6.x, p7.y),
        point(p5.x, p4.y),
        point(p2.x, p3.y),
        point(p1.x, p0.y),
    ];

    for i in 0..4 {
        let radius = radii[i];
        if radius > 0.0 {

            let arc_len = 0.5 * PI * radius;

            let step = circle_flattening_step(radius, options.tolerance);
            let num_segments = (arc_len / step).ceil();

            let num_recursions = num_segments.log2() as u32;

            fill_border_radius(
                centers[i],
                angles[i],
                radius,
                v[i*2 + 1],
                v[i*2],
                num_recursions,
                output,
            )?;
        }
    }

    Ok(output.end_geometry())
}

// recursively tessellate the rounded corners.
fn fill_border_radius(
    center: Point,
    angle: (f32, f32),
    radius: f32,
    va: VertexId,
    vb: VertexId,
    num_recursions: u32,
    output: &mut dyn GeometryBuilder<FillVertex>
) -> Result<(), GeometryBuilderError> {
    if num_recursions == 0 {
        return Ok(());
    }

    let mid_angle = (angle.0 + angle.1) * 0.5;

    let normal = vector(mid_angle.cos(), mid_angle.sin());
    let position = center + normal * radius;

    let vertex = output.add_vertex(FillVertex {
        position,
        normal,
    })?;

    output.add_triangle(vb, vertex, va);

    fill_border_radius(
        center,
        (angle.0, mid_angle),
        radius,
        va,
        vertex,
        num_recursions - 1,
        output
    )?;
    fill_border_radius(
        center,
        (mid_angle, angle.1),
        radius,
        vertex,
        vb,
        num_recursions - 1,
        output
    )
}

/// Tessellate the stroke for an axis-aligned rounded rectangle.
pub fn stroke_rounded_rectangle(
    rect: &Rect,
    radii: &BorderRadii,
    options: &StrokeOptions,
    output: &mut dyn GeometryBuilder<StrokeVertex>,
) -> TessellationResult {
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

    // right
    let p3 = point(x_max, y_min + tr);
    let p4 = point(x_max, y_max - br);

    // bottom
    let p6 = point(x_min + bl, y_max);
    let p5 = point(x_max - br, y_max);

    // left
    let p0 = point(x_min, y_min + tl);
    let p7 = point(x_min, y_max - bl);

    let sides = &[
        [p1, p2],
        [p3, p4],
        [p5, p6],
        [p7, p0],
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
            (arc_len / step).ceil() as u32 - 1
        } else {
            0
        }
    });

    {
        let mut builder = StrokeBuilder::new(options, output);
        builder.move_to(p0);
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
        builder.close();
    }

    Ok(output.end_geometry())
}

/// Tessellate a circle.
pub fn fill_circle(
    center: Point,
    radius: f32,
    options: &FillOptions,
    output: &mut dyn GeometryBuilder<FillVertex>,
) -> TessellationResult {
    output.begin_geometry();

    let radius = radius.abs();
    if radius == 0.0 {
        return Ok(output.end_geometry());
    }

    let up = vector(0.0, -1.0);
    let down = vector(0.0, 1.0);
    let left = vector(-1.0, 0.0);
    let right = vector(1.0, 0.0);

    let v = [
        output.add_vertex(FillVertex {
            position: center + (left * radius),
            normal: left
        })?,
        output.add_vertex(FillVertex {
            position: center + (up * radius),
            normal: up
        })?,
        output.add_vertex(FillVertex {
            position: center + (right * radius),
            normal: right
        })?,
        output.add_vertex(FillVertex {
            position: center + (down * radius),
            normal: down
        })?,
    ];

    output.add_triangle(v[0], v[3], v[1]);
    output.add_triangle(v[1], v[3], v[2]);

    let angles = [
        (PI, 1.5 * PI),
        (1.5* PI, 2.0 * PI),
        (0.0, PI * 0.5),
        (PI * 0.5, PI),
    ];

    let arc_len = 0.5 * PI * radius;
    let step = circle_flattening_step(radius, options.tolerance);
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
        )?;
    }

    Ok(output.end_geometry())
}

/// Tessellate the stroke for a circle.
pub fn stroke_circle(
    center: Point,
    radius: f32,
    options: &StrokeOptions,
    output: &mut dyn GeometryBuilder<StrokeVertex>
) -> TessellationResult {
    output.begin_geometry();

    let radius = radius.abs();
    if radius == 0.0 {
        return Ok(output.end_geometry());
    }

    let angle = (0.0, 2.0 * PI);
    let starting_point = center + vector(1.0, 0.0) * radius;

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
        builder.close();
    } // output borrow scope end

    Ok(output.end_geometry())
}

// tessellate the stroke for rounded corners using the inner points.
// assumming the builder started with move_to().
fn stroke_border_radius(
    center: Point,
    angle: (f32, f32),
    radius: f32,
    num_points: u32,
    builder: &mut StrokeBuilder,
) {
    let angle_size = (angle.0 - angle.1).abs();
    let starting_angle = angle.0.min(angle.1);

    for i in 1..num_points + 1 {
        let new_angle = i as f32 * (angle_size) / (num_points + 1) as f32 + starting_angle;
        let normal = vector(new_angle.cos(), new_angle.sin());

        builder.line_to(center + normal * radius)
    }
}

/// Tessellate an ellipse.
pub fn fill_ellipse(
    center: Point,
    radii: Vector,
    x_rotation: Angle,
    options: &FillOptions,
    output: &mut dyn GeometryBuilder<FillVertex>,
) -> TessellationResult {
    if radii.x == radii.y {
        return Ok(fill_circle(center, radii.x, options, output)?);
    }

    // TODO: This is far from optimal compared to the circle tessellation, but it
    // correctly takes the tolerance threshold into account which is harder to do
    // than with circles.

    let arc = Arc {
        center,
        radii,
        x_rotation,
        start_angle: Angle::radians(0.0),
        sweep_angle: Angle::radians(2.0 * PI-0.01),
    };

    use crate::path::builder::{Build, PathBuilder, FlatteningBuilder};
    use crate::path_fill::EventsBuilder;

    let mut path = FlatteningBuilder::new(
        EventsBuilder::new(),
        options.tolerance
    ).with_svg();

    // TODO don't need to go through quadratic bézier approximation here.
    path.move_to(arc.sample(0.0));
    arc.for_each_quadratic_bezier(&mut|curve| {
        path.quadratic_bezier_to(curve.ctrl, curve.to);
    });
    path.close();

    let events = path.build();

    // TODO: We could avoid checking for intersections, however the way we
    // generate the path is a little silly and because of finite float precision,
    // it will sometimes produce an intersection where the end of ellipse meets
    // the beginning, which confuses the fill tessellator.
    FillTessellator::new().tessellate_events(
        &events,
        &options,
        output,
    )
}

/// Tessellate the stroke for an ellipse.
pub fn stroke_ellipse(
    center: Point,
    radii: Vector,
    x_rotation: Angle,
    options: &StrokeOptions,
    output: &mut dyn GeometryBuilder<StrokeVertex>,
) -> TessellationResult {
    // TODO: This is far from optimal compared to the circle tessellation, but it
    // correctly takes the tolerance threshold into account which is harder to do
    // than with circles.

    let arc = Arc {
        center,
        radii,
        x_rotation,
        start_angle: Angle::radians(0.0),
        sweep_angle: Angle::radians(2.0 * PI-0.01),
    };

    use crate::path::builder::{Build, PathBuilder, FlatteningBuilder};

    output.begin_geometry();
    {
        let mut path = FlatteningBuilder::new(StrokeBuilder::new(options, output), options.tolerance).with_svg();

        path.move_to(arc.sample(0.0));
        arc.for_each_quadratic_bezier(&mut|curve| {
            path.quadratic_bezier_to(curve.ctrl, curve.to);
        });
        path.close();

        path.build()?;
    }

    Ok(output.end_geometry())
}

/// Tessellate a convex shape that is described by an iterator of points.
///
/// The shape is assumed to be convex, calling this function with a concave
/// shape may produce incorrect results.
pub fn fill_convex_polyline<Iter>(
    mut it: Iter,
    _options: &FillOptions,
    output: &mut dyn GeometryBuilder<FillVertex>
) -> TessellationResult
where
    Iter: Iterator<Item = Point> + Clone,
{
    // We insert 2nd point on line first in order to have the neighbors for normal calculation.
    let mut it1 = it.clone().cycle().skip(1);
    let mut it2 = it.clone().cycle().skip(2);

    output.begin_geometry();

    if let (Some(a1), Some(a2), Some(a3), Some(b2), Some(b3), Some(b4)) = (
        it.next(), it1.next(), it2.next(), it.next(), it1.next(), it2.next()
    ) {
        let a = output.add_vertex(
            FillVertex {
                position: a2,
                normal: compute_normal(a2 - a1, a3 - a2),
            }
        )?;
        let mut b = output.add_vertex(
            FillVertex {
                position: b3,
                normal: compute_normal(b3 - b2, b4 - b3),
            }
        )?;

        while let (Some(p1), Some(p2), Some(p3)) = (it.next(), it1.next(), it2.next()) {
            let c = output.add_vertex(
                FillVertex {
                    position: p2,
                    normal: compute_normal(p2 - p1, p3 - p2),
                }
            )?;

            output.add_triangle(a, b, c);

            b = c;
        }
    }

    Ok(output.end_geometry())
}

/// Tessellate the stroke for a shape that is described by an iterator of points.
///
/// Convenient when tessellating a shape that is represented as a slice `&[Point]`.
pub fn stroke_polyline<Iter>(
    it: Iter,
    is_closed: bool,
    options: &StrokeOptions,
    output: &mut dyn GeometryBuilder<StrokeVertex>
) -> TessellationResult
where
    Iter: Iterator<Item = Point>,
{
    let mut tess = StrokeTessellator::new();

    tess.tessellate_path(
        FromPolyline::new(is_closed, it),
        options,
        output
    )
}

/// Tessellate an arbitrary shape that is described by an iterator of points.
pub fn fill_polyline<Iter>(
    polyline: Iter,
    tessellator: &mut FillTessellator,
    options: &FillOptions,
    output: &mut dyn GeometryBuilder<FillVertex>
) -> TessellationResult
where
    Iter: Iterator<Item = Point>,
{
    tessellator.tessellate_path(
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
pub(crate) fn circle_flattening_step(radius:f32, mut tolerance: f32) -> f32 {
    // Don't allow high tolerance values (compared to the radius) to avoid edge cases.
    tolerance = f32::min(tolerance, radius);
    2.0 * f32::sqrt(2.0 * tolerance * radius - tolerance * tolerance)
}

#[test]
fn issue_358() {
    use crate::geometry_builder::NoOutput;

    fill_ellipse(
        point(25218.9902, 25669.6738),
        vector(2.0, 2.0),
        Angle { radians: 0.0 },
        &FillOptions::tolerance(1.0),
        &mut NoOutput::new(),
    ).unwrap();
}

#[test]
fn issue_366() {
    use crate::geometry_builder::NoOutput;

    fill_circle(
        point(0.0, 0.0),
        1.0,
        &FillOptions::tolerance(100.0),
        &mut NoOutput::new(),
    ).unwrap();

    stroke_circle(
        point(0.0, 0.0),
        1.0,
        &StrokeOptions::tolerance(100.0),
        &mut NoOutput::new(),
    ).unwrap();
}
