//! Round clipping must change only the clipped join and preserve vertex metadata.

use lyon_path::{geom::CubicBezierSegment, math::point, LineJoin, Path};
use lyon_tessellation::{
    ArcsClip, BuffersBuilder, StrokeOptions, StrokeTessellator, StrokeVertex, VertexBuffers,
};

fn segments(curvature: f32) -> [CubicBezierSegment<f32>; 2] {
    [
        CubicBezierSegment {
            from: point(-40.0, 0.0),
            ctrl1: point(-20.0, 0.0),
            ctrl2: point(-10.0, 0.0),
            to: point(0.0, 0.0),
        },
        CubicBezierSegment {
            from: point(0.0, 0.0),
            ctrl1: point(0.0, 10.0),
            ctrl2: point(-150.0 * curvature, 20.0),
            to: point(0.0, 40.0),
        },
    ]
}

fn options(mode: ArcsClip) -> StrokeOptions {
    StrokeOptions::default()
        .with_line_join(LineJoin::Arcs)
        .with_arcs_clip(mode)
        .with_line_width(4.0)
        .with_miter_limit(20.0)
        .with_tolerance(0.01)
}

fn path_from_segments(curvature: f32, mirror: f32) -> Path {
    let mut builder = Path::builder();
    let curves = segments(curvature);
    let transform = |p: lyon_path::math::Point| point(p.x, p.y * mirror);
    builder.begin(transform(curves[0].from));
    for c in curves {
        builder.cubic_bezier_to(transform(c.ctrl1), transform(c.ctrl2), transform(c.to));
    }
    builder.end(false);
    builder.build()
}

fn mesh(path: &Path, options: &StrokeOptions) -> VertexBuffers<lyon_path::math::Point, u32> {
    let mut mesh = VertexBuffers::new();
    StrokeTessellator::new()
        .tessellate_path(
            path,
            options,
            &mut BuffersBuilder::new(&mut mesh, |v: StrokeVertex| v.position()),
        )
        .unwrap();
    mesh
}

#[test]
fn round_clip_preserves_unclipped_arcs_and_round_fallbacks_exactly() {
    for curvature in [-0.4, -0.6, 0.1] {
        let path = path_from_segments(curvature, 1.0);
        let standard = mesh(&path, &options(ArcsClip::Butt));
        let rounded = mesh(&path, &options(ArcsClip::Round));
        assert_eq!(standard.vertices, rounded.vertices);
        assert_eq!(standard.indices, rounded.indices);
    }
}

#[test]
fn rounded_cut_adds_geometry_for_both_turn_directions_and_keeps_original_vertices() {
    for mirror in [-1.0, 1.0] {
        let path = path_from_segments(-0.4, mirror);
        let options = options(ArcsClip::Butt).with_miter_limit(1.6);
        let standard = mesh(&path, &options);
        let rounded = mesh(&path, &options.with_arcs_clip(ArcsClip::Round));
        assert!(rounded.vertices.len() > standard.vertices.len());
        assert!(
            standard
                .vertices
                .iter()
                .all(|p| rounded.vertices.contains(p)),
            "base stroke moved"
        );
        assert!(rounded
            .vertices
            .iter()
            .all(|p| p.x.is_finite() && p.y.is_finite()));
        for triangle in rounded.indices.chunks_exact(3) {
            let [a, b, c] =
                [triangle[0], triangle[1], triangle[2]].map(|i| rounded.vertices[i as usize]);
            if [a, b, c].iter().all(|p| standard.vertices.contains(p)) {
                continue;
            }
            assert!(
                (b - a).cross(c - a) <= 1.0e-4,
                "inverted triangle {:?} {:?} {:?}",
                a,
                b,
                c
            );
        }
    }
}

#[test]
fn round_cut_vertices_keep_join_metadata_instead_of_cap_center() {
    let path = path_from_segments(-0.4, 1.0);
    let options = options(ArcsClip::Round).with_miter_limit(1.6);
    let mut mesh: VertexBuffers<_, u32> = VertexBuffers::new();
    StrokeTessellator::new()
        .tessellate_path(
            &path,
            &options,
            &mut BuffersBuilder::new(&mut mesh, |v: StrokeVertex| {
                (v.position(), v.position_on_path(), v.line_width())
            }),
        )
        .unwrap();
    assert!(mesh.vertices.iter().all(|v| v.2 == 4.0));
    let original = mesh_positions_for_standard(&path, options);
    let new: Vec<_> = mesh
        .vertices
        .iter()
        .filter(|v| !original.vertices.contains(&v.0))
        .collect();
    assert!(!new.is_empty());
    assert!(
        new.iter().all(|v| v.1 == point(0.0, 0.0)),
        "tip must retain the real join source"
    );
}

fn mesh_positions_for_standard(
    path: &Path,
    options: StrokeOptions,
) -> VertexBuffers<lyon_path::math::Point, u32> {
    mesh(path, &options.with_arcs_clip(ArcsClip::Butt))
}

#[test]
fn straight_segment_miter_cuts_are_rounded_too() {
    let mut builder = Path::builder();
    builder.begin(point(-40.0, 0.0));
    builder.line_to(point(0.0, 0.0));
    builder.line_to(point(-30.0, 10.0));
    builder.end(false);
    let path = builder.build();
    let options = options(ArcsClip::Butt).with_miter_limit(2.0);
    let standard = mesh(&path, &options);
    let rounded = mesh(&path, &options.with_arcs_clip(ArcsClip::Round));
    assert!(rounded.vertices.len() > standard.vertices.len());
}

#[test]
fn zero_and_subunit_limits_produce_finite_rounded_geometry() {
    let path = path_from_segments(-0.4, 1.0);
    for limit in [0.0, 0.1, 0.5, 1.0, 1.6] {
        let style = options(ArcsClip::Butt).with_miter_limit(limit);
        let standard = mesh(&path, &style);
        let rounded = mesh(&path, &style.with_arcs_clip(ArcsClip::Round));
        assert!(rounded
            .vertices
            .iter()
            .all(|p| p.x.is_finite() && p.y.is_finite()));
        assert!(standard
            .vertices
            .iter()
            .all(|p| rounded.vertices.contains(p)));
        if limit == 0.0 {
            assert_eq!(standard.vertices, rounded.vertices);
            assert_eq!(standard.indices, rounded.indices);
        }
    }
}

#[test]
fn asymmetric_curved_miter_cut_gets_a_round_tip() {
    let mut builder = Path::builder();
    builder.begin(point(137.5625, 221.0));
    builder.cubic_bezier_to(
        point(324.5625, 333.0),
        point(441.5625, 125.0),
        point(316.15625, 114.0),
    );
    builder.cubic_bezier_to(
        point(400.5625, 104.0),
        point(495.5625, 63.0),
        point(580.15625, 99.0),
    );
    builder.end(false);
    let path = builder.build();
    let style = options(ArcsClip::Butt)
        .with_line_width(80.0)
        .with_miter_limit(2.5);
    let standard = mesh(&path, &style);
    let rounded = mesh(&path, &style.with_arcs_clip(ArcsClip::Round));
    let extra: Vec<_> = rounded
        .vertices
        .iter()
        .filter(|p| !standard.vertices.contains(p))
        .collect();
    assert!(!extra.is_empty());
    assert!(
        extra
            .iter()
            .all(|p| p.x < 225.0 && (70.0..160.0).contains(&p.y)),
        "only the marked tip should change: {:?}",
        extra
    );
}

#[test]
fn opposite_parallel_rectangle_gets_a_round_far_edge() {
    let mut builder = Path::builder();
    builder.begin(point(-40.0, 0.0));
    builder.line_to(point(0.0, 0.0));
    builder.cubic_bezier_to(point(-10.0, 0.0), point(-20.0, 5.0), point(-40.0, 5.0));
    builder.end(false);
    let path = builder.build();
    let style = options(ArcsClip::Butt).with_miter_limit(3.0);
    let standard = mesh(&path, &style);
    let rounded = mesh(&path, &style.with_arcs_clip(ArcsClip::Round));
    let old_x = standard
        .vertices
        .iter()
        .map(|p| p.x)
        .fold(f32::NEG_INFINITY, f32::max);
    let new_x = rounded
        .vertices
        .iter()
        .map(|p| p.x)
        .fold(f32::NEG_INFINITY, f32::max);
    assert!(
        (new_x - old_x - 2.0).abs() < 1.0e-4,
        "round rectangle should extend by half its cut width"
    );
}

#[test]
fn svg2_flat_clipping_is_the_default() {
    assert_eq!(StrokeOptions::default().arcs_clip, ArcsClip::Butt);
}

#[cfg(feature = "serialization")]
#[test]
fn old_options_without_clip_setting_load_as_standard_svg2() {
    let json = r#"{"start_cap":"Butt","end_cap":"Round","line_join":"Miter","line_width":8.0,"variable_line_width":null,"miter_limit":4.0,"tolerance":0.1}"#;
    let old: StrokeOptions = serde_json::from_str(json).unwrap();
    assert_eq!(old.arcs_clip, ArcsClip::Butt);
    let rounded = old
        .with_line_join(LineJoin::Arcs)
        .with_arcs_clip(ArcsClip::Round);
    let encoded = serde_json::to_string(&rounded).unwrap();
    assert_eq!(
        serde_json::from_str::<StrokeOptions>(&encoded).unwrap(),
        rounded
    );
}
