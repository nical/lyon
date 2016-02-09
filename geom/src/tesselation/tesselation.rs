use tesselation::polygon::*;
use tesselation::path::*;
use tesselation::{ WindingOrder };
use tesselation::vectors::{ Vec2, vec2_add, vec2_sub, Position2D };
use tesselation::vertex_builder::{ VertexBufferBuilder };
use tesselation::connection::{ Connections, apply_connections };
use tesselation::bezier::{ separate_bezier_faces, triangulate_quadratic_bezier };
use tesselation::monotone::{ is_y_monotone, DecompositionContext, TriangulationContext, };
use tesselation::path_to_polygon::*;

use vodk_id::id_vector::IdSlice;

pub fn tesselate_complex_path_fill<'l, Output: VertexBufferBuilder<Vec2>>(
    path: ComplexPathSlice<'l>,
    output: &mut Output
) -> Result<(), ()> {
    output.begin_geometry();

    let mut polygon = try!{ complex_path_to_polygon(path) };

    for v in path.vertices().as_slice() {
        output.push_vertex(v.position());
    }

    let vertex_positions = path.vertices();
    let mut beziers: Vec<[Vec2; 3]> = vec![];

    for p in &mut polygon.sub_polygons {
        separate_bezier_faces(p, vertex_positions, &mut beziers);
    }

    let mut connections = Connections::new();
    let mut ctx = DecompositionContext::new();

    let res = ctx.y_monotone_polygon_decomposition(&polygon, vertex_positions, &mut connections);
    assert_eq!(res, Ok(()));

    let mut monotone_polygons = Vec::new();
    apply_connections(&polygon, vertex_positions, &mut connections, &mut monotone_polygons);

    let mut triangulator = TriangulationContext::new();
    for monotone_poly in monotone_polygons {
        assert!(is_y_monotone(monotone_poly.slice(), vertex_positions));
        let res = triangulator.y_monotone_triangulation(monotone_poly.slice(), vertex_positions, output);
        assert_eq!(res, Ok(()));
    }

    for b in beziers {
        let from = b[0];
        let ctrl = b[1];
        let to = b[2];
        triangulate_quadratic_bezier(from, ctrl, to, 16, output);
    }

    return Ok(());
}

