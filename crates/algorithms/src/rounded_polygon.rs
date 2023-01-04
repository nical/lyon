use lyon_path::{
    geom::{euclid, Angle, Vector},
    traits::{PathBuilder},
    ArcFlags, Attributes, Polygon,
};
pub type Point = euclid::default::Point2D<f32>;

pub fn add_rounded_polygon<B: PathBuilder>(
    builder: &mut B,
    polygon: Polygon<Point>,
    radius: f32,
    clockwise: bool,
    attributes: Attributes,
) {
    //p points are original polygon points
    //q points are the actual points we will draw lines and arcs between

    let p_last = polygon.points[polygon.points.len() - 1];

    let (_, q_last) = get_line_points(p_last, polygon.points[0], radius);

    //We begin with the arc that replaces the first point
    builder.begin(q_last, attributes);

    let mut q_previous = q_last;
    let mut p_previous = p_last;
    for index in 0..polygon.points.len() {
        let p_current = polygon.points[index];
        let p_next = polygon.points[(index + 1) % polygon.points.len()];

        let (q_current, q_next) = get_line_points(p_current, p_next, radius);

        let is_right_turn = get_direction(p_previous, p_current, p_next) < 0.;

        arc_to(
            builder,
            Vector::new(radius, radius),
            Angle { radians: 0.0 },
            ArcFlags {
                large_arc: false,
                sweep: is_right_turn == clockwise,
            },
            q_previous,
            q_current,
            attributes,
        );

        builder.line_to(q_next, attributes);
        q_previous = q_next;
        p_previous = p_current;
    }

    builder.end(polygon.closed);
}

fn get_line_points(p1: Point, p2: Point, radius: f32) -> (Point, Point) {
    let dist = p1.distance_to(p2);
    let ratio = radius / dist;

    let r1 = p1.lerp(p2, ratio);
    let r2 = p1.lerp(p2, 1. - ratio);

    (Point::new(r1.x, r1.y), Point::new(r2.x, r2.y))
}

fn get_direction(p0: Point, p1: Point, p2: Point) -> f32 {
    (p2 - p0).cross(p1 - p0)
}

fn arc_to<B: PathBuilder>(
    builder: &mut B,
    radii: Vector<f32>,
    x_rotation: Angle<f32>,
    flags: ArcFlags,
    from: Point,
    to: Point,
    attributes: Attributes,
) {
    let svg_arc = lyon_path::geom::SvgArc {
        from,
        to,
        radii,
        x_rotation,
        flags: ArcFlags {
            large_arc: flags.large_arc,
            sweep: flags.sweep,
        },
    };

    if svg_arc.is_straight_line(){
        builder.line_to(to, attributes);
    }
    else{
        let geom_arc = svg_arc.to_arc();
        geom_arc.for_each_quadratic_bezier(&mut |curve| {
            builder.quadratic_bezier_to(curve.ctrl, curve.to, attributes);
        });
    }
}
