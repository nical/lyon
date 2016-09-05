use euclid;
use fixed;

pub use euclid::Point2D;

pub type Point = euclid::Point2D<f32>;
pub type IntPoint = euclid::Point2D<i32>;
pub type Int64Point = euclid::Point2D<i64>;
// Point and Vec2 are the same type but they should probably be separate types.
pub type Vec2 = euclid::Point2D<f32>;
pub type IntVec2 = euclid::Point2D<i32>;
pub type Size = euclid::Size2D<f32>;
pub type IntSize = euclid::Size2D<i32>;
pub type Rect = euclid::Rect<f32>;
pub type IntRect = euclid::Rect<i32>;

pub type FixedPoint32 = fixed::Fp32<fixed::_8>;
pub type TessPoint = Point2D<FixedPoint32>;
pub fn fixed(val: f32) -> FixedPoint32 { FixedPoint32::from_f32(val) }

pub type Vec3 = euclid::Point3D<f32>;
pub type IntVec3 = euclid::Point3D<i32>;

pub type Mat4 = euclid::Matrix4D<f32>;

pub fn point(x: f32, y: f32) -> Point { vec2(x, y) }
pub fn vec2(x: f32, y: f32) -> Vec2 { Vec2::new(x, y) }
pub fn int_vec2(x: i32, y: i32) -> IntVec2 { IntVec2::new(x, y) }
pub fn size(w: f32, h: f32) -> Size { Size::new(w, h) }
pub fn int_size(w: i32, h: i32) -> IntSize { IntSize::new(w, h) }
pub fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect { Rect::new(vec2(x, y), size(w, h)) }
pub fn int_rect(x: i32, y: i32, w: i32, h: i32) -> IntRect { IntRect::new(int_vec2(x, y), int_size(w, h)) }

pub trait Vec2Tuple<S> { fn tuple(self) -> (S, S); }

impl<S> Vec2Tuple<S> for euclid::Point2D<S> { fn tuple(self) ->(S, S) { (self.x, self.y) } }

pub trait Vec2Array<S> { fn array(self) -> [S; 2]; }

impl<S> Vec2Array<S> for euclid::Point2D<S> { fn array(self) ->[S; 2] { [self.x, self.y] } }

pub trait Vec2Length {
    fn length(self) -> f32;
}

pub trait Vec2SquareLength {
    fn square_length(self) -> f32;
}

impl Vec2Length for Vec2 {
    fn length(self) -> f32 { self.square_length().sqrt() }
}

impl Vec2SquareLength for Vec2 {
    fn square_length(self) -> f32 { self.x*self.x + self.y*self.y }
}
