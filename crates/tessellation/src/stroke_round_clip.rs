//! Experimental tangent-continuous biarc tips on actual SVG 2 miter cuts.
//! Vertex metadata stays attached to the join, not to the displaced tip.

use super::*;
use crate::GeometryBuilderError;

/// Append a convex biarc outside the existing cut without moving the base mesh.
/// Both tangents point out of the retained stroke. Degenerate/non-convex fits
/// retain the flat cut instead of adding a loop or overlapping the body.
#[allow(clippy::too_many_arguments)]
pub(super) fn emit(
    join: &EndpointData,
    ends: [Point; 2],
    tangents: [Vector64; 2],
    side: usize,
    tolerance: f32,
    vertex: &mut StrokeVertexData,
    attributes: &dyn AttributeStore,
    output: &mut (impl StrokeGeometryBuilder + ?Sized),
) -> Result<(), TessellationError> {
    let Some(cap) = RoundClip::new(join.position, ends, tangents, tolerance)? else {
        return Ok(());
    };
    vertex.position_on_path = join.position;
    vertex.half_width = join.half_width;
    vertex.side = if side == SIDE_POSITIVE {
        Side::Positive
    } else {
        Side::Negative
    };
    let center = emit_point(cap.center, vertex, attributes, output)?;
    let mut previous = emit_point(cap.arcs[0].start, vertex, attributes, output)?;
    for step in 1..=cap.steps() {
        let next = emit_point(cap.sample(step), vertex, attributes, output)?;
        if cap.clockwise {
            output.add_triangle(center, previous, next);
        } else {
            output.add_triangle(center, next, previous);
        }
        previous = next;
    }
    Ok(())
}

pub(super) fn vector64(v: Vector) -> Vector64 {
    Vector64::new(f64::from(v.x), f64::from(v.y))
}

/// Reconstruct the straight offset-edge direction before join clipping moved
/// its endpoints. The next edge's attachment points may not be initialized yet.
pub(super) fn edge_tangent(from: &EndpointData, to: &EndpointData, side: usize) -> Vector64 {
    let edge = to.position - from.position;
    let length = edge.length();
    let width_delta = to.half_width - from.half_width;
    let sin_angle = width_delta / length;
    let width_angle = if sin_angle.abs() <= 1.0 {
        sin_angle.asin()
    } else {
        0.0
    };
    let normal_angle =
        edge.angle_from_x_axis().radians + side_sign(side) * (PI * 0.5 + width_angle);
    vector64(edge + vector(normal_angle.cos(), normal_angle.sin()) * width_delta)
}

fn emit_point(
    position: Point64,
    vertex: &mut StrokeVertexData,
    attributes: &dyn AttributeStore,
    output: &mut (impl StrokeGeometryBuilder + ?Sized),
) -> Result<VertexId, TessellationError> {
    let position = point64_to_point(position).map_err(|_| GeometryBuilderError::InvalidVertex)?;
    vertex.normal = (position - vertex.position_on_path) / vertex.half_width;
    if !vertex.normal.x.is_finite() || !vertex.normal.y.is_finite() {
        return Err(GeometryBuilderError::InvalidVertex.into());
    }
    Ok(output.add_stroke_vertex(StrokeVertex(vertex, attributes))?)
}

struct RoundClip {
    center: Point64,
    // Each arc runs from its cut endpoint to the shared meeting point.
    arcs: [TipArc; 2],
    clockwise: bool,
}

impl RoundClip {
    fn new(
        at: Point,
        ends: [Point; 2],
        tangents: [Vector64; 2],
        tolerance: f32,
    ) -> Result<Option<Self>, GeometryBuilderError> {
        if !tolerance.is_finite() || tolerance <= 0.0 {
            return Err(GeometryBuilderError::InvalidVertex);
        }
        let [a, b] = ends.map(|p| Point64::new(f64::from(p.x), f64::from(p.y)));
        let chord = b - a;
        let length = chord.x.hypot(chord.y);
        if !length.is_finite() {
            return Err(GeometryBuilderError::InvalidVertex);
        }
        if length == 0.0 {
            return Ok(None);
        }
        let [Some(u), Some(v)] = tangents.map(unit) else {
            return Ok(None);
        };
        let center = a + chord * 0.5;
        let mut outward = Vector64::new(chord.y / length, -chord.x / length);
        let to_tip = center - Point64::new(f64::from(at.x), f64::from(at.y));
        if dot(outward, to_tip) < 0.0 {
            outward = -outward;
        }
        if dot(u, outward) < -1.0e-10 || dot(v, outward) < -1.0e-10 {
            return Ok(None);
        }
        // Equal-distance biarc, with second outline tangent -v. Solve in chord
        // units and rationalize the positive root to avoid cancellation.
        // Derivation: https://www.ryanjuckett.com/biarc-interpolation/
        let axis = chord * length.recip();
        let projection = dot(axis, u - v);
        let denominator = 2.0 * (1.0 + dot(u, v)).max(0.0);
        let root = (projection * projection + denominator).sqrt();
        let distance = if projection >= 0.0 {
            length / (root + projection)
        } else {
            length * (root - projection) / denominator
        };
        if !distance.is_finite() {
            return Ok(None);
        }
        let meeting = center + (u + v) * (distance * 0.5);
        let Some(first) = TipArc::new(a, meeting, u, f64::from(tolerance))? else {
            return Ok(None);
        };
        let Some(second) = TipArc::new(b, meeting, v, f64::from(tolerance))? else {
            return Ok(None);
        };
        let orientation = -chord.cross(outward).signum();
        // Opposite curvature signs on endpoint-to-meeting arcs make a convex
        // cap, so a fan rooted on the cut cannot overlap itself.
        if first.angle * orientation < -1.0e-10 || second.angle * orientation > 1.0e-10 {
            return Ok(None);
        }
        Ok(Some(Self {
            center,
            arcs: [first, second],
            clockwise: orientation < 0.0,
        }))
    }

    fn steps(&self) -> usize {
        self.arcs[0].steps + self.arcs[1].steps
    }

    fn sample(&self, step: usize) -> Point64 {
        if step <= self.arcs[0].steps {
            self.arcs[0].sample(step)
        } else {
            self.arcs[1].sample(self.steps() - step)
        }
    }
}

struct TipArc {
    start: Point64,
    end: Point64,
    tangent: Vector64,
    signed_radius: f64,
    angle: f64,
    steps: usize,
}

impl TipArc {
    fn new(
        start: Point64,
        end: Point64,
        tangent: Vector64,
        tolerance: f64,
    ) -> Result<Option<Self>, GeometryBuilderError> {
        let chord = end - start;
        let normal = Vector64::new(-tangent.y, tangent.x);
        let along = dot(chord, tangent);
        let across = dot(chord, normal);
        // Restrict to minor arcs; sample the exactly straight limit as a line.
        if along < -1.0e-10 * chord.x.hypot(chord.y) {
            return Ok(None);
        }
        let angle = 2.0 * across.atan2(along);
        let radius = if across == 0.0 {
            0.0
        } else {
            dot(chord, chord) / (2.0 * across)
        };
        let step_angle = if radius == 0.0 {
            1.0
        } else {
            4.0 * (tolerance / (2.0 * radius.abs())).min(1.0).sqrt().asin()
        };
        let steps = (angle.abs() / step_angle).ceil().max(2.0);
        if !steps.is_finite() || steps > 32_768.0 {
            return Err(GeometryBuilderError::TooManyVertices);
        }
        Ok(Some(Self {
            start,
            end,
            tangent,
            signed_radius: radius,
            angle,
            steps: steps as usize,
        }))
    }

    fn sample(&self, step: usize) -> Point64 {
        if step == 0 {
            return self.start;
        }
        if step == self.steps {
            return self.end;
        }
        let fraction = step as f64 / self.steps as f64;
        if self.signed_radius == 0.0 {
            return self.start + (self.end - self.start) * fraction;
        }
        let angle = self.angle * fraction;
        let normal = Vector64::new(-self.tangent.y, self.tangent.x);
        // 2 sin²(theta/2) avoids cancellation for nearly flat arcs.
        self.start
            + self.tangent * (self.signed_radius * angle.sin())
            + normal * (self.signed_radius * 2.0 * (angle * 0.5).sin().powi(2))
    }
}

fn dot(a: Vector64, b: Vector64) -> f64 {
    a.x * b.x + a.y * b.y
}

fn unit(v: Vector64) -> Option<Vector64> {
    let length = v.x.hypot(v.y);
    (length.is_finite() && length > 0.0).then(|| v * length.recip())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parallel_edges_retain_the_semicircle() {
        let cap = RoundClip::new(
            point(0.0, 0.0),
            [point(10.0, -3.0), point(10.0, 3.0)],
            [Vector64::new(1.0, 0.0); 2],
            0.01,
        )
        .unwrap()
        .unwrap();
        let apex = cap.arcs[0].end;
        assert!((apex.x - 13.0).abs() < 1.0e-10 && apex.y.abs() < 1.0e-10);
    }

    #[test]
    fn converging_edges_produce_a_shallow_tangent_cap_not_a_half_circle() {
        let cap = RoundClip::new(
            point(0.0, 0.0),
            [point(10.0, -3.0), point(10.0, 3.0)],
            [Vector64::new(1.0, 2.0), Vector64::new(1.0, -2.0)],
            0.001,
        )
        .unwrap()
        .unwrap();
        assert!(cap.arcs[0].end.x > 10.0 && cap.arcs[0].end.x < 11.0);
        assert!((cap.arcs[0].angle.abs() + cap.arcs[1].angle.abs()) < core::f64::consts::PI);
        for arc in &cap.arcs {
            let direction = unit(arc.sample(1) - arc.start).unwrap();
            assert!(dot(direction, arc.tangent) > 0.999);
            assert!(
                arc.signed_radius.abs() * (1.0 - (arc.angle / (2.0 * arc.steps as f64)).cos())
                    <= 0.001
            );
        }
    }

    #[test]
    fn asymmetric_cap_matches_both_tangents_and_the_internal_tangent() {
        let tangents = [Vector64::new(1.0, 0.2), Vector64::new(0.4, -1.0)];
        let cap = RoundClip::new(
            point(0.0, 0.0),
            [point(10.0, -3.0), point(10.0, 3.0)],
            tangents,
            0.0001,
        )
        .unwrap()
        .unwrap();
        assert!(cap.arcs[0].end.y.abs() > 0.1);
        let [a, b] = &cap.arcs;
        let ta = unit(a.end - a.sample(a.steps - 1)).unwrap();
        let tb = unit(b.sample(b.steps - 1) - b.end).unwrap();
        assert!(dot(ta, tb) > 0.999);
        for i in 0..cap.steps() {
            let p = cap.sample(i);
            let q = cap.sample(i + 1);
            assert!(p.x >= 10.0 - 1.0e-10);
            assert!((p - cap.center).cross(q - cap.center) >= -1.0e-10);
        }
    }

    #[test]
    fn reversing_endpoints_preserves_the_cap() {
        let ends = [point(10.0, -3.0), point(10.0, 3.0)];
        let tangents = [Vector64::new(1.0, 0.2), Vector64::new(0.4, -1.0)];
        let a = RoundClip::new(point(0.0, 0.0), ends, tangents, 0.01)
            .unwrap()
            .unwrap();
        let b = RoundClip::new(
            point(0.0, 0.0),
            [ends[1], ends[0]],
            [tangents[1], tangents[0]],
            0.01,
        )
        .unwrap()
        .unwrap();
        assert_ne!(a.clockwise, b.clockwise);
        for i in 0..=a.steps() {
            let delta = a.sample(i) - b.sample(b.steps() - i);
            assert!(delta.x.hypot(delta.y) < 1.0e-10);
        }
    }

    #[test]
    fn collapsed_cut_adds_no_vertices() {
        assert!(RoundClip::new(
            point(0.0, 0.0),
            [point(1.0, 1.0); 2],
            [Vector64::new(1.0, 0.0); 2],
            0.01
        )
        .unwrap()
        .is_none());
    }

    #[test]
    fn curved_cuts_use_boundary_tangents_and_are_shallower_than_semicircles() {
        // The two marked joins in the saved canvas, with width 80 / limit 1.5.
        let cases = [
            [
                point(111.5625, 287.0),
                point(334.5625, 409.0),
                point(499.5625, 237.0),
                point(316.15625, 114.0),
                point(408.5625, 66.0),
                point(495.5625, 63.0),
                point(580.15625, 99.0),
            ],
            [
                point(930.5625, 402.0),
                point(806.5625, 656.0),
                point(424.5625, 653.0),
                point(237.5625, 605.0),
                point(338.5625, 777.0),
                point(630.5625, 737.0),
                point(971.5625, 674.0),
            ],
        ];
        for points in cases {
            for mirror in [-1.0, 1.0] {
                let [_, a, b, at, c, d, _] = points.map(|p| point(p.x, p.y * mirror));
                let segment = |t: Vector, second: Vector| {
                    let EndpointDifferential::Regular {
                        unit_tangent,
                        curvature,
                    } = regular_differential(t, second)
                    else {
                        panic!("regular fixture");
                    };
                    SegmentEnd {
                        tangent: vector64(unit_tangent),
                        curvature,
                    }
                };
                let input = JoinInput {
                    at: Point64::new(f64::from(at.x), f64::from(at.y)),
                    incoming: segment((at - b) * 3.0, ((at - b) - (b - a)) * 6.0),
                    outgoing: segment((c - at) * 3.0, ((d - c) - (c - at)) * 6.0),
                    half_width: 40.0,
                    miter_limit: 1.5,
                };
                let JoinConstruction::Arcs(resolved) = stroke_arcs::construct_svg2(input).unwrap()
                else {
                    panic!("expected curved SVG2 join");
                };
                let ends = resolved
                    .clip_endpoints()
                    .unwrap()
                    .map(|p| point64_to_point(p).unwrap());
                for tolerance in [0.01, 1.1] {
                    let cap = RoundClip::new(at, ends, resolved.clip_tangents(), tolerance)
                        .unwrap()
                        .unwrap();
                    assert!(
                        cap.arcs[0].angle.abs() + cap.arcs[1].angle.abs()
                            < core::f64::consts::PI - 0.1
                    );
                    assert_eq!(
                        cap.arcs[0].tangent,
                        unit(resolved.clip_tangents()[0]).unwrap()
                    );
                    assert_eq!(
                        cap.arcs[1].tangent,
                        unit(resolved.clip_tangents()[1]).unwrap()
                    );
                    assert_eq!(
                        cap.sample(0),
                        Point64::new(f64::from(ends[0].x), f64::from(ends[0].y))
                    );
                    assert_eq!(
                        cap.sample(cap.steps()),
                        Point64::new(f64::from(ends[1].x), f64::from(ends[1].y))
                    );
                }
            }
        }
    }

    #[test]
    fn inward_or_degenerate_tangents_do_not_emit_overlapping_tips() {
        for tangents in [[Vector64::new(-1.0, 0.0); 2], [Vector64::new(0.0, 0.0); 2]] {
            assert!(RoundClip::new(
                point(0.0, 0.0),
                [point(10.0, -3.0), point(10.0, 3.0)],
                tangents,
                0.01
            )
            .unwrap()
            .is_none());
        }
    }
}
