//!
//! Y-monotone decomposition and triangulation of shapes.
//!
//! This module provides the tools to generate triangles from arbitrary shapes with connectivity
//! information (using a half-edge connectivity kernel).
//!
//! The implementation inspired by the book Computational Geometry, Algorithms And Applications 3rd edition.
//!
//! Note that a lot of the comments and variable labels in this module assume a coordinate
//! system where y is pointing downwards

use std::mem::swap;
use std::f32::consts::PI;

use tesselation::{ VertexId, VertexSlice, Direction, error };
use tesselation::vectors::{ Position2D };
use tesselation::vertex_builder::{ VertexBufferBuilder };
use tesselation::polygon::*;
use tesselation::connection::{ Connections };
use tesselation::sweep_line;
use tesselation::sweep_line::{
    EventType, compute_event_type, SweepLine, SweepLineEdge, is_below,
};

use vodk_alloc::*;
use vodk_id::*;
use vodk_math::{ Vec2 };

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct DecompositionError;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum TriangulationError {
    NotMonotone,
    InvalidPath,
    MissingFace,
    TriangleCount,
}

/// Can perform y-monotone decomposition on a connectivity kernel.
///
/// This object holds on to the memory that was allocated during previous
/// decompositions in order to avoid allocating during the next decompositions
/// if possible.
pub struct DecompositionContext;

impl DecompositionContext {
    pub fn new() -> DecompositionContext { DecompositionContext }

    /// Applies an y_monotone decomposition of a face in a connectivity kernel.
    ///
    /// This operation will add faces and edges to the connectivity kernel.
    pub fn y_monotone_polygon_decomposition<
        V: Position2D
    >(
        &mut self,
        polygon: ComplexPolygonSlice,
        vertex_positions: VertexSlice<V>,
        events: sweep_line::SortedEventSlice,
        connections: &mut Connections<ComplexPointId>
    ) -> Result<(), DecompositionError> {

        let mut sweep_line = SweepLine::new();

        for &e in events.events {
            let prev = polygon.previous(e);
            let next = polygon.next(e);
            let current_position = vertex_positions[polygon.vertex(e)].position();
            let previous_position = vertex_positions[polygon.vertex(prev)].position();
            let next_position = vertex_positions[polygon.vertex(next)].position();
            let vertex_type = compute_event_type(previous_position, current_position, next_position);

            sweep_line.set_current_position(current_position);
            let mut edge = SweepLineEdge {
                key: e,
                from: current_position,
                to: next_position,
                helper: Some((e, vertex_type)),
            };

            match vertex_type {
                EventType::Start => {
                    sweep_line.add(edge);
                }
                EventType::End => {
                    let prev_idx = sweep_line.find(prev).unwrap();
                    connect_with_helper_if_merge_vertex(e, prev_idx, &mut sweep_line, connections);
                    sweep_line.remove(prev);
                }
                EventType::Split => {
                    let right_idx = sweep_line.find_index_right_of_current_position().unwrap();

                    if let Some((helper_edge,_)) = sweep_line.get_helper(right_idx) {
                        connections.add_connection(e, helper_edge);
                    } else {
                        return Err(DecompositionError);
                    }
                    sweep_line.set_helper(right_idx, e, vertex_type);
                    sweep_line.add(edge);
                }
                EventType::Merge => {
                    let prev_idx = sweep_line.find(prev).unwrap();
                    connect_with_helper_if_merge_vertex(e, prev_idx, &mut sweep_line, connections);
                    sweep_line.remove(prev);

                    let right_idx = sweep_line.find_index_right_of_current_position().unwrap();
                    connect_with_helper_if_merge_vertex(e, right_idx, &mut sweep_line, connections);
                    sweep_line.set_helper(right_idx, e, vertex_type);
                }
                EventType::Right => {
                    let prev_idx = sweep_line.find(prev).unwrap();
                    connect_with_helper_if_merge_vertex(e, prev_idx, &mut sweep_line, connections);
                    sweep_line.remove(prev);
                    sweep_line.add(edge);
                }
                EventType::Left => {
                    let right_idx = sweep_line.find_index_right_of_current_position().unwrap();
                    connect_with_helper_if_merge_vertex(e, right_idx, &mut sweep_line, connections);

                    sweep_line.set_helper(right_idx, e, vertex_type);
                }
            }
        }

        return Ok(());
    }
}

fn connect_with_helper_if_merge_vertex(current_edge: ComplexPointId,
                                       sl_index: usize,
                                       sl: &mut SweepLine,
                                       connections: &mut Connections<ComplexPointId>) {
    if let Some((h, EventType::Merge)) = sl.get_helper(sl_index) {
        //println!("      helper {:?} of {:?} is a merge vertex", h, helper_edge);
        connections.add_connection(h, current_edge);
    }
}


/// Returns true if the face is y-monotone in O(n).
pub fn is_y_monotone<V: Position2D>(
    polygon: PolygonSlice,
    vertex_positions: VertexSlice<V>,
) -> bool {
    for point in polygon.point_ids() {
        let previous = vertex_positions[polygon.previous_vertex(point)].position();
        let current = vertex_positions[polygon.vertex(point)].position();
        let next = vertex_positions[polygon.next_vertex(point)].position();

        match compute_event_type(previous, current, next) {
            EventType::Split | EventType::Merge => {
                return false;
            }
            _ => {}
        }
    }
    return true;
}

pub trait Write<T> { fn write(&mut self, data: T); }

/// A dummy implementation that doesn't write anything. Useful when ignoring the output
/// of an algorithm.
impl<T> Write<T> for () { fn write(&mut self, _data: T) {} }

/// Write into a Vec.
impl<T> Write<T> for Vec<T> { fn write(&mut self, data: T) { self.push(data) } }

/// Writes triangles as indices in a &[u16].
pub struct SliceTriangleWriter<'l> {
    indices: &'l mut[u16],
    offset: usize,
}

impl<'l> Write<[VertexId; 3]> for SliceTriangleWriter<'l> {
    fn write(&mut self, indices: [VertexId; 3]) {
        debug_assert!(indices[0] != indices[1]);
        debug_assert!(indices[0] != indices[2]);
        debug_assert!(indices[1] != indices[2]);
        self.indices[self.offset] = indices[0].to_index() as u16;
        self.indices[self.offset+1] = indices[1].to_index() as u16;
        self.indices[self.offset+2] = indices[2].to_index() as u16;
        self.offset += 3;
    }
}

impl<'l> SliceTriangleWriter<'l> {
    pub fn new(buffer: &'l mut[u16]) -> SliceTriangleWriter {
        SliceTriangleWriter {
            indices: buffer,
            offset: 0,
        }
    }
}

/// Can perform y-monotone triangulation on a connectivity kernel.
///
/// This object holds on to the memory that was allocated during previous
/// triangulations, in order to avoid allocating during the next triangulations
/// if possible.
pub struct TriangulationContext {
    vertex_stack_storage: Allocation,
}

impl TriangulationContext {
    /// Constructor.
    pub fn new() -> TriangulationContext {
        TriangulationContext {
            vertex_stack_storage: Allocation::empty()
        }
    }

    /// Computes an y-monotone triangulation of a face in the connectivity kernel,
    /// outputing triangles by pack of 3 vertex indices in a TriangleStream.
    ///
    /// Returns the number of indices that were added to the stream.
    pub fn y_monotone_triangulation<
        P: Position2D,
        Output: VertexBufferBuilder<Vec2>
    >(
        &mut self,
        polygon: PolygonSlice,
        vertex_positions: VertexSlice<P>,
        output: &mut Output,
    ) -> Result<(), TriangulationError> {

        // for convenience
        let vertex = |circ: Circulator| { vertex_positions[polygon.vertex(circ.point)].position() };
        let next = |circ: Circulator| { Circulator { point: polygon.advance(circ.point, circ.direction), direction: circ.direction } };
        let previous = |circ: Circulator| { Circulator { point: polygon.advance(circ.point, circ.direction.reverse()), direction: circ.direction } };

        #[derive(Copy, Clone, Debug, PartialEq)]
        struct Circulator {
            point: PointId,
            direction: Direction,
        }

        let mut up = Circulator { point: polygon.first_point(), direction: Direction::Forward };
        let mut down = up.clone();

        loop {
            down = next(down);
            if vertex(up).y != vertex(down).y {
                break;
            }
            if down == up {
                // Avoid an infnite loop in the degenerate case where all vertices are in the same position.
                break;
            }
        }

        up.direction = if is_below(vertex(up), vertex(down)) { Direction::Forward }
                       else { Direction::Backward };

        down.direction = up.direction.reverse();

        // Find the bottom-most vertex (with the highest y value)
        let mut big_y = vertex(down);
        let guard = down;
        loop {
            down = next(down);
            let new_y = vertex(down);
            if is_below(big_y, new_y) {
                down = previous(down);
                break;
            }
            big_y = new_y;
            if down == guard {
                // We have looped through all vertices already because of
                // a degenerate input, avoid looping infinitely.
                break;
            }
        }
        // find the top-most vertex (with the smallest y value)
        let mut small_y = vertex(up);
        let guard = up;
        loop {
            up = next(up);
            let new_y = vertex(up);
            if is_below(new_y, small_y) {
                up = previous(up);
                break;
            }
            small_y = new_y;
            if up == guard {
                // We have looped through all vertices already because of
                // a degenerate input, avoid looping infinitely.
                break;
            }
        }

        // now that we have the top-most vertex, we will circulate simulataneously
        // from the left and right chains until we reach the bottom-most vertex

        // main chain
        let mut m = up.clone();

        // opposite chain
        let mut o = up.clone();
        m.direction = Direction::Forward;
        o.direction = Direction::Backward;

        m = next(m);
        o = next(o);

        if is_below(vertex(m), vertex(o)) {
            swap(&mut m, &mut o);
        }

        m = previous(m);
        // previous
        let mut p = m;

        // vertices already visited, waiting to be connected
        let mut storage = Allocation::empty();
        swap(&mut storage, &mut self.vertex_stack_storage);
        let mut vertex_stack: Vec<Circulator> = create_vec_from(storage);

        let mut triangle_count = 0;
        let mut i: i32 = 0;

        loop {
            //println!("   -- m: {:?}  o: {:?}", m.point, o.point);

            // walk edges from top to bottom, alternating between the left and
            // right chains. The chain we are currently iterating over is the
            // main chain (m) and the other one the opposite chain (o).
            // p is the previous iteration, regardless of which chain it is on.
            if is_below(vertex(m), vertex(o)) || m == down {
                swap(&mut m, &mut o);
            }

            if i < 2 {
                vertex_stack.push(m);
            } else {
                if vertex_stack.len() > 0 && m.direction != vertex_stack[vertex_stack.len()-1].direction {
                    for i in 0..vertex_stack.len() - 1 {
                        let id_1 = polygon.vertex(vertex_stack[i].point);
                        let id_2 = polygon.vertex(vertex_stack[i+1].point);
                        let id_opp = polygon.vertex(m.point);

                        output.push_indices(id_opp.handle, id_1.handle, id_2.handle);
                        triangle_count += 1;
                    }

                    vertex_stack.clear();

                    vertex_stack.push(p);
                    vertex_stack.push(m);

                } else {

                    let mut last_popped = vertex_stack.pop();

                    loop {
                        if vertex_stack.len() < 1 {
                            break;
                        }
                        let mut id_1 = polygon.vertex(vertex_stack[vertex_stack.len()-1].point);
                        let id_2 = polygon.vertex(last_popped.unwrap().point);
                        let mut id_3 = polygon.vertex(m.point);

                        if m.direction == Direction::Backward {
                            swap(&mut id_1, &mut id_3);
                        }

                        let v1 = vertex_positions[id_1].position();
                        let v2 = vertex_positions[id_2].position();
                        let v3 = vertex_positions[id_3].position();
                        if (v1 - v2).directed_angle(v3 - v2) > PI {
                            output.push_indices(id_1.handle, id_2.handle, id_3.handle);
                            triangle_count += 1;

                            last_popped = vertex_stack.pop();

                        } else {
                            break;
                        }
                    } // loop 2

                    if let Some(item) = last_popped {
                        vertex_stack.push(item);
                    }
                    vertex_stack.push(m);

                }
            }

            if m.point == down.point {
                if o.point == down.point {
                    break;
                }
            }

            i += 1;
            p = m;
            m = next(m);
            debug_assert!(!is_below(vertex(p), vertex(m)));
        }
        if triangle_count != polygon.num_vertices() as usize - 2 {
            return error(TriangulationError::TriangleCount);
        }

        // Keep the buffer to avoid reallocating it next time, if possible.
        self.vertex_stack_storage = vec::recycle(vertex_stack);
        return Ok(());
    }
}


#[cfg(test)]
use vodk_math::{ vec2 };

#[cfg(test)]
struct TestShape<'l> {
    label: &'l str,
    main: &'l[[f32;2]],
    holes: &'l[&'l[[f32;2]]],
}

#[cfg(test)]
fn test_shape(shape: &TestShape, angle: f32) {
    use tesselation::{ vertex_id_range, };
    use tesselation::connection::apply_connections;
    use tesselation::vertex_builder::{ VertexBuffers, simple_vertex_builder, };

    let mut vertices: Vec<Vec2> = Vec::new();
    vertices.extend(shape.main.iter().map(|v|{vec2(v[0], v[1])}));
    for hole in shape.holes {
        vertices.extend(hole.iter().map(|v|{vec2(v[0], v[1])}));
    }

    println!("vertices: {:?}", vertices);

    for ref mut v in &mut vertices[..] {
        // rotate all points around (0, 0).
        let cos = angle.cos();
        let sin = angle.sin();
        let (x, y) = (v.x, v.y);
        v.x = x*cos + y*sin;
        v.y = y*cos - x*sin;
    }

    println!("transformed vertices: {:?}", vertices);

    let mut polygon = ComplexPolygon::new();
    polygon.add_sub_polygon(vertex_id_range(0, shape.main.len() as u16), PolygonInfo::default());

    let mut from = shape.main.len() as u16;
    for hole in shape.holes {
        let to = from + hole.len() as u16;
        polygon.add_sub_polygon(ReverseIdRange::new(vertex_id_range(from, to)), PolygonInfo::default());
        from = to;
    }

    let vertex_positions = VertexSlice::new(&vertices[..]);
    let mut ctx = DecompositionContext::new();
    let mut connections = Connections::new();

    let mut sorted_events = sweep_line::EventVector::new();
    sorted_events.set_polygon(polygon.as_slice(), vertex_positions);
    //let mut algo = YMonotoneDecomposition::new();
    //let res = sweep_line::apply_y_sweep(&polygon, vertex_positions, sorted_events.as_slice(), &mut algo);

    let res = ctx.y_monotone_polygon_decomposition(
        polygon.as_slice(), vertex_positions, sorted_events.as_slice(), &mut connections
    );
    assert_eq!(res, Ok(()));

    let mut y_monotone_polygons = Vec::new();
    let res = apply_connections(polygon.as_slice(), vertex_positions, &mut connections, &mut y_monotone_polygons);
    assert!(res.is_ok());

    let mut triangulator = TriangulationContext::new();
    let mut buffers: VertexBuffers<Vec2> = VertexBuffers::new();

    println!("\n\n -- There are {:?} monotone polygons", y_monotone_polygons.len());
    for poly in y_monotone_polygons {
        println!("\n\n -- Triangulating polygon with vertices {:?}", poly.vertices);
        let mut i = 0;
        for &p in &poly.vertices {
            println!("     -> point {} vertex {:?} position {:?}", i, p, vertex_positions[p].position());
            i += 1;
        }
        assert!(is_y_monotone(poly.as_slice(), vertex_positions));
        let res = triangulator.y_monotone_triangulation(
            poly.as_slice(),
            vertex_positions,
            &mut simple_vertex_builder(&mut buffers)
        );
        assert_eq!(res, Ok(()));
    }

    for n in 0 .. buffers.indices.len()/3 {
        println!(" ===> {} {} {}", buffers.indices[n*3], buffers.indices[n*3+1], buffers.indices[n*3+2]);
    }
}

#[cfg(test)]
fn test_all_shapes(tests: &[TestShape]) {
    let mut angle = 0.0;
    while angle < 2.0*PI {
        for shape in tests {
            println!("\n\n\n   -- shape: {} (angle {:?})", shape.label, angle);
            test_shape(shape, angle);
        }
        angle += 0.005;
    }
}

#[test]
fn test_triangulate() {
    test_all_shapes(&[
        TestShape {
            label: &"Simple triangle",
            main: &[
                [-10.0, 5.0],
                [0.0, -5.0],
                [10.0, 5.0],
            ],
            holes: &[],
        },
        TestShape {
            label: &"Simple triangle",
            main: &[
                [1.0, 2.0],
                [1.5, 3.0],
                [0.0, 4.0],
            ],
            holes: &[],
        },
        TestShape {
            label: &"Simple rectangle",
            main: &[
                [1.0, 2.0],
                [1.5, 3.0],
                [0.0, 4.0],
                [-1.0, 1.0],
            ],
            holes: &[],
        },
        TestShape {
            label: &"",
            main: &[
                [0.0, 0.0],
                [3.0, 0.0],
                [2.0, 1.0],
                [3.0, 2.0],
                [2.0, 3.0],
                [0.0, 2.0],
                [1.0, 1.0],
            ],
            holes: &[],
        },
        TestShape {
            label: &"",
            main: &[
                [0.0, 0.0],
                [1.0, 1.0],
                [2.0, 0.0],
                [2.0, 4.0],
                [1.0, 3.0],
                [0.0, 4.0],
            ],
            holes: &[],
        },
        TestShape {
            label: &"",
            main: &[
                [0.0, 2.0],
                [1.0, 2.0],
                [0.0, 1.0],
                [2.0, 0.0],
                [3.0, 1.0],// 4
                [4.0, 0.0],
                [3.0, 2.0],
                [2.0, 1.0],// 7
                [3.0, 3.0],
                [2.0, 4.0]
            ],
            holes: &[],
        },
        TestShape {
            label: &"",
            main: &[
                [0.0, 0.0],
                [1.0, 0.0],
                [2.0, 0.0],
                [3.0, 0.0],
                [3.0, 1.0],
                [3.0, 2.0],
                [3.0, 3.0],
                [2.0, 3.0],
                [1.0, 3.0],
                [0.0, 3.0],
                [0.0, 2.0],
                [0.0, 1.0],
            ],
            holes: &[],
        },
    ]);
}

#[test]
fn test_triangulate_holes() {
    test_all_shapes(&[
        TestShape {
            label: &"Triangle with triangle hole",
            main: &[
                [-11.0, 5.0],
                [0.0, -5.0],
                [10.0, 5.0],
            ],
            holes: &[
                &[
                    [-5.0, 2.0],
                    [0.0, -2.0],
                    [4.0, 2.0],
                ]
            ]
        },
        TestShape {
            label: &"Square with triangle hole",
            main: &[
                [-10.0, -10.0],
                [ 10.0, -10.0],
                [ 10.0,  10.0],
                [-10.0,  10.0],
            ],
            holes: &[
                &[
                    [-4.0, 2.0],
                    [0.0, -2.0],
                    [4.0, 2.0],
                ]
            ],
        },
        TestShape {
            label: &"Square with two holes",
            main: &[
                [-10.0, -10.0],
                [ 10.0, -10.0],
                [ 10.0,  10.0],
                [-10.0,  10.0],
            ],
            holes: &[
                &[
                    [-8.0, -8.0],
                    [-4.0, -8.0],
                    [4.0, 8.0],
                    [-8.0, 8.0],
                ],
                &[
                    [8.0, -8.0],
                    [6.0, 7.0],
                    [-2.0, -8.0],
                ]
            ],
        },
        TestShape {
            label: &"",
            main: &[
                [0.0, 0.0],
                [1.0, 1.0],
                [2.0, 1.0],
                [3.0, 0.0],
                [4.0, 0.0],
                [5.0, 0.0],
                [3.0, 4.0],
                [1.0, 4.0],
            ],
            holes: &[
                &[
                    [2.0, 2.0],
                    [3.0, 2.0],
                    [2.5, 3.0],
                ]
            ],
        },
    ]);
}

#[test]
#[ignore]
fn test_triangulate_degenerate() {
    test_all_shapes(&[
        TestShape {
            label: &"3 points on the same line (1)",
            main: &[
                [0.0, 0.0],
                [0.0, 1.0],
                [0.0, 2.0],
            ],
            holes: &[],
        },
        TestShape {
            label: &"3 points on the same line (2)",
            main: &[
                [0.0, 0.0],
                [0.0, 2.0],
                [0.0, 1.0],
            ],
            holes: &[],
        },
        TestShape {
            label: &"All points in the same place (1)",
            main: &[
                [0.0, 0.0],
                [0.0, 0.0],
                [0.0, 0.0],
            ],
            holes: &[],
        },
        TestShape {
            label: &"All points in the same place (2)",
            main: &[
                [0.0, 0.0],
                [0.0, 0.0],
                [0.0, 0.0],
                [0.0, 0.0],
            ],
            holes: &[],
        },
        TestShape {
            label: &"Geometry comes back along a line on the y axis",
            main: &[
                [0.0, 0.0],
                [0.0, 2.0],
                [0.0, 1.0],
                [-1.0, 0.0],
            ],
            holes: &[],
        },
    ]);
}

#[test]
#[ignore]
fn test_triangulate_failures() {
    // Test cases that are known to fail but we want to make work eventually.
    test_all_shapes(&[
        TestShape {
            label: &"Duplicate point (1)",
            main: &[
                [0.0, 0.0],
                [1.0, 0.0],
                [1.0, 0.0],
            ],
            holes: &[],
        },
        TestShape {
            label: &"Duplicate point (2)",
            main: &[
                [0.0, 0.0],
                [1.0, 0.0],
                [1.0, 0.0],
                [1.0, 1.0],
            ],
            holes: &[],
        },
        TestShape {
            label: &"Duplicate point (3)",
            main: &[
              [0.0, 0.0],
              [1.0, 0.0],
              [1.0, 0.0],
              [1.0, 0.0],
              [1.0, 1.0],
            ],
            holes: &[],
        },
        TestShape {
            label: &"Geometry comes back along a line on the x axis",
            main: &[
                [0.0, 0.0],
                [2.0, 0.0],
                [1.0, 0.0],
                [0.0, 1.0],
            ],
            holes: &[],
        },
        TestShape {
            label: &"Geometry comes back along lines",
            main: &[
            // a mix of the previous 2 cases
                [0.0, 0.0],
                [2.0, 0.0],
                [1.0, 0.0],
                [1.0, 2.0],
                [1.0, 1.0],
                [-1.0, 1.0],
                [0.0, 1.0],
                [0.0, -1.0],
            ],
            holes: &[],
        },
        TestShape {
            label: &"...->A->B->A->...",
            main: &[
                // outer
                [0.0, 0.0],
                [1.0, 1.0], // <--
                [2.0, 1.0],
                [3.0, 0.0],
                [4.0, 0.0],
                [5.0, 0.0],
                [3.0, 4.0],
                [1.0, 4.0],
                [1.0, 1.0], // <--
            ],
            holes: &[
                &[
                    [2.0, 2.0],
                    [3.0, 2.0],
                    [2.5, 3.0],
                ]
            ],
        },
        TestShape {
            label: &"zero-area geometry shaped like a cross going back to the same position at the center",
            main: &[
                [1.0, 1.0],
                [2.0, 1.0],
                [1.0, 1.0],
                [2.0, 1.0],
                [1.0, 1.0],
                [0.0, 1.0],
                [1.0, 1.0],
                [1.0, 0.0],
            ],
            holes: &[],
        },
        TestShape {
            label: &"Self-intersection",
            main: &[
                [0.0, 0.0],
                [1.0, 0.0],
                [1.0, 1.0],
                [0.0, 1.0],
                [3.0, 0.0],
                [3.0, 1.0],
            ],
            holes: &[],
        },
    ]);
}
