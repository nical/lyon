use math::units::world;
use math::vector;

pub fn line_intersection<U>(
    a1: vector::Vector2D<U>,
    a2: vector::Vector2D<U>,
    b1: vector::Vector2D<U>,
    b2: vector::Vector2D<U>
) -> Option<vector::Vector2D<U>> {
    let det = (a1.x - a2.x) * (b1.y - b2.y) - (a1.y - a2.y) * (b1.x - b2.x);
    if det*det < 0.00001 {
        // The lines are very close to parallel
        return None;
    }
    let inv_det = 1.0 / det;
    let a = a1.x * a2.y - a1.y * a2.x;
    let b = b1.x * b2.y - b1.y * b2.x;
    return Some(vector::Vector2D {
        x: (a * (b1.x - b2.x) - b * (a1.x - a2.x)) * inv_det,
        y: (a * (b1.y - b2.y) - b * (a1.y - a2.y)) * inv_det
    });
}

pub fn tangent(v: world::Vec2) -> world::Vec2 {
    let l = v.length();
    return world::vec2(-v.y / l, v.x / l);
}

pub fn extrude_along_tangent(
    path: &[world::Vec2],
    i: uint,
    amount: f32,
    is_closed: bool
) -> world::Vec2 {

    let p1 = if i > 0 { path[i - 1] }
             else if is_closed { path[path.len()-1] }
             else { path[0] + path[0] - path[1] };

    let px = path[i];

    let p2 = if i < path.len() - 1 { path[i + 1] }
             else if is_closed { path[0] }
             else { path[i] + path[i] - path[i - 1] };

    let n1 = tangent(px - p1).times(amount);
    let n2 = tangent(p2 - px).times(amount);

    // Segment P1-->PX
    let pn1  = p1 + n1; // p1 extruded along the tangent n1
    let pn1x = px + n1; // px extruded along the tangent n1
    // Segment PX-->P2
    let pn2  = p2 + n2;
    let pn2x = px + n2;

    let inter = match line_intersection(pn1, pn1x, pn2x, pn2) {
        Some(v) => { v }
        None => {
            if (n1 - n2).square_length() < 0.00001 {
                px + n1
            } else {
                // TODO: the angle is very narrow, use rounded corner instead
                panic!("Not implemented yet");
            }
        }
    };
    return inter;
}
