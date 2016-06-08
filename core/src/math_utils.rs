use vodk_math::{ Vector2D };

#[cfg(test)]
use vodk_math::{ vec2 };

/// Defines an ordering between two points
pub fn is_below<U>(a: Vector2D<U>, b: Vector2D<U>) -> bool {
    a.y > b.y || (a.y == b.y && a.x > b.x)
}

// Compute the vector from ce center of an ellipse on of its points
pub fn ellipse_center_to_point<U>(center: Vector2D<U>, ellipse_point: Vector2D<U>, radii: Vector2D<U>) -> Vector2D<U>{
    Vector2D::new(
        (ellipse_point.x - center.x) / radii.x,
        (ellipse_point.y - center.y) / radii.y,
    )
}

pub fn ellipse_point_from_angle<U>(center: Vector2D<U>, radii: Vector2D<U>, angle: f32) -> Vector2D<U>{
    Vector2D::new(
        center.x + radii.x * angle.cos(),
        center.y + radii.y * angle.sin()
    )
}

pub fn norme<U>(v: Vector2D<U>) -> f32 {
    (v.x.powi(2) + v.y.powi(2)).sqrt()
}

pub fn angle_between<U>(start_vector : Vector2D<U>, end_vector : Vector2D<U>) -> f32 {
    let mut result = ((start_vector.x * end_vector.x + start_vector.y * end_vector.y) /
                 (norme(start_vector) * norme(end_vector))).acos() ;

    if (start_vector.x*end_vector.y - start_vector.y*end_vector.x) < 0.0{
        result = - result;
    }
    result
}

pub fn tangent<U>(v: Vector2D<U>) -> Vector2D<U> {
    let l = v.length();
    return Vector2D::new(-v.y / l, v.x / l);
}

pub fn line_intersection<U>(
    a1: Vector2D<U>,
    a2: Vector2D<U>,
    b1: Vector2D<U>,
    b2: Vector2D<U>
) -> Option<Vector2D<U>> {
    let det = (a1.x - a2.x) * (b1.y - b2.y) - (a1.y - a2.y) * (b1.x - b2.x);
    if det.abs() <= 0.000001 {
        // The lines are very close to parallel
        return None;
    }
    let inv_det = 1.0 / det;
    let a = a1.x * a2.y - a1.y * a2.x;
    let b = b1.x * b2.y - b1.y * b2.x;
    return Some(Vector2D::new(
        (a * (b1.x - b2.x) - b * (a1.x - a2.x)) * inv_det,
        (a * (b1.y - b2.y) - b * (a1.y - a2.y)) * inv_det
    ));
}

pub fn segment_intersection<U>(
    a1: Vector2D<U>,
    b1: Vector2D<U>,
    a2: Vector2D<U>,
    b2: Vector2D<U>
) -> Option<Vector2D<U>> {
    let v1 = b1 - a1;
    let v2 = b2 - a2;
    if v2.fuzzy_eq(Vector2D::new(0.0, 0.0)) {
        return None;
    }

    let v1_cross_v2 = v1.cross(v2);
    let a2_a1_cross_v1 = (a2 - a1).cross(v1);

    if v1_cross_v2 == 0.0 {
        if a2_a1_cross_v1 == 0.0 {

            let v1_sqr_len = v1.square_length();
            // check if a2 is between a1 and b1
            let v1_dot_a2a1 = v1.dot(&(a2-a1));
            if v1_dot_a2a1 > 0.0 && v1_dot_a2a1 < v1_sqr_len { return Some(a2); }

            // check if b2 is between a1 and b1
            let v1_dot_b2a1 = v1.dot(&(b2-a1));
            if v1_dot_b2a1 > 0.0 && v1_dot_b2a1 < v1_sqr_len { return Some(b2); }

            let v2_sqr_len = v2.square_length();
            // check if a1 is between a2 and b2
            let v2_dot_a1a2 = v2.dot(&(a1-a2));
            if v2_dot_a1a2 > 0.0 && v2_dot_a1a2 < v2_sqr_len { return Some(a1); }

            // check if b1 is between a2 and b2
            let v2_dot_b1a2 = v2.dot(&(b1-a2));
            if v2_dot_b1a2 > 0.0 && v2_dot_b1a2 < v2_sqr_len { return Some(b1); }

            return None;
        }

        return None;
    }

    let t = (a2 - a1).cross(v2) / v1_cross_v2;
    let u = a2_a1_cross_v1 / v1_cross_v2;

    if t > 0.0 && t < 1.0 && u > 0.0 && u < 1.0 {
        return Some(a1 + (v1 * t));
    }

    return None;
}

#[test]
fn test_segment_intersection() {

    assert!(segment_intersection(
        vec2(0.0, -2.0), vec2(-5.0, 2.0),
        vec2(-5.0, 0.0), vec2(-11.0, 5.0)
    ).is_none());

    let i = segment_intersection(
        vec2(0.0, 0.0), vec2(1.0, 1.0),
        vec2(0.0, 1.0), vec2(1.0, 0.0)
    ).unwrap();
    println!(" intersection: {:?}", i);
    assert!(i.fuzzy_eq(vec2(0.5, 0.5)));

    assert!(segment_intersection(
        vec2(0.0, 0.0), vec2(0.0, 1.0),
        vec2(1.0, 0.0), vec2(1.0, 1.0)
    ).is_none());

    assert!(segment_intersection(
        vec2(0.0, 0.0), vec2(1.0, 0.0),
        vec2(2.0, 0.0), vec2(3.0, 0.0)
    ).is_none());

    assert!(segment_intersection(
        vec2(0.0, 0.0), vec2(2.0, 0.0),
        vec2(1.0, 0.0), vec2(3.0, 0.0)
    ).is_some());

    assert!(segment_intersection(
        vec2(3.0, 0.0), vec2(1.0, 0.0),
        vec2(2.0, 0.0), vec2(4.0, 0.0)
    ).is_some());

    assert!(segment_intersection(
        vec2(2.0, 0.0), vec2(4.0, 0.0),
        vec2(3.0, 0.0), vec2(1.0, 0.0)
    ).is_some());

    assert!(segment_intersection(
        vec2(1.0, 0.0), vec2(4.0, 0.0),
        vec2(2.0, 0.0), vec2(3.0, 0.0)
    ).is_some());

    assert!(segment_intersection(
        vec2(2.0, 0.0), vec2(3.0, 0.0),
        vec2(1.0, 0.0), vec2(4.0, 0.0)
    ).is_some());

    assert!(segment_intersection(
        vec2(0.0, 0.0), vec2(1.0, 0.0),
        vec2(0.0, 1.0), vec2(1.0, 1.0)
    ).is_none());
}

pub fn line_horizontal_intersection<U>(
    a: Vector2D<U>,
    b: Vector2D<U>,
    y: f32
) -> f32 {
    let vx = b.x - a.x;
    let vy = b.y - a.y;
    if vy == 0.0 {
        // If the segment is horizontal, pick the biggest x value (the right-most point).
        // That's an arbitrary decision that serves the purpose of y-monotone decomposition
        return a.x.max(b.x);
    }
    return a.x + (y - a.y) * vx / vy;
}


#[cfg(test)]
fn assert_almost_eq(a: f32, b:f32) {
    if (a - b).abs() < 0.0001 { return; }
    println!("expected {} and {} to be equal", a, b);
    panic!();
}

#[test]
fn test_intersect_segment_horizontal() {
    assert_almost_eq(line_horizontal_intersection(vec2(0.0, 0.0), vec2(0.0, 2.0), 1.0), 0.0);
    assert_almost_eq(line_horizontal_intersection(vec2(0.0, 2.0), vec2(2.0, 0.0), 1.0), 1.0);
    assert_almost_eq(line_horizontal_intersection(vec2(0.0, 1.0), vec2(3.0, 0.0), 0.0), 3.0);
}
