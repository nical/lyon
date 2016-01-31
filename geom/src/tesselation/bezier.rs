use tesselation::{ Index, VertexId };
use tesselation::path::{ PointData, PointType };
use tesselation::monotone::{ Write };
use tesselation::vertex_builder::VertexBufferBuilder;
use tesselation::polygon::{ AbstractPolygon, Polygon, point_id };
use tesselation::vectors::{ Vec2, vec2_mul, vec2_add, vec2_sub, vec2_cross };

use vodk_id::id_vector::IdSlice;

pub fn separate_bezier_faces<Output: Write<[Vec2; 3]>>(
    polygon: &mut Polygon,
    vertices: IdSlice<VertexId, PointData>,
    out_beziers: &mut Output
) {
    let start = point_id(0);
    let mut it = start;
    loop {
        let next = polygon.next(it);
        if vertices[polygon.vertex(it)].point_type == PointType::Control {
            let ctrl = it;
            let prev = polygon.previous(it);
            if vertices[polygon.vertex(next)].point_type == PointType::Normal {
                let va = vertices[polygon.previous_vertex(it)].position;
                let vb = vertices[polygon.vertex(it)].position;
                let vc = vertices[polygon.next_vertex(it)].position;

                if vec2_cross(vec2_sub(vc, va), vec2_sub(vb, va)) < 0.0 {
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

pub fn triangulate_quadratic_bezier<Geometry: VertexBufferBuilder<[f32; 2]>>(
    from: [f32; 2],
    ctrl: [f32; 2],
    to: [f32; 2],
    num_points: u32,
    output: &mut Geometry
) {
    output.begin_geometry();
    println!("triangulate quadratic {:?} {:?} {:?}", from, ctrl, to);
    if vec2_cross(vec2_sub(to, from), vec2_sub(ctrl, from)) < 0.0 {
        // ctrl is outside the shape
        for i in 1..((num_points-1) as Index) {
            output.push_indices(0, i, i+1);
        }
    } else {
        // ctrl is inside the shape
        output.push_vertex(ctrl);
        for i in 1..(num_points as Index) {
            if i == i {
                output.push_indices(0, i, i+1);
            }
        }
    }
    for i in 0..num_points {
        let t: f32 = i as f32 / ((num_points - 1) as f32);
        let t2 = t*t;
        let one_t = 1.0 - t;
        let one_t2 = one_t * one_t;
        let new_vertex = vec2_add(
            vec2_add(
                vec2_mul(from, one_t2),
                vec2_mul(ctrl, 2.0*one_t*t)
            ),
            vec2_mul(to, t2)
        );
        output.push_vertex(new_vertex);
    }
}
