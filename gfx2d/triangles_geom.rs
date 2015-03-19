
use range::Range;
use math::units::world;
use math::units::texels;
use color::Rgba;
use geom_utils::{extrude_along_tangent};
use style::{StrokeStyle, FillStyle, StrokeFlags};
use style;
use std::num::FloatMath;
use shapes;
use std::default::Default;

static PI: f32 = 3.1415;

pub trait VertexType2D: Copy+Default {
    fn from_pos(pos: &world::Vec2) -> Self;
    fn set_pos(&mut self, &world::Vec2);
    fn set_uv(&mut self, &texels::Vec2);
    fn set_color(&mut self, &Rgba<u8>);
}

pub struct TriangleStream<'l, T: 'l> {
    pub vertices: &'l mut[T],
    pub vertex_cursor: usize,
}

impl<'l, T: Copy> TriangleStream<'l, T> {
    pub fn push_vertex(&mut self, vertex: &T) {
        self.vertices[self.vertex_cursor] = *vertex;
        self.vertex_cursor += 1;
    }

    pub fn push_quad(&mut self, a: &T, b: &T, c: &T, d: &T) {
        self.push_vertex(a);
        self.push_vertex(b);
        self.push_vertex(c);
        self.push_vertex(a);
        self.push_vertex(c);
        self.push_vertex(d);
    }

    pub fn push_triangle(&mut self, a: &T, b: &T, c: &T) {
        self.push_vertex(a);
        self.push_vertex(b);
        self.push_vertex(c);
    }
}


pub fn fill_rectangle<'l, T: VertexType2D>(
    stream: &mut TriangleStream<'l, T>,
    rectangle: &world::Rectangle,
    transform: &world::Mat3,
    fill: FillStyle<'l>,
) -> Range {
    let first_vertex = stream.vertex_cursor as u16;
    let uv_rect = texels::rect(0.0, 0.0, 1.0, 1.0);

    let mut a: T = VertexType2D::from_pos(&transform.transform_2d(&rectangle.top_left()));
    let mut b: T = VertexType2D::from_pos(&transform.transform_2d(&rectangle.top_right()));
    let mut c: T = VertexType2D::from_pos(&transform.transform_2d(&rectangle.bottom_right()));
    let mut d: T = VertexType2D::from_pos(&transform.transform_2d(&rectangle.bottom_left()));

    match fill {
        FillStyle::None => {}
        FillStyle::Color(color) => {
            a.set_color(color);
            b.set_color(color);
            c.set_color(color);
            d.set_color(color);
        }
        FillStyle::Texture(uv_transform) => {
            a.set_uv(&uv_transform.transform_2d(&uv_rect.top_left()));
            b.set_uv(&uv_transform.transform_2d(&uv_rect.top_right()));
            c.set_uv(&uv_transform.transform_2d(&uv_rect.bottom_right()));
            d.set_uv(&uv_transform.transform_2d(&uv_rect.bottom_left()));
        }
    }
    stream.push_quad(&a, &b, &c, &d);
    return Range {
        first: first_vertex,
        count: stream.vertex_cursor as u16 - first_vertex,
    };
}


pub fn fill_circle<'l, T: VertexType2D>(
    stream: &mut TriangleStream<'l, T>,
    circle: &shapes::Circle,
    num_points: u32,
    transform: &world::Mat3,
    fill: FillStyle<'l>,
) -> Range {
    let first_vertex = stream.vertex_cursor as u16;

    let mut center: T = VertexType2D::from_pos(
        &transform.transform_2d(&world::vec2(
            circle.center.x,
            circle.center.y
        ))
    );

    match fill {
        FillStyle::None => {}
        FillStyle::Color(color) => { center.set_color(color) }
        FillStyle::Texture(uv_transform) => {
            center.set_uv(&uv_transform.transform_2d(&texels::vec2(0.5, 0.5)))
        }
    }

    for i in range(0, num_points+1) {
        let dx_a = (i as f32 / num_points as f32 * 2.0 * PI).cos();
        let dy_a = (i as f32 / num_points as f32 * 2.0 * PI).sin();
        let mut a: T = VertexType2D::from_pos(
            &transform.transform_2d(&world::vec2(
                circle.center.x + circle.radius * dx_a,
                circle.center.y + circle.radius * dy_a
            ))
        );

        let dx_b = ((i+1) as f32 / num_points as f32 * 2.0 * PI).cos();
        let dy_b = ((i+1) as f32 / num_points as f32 * 2.0 * PI).sin();
        let mut b: T = VertexType2D::from_pos(
            &transform.transform_2d(&world::vec2(
                circle.center.x + circle.radius * dx_b,
                circle.center.y + circle.radius * dy_b
            ))
        );

        match fill {
            FillStyle::None => {}
            FillStyle::Color(color) => {
                a.set_color(color);
                b.set_color(color);
            }
            FillStyle::Texture(uv_transform) => {
                a.set_uv(
                    &uv_transform.transform_2d(&texels::vec2(
                        0.5 + dx_a * 0.5,
                        0.5 + dy_a * 0.5
                    ))
                );
                b.set_uv(
                    &uv_transform.transform_2d(&texels::vec2(
                        0.5 + dx_b * 0.5,
                        0.5 + dy_b * 0.5
                    ))
                );
            }
        }
        stream.push_triangle(&center, &a, &b);
    }
    return Range {
        first: first_vertex,
        count: stream.vertex_cursor as u16 - first_vertex,
    };
}


pub fn stroke_path<'l, T: VertexType2D>(
    stream: &mut TriangleStream<'l, T>,
    path: &[world::Vec2],
    aabb: &world::Rectangle,
    transform: &world::Mat3,
    style: StrokeStyle<'l>,
    thickness: f32,
    flags: StrokeFlags
) -> Range {
    let first_vertex = stream.vertex_cursor as u16;

    let is_closed = flags & style::STROKE_CLOSED != 0;

    let mut prev_v1: T = Default::default();
    let mut prev_v2: T = Default::default();
    for i in range(0, path.len()) {
        let mut p1 = path[i];
        let mut p2 = path[i];

        if flags & style::STROKE_INWARD == 0 && flags & style::STROKE_OUTWARD == 0 {
            p1 = extrude_along_tangent(path, i, thickness * 0.5, is_closed);
            p2 = extrude_along_tangent(path, i, -thickness * 0.5, is_closed);
        } else if flags & style::STROKE_OUTWARD != 0 {
            p1 = extrude_along_tangent(path, i, thickness, is_closed);
        } else if flags & style::STROKE_INWARD != 0 {
            p2 = extrude_along_tangent(path, i, -thickness, is_closed);
        }

        let mut v1: T = VertexType2D::from_pos(&transform.transform_2d(&p1));
        let mut v2: T = VertexType2D::from_pos(&transform.transform_2d(&p2));

        match style {
            StrokeStyle::None => {},
            StrokeStyle::Color(color) => {
                v1.set_color(color);
                v2.set_color(color);
            }
            StrokeStyle::Texture(_) => { panic!("TODO"); }
        }
        if i > 0 {
            stream.push_quad(&prev_v1, &prev_v2, &v2, &v1);
        }
        prev_v1 = v1;
        prev_v2 = v2;
    }

    return Range {
        first: first_vertex,
        count: stream.vertex_cursor as u16 - first_vertex,
    };
}
