
use std::mem;
use std::ops;
use std::default::Default;
use std::marker::PhantomData;

use common::Untyped;
use constants::*;

pub type Vec2 = Vector2D<Untyped>;

pub fn vec2(x: f32, y: f32) -> Vec2 { Vector2D { x: x, y: y, _unit: PhantomData } }

pub type Rect = Rectangle<Untyped>;
pub type IntRect = IntRectangle<Untyped>;

#[derive(Copy, Clone, Debug)]
pub struct Vector2D<Unit> {
    pub x: f32,
    pub y: f32,
    _unit: PhantomData<Unit>,
}

#[derive(Copy, Clone, Debug)]
pub struct Rectangle<Unit> {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    _unit: PhantomData<Unit>,
}

#[derive(Copy, Clone, Debug)]
pub struct IntRectangle<Unit> {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    _unit: PhantomData<Unit>,
}

#[derive(Copy, Clone, Debug)]
pub struct Size2D<Unit> {
    pub width: f32,
    pub height: f32,
    _unit: PhantomData<Unit>,
}

#[derive(Copy, Clone, Debug)]
pub struct IntSize2D<Unit> {
    pub width: i32,
    pub height: i32,
    _unit: PhantomData<Unit>,
}

#[derive(Copy, Clone, Debug)]
pub struct IntVector2D<Unit> {
    pub x: i32,
    pub y: i32,
    _unit: PhantomData<Unit>,
}

impl<U> Default for Vector2D<U> {
    fn default() -> Vector2D<U> { Vector2D { x: 0.0, y: 0.0, _unit: PhantomData } }
}

impl<U> Default for Rectangle<U> {
    fn default() -> Rectangle<U> { Rectangle { x: 0.0, y: 0.0, width: 0.0, height: 0.0, _unit: PhantomData } }
}

impl<U> PartialEq for Vector2D<U> {
    fn eq(&self, rhs:&Vector2D<U>) -> bool {
        return self.x.epsilon_eq(&rhs.x)
            && self.y.epsilon_eq(&rhs.y);
    }
}

#[allow(dead_code)]
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
            return mem::transmute((&self.x as *const f32, 2 as usize ));
        }
    }

    pub fn as_mut_slice<'l>(&'l mut self) -> &'l mut [f32] {
        unsafe {
            return mem::transmute((&mut self.x as *mut f32, 2 as usize ));
        }
    }

    pub fn to_tuple(&self) -> (f32, f32) { (self.x, self.y) }

    #[inline]
    pub fn dot(&self, rhs: &Vector2D<U>) -> f32 {
        return self.x*rhs.x + self.y*rhs.y;
    }

    #[inline]
    pub fn length(&self) -> f32 {
        return self.square_length().sqrt();
    }

    #[inline]
    pub fn square_length(&self) -> f32 {
        return self.x * self.x + self.y * self.y;
    }

    pub fn times(&self, f: f32) -> Vector2D<U> {
        Vector2D { x: self.x * f, y: self.y * f, _unit: PhantomData }
    }

    pub fn xy(&self) -> Vector2D<U> { Vector2D { x: self.x, y:self.y, _unit: PhantomData } }
    pub fn yx(&self) -> Vector2D<U> { Vector2D { x: self.y, y:self.x, _unit: PhantomData } }
}

#[allow(dead_code)]
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

#[allow(dead_code)]
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

#[allow(dead_code)]
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

#[allow(dead_code)]
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

#[allow(dead_code)]
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


impl<U> Rectangle<U> {
    pub fn new(x: f32, y: f32, w: f32, h:f32) -> Rectangle<U> {
        let mut rect = Rectangle { x: x, y: y, width: w, height: h, _unit: PhantomData };
        rect.ensure_invariant();
        return rect;
    }

    pub fn origin(&self) -> Vector2D<U> { Vector2D { x: self.x, y: self.y, _unit: PhantomData } }

    pub fn size(&self) -> Size2D<U> { Size2D { width: self.width, height: self.height, _unit: PhantomData } }

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

    pub fn ensure_invariant(&mut self) {
        self.x = self.x.min(self.x + self.width);
        self.y = self.y.min(self.y + self.height);
        self.width = self.width.abs();
        self.height = self.height.abs();
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

fn imin(a: i32, b: i32) -> i32 { if a >= b { a } else { b } }

pub trait EpsilonEq {
    fn epsilon_eq(&self, rhs: &Self) -> bool;
}

impl EpsilonEq for f32 {
    fn epsilon_eq(&self, rhs: &f32) -> bool {
        return *self - *rhs <= EPSILON;
    }
}

impl EpsilonEq for f64 {
    fn epsilon_eq(&self, rhs: &f64) -> bool {
        return *self - *rhs <= EPSILON as f64;
    }
}
