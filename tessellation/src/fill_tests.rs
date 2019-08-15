use crate::geom::math::*;
use crate::geometry_builder::*;
use crate::path::builder::{Build, FlatPathBuilder, PathBuilder};
use crate::path::{Path, PathSlice};
use crate::extra::rust_logo::build_logo_path;
use crate::{FillTessellator, TessellationError, FillOptions, FillVertex, OnError};

use std::env;

#[cfg(feature = "experimental")]
use crate::experimental;

#[cfg(not(feature = "experimental"))]
type Vertex = FillVertex;
#[cfg(feature = "experimental")]
type Vertex = Point;

fn tessellate_path(path: PathSlice, log: bool) -> Result<usize, TessellationError> {
    let mut buffers: VertexBuffers<Vertex, u16> = VertexBuffers::new();
    {
        let options = FillOptions::tolerance(0.05);

        #[cfg(not(feature = "experimental"))] {
            let mut tess = FillTessellator::new();
            let mut vertex_builder = simple_builder(&mut buffers);
            if log {
                tess.enable_logging();
            }
            tess.tessellate_path(
                path.iter(),
                &options,
                &mut vertex_builder
            )?;
        }

        #[cfg(feature = "experimental")] {
            use crate::path::builder::*;
            use crate::path::iterator::*;

            let mut builder = Path::builder();
            for e in path.iter().flattened(0.05) {
                builder.flat_event(e);
            }

            let mut vertex_builder = simple_builder(&mut buffers);
            let mut tess = experimental::FillTessellator::new();
            if log {
                tess.enable_logging();
            }
            tess.tessellate_path(
                &builder.build(),
                &options,
                &mut vertex_builder
            );
        }
    }
    return Ok(buffers.indices.len() / 3);
}

#[test]
fn test_too_many_vertices() {
    /// This test checks that the tessellator returns the proper error when
    /// the geometry builder run out of vertex ids.

    struct Builder { max_vertices: u32 }
    impl<T> GeometryBuilder<T> for Builder
    {
        fn add_vertex(&mut self, _: T) -> Result<VertexId, GeometryBuilderError> {
            if self.max_vertices == 0 {
                return Err(GeometryBuilderError::TooManyVertices);
            }
            self.max_vertices -= 1;
            Ok(VertexId(self.max_vertices))
        }
        fn begin_geometry(&mut self) {}
        fn add_triangle(&mut self, _a: VertexId, _b: VertexId, _c: VertexId) {}
        fn end_geometry(&mut self) -> Count { Count { vertices: 0, indices: 0 } }
        fn abort_geometry(&mut self) {}
    }

    let mut path = Path::builder().with_svg();
    build_logo_path(&mut path);
    let path = path.build();

    let mut tess = FillTessellator::new();
    let mut options = FillOptions::tolerance(0.05);
    options.on_error = OnError::Stop;

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
            // TODO: at the moment the experimental tessellator does not insert points at
            // self-intersections so it can't produce the expected triangle count for some
            // tests.
            #[cfg(feature = "experimental")]
            return;

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

        if add_logging {
            tessellate_path(path, true).unwrap();
        }
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
