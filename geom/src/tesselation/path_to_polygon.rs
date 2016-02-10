use tesselation::polygon::*;
use tesselation::path::*;
use tesselation::{ WindingOrder };

use vodk_id::ReverseIdRange;

pub fn complex_path_to_polygon(path: ComplexPathSlice) -> Result<ComplexPolygon, ()> {
    let mut polygon = ComplexPolygon::new();

    // TODO: for now we consider that the first path is the contour and the other
    // paths are holes...
    let mut is_first = true;
    for sp in path.path_ids() {
        let sub_path = path.sub_path(sp);

        if sub_path.info().winding_order.is_none() {
            continue;
        }

        let reverse = if sub_path.info().winding_order == Some(WindingOrder::Clockwise) { !is_first }
                      else { is_first };

        let path_info = sub_path.info();
        let mut poly = if reverse { Polygon::from_vertices(ReverseIdRange::new(path_info.range)) }
                       else { Polygon::from_vertices(path_info.range) };

        poly.info = PolygonInfo {
            aabb: Some(path_info.aabb),
            is_convex: path_info.is_convex,
            is_y_monotone: path_info.is_y_monotone,
            has_beziers: path_info.has_beziers,
            op: if is_first { Operator::Add } else { Operator::Substract },
        };

        polygon.sub_polygons.push(poly);

        is_first = false;
    }

    return Ok(polygon);
}


