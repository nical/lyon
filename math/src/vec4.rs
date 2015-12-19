
use std::mem;
use std::ops;
use std::default::Default;
use std::marker::PhantomData;

use common::Untyped;
use constants::*;

use super::vec3::Vector3D;
use super::vec2::Vector2D;

pub type Vec4 = Vector4D<Untyped>;

pub fn vec4(x: f32, y: f32, z: f32, w: f32) -> Vec4 { Vector4D { x: x, y: y, z: z, w: w, _unit: PhantomData } }

#[derive(Copy, Clone, Debug)]
pub struct Vector4D<Unit> {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
    _unit: PhantomData<Unit>,
}

impl<U> Default for Vector4D<U> {
    fn default() -> Vector4D<U> { Vector4D { x: 0.0, y: 0.0, z: 0.0, w: 0.0, _unit: PhantomData } }
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

    pub fn tuple(&self) -> (f32, f32, f32, f32) { (self.x, self.y, self.z, self.w) }

    pub fn array(&self) -> [f32; 4] { [self.x, self.y, self.z, self.w] }

    pub fn xy(&self) -> Vector2D<U> { Vector2D::new(self.x, self.y) }
    pub fn xz(&self) -> Vector2D<U> { Vector2D::new(self.x, self.z) }
    pub fn yz(&self) -> Vector2D<U> { Vector2D::new(self.y, self.z) }
    pub fn yx(&self) -> Vector2D<U> { Vector2D::new(self.y, self.x) }
    pub fn xyz(&self) -> Vector3D<U> { Vector3D::new(self.x, self.y, self.z) }
    pub fn zxy(&self) -> Vector3D<U> { Vector3D::new(self.z, self.x, self.y) }
    pub fn yzx(&self) -> Vector3D<U> { Vector3D::new(self.y, self.z, self.x) }
    pub fn xzy(&self) -> Vector3D<U> { Vector3D::new(self.x, self.z, self.y) }
    pub fn yxz(&self) -> Vector3D<U> { Vector3D::new(self.y, self.x, self.z) }
    pub fn wxyz(&self) -> Vector4D<U> { Vector4D::new(self.w, self.x, self.y, self.z) }
}

impl<U> PartialEq for Vector4D<U> {
    fn eq(&self, rhs:&Vector4D<U>) -> bool {
        return self.x.epsilon_eq(&rhs.x)
            && self.y.epsilon_eq(&rhs.y)
            && self.z.epsilon_eq(&rhs.z)
            && self.w.epsilon_eq(&rhs.w);
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

#[test]
fn test_vec4() {
    use matrix::Mat4;
    use vec3::vec3;

    let p1 = vec4(1.0, 2.0, 3.0, 0.0);
    let p2 = -p1;
    let p3 = p1 + p2;
    assert!(p3.length() < 0.001);
    let d = p1.dot(&p2);
    assert!(d < -0.0);
    let m1 = Mat4::identity();
    let p5 = m1.transform(&p1);
    let mut m1 = Mat4::identity();
    m1.rotate(PI, &vec3(1.0, 0.0, 0.0));
    let p6 = m1.transform(&p1);
    assert_eq!(p1, p5);
    assert_eq!(p6, vec4(1.0, -2.0, -3.0, 0.0));
}
