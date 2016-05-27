use tesselation::{
    vertex_id, vertex_id_range,
    VertexId, VertexIdRange,
    VertexSlice, MutVertexSlice,
//    crash,
};

use tesselation::bezier::*;

use vodk_math::{ Vec2, vec2, Rect, Untyped };

use vodk_id::{ Id, IdRange, ToIndex };

#[derive(Debug)]
pub struct Path_;
pub type PathId = Id<Path_, u16>;
pub type PathIdRange = IdRange<Path_, u16>;
pub fn path_id(idx: u16) -> PathId { PathId::new(idx) }

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum PointType {
    Normal,
    Control,
}

#[derive(Copy, Clone, Debug)]
pub struct PointData {
    pub position: Vec2,
    pub point_type: PointType,
}

#[derive(Clone, Debug)]
pub struct Path {
    vertices: Vec<PointData>,
    sub_paths: Vec<PathInfo>,
}

trait LineTo {
    fn line_to(&mut self, to: Vec2);
}

impl Path {
    pub fn new() -> Path {
        Path { vertices: Vec::new(), sub_paths: Vec::new() }
    }

    pub fn vertices(&self) -> VertexSlice<PointData> { VertexSlice::new(&self.vertices[..]) }

    pub fn mut_vertices(&mut self) -> MutVertexSlice<PointData> { MutVertexSlice::new(&mut self.vertices[..]) }

    pub fn num_vertices(&self) -> usize { self.as_slice().num_vertices() }

    pub fn sub_path(&self, id: PathId) -> SubPathSlice {
        SubPathSlice {
            vertices: VertexSlice::new(&self.vertices[..]),
            info: &self.sub_paths[id.handle.to_index()]
        }
    }

    pub fn path_ids(&self) -> PathIdRange {
        IdRange::new(0, self.sub_paths.len() as u16)
    }

    pub fn as_slice(&self) -> PathSlice {
        PathSlice {
            vertices: VertexSlice::new(&self.vertices[..]),
            sub_paths: &self.sub_paths[..],
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct PathVertexId {
    pub vertex_id: VertexId,
    pub path_id: PathId,
}

pub struct PathVertexIdRange {
    range: VertexIdRange,
    path_id: PathId,
}

impl Iterator for PathVertexIdRange {
    type Item = PathVertexId;
    fn next(&mut self) -> Option<PathVertexId> {
        return if let Some(next) = self.range.next() {
            Some(PathVertexId {
                vertex_id: next,
                path_id: self.path_id
            })
        } else {
            None
        };
    }
}

#[derive(Copy, Clone)]
pub struct PathSlice<'l> {
    vertices: VertexSlice<'l,PointData>,
    sub_paths: &'l[PathInfo],
}

impl<'l> PathSlice<'l> {

    pub fn vertices(&self) -> VertexSlice<PointData> { self.vertices }

    pub fn vertex_ids(&self, sub_path: PathId) -> PathVertexIdRange {
        PathVertexIdRange {
            range: self.sub_path(sub_path).vertex_ids(),
            path_id: sub_path,
        }
    }

    pub fn num_vertices(&self) -> usize { self.vertices.len() }

    pub fn num_sub_paths(&self) -> usize { self.sub_paths.len() }

    pub fn sub_path(&self, id: PathId) -> SubPathSlice {
        SubPathSlice {
            vertices: self.vertices,
            info: &self.sub_paths[id.handle.to_index()]
        }
    }

    pub fn path_ids(&self) -> PathIdRange {
        IdRange::new(0, self.sub_paths.len() as u16)
    }

    pub fn vertex(&self, id: PathVertexId) -> &PointData {
        &self.vertices[id.vertex_id]
    }

    pub fn next(&self, id: PathVertexId) -> PathVertexId {
        PathVertexId {
            path_id: id.path_id,
            vertex_id: self.sub_path(id.path_id).next(id.vertex_id),
        }
    }

    pub fn previous(&self, id: PathVertexId) -> PathVertexId {
        PathVertexId {
            path_id: id.path_id,
            vertex_id: self.sub_path(id.path_id).previous(id.vertex_id),
        }
    }
}

#[derive(Copy, Clone)]
pub struct SubPathSlice<'l> {
    vertices: VertexSlice<'l, PointData>,
    info: &'l PathInfo,
}

impl<'l> SubPathSlice<'l> {
    pub fn info(&self) -> &'l PathInfo { self.info }

    pub fn vertex(&self, id: VertexId) -> &PointData { &self.vertices[id] }

    pub fn first(&self) -> VertexId { self.info.range.first }

    pub fn last(&self) -> VertexId {
        vertex_id(self.info.range.first.handle + self.info.range.count - 1)
    }

    pub fn next(&self, id: VertexId) -> VertexId {
        let first = self.info.range.first.handle;
        let last = first + self.info.range.count - 1;
        debug_assert!(id.handle >= first);
        debug_assert!(id.handle <= last);
        return Id::new(if id.handle == last { first } else { id.handle + 1 });
    }

    pub fn previous(&self, id: VertexId) -> VertexId {
        let first = self.info.range.first.handle;
        let last = first + self.info.range.count - 1;
        debug_assert!(id.handle >= first);
        debug_assert!(id.handle <= last);
        return Id::new(if id.handle == first { last } else { id.handle - 1 });
    }

    pub fn next_vertex(&self, id: VertexId) -> &PointData {
        self.vertex(self.next(id))
    }

    pub fn previous_vertex(&self, id: VertexId) -> &PointData {
        self.vertex(self.previous(id))
    }

    pub fn vertex_ids(&self) -> VertexIdRange { self.info().range }

    pub fn num_vertices(&self) -> usize { self.vertices.len() }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct PathInfo {
    pub aabb: Rect,
    pub range: VertexIdRange,
    pub has_beziers: Option<bool>,
    pub is_closed: bool,
}

pub struct PathBuilder {
    path: Path,
    last_position: Vec2,
    last_ctrl: Vec2,
    top_left: Vec2,
    bottom_right: Vec2,
    tolerance: f32,
    offset: u16,
    // flags
    has_beziers: bool,
    flatten: bool,
    building: bool,
}

impl PathBuilder {
    pub fn new() -> PathBuilder {
        PathBuilder {
            path: Path::new(),
            last_position: vec2(0.0, 0.0),
            last_ctrl: vec2(0.0, 0.0),
            top_left: vec2(0.0, 0.0),
            bottom_right: vec2(0.0, 0.0),
            offset: 0,
            tolerance: 0.05,
            has_beziers: false,
            flatten: false,
        }
    }

    pub fn finish(self) -> Path { self.path }

    pub fn set_flattening(&mut self, flattening: bool) { self.flatten = flattening }

    pub fn set_tolerance(&mut self, tolerance: f32) { self.tolerance = tolerance }

    pub fn move_to(&mut self, to: Vec2)
    {
        if self.building {
            self.finish_sub_path(false);
        }
        self.last_position = to;
        self.last_ctrl = to;
        self.top_left = to;
        self.bottom_right = to;
        self.building = false;
    }

//    pub fn begin(path: &'l mut Path, pos: Vec2) {
//        let offset = path.vertices.len() as u16;
//        path.vertices.push(PointData { position: pos, point_type: PointType::Normal });
//        PathBuilder {
//            path: path,
//            last_position: pos,
//            last_ctrl: pos,
//            top_left: vec2(0.0, 0.0),
//            bottom_right: vec2(0.0, 0.0),
//            offset: offset,
//            tolerance: 0.05,
//            has_beziers: false,
//            flatten: false,
//        }
//    }

    pub fn line_to(mut self, to: Vec2) {
        self.last_ctrl = to;
        self.line_step_to(to);
    }

    fn line_step_to(mut self, to: Vec2) {
        self.push(to, PointType::Normal);
    }

    pub fn relative_line_to(mut self, to: Vec2) {
        let offset = self.last_position;
        assert!(!offset.x.is_nan() && !offset.y.is_nan());
        self.push(offset + to, PointType::Normal);
    }

    pub fn quadratic_bezier_to(mut self, ctrl: Vec2, to: Vec2) {
        self.last_ctrl = ctrl;
        if self.flatten {
            let from = self.last_position;
            let cubic = QuadraticBezierSegment { from: from, cp: ctrl, to: to }.to_cubic();
            flatten_cubic_bezier(cubic, self.tolerance, &mut self);
        } else {
            self.push(ctrl, PointType::Control);
            self.push(to, PointType::Normal);
            self.has_beziers = true;
        }
    }

    pub fn relative_quadratic_bezier_to(&mut self, ctrl: Vec2, to: Vec2) {
        let offset = self.last_position;
        self.quadratic_bezier_to(ctrl + offset, to + offset);
    }

    pub fn cubic_bezier_to(mut self, ctrl1: Vec2, ctrl2: Vec2, to: Vec2) {
        self.last_ctrl = ctrl2;
        if self.flatten {
            flatten_cubic_bezier(
                CubicBezierSegment{
                    from: self.last_position,
                    cp1: ctrl1,
                    cp2: ctrl2,
                    to: to,
                },
                self.tolerance,
                &mut self
            );
        } else {
            self.push(ctrl1, PointType::Control);
            self.push(ctrl2, PointType::Control);
            self.push(to, PointType::Normal);
            self.has_beziers = true;
        }
    }

    pub fn relative_cubic_bezier_to(&mut self, ctrl1: Vec2, ctrl2: Vec2, to: Vec2) {
        let offset = self.last_position;
        self.cubic_bezier_to(ctrl1 + offset, ctrl2 + offset, to + offset);
    }

    pub fn cubic_bezier_symetry_to(&mut self, ctrl2: Vec2, to: Vec2) {
        let ctrl = self.last_position + (self.last_position - self.last_ctrl);
        self.cubic_bezier_to(ctrl, ctrl2, to);
    }

    pub fn relative_cubic_bezier_symetry_to(&mut self, ctrl2: Vec2, to: Vec2) {
        let ctrl = self.last_position - self.last_ctrl;
        self.relative_cubic_bezier_to(ctrl, ctrl2, to);
    }

    pub fn quadratic_bezier_symetry_to(&mut self, to: Vec2) {
        let ctrl = self.last_position + (self.last_position - self.last_ctrl);
        self.quadratic_bezier_to(ctrl, to);
    }

    pub fn relative_quadratic_bezier_symetry_to(&mut self, to: Vec2) {
        let ctrl = self.last_position - self.last_ctrl;
        self.relative_quadratic_bezier_to(ctrl, to);
    }

    pub fn horizontal_line_to(&mut self, x: f32) {
        let y = self.last_position.y;
        self.line_to(vec2(x, y));
    }

    pub fn relative_horizontal_line_to(&mut self, dx: f32) {
        let p = self.last_position;
        self.line_to(vec2(p.x + dx, p.y));
    }

    pub fn vertical_line_to(&mut self, y: f32) {
        let x = self.last_position.x;
        self.line_to(vec2(x, y));
    }

    pub fn relative_vertical_line_to(&mut self, dy: f32) {
        let p = self.last_position;
        self.line_to(vec2(p.x, p.y + dy));
    }

    pub fn end(&mut self) -> PathId { self.finish_sub_path(false) }

    pub fn close(&mut self) -> PathId { self.finish_sub_path(true) }

    fn finish_sub_path(mut self, mut closed: bool) -> PathId {
        let offset = self.offset as usize;
        let last = self.path.vertices.len() - 1;
        // If the first and last vertices are the same, remove the last vertex.
        let last = if last > 0 && self.path.vertices[last].position.fuzzy_eq(self.path.vertices[offset].position) {
            self.path.vertices.pop();
            closed = true;
            last - 1
        } else { last };

        let vertex_count = last - offset + 1;

        let vertex_range = vertex_id_range(self.offset, self.offset + vertex_count as u16);
        let aabb = Rect::new(
            self.top_left.x, self.top_left.y,
            self.bottom_right.x - self.top_left.x, self.bottom_right.y - self.top_left.y,
        );

        let shape_info = PathInfo {
            range: vertex_range,
            aabb: aabb,
            has_beziers: Some(self.has_beziers),
            is_closed: closed,
        };

        let index = path_id(self.path.sub_paths.len() as u16);
        self.path.sub_paths.push(shape_info);
        return index;
    }

    fn push(&mut self, point: Vec2, ptype: PointType) {
        if point == self.last_position {
            return;
        }

        self.building = true;

        if self.path.vertices.len() == 0 {
            self.top_left = point;
            self.bottom_right = point;
        } else {
            if point.x < self.top_left.x { self.top_left.x = point.x; }
            if point.y < self.top_left.y { self.top_left.y = point.y; }
            if point.x > self.bottom_right.x { self.bottom_right.x = point.x; }
            if point.y > self.bottom_right.y { self.bottom_right.y = point.y; }
        }
        self.path.vertices.push(PointData{ position: point, point_type: ptype });
        self.last_position = point;
    }
}

pub fn flatten_cubic_bezier(
    bezier: CubicBezierSegment<Untyped>,
    tolerance: f32,
    path: &mut PathBuilder
) {
    let (t1, t2) = find_cubic_bezier_inflection_points(&bezier);
    let count = if t1.is_none() { 0 } else if t2.is_none() { 1 } else { 2 };
    let t1 = if let Some(t) = t1 { t } else { -1.0 };
    let t2 = if let Some(t) = t2 { t } else { -1.0 };

    // Check that at least one of the inflection points is inside [0..1]
    if count == 0 || ((t1 < 0.0 || t1 > 1.0) && (count == 1 || (t2 < 0.0 || t2 > 1.0))) {
        return flatten_cubic_bezier_segment(bezier, tolerance, path);
    }

    let mut t1min = t1;
    let mut t1max = t1;
    let mut t2min = t2;
    let mut t2max = t2;

    let mut remaining_cp = bezier;

    // For both inflection points, calulate the range where they can be linearly
    // approximated if they are positioned within [0,1]
    if count > 0 && t1 >= 0.0 && t1 < 1.0 {
        find_cubic_bezier_inflection_approximation_range(&bezier, t1, tolerance, &mut t1min, &mut t1max);
    }
    if count > 1 && t2 >= 0.0 && t2 < 1.0 {
        find_cubic_bezier_inflection_approximation_range(&bezier, t2, tolerance, &mut t2min, &mut t2max);
    }
    let mut next_bezier = bezier;
    let mut prev_bezier = bezier;

    // Process ranges. [t1min, t1max] and [t2min, t2max] are approximated by line
    // segments.
    if count == 1 && t1min <= 0.0 && t1max >= 1.0 {
        // The whole range can be approximated by a line segment.
        path.line_step_to(bezier.to);
        return;
    }

    if t1min > 0.0 {
        // Flatten the Bezier up until the first inflection point's approximation
        // point.
        split_cubic_bezier(&bezier, t1min, Some(&mut prev_bezier), Some(&mut remaining_cp));
        flatten_cubic_bezier_segment(prev_bezier, tolerance, path);
    }
    if t1max >= 0.0 && t1max < 1.0 && (count == 1 || t2min > t1max) {
        // The second inflection point's approximation range begins after the end
        // of the first, approximate the first inflection point by a line and
        // subsequently flatten up until the end or the next inflection point.
        split_cubic_bezier(&bezier, t1max, None, Some(&mut next_bezier));

        path.line_step_to(next_bezier.from);

        if count == 1 || (count > 1 && t2min >= 1.0) {
            // No more inflection points to deal with, flatten the rest of the curve.
            flatten_cubic_bezier_segment(next_bezier, tolerance, path);
        }
    } else if count > 1 && t2min > 1.0 {
        // We've already concluded t2min <= t1max, so if this is true the
        // approximation range for the first inflection point runs past the
        // end of the curve, draw a line to the end and we're done.
        path.line_step_to(bezier.to);
        return;
    }

    if count > 1 && t2min < 1.0 && t2max > 0.0 {
        if t2min > 0.0 && t2min < t1max {
            // In this case the t2 approximation range starts inside the t1
            // approximation range.
            split_cubic_bezier(&bezier, t1max, None, Some(&mut next_bezier));
            path.line_step_to(next_bezier.from);
        } else if t2min > 0.0 && t1max > 0.0 {
            split_cubic_bezier(&bezier, t1max, None, Some(&mut next_bezier));

            // Find a control points describing the portion of the curve between t1max and t2min.
            let t2mina = (t2min - t1max) / (1.0 - t1max);
            let tmp = next_bezier;
            split_cubic_bezier(&tmp, t2mina, Some(&mut prev_bezier), Some(&mut next_bezier));
            flatten_cubic_bezier_segment(prev_bezier, tolerance, path);
        } else if t2min > 0.0 {
            // We have nothing interesting before t2min, find that bit and flatten it.
            split_cubic_bezier(&bezier, t2min, Some(&mut prev_bezier), Some(&mut next_bezier));
            flatten_cubic_bezier_segment(prev_bezier, tolerance, path);
        }
        if t2max < 1.0 {
            // Flatten the portion of the curve after t2max
            split_cubic_bezier(&bezier, t2max, None, Some(&mut next_bezier));

            // Draw a line to the start, this is the approximation between t2min and
            // t2max.
            path.line_step_to(next_bezier.from);
            flatten_cubic_bezier_segment(next_bezier, tolerance, path);
            return;
        } else {
            // Our approximation range extends beyond the end of the curve.
            path.line_step_to(bezier.to);
            return;
        }
    }
}


fn flatten_cubic_bezier_segment<'l>(
    mut bezier: CubicBezierSegment<Untyped>,
    tolerance: f32,
    path: &mut PathBuilder
) {

    let end = bezier.to;

    // The algorithm implemented here is based on:
    // http://cis.usouthal.edu/~hain/general/Publications/Bezier/Bezier%20Offset%20Curves.pdf
    //
    // The basic premise is that for a small t the third order term in the
    // equation of a cubic bezier curve is insignificantly small. This can
    // then be approximated by a quadratic equation for which the maximum
    // difference from a linear approximation can be much more easily determined.
    let mut t = 0.0;
    while t < 1.0 {
        let v1 = bezier.cp1 - bezier.from;
        let v2 = bezier.cp2 - bezier.from;

        // To remove divisions and check for divide-by-zero, this is optimized from:
        // Float s2 = (v2.x * v1.y - v2.y * v1.x) / hypot(v1.x, v1.y);
        // t = 2 * Float(sqrt(tolerance / (3. * abs(s2))));
        let v1xv2 = v2.x * v1.y - v2.y * v1.x;
        let h = v1.x.hypot(v1.y);
        if v1xv2 * h == 0.0 {
            break;
        }
        let s2inv = h / v1xv2;

        t = 2.0 * (tolerance * s2inv.abs() / 3.0).sqrt();

        if t >= 0.999 {
            break;
        }

        bezier = bezier.split_in_place(t as f32);

        path.line_step_to(bezier.from);
    }

    path.line_step_to(end);
}


#[test]
fn test_path_builder_simple() {

    // clockwise
    {
        let mut path = PathBuilder::new();
        path.move_to(vec2(0.0, 0.0));
        path.line_to(vec2(1.0, 0.0));
        path.line_to(vec2(1.0, 1.0));
        let id = path.close();

        let path = path.finish();
        let info = path.sub_path(id).info();
        assert_eq!(path.vertices[0].position, vec2(0.0, 0.0));
        assert_eq!(path.vertices[1].position, vec2(1.0, 0.0));
        assert_eq!(path.vertices[2].position, vec2(1.0, 1.0));
        assert_eq!(path.vertices[0].point_type, PointType::Normal);
        assert_eq!(path.vertices[1].point_type, PointType::Normal);
        assert_eq!(path.vertices[2].point_type, PointType::Normal);
        assert_eq!(info.range, vertex_id_range(0, 3));
        assert_eq!(info.aabb, Rect::new(0.0, 0.0, 1.0, 1.0));
        let sub_path = path.sub_path(id);
        let first = sub_path.first();
        let next = sub_path.next(first);
        let prev = sub_path.previous(first);
        assert!(first != next);
        assert!(first != prev);
        assert!(next != prev);
        assert_eq!(first, sub_path.previous(next));
        assert_eq!(first, sub_path.next(prev));
    }

    // counter-clockwise
    {
        let mut path = PathBuilder::new();
        path.move_to(vec2(0.0, 0.0));
        path.line_to(vec2(1.0, 1.0));
        path.line_to(vec2(1.0, 0.0));
        let id = path.close();

        let path = path.finish();
        let info = path.sub_path(id).info();
        assert_eq!(info.range, vertex_id_range(3, 6));
        assert_eq!(info.aabb, Rect::new(0.0, 0.0, 1.0, 1.0));
    }

    // line_to back to the first vertex (should ignore the last vertex)
    {
        let mut path = PathBuilder::new();
        path.move_to(vec2(0.0, 0.0));
        path.line_to(vec2(1.0, 1.0));
        path.line_to(vec2(1.0, 0.0));
        path.line_to(vec2(0.0, 0.0));
        let id = path.close();

        let path = path.finish();
        let info = path.sub_path(id).info();
        assert_eq!(info.range, vertex_id_range(6, 9));
        assert_eq!(info.aabb, Rect::new(0.0, 0.0, 1.0, 1.0));
    }
}

#[test]
fn test_path_builder_simple_bezier() {
    // clockwise
    {
        let mut path = PathBuilder::new();
        path.move_to(vec2(0.0, 0.0));
        path.quadratic_bezier_to(vec2(1.0, 0.0), vec2(1.0, 1.0));
        let id = path.close();

        let path = path.finish();
        let info = path.sub_path(id).info();
        assert_eq!(info.range, vertex_id_range(0, 3));
        assert_eq!(info.aabb, Rect::new(0.0, 0.0, 1.0, 1.0));
    }

    // counter-clockwise
    {
        let mut path = PathBuilder::new();
        path.move_to(vec2(0.0, 0.0));
        path.quadratic_bezier_to(vec2(1.0, 1.0), vec2(1.0, 0.0));
        let id = path.close();

        let path = path.finish();
        let info = path.sub_path(id).info();
        assert_eq!(info.range, vertex_id_range(3, 6));
        assert_eq!(info.aabb, Rect::new(0.0, 0.0, 1.0, 1.0));
    }

    // a slightly more elaborate path
    {
        let mut path = PathBuilder::new();
        path.move_to(vec2(0.0, 0.0));
        path.line_to(vec2(0.1, 0.0));
        path.line_to(vec2(0.2, 0.1));
        path.line_to(vec2(0.3, 0.1));
        path.line_to(vec2(0.4, 0.0));
        path.line_to(vec2(0.5, 0.0));
        path.quadratic_bezier_to(vec2(0.5, 0.4), vec2(0.3, 0.4));
        path.line_to(vec2(0.1, 0.4));
        path.quadratic_bezier_to(vec2(-0.2, 0.1), vec2(-0.1, 0.0));
        let id = path.close();

        let path = path.finish();
        let info = path.sub_path(id).info();
        assert_eq!(info.aabb, Rect::new(-0.2, 0.0, 0.7, 0.4));
    }
}
