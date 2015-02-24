// Implementation inspired by Computational Geometry, Algorithms And Applications 3rd edition.
//
// Note that a lot of the code/comments/names in this module assume a coordinate
// system where y pointing downwards

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
/// (equivalent to counter-clockwise if y points upward)
/// ex: directed_angle([0,1], [1,0]) = 3/2 Pi rad = 270 deg
///     x       __
///   0-->     /  \
///  y|       |  x--> v2
///   v        \ |v1   
///              v
pub fn directed_angle<T>(v1: Vector2D<T>, v2: Vector2D<T>) -> f32 {
    let a = (v2.y).atan2(v2.x) - (v1.y).atan2(v1.x);
    return if a < 0.0 { a + 2.0 * PI } else { a };
}

pub fn deg(rad: f32) -> f32 {
    return rad / PI * 180.0;
}

fn get_vertex_type<T: Copy>(prev: Vector2D<T>, current: Vector2D<T>, next: Vector2D<T>) -> VertexType {
    // assuming clockwise path winding order
    let interrior_angle = directed_angle(prev - current, next - current);

    if current.y > prev.y && current.y >= next.y {
        if interrior_angle <= PI {
            return VertexType::Merge;
        } else {
            return VertexType::End;
        }
    }

    if current.y < prev.y && current.y <= next.y {
        if interrior_angle <= PI {
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
        path[kernel.edge(*a).vertex.as_index()].y.partial_cmp(&path[kernel.edge(*b).vertex.as_index()].y).unwrap().reverse()
    });    
}

pub fn sweep_status_push<T:Copy>(
    kernel: &ConnectivityKernel,
    path: &[Vector2D<T>],
    sweep: &mut Vec<EdgeId>,
    e: &EdgeId
) {
    println!(" -- insert {} in sweep status", e.as_index());
    sweep.push(*e);
    sort_x(&mut sweep[], kernel, path);
}

pub fn split_face(kernel: &mut ConnectivityKernel, a: EdgeId, b: EdgeId) {
//    loop {
//        if kernel.edge(a).face == kernel.edge(b).face  {
//            a = kernel.
//        }
//        
//    }
    kernel.split_face(a,b);
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
        println!(" vertex {} (edge {}) type {:?}", edge.vertex.as_index(), e.as_index(), vertex_type);

        match vertex_type {
            VertexType::Start => {
                sweep_status_push(kernel, path, &mut sweep_status, e);
                println!("set {} as helper of {}", e.as_index(), e.as_index());
                helper.insert(e.as_index(), (*e, VertexType::Start));
            }
            VertexType::End => {
                if let Some(&(h, VertexType::Merge)) = helper.get(&edge.prev.as_index()) {
                    kernel.split_face(edge.prev, h);
                } 
                sweep_status.retain(|item|{ *item != edge.prev });
            }
            VertexType::Split => {
                for i in 0 .. sweep_status.len() {
                    println!(" --- A look for vertex right of e x={} vertex={}",
                        path[kernel.edge(sweep_status[i]).vertex.as_index()].x,
                        kernel.edge(sweep_status[i]).vertex.as_index()
                    );
                    if path[kernel.edge(sweep_status[i]).vertex.as_index()].x > current_vertex.x {
                        if let Some(&(helper_edge,_)) = helper.get(&sweep_status[i].as_index()) {
                            kernel.split_face(*e, helper_edge);
                        }
                        helper.insert(sweep_status[i].as_index(), (*e, VertexType::Split));
                        println!("set {} as helper of {}", e.as_index(), sweep_status[i].as_index());
                        break;
                    }
                }
                sweep_status_push(kernel, path, &mut sweep_status, e);
                helper.insert(e.as_index(), (*e, VertexType::Split));
                println!("set {} as helper of {}", e.as_index(), e.as_index());
            }
            VertexType::Merge => {
                if let Some((h, VertexType::Merge)) = helper.remove(&edge.prev.as_index()) {
                    kernel.split_face(*e, h);
                }
                for i in 0 .. sweep_status.len() {
                    println!(" --- B look for vertex right of e x={} vertex={}",
                        path[kernel.edge(sweep_status[i]).vertex.as_index()].x,
                        kernel.edge(sweep_status[i]).vertex.as_index()
                    );
                    if path[kernel.edge(sweep_status[i]).vertex.as_index()].x > current_vertex.x {
                        println!(" --- D set {} as helper of {}",
                            edge.vertex.as_index(),
                            sweep_status[i].as_index()
                        );
                        println!("set {} as helper of {}", sweep_status[i].as_index(), e.as_index());
                        if let Some((prev_helper, VertexType::Merge)) = helper.insert(
                            sweep_status[i].as_index(),
                            (*e, VertexType::Merge)
                        ) {
                            kernel.split_face(prev_helper, *e);
                        }
                        break;
                    }
                }
            }
            VertexType::Left => {
                for i in 0 .. sweep_status.len() {
                    println!(" --- X look for vertex right of e x={} vertex={}", path[kernel.edge(sweep_status[i]).vertex.as_index()].x, kernel.edge(sweep_status[i]).vertex.as_index());
                    if path[kernel.edge(sweep_status[i]).vertex.as_index()].x > current_vertex.x {
                        println!(" --- meh {} x={}", kernel.edge(sweep_status[i]).vertex.as_index(), path[kernel.edge(sweep_status[i]).vertex.as_index()].x);
                        println!("set {} as helper of {}", e.as_index(), sweep_status[i].as_index());
                        if let Some((prev_helper, VertexType::Merge)) = helper.insert(sweep_status[i].as_index(), (*e, VertexType::Right)) {
                            kernel.split_face(prev_helper, *e);
                        }
                        break;
                    }
                }
            }
            VertexType::Right => {
                if let Some((h, VertexType::Merge)) = helper.remove(&edge.prev.as_index()) {
                    kernel.split_face(*e, h);
                }
                sweep_status.retain(|item|{ *item != edge.prev });
                sweep_status_push(kernel, path, &mut sweep_status, e);
                println!("set {} as helper of {}", e.as_index(), e.as_index());
                helper.insert(e.as_index(), (*e, VertexType::Left));
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
            VertexType::Split | VertexType::Merge => {
                println!("mot y monotone because of vertices {} {} {} edge {} {} {}",
                    kernel.edge(edge.prev).vertex.as_index(), edge.vertex.as_index(), kernel.edge(edge.next).vertex.as_index(), 
                    edge.prev.as_index(), e.as_index(), edge.next.as_index());
                return false;
            }
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
    loop {
        circ_down = circ_down.next();
        if path[circ_up.vertex_id().as_index()].y != path[circ_down.vertex_id().as_index()].y {
            break;
        }
    }

    if path[circ_up.vertex_id().as_index()].y < path[circ_down.vertex_id().as_index()].y {
        circ_up.set_direction(Direction::Backward);
    } else {
        circ_down.set_direction(Direction::Backward);
    }

    // find the bottom-most vertex (with the highest y value)
    let mut big_y = path[circ_down.vertex_id().as_index()].y;
    loop {
        assert_eq!(circ_down.face_id(), face);
        println!(" circulating down edge {} vertex {}", circ_down.edge_id().as_index(), circ_down.vertex_id().as_index());
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
        println!(" circulating up edge {} vertex {}", circ_up.edge_id().as_index(), circ_up.vertex_id().as_index());
        circ_up = circ_up.next();
        let new_y = path[circ_up.vertex_id().as_index()].y;
        if new_y > small_y {
            circ_up = circ_up.prev();
            break;
        }
        small_y = new_y;
    }

    println!(" -- start vertex {} end {} (end edge {})",
        circ_up.vertex_id().as_index(),
        circ_down.vertex_id().as_index(),
        circ_down.edge_id().as_index()
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

    let mut p = main_chain;
    let mut m = main_chain;
    let mut o = opposite_chain;
    let mut i = 0;
    loop {
        println!("\n ** main vertex: {} opposite vertex {} ",
            m.vertex_id().as_index(),
            o.vertex_id().as_index()
        );

        if path[m.vertex_id().as_index()].y > path[o.vertex_id().as_index()].y || m == circ_down {
            println!(" ** swap");
            swap(&mut m, &mut o);
        }

        println!(" ** do stuff with vertex {}", m.vertex_id().as_index());

        if vertex_stack.len() > 0 && m.direction() != vertex_stack[vertex_stack.len()-1].direction() {
            println!(" -- changing chain");
            for i in 0..vertex_stack.len() - 1 {
                let id_1 = vertex_stack[i].vertex_id();
                let id_2 = vertex_stack[i+1].vertex_id();
                let id_opp = m.vertex_id();

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
                p.vertex_id().as_index(), m.vertex_id().as_index()
            );
            vertex_stack.push(p);
            vertex_stack.push(m);

        } else {

            let mut last_popped = vertex_stack.pop();
            if let Some(item) = last_popped {
                println!(" -- popped {} from the stack", item.vertex_id().as_index());
            }

            loop {
                if vertex_stack.len() >= 1 {
                    let mut id_1 = vertex_stack[vertex_stack.len()-1].vertex_id();
                    let id_2 = last_popped.unwrap().vertex_id(); // TODO we popped it
                    let mut id_3 = m.vertex_id();

                    if m.direction() == Direction::Backward {
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
            vertex_stack.push(m);
            println!(" -- C - push vertex {} to stack", m.vertex_id().as_index());

        }



        if m == circ_down {
            println!(" ** main = circ_down");
            if o == circ_down {
                println!(" ** opposite = circ_down");
                break;
            }
        }

        println!(" ** advance");
        p = m;
        m = m.next();
        assert!(path[m.vertex_id().as_index()].y >= path[p.vertex_id().as_index()].y);
        i += 1;
        if i > 30 { panic!("infinite loop"); }
    }
/*
    loop {
        println!("\n -- triangulation loop main: {} opposite {} stack.len: {}",
            main_chain.vertex_id().as_index(),
            opposite_chain.vertex_id().as_index(),
            vertex_stack.len()
        );

        if path[main_chain.vertex_id().as_index()].y >= path[opposite_chain.vertex_id().as_index()].y {
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

//            if main_chain == circ_down && opposite_chain == circ_down {
//                println!(" -- end");
//                // both chains have reached the bottom-most vertex, we are done.
//                break;
//            }

            println!(" -- push vertirces {} and {} to the stack",
                previous.vertex_id().as_index(), main_chain.vertex_id().as_index()
            );
            vertex_stack.push(previous);
            vertex_stack.push(main_chain);

        } else {

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

        }

        if main_chain == circ_down {
            if opposite_chain == circ_down {
                // TODO[nical]
                // fill remaining triangles until the stack is empty

                // both chains have reached the bottom-most vertex, we are done.
                println!(" -- end");
                break;                
            }
            println!(" -- yayaya");
            println!(" -- swap main -> opposite");
            swap(&mut main_chain, &mut opposite_chain);
        }

        previous = main_chain;
        println!(" -- advance main chain");
        main_chain = main_chain.next();
    }
*/
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
            world::vec2(3.0, 1.0),// 4
            world::vec2(4.0, 0.0),
            world::vec2(3.0, 2.0),
            world::vec2(2.0, 1.0),// 7
            world::vec2(3.0, 3.0),
            world::vec2(2.0, 4.0)
        ];

        let indices = &mut [0 as u16; 24];

        triangulate(path, indices);
    }
    panic!("end");
}
