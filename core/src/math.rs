use euclid;
use fixed;

pub use euclid::{Point2D, Vector2D, TypedPoint2D, Radians};

pub type Point = euclid::Point2D<f32>;
pub type IntPoint = euclid::Point2D<i32>;
pub type Int64Point = euclid::Point2D<i64>;
pub type F64Point = euclid::Point2D<f64>;
// Point and Vec2 are the same type but they should probably be separate types.
pub type Vec2 = euclid::Vector2D<f32>;
pub type IntVec2 = euclid::Vector2D<i32>;
pub type Size = euclid::Size2D<f32>;
pub type IntSize = euclid::Size2D<i32>;
pub type Rect = euclid::Rect<f32>;
pub type IntRect = euclid::Rect<i32>;

pub type FixedPoint32 = fixed::Fp32<fixed::_16>;
pub type FixedPoint64 = fixed::Fp64<fixed::_16>;
pub type TessVec2 = Vector2D<FixedPoint32>;
pub type TessPoint = Point2D<FixedPoint32>;
pub type TessPoint64 = Point2D<FixedPoint64>;
#[inline]
pub fn fixed(val: f32) -> FixedPoint32 { FixedPoint32::from_f32(val) }

pub type Vec3 = euclid::Vector3D<f32>;
pub type IntVec3 = euclid::Vector3D<i32>;

pub type Mat4 = euclid::Transform3D<f32>;
pub type Transform2d = euclid::Transform2D<f32>;

pub use euclid::{vec2, rect};
pub use euclid::point2 as point;
pub use euclid::size2 as size;

#[inline]
pub fn int_vec2(x: i32, y: i32) -> IntVec2 { vec2(x, y) }
#[inline]
pub fn int_size(w: i32, h: i32) -> IntSize { IntSize::new(w, h) }
#[inline]
pub fn int_rect(x: i32, y: i32, w: i32, h: i32) -> IntRect { rect(x, y, w, h) }

#[inline]
pub fn rad(val: f32) -> Radians<f32> { Radians::new(val) }

pub trait Normalize {
    fn normalize(self) -> Self;
}

impl Normalize for Vec2 {
    #[inline]
    fn normalize(self) -> Self { self / self.length() }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BoolVec4 {
    pub x: bool,
    pub y: bool,
    pub z: bool,
    pub w: bool,
}

pub fn bvec4(x: bool, y: bool, z: bool, w: bool) -> BoolVec4 {
    BoolVec4 {
        x: x,
        y: y,
        z: z,
        w: w,
    }
}

impl BoolVec4 {
    #[inline]
    pub fn new(x: bool, y: bool, z: bool, w: bool) -> BoolVec4 { bvec4(x, y, z, w) }

    #[inline]
    pub fn any(self) -> bool { self.x || self.y || self.z || self.w }

    #[inline]
    pub fn all(self) -> bool { self.x && self.y && self.z && self.w }

    #[inline]
    pub fn and(self, other: BoolVec4) -> BoolVec4 {
        bvec4(self.x && other.x, self.y && other.y, self.z && other.z, self.w && other.w)
    }

    #[inline]
    pub fn or(self, other: BoolVec4) -> BoolVec4 {
        bvec4(self.x || other.x, self.y || other.y, self.z || other.z, self.w || other.w)
    }

    #[inline]
    pub fn tuple(&self) -> (bool, bool, bool, bool) { (self.x, self.y, self.z, self.w) }

    #[inline]
    pub fn array(&self) -> [bool; 4] { [self.x, self.y, self.z, self.w] }
}
