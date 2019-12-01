use crate::geom::math::*;
use crate::geometry_builder::*;
use crate::path::builder::{Build, FlatPathBuilder, PathBuilder};
use crate::path::{Path, PathSlice};
use crate::extra::rust_logo::build_logo_path;
use crate::{FillTessellator, TessellationError, FillOptions};

use std::env;

fn tessellate_path(path: PathSlice, log: bool) -> Result<usize, TessellationError> {
    let mut buffers: VertexBuffers<Point, u16> = VertexBuffers::new();
    {
        let options = FillOptions::tolerance(0.05);

        use crate::path::builder::*;
        use crate::path::iterator::*;

        let mut builder = Path::builder();
        for e in path.iter().flattened(0.05) {
            builder.path_event(e);
        }

        let mut vertex_builder = simple_builder(&mut buffers);
        let mut tess = FillTessellator::new();
        tess.set_logging(log);
        tess.tessellate_path(
            &builder.build(),
            &options,
            &mut vertex_builder
        )?;
    }
    return Ok(buffers.indices.len() / 3);
}

#[test]
fn test_too_many_vertices() {
    /// This test checks that the tessellator returns the proper error when
    /// the geometry builder run out of vertex ids.

    struct Builder { max_vertices: u32 }
    impl GeometryBuilder for Builder {
        fn begin_geometry(&mut self) {}
        fn add_triangle(&mut self, _a: VertexId, _b: VertexId, _c: VertexId) {}
        fn end_geometry(&mut self) -> Count { Count { vertices: 0, indices: 0 } }
        fn abort_geometry(&mut self) {}
    }

    impl FillGeometryBuilder for Builder {
        fn add_fill_vertex(&mut self, _pos: Point, _src: &mut dyn Iterator<Item=VertexSource>) -> Result<VertexId, GeometryBuilderError> {
            if self.max_vertices == 0 {
                return Err(GeometryBuilderError::TooManyVertices);
            }
            self.max_vertices -= 1;
            Ok(VertexId(self.max_vertices))
        }
    }

    let mut path = Path::builder().with_svg();
    build_logo_path(&mut path);
    let path = path.build();

    let mut tess = FillTessellator::new();
    let options = FillOptions::tolerance(0.05);

    assert_eq!(
        tess.tessellate_path(&path, &options, &mut Builder { max_vertices: 0 }),
        Err(TessellationError::TooManyVertices),
    );
    assert_eq!(
        tess.tessellate_path(&path, &options, &mut Builder { max_vertices: 10 }),
        Err(TessellationError::TooManyVertices),
    );

    assert_eq!(
        tess.tessellate_path(&path, &options, &mut Builder { max_vertices: 100 }),
        Err(TessellationError::TooManyVertices),
    );
}

fn test_path(path: PathSlice) {
    test_path_internal(path, None);
}


fn test_path_and_count_triangles(path: PathSlice, expected_triangle_count: usize) {
    test_path_internal(path, Some(expected_triangle_count));
}

fn test_path_internal(path: PathSlice, expected_triangle_count: Option<usize>) {
    let add_logging = env::var("LYON_ENABLE_LOGGING").is_ok();
    let find_test_case = env::var("LYON_REDUCED_TESTCASE").is_ok();

    let res = if find_test_case {
        ::std::panic::catch_unwind(|| tessellate_path(path, false))
    } else {
        Ok(tessellate_path(path, false))
    };

    if let Ok(Ok(num_triangles)) = res {
        if let Some(expected_triangles) = expected_triangle_count {
            if num_triangles != expected_triangles {
                tessellate_path(path, add_logging).unwrap();
                panic!("expected {} triangles, got {}", expected_triangles, num_triangles);
            }
        }
        return;
    }

    if find_test_case {
        crate::extra::debugging::find_reduced_test_case(
            path,
            &|path: Path| { return tessellate_path(path.as_slice(), false).is_err(); },
        );

    }

    if add_logging {
        tessellate_path(path, true).unwrap();
    }

    panic!();
}

fn test_path_with_rotations(path: Path, step: f32, expected_triangle_count: Option<usize>) {
    use std::f32::consts::PI;

    let mut angle = 0.0;
    while angle < PI * 2.0 {
        println!("\n\n ==================== angle = {}", angle);

        let mut tranformed_path = path.clone();
        let cos = angle.cos();
        let sin = angle.sin();
        for v in tranformed_path.mut_points() {
            let (x, y) = (v.x, v.y);
            v.x = x * cos + y * sin;
            v.y = y * cos - x * sin;
        }

        test_path_internal(tranformed_path.as_slice(), expected_triangle_count);

        angle += step;
    }
}

#[test]
fn test_simple_triangle() {
    let mut path = Path::builder();
    path.move_to(point(0.0, 0.0));
    path.line_to(point(1.0, 1.0));
    path.line_to(point(0.0, 1.0));
    path.close();

    test_path_with_rotations(path.build(), 0.01, Some(1));
}

#[test]
fn test_simple_monotone() {
    let mut path = Path::builder();
    path.move_to(point(0.0, 0.0));
    path.line_to(point(-1.0, 1.0));
    path.line_to(point(-3.0, 2.0));
    path.line_to(point(-1.0, 3.0));
    path.line_to(point(-4.0, 5.0));
    path.line_to(point(0.0, 6.0));
    path.close();

    let path = path.build();
    test_path_and_count_triangles(path.as_slice(), 4);
}

#[test]
fn test_simple_split() {
    let mut path = Path::builder();
    path.move_to(point(0.0, 0.0));
    path.line_to(point(2.0, 1.0));
    path.line_to(point(2.0, 3.0));
    path.line_to(point(1.0, 2.0));
    path.line_to(point(0.0, 3.0));
    path.close();

    test_path_with_rotations(path.build(), 0.001, Some(3));
}

#[test]
fn test_simple_merge_split() {
    let mut path = Path::builder();
    path.move_to(point(0.0, 0.0));
    path.line_to(point(1.0, 1.0));
    path.line_to(point(2.0, 0.0));
    path.line_to(point(2.0, 3.0));
    path.line_to(point(1.0, 2.0));
    path.line_to(point(0.0, 3.0));
    path.close();

    test_path_with_rotations(path.build(), 0.001, Some(4));

    // "M 0 0 L 1 1 L 2 0 L 1 3 L 0 4 L 0 3 Z"
}

#[test]
fn test_simple_aligned() {
    let mut path = Path::builder();
    path.move_to(point(0.0, 0.0));
    path.line_to(point(1.0, 0.0));
    path.line_to(point(2.0, 0.0));
    path.line_to(point(2.0, 1.0));
    path.line_to(point(2.0, 2.0));
    path.line_to(point(1.0, 2.0));
    path.line_to(point(0.0, 2.0));
    path.line_to(point(0.0, 1.0));
    path.close();

    test_path_with_rotations(path.build(), 0.001, Some(6));
}

#[test]
fn test_simple_1() {
    let mut path = Path::builder();
    path.move_to(point(0.0, 0.0));
    path.line_to(point(1.0, 1.0));
    path.line_to(point(2.0, 0.0));
    path.line_to(point(1.0, 3.0));
    path.line_to(point(0.5, 4.0));
    path.line_to(point(0.0, 3.0));
    path.close();

    test_path_with_rotations(path.build(), 0.001, Some(4));

    // "M 0 0 L 1 1 L 2 0 L 1 3 L 0 4 L 0 3 Z"
}


#[test]
fn test_simple_2() {
    let mut path = Path::builder();
    path.move_to(point(0.0, 0.0));
    path.line_to(point(1.0, 0.0));
    path.line_to(point(2.0, 0.0));
    path.line_to(point(3.0, 0.0));
    path.line_to(point(3.0, 1.0));
    path.line_to(point(3.0, 2.0));
    path.line_to(point(3.0, 3.0));
    path.line_to(point(2.0, 3.0));
    path.line_to(point(1.0, 3.0));
    path.line_to(point(0.0, 3.0));
    path.line_to(point(0.0, 2.0));
    path.line_to(point(0.0, 1.0));
    path.close();

    test_path_with_rotations(path.build(), 0.001, Some(10));
}

#[test]
fn test_hole_1() {
    let mut path = Path::builder();
    path.move_to(point(-11.0, 5.0));
    path.line_to(point(0.0, -5.0));
    path.line_to(point(10.0, 5.0));
    path.close();

    path.move_to(point(-5.0, 2.0));
    path.line_to(point(0.0, -2.0));
    path.line_to(point(4.0, 2.0));
    path.close();

    test_path_with_rotations(path.build(), 0.001, Some(6));
}

#[test]
fn test_degenerate_same_position() {
    let mut path = Path::builder();
    path.move_to(point(0.0, 0.0));
    path.line_to(point(0.0, 0.0));
    path.line_to(point(0.0, 0.0));
    path.line_to(point(0.0, 0.0));
    path.line_to(point(0.0, 0.0));
    path.line_to(point(0.0, 0.0));
    path.close();

    test_path_with_rotations(path.build(), 0.001, None);
}

#[test]
fn test_intersecting_bow_tie() {
    // Simple self-intersecting shape.
    // x  x
    // |\/|
    // |/\|
    // x  x
    let mut path = Path::builder();

    path.move_to(point(0.0, 0.0));
    path.line_to(point(2.0, 2.0));
    path.line_to(point(2.0, 0.0));
    path.line_to(point(0.0, 2.0));
    path.close();

    test_path(path.build().as_slice());
}

#[test]
fn test_auto_intersection_type1() {
    //  o.___
    //   \   'o
    //    \ /
    //     x  <-- intersection!
    //    / \
    //  o.___\
    //       'o
    let mut path = Path::builder();
    path.move_to(point(0.0, 0.0));
    path.line_to(point(2.0, 1.0));
    path.line_to(point(0.0, 2.0));
    path.line_to(point(2.0, 3.0));
    path.close();

    let path = path.build();
    test_path_and_count_triangles(path.as_slice(), 2);
}

#[test]
fn test_auto_intersection_type2() {
    //  o
    //  |\   ,o
    //  | \ / |
    //  |  x  | <-- intersection!
    //  | / \ |
    //  o'   \|
    //        o
    let mut path = Path::builder();
    path.move_to(point(0.0, 0.0));
    path.line_to(point(2.0, 3.0));
    path.line_to(point(2.0, 1.0));
    path.line_to(point(0.0, 2.0));
    path.close();

    let path = path.build();
    test_path_and_count_triangles(path.as_slice(), 2);
}

#[test]
fn test_auto_intersection_multi() {
    //      .
    //  ___/_\___
    //  | /   \ |
    //  |/     \|
    // /|       |\
    // \|       |/
    //  |\     /|
    //  |_\___/_|
    //     \ /
    //      '
    let mut path = Path::builder();
    path.move_to(point(20.0, 20.0));
    path.line_to(point(60.0, 20.0));
    path.line_to(point(60.0, 60.0));
    path.line_to(point(20.0, 60.0));
    path.close();

    path.move_to(point(40.0, 10.0));
    path.line_to(point(70.0, 40.0));
    path.line_to(point(40.0, 70.0));
    path.line_to(point(10.0, 40.0));
    path.close();

    let path = path.build();
    test_path_with_rotations(path, 0.011, Some(8));
}

#[test]
fn three_edges_below() {
    let mut builder = Path::builder();

    //       .
    //      /|
    //     / |
    //    x  |
    //   /|\ |
    //  / | \|
    // /__|  .


    builder.move_to(point(1.0, 0.0));
    builder.line_to(point(0.0, 1.0));
    builder.line_to(point(2.0, 2.0));
    builder.close();
    builder.line_to(point(-1.0, 2.0));
    builder.line_to(point(0.0, 1.0));
    builder.line_to(point(0.0, 2.0));
    builder.close();

    test_path(builder.build().as_slice());
}

#[test]
fn test_rust_logo_no_intersection() {
    let mut path = Path::builder().flattened(0.011).with_svg();

    build_logo_path(&mut path);

    test_path_with_rotations(path.build(), 0.011, None);
}

#[test]
fn test_rust_logo_with_intersection() {
    let mut path = Path::builder().flattened(0.011).with_svg();

    build_logo_path(&mut path);

    path.move_to(point(10.0, 30.0));
    path.line_to(point(130.0, 30.0));
    path.line_to(point(130.0, 60.0));
    path.line_to(point(10.0, 60.0));
    path.close();

    let path = path.build();

    test_path_with_rotations(path, 0.011, None);
}

#[cfg(test)]
fn scale_path(path: &mut Path, scale: f32) {
    for v in path.mut_points() {
        *v = *v * scale;
    }
}

#[test]
fn test_rust_logo_scale_up() {
    // The goal of this test is to check how resistent the tessellator is against integer
    // overflows, and catch regressions.

    let mut builder = Path::builder().with_svg();
    build_logo_path(&mut builder);
    let mut path = builder.build();

    scale_path(&mut path, 260.0);
    test_path(path.as_slice());
}

#[test]
fn test_rust_logo_scale_down() {
    // The goal of this test is to check that the tessellator can handle very small geometry.

    let mut builder = Path::builder().flattened(0.011).with_svg();
    build_logo_path(&mut builder);
    let mut path = builder.build();

    scale_path(&mut path, 0.005);
    test_path(path.as_slice());
}

#[test]
fn test_rust_logo_scale_down2() {
    // Issues with very small paths.

    let mut builder = Path::builder().flattened(0.011).with_svg();
    build_logo_path(&mut builder);
    let mut path = builder.build();

    scale_path(&mut path, 0.0000001);
    test_path(path.as_slice());
}

#[test]
fn test_simple_double_merge() {
    // This test triggers the code path where a merge event is resolved during another
    // merge event.
    //     / \ /
    //  \ / .-x    <-- merge vertex
    //   x-'      <-- current merge vertex
    let mut path = Path::builder();

    path.move_to(point(0.0, 2.0));
    path.line_to(point(1.0, 3.0));
    path.line_to(point(2.0, 0.0));
    path.line_to(point(3.0, 2.0));
    path.line_to(point(4.0, 1.0));
    path.line_to(point(2.0, 6.0));
    path.close();

    // "M 0 2 L 1 3 L 2 0 L 3 2 L 4 1 L 2 6 Z"
}

#[test]
fn test_double_merge_with_intersection() {
    // This test triggers the code path where a merge event is resolved during another
    // merge event.
    //     / \ /
    //  \ / .-x    <-- merge vertex
    //   x-'      <-- current merge vertex
    //
    // The test case generated from a reduced rotation of
    // test_rust_logo_with_intersection and has a self-intersection.
    let mut path = Path::builder();

    path.move_to(point(80.041534, 19.24472));
    path.line_to(point(76.56131, 23.062233));
    path.line_to(point(67.26949, 23.039438));
    path.line_to(point(65.989944, 23.178522));
    path.line_to(point(59.90927, 19.969215));
    path.line_to(point(56.916714, 25.207449));
    path.line_to(point(50.333813, 23.25274));
    path.line_to(point(48.42367, 28.978098));
    path.close();
    path.move_to(point(130.32213, 28.568213));
    path.line_to(point(130.65213, 58.5664));
    path.line_to(point(10.659382, 59.88637));
    path.close();

    test_path(path.build().as_slice());
    // "M 80.041534 19.24472 L 76.56131 23.062233 L 67.26949 23.039438 L 65.989944 23.178522 L 59.90927 19.969215 L 56.916714 25.207449 L 50.333813 23.25274 L 48.42367 28.978098 M 130.32213, 28.568213 L 130.65213 58.5664 L 10.659382 59.88637 Z"
}

#[test]
fn test_chained_merge_end() {
    // This test creates a succession of merge events that need to be resolved during
    // an end event.
    // |\/\  /\    /|  <-- merge
    // \   \/  \  / /  <-- merge
    //  \       \/ /   <-- merge
    //   \        /
    //    \      /
    //     \    /
    //      \  /
    //       \/        < -- end
    let mut path = Path::builder();

    path.move_to(point(1.0, 0.0));
    path.line_to(point(2.0, 1.0)); // <-- merge
    path.line_to(point(3.0, 0.0));
    path.line_to(point(4.0, 2.0)); // <-- merge
    path.line_to(point(5.0, 0.0));
    path.line_to(point(6.0, 3.0)); // <-- merge
    path.line_to(point(7.0, 0.0));
    path.line_to(point(5.0, 8.0)); // <-- end
    path.close();

    test_path_and_count_triangles(path.build().as_slice(), 6);
}

#[test]
fn test_chained_merge_left() {
    // This test creates a succession of merge events that need to be resolved during
    // a left event.
    // |\/\  /\    /|  <-- merge
    // |   \/  \  / |  <-- merge
    // |        \/  |  <-- merge
    // |            |
    //  \           |  <-- left
    //   \          |
    let mut path = Path::builder();

    path.move_to(point(1.0, 0.0));
    path.line_to(point(2.0, 1.0)); // <-- merge
    path.line_to(point(3.0, 0.0));
    path.line_to(point(4.0, 2.0)); // <-- merge
    path.line_to(point(5.0, 0.0));
    path.line_to(point(6.0, 3.0)); // <-- merge
    path.line_to(point(7.0, 0.0));
    path.line_to(point(7.0, 5.0));
    path.line_to(point(0.0, 4.0)); // <-- left
    path.close();

    test_path_and_count_triangles(path.build().as_slice(), 7);
}

#[test]
fn test_chained_merge_merge() {
    // This test creates a succession of merge events that need to be resolved during
    // another merge event.
    //      /\/\  /\    /|  <-- merge
    //     /    \/  \  / |  <-- merge
    //    /          \/  |  <-- merge
    // |\/               |  <-- merge (resolving)
    // |_________________|
    let mut path = Path::builder();

    path.move_to(point(1.0, 0.0));
    path.line_to(point(2.0, 1.0)); // <-- merge
    path.line_to(point(3.0, 0.0));
    path.line_to(point(4.0, 2.0)); // <-- merge
    path.line_to(point(5.0, 0.0));
    path.line_to(point(6.0, 3.0)); // <-- merge
    path.line_to(point(7.0, 0.0));
    path.line_to(point(7.0, 5.0));
    path.line_to(point(-1.0, 5.0));
    path.line_to(point(-1.0, 0.0));
    path.line_to(point(0.0, 4.0)); // <-- merge (resolving)
    path.close();

    test_path_and_count_triangles(path.build().as_slice(), 9);
}

#[test]
fn test_chained_merge_split() {
    // This test creates a succession of merge events that need to be resolved during
    // a split event.
    // |\/\  /\    /|  <-- merge
    // |   \/  \  / |  <-- merge
    // |        \/  |  <-- merge
    // |            |
    // |     /\     |  <-- split
    let mut path = Path::builder();

    path.move_to(point(1.0, 0.0));
    path.line_to(point(2.0, 1.0)); // <-- merge
    path.line_to(point(3.0, 0.0));
    path.line_to(point(4.0, 2.0)); // <-- merge
    path.line_to(point(5.0, 0.0));
    path.line_to(point(6.0, 3.0)); // <-- merge
    path.line_to(point(7.0, 0.0));
    path.line_to(point(7.0, 5.0));
    path.line_to(point(4.0, 4.0)); // <-- split
    path.line_to(point(1.0, 5.0));
    path.close();

    test_path_and_count_triangles(path.build().as_slice(), 8);

    // "M 1 0 L 2 1 L 3 0 L 4 2 L 5 0 L 6 3 L 7 0 L 7 5 L 4 4 L 1 5 Z"
}

// TODO: Check that chained merge events can't mess with the way we handle complex events.

#[test]
fn test_intersection_horizontal_precision() {
    // TODO make a cleaner test case exercising the same edge case.
    // This test has an almost horizontal segment e1 going from right to left intersected
    // by another segment e2. Since e1 is almost horizontal the intersection point ends up
    // with the same y coordinate and at the left of the current position when it is found.
    // The difficulty is that the intersection is therefore technically "above" the current
    // position, but we can't allow that because the ordering of the events is a strong
    // invariant of the algorithm.
    let mut builder = Path::builder();

    builder.move_to(point(-34.619564, 111.88655));
    builder.line_to(point(-35.656174, 111.891));
    builder.line_to(point(-39.304527, 121.766914));
    builder.close();

    builder.move_to(point(1.4426613, 133.40884));
    builder.line_to(point(-27.714422, 140.47032));
    builder.line_to(point(-55.960342, 23.841988));
    builder.close();

    test_path(builder.build().as_slice());
}

#[test]
fn test_overlapping_with_intersection() {
    // There are two overlapping segments a-b and b-a intersecting a segment
    // c-d.
    // This test used to fail until overlapping edges got dealt with before
    // intersection detection. The issue was that the one of the overlapping
    // edges would intersect properly and the second would end up in the degenerate
    // case where it would pass though a pain between two segments without
    // registering as an intersection.
    //
    //       a
    //     / | \
    //    c--|--d
    //       |
    //       b

    let mut builder = Path::builder();

    builder.move_to(point(2.0, -1.0));
    builder.line_to(point(2.0, -3.0));
    builder.line_to(point(3.0, -2.0));
    builder.line_to(point(1.0, -2.0));
    builder.line_to(point(2.0, -3.0));
    builder.close();

    test_path(builder.build().as_slice());
}

#[test]
fn test_split_with_intersections() {
    // This is a reduced test case that was showing a bug where duplicate intersections
    // were found during a split event, due to the sweep line beeing into a temporarily
    // inconsistent state when insert_edge was called.

    let mut builder = Path::builder();

    builder.move_to(point(-21.004179, -71.57515));
    builder.line_to(point(-21.927473, -70.94977));
    builder.line_to(point(-23.024633, -70.68942));
    builder.close();
    builder.move_to(point(16.036617, -27.254852));
    builder.line_to(point(-62.83691, -117.69249));
    builder.line_to(point(38.646027, -46.973236));
    builder.close();

    let path = builder.build();

    test_path(path.as_slice());
}

#[test]
fn test_colinear_1() {
    let mut builder = Path::builder();
    builder.move_to(point(20.0, 150.0));
    builder.line_to(point(80.0, 150.0));
    builder.close();

    let path = builder.build();

    test_path_with_rotations(path, 0.01, None);
}

#[test]
fn test_colinear_2() {
    let mut builder = Path::builder();
    builder.move_to(point(20.0, 150.0));
    builder.line_to(point(80.0, 150.0));
    builder.line_to(point(20.0, 150.0));
    builder.close();

    let path = builder.build();

    test_path_with_rotations(path, 0.01, None);
}

#[test]
fn test_colinear_3() {
    let mut builder = Path::builder();
    // The path goes through many points along a line.
    builder.move_to(point(0.0, 1.0));
    builder.line_to(point(0.0, 3.0));
    builder.line_to(point(0.0, 5.0));
    builder.line_to(point(0.0, 4.0));
    builder.line_to(point(0.0, 2.0));
    builder.close();

    let path = builder.build();

    test_path(path.as_slice());
}

#[test]
fn test_colinear_4() {
    // The path goes back and forth along a line.
    let mut builder = Path::builder();
    builder.move_to(point(0.0, 2.0));
    builder.line_to(point(0.0, 1.0));
    builder.line_to(point(0.0, 3.0));
    builder.line_to(point(0.0, 0.0));
    builder.close();

    let path = builder.build();

    test_path(path.as_slice());
}

#[test]
fn test_colinear_touching_squares() {
    // Two squares touching.
    //
    // x-----x-----x
    // |     |     |
    // |     |     |
    // x-----x-----x
    //
    let mut builder = Path::builder();
    builder.move_to(point(0.0, 0.0));
    builder.line_to(point(1.0, 0.0));
    builder.line_to(point(1.0, 1.0));
    builder.line_to(point(0.0, 1.0));

    builder.move_to(point(1.0, 0.0));
    builder.line_to(point(2.0, 0.0));
    builder.line_to(point(2.0, 1.0));
    builder.line_to(point(1.0, 1.0));

    builder.close();

    let path = builder.build();

    test_path(path.as_slice());
}

#[test]
fn angle_precision() {
    // This test case has some edges that are almost parallel and the
    // imprecision of the angle computation causes them to be in the
    // wrong order in the sweep line.
    let mut builder = Path::builder();

    builder.move_to(point(0.007982401, 0.0121872));
    builder.line_to(point(0.008415101, 0.0116545));
    builder.line_to(point(0.008623006, 0.011589845));
    builder.line_to(point(0.008464893, 0.011639819));
    builder.line_to(point(0.0122631, 0.0069716));
    builder.close();

    test_path(builder.build().as_slice());
}

#[test]
fn n_segments_intersecting() {
    use std::f32::consts::PI;

    // This test creates a lot of segments that intersect at the same
    // position (center). Very good at finding precision issues.

    for i in 1..10 {
        let mut builder = Path::builder();

        let center = point(-2.0, -5.0);
        let n = i * 4 - 1;
        let delta = PI / n as f32;
        let mut radius = 1000.0;
        builder.move_to(center + vector(radius, 0.0));
        builder.line_to(center - vector(-radius, 0.0));
        for i in 0..n {
            let (s, c) = (i as f32 * delta).sin_cos();
            builder.line_to(center + vector(c, s) * radius);
            builder.line_to(center - vector(c, s) * radius);
            radius = -radius;
        }
        builder.close();

        test_path_with_rotations(builder.build(), 0.03, None);
    }
}

#[test]
fn back_along_previous_edge() {
    // This test has edges that come back along the previous edge.
    let mut builder = Path::builder();

    builder.move_to(point(0.0, 0.0));
    builder.line_to(point(1.0, 1.0));
    builder.line_to(point(0.8, 0.8));
    builder.line_to(point(1.5, 1.5));
    builder.close();

    test_path(builder.build().as_slice());
}

#[test]
fn test_colinear_touching_squares2() {
    // Two squares touching.
    //
    // x-----x
    // |     x-----x
    // |     |     |
    // x-----x     |
    //       x-----x
    //
    let mut builder = Path::builder();
    builder.move_to(point(0.0, 0.0));
    builder.line_to(point(10.0, 0.0));
    builder.line_to(point(10.0, 10.0));
    builder.line_to(point(0.0, 10.0));

    builder.move_to(point(10.0, 1.0));
    builder.line_to(point(20.0, 1.0));
    builder.line_to(point(20.0, 11.0));
    builder.line_to(point(10.0, 11.0));

    builder.close();

    let path = builder.build();

    test_path(path.as_slice());
}

#[test]
fn test_colinear_touching_squares3() {
    // Two squares touching.
    //
    //       x-----x
    // x-----x     |
    // |     |     |
    // |     x-----x
    // x-----x
    //
    let mut builder = Path::builder();
    builder.move_to(point(0.0, 1.0));
    builder.line_to(point(10.0, 1.0));
    builder.line_to(point(10.0, 11.0));
    builder.line_to(point(0.0, 11.0));

    builder.move_to(point(10.0, 0.0));
    builder.line_to(point(20.0, 0.0));
    builder.line_to(point(20.0, 10.0));
    builder.line_to(point(10.0, 10.0));

    builder.close();

    let path = builder.build();

    test_path(path.as_slice());
}


#[test]
fn test_unknown_issue_1() {
    // This test case used to fail but does not fail anymore, probably thanks to
    // the fixed-to-f32 workaround (cf.) test_fixed_to_f32_precision.
    // TODO: figure out what the issue was and what fixed it.
    let mut builder = Path::builder();

    builder.move_to(point(-3.3709216, 9.467676));
    builder.line_to(point(-13.078612, 7.0675235));
    builder.line_to(point(-10.67846, -2.6401677));
    builder.close();

    builder.move_to(point(-4.800305, 19.415382));
    builder.line_to(point(-14.507996, 17.01523));
    builder.line_to(point(-12.107843, 7.307539));
    builder.close();

    test_path(builder.build().as_slice());
}

#[test]
fn test_colinear_touching_squares_rotated() {
    // Two squares touching.
    //
    //       x-----x
    // x-----x     |
    // |     |     |
    // |     x-----x
    // x-----x
    //
    let mut builder = Path::builder();
    builder.move_to(point(0.0, 1.0));
    builder.line_to(point(10.0, 1.0));
    builder.line_to(point(10.0, 11.0));
    builder.line_to(point(0.0, 11.0));

    builder.move_to(point(10.0, 0.0));
    builder.line_to(point(20.0, 0.0));
    builder.line_to(point(20.0, 10.0));
    builder.line_to(point(10.0, 10.0));

    builder.close();

    let path = builder.build();

    test_path_with_rotations(path, 0.01, None)
}

#[test]
fn test_point_on_edge_right() {
    //     a
    //    /|
    //   / x  <--- point exactly on edge ab
    //  / /|\
    // x-' | \
    //     b--x
    //
    let mut builder = Path::builder();
    builder.move_to(point(0.0, 0.0));
    builder.line_to(point(0.0, 100.0));
    builder.line_to(point(50.0, 100.0));
    builder.line_to(point(0.0, 50.0));
    builder.line_to(point(-50.0, 100.0));
    builder.close();

    let path = builder.build();

    test_path(path.as_slice());
}

#[test]
fn test_point_on_edge_left() {
    //     a
    //     |\
    //     x \  <--- point exactly on edge ab
    //    /|\ \
    //   / | `-x
    //  x--b
    //
    let mut builder = Path::builder();
    builder.move_to(point(0.0, 0.0));
    builder.line_to(point(0.0, 100.0));
    builder.line_to(point(-50.0, 100.0));
    builder.line_to(point(0.0, 50.0));
    builder.line_to(point(50.0, 100.0));
    builder.close();

    let path = builder.build();

    test_path(path.as_slice());
}

#[test]
fn test_point_on_edge2() {
    // Point b (from edges a-b and b-c) is positionned exactly on
    // the edge d-e.
    //
    //     d-----+
    //     |     |
    //  a--b--c  |
    //  |  |  |  |
    //  +-----+  |
    //     |     |
    //     e-----+
    let mut builder = Path::builder();

    builder.move_to(point(1.0, 1.0));
    builder.line_to(point(2.0, 1.0));
    builder.line_to(point(3.0, 1.0));
    builder.line_to(point(3.0, 2.0));
    builder.line_to(point(1.0, 2.0));
    builder.close();

    builder.move_to(point(2.0, 0.0));
    builder.line_to(point(2.0, 3.0));
    builder.line_to(point(4.0, 3.0));
    builder.line_to(point(4.0, 0.0));
    builder.close();

    test_path(builder.build().as_slice());
}

#[test]
fn test_coincident_simple_1() {
    // 0___5
    //  \ /
    // 1 x 4
    //  /_\
    // 2   3

    // A self-intersecting path with two points at the same position.
    let mut builder = Path::builder();
    builder.move_to(point(0.0, 0.0));
    builder.line_to(point(1.0, 1.0)); // <--
    builder.line_to(point(0.0, 2.0));
    builder.line_to(point(2.0, 2.0));
    builder.line_to(point(1.0, 1.0)); // <--
    builder.line_to(point(2.0, 0.0));
    builder.close();

    let path = builder.build();

    test_path(path.as_slice());

    // "M 0 0 L 1 1 L 0 2 L 2 2 L 1 1 L 2 0 Z"
}


#[test]
fn test_coincident_simple_2() {
    // A self-intersecting path with two points at the same position.
    let mut builder = Path::builder();
    builder.move_to(point(0.0, 0.0));
    builder.line_to(point(1.0, 1.0)); // <--
    builder.line_to(point(2.0, 0.0));
    builder.line_to(point(2.0, 2.0));
    builder.line_to(point(1.0, 1.0)); // <--
    builder.line_to(point(0.0, 2.0));
    builder.close();

    let path = builder.build();

    test_path(path.as_slice());
}

#[test]
fn test_coincident_simple_rotated() {
    // Same as test_coincident_simple with the usual rotations
    // applied.
    let mut builder = Path::builder();
    builder.move_to(point(0.0, 0.0));
    builder.line_to(point(1.0, 1.0)); // <--
    builder.line_to(point(0.0, 2.0));
    builder.line_to(point(2.0, 2.0));
    builder.line_to(point(1.0, 1.0)); // <--
    builder.line_to(point(2.0, 0.0));
    builder.close();

    let path = builder.build();

    test_path_with_rotations(path, 0.01, None);
}

#[test]
fn test_identical_squares() {
    // Two identical sub paths. It is pretty much the worst type of input for
    // the tessellator as far as I know.
    let mut builder = Path::builder();
    builder.move_to(point(0.0, 0.0));
    builder.line_to(point(1.0, 0.0));
    builder.line_to(point(1.0, 1.0));
    builder.line_to(point(0.0, 1.0));
    builder.close();
    builder.move_to(point(0.0, 0.0));
    builder.line_to(point(1.0, 0.0));
    builder.line_to(point(1.0, 1.0));
    builder.line_to(point(0.0, 1.0));
    builder.close();

    let path = builder.build();

    test_path(path.as_slice());
}

#[test]
fn test_close_at_first_position() {
    // This path closes at the first position which requires some special handling in the event
    // builder in order to properly add the last vertex events (since first == current, we can't
    // test against the angle of (current, first, second)).
    let mut builder = Path::builder();

    builder.move_to(point(107.400665, 91.79798));
    builder.line_to(point(108.93136, 91.51076));
    builder.line_to(point(107.84248, 91.79686));
    builder.line_to(point(107.400665, 91.79798));
    builder.close();

    test_path(builder.build().as_slice());
}

#[test]
fn test_fixed_to_f32_precision() {
    // This test appears to hit a precision issue in the conversion from fixed 16.16
    // to f32, causing a point to appear slightly above another when it should not.
    let mut builder = Path::builder();

    builder.move_to(point(68.97998, 796.05));
    builder.line_to(point(61.27998, 805.35));
    builder.line_to(point(55.37999, 799.14996));
    builder.line_to(point(68.98, 796.05));
    builder.close();

    test_path(builder.build().as_slice());
}

#[test]
fn test_no_close() {
    let mut builder = Path::builder();

    builder.move_to(point(1.0, 1.0));
    builder.line_to(point(5.0, 1.0));
    builder.line_to(point(1.0, 5.0));

    test_path(builder.build().as_slice());
}

#[test]
fn test_empty_path() {
    test_path_and_count_triangles(Path::new().as_slice(), 0);
}

#[test]
fn test_exp_no_intersection_01() {
    let mut builder = Path::builder();

    builder.move_to(point(80.041534, 19.24472));
    builder.line_to(point(76.56131, 23.062233));
    builder.line_to(point(67.26949, 23.039438));
    builder.line_to(point(48.42367, 28.978098));
    builder.close();

    test_path(builder.build().as_slice());

    // SVG path syntax:
    // "M 80.041534 19.24472 L 76.56131 23.062233 L 67.26949 23.039438 L 48.42367 28.978098 Z"
}


#[test]
fn test_intersecting_star_shape() {
    let mut builder = Path::builder();
    builder.move_to(point(0.0, 0.0));
    builder.line_to(point(100.0, 0.0));
    builder.line_to(point(50.0, 50.0));
    builder.close();
    builder.move_to(point(0.0, 25.0));
    builder.line_to(point(100.0, 25.0));
    builder.line_to(point(50.0, -25.0));
    builder.close();

    let path = builder.build();

    test_path_with_rotations(path, 0.01, None);
}

#[test]
fn issue_476_original() {
    let mut builder = Path::builder();

    builder.move_to(point(10720.101,7120.1816));
    builder.line_to(point(10720.099,7120.1816));
    builder.line_to(point(10720.1,7120.182));
    builder.line_to(point(10720.099,7120.1836));
    builder.line_to(point(10720.101,7120.1846));
    builder.line_to(point(10720.098,7120.1855));
    builder.line_to(point(10720.096,7120.189));
    builder.line_to(point(10720.096,7120.1885));
    builder.line_to(point(10720.094,7120.188));
    builder.line_to(point(10720.095,7120.1885));
    builder.line_to(point(10720.095,7120.1885));
    builder.line_to(point(10720.094,7120.189));
    builder.line_to(point(10720.095,7120.1885));
    builder.line_to(point(10720.091,7120.1865));
    builder.line_to(point(10720.096,7120.1855));
    builder.line_to(point(10720.097,7120.1836));
    builder.line_to(point(10720.098,7120.1846));
    builder.line_to(point(10720.099,7120.1816));
    builder.line_to(point(10720.098,7120.1826));
    builder.line_to(point(10720.097,7120.181));
    builder.line_to(point(10720.1,7120.1807));
    builder.close();

    test_path(builder.build().as_slice());
}

#[test]
fn issue_476_reduced() {
    let mut builder = Path::builder();

    builder.move_to(point(10720.101, 7120.1816));
    builder.line_to(point(10720.099, 7120.1816));
    builder.line_to(point(10720.096, 7120.1855));
    builder.line_to(point(10720.098, 7120.1846));
    builder.line_to(point(10720.099, 7120.1816));
    builder.line_to(point(10720.098, 7120.1826));
    builder.line_to(point(10720.097, 7120.181));
    builder.close();

    test_path(builder.build().as_slice());

    // SVG path syntax:
    // "M 10720.101 7120.1816 L 10720.099 7120.1816 L 10720.096 7120.1855 L 10720.098 7120.1846 L 10720.099 7120.1816 L 10720.098 7120.1826 L 10720.097 7120.181 Z"
}

#[test]
fn issue_481_original() {
    let mut builder = Path::builder();

    builder.move_to(point(0.9177246,0.22070313));
    builder.line_to(point(0.9111328,0.21826172));
    builder.line_to(point(0.91625977,0.22265625));
    builder.line_to(point(0.9111328,0.22753906));
    builder.line_to(point(0.9309082,0.2397461));
    builder.line_to(point(0.92163086,0.24121094));
    builder.line_to(point(0.91796875,0.23486328));
    builder.line_to(point(0.91845703,0.23999023));
    builder.line_to(point(0.90649414,0.24633789));
    builder.line_to(point(0.9038086,0.23022461));
    builder.line_to(point(0.89575195,0.23779297));
    builder.line_to(point(0.88671875,0.23583984));
    builder.line_to(point(0.88427734,0.2277832));
    builder.line_to(point(0.88671875,0.22143555));
    builder.line_to(point(0.8964844,0.21972656));
    builder.line_to(point(0.904541,0.22460938));
    builder.line_to(point(0.9111328,0.21459961));
    builder.line_to(point(0.907959,0.24072266));
    builder.line_to(point(0.9094238,0.24169922));
    builder.line_to(point(0.9104004,0.24047852));
    builder.line_to(point(0.9111328,0.23950195));
    builder.line_to(point(0.91674805,0.24047852));
    builder.line_to(point(0.91259766,0.23803711));
    builder.line_to(point(0.8864746,0.22998047));
    builder.line_to(point(0.88793945,0.22998047));
    builder.line_to(point(0.8874512,0.22827148));
    builder.line_to(point(0.8852539,0.2265625));
    builder.line_to(point(0.8864746,0.22924805));
    builder.line_to(point(0.8869629,0.22607422));
    builder.line_to(point(0.88793945,0.22827148));
    builder.line_to(point(0.8894043,0.22729492));
    builder.line_to(point(0.8869629,0.22607422));
    builder.line_to(point(0.8918457,0.22680664));
    builder.line_to(point(0.89453125,0.2265625));
    builder.line_to(point(0.89282227,0.22558594));
    builder.line_to(point(0.8911133,0.2241211));
    builder.line_to(point(0.8898926,0.22436523));
    builder.line_to(point(0.89038086,0.22558594));
    builder.line_to(point(0.9238281,0.23022461));
    builder.line_to(point(0.9213867,0.23022461));
    builder.line_to(point(0.91918945,0.22729492));
    builder.line_to(point(0.92211914,0.22680664));
    builder.close();

    test_path(builder.build().as_slice());
}

#[test]
fn issue_481_reduced() {
    let mut builder = Path::builder();

    builder.move_to(point(0.88427734, 0.2277832));
    builder.line_to(point(0.88671875, 0.22143555));
    builder.line_to(point(0.91259766, 0.23803711));
    builder.line_to(point(0.8869629, 0.22607422));
    builder.line_to(point(0.88793945, 0.22827148));
    builder.line_to(point(0.8894043, 0.22729492));
    builder.line_to(point(0.8869629, 0.22607422));
    builder.line_to(point(0.89453125, 0.2265625));
    builder.close();

    test_path(builder.build().as_slice());

    // SVG path syntax:
    // "M 0.88427734 0.2277832 L 0.88671875 0.22143555 L 0.91259766 0.23803711 L 0.8869629 0.22607422 L 0.88793945 0.22827148 L 0.8894043 0.22729492 L 0.8869629 0.22607422 L 0.89453125 0.2265625 Z"
}

#[test]
fn overlapping_horizontal() {
    let mut builder = Path::builder();

    builder.move_to(point(10.0, 0.0));
    builder.line_to(point(0.0, 0.0));
    builder.line_to(point(15.0, 0.0));
    builder.line_to(point(10.0, 5.0));
    builder.close();

    test_path(builder.build().as_slice());

    // SVG path syntax:
    // "M 10 0 L 0 0 L 15 0 L 10 5 Z"
}

#[test]
fn triangle() {
    let mut builder = Path::builder();
    builder.move_to(point(0.0, 0.0));
    builder.line_to(point(5.0, 1.0));
    builder.line_to(point(3.0, 5.0));
    builder.close();

    let path = builder.build();

    let mut tess = FillTessellator::new();
    tess.set_logging(true);

    let mut buffers: VertexBuffers<Point, u16> = VertexBuffers::new();

    tess.tessellate_path(
        &path,
        &FillOptions::default(),
        &mut simple_builder(&mut buffers),
    ).unwrap();
}

#[test]
fn new_tess_1() {
    let mut builder = Path::builder();
    builder.move_to(point(0.0, 0.0));
    builder.line_to(point(5.0, 0.0));
    builder.line_to(point(5.0, 5.0));
    builder.line_to(point(0.0, 5.0));
    builder.close();
    builder.move_to(point(1.0, 1.0));
    builder.line_to(point(4.0, 1.0));
    builder.line_to(point(4.0, 4.0));
    builder.line_to(point(1.0, 4.0));
    builder.close();

    let path = builder.build();

    let mut tess = FillTessellator::new();

    let mut buffers: VertexBuffers<Point, u16> = VertexBuffers::new();

    tess.tessellate_path(
        &path,
        &FillOptions::default(),
        &mut simple_builder(&mut buffers),
    ).unwrap();
}

#[test]
fn new_tess_2() {

    let mut builder = Path::builder();
    builder.move_to(point(0.0, 0.0));
    builder.line_to(point(5.0, -5.0));
    builder.line_to(point(10.0, 0.0));
    builder.line_to(point(9.0, 5.0));
    builder.line_to(point(10.0, 10.0));
    builder.line_to(point(5.0, 6.0));
    builder.line_to(point(0.0, 10.0));
    builder.line_to(point(1.0, 5.0));
    builder.close();

    builder.move_to(point(20.0, -1.0));
    builder.line_to(point(25.0, 1.0));
    builder.line_to(point(25.0, 9.0));
    builder.close();


    let path = builder.build();

    let mut tess = FillTessellator::new();

    let mut buffers: VertexBuffers<Point, u16> = VertexBuffers::new();

    tess.tessellate_path(
        &path,
        &FillOptions::default(),
        &mut simple_builder(&mut buffers),
    ).unwrap();
}

#[test]
fn new_tess_merge() {

    let mut builder = Path::builder();
    builder.move_to(point(0.0, 0.0));  // start
    builder.line_to(point(5.0, 5.0));  // merge
    builder.line_to(point(5.0, 1.0));  // start
    builder.line_to(point(10.0, 6.0)); // merge
    builder.line_to(point(11.0, 2.0)); // start
    builder.line_to(point(11.0, 10.0));// end
    builder.line_to(point(0.0, 9.0));  // left
    builder.close();

    let path = builder.build();

    let mut tess = FillTessellator::new();

    let mut buffers: VertexBuffers<Point, u16> = VertexBuffers::new();

    tess.tessellate_path(
        &path,
        &FillOptions::default(),
        &mut simple_builder(&mut buffers),
    ).unwrap();

    // "M 0 0 L 5 5 L 5 1 L 10 6 L 11 2 L 11 10 L 0 9 Z"
}

#[test]
fn test_intersection_1() {
    let mut builder = Path::builder();

    builder.move_to(point(118.82771, 64.41283));
    builder.line_to(point(23.451895, 50.336365));
    builder.line_to(point(123.39044, 68.36287));
    builder.close();

    builder.move_to(point(80.39975, 58.73177));
    builder.line_to(point(80.598236, 60.38033));
    builder.line_to(point(63.05017, 63.488304));
    builder.close();

    let path = builder.build();

    let mut tess = FillTessellator::new();

    let mut buffers: VertexBuffers<Point, u16> = VertexBuffers::new();

    tess.tessellate_path(
        &path,
        &FillOptions::default(),
        &mut simple_builder(&mut buffers),
    ).unwrap();

    // SVG path syntax:
    // "M 118.82771 64.41283 L 23.451895 50.336365 L 123.39044 68.36287 ZM 80.39975 58.73177 L 80.598236 60.38033 L 63.05017 63.488304 Z"
}

#[test]
fn new_tess_points_too_close() {
    // The first and last point are almost equal but not quite.

    let mut builder = Path::builder();

    builder.move_to(point(52.90753, -72.15962));
    builder.line_to(point(45.80301, -70.96051));
    builder.line_to(point(50.91391, -83.96548));
    builder.line_to(point(52.90654, -72.159454));
    builder.close();

    let mut tess = FillTessellator::new();
    let mut buffers: VertexBuffers<Point, u16> = VertexBuffers::new();

    tess.tessellate_path(
        &builder.build(),
        &FillOptions::default(),
        &mut simple_builder(&mut buffers),
    ).unwrap();

    // SVG path syntax:
    // "M 52.90753 -72.15962 L 45.80301 -70.96051 L 50.91391 -83.96548 L 52.90654 -72.159454 Z"
}

#[test]
fn new_tess_coincident_simple() {
    let mut builder = Path::builder();

    builder.move_to(point(0.0, 0.0));
    builder.line_to(point(0.0, 0.0));
    builder.line_to(point(1.0, 0.0));
    builder.line_to(point(1.0, 0.0));
    builder.line_to(point(0.0, 1.0));
    builder.line_to(point(0.0, 1.0));
    builder.close();

    let mut tess = FillTessellator::new();

    let mut buffers: VertexBuffers<Point, u16> = VertexBuffers::new();

    tess.tessellate_path(
        &builder.build(),
        &FillOptions::default(),
        &mut simple_builder(&mut buffers),
    ).unwrap();
}

#[test]
fn new_tess_overlapping_1() {
    let mut builder = Path::builder();

    builder.move_to(point(0.0, 0.0));
    builder.line_to(point(2.0, 2.0));
    builder.line_to(point(3.0, 1.0));
    builder.line_to(point(0.0, 4.0));
    builder.close();

    let mut tess = FillTessellator::new();

    let mut buffers: VertexBuffers<Point, u16> = VertexBuffers::new();

    tess.tessellate_path(
        &builder.build(),
        &FillOptions::default(),
        &mut simple_builder(&mut buffers),
    ).unwrap();
}

#[test]
fn reduced_test_case_01() {
    let mut builder = Path::builder();

    builder.move_to(point(0.73951757, 0.3810749));
    builder.line_to(point(0.4420668, 0.05925262));
    builder.line_to(point(0.54023945, 0.16737175));
    builder.line_to(point(0.8839954, 0.39966547));
    builder.line_to(point(0.77066493, 0.67880523));
    builder.line_to(point(0.48341691, 0.09270251));
    builder.line_to(point(0.053493023, 0.18919432));
    builder.line_to(point(0.6088793, 0.57187665));
    builder.line_to(point(0.2899257, 0.09821439));
    builder.close();

    let mut tess = FillTessellator::new();

    let mut buffers: VertexBuffers<Point, u16> = VertexBuffers::new();

    tess.tessellate_path(
        &builder.build(),
        &FillOptions::default(),
        &mut simple_builder(&mut buffers),
    ).unwrap();

    // SVG path syntax:
    // "M 0.73951757 0.3810749 L 0.4420668 0.05925262 L 0.54023945 0.16737175 L 0.8839954 0.39966547 L 0.77066493 0.67880523 L 0.48341691 0.09270251 L 0.053493023 0.18919432 L 0.6088793 0.57187665 L 0.2899257 0.09821439 Z"
}

#[test]
fn reduced_test_case_02() {
    let mut builder = Path::builder();

    builder.move_to(point(-849.0441, 524.5503));
    builder.line_to(point(857.67084, -518.10205));
    builder.line_to(point(900.9668, -439.50897));
    builder.line_to(point(-892.3401, 445.9572));
    builder.line_to(point(-478.20224, -872.66327));
    builder.line_to(point(486.82892, 879.1116));
    builder.line_to(point(406.3725, 918.8378));
    builder.line_to(point(-397.74573, -912.3896));
    builder.line_to(point(-314.0522, -944.7439));
    builder.line_to(point(236.42209, 975.91394));
    builder.line_to(point(-227.79541, -969.4657));
    builder.line_to(point(-139.66971, -986.356));
    builder.line_to(point(148.29639, 992.80426));
    builder.line_to(point(-50.38492, -995.2788));
    builder.line_to(point(39.340546, -996.16223));
    builder.line_to(point(-30.713806, 1002.6105));
    builder.line_to(point(-120.157104, 995.44745));
    builder.line_to(point(128.78381, -988.9992));
    builder.line_to(point(217.22491, -973.84735));
    builder.line_to(point(-208.5982, 980.2956));
    builder.line_to(point(303.95184, -950.8286));
    builder.line_to(point(388.26636, -920.12854));
    builder.line_to(point(-379.63965, 926.5768));
    builder.line_to(point(-460.8624, 888.4425));
    builder.line_to(point(469.48914, -881.99426));
    builder.line_to(point(546.96686, -836.73254));
    builder.line_to(point(-538.3402, 843.1808));
    builder.close();

    let mut tess = FillTessellator::new();

    let mut buffers: VertexBuffers<Point, u16> = VertexBuffers::new();

    tess.tessellate_path(
        &builder.build(),
        &FillOptions::default(),
        &mut simple_builder(&mut buffers),
    ).unwrap();

    // SVG path syntax:
    // "M -849.0441 524.5503 L 857.67084 -518.10205 L 900.9668 -439.50897 L -892.3401 445.9572 L -478.20224 -872.66327 L 486.82892 879.1116 L 406.3725 918.8378 L -397.74573 -912.3896 L -314.0522 -944.7439 L 236.42209 975.91394 L -227.79541 -969.4657 L -139.66971 -986.356 L 148.29639 992.80426 L -50.38492 -995.2788 L 39.340546 -996.16223 L -30.713806 1002.6105 L -120.157104 995.44745 L 128.78381 -988.9992 L 217.22491 -973.84735 L -208.5982 980.2956 L 303.95184 -950.8286 L 388.26636 -920.12854 L -379.63965 926.5768 L -460.8624 888.4425 L 469.48914 -881.99426 L 546.96686 -836.73254 L -538.3402 843.1808 Z"
}

#[test]
fn reduced_test_case_03() {
    let mut builder = Path::builder();

    builder.move_to(point(997.2859, 38.078064));
    builder.line_to(point(-1000.8505, -48.24139));
    builder.line_to(point(-980.1207, -212.09396));
    builder.line_to(point(976.556, 201.93065));
    builder.line_to(point(929.13965, 360.13647));
    builder.line_to(point(-932.70435, -370.29977));
    builder.line_to(point(-859.89484, -518.5434));
    builder.line_to(point(856.33014, 508.38007));
    builder.line_to(point(760.1136, 642.6178));
    builder.line_to(point(-763.6783, -652.7811));
    builder.line_to(point(-646.6792, -769.3514));
    builder.line_to(point(643.1145, 759.188));
    builder.line_to(point(508.52423, 854.91095));
    builder.line_to(point(-512.0889, -865.0742));
    builder.line_to(point(-363.57895, -937.33875));
    builder.line_to(point(360.01428, 927.1754));
    builder.line_to(point(201.63538, 974.01044));
    builder.line_to(point(-205.20004, -984.1737));
    builder.line_to(point(-41.272438, -1004.30164));
    builder.line_to(point(37.707764, 994.1383));
    builder.line_to(point(-127.297035, 987.01013));
    builder.line_to(point(123.73236, -997.1734));
    builder.line_to(point(285.31345, -962.9835));
    builder.line_to(point(-288.8781, 952.82025));
    builder.line_to(point(-442.62796, 892.5013));
    builder.line_to(point(439.0633, -902.6646));
    builder.line_to(point(580.7881, -817.8619));
    builder.line_to(point(-584.3528, 807.6986));
    builder.line_to(point(-710.18646, 700.7254));
    builder.line_to(point(706.62177, -710.8888));
    builder.line_to(point(813.13196, -584.6631));
    builder.line_to(point(-816.69666, 574.49976));
    builder.line_to(point(-900.9784, 432.46442));
    builder.line_to(point(897.4137, -442.62775));
    builder.line_to(point(957.1676, -288.65726));
    builder.line_to(point(-960.7323, 278.49396));
    builder.line_to(point(-994.3284, 116.7885));
    builder.line_to(point(990.76373, -126.95181));
    builder.close();

    let mut tess = FillTessellator::new();

    let mut buffers: VertexBuffers<Point, u16> = VertexBuffers::new();

    tess.tessellate_path(
        &builder.build(),
        &FillOptions::default(),
        &mut simple_builder(&mut buffers),
    ).unwrap();

    // SVG path syntax:
    // "M 997.2859 38.078064 L -1000.8505 -48.24139 L -980.1207 -212.09396 L 976.556 201.93065 L 929.13965 360.13647 L -932.70435 -370.29977 L -859.89484 -518.5434 L 856.33014 508.38007 L 760.1136 642.6178 L -763.6783 -652.7811 L -646.6792 -769.3514 L 643.1145 759.188 L 508.52423 854.91095 L -512.0889 -865.0742 L -363.57895 -937.33875 L 360.01428 927.1754 L 201.63538 974.01044 L -205.20004 -984.1737 L -41.272438 -1004.30164 L 37.707764 994.1383 L -127.297035 987.01013 L 123.73236 -997.1734 L 285.31345 -962.9835 L -288.8781 952.82025 L -442.62796 892.5013 L 439.0633 -902.6646 L 580.7881 -817.8619 L -584.3528 807.6986 L -710.18646 700.7254 L 706.62177 -710.8888 L 813.13196 -584.6631 L -816.69666 574.49976 L -900.9784 432.46442 L 897.4137 -442.62775 L 957.1676 -288.65726 L -960.7323 278.49396 L -994.3284 116.7885 L 990.76373 -126.95181 Z"
}

#[test]
fn reduced_test_case_04() {
    let mut builder = Path::builder();

    builder.move_to(point(540.7645, 838.81036));
    builder.line_to(point(-534.48315, -847.5593));
    builder.line_to(point(-347.42682, -940.912));
    builder.line_to(point(151.33032, 984.5845));
    builder.line_to(point(-145.04895, -993.33344));
    builder.line_to(point(63.80545, -1002.5327));
    builder.line_to(point(-57.52408, 993.78375));
    builder.line_to(point(-263.7273, 959.35864));
    builder.line_to(point(270.00864, -968.1076));
    builder.line_to(point(464.54828, -891.56274));
    builder.line_to(point(-458.26697, 882.81384));
    builder.line_to(point(-632.64087, 767.49457));
    builder.line_to(point(638.9222, -776.2435));
    builder.line_to(point(785.5095, -627.18994));
    builder.line_to(point(-779.22815, 618.4409));
    builder.line_to(point(-891.62213, 442.1673));
    builder.line_to(point(897.9035, -450.91632));
    builder.line_to(point(971.192, -255.12662));
    builder.line_to(point(-964.9106, 246.37766));
    builder.line_to(point(-927.4177, -370.5181));
    builder.line_to(point(933.6991, 361.7691));
    builder.line_to(point(837.23865, 547.24194));
    builder.line_to(point(-830.9573, -555.9909));
    builder.line_to(point(-698.0427, -717.3555));
    builder.close();

    let mut tess = FillTessellator::new();

    let mut buffers: VertexBuffers<Point, u16> = VertexBuffers::new();

    tess.tessellate_path(
        &builder.build(),
        &FillOptions::default(),
        &mut simple_builder(&mut buffers),
    ).unwrap();

    // SVG path syntax:
    // "M 540.7645 838.81036 L -534.48315 -847.5593 L -347.42682 -940.912 L 151.33032 984.5845 L -145.04895 -993.33344 L 63.80545 -1002.5327 L -57.52408 993.78375 L -263.7273 959.35864 L 270.00864 -968.1076 L 464.54828 -891.56274 L -458.26697 882.81384 L -632.64087 767.49457 L 638.9222 -776.2435 L 785.5095 -627.18994 L -779.22815 618.4409 L -891.62213 442.1673 L 897.9035 -450.91632 L 971.192 -255.12662 L -964.9106 246.37766 L -927.4177 -370.5181 L 933.6991 361.7691 L 837.23865 547.24194 L -830.9573 -555.9909 L -698.0427 -717.3555 Z"
}

#[test]
fn reduced_test_case_05() {
    let mut builder = Path::builder();

    builder.move_to(point(540.7645, 838.81036));
    builder.line_to(point(-534.48315, -847.5593));
    builder.line_to(point(-347.42682, -940.912));
    builder.line_to(point(353.70816, 932.163));
    builder.line_to(point(151.33032, 984.5845));
    builder.line_to(point(-145.04895, -993.33344));
    builder.line_to(point(63.80545, -1002.5327));
    builder.line_to(point(-263.7273, 959.35864));
    builder.line_to(point(270.00864, -968.1076));
    builder.line_to(point(464.54828, -891.56274));
    builder.line_to(point(-458.26697, 882.81384));
    builder.line_to(point(-632.64087, 767.49457));
    builder.line_to(point(638.9222, -776.2435));
    builder.line_to(point(785.5095, -627.18994));
    builder.line_to(point(-779.22815, 618.4409));
    builder.line_to(point(-891.62213, 442.1673));
    builder.line_to(point(897.9035, -450.91632));
    builder.line_to(point(971.192, -255.12662));
    builder.line_to(point(-964.9106, 246.37766));
    builder.line_to(point(-995.89075, 39.628937));
    builder.line_to(point(1002.1721, -48.3779));
    builder.line_to(point(989.48975, 160.29398));
    builder.line_to(point(-983.2084, -169.04297));
    builder.line_to(point(-927.4177, -370.5181));
    builder.line_to(point(933.6991, 361.7691));
    builder.line_to(point(837.23865, 547.24194));
    builder.line_to(point(-830.9573, -555.9909));
    builder.line_to(point(-698.0427, -717.3555));
    builder.line_to(point(704.3241, 708.6065));
    builder.close();

    let mut tess = FillTessellator::new();

    let mut buffers: VertexBuffers<Point, u16> = VertexBuffers::new();

    tess.tessellate_path(
        &builder.build(),
        &FillOptions::default(),
        &mut simple_builder(&mut buffers),
    ).unwrap();

    // SVG path syntax:
    // "M 540.7645 838.81036 L -534.48315 -847.5593 L -347.42682 -940.912 L 353.70816 932.163 L 151.33032 984.5845 L -145.04895 -993.33344 L 63.80545 -1002.5327 L -263.7273 959.35864 L 270.00864 -968.1076 L 464.54828 -891.56274 L -458.26697 882.81384 L -632.64087 767.49457 L 638.9222 -776.2435 L 785.5095 -627.18994 L -779.22815 618.4409 L -891.62213 442.1673 L 897.9035 -450.91632 L 971.192 -255.12662 L -964.9106 246.37766 L -995.89075 39.628937 L 1002.1721 -48.3779 L 989.48975 160.29398 L -983.2084 -169.04297 L -927.4177 -370.5181 L 933.6991 361.7691 L 837.23865 547.24194 L -830.9573 -555.9909 L -698.0427 -717.3555 L 704.3241 708.6065 Z"
}

#[test]
fn reduced_test_case_06() {
    let mut builder = Path::builder();

    builder.move_to(point(831.9957, 561.9206));
    builder.line_to(point(-829.447, -551.4562));
    builder.line_to(point(-505.64172, -856.7632));
    builder.line_to(point(508.19046, 867.2276));
    builder.line_to(point(83.98413, 1001.80585));
    builder.line_to(point(-81.435394, -991.34143));
    builder.line_to(point(359.1525, -928.5361));
    builder.line_to(point(-356.60376, 939.0005));
    builder.line_to(point(-726.3096, 691.25085));
    builder.line_to(point(728.8583, -680.78644));
    builder.line_to(point(-951.90845, 307.6267));
    builder.close();

    let mut tess = FillTessellator::new();

    let mut buffers: VertexBuffers<Point, u16> = VertexBuffers::new();

    tess.tessellate_path(
        &builder.build(),
        &FillOptions::default(),
        &mut simple_builder(&mut buffers),
    ).unwrap();

    // SVG path syntax:
    // "M 831.9957 561.9206 L -829.447 -551.4562 L -505.64172 -856.7632 L 508.19046 867.2276 L 83.98413 1001.80585 L -81.435394 -991.34143 L 359.1525 -928.5361 L -356.60376 939.0005 L -726.3096 691.25085 L 728.8583 -680.78644 L -951.90845 307.6267 Z"
}

#[test]
fn reduced_test_case_07() {
    let mut builder = Path::builder();

    builder.move_to(point(960.5097, -271.01678));
    builder.line_to(point(-967.03217, 262.446));
    builder.line_to(point(-987.3192, -182.13324));
    builder.line_to(point(980.7969, 173.56247));
    builder.line_to(point(806.1792, 582.91675));
    builder.line_to(point(-812.7016, -591.48755));
    builder.line_to(point(-477.76422, -884.53925));
    builder.line_to(point(471.24182, 875.9685));
    builder.line_to(point(42.32347, 994.6751));
    builder.line_to(point(-48.845886, -1003.2459));
    builder.line_to(point(389.10114, -924.0962));
    builder.line_to(point(-395.62357, 915.5254));
    builder.line_to(point(-755.85846, 654.19574));
    builder.line_to(point(749.3361, -662.7665));
    builder.close();

    let mut tess = FillTessellator::new();

    let mut buffers: VertexBuffers<Point, u16> = VertexBuffers::new();

    tess.tessellate_path(
        &builder.build(),
        &FillOptions::default(),
        &mut simple_builder(&mut buffers),
    ).unwrap();

    // SVG path syntax:
    // "M 960.5097 -271.01678 L -967.03217 262.446 L -987.3192 -182.13324 L 980.7969 173.56247 L 806.1792 582.91675 L -812.7016 -591.48755 L -477.76422 -884.53925 L 471.24182 875.9685 L 42.32347 994.6751 L -48.845886 -1003.2459 L 389.10114 -924.0962 L -395.62357 915.5254 L -755.85846 654.19574 L 749.3361 -662.7665 Z"
}

#[test]
fn reduced_test_case_08() {
    let mut builder = Path::builder();

    builder.move_to(point(-85.92998, 24.945076));
    builder.line_to(point(-79.567345, 28.325748));
    builder.line_to(point(-91.54697, 35.518726));
    builder.line_to(point(-85.92909, 24.945545));
    builder.close();

    builder.move_to(point(-57.761955, 34.452206));
    builder.line_to(point(-113.631676, 63.3717));
    builder.line_to(point(-113.67784, 63.347214));
    builder.close();

    let mut tess = FillTessellator::new();

    let mut buffers: VertexBuffers<Point, u16> = VertexBuffers::new();

    tess.tessellate_path(
        &builder.build(),
        &FillOptions::default(),
        &mut simple_builder(&mut buffers),
    ).unwrap();

    // SVG path syntax:
    // "M -85.92998 24.945076 L -79.567345 28.325748 L -91.54697 35.518726 L -85.92909 24.945545 ZM -57.761955 34.452206 L -113.631676 63.3717 L -113.67784 63.347214 Z"
}

#[test]
fn reduced_test_case_09() {
    let mut builder = Path::builder();

    builder.move_to(point(659.9835, 415.86328));
    builder.line_to(point(70.36328, 204.36978));
    builder.line_to(point(74.12529, 89.01107));
    builder.close();

    builder.move_to(point(840.2258, 295.46188));
    builder.line_to(point(259.41193, 272.18054));
    builder.line_to(point(728.914, 281.41678));
    builder.close();

    let mut tess = FillTessellator::new();

    let mut buffers: VertexBuffers<Point, u16> = VertexBuffers::new();

    tess.tessellate_path(
        &builder.build(),
        &FillOptions::default(),
        &mut simple_builder(&mut buffers),
    ).unwrap();

    // SVG path syntax:
    // "M 659.9835 415.86328 L 70.36328 204.36978 L 74.12529 89.01107 ZM 840.2258 295.46188 L 259.41193 272.18054 L 728.914 281.41678 Z"
}

#[test]
fn reduced_test_case_10() {
    let mut builder = Path::builder();

    builder.move_to(point(993.5114, -94.67855));
    builder.line_to(point(-938.76056, -355.94995));
    builder.line_to(point(933.8779, 346.34995));
    builder.line_to(point(-693.6775, -727.42883));
    builder.line_to(point(-311.68665, -955.7822));
    builder.line_to(point(306.80408, 946.1823));
    builder.line_to(point(-136.43655, 986.182));
    builder.line_to(point(131.55396, -995.782));
    builder.line_to(point(548.25525, -839.50555));
    builder.line_to(point(-553.13776, 829.9056));
    builder.line_to(point(-860.76697, 508.30533));
    builder.line_to(point(855.88434, -517.90533));
    builder.close();

    let mut tess = FillTessellator::new();

    let mut buffers: VertexBuffers<Point, u16> = VertexBuffers::new();

    tess.tessellate_path(
        &builder.build(),
        &FillOptions::default(),
        &mut simple_builder(&mut buffers),
    ).unwrap();

    // SVG path syntax:
    // "M 993.5114 -94.67855 L -938.76056 -355.94995 L 933.8779 346.34995 L -693.6775 -727.42883 L -311.68665 -955.7822 L 306.80408 946.1823 L -136.43655 986.182 L 131.55396 -995.782 L 548.25525 -839.50555 L -553.13776 829.9056 L -860.76697 508.30533 L 855.88434 -517.90533 Z"
}

#[test]
fn reduced_test_case_11() {
    let mut builder = Path::builder();

    builder.move_to(point(10.0095005, 0.89995164));
    builder.line_to(point(10.109498, 10.899451));
    builder.line_to(point(0.10999817, 10.99945));
    builder.close();

    builder.move_to(point(19.999, -0.19999667));
    builder.line_to(point(20.098999, 9.799503));
    builder.line_to(point(10.099499, 9.899502));
    builder.close();

    let mut tess = FillTessellator::new();

    let mut buffers: VertexBuffers<Point, u16> = VertexBuffers::new();

    tess.tessellate_path(
        &builder.build(),
        &FillOptions::default(),
        &mut simple_builder(&mut buffers),
    ).unwrap();

    // SVG path syntax:
    // "M 10.0095005 0.89995164 L 10.109498 10.899451 L 0.10999817 10.99945 ZM 19.999 -0.19999667 L 20.098999 9.799503 L 10.099499 9.899502 Z"
}

#[test]
fn reduced_test_case_12() {
    let mut builder = Path::builder();

    builder.move_to(point(5.5114865, -8.40378));
    builder.line_to(point(14.377752, -3.7789207));
    builder.line_to(point(9.7528925, 5.0873456));
    builder.close();

    builder.move_to(point(4.62486, -8.866266));
    builder.line_to(point(18.115986, -13.107673));
    builder.line_to(point(13.491126, -4.2414064));
    builder.close();

    let mut tess = FillTessellator::new();

    let mut buffers: VertexBuffers<Point, u16> = VertexBuffers::new();

    tess.tessellate_path(
        &builder.build(),
        &FillOptions::default(),
        &mut simple_builder(&mut buffers),
    ).unwrap();

    // SVG path syntax:
    // "M 5.5114865 -8.40378 L 14.377752 -3.7789207 L 9.7528925 5.0873456 ZM 4.62486 -8.866266 L 18.115986 -13.107673 L 13.491126 -4.2414064 Z"
}

#[test]
fn reduced_test_case_13() {
    let mut builder = Path::builder();

    builder.move_to(point(-989.1437, 132.75488));
    builder.line_to(point(994.39124, -123.3494));
    builder.line_to(point(518.279, 861.4989));
    builder.line_to(point(-513.03143, -852.09344));
    builder.line_to(point(-364.97452, -925.282));
    builder.line_to(point(370.2221, 934.68744));
    builder.line_to(point(-206.8905, -973.10284));
    builder.line_to(point(-43.09149, -994.2518));
    builder.line_to(point(48.33908, 1003.6572));
    builder.line_to(point(-116.706924, 997.5573));
    builder.line_to(point(121.95452, -988.15186));
    builder.line_to(point(283.74548, -954.96936));
    builder.line_to(point(-278.49792, 964.3749));
    builder.line_to(point(-432.6207, 905.0151));
    builder.line_to(point(437.86832, -895.6096));
    builder.line_to(point(959.78815, -284.84253));
    builder.line_to(point(-954.5406, 294.24802));
    builder.close();

    let mut tess = FillTessellator::new();

    let mut buffers: VertexBuffers<Point, u16> = VertexBuffers::new();

    tess.tessellate_path(
        &builder.build(),
        &FillOptions::default(),
        &mut simple_builder(&mut buffers),
    ).unwrap();

    // SVG path syntax:
    // "M -989.1437 132.75488 L 994.39124 -123.3494 L 518.279 861.4989 L -513.03143 -852.09344 L -364.97452 -925.282 L 370.2221 934.68744 L -206.8905 -973.10284 L -43.09149 -994.2518 L 48.33908 1003.6572 L -116.706924 997.5573 L 121.95452 -988.15186 L 283.74548 -954.96936 L -278.49792 964.3749 L -432.6207 905.0151 L 437.86832 -895.6096 L 959.78815 -284.84253 L -954.5406 294.24802 Z"
}

#[test]
fn reduced_test_case_14() {
    let mut builder = Path::builder();

    builder.move_to(point(0.0, 0.0));
    builder.line_to(point(10.0, 20.0));
    builder.line_to(point(10.0, 10.0));
    builder.line_to(point(40.0, 25.0));
    builder.line_to(point(50.0, 0.0));
    builder.line_to(point(50.0, 60.0));
    builder.line_to(point(40.0, 30.0));
    builder.line_to(point(40.0, 60.0));
    builder.line_to(point(30.0, 60.0));
    builder.line_to(point(40.0, 30.0));
    builder.line_to(point(20.0, 60.0));
    builder.line_to(point(0.0, 60.0));
    builder.close();

    let mut tess = FillTessellator::new();

    let mut buffers: VertexBuffers<Point, u16> = VertexBuffers::new();

    tess.tessellate_path(
        &builder.build(),
        &FillOptions::default(),
        &mut simple_builder(&mut buffers),
    ).unwrap();

    // SVG path syntax:
    // "M 0 0 L 100 200 100 100 400 250 500 0 500 600 400 300 400 600 300 600 400 300 200 600 100 600 400 300 0 600 Z"
}
