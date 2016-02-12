use tesselation::{ Index };
use tesselation::vertex_builder::VertexBufferBuilder;

use vodk_math::vec2::{ Vector2D };
use vodk_math::units::Unit;

pub fn triangulate_quadratic_bezier<U: Unit, Geometry: VertexBufferBuilder<Vector2D<U>>>(
    from: Vector2D<U>,
    ctrl: Vector2D<U>,
    to: Vector2D<U>,
    num_points: u32,
    output: &mut Geometry
) {
    output.begin_geometry();
    println!("triangulate quadratic {:?} {:?} {:?}", from, ctrl, to);
    if (to - from).cross(ctrl - from) < 0.0 {
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
        output.push_vertex(sample_quadratic_bezier(from, ctrl, to, t));
    }
}

pub fn sample_quadratic_bezier<U: Unit>(
    from: Vector2D<U>,
    ctrl: Vector2D<U>,
    to: Vector2D<U>,
    t: f32
) -> Vector2D<U> {
    let t2 = t*t;
    let one_t = 1.0 - t;
    let one_t2 = one_t * one_t;
    return from * one_t2
         + ctrl * 2.0*one_t*t
         + to * t2;
}

pub fn sample_cubic_bezier<U: Unit>(
    from: Vector2D<U>,
    ctrl1: Vector2D<U>,
    ctrl2: Vector2D<U>,
    to: Vector2D<U>,
    t: f32
) -> Vector2D<U> {
    let t2 = t*t;
    let t3 = t2*t;
    let one_t = 1.0 - t;
    let one_t2 = one_t*one_t;
    let one_t3 = one_t2*one_t;
    return from * one_t3
         + ctrl1 * 3.0*one_t2*t
         + ctrl2 * 3.0*one_t*t2
         + to * t3
}
