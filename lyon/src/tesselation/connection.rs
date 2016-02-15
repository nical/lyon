
use tesselation::{ VertexId, error };
use tesselation::polygon::*;
use tesselation::vectors::{ Position2D };

use vodk_id::IdSlice;

struct Connection<Poly: AbstractPolygon> {
    from: Poly::PointId,
    to: Poly::PointId,
    processed_face: bool,
    processed_opposite_face: bool,
}

pub struct Connections<Poly: AbstractPolygon> {
    connections: Vec<Connection<Poly>>,
}

impl<Poly: AbstractPolygon> Connections<Poly> {

    pub fn new() -> Connections<Poly> {
        Connections {
            connections: Vec::with_capacity(4), // kinda arbitrary...
        }
    }

    pub fn add_connection(&mut self, from: Poly::PointId, to: Poly::PointId) {
        self.connections.push(Connection{
            from: from, to: to, processed_face: false, processed_opposite_face: false
        });
    }

    pub fn is_empty(&self) -> bool { self.connections.is_empty() }

    pub fn clear_flags(&mut self) {
        for diag in &mut self.connections {
            diag.processed_face = false;
            diag.processed_opposite_face = false;
        }
    }
}

#[derive(Debug)]
pub struct Error;

/// Apply a partition defined by connections to a polygon and provide the result by populating
/// an array of simple polygons.
///
/// This function can't produce complex polygons so the result might come accross as surprising
/// if the input polygon has holes that are not connected with the contour through connections.
/// More generally this is intended for use to apply the y-monotone decomposition of a polygon,
/// which we know to produce a valid input for teh partition.
pub fn apply_connections<Poly: AbstractPolygon, V: Position2D>(
    polygon: &Poly,
    vertices: IdSlice<VertexId, V>,
    connections: &mut Connections<Poly>,
    output: &mut Vec<Polygon>
) -> Result<(), Error> {
    let mut info = if let Some(slice) = polygon.as_slice() { slice.info().clone() }
                   else { PolygonInfo::default() };
    if info.is_convex == Some(false) { info.is_convex = None; }
    if info.is_y_monotone == Some(false) { info.is_y_monotone = None; }
    if info.has_beziers == Some(true) { info.is_y_monotone = None; }

    //println!(" ------ polygon partition, {} connections", connections.connections.len());
    connections.clear_flags();
    for i in 0..connections.connections.len() {
        let from = connections.connections[i].from;
        let to = connections.connections[i].to;
        //println!("     -- connection, {:?} -> {:?}", from, to);
        if !connections.connections[i].processed_face {
            let mut p = try!{ gen_polygon(polygon, vertices, connections, from, to) };
            p.info = info.clone();
            output.push(p);
        }
        if !connections.connections[i].processed_opposite_face {
            let mut p = try!{ gen_polygon(polygon, vertices, connections, to, from) };
            p.info = info.clone();
            output.push(p);
        }
    }
    return Ok(());
}

fn gen_polygon<Poly: AbstractPolygon, V: Position2D>(
    polygon: &Poly,
    vertices: IdSlice<VertexId, V>,
    connections: &mut Connections<Poly>,
    first_point: Poly::PointId,
    second_point: Poly::PointId,
) -> Result<Polygon, Error> {
    //println!(" ------------ gen polygon");
    let mut new_poly = Polygon::new();
    let mut prev = first_point;
    let mut it = second_point;
    let mut loop_counter = 0;
    loop {
        //println!("\n\n ------ point {:?}", it);
        new_poly.push_vertex(polygon.vertex(it));

        // Find our next point which is either the next point of along the polygon or a point
        // along one of the connections.
        let origin = vertices[polygon.vertex(prev)].position();
        let center = vertices[polygon.vertex(it)].position();
        let poly_next = polygon.next(it);

        // selected is the index of the connection that we will follow, or None if we are
        // moving along the input polygon without touching a connection
        let mut selected = None;
        // find the best connection (if any) by keeping track of the lowest angle
        let center_to_origin = origin - center;
        let mut angle = center_to_origin.directed_angle(
            vertices[polygon.vertex(poly_next)].position() - center
        );
        //println!("\n -- next {:?} start with angle {}", poly_next, angle);

        for i in 0..connections.connections.len() {
            let diag = &connections.connections[i];
            let diag_next = if diag.from == it { diag.to }
                            else if diag.to == it { diag.from }
                            else { continue; };

            if diag_next == prev {
                continue;
            }

            let diag_angle = center_to_origin.directed_angle(
                vertices[polygon.vertex(diag_next)].position() - center
            );

            //println!(" -- connection {:?} angle {}", diag_next, diag_angle);

            if diag_angle > angle {
                selected = Some(i);
                angle = diag_angle;
            }
        }

        // we are about to update it
        prev = it;

        if let Some(idx) = selected {
            // Going along the connection at index i

            // we need to update the corresponding processed_face flags so that apply_connections
            // doesn't go over this polygon again.
            let diag = &mut connections.connections[idx];
            if diag.from == it {
                diag.processed_face = true;
                it = diag.to;
            } else {
                diag.processed_opposite_face = true;
                it = diag.from;
            }
        } else {
            // Not going along a connection
            it = poly_next;
        }

        if it == second_point && prev == first_point {
            // back to where we began, the work is done.
            break;
        }
        //println!(" -- {:?} : {:?} : {:?}", it, polygon.vertex(it), vertices[polygon.vertex(it)].position());
        loop_counter += 1;
        if loop_counter > polygon.num_vertices() * 2 {
            return error(Error);
        }
    }

    return Ok(new_poly);
}

#[cfg(test)]
use tesselation::{ vertex_id, vertex_id_range };

#[cfg(test)]
use vodk_math::{ Vec2, vec2 };

#[test]
fn test_gen_polygon_no_connection() {
    let positions: &[Vec2] = &[
        vec2(0.0, 0.0),
        vec2(1.0,-1.0),
        vec2(2.0,-1.0),
        vec2(3.0, 0.0),
        vec2(2.0, 1.0),
        vec2(1.0, 1.0),
    ];

    let poly = Polygon::from_vertices(vertex_id_range(0, 6));

    let mut connections = Connections::new();

    let vertices = IdSlice::new(positions);

    let new_poly = gen_polygon(&poly, vertices, &mut connections, point_id(0), point_id(1)).unwrap();
    assert_eq!(&new_poly.vertices[..], &[
        vertex_id(1),
        vertex_id(2),
        vertex_id(3),
        vertex_id(4),
        vertex_id(5),
        vertex_id(0),
    ]);
}

#[test]
fn test_gen_polygon_simple() {
    let positions: &[Vec2] = &[
        vec2(0.0, 0.0),
        vec2(1.0,-1.0),
        vec2(2.0,-1.0),
        vec2(3.0, 0.0),
        vec2(2.0, 1.0),
        vec2(1.0, 1.0),
    ];

    let poly = Polygon::from_vertices(vertex_id_range(0, 6));

    let mut connections = Connections::new();
    connections.add_connection(point_id(2), point_id(4));

    let vertices = IdSlice::new(positions);

    let new_poly = gen_polygon(&poly, vertices, &mut connections, point_id(0), point_id(1)).unwrap();
    assert_eq!(&new_poly.vertices[..], &[
        vertex_id(1),
        vertex_id(2),
        vertex_id(4),
        vertex_id(5),
        vertex_id(0),
    ]);
}

#[test]
fn test_gen_polygon_two_connections() {
    let positions: &[Vec2] = &[
        vec2(0.0, 0.0),
        vec2(1.0,-1.0),
        vec2(2.0,-1.0),
        vec2(3.0, 0.0),
        vec2(2.0, 1.0),
        vec2(1.0, 1.0),
    ];

    let poly = Polygon::from_vertices(vertex_id_range(0, 6));

    let mut connections = Connections::new();
    connections.add_connection(point_id(2), point_id(5));
    connections.add_connection(point_id(2), point_id(4));

    let vertices = IdSlice::new(positions);

    let new_poly = gen_polygon(&poly, vertices, &mut connections, point_id(0), point_id(1)).unwrap();
    assert_eq!(&new_poly.vertices[..], &[
        vertex_id(1),
        vertex_id(2),
        vertex_id(5),
        vertex_id(0),
    ]);
}

#[test]
fn test_gen_polygon_only_connections() {
    // The shape looks like this:
    //
    //  0   1   2
    //   +--+--+
    //   | / \ |
    //   |/   \|
    // 7 +     + 3
    //   |\   /|
    //   | \ / |
    //   +--+--+
    //  6   5   4
    //
    // And we want to check gen_polygon behaves properly for the losange inside, composed
    // of connections only.

    let positions: &[Vec2] = &[
        vec2(0.0, 0.0),
        vec2(0.0,-1.0),
        vec2(0.0,-2.0),
        vec2(1.0,-2.0),
        vec2(2.0,-2.0),
        vec2(2.0,-1.0),
        vec2(2.0, 0.0),
        vec2(1.0, 0.0),
    ];

    let poly = Polygon::from_vertices(vertex_id_range(0, 8));

    let mut connections = Connections::new();
    connections.add_connection(point_id(1), point_id(3));
    connections.add_connection(point_id(7), point_id(1));
    connections.add_connection(point_id(7), point_id(5));
    connections.add_connection(point_id(3), point_id(5));

    let vertices = IdSlice::new(positions);

    let new_poly = gen_polygon(&poly, vertices, &mut connections, point_id(1), point_id(3)).unwrap();
    assert_eq!(&new_poly.vertices[..], &[
        vertex_id(3),
        vertex_id(5),
        vertex_id(7),
        vertex_id(1),
    ]);
}

#[test]
fn test_gen_polygon_with_holes() {
    fn point(poly: PolygonId, idx: u16) -> ComplexPointId {
        ComplexPointId { point: point_id(idx), polygon_id: poly }
    }

    fn a(idx: u16) -> ComplexPointId { point(polygon_id(0), idx) }
    fn b(idx: u16) -> ComplexPointId { point(polygon_id(1), idx) }
    fn c(idx: u16) -> ComplexPointId { point(polygon_id(2), idx) }
    fn v(idx: u16) -> VertexId { vertex_id(idx) }

    // The shape looks like this:
    //  a0       a1   a2
    //   +-------+---+
    //   |       |   |
    //   | b0+---+b3 |
    //   |   |   |   |
    //   | b1+---+b2 |
    //   |       |   |
    //   | c0+---+c3 |
    //   |   |   |   |
    //   | c1+---+c2 |
    //   |           |
    //   +-----------+
    //  a4           a3

    let mut connections = Connections::new();
    connections.add_connection(a(1), b(3));
    connections.add_connection(c(3), b(2));

    let positions: &[Vec2] = &[
        // a
        vec2(0.0, 0.0),// v(0) :a(0)
        vec2(2.0, 0.0),// v(1) :a(1)
        vec2(3.0, 0.0),// v(2) :a(2)
        vec2(3.0, 5.0),// v(3) :a(3)
        vec2(0.0, 5.0),// v(4) :a(4)
        // b
        vec2(1.0, 1.0),// v(5) :b(0)
        vec2(1.0, 2.0),// v(6) :b(1)
        vec2(2.0, 2.0),// v(7) :b(2)
        vec2(2.0, 1.0),// v(8) :b(3)
        // b
        vec2(1.0, 3.0),// v(9) :c(0)
        vec2(1.0, 4.0),// v(10):c(1)
        vec2(2.0, 4.0),// v(11):c(2)
        vec2(2.0, 3.0),// v(12):c(3)
    ];
    let poly = ComplexPolygon {
        sub_polygons: vec![
            Polygon::from_vertices(vertex_id_range(0, 5)),
            Polygon::from_vertices(vertex_id_range(5, 9)),
            Polygon::from_vertices(vertex_id_range(9, 13)),
        ]
    };

    let vertices = IdSlice::new(positions);

    let new_poly = gen_polygon(&poly, vertices, &mut connections, a(0), a(1)).unwrap();
    assert_eq!(&new_poly.vertices[..], &[
        v(1), v(8), v(5), v(6), v(7), v(12), v(9), v(10), v(11),
        v(12), v(7), v(8), v(1), v(2), v(3), v(4), v(0)
    ]);
    assert!(connections.connections[0].processed_face);
    assert!(connections.connections[0].processed_opposite_face);
    assert!(connections.connections[1].processed_face);
    assert!(connections.connections[1].processed_opposite_face);
}

#[test]
fn test_apply_connections_connections() {
    // The shape looks like this:
    //
    //  0   1   2
    //   +--+--+
    //   | / \ |
    //   |/   \|
    // 7 +     + 3
    //   |\   /|
    //   | \ / |
    //   +--+--+
    //  6   5   4
    //
    // And we want to check gen_polygon behaves properly for the losange inside, composed
    // of connections only.

    let positions: &[Vec2] = &[
        vec2(0.0, 0.0),
        vec2(0.0,-1.0),
        vec2(0.0,-2.0),
        vec2(1.0,-2.0),
        vec2(2.0,-2.0),
        vec2(2.0,-1.0),
        vec2(2.0, 0.0),
        vec2(1.0, 0.0),
    ];

    let poly = Polygon::from_vertices(vertex_id_range(0, 8));

    let mut connections = Connections::new();
    connections.add_connection(point_id(1), point_id(3));
    connections.add_connection(point_id(7), point_id(1));
    connections.add_connection(point_id(7), point_id(5));
    connections.add_connection(point_id(3), point_id(5));

    let vertices = IdSlice::new(positions);

    let mut partition = Vec::new();

    apply_connections(&poly, vertices, &mut connections, &mut partition).unwrap();
    assert_eq!(partition.len(), 5);
}

#[test]
fn test_apply_connections_no_connections() {
    let positions: &[Vec2] = &[
        vec2(0.0, 0.0),
        vec2(0.0,-1.0),
        vec2(0.0,-2.0),
        vec2(1.0,-2.0),
        vec2(2.0,-2.0),
        vec2(2.0,-1.0),
        vec2(2.0, 0.0),
        vec2(1.0, 0.0),
    ];

    let poly = Polygon::from_vertices(vertex_id_range(0, 8));

    let mut no_connections = Connections::new();

    let vertices = IdSlice::new(positions);

    let mut partition = Vec::new();

    apply_connections(&poly, vertices, &mut no_connections, &mut partition).unwrap();
    assert_eq!(partition.len(), 0);
}
