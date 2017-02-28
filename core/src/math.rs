use euclid;
use fixed;

pub use euclid::TypedPoint2D;
pub use euclid::Point2D;
pub use euclid::Radians;

pub type Point = euclid::Point2D<f32>;
pub type IntPoint = euclid::Point2D<i32>;
pub type Int64Point = euclid::Point2D<i64>;
pub type F64Point = euclid::Point2D<f64>;
// Point and Vec2 are the same type but they should probably be separate types.
pub type Vec2 = euclid::Point2D<f32>;
pub type IntVec2 = euclid::Point2D<i32>;
pub type Size = euclid::Size2D<f32>;
pub type IntSize = euclid::Size2D<i32>;
pub type Rect = euclid::Rect<f32>;
pub type IntRect = euclid::Rect<i32>;

pub type FixedPoint32 = fixed::Fp32<fixed::_16>;
pub type FixedPoint64 = fixed::Fp64<fixed::_16>;
pub type TessVec2 = Point2D<FixedPoint32>;
pub type TessPoint = Point2D<FixedPoint32>;
pub type TessPoint64 = Point2D<FixedPoint64>;
#[inline]
pub fn fixed(val: f32) -> FixedPoint32 { FixedPoint32::from_f32(val) }

pub type Vec3 = euclid::Point3D<f32>;
pub type IntVec3 = euclid::Point3D<i32>;
pub type IntVec4 = euclid::Point4D<i32>;

pub type Mat4 = euclid::Matrix4D<f32>;
pub type Transform2d = euclid::Matrix2D<f32>;

#[inline]
pub fn point(x: f32, y: f32) -> Point { vec2(x, y) }
#[inline]
pub fn vec2(x: f32, y: f32) -> Vec2 { Vec2::new(x, y) }
#[inline]
pub fn int_vec2(x: i32, y: i32) -> IntVec2 { IntVec2::new(x, y) }
#[inline]
pub fn int_vec4(x: i32, y: i32, z: i32, w: i32) -> IntVec4 { IntVec4::new(x, y, z, w) }
#[inline]
pub fn size(w: f32, h: f32) -> Size { Size::new(w, h) }
#[inline]
pub fn int_size(w: i32, h: i32) -> IntSize { IntSize::new(w, h) }
#[inline]
pub fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect { Rect::new(vec2(x, y), size(w, h)) }
#[inline]
pub fn int_rect(x: i32, y: i32, w: i32, h: i32) -> IntRect { IntRect::new(int_vec2(x, y), int_size(w, h)) }

#[inline]
pub fn rad(val: f32) -> Radians<f32> { Radians::new(val) }

pub trait Vec2Array<S> {
    #[inline]
    fn array(self) -> [S; 2];
}

impl<S> Vec2Array<S> for euclid::Point2D<S> {
    #[inline]
    fn array(self) ->[S; 2] { [self.x, self.y] }
}

pub trait Vec2Length {
    fn length(self) -> f32;
    fn normalized(self) -> Self;
}

pub trait Vec2SquareLength {
    #[inline]
    fn square_length(self) -> f32;
}

impl Vec2Length for Vec2 {
    #[inline]
    fn length(self) -> f32 { self.square_length().sqrt() }

    #[inline]
    fn normalized(self) -> Self { self / self.length() }
}

impl Vec2SquareLength for Vec2 {
    #[inline]
    fn square_length(self) -> f32 { self.x*self.x + self.y*self.y }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BoolVec4 {
    pub x: bool,
    pub y: bool,
    pub z: bool,
    pub w: bool,
}

pub fn bvec4(x: bool, y: bool, z: bool, w: bool) -> BoolVec4 {
    BoolVec4 { x: x, y: y, z: z, w: w }
}

impl BoolVec4 {
    #[inline]
    pub fn new(x: bool, y: bool, z: bool, w: bool) -> BoolVec4 {
        bvec4(x, y, z, w)
    }

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
