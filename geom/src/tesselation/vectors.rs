use std::f32::consts::PI;

pub static X: usize = 0;
pub static Y: usize = 1;
pub static Z: usize = 2;
pub static W: usize = 3;

pub static U: usize = 0;
pub static V: usize = 1;

pub type Vec2 = [f32; 2];
pub type Vec3 = [f32; 3];
pub type Vec4 = [f32; 4];

pub fn vec2_square_len(a: Vec2) -> f32 { a[X]*a[X] + a[Y]*a[Y] }
pub fn vec2_len(a: Vec2) -> f32 { vec2_square_len(a).sqrt() }
pub fn vec2_add(a: Vec2, b: Vec2) -> Vec2 { [a[X]+b[X], a[Y]+b[Y]] }
pub fn vec2_sub(a: Vec2, b: Vec2) -> Vec2 { [a[X]-b[X], a[Y]-b[Y]] }
pub fn vec2_mul(a: Vec2, b: f32) -> Vec2 { [a[X]*b, a[Y]*b] }
pub fn vec2_cross(a: Vec2, b: Vec2) -> f32 { a[X]*b[Y] - a[Y]*b[X] }

pub fn f32_almost_eq(a: f32, b:f32) -> bool { (a - b).abs() < 0.000001 }
pub fn vec2_almost_eq(a: Vec2, b: Vec2) -> bool {
    vec2_square_len(vec2_sub(a, b)) < 0.000001
}


/// Angle between vectors v1 and v2 (oriented clockwise with y pointing downward).
///
/// (equivalent to counter-clockwise if y points upward)
///
/// ex: directed_angle([0,1], [1,0]) = 3/2 Pi rad
///     x       __
///   0-->     /  \
///  y|       |  x--> v2
///   v        \ |v1
///              v
pub fn directed_angle(v1: Vec2, v2: Vec2) -> f32 {
    let a = (v2.y()).atan2(v2.x()) - (v1.y()).atan2(v1.x());
    return if a < 0.0 { a + 2.0 * PI } else { a };
}

#[test]
pub fn test_directed_angle() {
    assert!(f32_almost_eq(directed_angle([1.0, 1.0], [1.0, 1.0]), 0.0));
    assert!(f32_almost_eq(directed_angle([1.0, 0.0], [0.0, 1.0]), PI * 0.5));
    assert!(f32_almost_eq(directed_angle([1.0, 0.0], [-1.0, 0.0]), PI));
    assert!(f32_almost_eq(directed_angle([1.0, 0.0], [0.0, -1.0]), PI * 1.5));

    assert!(f32_almost_eq(directed_angle([1.0, -1.0], [1.0, 0.0]), PI * 0.25));
    assert!(f32_almost_eq(directed_angle([1.0, -1.0], [1.0, 1.0]), PI * 0.5));
    assert!(f32_almost_eq(directed_angle([1.0, -1.0], [-1.0, 1.0]), PI));
    assert!(f32_almost_eq(directed_angle([1.0, -1.0], [-1.0, -1.0]), PI * 1.5));

    assert!(f32_almost_eq(directed_angle([10.0, -10.0], [3.0, 0.0]), PI * 0.25));
    assert!(f32_almost_eq(directed_angle([10.0, -10.0], [3.0, 3.0]), PI * 0.5));
    assert!(f32_almost_eq(directed_angle([10.0, -10.0], [-3.0, 3.0]), PI));
    assert!(f32_almost_eq(directed_angle([10.0, -10.0], [-3.0, -3.0]), PI * 1.5));

    assert!(f32_almost_eq(directed_angle([-1.0, 0.0], [1.0, 0.0]), PI));
    assert!(f32_almost_eq(directed_angle([-1.0, 0.0], [0.0, 1.0]), PI * 1.5));
    assert!(f32_almost_eq(directed_angle([-1.0, 0.0], [0.0, -1.0]), PI * 0.5));
}

//pub trait Attribute<AttributeType, AttributeName> {
pub trait Attribute<AttributeType> {
    fn get<'l>(&'l self) -> &'l AttributeType;
    fn get_mut<'l>(&'l mut self) -> &'l mut AttributeType;
}

impl<T> Attribute<T> for T {
    fn get(&self) -> &T { self }
    fn get_mut(&mut self) -> &mut T { self }
}

pub struct MtlId { pub handle: u32 }

pub trait MaterialId {
    fn mtl(&self) -> MtlId;
    //fn mtl_mut(&mut self) -> &mut MtlId;
}

pub trait Position2D {
    fn position(&self) -> [f32; 2];
    //fn position_mut(&mut self) -> &mut Vector2D<Self::Unit>;
    fn x(&self) -> f32 { self.position()[X] }
    fn y(&self) -> f32 { self.position()[Y] }
}

pub trait Position3D {
    fn position(&self) -> [f32; 3];
    //fn position_mut(&mut self) -> &mut Vector3D<Self::Unit>;
    fn x(&self) -> f32 { self.position()[X] }
    fn y(&self) -> f32 { self.position()[Y] }
    fn z(&self) -> f32 { self.position()[Z] }
}

pub trait Position4D {
    fn position(&self) -> [f32; 4];
    //fn position_mut(&mut self) -> &mut Vector4D<Self::Unit>;
    fn x(&self) -> f32 { self.position()[X] }
    fn y(&self) -> f32 { self.position()[Y] }
    fn z(&self) -> f32 { self.position()[Z] }
    fn w(&self) -> f32 { self.position()[W] }
}

pub trait Normal2D {
    fn normal(&self) -> [f32; 2];
    //fn normal_mut(&mut self) -> &mut Vector2D<Self::Unit>;
    fn nx(&self) -> f32 { self.normal()[X] }
    fn ny(&self) -> f32 { self.normal()[Y] }
}

pub trait Normal3D {
    fn normal(&self) -> [f32; 3];
    //fn normal_mut(&mut self) -> &mut Vector3D<Self::Unit>;
    fn nx(&self) -> f32 { self.normal()[X] }
    fn ny(&self) -> f32 { self.normal()[Y] }
    fn nz(&self) -> f32 { self.normal()[Z] }
}

pub trait Normal4D {
    fn normal(&self) -> [f32; 4];
    //fn normal_mut(&mut self) -> &mut Vector4D<Self::Unit>;
    fn nx(&self) -> f32 { self.normal()[X] }
    fn ny(&self) -> f32 { self.normal()[Y] }
    fn nz(&self) -> f32 { self.normal()[Z] }
    fn nw(&self) -> f32 { self.normal()[W] }
}

pub trait TextureCoordinates {
    fn uv(&self) -> [f32; 2];
    //fn uv_mut(&mut self) -> &mut Vector2D<Self::Unit>;
    fn u(&self) -> f32 { self.uv()[X] }
    fn v(&self) -> f32 { self.uv()[Y] }
}

#[derive(Copy, Clone, Debug)]
pub struct Rgba<T> { r: T, g: T, b: T, a: T }

pub trait Color {
    type ScalarType: Copy;
    fn rgba(&self) -> &Rgba<Self::ScalarType>;
    //fn rgba_mut(&mut self) -> &mut Rgba<Self::ScalarType>;
    fn r(&self) -> Self::ScalarType { self.rgba().r }
    fn g(&self) -> Self::ScalarType { self.rgba().g }
    fn b(&self) -> Self::ScalarType { self.rgba().b }
    fn a(&self) -> Self::ScalarType { self.rgba().a }
}

impl Position2D for Vec2 { fn position(&self) -> Vec2 { *self } }

impl Position3D for Vec3 { fn position(&self) -> Vec3 { *self } }

impl Position4D for Vec4 { fn position(&self) -> Vec4 { *self } }

impl TextureCoordinates for Vec2 { fn uv(&self) -> Vec2 { *self } }

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub fn top_left(&self) -> Vec2 { [self.x, self.y] }

    pub fn top_right(&self) -> Vec2 { [self.x_most(), self.y] }

    pub fn bottom_right(&self) -> Vec2 { [self.x_most(), self.y_most()] }

    pub fn bottom_left(&self) -> Vec2 { [self.x, self.y_most()] }

    pub fn size(&self) -> Vec2 { [self.width, self.height] }

    pub fn x_most(&self) -> f32 { self.x + self.width }

    pub fn y_most(&self) -> f32 { self.y + self.height }

    pub fn is_empty(&self) -> bool { self.x == 0.0 || self.y == 0.0 }

    pub fn contains(&self, p: Vec2) -> bool {
        return self.x <= p.x() && self.y <= p.y() && self.x_most() >= p.x() && self.y_most() >= p.y();
    }

    pub fn intersects(&self, other: &Rect) -> bool {
        return self.x < other.x_most() && other.x < self.x_most() &&
            self.y < other.y_most() && other.y < self.y_most();
    }

    pub fn inflate(&mut self, d: f32) {
        self.x -= d;
        self.y -= d;
        self.width += 2.0*d;
        self.height += 2.0*d;
    }

    pub fn deflate(&mut self, d: f32) { self.inflate(-d); }

    pub fn translate(&mut self, v: Vec2) {
        self.x += v.x();
        self.y += v.y();
    }

    pub fn scale(&mut self, s: Vec2) {
        self.x *= s.x();
        self.y *= s.y();
    }
}

impl ::std::default::Default for Rect {
    fn default() -> Rect { Rect { x: 0.0, y: 0.0, width: 0.0, height: 0.0 } }
}
