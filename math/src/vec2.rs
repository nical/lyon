#![allow(dead_code)]


use std::mem::transmute;
use std::ops;
use std::default::Default;
use std::marker::PhantomData;
use std::convert::{ From, AsMut, AsRef };
use std::fmt;


use units::{ Unit, Untyped };
use constants::*;
use super::fuzzy_eq;

pub type Vec2 = Vector2D<Untyped>;

pub fn vec2(x: f32, y: f32) -> Vec2 { Vector2D::new(x, y) }

pub type Rect = Rectangle<Untyped>;
pub type IntRect = IntRectangle<Untyped>;

pub struct Vector2D<Unit = Untyped> {
    pub x: f32,
    pub y: f32,
    _unit: PhantomData<Unit>,
}

pub struct Rectangle<Unit = Untyped> {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    _unit: PhantomData<Unit>,
}

pub struct IntRectangle<Unit = Untyped> {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    _unit: PhantomData<Unit>,
}

pub struct Size2D<Unit = Untyped> {
    pub width: f32,
    pub height: f32,
    _unit: PhantomData<Unit>,
}

pub struct IntSize2D<Unit = Untyped> {
    pub width: i32,
    pub height: i32,
    _unit: PhantomData<Unit>,
}

pub struct IntVector2D<Unit = Untyped> {
    pub x: i32,
    pub y: i32,
    _unit: PhantomData<Unit>,
}

impl<U> AsRef<[f32; 2]> for Vector2D<U> {
    fn as_ref(&self) -> &[f32; 2] { unsafe { transmute(self) } }
}

impl<U> AsRef<(f32, f32)> for Vector2D<U> {
    fn as_ref(&self) -> &(f32, f32) { unsafe { transmute(self) } }
}

impl<U> AsMut<[f32; 2]> for Vector2D<U> {
    fn as_mut(&mut self) -> &mut [f32; 2] { unsafe { transmute(self) } }
}

impl<U> AsMut<(f32, f32)> for Vector2D<U> {
    fn as_mut(&mut self) -> &mut (f32, f32) { unsafe { transmute(self) } }
}

impl<U> AsRef<Vector2D<U>> for [f32; 2] {
    fn as_ref(&self) -> &Vector2D<U> { unsafe { transmute(self) } }
}

impl<U> AsRef<Vector2D<U>> for (f32, f32) {
    fn as_ref(&self) -> &Vector2D<U> { unsafe { transmute(self) } }
}

impl<U> AsMut<Vector2D<U>> for [f32; 2] {
    fn as_mut(&mut self) -> &mut Vector2D<U> { unsafe { transmute(self) } }
}

impl<U> AsMut<Vector2D<U>> for (f32, f32) {
    fn as_mut(&mut self) -> &mut Vector2D<U> { unsafe { transmute(self) } }
}

impl<U> From<[f32; 2]> for Vector2D<U> {
    fn from(v: [f32; 2]) -> Vector2D<U> { Vector2D::new(v[0], v[1]) }
}

impl<U> From<(f32, f32)> for Vector2D<U> {
    fn from(v: (f32, f32)) -> Vector2D<U> { let (x, y) = v; Vector2D::new(x, y) }
}

impl<U> From<Vector2D<U>> for [f32; 2]  {
    fn from(v: Vector2D<U>) -> [f32; 2] { [v.x, v.y] }
}

impl<U> From<Vector2D<U>> for (f32, f32)  {
    fn from(v: Vector2D<U>) -> (f32, f32) { (v.x, v.y) }
}


impl<U> Default for Vector2D<U> {
    fn default() -> Vector2D<U> { Vector2D::new(0.0, 0.0) }
}

impl<U> Default for Rectangle<U> {
    fn default() -> Rectangle<U> { Rectangle::new(0.0, 0.0, 0.0, 0.0) }
}

impl<U> Vector2D<U> {
    pub fn new(x: f32, y: f32) -> Vector2D<U> {
        Vector2D {
            x: x,
            y: y,
            _unit: PhantomData
        }
    }

    pub fn from_slice(from: &[f32]) -> Vector2D<U> {
        assert!(from.len() >= 2);
        return Vector2D {
            x: from[0],
            y: from[1],
            _unit: PhantomData
        };
    }

    pub fn as_slice<'l>(&'l self) -> &'l [f32] {
        unsafe {
            return transmute((&self.x as *const f32, 2 as usize ));
        }
    }

    pub fn as_mut_slice<'l>(&'l mut self) -> &'l mut [f32] {
        unsafe {
            return transmute((&mut self.x as *mut f32, 2 as usize ));
        }
    }

    #[inline]
    pub fn array(&self) -> [f32; 2] { [self.x, self.y] }

    #[inline]
    pub fn tuple(&self) -> (f32, f32) { (self.x, self.y) }

    #[inline]
    pub fn from_array(array: [f32; 2]) -> Vector2D<U>{ Vector2D::new(array[0], array[1]) }

    #[inline]
    pub fn from_tuple(tuple: (f32, f32)) -> Vector2D<U> {
        let (x, y) = tuple;
        Vector2D::new(x, y)
    }

    #[inline]
    pub fn dot(&self, rhs: &Vector2D<U>) -> f32 {
        return self.x*rhs.x + self.y*rhs.y;
    }

    #[inline]
    pub fn cross(&self, rhs: Vector2D<U>) -> f32 {
        self.x * rhs.y - self.y * rhs.x
    }

    #[inline]
    pub fn length(&self) -> f32 {
        return self.square_length().sqrt();
    }

    #[inline]
    pub fn square_length(&self) -> f32 {
        return self.x * self.x + self.y * self.y;
    }

    pub fn normalized(&self) -> Vector2D<U> { *self / self.length() }

    pub fn xy(&self) -> Vector2D<U> { Vector2D { x: self.x, y:self.y, _unit: PhantomData } }
    pub fn yx(&self) -> Vector2D<U> { Vector2D { x: self.y, y:self.x, _unit: PhantomData } }

    /// Angle between vectors v1 and v2 (oriented clockwise assyming y points downwards).
    /// The result is a number between 0 and 2*PI.
    ///
    /// ex: directed_angle([0,1], [1,0]) = 3/2 Pi rad
    ///     x       __
    ///   0-->     /  \
    ///  y|       |  x--> v2
    ///   v        \ |v1
    ///              v
    ///
    /// Or, assuming y points upwards:
    /// directed_angle([0,-1], [1,0]) = 1/2 Pi rad
    ///
    ///   ^           v2
    ///  y|          x-->
    ///   0-->    v1 | /
    ///     x        v-
    ///
    pub fn directed_angle(self, other: Vector2D<U>) -> f32 {
        let a = (other.y).atan2(other.x) - (self.y).atan2(self.x);
        return if a < 0.0 { a + 2.0 * PI } else { a };
    }

    pub fn fuzzy_eq(self, rhs: Vector2D<U>) -> bool {
        return fuzzy_eq(self.x, rhs.x) && fuzzy_eq(self.y, rhs.y);
    }
}

impl<U> ops::Add<Vector2D<U>> for Vector2D<U> {

    type Output = Vector2D<U>;

    #[inline]
    fn add(self, rhs: Vector2D<U>) -> Vector2D<U> {
        return Vector2D {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            _unit: PhantomData
        };
    }
}

impl<U> ops::Sub<Vector2D<U>> for Vector2D<U> {

    type Output = Vector2D<U>;

    #[inline]
    fn sub(self, rhs: Vector2D<U>) -> Vector2D<U> {
        return Vector2D {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            _unit: PhantomData
        };
    }
}

impl<U> ops::Mul<Vector2D<U>> for Vector2D<U> {

    type Output = Vector2D<U>;

    #[inline]
    fn mul(self, rhs: Vector2D<U>) -> Vector2D<U> {
        return Vector2D {
            x: self.x * rhs.x,
            y: self.y * rhs.y,
            _unit: PhantomData
        };
    }
}

impl<U> ops::Mul<f32> for Vector2D<U> {

    type Output = Vector2D<U>;

    #[inline]
    fn mul(self, rhs: f32) -> Vector2D<U> {
        return Vector2D::new(self.x * rhs, self.y * rhs);
    }
}

impl<U> ops::Div<Vector2D<U>> for Vector2D<U> {

    type Output = Vector2D<U>;

    #[inline]
    fn div(self, rhs: Vector2D<U>) -> Vector2D<U> {
        return Vector2D {
            x: self.x / rhs.x,
            y: self.y / rhs.y,
            _unit: PhantomData
        };
    }
}

impl<U> ops::Div<f32> for Vector2D<U> {

    type Output = Vector2D<U>;

    #[inline]
    fn div(self, rhs: f32) -> Vector2D<U> {
        return Vector2D::new(self.x / rhs, self.y / rhs);
    }
}

impl<U> ops::Neg for Vector2D<U> {

    type Output = Vector2D<U>;

    #[inline]
    fn neg(self) -> Vector2D<U> {
        return Vector2D {
            x: -self.x,
            y: -self.y,
            _unit: PhantomData
        };
    }
}

impl<U: Unit> fmt::Debug for Vector2D<U> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Vector2D<{}>[{}, {}]", U::name(), self.x, self.y)
    }
}

impl<U> Copy for Vector2D<U> {}

impl<U> Clone for Vector2D<U> {
    fn clone(&self) -> Vector2D<U> { *self }
}

impl<U> PartialEq for Vector2D<U> {
    fn eq(&self, other: &Vector2D<U>) -> bool { self.x == other.x && self.y == other.y }
}

impl<U> Rectangle<U> {
    pub fn new(x: f32, y: f32, w: f32, h:f32) -> Rectangle<U> {
        let mut rect = Rectangle { x: x, y: y, width: w, height: h, _unit: PhantomData };
        rect.ensure_invariant();
        return rect;
    }

    pub fn origin(&self) -> Vector2D<U> { Vector2D { x: self.x, y: self.y, _unit: PhantomData } }

    pub fn size(&self) -> Size2D<U> { Size2D::new(self.width, self.height) }

    pub fn move_by(&mut self, v: Vector2D<U>) {
        self.x = self.x + v.x;
        self.y = self.y + v.y;
    }

    pub fn scale_by(&mut self, v: f32) {
        self.x = self.x * v;
        self.y = self.y * v;
        self.width = self.width * v;
        self.height = self.height * v;
        self.ensure_invariant();
    }

    pub fn top_left(&self) -> Vector2D<U> {
        Vector2D {
            x: self.x,
            y: self.y,
            _unit: PhantomData
        }
    }
    pub fn top_right(&self) -> Vector2D<U> {
        Vector2D {
            x: self.x + self.width,
            y: self.y,
            _unit: PhantomData
        }
    }

    pub fn bottom_right(&self) -> Vector2D<U> {
        Vector2D {
            x: self.x + self.width,
            y: self.y + self.height,
            _unit: PhantomData
        }
    }

    pub fn bottom_left(&self) -> Vector2D<U> {
        Vector2D {
            x: self.x,
            y: self.y + self.height,
            _unit: PhantomData
        }
    }

    pub fn x_most(&self) -> f32 { self.x + self.width }

    pub fn y_most(&self) -> f32 { self.y + self.height }

    pub fn contains(&self, other: &Rectangle<U>) -> bool {
        return self.x <= other.x &&
               self.y <= self.y &&
               self.x_most() >= other.x_most() &&
               self.y_most() >= other.y_most();
    }

    pub fn intersects(&self, other: &Rectangle<U>) -> bool {
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

    pub fn ensure_invariant(&mut self) {
        self.x = self.x.min(self.x + self.width);
        self.y = self.y.min(self.y + self.height);
        self.width = self.width.abs();
        self.height = self.height.abs();
    }
}

impl<U: Unit> fmt::Debug for Rectangle<U> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Rectangle<{}>[x:{}, y:{} w:{} h:{}]", U::name(), self.x, self.y, self.width, self.height)
    }
}

impl<U> Copy for Rectangle<U> {}

impl<U> Clone for Rectangle<U> {
    fn clone(&self) -> Rectangle<U> { *self }
}

impl<U> PartialEq for Rectangle<U> {
    fn eq(&self, other: &Rectangle<U>) -> bool {
        self.x == other.x && self.y == other.y && self.width == other.width && self.height == other.height
    }
}

impl<U> IntRectangle<U> {
    pub fn new(x: i32, y: i32, w: i32, h:i32) -> IntRectangle<U> {
        let mut rect = IntRectangle { x: x, y: y, width: w, height: h, _unit: PhantomData };
        rect.ensure_invariant();
        return rect;
    }

    pub fn origin(&self) -> IntVector2D<U> { IntVector2D { x: self.x, y: self.y, _unit: PhantomData } }

    pub fn size(&self) -> IntSize2D<U> { IntSize2D { width: self.width, height: self.height, _unit: PhantomData } }

    pub fn move_by(&mut self, v: IntVector2D<U>) {
        self.x = self.x + v.x;
        self.y = self.y + v.y;
    }

    pub fn top_left(&self) -> IntVector2D<U> {
        IntVector2D {
            x: self.x,
            y: self.y,
            _unit: PhantomData
        }
    }
    pub fn top_right(&self) -> IntVector2D<U> {
        IntVector2D {
            x: self.x + self.width,
            y: self.y,
            _unit: PhantomData
        }
    }

    pub fn bottom_right(&self) -> IntVector2D<U> {
        IntVector2D {
            x: self.x + self.width,
            y: self.y + self.height,
            _unit: PhantomData
        }
    }

    pub fn bottom_left(&self) -> IntVector2D<U> {
        IntVector2D {
            x: self.x,
            y: self.y + self.height,
            _unit: PhantomData
        }
    }

    pub fn x_most(&self) -> i32 { self.x + self.width }

    pub fn y_most(&self) -> i32 { self.y + self.height }

    pub fn contains(&self, other: &IntRectangle<U>) -> bool {
        return self.x <= other.x &&
               self.y <= self.y &&
               self.x_most() >= other.x_most() &&
               self.y_most() >= other.y_most();
    }

    pub fn ensure_invariant(&mut self) {
        self.x = imin(self.x, self.x + self.width);
        self.y = imin(self.y, self.y + self.height);
        self.width = self.width.abs();
        self.height = self.height.abs();
    }
}

impl<U: Unit> fmt::Debug for IntRectangle<U> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "IntRectangle<{}>[x:{}, y:{} w:{} h:{}]", U::name(), self.x, self.y, self.width, self.height)
    }
}

impl<U> Copy for IntRectangle<U> {}

impl<U> Clone for IntRectangle<U> {
    fn clone(&self) -> IntRectangle<U> { *self }
}

impl<U> PartialEq for IntRectangle<U> {
    fn eq(&self, other: &IntRectangle<U>) -> bool {
        self.x == other.x && self.y == other.y && self.width == other.width && self.height == other.height
    }
}

fn imin(a: i32, b: i32) -> i32 { if a >= b { a } else { b } }

impl<U> Size2D<U> {
    pub fn new(w: f32, h: f32) -> Size2D<U> { Size2D { width: w, height: h, _unit: PhantomData } }
}

impl<U: Unit> fmt::Debug for Size2D<U> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Size2D<{}>[{}, {}]", U::name(), self.width, self.height)
    }
}

impl<U> Copy for Size2D<U> {}

impl<U> Clone for Size2D<U> {
    fn clone(&self) -> Size2D<U> { *self }
}

impl<U> PartialEq for Size2D<U> {
    fn eq(&self, other: &Size2D<U>) -> bool {
        self.width == other.width && self.height == other.height
    }
}

impl<U> IntSize2D<U> {
    pub fn new(w: i32, h: i32) -> IntSize2D<U> { IntSize2D { width: w, height: h, _unit: PhantomData } }
}

impl<U: Unit> fmt::Debug for IntSize2D<U> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "IntSize2D<{}>[{}, {}]", U::name(), self.width, self.height)
    }
}

impl<U> Copy for IntSize2D<U> {}

impl<U> Clone for IntSize2D<U> {
    fn clone(&self) -> IntSize2D<U> { *self }
}

impl<U: Unit> fmt::Debug for IntVector2D<U> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "IntVector2D<{}>[{}, {}]", U::name(), self.x, self.y)
    }
}

impl<U> Copy for IntVector2D<U> {}

impl<U> Clone for IntVector2D<U> {
    fn clone(&self) -> IntVector2D<U> { *self }
}

impl<U> PartialEq for IntVector2D<U> {
    fn eq(&self, other: &IntVector2D<U>) -> bool {
        self.x == other.x && self.y == other.y
    }
}

pub fn test_directed_angle() {
    assert!(fuzzy_eq(vec2(1.0, 1.0).directed_angle(vec2(1.0, 1.0)), 0.0));
    assert!(fuzzy_eq(vec2(1.0, 0.0).directed_angle(vec2(0.0, 1.0)), PI * 0.5));
    assert!(fuzzy_eq(vec2(1.0, 0.0).directed_angle(vec2(-1.0, 0.0)), PI));
    assert!(fuzzy_eq(vec2(1.0, 0.0).directed_angle(vec2(0.0, -1.0)), PI * 1.5));
    assert!(fuzzy_eq(vec2(1.0, -1.0).directed_angle(vec2(1.0, 0.0)), PI * 0.25));
    assert!(fuzzy_eq(vec2(1.0, -1.0).directed_angle(vec2(1.0, 1.0)), PI * 0.5));
    assert!(fuzzy_eq(vec2(1.0, -1.0).directed_angle(vec2(-1.0, 1.0)), PI));
    assert!(fuzzy_eq(vec2(1.0, -1.0).directed_angle(vec2(-1.0, -1.0)), PI * 1.5));
    assert!(fuzzy_eq(vec2(10.0, -10.0).directed_angle(vec2(3.0, 0.0)), PI * 0.25));
    assert!(fuzzy_eq(vec2(10.0, -10.0).directed_angle(vec2(3.0, 3.0)), PI * 0.5));
    assert!(fuzzy_eq(vec2(10.0, -10.0).directed_angle(vec2(-3.0, 3.0)), PI));
    assert!(fuzzy_eq(vec2(10.0, -10.0).directed_angle(vec2(-3.0, -3.0)), PI * 1.5));
    assert!(fuzzy_eq(vec2(-1.0, 0.0).directed_angle(vec2(1.0, 0.0)), PI));
    assert!(fuzzy_eq(vec2(-1.0, 0.0).directed_angle(vec2(0.0, 1.0)), PI * 1.5));
    assert!(fuzzy_eq(vec2(-1.0, 0.0).directed_angle(vec2(0.0, -1.0)), PI * 0.5));
}

pub fn array_to_vec2_slice<U>(slice: &[[f32; 2]]) -> &[Vector2D<U>] { unsafe { transmute(slice) } }

pub fn vec2_to_array_slice<U>(slice: &[Vector2D<U>]) -> &[[f32; 2]] { unsafe { transmute(slice) } }

pub fn tuple_to_vec2_slice<U>(slice: &[(f32, f32)]) -> &[Vector2D<U>] { unsafe { transmute(slice) } }

pub fn vec2_to_tuple_slice<U>(slice: &[Vector2D<U>]) -> &[(f32, f32)] { unsafe { transmute(slice) } }

// The orphan rule forbids this:
//
//impl<'l, U: Unit> From<&'l [[f32; 2]]> for &'l [Vector2D<U>] {
//    fn from(slice: &'l[[f32; 2]]) -> &'l[Vector2D<U>] { array_to_vec2_slice(slice) }
//}
//
//impl<'l, U: Unit> From<&'l [Vector2D<U>]> for &'l [[f32; 2]] {
//    fn from(slice: &'l[Vector2D<U>]) -> &'l[[f32; 2]] { vec2_to_array_slice(slice) }
//}
