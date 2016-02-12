use tesselation::{ VertexId };
use tesselation::polygon::*;
use tesselation::path::*;
use tesselation::vectors::{ Position2D };
use tesselation::vertex_builder::{ VertexBufferBuilder };
use tesselation::connection::{ Connections, apply_connections };
use tesselation::bezier::{ triangulate_quadratic_bezier };
use tesselation::monotone::{ is_y_monotone, DecompositionContext, TriangulationContext, };
use tesselation::path_to_polygon::*;
use tesselation::monotone::{ Write };

use vodk_id::id_vector::IdSlice;
use vodk_math::vec2::Vec2;


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

    let maybe_slice = polygon.as_slice();

    let y_monotone = if let Some(slice) = maybe_slice {
        slice.info().is_y_monotone == Some(true)
    } else {
        false
    };

    let mut monotone_polygon_vec = Vec::new();
    let mut monotone_polygon_slices = Vec::new();
    if y_monotone {
        monotone_polygon_slices.push(maybe_slice.unwrap());
    } else {
        let mut connections = Connections::new();
        let mut ctx = DecompositionContext::new();

        let res = ctx.y_monotone_polygon_decomposition(&polygon, vertex_positions, &mut connections);
        if !res.is_ok() {
            return Err(());
        }

        if maybe_slice.is_some() && connections.is_empty() {
            monotone_polygon_slices.push(maybe_slice.unwrap());
        } else {
            let res = apply_connections(&polygon, vertex_positions, &mut connections, &mut monotone_polygon_vec);
            if !res.is_ok() {
                return Err(());
            }
            monotone_polygon_slices.extend(monotone_polygon_vec.iter().map(|item|{item.slice()}));
        }
    };

    let mut triangulator = TriangulationContext::new();
    for &monotone_poly in &monotone_polygon_slices[..] {
        debug_assert!(is_y_monotone(monotone_poly, vertex_positions));
        let res = triangulator.y_monotone_triangulation(monotone_poly, vertex_positions, output);
        if !res.is_ok() {
            return Err(());
        }
    }

    for b in beziers {
        let from = b[0];
        let ctrl = b[1];
        let to = b[2];
        triangulate_quadratic_bezier(from, ctrl, to, 16, output);
    }

    return Ok(());
}

// TODO: merge this with the polygon generation instead of removing points after the fact.
fn separate_bezier_faces<Output: Write<[Vec2; 3]>>(
    polygon: &mut Polygon,
    vertices: IdSlice<VertexId, PointData>,
    out_beziers: &mut Output
) {
    if polygon.info().has_beziers == Some(false) {
        return;
    }

    let start = point_id(0);
    let mut it = start;
    loop {
        let next = polygon.next(it);
        if vertices[polygon.vertex(it)].point_type == PointType::Control {
            let prev = polygon.previous(it);
            if vertices[polygon.vertex(next)].point_type == PointType::Normal {
                let va = vertices[polygon.previous_vertex(it)].position;
                let vb = vertices[polygon.vertex(it)].position;
                let vc = vertices[polygon.next_vertex(it)].position;

                if (vc - va).cross(vb - va) < 0.0 {
                    // The control point is outside the shape, just need to cut this triangle out.
                    polygon.remove_vertex(it);

                    it = polygon.next(prev);
                } else {
                    // The control point is inside the shape. The loop already wraps around it so
                    // no need to extract this triangle out of the loop.
                    it = next;
                }
                out_beziers.write([va, vb, vc]);
            } else {
                panic!("Only support quadratic bezier curves for now");
            }
        } else {
            it = next;
        }

        if it == start {
            return;
        }
    }
}

