use lyon_path::{
    builder::{NoAttributes, WithSvg},
    geom::{euclid, Angle, Vector},
    path::BuilderImpl,
    traits::SvgPathBuilder,
    ArcFlags, Polygon,
};
pub type Point = euclid::default::Point2D<f32>;

pub fn add_rounded_polygon(
    b: &mut NoAttributes<BuilderImpl>,
    polygon: Polygon<Point>,
    radius: f32,
    clockwise: bool,
) {
    let builder = NoAttributes::<BuilderImpl>::new();
    let mut svg_builder = WithSvg::new(builder);

    let (_, p_last) = get_line_points(
        polygon.points[polygon.points.len() - 1],
        polygon.points[0],
        radius,
    );

    svg_builder.move_to(p_last);
    for index in 0..polygon.points.len() {
        let (p1, p2) = get_line_points(
            polygon.points[index],
            polygon.points[(index + 1) % polygon.points.len()],
            radius,
        );

        let is_right_turn = get_direction(
            polygon.points[(polygon.points.len() + index - 1) % polygon.points.len()],
            polygon.points[index],
            polygon.points[(index + 1) % polygon.points.len()],
        ) < 0.;

        svg_builder.arc_to(
            Vector::new(radius, radius),
            Angle { radians: 0.0 },
            ArcFlags {
                large_arc: false,
                sweep: is_right_turn == clockwise,
            },
            p1,
        );
        svg_builder.line_to(p2);
    }

    svg_builder.close();

    let path = svg_builder.build();
    b.extend_from_paths(&[path.as_slice()]);
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
