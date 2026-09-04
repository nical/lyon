//! Internal flattening and triangulation for one resolved SVG 2 `arcs` join.

use alloc::vec::Vec;
use core::fmt;

#[cfg(not(feature = "std"))]
use num_traits::Float;

use crate::stroke_arcs::{ArcSweep, BoundaryPiece, Point64, ResolvedArcsJoin, Vector64};

const MAX_VERTEX_COUNT: usize = 4_096;
const MIN_FAN_VERTEX_COUNT: usize = 12;
const TRIANGULATION_EPSILON: f64 = 1.0e-12;

/// Reusable buffers for one outer `arcs` join region.
#[derive(Debug, Default)]
pub(crate) struct ArcsMesh {
    vertices: Vec<Point64>,
    indices: Vec<usize>,
    remaining: Vec<usize>,
    #[cfg(test)]
    buffered_triangle_output: bool,
    #[cfg(test)]
    buffered_quad_output: bool,
    #[cfg(test)]
    buffered_fan_output: bool,
}

pub(crate) enum ArcsOutput<'a> {
    Triangle(DirectArcsTriangle),
    Quad(DirectArcsQuad),
    Fan {
        vertices: &'a [Point64],
        fan: ValidatedFan,
    },
    Mesh {
        vertices: &'a [Point64],
        indices: &'a [usize],
    },
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct DirectArcsTriangle {
    pub(crate) middle: Point64,
    pub(crate) indices: [usize; 3],
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct DirectArcsQuad {
    pub(crate) middle: [Point64; 2],
    pub(crate) indices: [u8; 6],
}

/// Compact topology for a fan already validated against its polygon boundary.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ValidatedFan {
    root: usize,
    vertex_count: usize,
    forward: bool,
}

impl ValidatedFan {
    pub(crate) fn triangles(self) -> impl ExactSizeIterator<Item = [usize; 3]> {
        FanTriangles {
            root: self.root,
            current: next_fan_index(self.root, self.vertex_count, self.forward),
            vertex_count: self.vertex_count,
            remaining: self.vertex_count - 2,
            forward: self.forward,
        }
    }

    fn append_indices(self, indices: &mut Vec<usize>) {
        for [first, second, third] in self.triangles() {
            push_triangle(indices, first, second, third);
        }
    }
}

struct FanTriangles {
    root: usize,
    current: usize,
    vertex_count: usize,
    remaining: usize,
    forward: bool,
}

impl Iterator for FanTriangles {
    type Item = [usize; 3];

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }

        let next = next_fan_index(self.current, self.vertex_count, self.forward);
        let triangle = [self.root, self.current, next];
        self.current = next;
        self.remaining -= 1;
        Some(triangle)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl ExactSizeIterator for FanTriangles {}

#[derive(Clone, Copy)]
struct MeshLayout {
    incoming_boundary: BoundaryPiece,
    outgoing_boundary: BoundaryPiece,
    incoming_sweep_radians: f64,
    outgoing_sweep_radians: f64,
    incoming_steps: usize,
    outgoing_steps: usize,
    clip_vertex_count: usize,
    vertex_count: usize,
}

impl ArcsMesh {
    pub(crate) fn tessellate(
        &mut self,
        join: &ResolvedArcsJoin,
        tolerance: f64,
    ) -> Result<(), ArcsMeshError> {
        self.tessellate_impl::<true, true, true, true>(join, tolerance)
            .map(|_| ())
    }

    pub(crate) fn tessellate_for_output(
        &mut self,
        join: &ResolvedArcsJoin,
        tolerance: f64,
    ) -> Result<ArcsOutput<'_>, ArcsMeshError> {
        self.clear_buffers();
        let layout = prepare_mesh_layout::<true>(join, tolerance)?;
        if layout.vertex_count == 3 && self.direct_triangle_output_enabled() {
            return direct_triangle(layout.incoming_boundary, layout.outgoing_boundary)
                .map(ArcsOutput::Triangle);
        }
        if layout.vertex_count == 4
            && layout.clip_vertex_count == 1
            && self.direct_quad_output_enabled()
        {
            if let Some(quad) = direct_quad(layout.incoming_boundary, layout.outgoing_boundary) {
                return Ok(ArcsOutput::Quad(quad));
            }
        }

        self.tessellate_non_triangle_for_output(layout)
    }

    fn tessellate_non_triangle_for_output(
        &mut self,
        layout: MeshLayout,
    ) -> Result<ArcsOutput<'_>, ArcsMeshError> {
        self.append_layout_vertices::<true>(layout);
        let preferred_fan_roots = [
            layout.incoming_steps,
            layout.incoming_steps + layout.clip_vertex_count,
        ];
        let preferred_fan_roots = if layout.vertex_count >= MIN_FAN_VERTEX_COUNT {
            &preferred_fan_roots[..1 + layout.clip_vertex_count]
        } else {
            &[]
        };
        let signed_area = polygon_signed_area(&self.vertices);
        if let Some(fan) = find_visible_fan(&self.vertices, preferred_fan_roots, signed_area) {
            if self.direct_fan_output_enabled() {
                return Ok(ArcsOutput::Fan {
                    vertices: &self.vertices,
                    fan,
                });
            }

            self.reserve_triangulation_buffers(layout.vertex_count)?;
            fan.append_indices(&mut self.indices);
            return Ok(ArcsOutput::Mesh {
                vertices: &self.vertices,
                indices: &self.indices,
            });
        }

        self.reserve_triangulation_buffers(layout.vertex_count)?;
        triangulate_with_ear_clipping(
            &mut self.indices,
            &self.vertices,
            &mut self.remaining,
            signed_area,
        )?;
        Ok(ArcsOutput::Mesh {
            vertices: &self.vertices,
            indices: &self.indices,
        })
    }

    #[cfg(test)]
    pub(crate) fn set_direct_triangle_output(&mut self, enabled: bool) {
        self.buffered_triangle_output = !enabled;
    }

    #[cfg(test)]
    pub(crate) fn set_direct_quad_output(&mut self, enabled: bool) {
        self.buffered_quad_output = !enabled;
    }

    #[cfg(test)]
    pub(crate) fn set_direct_fan_output(&mut self, enabled: bool) {
        self.buffered_fan_output = !enabled;
    }

    #[inline]
    fn direct_triangle_output_enabled(&self) -> bool {
        #[cfg(test)]
        {
            !self.buffered_triangle_output
        }
        #[cfg(not(test))]
        {
            true
        }
    }

    #[inline]
    fn direct_quad_output_enabled(&self) -> bool {
        #[cfg(test)]
        {
            !self.buffered_quad_output
        }
        #[cfg(not(test))]
        {
            true
        }
    }

    #[inline]
    fn direct_fan_output_enabled(&self) -> bool {
        #[cfg(test)]
        {
            !self.buffered_fan_output
        }
        #[cfg(not(test))]
        {
            true
        }
    }

    fn tessellate_impl<
        const ALLOW_FAN: bool,
        const RECURRENT_ROTATION: bool,
        const SINGLE_CHORD_FAST_PATH: bool,
        const DIRECT_TRIANGLE_FAST_PATH: bool,
    >(
        &mut self,
        join: &ResolvedArcsJoin,
        tolerance: f64,
    ) -> Result<TriangulationMethod, ArcsMeshError> {
        self.clear_buffers();
        let layout = prepare_mesh_layout::<SINGLE_CHORD_FAST_PATH>(join, tolerance)?;
        self.tessellate_layout::<ALLOW_FAN, RECURRENT_ROTATION, DIRECT_TRIANGLE_FAST_PATH>(layout)
    }

    fn clear_buffers(&mut self) {
        self.vertices.clear();
        self.indices.clear();
        self.remaining.clear();
    }

    fn tessellate_layout<
        const ALLOW_FAN: bool,
        const RECURRENT_ROTATION: bool,
        const DIRECT_TRIANGLE_FAST_PATH: bool,
    >(
        &mut self,
        layout: MeshLayout,
    ) -> Result<TriangulationMethod, ArcsMeshError> {
        self.reserve_triangulation_buffers(layout.vertex_count)?;
        self.append_layout_vertices::<RECURRENT_ROTATION>(layout);
        let preferred_fan_roots = [
            layout.incoming_steps,
            layout.incoming_steps + layout.clip_vertex_count,
        ];
        let preferred_fan_roots = if ALLOW_FAN && layout.vertex_count >= MIN_FAN_VERTEX_COUNT {
            &preferred_fan_roots[..1 + layout.clip_vertex_count]
        } else {
            &[]
        };
        triangulate_polygon_impl::<DIRECT_TRIANGLE_FAST_PATH>(
            &mut self.indices,
            &self.vertices,
            &mut self.remaining,
            preferred_fan_roots,
        )
    }

    fn append_layout_vertices<const RECURRENT_ROTATION: bool>(&mut self, layout: MeshLayout) {
        self.vertices.reserve(layout.vertex_count);
        if RECURRENT_ROTATION {
            append_boundary(
                &mut self.vertices,
                layout.incoming_boundary,
                BoundaryTraversal::Forward,
                true,
                layout.incoming_steps,
                layout.incoming_sweep_radians,
            );
        } else {
            append_boundary_trigonometric(
                &mut self.vertices,
                layout.incoming_boundary,
                BoundaryTraversal::Forward,
                true,
                layout.incoming_steps,
                layout.incoming_sweep_radians,
            );
        }
        if layout.clip_vertex_count != 0 {
            self.vertices.push(boundary_end(layout.outgoing_boundary));
        }
        if RECURRENT_ROTATION {
            append_boundary(
                &mut self.vertices,
                layout.outgoing_boundary,
                BoundaryTraversal::Reverse,
                false,
                layout.outgoing_steps,
                layout.outgoing_sweep_radians,
            );
        } else {
            append_boundary_trigonometric(
                &mut self.vertices,
                layout.outgoing_boundary,
                BoundaryTraversal::Reverse,
                false,
                layout.outgoing_steps,
                layout.outgoing_sweep_radians,
            );
        }
    }

    fn reserve_triangulation_buffers(&mut self, vertex_count: usize) -> Result<(), ArcsMeshError> {
        let index_count = vertex_count.saturating_sub(2).checked_mul(3).ok_or(
            ArcsMeshError::TooManyVertices {
                limit: MAX_VERTEX_COUNT,
            },
        )?;
        self.indices.reserve(index_count);
        self.remaining.reserve(vertex_count);
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn tessellate_with_ear_clipping(
        &mut self,
        join: &ResolvedArcsJoin,
        tolerance: f64,
    ) -> Result<(), ArcsMeshError> {
        self.tessellate_impl::<false, true, true, true>(join, tolerance)
            .map(|_| ())
    }

    #[cfg(test)]
    pub(crate) fn tessellate_with_trigonometric_flattening(
        &mut self,
        join: &ResolvedArcsJoin,
        tolerance: f64,
    ) -> Result<(), ArcsMeshError> {
        self.tessellate_impl::<true, false, true, true>(join, tolerance)
            .map(|_| ())
    }

    #[cfg(test)]
    pub(crate) fn tessellate_without_single_chord_fast_path(
        &mut self,
        join: &ResolvedArcsJoin,
        tolerance: f64,
    ) -> Result<(), ArcsMeshError> {
        self.tessellate_impl::<true, true, false, true>(join, tolerance)
            .map(|_| ())
    }

    #[cfg(test)]
    pub(crate) fn tessellate_without_direct_triangle_fast_path(
        &mut self,
        join: &ResolvedArcsJoin,
        tolerance: f64,
    ) -> Result<(), ArcsMeshError> {
        self.tessellate_impl::<true, true, true, false>(join, tolerance)
            .map(|_| ())
    }

    pub(crate) fn vertices(&self) -> &[Point64] {
        &self.vertices
    }

    pub(crate) fn indices(&self) -> &[usize] {
        &self.indices
    }

    /// Returns the number of triangles in the mesh.
    #[must_use]
    pub(crate) fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }
}

/// Failure produced while flattening or indexing a resolved join.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ArcsMeshError {
    InvalidTolerance,
    InvalidBoundary { segment: &'static str },
    TooManyVertices { limit: usize },
    NonTriangulatableBoundary,
}

impl fmt::Display for ArcsMeshError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::InvalidTolerance => {
                formatter.write_str("tolerance must be finite and greater than zero")
            }
            Self::InvalidBoundary { segment } => {
                write!(formatter, "{segment} boundary contains invalid geometry")
            }
            Self::TooManyVertices { limit } => {
                write!(
                    formatter,
                    "flattened join would exceed the limit of {limit} vertices"
                )
            }
            Self::NonTriangulatableBoundary => {
                formatter.write_str("join boundary is not a triangulatable simple polygon")
            }
        }
    }
}

/// Flattens and triangulates one resolved `arcs` join.
///
/// Each circular boundary is split so the maximum distance between the arc
/// and its chord does not exceed `tolerance`. The returned triangle winding is
/// counter-clockwise in mathematical coordinates for either turn direction.
///
/// # Errors
///
/// Returns [`ArcsMeshError`] for invalid tolerance, non-finite geometry, or a
/// subdivision that would exceed the module's safety limit, or a boundary
/// which does not form a triangulatable simple polygon.
#[must_use = "the generated join mesh must be handled"]
pub(crate) fn tessellate_arcs_join(
    join: &ResolvedArcsJoin,
    tolerance: f64,
) -> Result<ArcsMesh, ArcsMeshError> {
    let mut mesh = ArcsMesh::default();
    mesh.tessellate(join, tolerance)?;
    Ok(mesh)
}

fn validate_boundary(boundary: BoundaryPiece, segment: &'static str) -> Result<(), ArcsMeshError> {
    let valid = match boundary {
        BoundaryPiece::Line { start, end } => point_is_finite(start) && point_is_finite(end),
        BoundaryPiece::Arc {
            center,
            radius,
            start,
            end,
            sweep,
        } => {
            point_is_finite(center)
                && radius.is_finite()
                && radius > 0.0
                && point_is_finite(start)
                && point_is_finite(end)
                && sweep.is_finite()
        }
    };
    if !valid {
        return Err(ArcsMeshError::InvalidBoundary { segment });
    }

    Ok(())
}

#[inline]
fn prepare_mesh_layout<const SINGLE_CHORD_FAST_PATH: bool>(
    join: &ResolvedArcsJoin,
    tolerance: f64,
) -> Result<MeshLayout, ArcsMeshError> {
    if !tolerance.is_finite() || tolerance <= 0.0 {
        return Err(ArcsMeshError::InvalidTolerance);
    }
    validate_boundary(join.incoming_boundary(), "incoming")?;
    validate_boundary(join.outgoing_boundary(), "outgoing")?;
    let incoming_boundary = join.incoming_boundary();
    let outgoing_boundary = join.outgoing_boundary();
    let (incoming_steps, incoming_sweep_radians) =
        prepare_boundary::<SINGLE_CHORD_FAST_PATH>(incoming_boundary, tolerance)?;
    let (outgoing_steps, outgoing_sweep_radians) =
        prepare_boundary::<SINGLE_CHORD_FAST_PATH>(outgoing_boundary, tolerance)?;
    let clip_vertex_count = usize::from(join.is_clipped());
    let vertex_count = 1usize
        .checked_add(incoming_steps)
        .and_then(|count| count.checked_add(clip_vertex_count))
        .and_then(|count| count.checked_add(outgoing_steps))
        .ok_or(ArcsMeshError::TooManyVertices {
            limit: MAX_VERTEX_COUNT,
        })?;
    if vertex_count > MAX_VERTEX_COUNT {
        return Err(ArcsMeshError::TooManyVertices {
            limit: MAX_VERTEX_COUNT,
        });
    }

    Ok(MeshLayout {
        incoming_boundary,
        outgoing_boundary,
        incoming_sweep_radians,
        outgoing_sweep_radians,
        incoming_steps,
        outgoing_steps,
        clip_vertex_count,
        vertex_count,
    })
}

#[cfg(test)]
fn boundary_steps(boundary: BoundaryPiece, tolerance: f64) -> Result<usize, ArcsMeshError> {
    boundary_steps_impl::<true>(boundary, tolerance)
}

fn boundary_steps_impl<const SINGLE_CHORD_FAST_PATH: bool>(
    boundary: BoundaryPiece,
    tolerance: f64,
) -> Result<usize, ArcsMeshError> {
    prepare_boundary::<SINGLE_CHORD_FAST_PATH>(boundary, tolerance).map(|prepared| prepared.0)
}

fn prepare_boundary<const SINGLE_CHORD_FAST_PATH: bool>(
    boundary: BoundaryPiece,
    tolerance: f64,
) -> Result<(usize, f64), ArcsMeshError> {
    let BoundaryPiece::Arc {
        radius,
        start,
        end,
        center,
        sweep,
        ..
    } = boundary
    else {
        return Ok((1, 0.0));
    };

    if SINGLE_CHORD_FAST_PATH
        && sweep_is_minor(sweep, start - center, end - center)
        && (tolerance >= radius
            || point_distance_squared(start, end) <= 4.0 * tolerance * (2.0 * radius - tolerance))
    {
        return Ok((1, 0.0));
    }

    let sweep_radians = sweep.resolve(start - center, end - center);
    let sine = (tolerance / (2.0 * radius)).sqrt().min(1.0);
    let maximum_step = 4.0 * sine.asin();
    let required_steps = (sweep_radians.abs() / maximum_step).ceil().max(1.0);
    if !required_steps.is_finite() || required_steps > MAX_VERTEX_COUNT as f64 {
        return Err(ArcsMeshError::TooManyVertices {
            limit: MAX_VERTEX_COUNT,
        });
    }

    Ok((required_steps as usize, sweep_radians))
}

fn sweep_is_minor(sweep: ArcSweep, start: Vector64, end: Vector64) -> bool {
    if let Some(sweep_radians) = sweep.resolved_radians() {
        return sweep_radians.abs() <= core::f64::consts::PI;
    }

    let cross = start.cross(end);
    if sweep.counter_clockwise() {
        cross >= 0.0
    } else {
        cross <= 0.0
    }
}

#[derive(Clone, Copy)]
enum BoundaryTraversal {
    Forward,
    Reverse,
}

fn append_boundary(
    vertices: &mut Vec<Point64>,
    boundary: BoundaryPiece,
    traversal: BoundaryTraversal,
    include_start: bool,
    steps: usize,
    sweep_radians: f64,
) {
    match boundary {
        BoundaryPiece::Line { start, end } => {
            let (start, end) = orient_endpoints(start, end, traversal);
            if include_start {
                vertices.push(start);
            }
            vertices.push(end);
        }
        BoundaryPiece::Arc {
            center,
            radius,
            start,
            end,
            ..
        } => {
            let (start, end, sweep_radians) = match traversal {
                BoundaryTraversal::Forward => (start, end, sweep_radians),
                BoundaryTraversal::Reverse => (end, start, -sweep_radians),
            };
            if include_start {
                vertices.push(start);
            }
            if steps > 1 {
                let start_radius = start - center;
                let scale = radius
                    / (start_radius.x * start_radius.x + start_radius.y * start_radius.y).sqrt();
                let mut radial = start_radius * scale;
                let angle_step = sweep_radians / steps as f64;
                let (sin_step, cos_step) = angle_step.sin_cos();
                for _ in 1..steps {
                    radial = crate::stroke_arcs::Vector64::new(
                        radial.x * cos_step - radial.y * sin_step,
                        radial.x * sin_step + radial.y * cos_step,
                    );
                    vertices.push(center + radial);
                }
            }
            vertices.push(end);
        }
    }
}

#[allow(dead_code)] // Retained as the profiling baseline for recurrent rotation.
fn append_boundary_trigonometric(
    vertices: &mut Vec<Point64>,
    boundary: BoundaryPiece,
    traversal: BoundaryTraversal,
    include_start: bool,
    steps: usize,
    sweep_radians: f64,
) {
    match boundary {
        BoundaryPiece::Line { start, end } => {
            let (start, end) = orient_endpoints(start, end, traversal);
            if include_start {
                vertices.push(start);
            }
            vertices.push(end);
        }
        BoundaryPiece::Arc {
            center,
            radius,
            start,
            end,
            ..
        } => {
            let (start, end, sweep_radians) = match traversal {
                BoundaryTraversal::Forward => (start, end, sweep_radians),
                BoundaryTraversal::Reverse => (end, start, -sweep_radians),
            };
            if include_start {
                vertices.push(start);
            }
            let start_radius = start - center;
            let start_angle = start_radius.y.atan2(start_radius.x);
            for step in 1..steps {
                let angle = start_angle + sweep_radians * step as f64 / steps as f64;
                vertices.push(Point64::new(
                    center.x + radius * angle.cos(),
                    center.y + radius * angle.sin(),
                ));
            }
            vertices.push(end);
        }
    }
}

fn orient_endpoints(
    start: Point64,
    end: Point64,
    traversal: BoundaryTraversal,
) -> (Point64, Point64) {
    match traversal {
        BoundaryTraversal::Forward => (start, end),
        BoundaryTraversal::Reverse => (end, start),
    }
}

fn direct_triangle(
    incoming_boundary: BoundaryPiece,
    outgoing_boundary: BoundaryPiece,
) -> Result<DirectArcsTriangle, ArcsMeshError> {
    let vertices = [
        boundary_start(incoming_boundary),
        boundary_end(incoming_boundary),
        boundary_start(outgoing_boundary),
    ];
    let signed_area = signed_triangle_area(vertices[0], vertices[1], vertices[2]);
    let indices = if signed_area < 0.0 {
        [2, 1, 0]
    } else {
        [0, 1, 2]
    };
    if signed_area.abs()
        <= triangle_epsilon(
            vertices[indices[0]],
            vertices[indices[1]],
            vertices[indices[2]],
        )
    {
        return Err(ArcsMeshError::NonTriangulatableBoundary);
    }

    Ok(DirectArcsTriangle {
        middle: vertices[1],
        indices,
    })
}

fn direct_quad(
    incoming_boundary: BoundaryPiece,
    outgoing_boundary: BoundaryPiece,
) -> Option<DirectArcsQuad> {
    let vertices = [
        boundary_start(incoming_boundary),
        boundary_end(incoming_boundary),
        boundary_end(outgoing_boundary),
        boundary_start(outgoing_boundary),
    ];
    let order = if polygon_signed_area(&vertices) < 0.0 {
        [3, 2, 1, 0]
    } else {
        [0, 1, 2, 3]
    };

    for position in 0..4 {
        let previous = order[(position + 3) % 4];
        let current = order[position];
        let next = order[(position + 1) % 4];
        let opposite = order[(position + 2) % 4];
        let first = vertices[previous];
        let second = vertices[current];
        let third = vertices[next];
        if signed_triangle_area(first, second, third) <= triangle_epsilon(first, second, third)
            || point_in_counter_clockwise_triangle(vertices[opposite], first, second, third)
        {
            continue;
        }

        let final_triangle = match position {
            0 => [order[1], order[2], order[3]],
            1 => [order[0], order[2], order[3]],
            2 => [order[0], order[1], order[3]],
            _ => [order[0], order[1], order[2]],
        };
        let [final_first, final_second, final_third] = final_triangle;
        if signed_triangle_area(
            vertices[final_first],
            vertices[final_second],
            vertices[final_third],
        ) <= triangle_epsilon(
            vertices[final_first],
            vertices[final_second],
            vertices[final_third],
        ) {
            return None;
        }

        // Every index comes from the four-element `order` array.
        let indices = [
            previous as u8,
            current as u8,
            next as u8,
            final_first as u8,
            final_second as u8,
            final_third as u8,
        ];
        return Some(DirectArcsQuad {
            middle: [vertices[1], vertices[2]],
            indices,
        });
    }

    None
}

#[cfg(test)]
fn triangulate_polygon(
    indices: &mut Vec<usize>,
    vertices: &[Point64],
    remaining: &mut Vec<usize>,
    preferred_fan_roots: &[usize],
) -> Result<TriangulationMethod, ArcsMeshError> {
    triangulate_polygon_general(indices, vertices, remaining, preferred_fan_roots)
}

#[inline]
fn triangulate_polygon_impl<const DIRECT_TRIANGLE_FAST_PATH: bool>(
    indices: &mut Vec<usize>,
    vertices: &[Point64],
    remaining: &mut Vec<usize>,
    preferred_fan_roots: &[usize],
) -> Result<TriangulationMethod, ArcsMeshError> {
    if DIRECT_TRIANGLE_FAST_PATH && vertices.len() == 3 {
        let signed_area = signed_triangle_area(vertices[0], vertices[1], vertices[2]);
        let (first, second, third) = if signed_area < 0.0 {
            (2, 1, 0)
        } else {
            (0, 1, 2)
        };
        if signed_area.abs() <= triangle_epsilon(vertices[first], vertices[second], vertices[third])
        {
            return Err(ArcsMeshError::NonTriangulatableBoundary);
        }
        push_triangle(indices, first, second, third);
        return Ok(TriangulationMethod::EarClipping);
    }

    triangulate_polygon_general(indices, vertices, remaining, preferred_fan_roots)
}

fn triangulate_polygon_general(
    indices: &mut Vec<usize>,
    vertices: &[Point64],
    remaining: &mut Vec<usize>,
    preferred_fan_roots: &[usize],
) -> Result<TriangulationMethod, ArcsMeshError> {
    let signed_area = polygon_signed_area(vertices);
    if let Some(fan) = find_visible_fan(vertices, preferred_fan_roots, signed_area) {
        fan.append_indices(indices);
        return Ok(TriangulationMethod::Fan);
    }

    triangulate_with_ear_clipping(indices, vertices, remaining, signed_area)
}

fn triangulate_with_ear_clipping(
    indices: &mut Vec<usize>,
    vertices: &[Point64],
    remaining: &mut Vec<usize>,
    signed_area: f64,
) -> Result<TriangulationMethod, ArcsMeshError> {
    remaining.extend(0..vertices.len());
    if signed_area < 0.0 {
        remaining.reverse();
    }

    while remaining.len() > 3 {
        if clip_one_ear(indices, vertices, remaining)? {
            continue;
        }
        if let Some(position) = redundant_vertex_position(vertices, remaining) {
            remaining.remove(position);
            continue;
        }

        return Err(ArcsMeshError::NonTriangulatableBoundary);
    }

    let [first, second, third] = remaining[..] else {
        return Err(ArcsMeshError::NonTriangulatableBoundary);
    };
    if signed_triangle_area(vertices[first], vertices[second], vertices[third])
        <= triangle_epsilon(vertices[first], vertices[second], vertices[third])
    {
        return Err(ArcsMeshError::NonTriangulatableBoundary);
    }
    push_triangle(indices, first, second, third);
    Ok(TriangulationMethod::EarClipping)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TriangulationMethod {
    Fan,
    EarClipping,
}

/// Finds an O(n) fan when a preferred root sees every boundary edge.
/// The area check rejects fans that cross a concavity or overlap.
fn find_visible_fan(
    vertices: &[Point64],
    preferred_roots: &[usize],
    signed_polygon_area: f64,
) -> Option<ValidatedFan> {
    preferred_roots
        .iter()
        .copied()
        .find_map(|root| validate_visible_fan(vertices, root, signed_polygon_area))
}

fn validate_visible_fan(
    vertices: &[Point64],
    root: usize,
    signed_polygon_area: f64,
) -> Option<ValidatedFan> {
    if vertices.len() < 3 || root >= vertices.len() || signed_polygon_area == 0.0 {
        return None;
    }

    let forward = signed_polygon_area > 0.0;
    let polygon_area = signed_polygon_area.abs();
    let fan = ValidatedFan {
        root,
        vertex_count: vertices.len(),
        forward,
    };
    let mut triangle_area = 0.0;
    for [root, first, second] in fan.triangles() {
        let area = signed_triangle_area(vertices[root], vertices[first], vertices[second]);
        if area <= triangle_epsilon(vertices[root], vertices[first], vertices[second]) {
            return None;
        }
        triangle_area += area;
    }

    let scale = vertices
        .iter()
        .map(|point| point.x * point.x + point.y * point.y)
        .fold(1.0, f64::max);
    let area_epsilon = TRIANGULATION_EPSILON * scale.max(polygon_area) * vertices.len() as f64;
    if (triangle_area - polygon_area).abs() > area_epsilon {
        return None;
    }

    Some(fan)
}

#[inline]
fn next_fan_index(index: usize, vertex_count: usize, forward: bool) -> usize {
    if forward {
        if index + 1 == vertex_count {
            0
        } else {
            index + 1
        }
    } else if index == 0 {
        vertex_count - 1
    } else {
        index - 1
    }
}

fn clip_one_ear(
    indices: &mut Vec<usize>,
    vertices: &[Point64],
    remaining: &mut Vec<usize>,
) -> Result<bool, ArcsMeshError> {
    for position in 0..remaining.len() {
        let previous = remaining[(position + remaining.len() - 1) % remaining.len()];
        let current = remaining[position];
        let next = remaining[(position + 1) % remaining.len()];
        let first = vertices[previous];
        let second = vertices[current];
        let third = vertices[next];
        if signed_triangle_area(first, second, third) <= triangle_epsilon(first, second, third) {
            continue;
        }
        let contains_vertex = remaining.iter().copied().any(|candidate| {
            candidate != previous
                && candidate != current
                && candidate != next
                && point_in_counter_clockwise_triangle(vertices[candidate], first, second, third)
        });
        if contains_vertex {
            continue;
        }

        push_triangle(indices, previous, current, next);
        remaining.remove(position);
        return Ok(true);
    }

    Ok(false)
}

fn push_triangle(indices: &mut Vec<usize>, first: usize, second: usize, third: usize) {
    indices.push(first);
    indices.push(second);
    indices.push(third);
}

fn polygon_signed_area(vertices: &[Point64]) -> f64 {
    vertices
        .iter()
        .copied()
        .zip(vertices.iter().copied().cycle().skip(1))
        .take(vertices.len())
        .map(|(first, second)| first.x * second.y - first.y * second.x)
        .sum::<f64>()
        * 0.5
}

fn point_in_counter_clockwise_triangle(
    point: Point64,
    first: Point64,
    second: Point64,
    third: Point64,
) -> bool {
    let epsilon = triangle_epsilon(first, second, third);
    signed_triangle_area(first, second, point) >= -epsilon
        && signed_triangle_area(second, third, point) >= -epsilon
        && signed_triangle_area(third, first, point) >= -epsilon
}

fn redundant_vertex_position(vertices: &[Point64], remaining: &[usize]) -> Option<usize> {
    (0..remaining.len()).find(|position| {
        let previous = vertices[remaining[(position + remaining.len() - 1) % remaining.len()]];
        let current = vertices[remaining[*position]];
        let next = vertices[remaining[(*position + 1) % remaining.len()]];
        let incoming = current - previous;
        let outgoing = next - current;
        let area = signed_triangle_area(previous, current, next).abs();
        let same_direction = incoming.x * outgoing.x + incoming.y * outgoing.y >= 0.0;

        point_distance_squared(previous, current) <= TRIANGULATION_EPSILON
            || point_distance_squared(current, next) <= TRIANGULATION_EPSILON
            || (area <= triangle_epsilon(previous, current, next) && same_direction)
    })
}

fn signed_triangle_area(first: Point64, second: Point64, third: Point64) -> f64 {
    let first_edge = second - first;
    let second_edge = third - first;
    (first_edge.x * second_edge.y - first_edge.y * second_edge.x) * 0.5
}

fn triangle_epsilon(first: Point64, second: Point64, third: Point64) -> f64 {
    TRIANGULATION_EPSILON
        * point_distance_squared(first, second)
            .max(point_distance_squared(first, third))
            .max(1.0)
}

fn point_distance_squared(first: Point64, second: Point64) -> f64 {
    let difference = second - first;
    difference.x * difference.x + difference.y * difference.y
}

fn boundary_end(boundary: BoundaryPiece) -> Point64 {
    match boundary {
        BoundaryPiece::Line { end, .. } | BoundaryPiece::Arc { end, .. } => end,
    }
}

fn boundary_start(boundary: BoundaryPiece) -> Point64 {
    match boundary {
        BoundaryPiece::Line { start, .. } | BoundaryPiece::Arc { start, .. } => start,
    }
}

fn point_is_finite(point: Point64) -> bool {
    point.x.is_finite() && point.y.is_finite()
}

#[cfg(test)]
mod tests {
    use alloc::string::ToString;

    use super::*;
    use crate::stroke_arcs::{construct, JoinConstruction, JoinInput, SegmentEnd, Vector64};

    const TEST_EPSILON: f64 = 1.0e-9;

    #[test]
    fn tessellation_rejects_zero_tolerance() {
        let join = resolved_join(right_angle_input(0.0, 0.1));

        let error = tessellate_arcs_join(&join, 0.0).expect_err("zero tolerance must fail");

        assert_eq!(
            error.to_string(),
            "tolerance must be finite and greater than zero"
        );
    }

    #[test]
    fn finer_tolerance_adds_vertices_to_curved_boundaries() {
        let join = resolved_join(right_angle_input(0.1, 0.1));

        let coarse = tessellate_arcs_join(&join, 0.5).expect("coarse tessellation must succeed");
        let fine = tessellate_arcs_join(&join, 0.001).expect("fine tessellation must succeed");

        assert!(fine.vertices().len() > coarse.vertices().len());
    }

    #[test]
    fn nearly_smooth_join_stays_inside_its_miter_limit() {
        let input = JoinInput {
            at: Point64::new(0.0, 0.0),
            incoming: SegmentEnd {
                tangent: Vector64::new(91.0, -144.0),
                curvature: -0.004_263_384_207_957_165,
            },
            outgoing: SegmentEnd {
                tangent: Vector64::new(100.0, -139.0),
                curvature: 0.004_575_682_135_228_179_5,
            },
            half_width: 14.0,
            miter_limit: 4.0,
        };
        let join = resolved_join(input);

        let mesh = tessellate_arcs_join(&join, 0.1).expect("the join must tessellate");
        let maximum_distance = mesh
            .vertices()
            .iter()
            .map(|point| (point.x * point.x + point.y * point.y).sqrt())
            .fold(0.0_f64, f64::max);

        assert!(
            maximum_distance <= input.half_width * input.miter_limit + TEST_EPSILON,
            "join reached {}, beyond the miter limit",
            maximum_distance,
        );
    }

    #[test]
    fn arc_subdivision_keeps_sagitta_within_tolerance() {
        let tolerance = 0.1;
        let radius = 10.0;
        let sweep_radians = core::f64::consts::FRAC_PI_2;
        let boundary = BoundaryPiece::Arc {
            center: Point64::new(0.0, 0.0),
            radius,
            start: Point64::new(radius, 0.0),
            end: Point64::new(0.0, radius),
            sweep: ArcSweep::lazy(true),
        };

        let steps = boundary_steps(boundary, tolerance)
            .expect("the finite arc should have a subdivision count");
        let sagitta = radius * (1.0 - (sweep_radians / steps as f64 / 2.0).cos());

        assert!(sagitta <= tolerance + TEST_EPSILON);
    }

    #[test]
    fn quarter_circle_uses_one_chord_when_tolerance_exceeds_its_sagitta() {
        let boundary = quarter_circle_boundary(10.0);

        let steps =
            boundary_steps(boundary, 3.0).expect("the finite arc should have a subdivision count");

        assert_eq!(steps, 1);
    }

    #[test]
    fn quarter_circle_is_subdivided_when_one_chord_exceeds_tolerance() {
        let boundary = quarter_circle_boundary(10.0);

        let steps =
            boundary_steps(boundary, 2.0).expect("the finite arc should have a subdivision count");

        assert_eq!(steps, 2);
    }

    #[test]
    fn major_arc_does_not_use_the_minor_arc_chord_bound() {
        let mut boundary = quarter_circle_boundary(10.0);
        let BoundaryPiece::Arc { sweep, .. } = &mut boundary else {
            unreachable!("the fixture is an arc");
        };
        *sweep = ArcSweep::resolved(3.0 * core::f64::consts::FRAC_PI_2);

        let steps =
            boundary_steps(boundary, 3.0).expect("the finite arc should have a subdivision count");

        assert_eq!(steps, 3);
    }

    #[test]
    fn lazy_major_arc_does_not_use_the_minor_arc_chord_bound() {
        let boundary = BoundaryPiece::Arc {
            center: Point64::new(0.0, 0.0),
            radius: 10.0,
            start: Point64::new(10.0, 0.0),
            end: Point64::new(0.0, -10.0),
            sweep: ArcSweep::lazy(true),
        };

        let steps =
            boundary_steps(boundary, 3.0).expect("the finite arc should have a subdivision count");

        assert_eq!(steps, 3);
    }

    #[test]
    fn recurrent_arc_rotation_stays_on_the_requested_circle() {
        let radius = 10.0;
        let center = Point64::new(3.0, -7.0);
        let end = Point64::new(center.x + radius, center.y);
        let boundary = BoundaryPiece::Arc {
            center,
            radius,
            start: end,
            end,
            sweep: ArcSweep::resolved(core::f64::consts::TAU),
        };
        let mut vertices = Vec::new();

        append_boundary(
            &mut vertices,
            boundary,
            BoundaryTraversal::Forward,
            true,
            MAX_VERTEX_COUNT,
            core::f64::consts::TAU,
        );

        assert_eq!(vertices.first(), Some(&end));
        assert_eq!(vertices.last(), Some(&end));
        assert!(vertices.iter().all(|point| {
            let radial = *point - center;
            ((radial.x * radial.x + radial.y * radial.y).sqrt() - radius).abs() <= 1.0e-10
        }));
    }

    #[test]
    fn visible_polygon_uses_the_linear_fan_path() {
        let vertices = [
            Point64::new(0.0, 0.0),
            Point64::new(2.0, 0.0),
            Point64::new(1.0, 1.0),
            Point64::new(2.0, 2.0),
            Point64::new(0.0, 2.0),
        ];
        let mut indices = Vec::new();
        let mut remaining = Vec::new();

        let method = triangulate_polygon(&mut indices, &vertices, &mut remaining, &[2])
            .expect("the visible concave polygon must triangulate");

        assert_eq!(method, TriangulationMethod::Fan);
        assert_eq!(indices.len(), (vertices.len() - 2) * 3);
    }

    #[test]
    fn validated_fan_streams_the_buffered_index_order() {
        let vertices = [
            Point64::new(0.0, 0.0),
            Point64::new(2.0, 0.0),
            Point64::new(1.0, 1.0),
            Point64::new(2.0, 2.0),
            Point64::new(0.0, 2.0),
        ];
        let signed_area = polygon_signed_area(&vertices);
        let fan = validate_visible_fan(&vertices, 2, signed_area)
            .expect("the preferred root must see every boundary edge");
        let streamed: Vec<_> = fan.triangles().flatten().collect();
        let mut buffered = Vec::new();

        fan.append_indices(&mut buffered);

        assert_eq!(streamed, buffered);
    }

    #[test]
    fn invisible_fan_root_falls_back_to_ear_clipping() {
        let vertices = [
            Point64::new(0.0, 0.0),
            Point64::new(2.0, 0.0),
            Point64::new(1.0, 1.0),
            Point64::new(2.0, 2.0),
            Point64::new(0.0, 2.0),
        ];
        let mut indices = Vec::new();
        let mut remaining = Vec::new();

        let method = triangulate_polygon(&mut indices, &vertices, &mut remaining, &[0])
            .expect("ear clipping must handle an invisible preferred root");

        assert_eq!(method, TriangulationMethod::EarClipping);
        assert_eq!(indices.len(), (vertices.len() - 2) * 3);
    }

    #[test]
    fn counter_clockwise_triangle_keeps_its_index_order() {
        let vertices = [
            Point64::new(0.0, 0.0),
            Point64::new(2.0, 0.0),
            Point64::new(0.0, 2.0),
        ];
        let mut indices = Vec::new();
        let mut remaining = Vec::new();

        triangulate_polygon(&mut indices, &vertices, &mut remaining, &[])
            .expect("the triangle must triangulate");

        assert_eq!(indices, [0, 1, 2]);
    }

    #[test]
    fn clockwise_triangle_reverses_its_index_order() {
        let vertices = [
            Point64::new(0.0, 0.0),
            Point64::new(0.0, 2.0),
            Point64::new(2.0, 0.0),
        ];
        let mut indices = Vec::new();
        let mut remaining = Vec::new();

        triangulate_polygon(&mut indices, &vertices, &mut remaining, &[])
            .expect("the triangle must triangulate");

        assert_eq!(indices, [2, 1, 0]);
    }

    #[test]
    fn representative_curved_join_uses_the_linear_fan_path() {
        let join = resolved_join(right_angle_input(0.04, 0.04));
        let mut mesh = ArcsMesh::default();

        let method = mesh
            .tessellate_impl::<true, true, true, true>(&join, 0.0001)
            .expect("the representative join must tessellate");

        assert_eq!(method, TriangulationMethod::Fan);
    }

    #[test]
    fn representative_curved_join_returns_a_direct_fan() {
        let join = resolved_join(right_angle_input(0.04, 0.04));
        let mut mesh = ArcsMesh::default();

        let output = mesh
            .tessellate_for_output(&join, 0.0001)
            .expect("the representative join must tessellate");

        let ArcsOutput::Fan { vertices, fan } = output else {
            panic!("the detailed visible join must use direct fan output");
        };
        assert_eq!(fan.triangles().len(), vertices.len() - 2);
        assert!(mesh.indices.is_empty());
    }

    #[test]
    fn representative_clipped_join_returns_a_direct_quad() {
        let mut input = right_angle_input(0.0, 0.04);
        input.miter_limit = 1.2;
        let join = resolved_join(input);
        let mut mesh = ArcsMesh::default();

        let output = mesh
            .tessellate_for_output(&join, 0.1)
            .expect("the clipped join must tessellate");

        assert!(
            matches!(output, ArcsOutput::Quad(_))
                && mesh.vertices.is_empty()
                && mesh.indices.is_empty()
                && mesh.remaining.is_empty()
        );
    }

    #[test]
    fn direct_quad_preserves_the_buffered_vertex_and_index_order() {
        let mut input = right_angle_input(0.0, 0.04);
        input.miter_limit = 1.2;
        let join = resolved_join(input);
        let buffered = tessellate_arcs_join(&join, 0.1).expect("the clipped join must tessellate");
        let direct = direct_quad(join.incoming_boundary(), join.outgoing_boundary())
            .expect("the four-vertex boundary must support direct output");
        let direct_vertices = [
            boundary_start(join.incoming_boundary()),
            direct.middle[0],
            direct.middle[1],
            boundary_start(join.outgoing_boundary()),
        ];
        let direct_indices = direct.indices.map(usize::from);

        assert_eq!(
            (direct_vertices.as_slice(), direct_indices.as_slice()),
            (buffered.vertices(), buffered.indices())
        );
    }

    #[test]
    fn direct_quad_matches_ear_clipping_for_both_windings_and_a_concavity() {
        let cases = [
            [
                Point64::new(0.0, 0.0),
                Point64::new(2.0, 0.0),
                Point64::new(2.0, 2.0),
                Point64::new(0.0, 2.0),
            ],
            [
                Point64::new(0.0, 2.0),
                Point64::new(2.0, 2.0),
                Point64::new(2.0, 0.0),
                Point64::new(0.0, 0.0),
            ],
            [
                Point64::new(0.0, 0.0),
                Point64::new(2.0, 0.0),
                Point64::new(0.75, 0.5),
                Point64::new(0.0, 2.0),
            ],
        ];

        for vertices in cases {
            let incoming = BoundaryPiece::Line {
                start: vertices[0],
                end: vertices[1],
            };
            let outgoing = BoundaryPiece::Line {
                start: vertices[3],
                end: vertices[2],
            };
            let direct = direct_quad(incoming, outgoing)
                .expect("the valid four-vertex boundary must support direct output");
            let mut buffered_indices = Vec::new();
            let mut remaining = Vec::new();

            triangulate_with_ear_clipping(
                &mut buffered_indices,
                &vertices,
                &mut remaining,
                polygon_signed_area(&vertices),
            )
            .expect("the valid four-vertex boundary must triangulate");

            assert_eq!(direct.indices.map(usize::from).as_slice(), buffered_indices);
        }
    }

    #[test]
    fn direct_quad_rejects_a_degenerate_boundary_for_the_buffered_fallback() {
        let incoming = BoundaryPiece::Line {
            start: Point64::new(0.0, 0.0),
            end: Point64::new(1.0, 0.0),
        };
        let outgoing = BoundaryPiece::Line {
            start: Point64::new(3.0, 0.0),
            end: Point64::new(2.0, 0.0),
        };

        assert!(direct_quad(incoming, outgoing).is_none());
    }

    #[test]
    fn tessellation_uses_the_clip_edge_as_part_of_the_outer_chain() {
        let mut input = right_angle_input(0.0, 0.1);
        input.miter_limit = 1.2;
        let join = resolved_join(input);
        let clip = join.clip.expect("the input should produce a clip edge");

        let mesh = tessellate_arcs_join(&join, 0.01).expect("the clipped join must tessellate");

        assert!(mesh
            .vertices()
            .windows(2)
            .any(|edge| point_is_near(edge[0], clip.incoming)
                && point_is_near(edge[1], clip.outgoing)));
    }

    #[test]
    fn every_triangle_has_counter_clockwise_winding_for_a_left_turn() {
        let input = right_angle_input(0.1, 0.1);
        let join = resolved_join(input);

        let mesh = tessellate_arcs_join(&join, 0.01).expect("the curved join must tessellate");

        assert!(all_triangles_are_counter_clockwise(&mesh));
    }

    #[test]
    fn every_triangle_has_counter_clockwise_winding_for_a_right_turn() {
        let mut input = right_angle_input(-0.1, -0.1);
        input.outgoing.tangent = Vector64::new(0.0, -1.0);
        let join = resolved_join(input);

        let mesh =
            tessellate_arcs_join(&join, 0.01).expect("the reflected curved join must tessellate");

        assert!(all_triangles_are_counter_clockwise(&mesh));
    }

    #[test]
    fn all_indices_reference_existing_vertices() {
        let input = right_angle_input(0.1, 0.1);
        let join = resolved_join(input);

        let mesh = tessellate_arcs_join(&join, 0.01).expect("the curved join must tessellate");

        assert!(mesh
            .indices()
            .iter()
            .all(|index| *index < mesh.vertices().len()));
    }

    #[test]
    fn representative_support_combinations_produce_valid_meshes() {
        let cases = [(0.0, 0.04, 1.2), (0.04, 0.04, 4.0)];

        for (incoming_curvature, outgoing_curvature, miter_limit) in cases {
            let mut input = right_angle_input(incoming_curvature, outgoing_curvature);
            input.half_width = 10.0;
            input.miter_limit = miter_limit;
            let join = resolved_join(input);
            let mesh = tessellate_arcs_join(&join, 0.25).expect("diagnostic join must tessellate");

            assert!(
                all_triangles_are_counter_clockwise(&mesh),
                "invalid mesh for curvatures {}, {}",
                incoming_curvature,
                outgoing_curvature
            );
        }
    }

    #[test]
    fn triangles_cover_a_curved_join_without_overlap() {
        let input = right_angle_input(0.1, 0.1);
        let join = resolved_join(input);

        let mesh = tessellate_arcs_join(&join, 0.25).expect("the curved join must tessellate");
        let polygon_area = polygon_signed_area(mesh.vertices()).abs();
        let triangle_area = mesh
            .indices()
            .chunks_exact(3)
            .map(|triangle| {
                signed_triangle_area(
                    mesh.vertices()[triangle[0]],
                    mesh.vertices()[triangle[1]],
                    mesh.vertices()[triangle[2]],
                )
            })
            .sum::<f64>();

        assert!((triangle_area - polygon_area).abs() <= TEST_EPSILON * polygon_area.max(1.0));
    }

    fn resolved_join(input: JoinInput) -> ResolvedArcsJoin {
        let construction = construct(input).expect("test join construction must succeed");
        let JoinConstruction::Arcs(join) = construction else {
            panic!("test input should produce an arcs join");
        };

        join
    }

    fn quarter_circle_boundary(radius: f64) -> BoundaryPiece {
        BoundaryPiece::Arc {
            center: Point64::new(0.0, 0.0),
            radius,
            start: Point64::new(radius, 0.0),
            end: Point64::new(0.0, radius),
            sweep: ArcSweep::lazy(true),
        }
    }

    fn right_angle_input(incoming_curvature: f64, outgoing_curvature: f64) -> JoinInput {
        JoinInput {
            at: Point64::new(0.0, 0.0),
            incoming: SegmentEnd {
                tangent: Vector64::new(1.0, 0.0),
                curvature: incoming_curvature,
            },
            outgoing: SegmentEnd {
                tangent: Vector64::new(0.0, 1.0),
                curvature: outgoing_curvature,
            },
            half_width: 2.0,
            miter_limit: 4.0,
        }
    }

    fn all_triangles_are_counter_clockwise(mesh: &ArcsMesh) -> bool {
        mesh.indices().chunks_exact(3).all(|indices| {
            let first = mesh.vertices()[indices[0]];
            let second = mesh.vertices()[indices[1]];
            let third = mesh.vertices()[indices[2]];
            let first_edge = second - first;
            let second_edge = third - first;
            first_edge.x * second_edge.y - first_edge.y * second_edge.x > TEST_EPSILON
        })
    }

    fn point_is_near(actual: Point64, expected: Point64) -> bool {
        (actual.x - expected.x).abs() <= TEST_EPSILON
            && (actual.y - expected.y).abs() <= TEST_EPSILON
    }
}
