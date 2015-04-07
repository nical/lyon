
use std::mem;
use std::ops;
use std::default::Default;
use std::marker::PhantomData;

use common::Untyped;
use constants::*;

pub type Vec2 = Vector2D<Untyped>;
pub type Vec3 = Vector3D<Untyped>;
pub type Vec4 = Vector4D<Untyped>;

pub fn vec2(x: f32, y: f32) -> Vec2 { Vector2D { x: x, y: y, _unit: PhantomData } }
pub fn vec3(x: f32, y: f32, z: f32) -> Vec3 { Vector3D { x: x, y: y, z: z, _unit: PhantomData } }
pub fn vec4(x: f32, y: f32, z: f32, w: f32) -> Vec4 { Vector4D { x: x, y: y, z: z, w: w, _unit: PhantomData } }

pub type Rect = Rectangle<Untyped>;

pub trait ScalarMul<T> {
    fn scalar_mul(&self, scalar: T) -> Self;
    fn scalar_mul_in_place(&mut self, scalar: T);
}

#[derive(Copy, Clone, Debug)]
pub struct Vector2D<Unit> {
    pub x: f32,
    pub y: f32,
    _unit: PhantomData<Unit>,
}

#[derive(Copy, Clone, Debug)]
pub struct Vector3D<Unit> {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    _unit: PhantomData<Unit>,
}

#[derive(Copy, Clone, Debug)]
pub struct Vector4D<Unit> {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
    _unit: PhantomData<Unit>,
}

#[derive(Copy, Clone, Debug)]
pub struct Rectangle<Unit> {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    _unit: PhantomData<Unit>,
}

impl<U> Default for Vector2D<U> {
    fn default() -> Vector2D<U> { Vector2D { x: 0.0, y: 0.0, _unit: PhantomData } }
}

impl<U> Default for Vector3D<U> {
    fn default() -> Vector3D<U> { Vector3D { x: 0.0, y: 0.0, z: 0.0, _unit: PhantomData } }
}

impl<U> Default for Vector4D<U> {
    fn default() -> Vector4D<U> { Vector4D { x: 0.0, y: 0.0, z: 0.0, w: 0.0, _unit: PhantomData } }
}

impl<U> Default for Rectangle<U> {
    fn default() -> Rectangle<U> { Rectangle { x: 0.0, y: 0.0, w: 0.0, h: 0.0, _unit: PhantomData } }
}

#[allow(dead_code)]
impl<U> Vector4D<U> {
    pub fn new(x: f32, y: f32, z: f32, w: f32) -> Vector4D<U> {
        Vector4D {
            x: x,
            y: y,
            z: z,
            w: w,
            _unit: PhantomData
        }
    }

    pub fn from_slice(from: &[f32]) -> Vector4D<U> {
        assert!(from.len() >= 4);
        return Vector4D {
            x: from[0],
            y: from[1],
            z: from[2],
            w: from[3],
            _unit: PhantomData,
        };
    }

    pub fn as_slice<'l>(&'l self) -> &'l [f32] {
        unsafe {
            return mem::transmute((&self.x as *const f32, 4 as usize ));
        }
    }

    pub fn as_mut_slice<'l>(&'l mut self) -> &'l mut [f32] {
        unsafe {
            return mem::transmute((&mut self.x as *mut f32, 4 as usize ));
        }
    }

    #[inline]
    pub fn dot(&self, rhs: &Vector4D<U>) -> f32 {
        return self.x*rhs.x + self.y*rhs.y + self.z*rhs.z + self.w*rhs.w;
    }

    pub fn length(&self) -> f32 {
        return self.square_length().sqrt();
    }

    pub fn square_length(&self) -> f32 {
        return self.x * self.x + self.y * self.y + self.z * self.z + self.w * self.w;
    }

    pub fn to_tuple(&self) -> (f32, f32, f32, f32) { (self.x, self.y, self.z, self.w) }

    pub fn xy(&self) -> Vector2D<U> { Vector2D { x: self.x, y:self.y, _unit: PhantomData } }
    pub fn xz(&self) -> Vector2D<U> { Vector2D { x: self.x, y:self.z, _unit: PhantomData } }
    pub fn yz(&self) -> Vector2D<U> { Vector2D { x: self.y, y:self.z, _unit: PhantomData } }
    pub fn yx(&self) -> Vector2D<U> { Vector2D { x: self.y, y:self.x, _unit: PhantomData } }
    pub fn xyz(&self) -> Vector3D<U> { Vector3D { x: self.x, y:self.y, z: self.z, _unit: PhantomData } }
    pub fn zxy(&self) -> Vector3D<U> { Vector3D { x: self.z, y:self.x, z: self.y, _unit: PhantomData } }
    pub fn yzx(&self) -> Vector3D<U> { Vector3D { x: self.y, y:self.z, z: self.x, _unit: PhantomData } }
    pub fn xzy(&self) -> Vector3D<U> { Vector3D { x: self.x, y:self.z, z: self.y, _unit: PhantomData } }
    pub fn yxz(&self) -> Vector3D<U> { Vector3D { x: self.y, y:self.x, z: self.z, _unit: PhantomData } }
    pub fn wxyz(&self) -> Vector4D<U> { Vector4D { x: self.w, y:self.x, z: self.y, w:self.z, _unit: PhantomData } }
}

impl<U> PartialEq for Vector4D<U> {
    fn eq(&self, rhs:&Vector4D<U>) -> bool {
        return self.x.epsilon_eq(&rhs.x)
            && self.y.epsilon_eq(&rhs.y)
            && self.z.epsilon_eq(&rhs.z)
            && self.w.epsilon_eq(&rhs.w);
    }
}

impl<U> PartialEq for Vector3D<U> {
    fn eq(&self, rhs:&Vector3D<U>) -> bool {
        return self.x.epsilon_eq(&rhs.x)
            && self.y.epsilon_eq(&rhs.y)
            && self.z.epsilon_eq(&rhs.z);
    }
}

impl<U> PartialEq for Vector2D<U> {
    fn eq(&self, rhs:&Vector2D<U>) -> bool {
        return self.x.epsilon_eq(&rhs.x)
            && self.y.epsilon_eq(&rhs.y);
    }
}


#[allow(dead_code)]
impl<U> ops::Add<Vector4D<U>> for Vector4D<U> {

    type Output = Vector4D<U>;

    #[inline]
    fn add(self, rhs: Vector4D<U>) -> Vector4D<U> {
        return Vector4D {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
            w: self.w + rhs.w,
            _unit: PhantomData
        };
    }
}

#[allow(dead_code)]
impl<U> ops::Sub<Vector4D<U>> for Vector4D<U> {

    type Output = Vector4D<U>;

    #[inline]
    fn sub(self, rhs: Vector4D<U>) -> Vector4D<U> {
        return Vector4D {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            z: self.z - rhs.z,
            w: self.w - rhs.w,
            _unit: PhantomData
        };
    }
}

#[allow(dead_code)]
impl<U> ops::Mul<Vector4D<U>> for Vector4D<U> {

    type Output = Vector4D<U>;

    #[inline]
    fn mul(self, rhs: Vector4D<U>) -> Vector4D<U> {
        return Vector4D {
            x: self.x * rhs.x,
            y: self.y * rhs.y,
            z: self.z * rhs.z,
            w: self.w * rhs.w,
            _unit: PhantomData
        };
    }
}

#[allow(dead_code)]
impl<U> ScalarMul<f32> for Vector4D<U> {

    #[inline]
    fn scalar_mul(&self, rhs: f32) -> Vector4D<U> {
        return Vector4D {
            x: self.x * rhs,
            y: self.y * rhs,
            z: self.z * rhs,
            w: self.w * rhs,
            _unit: PhantomData
        };
    }

    #[inline]
    fn scalar_mul_in_place(&mut self, rhs: f32) {
        self.x = self.x * rhs;
        self.y = self.y * rhs;
        self.z = self.z * rhs;
        self.w = self.w * rhs;
    }
}

#[allow(dead_code)]
impl<U> ops::Div<Vector4D<U>> for Vector4D<U> {

    type Output = Vector4D<U>;

    #[inline]
    fn div(self, rhs: Vector4D<U>) -> Vector4D<U> {
        return Vector4D {
            x: self.x / rhs.x,
            y: self.y / rhs.y,
            z: self.z / rhs.z,
            w: self.w / rhs.w,
            _unit: PhantomData
        };
    }
}

#[allow(dead_code)]
impl<U> ops::Neg for Vector4D<U> {

    type Output = Vector4D<U>;

    #[inline]
    fn neg(self) -> Vector4D<U> {
        return Vector4D {
            x: -self.x,
            y: -self.y,
            z: -self.z,
            w: -self.w,
            _unit: PhantomData
        };
    }
}

#[allow(dead_code)]
impl<U> Vector3D<U> {
    pub fn new(x: f32, y: f32, z: f32) -> Vector3D<U> {
        Vector3D {
            x: x,
            y: y,
            z: z,
            _unit: PhantomData
        }
    }

    pub fn from_slice(from: &[f32]) -> Vector3D<U> {
        assert!(from.len() >= 3);
        return Vector3D {
            x: from[0],
            y: from[1],
            z: from[2],
            _unit: PhantomData
        };
    }

    pub fn as_slice<'l>(&'l self) -> &'l [f32] {
        unsafe {
            return mem::transmute((&self.x as *const f32, 3 as usize ));
        }
    }

    pub fn as_mut_slice<'l>(&'l mut self) -> &'l mut [f32] {
        unsafe {
            return mem::transmute((&mut self.x as *mut f32, 3 as usize ));
        }
    }

    #[inline]
    pub fn dot(&self, rhs: &Vector3D<U>) -> f32 {
        return self.x*rhs.x + self.y*rhs.y + self.z*rhs.z;
    }

    #[inline]
    pub fn cross(&self, rhs: &Vector3D<U>) -> Vector3D<U> {
        return Vector3D {
            x: (self.y * rhs.z) - (self.z * rhs.y),
            y: (self.z * rhs.x) - (self.x * rhs.z),
            z: (self.x * rhs.y) - (self.y * rhs.x),
            _unit: PhantomData
        }
    }

    pub fn length(&self) -> f32 {
        return self.square_length().sqrt();
    }

    pub fn square_length(&self) -> f32 {
        return self.x * self.x + self.y * self.y + self.z * self.z;
    }

    pub fn to_tuple(&self) -> (f32, f32, f32) { (self.x, self.y, self.z) }

    pub fn xy(&self) -> Vector2D<U> { Vector2D { x: self.x, y:self.y, _unit: PhantomData } }
    pub fn xz(&self) -> Vector2D<U> { Vector2D { x: self.x, y:self.z, _unit: PhantomData } }
    pub fn yz(&self) -> Vector2D<U> { Vector2D { x: self.y, y:self.z, _unit: PhantomData } }
    pub fn yx(&self) -> Vector2D<U> { Vector2D { x: self.y, y:self.x, _unit: PhantomData } }
    pub fn xyz(&self) -> Vector3D<U> { Vector3D { x: self.x, y:self.y, z: self.z, _unit: PhantomData } }
    pub fn zxy(&self) -> Vector3D<U> { Vector3D { x: self.z, y:self.x, z: self.y, _unit: PhantomData } }
    pub fn yzx(&self) -> Vector3D<U> { Vector3D { x: self.y, y:self.z, z: self.x, _unit: PhantomData } }
    pub fn xzy(&self) -> Vector3D<U> { Vector3D { x: self.x, y:self.z, z: self.y, _unit: PhantomData } }
    pub fn yxz(&self) -> Vector3D<U> { Vector3D { x: self.y, y:self.x, z: self.z, _unit: PhantomData } }

    pub fn to_vec4(&self, w: f32) -> Vector4D<U> {
        Vector4D {
            x: self.x,
            y: self.y,
            z: self.z,
            w: w,
            _unit: PhantomData
        }
    }
}

#[allow(dead_code)]
impl<U> ops::Add<Vector3D<U>> for Vector3D<U> {

    type Output = Vector3D<U>;

    #[inline]
    fn add(self, rhs: Vector3D<U>) -> Vector3D<U> {
        return Vector3D {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
            _unit: PhantomData
        };
    }
}

#[allow(dead_code)]
impl<U> ops::Sub<Vector3D<U>> for Vector3D<U> {

    type Output = Vector3D<U>;

    #[inline]
    fn sub(self, rhs: Vector3D<U>) -> Vector3D<U> {
        return Vector3D {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            z: self.z - rhs.z,
            _unit: PhantomData
        };
    }
}

#[allow(dead_code)]
impl<U> ops::Mul<Vector3D<U>> for Vector3D<U> {

    type Output = Vector3D<U>;

    #[inline]
    fn mul(self, rhs: Vector3D<U>) -> Vector3D<U> {
        return Vector3D {
            x: self.x * rhs.x,
            y: self.y * rhs.y,
            z: self.z * rhs.z,
            _unit: PhantomData
        };
    }
}

#[allow(dead_code)]
impl<U> ScalarMul<f32> for Vector3D<U> {

    #[inline]
    fn scalar_mul(&self, rhs: f32) -> Vector3D<U> {
        return Vector3D {
            x: self.x * rhs,
            y: self.y * rhs,
            z: self.z * rhs,
            _unit: PhantomData
        };
    }

    #[inline]
    fn scalar_mul_in_place(&mut self, rhs: f32) {
        self.x = self.x * rhs;
        self.y = self.y * rhs;
        self.z = self.z * rhs;
    }
}


#[allow(dead_code)]
impl<U> ops::Neg for Vector3D<U> {

    type Output = Vector3D<U>;

    #[inline]
    fn neg(self) -> Vector3D<U> {
        return Vector3D {
            x: -self.x,
            y: -self.y,
            z: -self.z,
            _unit: PhantomData
        };
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
impl<U> ScalarMul<f32> for Vector2D<U> {

    #[inline]
    fn scalar_mul(&self, rhs: f32) -> Vector2D<U> {
        return Vector2D {
            x: self.x * rhs,
            y: self.y * rhs,
            _unit: PhantomData
        };
    }

    #[inline]
    fn scalar_mul_in_place(&mut self, rhs: f32) {
        self.x = self.x * rhs;
        self.y = self.y * rhs;
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
        let mut rect = Rectangle { x: x, y: y, w: w, h: h, _unit: PhantomData };
        rect.ensure_invariant();
        return rect;
    }

    pub fn origin(&self) -> Vector2D<U> { Vector2D { x: self.x, y: self.y, _unit: PhantomData } }

    pub fn size(&self) -> Vector2D<U> { Vector2D { x: self.w, y: self.h, _unit: PhantomData } }

    pub fn move_by(&mut self, v: Vector2D<U>) {
        self.x = self.x + v.x;
        self.y = self.y + v.y;
    }

    pub fn scale_by(&mut self, v: f32) {
        self.x = self.x * v;
        self.y = self.y * v;
        self.w = self.w * v;
        self.h = self.h * v;
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
            x: self.x + self.w,
            y: self.y,
            _unit: PhantomData
        }
    }

    pub fn bottom_right(&self) -> Vector2D<U> {
        Vector2D {
            x: self.x + self.w,
            y: self.y + self.h,
            _unit: PhantomData
        }
    }

    pub fn bottom_left(&self) -> Vector2D<U> {
        Vector2D {
            x: self.x,
            y: self.y + self.h,
            _unit: PhantomData
        }
    }

    pub fn x_most(&self) -> f32 { self.x + self.w }

    pub fn y_most(&self) -> f32 { self.y + self.h }

    pub fn contains(&self, other: &Rectangle<U>) -> bool {
        return self.x <= other.x &&
               self.y <= self.y &&
               self.x_most() >= other.x_most() &&
               self.y_most() >= other.y_most();
    }

    pub fn ensure_invariant(&mut self) {
        self.x = self.x.min(self.x + self.w);
        self.y = self.y.min(self.y + self.h);
        self.w = self.w.abs();
        self.h = self.h.abs();
    }
}

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

#[cfg(test)]
use matrix::Mat4;

#[test]
fn test_vec4() {
    let p1 = vec4(1.0, 2.0, 3.0, 0.0);
    let p2 = -p1;
    let p3 = p1 + p2;
    let d = p1.dot(&p2);
    let m1 = Mat4::identity();
    let p5 = m1.transform(&p1);
    let mut m1 = Mat4::identity();
    m1.rotate(PI, &vec3(1.0, 0.0, 0.0));
    let p6 = m1.transform(&p1);
    assert_eq!(p1, p5);
    assert_eq!(p6, vec4(1.0, -2.0, -3.0, 0.0));
}
