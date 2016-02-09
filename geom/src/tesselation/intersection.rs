use tesselation::{ VertexId, vertex_id, vertex_id_range };
use tesselation::polygon::*;
use tesselation::vectors::{ Position2D, Vec2, vec2_sub, vec2_len, directed_angle };
use vodk_id::id_vector::IdSlice;

#[derive(Debug)]
pub struct Error; // placeholder

struct Intersection<Poly: AbstractPolygon> {
    from: Poly::PointId,
    to: Poly::PointId,
    inter: VertexId,
    processed_face: bool,
    processed_opposite_face: bool,
}

pub struct Intersections<Poly: AbstractPolygon> {
    intersections: Vec<Intersection<Poly>>
}

impl<Poly: AbstractPolygon> Intersections<Poly> {
    pub fn new() -> Intersections<Poly> {
        Intersections {
            intersections: Vec::with_capacity(4),
        }
    }

    pub fn add_intersection(&mut self, from: Poly::PointId, to: Poly::PointId, inter: VertexId) {
        self.intersections.push(Intersection{
            from: from, to: to, inter: inter,
            processed_face: false, processed_opposite_face: false,
        });
    }

    pub fn is_empty(&self) -> bool { self.intersections.is_empty() }

    pub fn clear_flags(&mut self) {
        for inter in &mut self.intersections {
            inter.processed_face = false;
            inter.processed_opposite_face = false;
        }
    }
}

pub fn apply_intersections<Poly: AbstractPolygon, V: Position2D>(
    polygon: &Poly,
    vertices: IdSlice<VertexId, V>,
    intersections: &mut Intersections<Poly>,
    output: &mut Vec<Polygon>
) -> Result<(), Error> {
    Err(Error)
}

fn gen_polygon<Poly: AbstractPolygon, V: Position2D>(
    polygon: &Poly,
    vertices: IdSlice<VertexId, V>,
    intersections: &mut Intersections<Poly>,
    first_point: Poly::PointId,
    second_point: Poly::PointId,
) -> Result<Polygon, Error> {
    let mut new_poly = Polygon::new();
    let mut prev = first_point;
    let mut it = second_point;
    let mut loop_counter = 0;
    loop {
        //println!("\n\n ------ point {:?}", it);
        new_poly.push_vertex(polygon.vertex(it));

        let mut selected = None;
        let mut shortest_dist = ::std::f32::NAN;

        for i in 0..intersections.intersections.len() {
            let intersect = &intersections.intersections[i];
            let to = if intersect.from == it { intersect.to }
                     else if intersect.to == it { intersect.from }
                     else { continue; };
            let from_v = vertices[polygon.vertex(it)].position();
            let to_v = vertices[polygon.vertex(to)].position();
            let inter_v = vertices[intersect.inter].position();
            let dist = vec2_len(vec2_sub(from_v, inter_v));
            if shortest_dist.is_nan() || dist < shortest_dist {
                shortest_dist = dist;
                selected = Some(i);
            }
        }

        if let Some(idx) = selected {
        }

        if it == second_point && prev == first_point {
            // back to where we began, the work is done.
            break;
        }

        loop_counter += 1;
        if loop_counter > polygon.num_vertices() * 2 {
            return Err(Error);
        }
    }

    Err(Error)
}
