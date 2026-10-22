
use alloc::vec::Vec;

use crate::offset::*;
use crate::stroke::*;
use crate::geom;
use geom::euclid::point2 as point;
use geom::{QuadraticBezierSegment, CubicBezierSegment, vector};
use path::{Path, PathEvent};
use crate::math::{Point, Vector};

fn normal(v: Vector) -> Vector {
    Vector::new(-v.y, v.x)
}

pub fn stroke_to_fill(
    input: impl Iterator<Item = PathEvent>,
    options: &StrokeToFillOptions,
) -> Path {
    let mut builder = Path::builder();

    Stroker::new().stroke_to_fill(input, options, &mut builder);

    builder.build()
}

fn stroke_polyline(points: &[Point], close: bool, options: &StrokeToFillOptions) -> Path {
    let mut path_builder = Path::builder();
    {
        let mut stroker = Stroker::new();
        let mut builder = stroker.builder(&mut path_builder, options);
        builder.begin(points[0]);
        for p in &points[1..] {
            builder.line_to(*p);
        }
        builder.end(close);
    }
    path_builder.build()
}

/// Absolute area of each contour. Only supports line segments.
fn contour_areas(path: &Path) -> Vec<f32> {
    let mut areas = Vec::new();
    let mut area = 0.0;
    for event in path.iter() {
        match event {
            PathEvent::Begin { .. } => area = 0.0,
            PathEvent::Line { from, to } => area += from.to_vector().cross(to.to_vector()),
            PathEvent::End { last, first, .. } => {
                area += last.to_vector().cross(first.to_vector());
                areas.push(area.abs() * 0.5);
            }
            _ => panic!("unexpected curve"),
        }
    }
    areas
}

#[test]
fn open_sub_path_ending_at_start() {
    let options = StrokeToFillOptions::new(10.0).with_line_cap(LineCap::Square);
    let triangle = [
        point(0.0, 0.0),
        point(100.0, 0.0),
        point(100.0, 100.0),
        point(0.0, 0.0),
    ];
    let hairpin = [point(0.0, 0.0), point(100.0, 0.0), point(0.0, 0.0)];

    for points in [&triangle[..], &hairpin[..]] {
        let path = stroke_polyline(points, false, &options);

        // A single contour that goes out and back, with a cap at each end.
        let mut begins = 0;
        let mut ends = 0;
        for event in path.iter() {
            match event {
                PathEvent::Begin { .. } => begins += 1,
                PathEvent::End { close, .. } => {
                    assert!(close);
                    ends += 1;
                }
                _ => {}
            }
        }
        assert_eq!((begins, ends), (1, 1));

        // The square cap at the start point extends behind it.
        let min_x = path
            .iter()
            .filter_map(|event| match event {
                PathEvent::Line { to, .. } => Some(to.x),
                _ => None,
            })
            .fold(f32::MAX, f32::min);
        assert!(min_x < -4.9, "min_x = {min_x}");
    }
}

#[test]
fn closed_sub_path_with_coarse_tolerance() {
    let square = [
        point(0.0, 0.0),
        point(100.0, 0.0),
        point(100.0, 100.0),
        point(0.0, 100.0),
    ];

    for tolerance in [0.25, 1.5] {
        let options = StrokeToFillOptions::new(10.0)
            .with_line_join(LineJoin::Miter)
            .with_tolerance(tolerance);
        let path = stroke_polyline(&square, true, &options);

        // The outer contour has miter corners, the inner one a small fold at
        // the closing corner.
        let areas = contour_areas(&path);
        assert_eq!(areas.len(), 2);
        assert!((areas[0] - 110.0 * 110.0).abs() < 1.0, "tolerance {tolerance}: {areas:?}");
        assert!((areas[1] - 90.0 * 90.0).abs() < 50.0, "tolerance {tolerance}: {areas:?}");
    }
}

/// Distance that the join of a wedge reaches past the corner, along the outer
/// bisector. The two segments meet at the given angle.
fn wedge_join_extent(angle: f32, join: LineJoin, miter_limit: f32, half_width: f32) -> f32 {
    let corner = point(100.0, 0.0);
    let d_out = Vector::new(-angle.cos(), angle.sin());
    let wedge = [point(0.0, 0.0), corner, corner + d_out * 100.0];
    let options = StrokeToFillOptions::new(half_width * 2.0)
        .with_line_join(join)
        .with_miter_limit(miter_limit)
        .with_line_cap(LineCap::Butt);
    let path = stroke_polyline(&wedge, false, &options);

    let bisector = (Vector::new(1.0, 0.0) - d_out).normalize();
    path.iter()
        .filter_map(|event| match event {
            PathEvent::Line { to, .. } => Some((to - corner).dot(bisector)),
            _ => None,
        })
        .fold(f32::MIN, f32::max)
}

#[test]
fn miter_limit() {
    let miter_limit = 4.0;
    let hw = 5.0;
    for degrees in (10..=60).step_by(2) {
        let angle = (degrees as f32).to_radians();
        // Ratio between the miter length and the stroke width.
        let ratio = 1.0 / (angle * 0.5).sin();

        let miter = wedge_join_extent(angle, LineJoin::Miter, miter_limit, hw);
        if ratio <= miter_limit {
            assert!((miter - ratio * hw).abs() < 1e-3, "{degrees} degrees: {miter}");
        } else {
            // Bevel.
            assert!(miter < hw, "{degrees} degrees: {miter}");
        }

        let clipped = wedge_join_extent(angle, LineJoin::MiterClip, miter_limit, hw);
        let expected = (ratio * hw).min(miter_limit * hw);
        assert!((clipped - expected).abs() < 1e-3, "{degrees} degrees: {clipped} != {expected}");
    }
}

#[test]
fn miter_clip_fallback_stays_on_the_clip_line() {
    // Both offset lines are parallel to the clip line (x = 10), so there is
    // no intersection to compute. The fallback must be on the clip line, at
    // the clip distance.
    let (v0, v1) = get_clip_intersections(
        Vector::new(5.0, 0.0),
        Vector::new(-5.0, 0.0),
        Vector::new(1.0, 0.0),
        10.0,
    );

    for v in [v0, v1] {
        assert!((v.x - 10.0).abs() < 1e-4 && v.y.abs() < 1e-4, "{v:?}");
    }
}

#[test]
fn miter_clip_intersections_with_a_small_width() {
    // A 90 degree turn with a miter limit of 1.2, so the miter is clipped.
    // The result must not depend on the scale of the stroke.
    let t0 = Vector::new(1.0, 0.0);
    let t1 = Vector::new(0.0, 1.0);
    let n = compute_normal(t0, t1);
    for w in [10.0, 1.0, 1e-3, 1e-4, 1e-5, 1e-6] {
        let (v0, v1) = get_clip_intersections(normal(t0) * w, normal(t1) * w, n, 1.2 * w);
        let (v0, v1) = (v0 / w, v1 / w);
        // The intersections are on their offset edges, on the clip line.
        let expected = 1.2 * std::f32::consts::SQRT_2 - 1.0;
        assert!((v0.y - 1.0).abs() < 1e-3 && (v0.x + expected).abs() < 1e-3, "{w}: {v0:?}");
        assert!((v1.x + 1.0).abs() < 1e-3 && (v1.y - expected).abs() < 1e-3, "{w}: {v1:?}");
    }
}

#[test]
fn empty_sub_path_caps() {
    // With add_empty_caps, a zero-length sub-path draws its caps around the
    // point. Butt and inverted caps have no area there, so a sub-path with
    // only those must draw nothing. A single cap with area still draws.
    use crate::stroke::LineCap::*;
    let cases = [
        (Butt, Butt, false),
        (RoundInverted, RoundInverted, false),
        (TriangleInverted, Butt, false),
        (Butt, RoundInverted, false),
        (Round, Round, true),
        (Butt, Round, true),
        (RoundInverted, Round, true),
        (Square, TriangleInverted, true),
    ];
    for (start_cap, end_cap, draws) in cases {
        let mut options = StrokeToFillOptions::new(10.0)
            .with_start_cap(start_cap)
            .with_end_cap(end_cap);
        options.add_empty_caps = true;

        let mut builder = Path::builder();
        {
            let mut stroker = Stroker::new();
            let mut stroke = stroker.builder(&mut builder, &options);
            stroke.begin(point(3.0, 4.0));
            stroke.end(false);
        }
        let count = builder.build().iter().count();
        assert_eq!(count > 0, draws, "{start_cap:?} {end_cap:?}");
    }
}

/// Furthest extent of the outline along a direction, sampling curves.
fn extent(path: &Path, direction: Vector) -> f32 {
    let mut extent = f32::MIN;
    for event in path.iter() {
        for i in 0..=32 {
            let t = i as f32 / 32.0;
            let p = match event {
                PathEvent::Begin { at } => at,
                PathEvent::Line { from, to } => from.lerp(to, t),
                PathEvent::Quadratic { from, ctrl, to } => {
                    QuadraticBezierSegment { from, ctrl, to }.sample(t)
                }
                PathEvent::Cubic { from, ctrl1, ctrl2, to } => {
                    CubicBezierSegment { from, ctrl1, ctrl2, to }.sample(t)
                }
                PathEvent::End { .. } => continue,
            };
            extent = extent.max(p.to_vector().dot(direction));
        }
    }
    extent
}

#[test]
fn hairpin_turns() {
    let hw = 5.0;
    let miter_limit = 4.0;
    // Turning angles in degrees, up to a complete reversal.
    for degrees in [179.0f32, 179.5, 179.6, 179.8, 179.95, 180.0] {
        let a = (180.0 - degrees).to_radians();
        let corner = point(100.0, 0.0);
        let d_out = Vector::new(-a.cos(), a.sin());
        let hairpin = [point(0.0, 0.0), corner, corner + d_out * 100.0];
        // Distance past the corner along the outer bisector.
        let bisector = (Vector::new(1.0, 0.0) - d_out).normalize();
        let past_corner = |path: &Path| extent(path, bisector) - corner.to_vector().dot(bisector);
        let stroke = |join| {
            let options = StrokeToFillOptions::new(hw * 2.0)
                .with_line_join(join)
                .with_miter_limit(miter_limit)
                .with_line_cap(LineCap::Butt);
            stroke_polyline(&hairpin, false, &options)
        };

        // The round join wraps around the tip.
        let round = past_corner(&stroke(LineJoin::Round));
        assert!((round - hw).abs() < 0.01, "{degrees}: {round}");

        // The miter limit is exceeded so miter joins are bevels.
        let miter: Vec<PathEvent> = stroke(LineJoin::Miter).iter().collect();
        let bevel: Vec<PathEvent> = stroke(LineJoin::Bevel).iter().collect();
        assert_eq!(miter, bevel, "{degrees}");

        // The miter is clipped half of the miter length limit past the corner.
        let clipped = past_corner(&stroke(LineJoin::MiterClip));
        assert!((clipped - miter_limit * hw).abs() < 0.01, "{degrees}: {clipped}");
    }
}

#[test]
fn closed_two_point_sub_path() {
    let options = StrokeToFillOptions::new(10.0).with_line_join(LineJoin::Round);
    let path = stroke_polyline(&[point(0.0, 0.0), point(100.0, 0.0)], true, &options);
    let max = extent(&path, Vector::new(1.0, 0.0));
    let min = -extent(&path, Vector::new(-1.0, 0.0));
    assert!((max - 105.0).abs() < 0.01, "{max}");
    assert!((min + 5.0).abs() < 0.01, "{min}");
}

#[test]
fn sub_paths_smaller_than_the_tolerance() {
    // A closed polygon about 0.18 units wide (from the paris-30k.svg test file).
    let polygon = [
        point(0.396, 0.975),
        point(0.396, 0.916),
        point(0.426, 0.878),
        point(0.465, 0.857),
        point(0.504, 0.857),
        point(0.548, 0.881),
        point(0.572, 0.935),
        point(0.568, 0.985),
        point(0.542, 1.014),
        point(0.504, 1.034),
        point(0.46, 1.034),
        point(0.416, 1.009),
    ];
    let directions = [
        Vector::new(1.0, 0.0),
        Vector::new(-1.0, 0.0),
        Vector::new(0.0, 1.0),
        Vector::new(0.0, -1.0),
    ];
    let polygon_extents = directions.map(|d| {
        polygon.iter().map(|p| p.to_vector().dot(d)).fold(f32::MIN, f32::max)
    });

    for join in [LineJoin::Round, LineJoin::Bevel, LineJoin::Miter] {
        for close in [true, false] {
            let options = StrokeToFillOptions::new(1.0)
                .with_line_join(join)
                .with_line_cap(LineCap::Round)
                .with_tolerance(0.25);
            let path = stroke_polyline(&polygon, close, &options);
            for (d, polygon_extent) in directions.iter().zip(polygon_extents) {
                // The stroke extends at least the half width beyond the polygon,
                // and not much more (the miter limit is 4).
                let e = extent(&path, *d) - polygon_extent;
                assert!(e > 0.5 - 0.25 && e < 2.0, "{join:?} close:{close} {d:?}: {e}");
            }
        }
    }

    // Sub-paths made of a single point are still empty without add_empty_caps.
    let options = StrokeToFillOptions::new(1.0).with_line_join(LineJoin::Round);
    let path = stroke_polyline(&[point(1.0, 1.0), point(1.0, 1.0), point(1.0, 1.0)], true, &options);
    assert_eq!(path.iter().count(), 0);
}

#[test]
fn short_first_segment_of_closed_sub_path() {
    // A thin closed shape (from the paris-30k.svg test file) whose first segment
    // is short enough to be discarded. Both sides of the stroke must see the
    // same simplified sub-path, otherwise neither side gets an outer join at
    // the top.
    let mut builder = Path::builder();
    builder.begin(point(0.087, 10.391));
    builder.line_to(point(0.3, 10.391));
    builder.cubic_bezier_to(point(0.234, 10.257), point(0.196, 10.106), point(0.196, 9.947));
    builder.line_to(point(0.196, 7.799));
    builder.cubic_bezier_to(point(0.196, 7.52), point(0.31, 7.268), point(0.495, 7.086));
    builder.cubic_bezier_to(point(0.31, 6.904), point(0.196, 6.652), point(0.196, 6.373));
    builder.line_to(point(0.196, 4.635));
    builder.cubic_bezier_to(point(0.196, 4.476), point(0.234, 4.324), point(0.3, 4.191));
    builder.line_to(point(0.087, 4.191));
    builder.end(true);
    let source = builder.build();

    let tolerance = 0.5;
    for width in [1.0, 25.0] {
        let options = StrokeToFillOptions::new(width)
            .with_line_join(LineJoin::Round)
            .with_tolerance(tolerance);
        let path = stroke_to_fill(source.iter(), &options);
        let top = extent(&path, Vector::new(0.0, 1.0));
        let bottom = -extent(&path, Vector::new(0.0, -1.0));
        let expected_top = 10.391 + width * 0.5;
        let expected_bottom = 4.191 - width * 0.5;
        assert!((top - expected_top).abs() < tolerance, "width {width}: top {top}");
        assert!((bottom - expected_bottom).abs() < tolerance, "width {width}: bottom {bottom}");
    }
}

#[test]
fn short_closing_segment() {
    // A closed quadrilateral about 0.35 units wide (from the paris-30k.svg test
    // file). Some of its segments, including the closing one, are short enough
    // to be discarded, and the stroke is much wider than the shape.
    let polygon = [
        point(0.779, 0.176),
        point(1.091, 0.084),
        point(1.155, 0.289),
        point(0.878, 0.381),
    ];
    let tolerance = 0.5;
    let half_width = 12.5;
    let options = StrokeToFillOptions::new(half_width * 2.0)
        .with_line_join(LineJoin::Round)
        .with_tolerance(tolerance);
    let path = stroke_polyline(&polygon, true, &options);

    // The outer side of the stroke is a disk around the shape.
    for i in 0..16 {
        let a = i as f32 * std::f32::consts::TAU / 16.0;
        let d = Vector::new(a.cos(), a.sin());
        let ideal = polygon.iter().map(|p| p.to_vector().dot(d)).fold(f32::MIN, f32::max)
            + half_width;
        let e = extent(&path, d);
        assert!((e - ideal).abs() < tolerance, "{d:?}: {e} (ideal {ideal})");
    }
}

#[test]
fn collinear_curves_that_change_direction() {
    let hw = 5.0;
    let options = StrokeToFillOptions::new(hw * 2.0)
        .with_line_join(LineJoin::Round)
        .with_line_cap(LineCap::Butt);
    let quad = |ctrl: (f32, f32), to: (f32, f32)| {
        QuadraticBezierSegment { from: point(0.0, 0.0), ctrl: point(ctrl.0, ctrl.1), to: point(to.0, to.1) }
    };
    let cubic = |ctrl1: (f32, f32), ctrl2: (f32, f32), to: (f32, f32)| CubicBezierSegment {
        from: point(0.0, 0.0),
        ctrl1: point(ctrl1.0, ctrl1.1),
        ctrl2: point(ctrl2.0, ctrl2.1),
        to: point(to.0, to.1),
    };
    let quads = [
        // Goes out to x = 50 and back to the start.
        quad((100.0, 0.0), (0.0, 0.0)),
        // Goes past its end to x = 112.5.
        quad((150.0, 0.0), (100.0, 0.0)),
        // Goes back past its start.
        quad((-100.0, 0.0), (10.0, 0.0)),
    ];
    let cubics = [
        // Goes past both ends.
        cubic((100.0, 0.0), (-100.0, 0.0), (10.0, 0.0)),
        cubic((0.0, 0.0), (100.0, 0.0), (0.0, 0.0)),
    ];

    let check = |path: Path, sample: &dyn Fn(f32) -> Point| {
        // The round joins at the turning points extend the stroke by the
        // half width.
        let n = 100_000;
        let (min, max) = (0..=n)
            .map(|i| sample(i as f32 / n as f32).x)
            .fold((f32::MAX, f32::MIN), |(min, max), x| (min.min(x), max.max(x)));
        let start = sample(0.0).x;
        let end = sample(1.0).x;
        let expected_max = if max > start.max(end) { max + hw } else { max };
        let expected_min = if min < start.min(end) { min - hw } else { min };

        let max_x = extent(&path, Vector::new(1.0, 0.0));
        let min_x = -extent(&path, Vector::new(-1.0, 0.0));
        assert!((max_x - expected_max).abs() < 0.01, "{max_x} != {expected_max}");
        assert!((min_x - expected_min).abs() < 0.01, "{min_x} != {expected_min}");
    };

    for curve in quads {
        let mut builder = Path::builder();
        {
            let mut stroker = Stroker::new();
            let mut stroke = stroker.builder(&mut builder, &options);
            stroke.begin(curve.from);
            stroke.quadratic_bezier_to(curve.ctrl, curve.to);
            stroke.end(false);
        }
        check(builder.build(), &|t| curve.sample(t));
    }

    for curve in cubics {
        let mut builder = Path::builder();
        {
            let mut stroker = Stroker::new();
            let mut stroke = stroker.builder(&mut builder, &options);
            stroke.begin(curve.from);
            stroke.cubic_bezier_to(curve.ctrl1, curve.ctrl2, curve.to);
            stroke.end(false);
        }
        check(builder.build(), &|t| curve.sample(t));
    }
}

/// Winding number of the outline around a point, flattening curves.
fn winding_number(path: &Path, p: Point, tolerance: f32) -> i32 {
    let mut winding = 0;
    let mut add_edge = |from: Point, to: Point| {
        let side = (to - from).cross(p - from);
        if from.y <= p.y && to.y > p.y && side > 0.0 {
            winding += 1;
        } else if from.y > p.y && to.y <= p.y && side < 0.0 {
            winding -= 1;
        }
    };
    for event in path.iter() {
        match event {
            PathEvent::Begin { .. } => {}
            PathEvent::Line { from, to } => add_edge(from, to),
            PathEvent::Quadratic { from, ctrl, to } => QuadraticBezierSegment { from, ctrl, to }
                .for_each_flattened(tolerance, &mut |s| add_edge(s.from, s.to)),
            PathEvent::Cubic { from, ctrl1, ctrl2, to } => CubicBezierSegment { from, ctrl1, ctrl2, to }
                .for_each_flattened(tolerance, &mut |s| add_edge(s.from, s.to)),
            PathEvent::End { last, first, .. } => add_edge(last, first),
        }
    }
    winding
}

#[test]
fn closing_join_after_cubic() {
    // A closed shape made of two cubic bézier curves (from the tiger.svg test
    // file). On the reversed side of the stroke, the quadratic bézier curves
    // that approximate the last cubic bézier curve do not end exactly at the
    // start of the sub-path. Both sides must still agree about the closing
    // join.
    let start = point(246.51247, 807.6567);
    let mut builder = Path::builder();
    builder.begin(start);
    builder.cubic_bezier_to(start, point(266.99396, 768.10626), point(228.50287, 813.6599));
    builder.cubic_bezier_to(point(228.50287, 813.6599), point(247.92499, 796.35657), start);
    builder.end(true);
    let source = builder.build();

    let half_width = 12.5;
    for tolerance in [0.25, 0.05, 0.01] {
        let options = StrokeToFillOptions::new(half_width * 2.0)
            .with_line_join(LineJoin::Round)
            .with_tolerance(tolerance);
        let path = stroke_to_fill(source.iter(), &options);

        // The start point is on the source path, so the stroke covers a disk
        // around it.
        for i in 0..32 {
            let a = i as f32 * std::f32::consts::TAU / 32.0;
            let p = start + Vector::new(a.cos(), a.sin()) * (half_width - 2.0 * tolerance);
            let winding = winding_number(&path, p, tolerance * 0.1);
            assert!(winding != 0, "tolerance {tolerance}: {p:?} is not covered");
        }
    }
}

#[test]
fn curve_ending_at_its_start_after_discarded_segments() {
    // A quadratic bézier curve that ends where it starts (a line that goes to
    // its turning point and back) cannot be stretched to start at the end of
    // the previous segment, so it is moved as a whole. As the last segment of
    // a closed sub-path, it does not end at the first point, and a line closes
    // the sub-path.
    let mut builder = Path::builder();
    builder.begin(point(0.0, 0.0));
    builder.line_to(point(100.0, 0.0));
    builder.line_to(point(100.0, 100.0));
    builder.line_to(point(0.1, 0.1));
    builder.line_to(point(0.05, 0.12));
    builder.quadratic_bezier_to(point(-30.0, 30.0), point(0.05, 0.12));
    builder.end(true);
    let source = builder.build();

    let half_width = 5.0;
    for join in [LineJoin::Miter, LineJoin::Bevel, LineJoin::Round] {
        let options = StrokeToFillOptions::new(half_width * 2.0)
            .with_line_join(join)
            .with_tolerance(0.25);
        let path = stroke_to_fill(source.iter(), &options);

        // The curve up to near its turning point, the corners and the middle
        // of the sides are covered. The turning point itself is on the edge of
        // the stroke with miter and bevel joins.
        let tip = QuadraticBezierSegment {
            from: point(0.05, 0.12),
            ctrl: point(-30.0, 30.0),
            to: point(0.05, 0.12),
        }
        .sample(0.45);
        for p in [tip, point(0.0, 0.0), point(100.0, 0.0), point(100.0, 100.0), point(50.0, 0.0), point(50.0, 50.0)] {
            for offset in [Vector::new(0.0, 0.0), Vector::new(3.0, 0.0), Vector::new(0.0, -3.0)] {
                let winding = winding_number(&path, p + offset, 0.01);
                assert!(winding != 0, "{join:?}: {:?} is not covered", p + offset);
            }
        }
    }
}

fn cubic(points: [(f32, f32); 4]) -> CubicBezierSegment<f32> {
    let [from, ctrl1, ctrl2, to] = points.map(|(x, y)| point(x, y));
    CubicBezierSegment { from, ctrl1, ctrl2, to }
}

#[test]
fn straight_cubic_bezier_turnarounds() {
    // A short straight cubic bézier curve that doesn't change direction (from
    // the paris-30k.svg test file). Its coefficients are small enough for
    // the root finding to return noise.
    let short = cubic([(797.399, 253.83398), (797.387, 253.84003), (797.376, 253.84497), (797.365, 253.85101)]);
    assert!(cubic_bezier_is_linear(&short, 0.25));
    assert!(cubic_bezier_turnarounds(&short).is_empty());

    // Goes past its end and comes back.
    let back = cubic([(0.0, 0.0), (100.0, 0.0), (150.0, 0.0), (100.0, 0.0)]);
    assert_eq!(cubic_bezier_turnarounds(&back).len(), 1);
}

#[test]
fn cubic_bezier_cusps() {
    let tolerance = 0.25;
    let cusp = cubic([(0.0, 0.0), (100.0, 100.0), (0.0, 100.0), (100.0, 0.0)]);
    let cusps = find_cubic_bezier_cusps(&cusp, tolerance);
    assert_eq!(cusps.len(), 1);
    assert!((cusps[0] - 0.5).abs() < 1e-5, "{cusps:?}");

    // The speed of these curves has extrema, but no cusps.
    let arch = cubic([(0.0, 0.0), (0.0, 100.0), (100.0, 100.0), (100.0, 0.0)]);
    let s_curve = cubic([(0.0, 0.0), (100.0, 0.0), (0.0, 100.0), (100.0, 100.0)]);
    let with_loop = cubic([(0.0, 0.0), (150.0, 100.0), (-50.0, 100.0), (100.0, 0.0)]);
    // The speed of these curves vanishes at one end, where a control point is
    // on the endpoint. It is not a cusp within the curve.
    let flat_start = cubic([(0.0, 0.0), (0.0, 0.0), (100.0, 100.0), (100.0, 0.0)]);
    let flat_end = cubic([(0.0, 0.0), (0.0, 100.0), (30.0, 10.0), (30.0, 10.0)]);
    for curve in [arch, s_curve, with_loop, flat_start, flat_end] {
        assert!(find_cubic_bezier_cusps(&curve, tolerance).is_empty(), "{curve:?}");
    }

    // Moving a control point of the cusp rounds its tip, with a radius of
    // curvature of 0.093 and 0.37 respectively.
    let tight = cubic([(0.0, 0.0), (100.0, 100.0), (10.0, 100.0), (100.0, 0.0)]);
    let round = cubic([(0.0, 0.0), (100.0, 100.0), (20.0, 100.0), (100.0, 0.0)]);
    assert_eq!(find_cubic_bezier_cusps(&tight, tolerance).len(), 1);
    assert!(find_cubic_bezier_cusps(&tight, 0.05).is_empty());
    assert!(find_cubic_bezier_cusps(&round, tolerance).is_empty());

    // The stroke gets a round join at the cusp, whose tip is at y = 75.
    let options = StrokeToFillOptions::new(20.0)
        .with_line_join(LineJoin::Round)
        .with_line_cap(LineCap::Butt);
    let mut builder = Path::builder();
    {
        let mut stroker = Stroker::new();
        let mut stroke = stroker.builder(&mut builder, &options);
        stroke.begin(cusp.from);
        stroke.cubic_bezier_to(cusp.ctrl1, cusp.ctrl2, cusp.to);
        stroke.end(false);
    }
    let max_y = extent(&builder.build(), Vector::new(0.0, 1.0));
    assert!((max_y - 85.0).abs() < 0.01, "{max_y}");
}


fn quad(from: (f32, f32), ctrl: (f32, f32), to: (f32, f32)) -> QuadraticBezierSegment<f32> {
    QuadraticBezierSegment {
        from: point(from.0, from.1),
        ctrl: point(ctrl.0, ctrl.1),
        to: point(to.0, to.1),
    }
}

const CURVES: [((f32, f32), (f32, f32), (f32, f32)); 4] = [
    ((0.0, 0.0), (50.0, 100.0), (100.0, 0.0)),
    ((0.0, 0.0), (100.0, 100.0), (20.0, 0.0)),
    ((10.0, 30.0), (200.0, 0.0), (0.0, 90.0)),
    ((0.0, 0.0), (1000.0, 1.0), (0.0, 2.0)),
];

#[test]
fn degenerate_curve_produces_offset_pieces() {
    // A quadratic that goes out along a line and comes back. Its tangent
    // is undefined at the turning point, where the offset is only
    // defined as a one-sided limit. The two halves must still be offset.
    let curve = quad((0.0, 0.0), (1.0, 0.0), (0.0, 0.0));

    for (dist, expected) in [
        (1.0f32, [(0.0, 1.0), (0.5, 1.0), (0.5, -1.0), (0.0, -1.0)]),
        (-1.0f32, [(0.0, -1.0), (0.5, -1.0), (0.5, 1.0), (0.0, 1.0)]),
    ] {
        let mut endpoints = Vec::new();
        for_each_offsets(&curve, [dist], 0.25, &mut |sub| {
            endpoints.push(sub[0].from);
            endpoints.push(sub[0].to);
        });

        // Two pieces, ending at the offsets on each side of the turning
        // point.
        assert_eq!(endpoints.len(), 4, "dist {dist}: {endpoints:?}");
        for (i, &(x, y)) in expected.iter().enumerate() {
            assert!(
                (endpoints[i].x - x).abs() < 1e-4 && (endpoints[i].y - y).abs() < 1e-4,
                "dist {dist}: {endpoints:?} != {expected:?}",
            );
        }
    }
}

#[test]
fn degenerate_curve_with_cusp_near_an_end() {
    // A collinear curve that overshoots its end, with the turning point
    // at t = 1/102. critical_points discards it for being too close to an
    // end, and turn_splits places a few splits around it rather than
    // exactly on it. The offset must still be the two one-sided pieces,
    // with no sub-curve across the cusp.
    let curve = quad((0.0, 0.0), (1.0, 0.0), (-100.0, 0.0));
    let tip = 1.0 / 102.0;

    let mut endpoints = Vec::new();
    for_each_offsets(&curve, [1.0], 0.25, &mut |sub| {
        endpoints.push(sub[0].from);
        endpoints.push(sub[0].to);
    });

    let expected = [(0.0, 1.0), (tip, 1.0), (tip, -1.0), (-100.0, -1.0)];
    assert_eq!(endpoints.len(), 4, "{endpoints:?}");
    for (i, &(x, y)) in expected.iter().enumerate() {
        assert!(
            (endpoints[i].x - x).abs() < 1e-4 && (endpoints[i].y - y).abs() < 1e-4,
            "{endpoints:?} != {expected:?}",
        );
    }
}

#[test]
fn critical_points_match_radius_of_curvature() {
    for (from, ctrl, to) in CURVES {
        let curve = quad(from, ctrl, to);
        let b2 = (curve.from.to_vector() - curve.ctrl.to_vector() * 2.0 + curve.to.to_vector()) * 2.0;
        let radius = |t: f32| {
            let b1 = curve.derivative(t);
            b1.length().powi(3) / b1.cross(b2).abs()
        };

        for offset in [1.0f32, 5.0, 20.0, 40.0, 80.0, -40.0] {
            // Find where the radius of curvature crosses the offset distance
            // by sampling, away from the endpoints where critical_points
            // discards solutions.
            let mut expected = Vec::new();
            let n = 100_000;
            let mut prev = radius(0.0) - offset.abs();
            for i in 1..=n {
                let t = i as f32 / n as f32;
                let r = radius(t) - offset.abs();
                if r.signum() != prev.signum() && t > 0.051 && t < 0.949 {
                    expected.push(t);
                }
                prev = r;
            }

            let (t1, t2) = critical_points(&curve, offset);
            let found: Vec<f32> = [t1, t2].into_iter().flatten().collect();
            assert_eq!(found.len(), expected.len(), "{curve:?} {offset}: {found:?} != {expected:?}");
            for (a, b) in found.iter().zip(expected.iter()) {
                assert!((a - b).abs() < 1e-3, "{curve:?} {offset}: {found:?} != {expected:?}");
            }
        }
    }
}

#[test]
fn offset_is_within_tolerance() {
    let tolerance = 0.25;
    for (from, ctrl, to) in CURVES {
        let curve = quad(from, ctrl, to);
        for dist in [1.0f32, -1.0, 5.0, -5.0, 20.0, -20.0, 50.0, -50.0] {
            let exact = |t: f32| {
                let tangent = curve.derivative(t).normalize();
                curve.sample(t) + vector(-tangent.y, tangent.x) * dist
            };
            // Sample the exact offset curve uniformly in t and in tangent
            // angle, so that sharp turns are sampled densely as well.
            let n = 2000;
            let p0 = curve.ctrl - curve.from;
            let q = curve.from.to_vector() - curve.ctrl.to_vector() * 2.0 + curve.to.to_vector();
            let turn = p0.angle_to(p0 + q).radians;
            let mut ts: Vec<f32> = (0..=n).map(|i| i as f32 / n as f32).collect();
            for i in 1..n {
                // The tangent is parallel to p0 + t q.
                let angle = turn * i as f32 / n as f32;
                let dir = p0.normalize() * angle.cos() + vector(-p0.y, p0.x).normalize() * angle.sin();
                let t = -p0.cross(dir) / q.cross(dir);
                if t > 0.0 && t < 1.0 {
                    ts.push(t);
                }
            }
            ts.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let samples: Vec<_> = ts.iter().map(|t| exact(*t)).collect();

            // Distance to the exact offset curve: closest sample, refined
            // with a ternary search between its neighbors.
            let distance = |p: geom::Point<f32>| {
                let i = (0..samples.len())
                    .min_by(|a, b| {
                        let da = (samples[*a] - p).square_length();
                        let db = (samples[*b] - p).square_length();
                        da.partial_cmp(&db).unwrap()
                    })
                    .unwrap();
                let mut lo = ts[i.saturating_sub(1)];
                let mut hi = ts[(i + 1).min(ts.len() - 1)];
                for _ in 0..40 {
                    let t1 = lo + (hi - lo) / 3.0;
                    let t2 = hi - (hi - lo) / 3.0;
                    if (exact(t1) - p).length() < (exact(t2) - p).length() {
                        hi = t2;
                    } else {
                        lo = t1;
                    }
                }
                (exact((lo + hi) * 0.5) - p).length()
            };

            let mut max_err: f32 = 0.0;
            for_each_offsets(&curve, [dist], tolerance, &mut |sub_curve| {
                for i in 0..=16 {
                    max_err = max_err.max(distance(sub_curve[0].sample(i as f32 / 16.0)));
                }
            });
            assert!(max_err < tolerance * 1.2, "{curve:?} {dist}: {max_err}");
        }
    }
}

#[test]
fn ranges_too_small_to_halve() {
    // The range is the precision of its parameters, so halving it gives
    // an empty range before reaching the minimum step. The offset is on
    // the concave side, beyond the radius of curvature (about 1400), so
    // the error is measured rather than bounded, and at these
    // coordinates it can't be measured within the tolerance.
    let curve = quad((1.0e7, 0.0), (1.0e7 + 1000.0, 2000.0), (1.0e7 + 2000.0, 0.0));
    let start = 0.25f32;
    let end = f32::from_bits(start.to_bits() + 1);
    let mut count = 0;
    for_each_offset_inner(&curve, start..end, [-3000.0], 0.0001, &mut |_| count += 1);
    assert!(count > 0);
}

#[test]
fn offset_error_bound_holds() {
    for (from, ctrl, to) in CURVES {
        let curve = quad(from, ctrl, to);
        for dist in [0.5f32, -0.5, 5.0, -5.0, 20.0, -20.0] {
            let tangent = |t: f32| curve.derivative(t).normalize();
            let exact = |t: f32| curve.sample(t) + normal(tangent(t)) * dist;
            let n = 64;
            for i in 0..n {
                let (t0, t1) = (i as f32 / n as f32, (i + 1) as f32 / n as f32);
                let sub_curve = curve.split_range(t0..t1);
                let (v0, v1) = (tangent(t0), tangent(t1));
                let bisector = normal((v0 + v1).normalize());
                let cos_half_turn = bisector.dot(normal(v1));
                if cos_half_turn < (MAX_TURN * 0.5).cos() {
                    continue;
                }
                let offset_curve = QuadraticBezierSegment {
                    from: exact(t0),
                    ctrl: sub_curve.ctrl + bisector * (dist / cos_half_turn),
                    to: exact(t1),
                };
                let bound = offset_error_bound(&sub_curve, cos_half_turn, dist);

                // Distance to the exact offset curve, sampled densely
                // around the range.
                let samples: Vec<geom::Point<f32>> = (0..=400)
                    .map(|j| exact((t0 + (t1 - t0) * (j as f32 / 200.0 - 0.5)).max(0.0).min(1.0)))
                    .collect();
                let mut error: f32 = 0.0;
                for j in 0..=16 {
                    let p = offset_curve.sample(j as f32 / 16.0);
                    let d = samples
                        .windows(2)
                        .map(|s| crate::geom::LineSegment { from: s[0], to: s[1] }.distance_to_point(p))
                        .fold(f32::MAX, f32::min);
                    error = error.max(d);
                }
                assert!(error <= bound + 1e-3, "{curve:?} {dist} {t0}: {error} > {bound}");
            }
        }
    }
}
