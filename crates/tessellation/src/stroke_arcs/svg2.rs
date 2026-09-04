//! SVG 2 CR / Editor's Draft radius adjustment (painting.html#LineJoinShape).
//! Radius adjustment preserves each edge's attachment point and tangent.

use super::*;

pub(crate) fn construct_svg2(input: JoinInput) -> Result<JoinConstruction, JoinError> {
    validate_input(input)?;
    let incoming = normalize(input.incoming.tangent, "incoming")?;
    let outgoing = normalize(input.outgoing.tangent, "outgoing")?;
    let cross = incoming.cross(outgoing);
    let dot = incoming.dot(outgoing);
    if cross.abs() <= PARALLEL_EPSILON && dot > 0.0 {
        return Ok(JoinConstruction::Empty);
    }
    if input.incoming.curvature == 0.0 && input.outgoing.curvature == 0.0 {
        return Ok(JoinConstruction::MiterClip);
    }
    let (a, b, limit) = outer_curvature_values(input, cross);
    if a > limit || b > limit {
        return Err(JoinErrorKind::ExcessiveOuterCurvature.into());
    }
    if cross.abs() <= PARALLEL_EPSILON && dot < 0.0 {
        return Ok(JoinConstruction::ParallelRectangle(parallel_rectangle(
            input, incoming,
        )));
    }
    let mut prepared = prepare_join(input, incoming, outgoing)?;
    let intersection = match closest_support_intersection(
        prepared.incoming_support,
        prepared.outgoing_support,
        input.at,
    ) {
        Ok(point) => point,
        Err(_) => adjust_supports(&mut prepared).ok_or(JoinErrorKind::RadiusAdjustmentFailed)?,
    };
    resolve_boundaries::<true, false>(input, prepared, intersection)
}

/// Changes only radii and centers along the attachment normals. Both circles
/// change by the same distance; straight supports remain fixed.
fn adjust_supports(join: &mut PreparedJoin) -> Option<Point64> {
    match (join.incoming_support, join.outgoing_support) {
        (
            SupportCurve::Circle {
                center: a,
                radius: ra,
            },
            SupportCurve::Circle {
                center: b,
                radius: rb,
            },
        ) => {
            let na = (a - join.incoming_offset_point) * ra.recip();
            let nb = (b - join.outgoing_offset_point) * rb.recip();
            let distance = (b - a).square_length().sqrt();
            let nested = distance < (ra - rb).abs();
            let (sa, sb) = if nested {
                if ra > rb {
                    (-1.0, 1.0)
                } else {
                    (1.0, -1.0)
                }
            } else {
                (1.0, 1.0)
            };
            // Normalize the tangency quadratic before solving, so its tolerances
            // do not depend on canvas units or stroke width.
            let scale = distance.max(ra).max(rb);
            let d = (b - a) * scale.recip();
            let v = nb * sb - na * sa;
            let r = if nested { (ra - rb).abs() } else { ra + rb } / scale;
            let dr = if nested { -2.0 } else { 2.0 };
            let delta = first_nonnegative_root(
                v.square_length() - dr * dr,
                2.0 * (d.dot(v) - r * dr),
                d.square_length() - r * r,
            )? * scale;
            if nested && delta > (ra - rb).abs() * 0.5 + scale * 1.0e-10 {
                return None;
            }
            let new_ra = ra + sa * delta;
            let new_rb = rb + sb * delta;
            let new_a = a + na * (sa * delta);
            let new_b = b + nb * (sb * delta);
            let axis = new_b - new_a;
            let length = axis.square_length().sqrt();
            if new_ra <= 0.0 || new_rb <= 0.0 || length <= scale * 1.0e-12 {
                return None;
            }
            let target = if nested {
                (new_ra - new_rb).abs()
            } else {
                new_ra + new_rb
            };
            if (length - target).abs() > 1.0e-8 * scale.max(target) {
                return None;
            }
            let point = if nested && new_rb > new_ra {
                new_b - axis * (new_rb / length)
            } else {
                new_a + axis * (new_ra / length)
            };
            if !point.is_finite() {
                return None;
            }
            join.incoming_support = SupportCurve::Circle {
                center: new_a,
                radius: new_ra,
            };
            join.outgoing_support = SupportCurve::Circle {
                center: new_b,
                radius: new_rb,
            };
            Some(point)
        }
        (line @ SupportCurve::Line { .. }, circle @ SupportCurve::Circle { .. }) => {
            let (circle, point) = adjust_circle_to_line(line, circle, join.outgoing_offset_point)?;
            join.outgoing_support = circle;
            Some(point)
        }
        (circle @ SupportCurve::Circle { .. }, line @ SupportCurve::Line { .. }) => {
            let (circle, point) = adjust_circle_to_line(line, circle, join.incoming_offset_point)?;
            join.incoming_support = circle;
            Some(point)
        }
        (SupportCurve::Line { .. }, SupportCurve::Line { .. }) => None,
    }
}

fn adjust_circle_to_line(
    line: SupportCurve,
    circle: SupportCurve,
    attachment: Point64,
) -> Option<(SupportCurve, Point64)> {
    let SupportCurve::Line { point, direction } = line else {
        return None;
    };
    let SupportCurve::Circle { center, radius } = circle else {
        return None;
    };
    let normal = (center - attachment) * radius.recip();
    let line_normal = normalize(direction, "line").ok()?.left_normal();
    let distance = (attachment - point).dot(line_normal);
    let mut best = f64::INFINITY;
    for side in [-1.0, 1.0] {
        let denominator = side - normal.dot(line_normal);
        if denominator.abs() <= PARALLEL_EPSILON {
            continue;
        }
        let candidate = distance / denominator;
        if candidate >= radius && candidate < best {
            best = candidate;
        }
    }
    if !best.is_finite() {
        return None;
    }
    let center = attachment + normal * best;
    let contact = center - line_normal * (center - point).dot(line_normal);
    contact.is_finite().then_some((
        SupportCurve::Circle {
            center,
            radius: best,
        },
        contact,
    ))
}

/// Cancellation-resistant quadratic roots, including the linear limit.
fn first_nonnegative_root(a: f64, b: f64, c: f64) -> Option<f64> {
    let scale = a.abs().max(b.abs()).max(c.abs());
    if scale == 0.0 || !scale.is_finite() {
        return None;
    }
    let (a, b, c) = (a / scale, b / scale, c / scale);
    if a.abs() <= 1.0e-14 {
        let root = -c / b;
        return (root >= 0.0 && root.is_finite()).then_some(root);
    }
    let discriminant = b * b - 4.0 * a * c;
    if discriminant < -1.0e-14 {
        return None;
    }
    let q = -0.5 * (b + discriminant.max(0.0).sqrt().copysign(b));
    [q / a, c / q]
        .iter()
        .copied()
        .filter(|root| root.is_finite() && *root >= 0.0)
        .min_by(f64::total_cmp)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(ka: f64, kb: f64) -> JoinInput {
        JoinInput {
            at: Point64::new(0.0, 0.0),
            incoming: SegmentEnd {
                tangent: Vector64::new(1.0, 0.0),
                curvature: ka,
            },
            outgoing: SegmentEnd {
                tangent: Vector64::new(0.0, 1.0),
                curvature: kb,
            },
            half_width: 2.0,
            miter_limit: 100.0,
        }
    }

    fn circle(center: (f64, f64), radius: f64) -> SupportCurve {
        SupportCurve::Circle {
            center: Point64::new(center.0, center.1),
            radius,
        }
    }

    fn assert_contact(support: SupportCurve, point: Point64, epsilon: f64) {
        let error = match support {
            SupportCurve::Circle { center, radius } => {
                ((point - center).square_length().sqrt() - radius).abs()
            }
            SupportCurve::Line {
                point: start,
                direction,
            } => (point - start).cross(direction).abs(),
        };
        assert!(
            error < epsilon,
            "contact error {}: {:?}, {:?}",
            error,
            support,
            point
        );
    }

    #[test]
    fn svg2_adjusts_disjoint_supports() {
        for (ka, kb) in [(0.0, -0.4), (-0.4, 0.0), (-0.4, -0.4)] {
            let input = input(ka, kb);
            let result = construct_svg2(input);
            let Ok(JoinConstruction::Arcs(join)) = result else {
                panic!("{}, {}: {:?}", ka, kb, result);
            };
            assert!(!join.clipped);
            assert_contact(join.incoming_support, join.intersection, 1.0e-9);
            assert_contact(join.outgoing_support, join.intersection, 1.0e-9);
            assert_contact(join.incoming_support, join.incoming_offset_point, 1.0e-9);
            assert_contact(join.outgoing_support, join.outgoing_offset_point, 1.0e-9);
        }
    }

    #[test]
    fn nested_circles_shrink_larger_and_grow_smaller_by_equal_amounts() {
        let mut join = prepare_join(
            input(0.1, 0.2),
            Vector64::new(1.0, 0.0),
            Vector64::new(0.0, 1.0),
        )
        .unwrap();
        join.incoming_support = circle((0.0, 0.0), 5.0);
        join.incoming_offset_point = Point64::new(-5.0, 0.0);
        join.outgoing_support = circle((1.0, 0.0), 1.0);
        join.outgoing_offset_point = Point64::new(2.0, 0.0);
        let contact = adjust_supports(&mut join).expect("internal tangency");
        assert_eq!(join.incoming_support, circle((-1.5, 0.0), 3.5));
        assert_eq!(join.outgoing_support, circle((-0.5, 0.0), 2.5));
        assert_contact(join.incoming_support, contact, 1.0e-10);
        assert_contact(join.outgoing_support, contact, 1.0e-10);
    }

    #[test]
    fn separate_circles_grow_equally_and_keep_attachment_normals() {
        let original = prepare_join(
            input(-0.4, -0.4),
            Vector64::new(1.0, 0.0),
            Vector64::new(0.0, 1.0),
        )
        .unwrap();
        let mut adjusted = original;
        let contact = adjust_supports(&mut adjusted).expect("external tangency");
        let mut deltas = [0.0; 2];
        for (index, (before, after, attachment)) in [
            (
                original.incoming_support,
                adjusted.incoming_support,
                original.incoming_offset_point,
            ),
            (
                original.outgoing_support,
                adjusted.outgoing_support,
                original.outgoing_offset_point,
            ),
        ]
        .iter()
        .copied()
        .enumerate()
        {
            let SupportCurve::Circle {
                center: old,
                radius: old_r,
            } = before
            else {
                unreachable!()
            };
            let SupportCurve::Circle {
                center: new,
                radius: new_r,
            } = after
            else {
                unreachable!()
            };
            deltas[index] = new_r - old_r;
            assert!(deltas[index] > 0.0);
            assert!(
                ((old - attachment) * old_r.recip() - (new - attachment) * new_r.recip())
                    .square_length()
                    < 1.0e-20
            );
            assert_contact(after, contact, 1.0e-9);
        }
        assert!((deltas[0] - deltas[1]).abs() < 1.0e-10);
    }

    #[test]
    fn curvature_guard_uses_outer_side() {
        let excessive = input(0.0, -0.6);
        assert_eq!(
            construct_svg2(excessive).unwrap_err().0,
            JoinErrorKind::ExcessiveOuterCurvature
        );
        assert!(
            construct_svg2(input(0.0, 0.6)).is_ok(),
            "inward curvature is not the outer guard"
        );
    }

    #[test]
    fn adjusted_join_is_scale_and_reversal_invariant() {
        for scale in [1.0e-4, 1.0, 1.0e4] {
            let mut original = input(-0.4 / scale, -0.3 / scale);
            original.half_width *= scale;
            let reverse = JoinInput {
                incoming: SegmentEnd {
                    tangent: -original.outgoing.tangent,
                    curvature: -original.outgoing.curvature,
                },
                outgoing: SegmentEnd {
                    tangent: -original.incoming.tangent,
                    curvature: -original.incoming.curvature,
                },
                ..original
            };
            let (Ok(JoinConstruction::Arcs(a)), Ok(JoinConstruction::Arcs(b))) =
                (construct_svg2(original), construct_svg2(reverse))
            else {
                panic!("missing arcs at scale {}", scale);
            };
            assert!((a.intersection - b.intersection).square_length() < 1.0e-16 * scale * scale);
            assert_contact(a.incoming_support, a.intersection, 1.0e-8 * scale);
        }
    }

    #[test]
    fn svg2_still_clips_refitted_arcs_at_the_miter_limit() {
        let result = construct_svg2(JoinInput {
            miter_limit: 1.0,
            ..input(-0.4, -0.4)
        })
        .unwrap();
        assert!(
            matches!(
                result,
                JoinConstruction::Arcs(ResolvedArcsJoin { clipped: true, .. })
                    | JoinConstruction::RadialClip(_)
            ),
            "{:?}",
            result
        );
    }

    #[test]
    fn opposite_parallel_tangents_use_the_svg2_rectangle() {
        let mut input = input(0.1, 0.1);
        input.outgoing.tangent = -input.incoming.tangent;
        assert!(matches!(
            construct_svg2(input),
            Ok(JoinConstruction::ParallelRectangle(_))
        ));
    }
}
