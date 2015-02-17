// Implementation inspired by the course at http://research.engineering.wustl.edu/~pless/546/lectures/l7.html

use halfedge::*;
use math::vector::*;
use std::num::Float;
use std::cmp::{Ordering, PartialOrd};
use std::iter::FromIterator;
use std::collections::HashMap;
use math::units::world;
use std::fmt::Show;
use std::mem::swap;

#[derive(Show, Copy, Clone)]
enum VertexType {
    Start,
    End,
    Split,
    Merge,
    Left,
    Right,
}

/// Angle between v1 and v2 (oriented clockwise with y pointing downward)
pub fn directed_angle<T>(v1: Vector2D<T>, v2: Vector2D<T>) -> f32 {
    let a = (v2.y).atan2(v2.x) - (v1.y).atan2(v1.x);
    return if a < 0.0 { a + 2.0 * PI } else { a };
}

pub fn deg(rad: f32) -> f32 {
    return rad / PI * 180.0;
}

fn get_vertex_type<T: Copy>(prev: Vector2D<T>, current: Vector2D<T>, next: Vector2D<T>) -> VertexType {
    // assuming clockwise path winding order
    let interrior_angle = directed_angle(next - current, prev - current);

    if current.y > prev.y && current.y >= next.y {
        if interrior_angle >= PI {
            return VertexType::Merge;
        } else {
            return VertexType::End;
        }
    }

    if current.y < prev.y && current.y <= next.y {
        if interrior_angle >= PI {
            return VertexType::Split;
        } else {
            return VertexType::Start;
        }
        return VertexType::End;
    }

    return if prev.y < next.y { VertexType::Right } else { VertexType::Left };
}

pub fn find(slice: &[EdgeId], item: EdgeId) -> Option<usize> {
    for i in range(0, slice.len()) {
        if slice[i] == item { return Some(i); }
    }
    return None;
}
pub fn sort_x<T: Copy>(slice: &mut[EdgeId], kernel: &ConnectivityKernel, path: &[Vector2D<T>]) {
    slice.sort_by(|a, b| {
        path[kernel.edge(*a).vertex.as_index()].y.partial_cmp(&path[kernel.edge(*b).vertex.as_index()].y).unwrap()
    });    
}

pub fn y_monotone_decomposition<T: Copy+Show>(
    kernel: &mut ConnectivityKernel,
    path: &[Vector2D<T>]
) {
    let mut sorted_edges: Vec<EdgeId> = FromIterator::from_iter(kernel.edge_ids());

    // sort indices by increasing y coordinate of the corresponding vertex
    sorted_edges.sort_by(|a, b| {
        if path[kernel.edge(*a).vertex.as_index()].y > path[kernel.edge(*b).vertex.as_index()].y {
            return Ordering::Greater;
        }
        if path[kernel.edge(*a).vertex.as_index()].y < path[kernel.edge(*b).vertex.as_index()].y {
            return Ordering::Less;
        }
        if path[kernel.edge(*a).vertex.as_index()].x < path[kernel.edge(*b).vertex.as_index()].x {
            return Ordering::Greater;
        }
        if path[kernel.edge(*a).vertex.as_index()].x > path[kernel.edge(*b).vertex.as_index()].x {
            return Ordering::Less;
        }
        return Ordering::Equal;
    });

    // list of edges that intercept the sweep line, sorted by increasing x coordinate
    let mut sweep_status: Vec<EdgeId> = vec!();
    let mut helper: HashMap<usize, (EdgeId, VertexType)> = HashMap::new();

    for e in sorted_edges.iter() {
        let edge = *kernel.edge(*e);
        let current_vertex = path[edge.vertex.as_index()];
        let previous_vertex = path[kernel.edge(edge.prev).vertex.as_index()];
        let next_vertex = path[kernel.edge(edge.next).vertex.as_index()];
        let vertex_type = get_vertex_type(previous_vertex, current_vertex, next_vertex);
        println!(" vertex {} type {:?}", edge.vertex.as_index(), vertex_type);
        match vertex_type {
            VertexType::Start => {
                // Insert this vertex and its edges into the sweep line status.
                sweep_status.push(*e);
                sweep_status.push(edge.next);
                sort_x(&mut sweep_status[], kernel, path);
                // Set the helper of the left edge to current_vertex.
                if let Some((prev_helper, VertexType::Merge)) = helper.insert(e.as_index(), (*e, VertexType::Start)) {
                    kernel.split_face(prev_helper, *e);
                }
            }
            VertexType::End => {
                // Delete both edges from the sweep line status.
                sweep_status.retain(|item|{ *item != *e && *item != edge.next });
            }
            VertexType::Split => {
                // Search the sweep line status to find the edge e lying immediately to the left of v.
                for i in range(0, sweep_status.len()) {
                    // the sweep status is sorted by increasing x.
                    if path[kernel.edge(sweep_status[i]).vertex.as_index()].x > current_vertex.x {
                        let (h,_) = helper[sweep_status[i-1].as_index()];
                        // Add a diagonal connecting v to helper(e).
                        kernel.split_face(*e, h);
                        break;
                    }
                }

                // Add the two edges incident to v in the sweep line status, and make v the
                sweep_status.push(*e);
                sweep_status.push(edge.next);
                sort_x(&mut sweep_status[], kernel, path);
                // helper of the left-most of these two edges and make v the new helper of e.
                if let Some((prev_helper, VertexType::Merge)) = helper.insert(edge.next.as_index(), (*e, VertexType::Split)) {
                    kernel.split_face(prev_helper, *e);
                }
            }
            VertexType::Merge => {
                // Find the two edges incident to this vertex in the sweep line status (they must be adjacent).
                // Delete them both.
                sweep_status.retain(|item|{ *item != *e && *item != edge.next });
                // Search the sweep line status to find the edge lying immediately to the left of v.
                for i in range(0, sweep_status.len()) {
                    // the sweep status is sorted by increasing x.
                    if path[kernel.edge(sweep_status[i]).vertex.as_index()].x > current_vertex.x {
                        if let Some((prev_helper, VertexType::Merge)) = helper.insert(sweep_status[i-1].as_index(), (*e, VertexType::Merge)) {
                            kernel.split_face(prev_helper, *e);
                        }
                        break;
                    }
                }
            }
            VertexType::Right => {
                // look for the previous edge in the sweep status and replace it with the next edge
                for i in range(0, sweep_status.len()) {
                    if sweep_status[i] == *e {
                        sweep_status[i] = edge.next;
                        break;
                    }
                }
                for i in range(0, sweep_status.len()) {
                    if path[kernel.edge(sweep_status[i]).vertex.as_index()].x > current_vertex.x {
                        if let Some((prev_helper, VertexType::Merge)) = helper.insert(sweep_status[i-1].as_index(), (*e, VertexType::Right)) {
                            kernel.split_face(prev_helper, *e);
                        }
                        break;
                    }
                }
            }
            VertexType::Left => {
                // look for the next edge in the sweep status and replace it with the previous edge
                for i in range(0, sweep_status.len()) {
                    if sweep_status[i] == edge.next {
                        sweep_status[i] = *e;
                        break;
                    }
                }
                if let Some((prev_helper, VertexType::Merge)) = helper.insert(e.as_index(), (*e, VertexType::Left)) {
                    kernel.split_face(prev_helper, *e);
                }
            }
        }
    }
}

pub fn is_y_monotone<T:Copy+Show>(kernel: &ConnectivityKernel, path: &[Vector2D<T>], face: FaceId) -> bool {
    for e in kernel.walk_edges_around_face(face) {
        let edge = kernel.edge(e);
        let current_vertex = path[edge.vertex.as_index()];
        let previous_vertex = path[kernel.edge(edge.prev).vertex.as_index()];
        let next_vertex = path[kernel.edge(edge.next).vertex.as_index()];
        match get_vertex_type(previous_vertex, current_vertex, next_vertex) {
            VertexType::Split | VertexType::Merge => { return false; }
            _ => {}
        }
    }
    return true;
}

// Returns the number of indices added
pub fn y_monotone_triangulation<T: Copy+Show>(
    kernel: &ConnectivityKernel,
    face: FaceId,
    path: &[Vector2D<T>],
    indices: &mut[u16],
) -> usize {
    println!(" ------- y_monotone_triangulation face {} path.len: {}", face.as_index(), path.len());

    let first_edge = kernel.face(face).first_edge;
    println!(" -- first edge of this face is {}", first_edge.as_index());
    let mut circ_up = DirectedEdgeCirculator::new(kernel, first_edge, Direction::Forward);
    let mut circ_down = circ_up.clone();
    circ_down = circ_down.next();

    if path[circ_up.vertex_id().as_index()].y < path[circ_down.vertex_id().as_index()].y {
        circ_up.set_direction(Direction::Backward);
    } else {
        circ_down.set_direction(Direction::Backward);
    }

    // find the bottom-most vertex (with the highest y value)
    let mut big_y = path[circ_down.vertex_id().as_index()].y;
    loop {
        assert_eq!(circ_down.face_id(), face);
        circ_down = circ_down.next();
        let new_y = path[circ_down.vertex_id().as_index()].y;
        if new_y < big_y {
            circ_down = circ_down.prev();
            break;
        }
        big_y = new_y;
    }

    // find the top-most vertex (with the smallest y value)
    let mut small_y = path[circ_up.vertex_id().as_index()].y;
    loop {
        assert_eq!(circ_up.face_id(), face);
        circ_up = circ_up.next();
        let new_y = path[circ_up.vertex_id().as_index()].y;
        if new_y > small_y {
            circ_up = circ_up.prev();
            break;
        }
        small_y = new_y;
    }

    println!(" -- start vertex {} end {}",
        circ_up.vertex_id().as_index(),
        circ_down.vertex_id().as_index()
    );

    // keep track of how many indicies we add.
    let mut index_cursor = 0;

    // vertices already visited, waiting to be connected
    let mut vertex_stack: Vec<DirectedEdgeCirculator> = Vec::new();
    // now that we have the top-most vertex, we will circulate simulataneously
    // from the left and right chains until we reach the bottom-most vertex
    let mut main_chain = circ_up.clone();
    let mut opposite_chain = circ_up.clone();
    main_chain.set_direction(Direction::Forward);
    opposite_chain.set_direction(Direction::Backward);

    main_chain = main_chain.next();
    opposite_chain = opposite_chain.next();

    if path[main_chain.vertex_id().as_index()].y > path[opposite_chain.vertex_id().as_index()].y {
        swap(&mut main_chain, &mut opposite_chain);
    }

    vertex_stack.push(main_chain.prev());
    println!(" -- push first vertex {} to stack", main_chain.prev().vertex_id().as_index());

    let mut previous = main_chain;

    loop {
        println!("\n -- triangulation loop main: {} opposite {} stack.len: {}",
            main_chain.vertex_id().as_index(),
            opposite_chain.vertex_id().as_index(),
            vertex_stack.len()
        );

        if path[main_chain.vertex_id().as_index()].y > path[opposite_chain.vertex_id().as_index()].y {
            println!(" -- swap main -> opposite");
            swap(&mut main_chain, &mut opposite_chain);
        }

        if vertex_stack.len() > 0 && main_chain.direction() != vertex_stack[vertex_stack.len()-1].direction() {
            println!(" -- changing chain");
            for i in 0..vertex_stack.len() - 1 {
                let id_1 = vertex_stack[i].vertex_id();
                let id_2 = vertex_stack[i+1].vertex_id();
                let id_opp = main_chain.vertex_id();
                
                indices[index_cursor  ] = id_opp.as_index() as u16;
                indices[index_cursor+1] = id_1.as_index() as u16;
                indices[index_cursor+2] = id_2.as_index() as u16;
                index_cursor += 3;

                println!(" ==== X - make a triangle {} {} {}",
                    id_opp.as_index(), id_1.as_index(), id_2.as_index()
                );
            }
            
            println!(" -- clear stack");
            vertex_stack.clear();

            println!(" -- push vertirces {} and {} to the stack",
                previous.vertex_id().as_index(), main_chain.vertex_id().as_index()
            );
            vertex_stack.push(previous);
            vertex_stack.push(main_chain);

            println!(" -- advance main chain");
            previous = main_chain;
            main_chain = main_chain.next();

            continue;
        }

        let mut last_popped = vertex_stack.pop();
        if let Some(item) = last_popped {
            println!(" -- popped {} from the stack", item.vertex_id().as_index());
        }

        loop {
            if vertex_stack.len() >= 1 {
                let mut id_1 = vertex_stack[vertex_stack.len()-1].vertex_id();
                let id_2 = last_popped.unwrap().vertex_id(); // TODO we popped it
                let mut id_3 = main_chain.vertex_id();

                if main_chain.direction() == Direction::Backward {
                    swap(&mut id_1, &mut id_3);
                }

                let v1 = path[id_1.as_index()];
                let v2 = path[id_2.as_index()];
                let v3 = path[id_3.as_index()];
                println!(" -- trying triangle {} {} {}", id_1.as_index(), id_2.as_index(), id_3.as_index());
                if directed_angle(v1 - v2, v3 - v2) > PI {
                    // can make a triangle
                    indices[index_cursor  ] = id_1.as_index() as u16;
                    indices[index_cursor+1] = id_2.as_index() as u16;
                    indices[index_cursor+2] = id_3.as_index() as u16;
                    index_cursor += 3;

                    last_popped = vertex_stack.pop();

                    println!(" ===== A - make a triangle {} {} {}",
                        id_1.as_index(), id_2.as_index(), id_3.as_index()
                    );
                } else {
                    break;   
                }
            } else {
                break;
            }
            // continue as long as we manage to make some triangles from the stack
        } // loop 2

        if let Some(item) = last_popped {
            println!(" -- push last popped vertex {} to stack", item.vertex_id().as_index());
            vertex_stack.push(item);
        }
        vertex_stack.push(main_chain);
        println!(" -- C - push vertex {} to stack", main_chain.vertex_id().as_index());

        if main_chain == circ_down && opposite_chain == circ_down {
            println!(" -- end");

            // TODO[nical]
            // fill remaining triangles until the stack is empty

            // both chains have reached the bottom-most vertex, we are done.
            break;
        }

        println!(" -- advance main chain");
        previous = main_chain;
        main_chain = main_chain.next();
    }

    return index_cursor;
}

/// Returns the number of indices added for convenience
pub fn triangulate<T: Copy+Show>(
    path: &[Vector2D<T>],
    indices: &mut[u16]
) -> usize {
    // num triangles = num vertices - 2
    assert!(indices.len() / 3 >= path.len() - 2);

    let mut kernel = ConnectivityKernel::from_loop(path.len() as u16);

    y_monotone_decomposition(&mut kernel, path);

    let indices_len = indices.len();
    let mut index_cursor: usize = 0;
    for f in kernel.face_ids() {
        assert!(is_y_monotone(&kernel, path, f));
        index_cursor += y_monotone_triangulation(
            &kernel, f,
            path, &mut indices[index_cursor..indices_len]
        );
    }

    assert_eq!(index_cursor, (path.len() - 2) * 3);
    return index_cursor;
}

#[test]
fn test_triangulate() {
    {
        println!("\n\n -- path 1");
        // y monotone.
        let path = &[
            world::vec2(1.0, 2.0),
            world::vec2(1.5, 3.0),
            world::vec2(0.0, 4.0),
            world::vec2(-1.0, 1.0),
        ];

        let indices = &mut [0 as u16; 6];

        triangulate(path, indices);
    }

    {
        println!("\n\n -- path 2");
        // y monotone.
        let path = &[
            world::vec2(0.0, 0.0),
            world::vec2(3.0, 0.0),
            world::vec2(2.0, 1.0),
            world::vec2(3.0, 2.0),
            world::vec2(2.0, 3.0),
            world::vec2(0.0, 2.0),
            world::vec2(1.0, 1.0),
        ];

        let indices = &mut [0 as u16; 15];

        triangulate(path, indices);
    }

    {
        println!("\n\n -- path 3");
        // not monotone, needs 1 split.
        let path = &[
            world::vec2(0.0, 0.0),
            world::vec2(1.0, 1.0),// <
            world::vec2(2.0, 0.0),//  |
            world::vec2(2.0, 4.0),//  |
            world::vec2(1.0, 3.0),// <
            world::vec2(0.0, 4.0),
        ];

        let indices = &mut [0 as u16; 12];

        triangulate(path, indices);
    }

    {
        println!("\n\n -- path 4");
        let path = &[
            world::vec2(0.0, 2.0),
            world::vec2(1.0, 2.0),
            world::vec2(0.0, 1.0),
            world::vec2(2.0, 0.0),
            world::vec2(3.0, 1.0),
            world::vec2(4.0, 0.0),
            world::vec2(3.0, 2.0),
            world::vec2(2.0, 1.0),
            world::vec2(3.0, 3.0),
            world::vec2(2.0, 4.0)
        ];

        let indices = &mut [0 as u16; 24];

        triangulate(path, indices);
    }
    panic!("end");
}
