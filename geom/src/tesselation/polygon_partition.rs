use tesselation::polygon::*;

use half_edge::vectors::{ Position2D, Vec2, vec2_sub, directed_angle };
use half_edge::kernel::{ VertexId };

use vodk_id::id_vector::IdSlice;

struct Diagonal<Poly: AbstractPolygon> {
    from: Poly::PointId,
    to: Poly::PointId,
    processed_face: bool,
    processed_opposite_face: bool,
}

pub struct Diagonals<Poly: AbstractPolygon> {
    diagonals: Vec<Diagonal<Poly>>,
}

impl<Poly: AbstractPolygon> Diagonals<Poly> {

    pub fn new() -> Diagonals<Poly> {
        Diagonals {
            diagonals: Vec::with_capacity(4), // kinda arbitrary...
        }
    }

    pub fn add_diagonal(&mut self, from: Poly::PointId, to: Poly::PointId) {
        self.diagonals.push(Diagonal{
            from: from, to: to, processed_face: false, processed_opposite_face: false
        });
    }

    pub fn is_empty(&self) -> bool { self.diagonals.is_empty() }

    pub fn clear_flags(&mut self) {
        for diag in &mut self.diagonals {
            diag.processed_face = false;
            diag.processed_opposite_face = false;
        }
    }
}

/// Apply a partition defined by diagonals to a polygon and provide the result by populating
/// an array of simple polygons.
///
/// This function can't produce complex polygons so the result might come accross as surprising
/// if the input polygon has holes that are not connected with the contour through diagonals.
/// More generally this is intended for use to apply the y-monotone decomposition of a polygon,
/// which we know to produce a valid input for teh partition.
pub fn partition_polygon<Poly: AbstractPolygon, V: Position2D>(
    polygon: &Poly,
    vertices: IdSlice<VertexId, V>,
    diagonals: &mut Diagonals<Poly>,
    output: &mut Vec<Polygon>
) {
    println!(" ------ polygon partition, {} diagonals", diagonals.diagonals.len());
    diagonals.clear_flags();
    for i in 0..diagonals.diagonals.len() {
        let from = diagonals.diagonals[i].from;
        let to = diagonals.diagonals[i].to;
        println!("     -- diagonal, {:?} -> {:?}", from, to);
        if !diagonals.diagonals[i].processed_face {
            output.push(gen_polygon(polygon, vertices, diagonals, from, to));
        }
        if !diagonals.diagonals[i].processed_opposite_face {
            output.push(gen_polygon(polygon, vertices, diagonals, to, from));
        }
    }
}

fn gen_polygon<Poly: AbstractPolygon, V: Position2D>(
    polygon: &Poly,
    vertices: IdSlice<VertexId, V>,
    diagonals: &mut Diagonals<Poly>,
    first_point: Poly::PointId,
    second_point: Poly::PointId,
) -> Polygon {
    println!(" ------------ gen polygon");
    let mut new_poly = Polygon::new();
    let mut prev = first_point;
    let mut it = second_point;
    let mut counter = 0;
    loop {
        //println!("\n\n ------ point {:?}", it);
        new_poly.push_vertex(polygon.vertex(it));

        // Find our next point which is either the next point of along the polygon or a point
        // along one of the diagonals.
        let origin = vertices[polygon.vertex(prev)].position();
        let center = vertices[polygon.vertex(it)].position();
        let poly_next = polygon.next(it);

        // selected is the index of the diagonal that we will follow, or None if we are
        // moving along the input polygon without touching a diagonal
        let mut selected = None;
        // find the best diagonal (if any) by keeping track of the lowest angle
        let center_to_origin = vec2_sub(origin, center);
        let mut angle = directed_angle(
            center_to_origin,
            vec2_sub(vertices[polygon.vertex(poly_next)].position(), center)
        );
        //println!("\n -- next {:?} start with angle {}", poly_next, angle);

        for i in 0..diagonals.diagonals.len() {
            let diag = &diagonals.diagonals[i];
            let diag_next = if diag.from == it { diag.to }
                            else if diag.to == it { diag.from }
                            else { continue; };

            if diag_next == prev {
                continue;
            }

            let diag_angle = directed_angle(
                center_to_origin,
                vec2_sub(vertices[polygon.vertex(diag_next)].position(), center)
            );

            //println!(" -- diagonal {:?} angle {}", diag_next, diag_angle);

            if diag_angle > angle {
                selected = Some(i);
                angle = diag_angle;
            }
        }

        // we are about to update it
        prev = it;

        if let Some(idx) = selected {
            // Going along the diagonal at index i

            // we need to update the corresponding processed_face flags so that apply_diagonals
            // doesn't go over this polygon again.
            let diag = &mut diagonals.diagonals[idx];
            if diag.from == it {
                diag.processed_face = true;
                it = diag.to;
            } else {
                diag.processed_opposite_face = true;
                it = diag.from;
            }
        } else {
            // Not going along a diagonal
            it = poly_next;
        }

        if it == second_point && prev == first_point {
            // back to where we began, the work is done.
            break;
        }
        println!(" -- {:?}", it);
        counter += 1;
        if counter > polygon.num_vertices() * 2 {
            panic!("infinite loop ?");
        }
    }

    return new_poly;
}

#[test]
fn test_gen_polygon_no_diagonal() {
    use half_edge::kernel::vertex_id;

    let positions: &[Vec2] = &[
        [0.0, 0.0],
        [1.0,-1.0],
        [2.0,-1.0],
        [3.0, 0.0],
        [2.0, 1.0],
        [1.0, 1.0],
    ];

    let poly = Polygon::from_vertices(vertex_id_range(0, 6));

    let mut diagonals = Diagonals::new();

    let vertices = IdSlice::new(positions);

    let new_poly = gen_polygon(&poly, vertices, &mut diagonals, point_id(0), point_id(1));
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
    use half_edge::kernel::vertex_id;

    let positions: &[Vec2] = &[
        [0.0, 0.0],
        [1.0,-1.0],
        [2.0,-1.0],
        [3.0, 0.0],
        [2.0, 1.0],
        [1.0, 1.0],
    ];

    let poly = Polygon::from_vertices(vertex_id_range(0, 6));

    let mut diagonals = Diagonals::new();
    diagonals.add_diagonal(point_id(2), point_id(4));

    let vertices = IdSlice::new(positions);

    let new_poly = gen_polygon(&poly, vertices, &mut diagonals, point_id(0), point_id(1));
    assert_eq!(&new_poly.vertices[..], &[
        vertex_id(1),
        vertex_id(2),
        vertex_id(4),
        vertex_id(5),
        vertex_id(0),
    ]);
}

#[test]
fn test_gen_polygon_two_diagonals() {
    use half_edge::kernel::vertex_id;

    let positions: &[Vec2] = &[
        [0.0, 0.0],
        [1.0,-1.0],
        [2.0,-1.0],
        [3.0, 0.0],
        [2.0, 1.0],
        [1.0, 1.0],
    ];

    let poly = Polygon::from_vertices(vertex_id_range(0, 6));

    let mut diagonals = Diagonals::new();
    diagonals.add_diagonal(point_id(2), point_id(5));
    diagonals.add_diagonal(point_id(2), point_id(4));

    let vertices = IdSlice::new(positions);

    let new_poly = gen_polygon(&poly, vertices, &mut diagonals, point_id(0), point_id(1));
    assert_eq!(&new_poly.vertices[..], &[
        vertex_id(1),
        vertex_id(2),
        vertex_id(5),
        vertex_id(0),
    ]);
}

#[test]
fn test_gen_polygon_only_diagonals() {
    use half_edge::kernel::vertex_id;

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
    // of diagonals only.

    let positions: &[Vec2] = &[
        [0.0, 0.0],
        [0.0,-1.0],
        [0.0,-2.0],
        [1.0,-2.0],
        [2.0,-2.0],
        [2.0,-1.0],
        [2.0, 0.0],
        [1.0, 0.0],
    ];

    let poly = Polygon::from_vertices(vertex_id_range(0, 8));

    let mut diagonals = Diagonals::new();
    diagonals.add_diagonal(point_id(1), point_id(3));
    diagonals.add_diagonal(point_id(7), point_id(1));
    diagonals.add_diagonal(point_id(7), point_id(5));
    diagonals.add_diagonal(point_id(3), point_id(5));

    let vertices = IdSlice::new(positions);

    let new_poly = gen_polygon(&poly, vertices, &mut diagonals, point_id(1), point_id(3));
    assert_eq!(&new_poly.vertices[..], &[
        vertex_id(3),
        vertex_id(5),
        vertex_id(7),
        vertex_id(1),
    ]);
}

#[test]
fn test_gen_polygon_with_holes() {
    use half_edge::kernel::vertex_id;

    fn point(poly: PolygonId, idx: u16) -> ComplexPointId {
        ComplexPointId { point: point_id(idx), polygon_id: poly }
    }

    fn a(idx: u16) -> ComplexPointId { point( polygon_id(0), idx) }
    fn b(idx: u16) -> ComplexPointId { point( polygon_id(1), idx) }
    fn c(idx: u16) -> ComplexPointId { point( polygon_id(2), idx) }
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

    let mut diagonals = Diagonals::new();
    diagonals.add_diagonal(a(1), b(3));
    diagonals.add_diagonal(c(3), b(2));

    let positions: &[Vec2] = &[
        // a
        [0.0, 0.0],// v(0) :a(0)
        [2.0, 0.0],// v(1) :a(1)
        [3.0, 0.0],// v(2) :a(2)
        [3.0, 5.0],// v(3) :a(3)
        [0.0, 5.0],// v(4) :a(4)
        // b
        [1.0, 1.0],// v(5) :b(0)
        [1.0, 2.0],// v(6) :b(1)
        [2.0, 2.0],// v(7) :b(2)
        [2.0, 1.0],// v(8) :b(3)
        // b
        [1.0, 3.0],// v(9) :c(0)
        [1.0, 4.0],// v(10):c(1)
        [2.0, 4.0],// v(11):c(2)
        [2.0, 3.0],// v(12):c(3)
    ];
    let poly = ComplexPolygon {
        main: Polygon::from_vertices(vertex_id_range(0, 5)),
        holes: vec![
            Polygon::from_vertices(vertex_id_range(5, 9)),
            Polygon::from_vertices(vertex_id_range(9, 13)),
        ]
    };

    let vertices = IdSlice::new(positions);

    let new_poly = gen_polygon(&poly, vertices, &mut diagonals, a(0), a(1));
    assert_eq!(&new_poly.vertices[..], &[
        v(1), v(8), v(5), v(6), v(7), v(12), v(9), v(10), v(11),
        v(12), v(7), v(8), v(1), v(2), v(3), v(4), v(0)
    ]);
    assert!(diagonals.diagonals[0].processed_face);
    assert!(diagonals.diagonals[0].processed_opposite_face);
    assert!(diagonals.diagonals[1].processed_face);
    assert!(diagonals.diagonals[1].processed_opposite_face);
}

#[test]
fn test_partition_polygon_diagonals() {
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
    // of diagonals only.

    let positions: &[Vec2] = &[
        [0.0, 0.0],
        [0.0,-1.0],
        [0.0,-2.0],
        [1.0,-2.0],
        [2.0,-2.0],
        [2.0,-1.0],
        [2.0, 0.0],
        [1.0, 0.0],
    ];

    let poly = Polygon::from_vertices(vertex_id_range(0, 8));

    let mut diagonals = Diagonals::new();
    diagonals.add_diagonal(point_id(1), point_id(3));
    diagonals.add_diagonal(point_id(7), point_id(1));
    diagonals.add_diagonal(point_id(7), point_id(5));
    diagonals.add_diagonal(point_id(3), point_id(5));

    let vertices = IdSlice::new(positions);

    let mut partition = Vec::new();

    partition_polygon(&poly, vertices, &mut diagonals, &mut partition);
    assert_eq!(partition.len(), 5);
}

#[test]
fn test_partition_polygon_no_diagonals() {
    let positions: &[Vec2] = &[
        [0.0, 0.0],
        [0.0,-1.0],
        [0.0,-2.0],
        [1.0,-2.0],
        [2.0,-2.0],
        [2.0,-1.0],
        [2.0, 0.0],
        [1.0, 0.0],
    ];

    let poly = Polygon::from_vertices(vertex_id_range(0, 8));

    let mut no_diagonals = Diagonals::new();

    let vertices = IdSlice::new(positions);

    let mut partition = Vec::new();

    partition_polygon(&poly, vertices, &mut no_diagonals, &mut partition);
    assert_eq!(partition.len(), 0);
}
