use std::f32::consts::PI;
use vodk_math::{ Vector2D, Vec2 };

#[cfg(test)]
use vodk_math::{ vec2 };

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum EventType {
    Start,
    End,
    Split,
    Merge,
    Left,
    Right,
}

pub fn intersect_segment_with_horizontal<U>(a: Vector2D<U>, b: Vector2D<U>, y: f32) -> f32 {
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
    assert_almost_eq(intersect_segment_with_horizontal(vec2(0.0, 0.0), vec2(0.0, 2.0), 1.0), 0.0);
    assert_almost_eq(intersect_segment_with_horizontal(vec2(0.0, 2.0), vec2(2.0, 0.0), 1.0), 1.0);
    assert_almost_eq(intersect_segment_with_horizontal(vec2(0.0, 1.0), vec2(3.0, 0.0), 0.0), 3.0);
}

pub fn compute_event_type(prev: Vec2, current: Vec2, next: Vec2) -> EventType {
    // assuming clockwise vertex_positions winding order
    let interrior_angle = (prev - current).directed_angle(next - current);

    // If the interrior angle is exactly 0 we'll have degenerate (invisible 0-area) triangles
    // which is yucks but we can live with it for the sake of being robust against degenerate
    // inputs. So special-case them so that they don't get considered as Merge ot Split vertices
    // otherwise there can be no monotone decomposition of a shape where all points are on the
    // same line.

    if is_below(current, prev) && is_below(current, next) {
        if interrior_angle < PI && interrior_angle != 0.0 {
            return EventType::Merge;
        } else {
            return EventType::End;
        }
    }

    if !is_below(current, prev) && !is_below(current, next) {
        if interrior_angle < PI && interrior_angle != 0.0 {
            return EventType::Split;
        } else {
            return EventType::Start;
        }
    }

    if prev.y == next.y {
        return if prev.x < next.x { EventType::Right } else { EventType::Left };
    }
    return if prev.y < next.y { EventType::Right } else { EventType::Left };
}

/// Defines an ordering between two points
pub fn is_below(a: Vec2, b: Vec2) -> bool { a.y > b.y || (a.y == b.y && a.x > b.x) }
pub fn is_right_of(a: Vec2, b: Vec2) -> bool { a.x > b.x || (a.x == b.x && a.y > b.y) }

