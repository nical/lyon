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
    }

    return if prev.y < next.y { VertexType::Right } else { VertexType::Left };
}

pub fn find(slice: &[EdgeId], item: EdgeId) -> Option<usize> {
    for i in 0 .. slice.len() {
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

pub fn split_face(kernel: &mut ConnectivityKernel, a: EdgeId, b: EdgeId) -> FaceId {
    // TODO[nical] This currently doesn't work if one of the vertex was already
    // split, because it may now belong to another face.

    //loop {
    //    if kernel.edge(a).face == kernel.edge(b).face  {
    //        a = kernel.
    //    }
    //    
    //}
    return kernel.split_face(a,b);
}

pub fn y_monotone_decomposition<T: Copy+Show>(
    kernel: &mut ConnectivityKernel,
    face_id: FaceId,
    path: &[Vector2D<T>],
    new_faces: &mut Vec<FaceId>
) {
    let mut sorted_edges: Vec<EdgeId> = FromIterator::from_iter(kernel.walk_edges_around_face(face_id));

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
                    new_faces.push(kernel.split_face(edge.prev, h));
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
                            new_faces.push(kernel.split_face(*e, helper_edge));
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
                    new_faces.push(kernel.split_face(*e, h));
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
                            new_faces.push(kernel.split_face(prev_helper, *e));
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
                            new_faces.push(kernel.split_face(prev_helper, *e));
                        }
                        break;
                    }
                }
            }
            VertexType::Right => {
                if let Some((h, VertexType::Merge)) = helper.remove(&edge.prev.as_index()) {
                    new_faces.push(kernel.split_face(*e, h));
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
                println!("not y monotone because of vertices {} {} {} edge {} {} {}",
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
    let mut up = DirectedEdgeCirculator::new(kernel, first_edge, Direction::Forward);
    let mut down = up.clone();
    loop {
        down = down.next();
        if path[up.vertex_id().as_index()].y != path[down.vertex_id().as_index()].y {
            break;
        }
    }

    if path[up.vertex_id().as_index()].y < path[down.vertex_id().as_index()].y {
        up.set_direction(Direction::Backward);
    } else {
        down.set_direction(Direction::Backward);
    }

    // find the bottom-most vertex (with the highest y value)
    let mut big_y = path[down.vertex_id().as_index()].y;
    loop {
        assert_eq!(down.face_id(), face);
        println!(" circulating down edge {} vertex {}", down.edge_id().as_index(), down.vertex_id().as_index());
        down = down.next();
        let new_y = path[down.vertex_id().as_index()].y;
        if new_y < big_y {
            down = down.prev();
            break;
        }
        big_y = new_y;
    }

    // find the top-most vertex (with the smallest y value)
    let mut small_y = path[up.vertex_id().as_index()].y;
    loop {
        assert_eq!(up.face_id(), face);
        println!(" circulating up edge {} vertex {}", up.edge_id().as_index(), up.vertex_id().as_index());
        up = up.next();
        let new_y = path[up.vertex_id().as_index()].y;
        if new_y > small_y {
            up = up.prev();
            break;
        }
        small_y = new_y;
    }

    println!(" -- start vertex {} end {} (end edge {})",
        up.vertex_id().as_index(),
        down.vertex_id().as_index(),
        down.edge_id().as_index()
    );

    // keep track of how many indicies we add.
    let mut index_cursor = 0;

    // vertices already visited, waiting to be connected
    let mut vertex_stack: Vec<DirectedEdgeCirculator> = Vec::new();
    // now that we have the top-most vertex, we will circulate simulataneously
    // from the left and right chains until we reach the bottom-most vertex

    // main chain
    let mut m = up.clone();

    // opposite chain
    let mut o = up.clone();
    m.set_direction(Direction::Forward);
    o.set_direction(Direction::Backward);

    m = m.next();
    o = o.next();

    if path[m.vertex_id().as_index()].y > path[o.vertex_id().as_index()].y {
        swap(&mut m, &mut o);
    }

    vertex_stack.push(m.prev());
    println!(" -- push first vertex {} to stack", m.prev().vertex_id().as_index());

    // previous
    let mut p = m;

    loop {
        // walk edges from top to bottom, alternating between the left and 
        // right chains. The chain we are currently iterating over is the
        // main chain (m) and the other one the opposite chain (o).
        // p is the previous iteration, regardless of whcih chain it is on.
        println!("\n ** main vertex: {} opposite vertex {} ",
            m.vertex_id().as_index(),
            o.vertex_id().as_index()
        );

        if path[m.vertex_id().as_index()].y > path[o.vertex_id().as_index()].y || m == down {
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
                if vertex_stack.len() < 1 {
                    break;
                }
                let mut id_1 = vertex_stack[vertex_stack.len()-1].vertex_id();
                let id_2 = last_popped.unwrap().vertex_id();
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
            } // loop 2

            if let Some(item) = last_popped {
                println!(" -- push last popped vertex {} to stack", item.vertex_id().as_index());
                vertex_stack.push(item);
            }
            vertex_stack.push(m);
            println!(" -- C - push vertex {} to stack", m.vertex_id().as_index());

        }

        if m == down {
            println!(" ** main = down");
            if o == down {
                println!(" ** opposite = down");
                break;
            }
        }

        println!(" ** advance");
        p = m;
        m = m.next();
        assert!(path[m.vertex_id().as_index()].y >= path[p.vertex_id().as_index()].y);
    }

    return index_cursor;
}

#[derive(Copy)]
pub struct TriangulationDescriptor<'l, T> {
    vertices: &'l[Vector2D<T>],
    holes: Option<&'l[&'l[Vector2D<T>]]>,
}


/// Returns the number of indices added for convenience
pub fn triangulate<T: Copy+Show>(
    inputs: TriangulationDescriptor<T>,
    indices: &mut[u16]
) -> usize {
    assert!(inputs.holes.is_none(), "TODO[nical] unimplemented");

    let path = inputs.vertices;
    // num triangles = num vertices - 2
    assert!(indices.len() / 3 >= path.len() - 2);

    let first_face = FaceId { handle: 1 };

    let mut kernel = ConnectivityKernel::from_loop(path.len() as u16);
    let mut new_faces: Vec<FaceId> = vec!(first_face);

    y_monotone_decomposition(&mut kernel, first_face, path, &mut new_faces);

    let indices_len = indices.len();
    let mut index_cursor: usize = 0;
    for &f in new_faces.iter() {
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
    let paths : &[&[world::Vec2]] = &[
        &[
            world::vec2(1.0, 2.0),
            world::vec2(1.5, 3.0),
            world::vec2(0.0, 4.0),
        ],
        &[
            world::vec2(1.0, 2.0),
            world::vec2(1.5, 3.0),
            world::vec2(0.0, 4.0),
            world::vec2(-1.0, 1.0),
        ],
        &[
            world::vec2(1.0, 2.0),
            world::vec2(1.5, 3.0),
            world::vec2(0.0, 4.0),
            world::vec2(-1.0, 1.0),
        ],
        &[
            world::vec2(0.0, 0.0),
            world::vec2(3.0, 0.0),
            world::vec2(2.0, 1.0),
            world::vec2(3.0, 2.0),
            world::vec2(2.0, 3.0),
            world::vec2(0.0, 2.0),
            world::vec2(1.0, 1.0),
        ],
        &[
            world::vec2(0.0, 0.0),
            world::vec2(1.0, 1.0),// <
            world::vec2(2.0, 0.0),//  |
            world::vec2(2.0, 4.0),//  |
            world::vec2(1.0, 3.0),// <
            world::vec2(0.0, 4.0),
        ],
        &[
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
        ],
        &[
            world::vec2(0.0, 0.0),
            world::vec2(1.0, 0.0),
            world::vec2(2.0, 0.0),
            world::vec2(3.0, 0.0),
            world::vec2(3.0, 1.0),
            world::vec2(3.0, 2.0),
            world::vec2(3.0, 3.0),
            world::vec2(2.0, 3.0),
            world::vec2(1.0, 3.0),
            world::vec2(0.0, 3.0),
            world::vec2(0.0, 2.0),
            world::vec2(0.0, 1.0),
        ],
    ];

    let indices = &mut [0 as u16; 1024];
    for i in 0 .. paths.len() {
        println!("\n\n -- path {}", i);
        let desc = TriangulationDescriptor {
            vertices: &paths[i][],
            holes: None,
        };
        triangulate(desc, indices);
    }
}

#[test]
fn test_triangulate_holes() {
    let paths : &[(&[world::Vec2], &[&[world::Vec2]])] = &[
        (
            &[
                world::vec2(-10.0, 5.0),
                world::vec2(0.0, -5.0),
                world::vec2(10.0, 5.0),
            ],
            &[&[
                world::vec2(-4.0, 2.0),
                world::vec2(0.0, -2.0),
                world::vec2(4.0, 2.0),
            ]]
        )
    ];

    let indices = &mut [0 as u16; 1024];
    for i in 0 .. paths.len() {
        println!("\n\n -- path {}", i);
        let &(path, holes) = &paths[i];
        let desc = TriangulationDescriptor {
            vertices: &path[],
            holes: Some(holes),
        };
        triangulate(desc, indices);
    }

}