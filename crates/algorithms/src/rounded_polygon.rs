use lyon_path::{
    geom::{euclid, Angle, Vector},
    traits::PathBuilder,
    ArcFlags, Attributes, Polygon, Winding,
};

pub type Point = euclid::default::Point2D<f32>;

/// Adds a sub-path from a polygon but rounds the corners.
///
/// There must be no sub-path in progress when this method is called.
/// No sub-path is in progress after the method is called.
pub fn add_rounded_polygon<B: PathBuilder>(
    builder: &mut B,
    polygon: Polygon<Point>,
    radius: f32,
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

        let turn_winding = get_winding(p_previous, p_current, p_next);
        arc(
            builder,
            Vector::new(radius, radius),
            Angle { radians: 0.0 },
            ArcFlags {
                large_arc: false,
                sweep: turn_winding == Winding::Negative,
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

fn get_winding(p0: Point, p1: Point, p2: Point) -> Winding {
    let cross = (p2 - p0).cross(p1 - p0);
    if cross.is_sign_positive() {
        Winding::Positive
    } else {
        Winding::Negative
    }
}

fn arc<B: PathBuilder>(
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
        flags,
    };

    if svg_arc.is_straight_line() {
        builder.line_to(to, attributes);
    } else {
        let geom_arc = svg_arc.to_arc();
        geom_arc.for_each_quadratic_bezier(&mut |curve| {
            builder.quadratic_bezier_to(curve.ctrl, curve.to, attributes);
        });
    }
}

#[test]
fn rounded_polygon() {
    use crate::geom::point;
    use crate::rounded_polygon::*;
    use alloc::vec::Vec;
    use euclid::approxeq::ApproxEq;

    type Point = euclid::Point2D<f32, euclid::UnknownUnit>;
    type Event = path::Event<Point, Point>;
    let arrow_points = [
        point(-1.0, -0.3),
        point(0.0, -0.3),
        point(0.0, -1.0),
        point(1.5, 0.0),
        point(0.0, 1.0),
        point(0.0, 0.3),
        point(-1.0, 0.3),
    ];

    let arrow_polygon = Polygon {
        points: &arrow_points,
        closed: true,
    };

    let mut builder = lyon_path::Path::builder();
    add_rounded_polygon(&mut builder, arrow_polygon, 2.0, lyon_path::NO_ATTRIBUTES);
    let arrow_path = builder.build();

    //check that we have the right ordering of event types
    let actual_events: alloc::vec::Vec<_> = arrow_path.into_iter().collect();

    let actual_event_types = actual_events
        .iter()
        .map(|x| match x {
            Event::Begin { at: _ } => "b",
            Event::Line { from: _, to: _ } => "l",
            Event::Quadratic {
                from: _,
                ctrl: _,
                to: _,
            } => "q",
            Event::Cubic {
                from: _,
                ctrl1: _,
                ctrl2: _,
                to: _,
            } => "c",
            Event::End {
                last: _,
                first: _,
                close: _,
            } => "e",
        })
        .collect::<alloc::vec::Vec<_>>()
        .concat();

    assert_eq!(actual_event_types, "bqqqlqqlqqlqqlqqlqqlqqle");

    let expected_lines = std::vec![
        (point(1.0, -0.3), point(-2.0, -0.3)),
        (point(0.0, -2.3), point(0.0, 1.0)),
        (point(1.66, 0.11), point(-0.16, -1.11)),
        (point(-0.16, 1.11), point(1.66, -0.11)),
        (point(-0.0, -1.0), point(0.0, 2.3)),
        (point(-2.0, 0.3), point(1.0, 0.3)),
        (point(-1.0, -1.7), point(-1.0, 1.7))
    ];

    //Check that the lines are approximately correct
    let actual_lines: Vec<_> = arrow_path
        .into_iter()
        .filter_map(|event| match event {
            Event::Line { from, to } => Some((from, to)),
            _ => None,
        })
        .collect();

    for (actual, expected) in actual_lines.into_iter().zip(expected_lines.into_iter()) {
        for (actual_point, expected_point) in [(actual.0, expected.0), (actual.1, expected.1)] {
            assert!(actual_point.approx_eq_eps(&expected_point, &Point::new(0.01, 0.01)))
        }
    }

    //Check that each event goes from the end of the previous event

    let mut previous = actual_events[0].to();

    for e in actual_events {
        e.from().approx_eq(&previous);
        previous = e.to();
    }
}
