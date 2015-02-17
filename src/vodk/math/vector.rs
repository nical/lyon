
// Math module providing simple vector and matrix types and operations.
// This module is meant to make strongly typed units easy.
//
// Ideally this could be replaced by cgmath-rs but there are a few issues
// around introducing strongly typed units that make it inconvenient
// (see cgmath_glue.rs).

// TODO:
// * support operations against Float32<U> instead of f32

use std::mem;
use std::ops;
use std::num::Float;
use std::default::Default;

pub static EPSILON: f32 = 0.000001;
pub static PI: f32 = 3.14159265359;

#[derive(Copy, PartialEq, Show)]
pub struct Untyped;

pub type Vec2 = Vector2D<Untyped>;
pub type Vec3 = Vector3D<Untyped>;
pub type Vec4 = Vector4D<Untyped>;

pub fn vec2(x: f32, y: f32) -> Vec2 { Vector2D { x: x, y: y } }
pub fn vec3(x: f32, y: f32, z: f32) -> Vec3 { Vector3D { x: x, y: y, z: z } }
pub fn vec4(x: f32, y: f32, z: f32, w: f32) -> Vec4 { Vector4D { x: x, y: y, z: z, w: w } }

pub type Mat4 = Matrix4x4<Untyped>;
pub type Mat3 = Matrix3x3<Untyped>;
pub type Mat2 = Matrix2x2<Untyped>;

pub type Rect = Rectangle<Untyped>;

pub trait ScalarMul<T> {
    fn scalar_mul(&self, scalar: T) -> Self;
    fn scalar_mul_in_place(&mut self, scalar: T);
}

#[derive(Copy, Clone, Show)]
pub struct Vector2D<Unit> {
    pub x: f32,
    pub y: f32,
}

#[derive(Copy, Clone, Show)]
pub struct Vector3D<Unit> {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Copy, Clone, Show)]
pub struct Vector4D<Unit> {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

#[derive(Copy, Clone, Show)]
pub struct Rectangle<Unit> {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl<U> Default for Vector2D<U> {
    fn default() -> Vector2D<U> { Vector2D { x: 0.0, y: 0.0 } }
}

impl<U> Default for Vector3D<U> {
    fn default() -> Vector3D<U> { Vector3D { x: 0.0, y: 0.0, z: 0.0 } }
}

impl<U> Default for Vector4D<U> {
    fn default() -> Vector4D<U> { Vector4D { x: 0.0, y: 0.0, z: 0.0, w: 0.0 } }
}

impl<U> Default for Rectangle<U> {
    fn default() -> Rectangle<U> { Rectangle { x: 0.0, y: 0.0, w: 0.0, h: 0.0 } }
}

#[allow(dead_code)]
impl<U> Vector4D<U> {
    pub fn from_slice(from: &[f32]) -> Vector4D<U> {
        assert!(from.len() >= 4);
        return Vector4D {
            x: from[0],
            y: from[1],
            z: from[2],
            w: from[3]
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

    pub fn xy(&self) -> Vector2D<U> { Vector2D { x: self.x, y:self.y } }
    pub fn xz(&self) -> Vector2D<U> { Vector2D { x: self.x, y:self.z } }
    pub fn yz(&self) -> Vector2D<U> { Vector2D { x: self.y, y:self.z } }
    pub fn yx(&self) -> Vector2D<U> { Vector2D { x: self.y, y:self.x } }
    pub fn xyz(&self) -> Vector3D<U> { Vector3D { x: self.x, y:self.y, z: self.z } }
    pub fn zxy(&self) -> Vector3D<U> { Vector3D { x: self.z, y:self.x, z: self.y } }
    pub fn yzx(&self) -> Vector3D<U> { Vector3D { x: self.y, y:self.z, z: self.x } }
    pub fn xzy(&self) -> Vector3D<U> { Vector3D { x: self.x, y:self.z, z: self.y } }
    pub fn yxz(&self) -> Vector3D<U> { Vector3D { x: self.y, y:self.x, z: self.z } }
    pub fn wxyz(&self) -> Vector4D<U> { Vector4D { x: self.w, y:self.x, z: self.y, w:self.z } }
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
            w: self.w + rhs.w
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
            w: self.w - rhs.w
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
            w: self.w * rhs.w
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
            w: self.w * rhs
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
            w: self.w / rhs.w
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
            w: -self.w
        };
    }
}

#[allow(dead_code)]
impl<U> Vector3D<U> {
    pub fn from_slice(from: &[f32]) -> Vector3D<U> {
        assert!(from.len() >= 3);
        return Vector3D {
            x: from[0],
            y: from[1],
            z: from[2],
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
            z: (self.x * rhs.y) - (self.y * rhs.x)
        }
    }

    pub fn length(&self) -> f32 {
        return self.square_length().sqrt();
    }

    pub fn square_length(&self) -> f32 {
        return self.x * self.x + self.y * self.y + self.z * self.z;
    }

    pub fn to_tuple(&self) -> (f32, f32, f32) { (self.x, self.y, self.z) }

    pub fn xy(&self) -> Vector2D<U> { Vector2D { x: self.x, y:self.y } }
    pub fn xz(&self) -> Vector2D<U> { Vector2D { x: self.x, y:self.z } }
    pub fn yz(&self) -> Vector2D<U> { Vector2D { x: self.y, y:self.z } }
    pub fn yx(&self) -> Vector2D<U> { Vector2D { x: self.y, y:self.x } }
    pub fn xyz(&self) -> Vector3D<U> { Vector3D { x: self.x, y:self.y, z: self.z } }
    pub fn zxy(&self) -> Vector3D<U> { Vector3D { x: self.z, y:self.x, z: self.y } }
    pub fn yzx(&self) -> Vector3D<U> { Vector3D { x: self.y, y:self.z, z: self.x } }
    pub fn xzy(&self) -> Vector3D<U> { Vector3D { x: self.x, y:self.z, z: self.y } }
    pub fn yxz(&self) -> Vector3D<U> { Vector3D { x: self.y, y:self.x, z: self.z } }

    pub fn to_vec4(&self, w: f32) -> Vector4D<U> {
        Vector4D {
            x: self.x,
            y: self.y,
            z: self.z,
            w: w,
        }
    }
}

impl<U> Matrix4x4<U> {
    pub fn perspective(
        fovy: f32, aspect: f32, near: f32, far: f32,
        mat: &mut Matrix4x4<U>
    ) {
        let f = 1.0 / (fovy / 2.0).tan();
        let nf: f32 = 1.0 / (near - far);

        mat._11 = f / aspect;
        mat._21 = 0.0;
        mat._31 = 0.0;
        mat._41 = 0.0;
        mat._12 = 0.0;
        mat._22 = f;
        mat._32 = 0.0;
        mat._42 = 0.0;
        mat._13 = 0.0;
        mat._23 = 0.0;
        mat._33 = (far + near) * nf;
        mat._43 = -1.0;
        mat._14 = 0.0;
        mat._24 = 0.0;
        mat._34 = (2.0 * far * near) * nf;
        mat._44 = 0.0;
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
        };
    }
}



#[allow(dead_code)]
impl<U> Vector2D<U> {
    pub fn from_slice(from: &[f32]) -> Vector2D<U> {
        assert!(from.len() >= 2);
        return Vector2D {
            x: from[0],
            y: from[1],
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
        Vector2D { x: self.x * f, y: self.y * f }
    }

    pub fn xy(&self) -> Vector2D<U> { Vector2D { x: self.x, y:self.y } }
    pub fn yx(&self) -> Vector2D<U> { Vector2D { x: self.y, y:self.x } }
}

#[allow(dead_code)]
impl<U> ops::Add<Vector2D<U>> for Vector2D<U> {

    type Output = Vector2D<U>;

    #[inline]
    fn add(self, rhs: Vector2D<U>) -> Vector2D<U> {
        return Vector2D {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
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
        };
    }
}

#[derive(Copy, Clone, PartialEq, Show)]
pub struct Matrix2x2<Unit> {
    pub _11: f32, pub _21: f32,
    pub _12: f32, pub _22: f32,
}

#[derive(Copy, Clone, PartialEq, Show)]
pub struct Matrix3x3<Unit> {
    pub _11: f32, pub _21: f32, pub _31: f32,
    pub _12: f32, pub _22: f32, pub _32: f32,
    pub _13: f32, pub _23: f32, pub _33: f32,
}

#[derive(Copy, Clone, PartialEq, Show)]
pub struct Matrix4x4<Unit> {
    pub _11: f32, pub _21: f32, pub _31: f32, pub _41: f32,
    pub _12: f32, pub _22: f32, pub _32: f32, pub _42: f32,
    pub _13: f32, pub _23: f32, pub _33: f32, pub _43: f32,
    pub _14: f32, pub _24: f32, pub _34: f32, pub _44: f32,
}



impl<U> Matrix2x2<U> {

    pub fn from_slice(from: &[f32]) -> Matrix2x2<U> {
        assert!(from.len() >= 4);
        return Matrix2x2 {
            _11: from[0], _21: from[1],
            _12: from[2], _22: from[3],
        };
    }

    pub fn as_slice<'l>(&'l self) -> &'l [f32] {
        unsafe {
            return mem::transmute((&self._11 as *const f32, 4 as usize ));
        }
    }

    pub fn as_mut_slice<'l>(&'l mut self) -> &'l mut [f32] {
        unsafe {
            return mem::transmute((&mut self._11 as *mut f32, 4 as usize ));
        }
    }

    pub fn row_1<'l>(&'l self) -> &'l Vector2D<U> {
        unsafe { mem::transmute(&self._11 as *const f32) }
    }

    pub fn row_2<'l>(&'l self) -> &'l Vector2D<U> {
        unsafe { mem::transmute(&self._12 as *const f32) }
    }

    #[inline]
    pub fn transform(&self, v: &Vector2D<U>) -> Vector2D<U> {
        Vector2D {
            x: v.x * self._11 + v.y * self._21,
            y: v.x * self._12 + v.y * self._22,
        }
    }

    #[inline]
    pub fn identity() -> Matrix2x2<U> {
        Matrix2x2 {
            _11: 1.0, _21: 0.0,
            _12: 0.0, _22: 1.0,
        }
    }

    #[inline]
    pub fn set_indentity(&mut self) {
        self._11 = 1.0; self._21 = 0.0;
        self._12 = 0.0; self._22 = 1.0;
    }
}

#[allow(dead_code)]
impl<U> Matrix3x3<U> {

    pub fn from_slice(from: &[f32]) -> Matrix3x3<U> {
        assert_eq!(from.len(), 9);
        return Matrix3x3 {
            _11: from[0], _21: from[1], _31: from[2],
            _12: from[3], _22: from[4], _32: from[5],
            _13: from[6], _23: from[7], _33: from[8],
        };
    }

    pub fn as_slice<'l>(&'l self) -> &'l [f32] {
        unsafe {
            return mem::transmute((&self._11 as *const f32, 9 as usize ));
        }
    }

    pub fn as_mut_slice<'l>(&'l mut self) -> &'l mut [f32] {
        unsafe {
            return mem::transmute((&mut self._11 as *mut f32, 9 as usize ));
        }
    }

    pub fn transform(&self, p: &Vector3D<U>) -> Vector3D<U> {
        Vector3D {
            x: p.x * self._11 + p.y * self._21 + p.z * self._31,
            y: p.x * self._12 + p.y * self._22 + p.z * self._32,
            z: p.x * self._13 + p.y * self._23 + p.z * self._33,
        }
    }

    pub fn transform_2d(&self, p: &Vector2D<U>) -> Vector2D<U> {
        Vector2D {
            x: p.x * self._11 + p.y * self._21 + self._31,
            y: p.x * self._12 + p.y * self._22 + self._32,
        }
    }

    pub fn scale_by(&mut self, v: &Vector2D<U>) {
        self._11 = self._11 * v.x;
        self._21 = self._21 * v.x;
        self._31 = self._31 * v.x;
        self._12 = self._12 * v.y;
        self._22 = self._22 * v.y;
        self._32 = self._32 * v.y;
    }

    pub fn scale(v: &Vector2D<U>) -> Matrix3x3<U> {
        return Matrix3x3 {
            _11: v.x,  _21: 0.0,  _31: 0.0,
            _12: 0.0,  _22: v.y,  _32: 0.0,
            _13: 0.0,  _23: 0.0,  _33: 1.0,
        }
    }

    pub fn translation(v: &Vector2D<U>) -> Matrix3x3<U> {
        return Matrix3x3 {
            _11: 1.0, _21: 1.0, _31: v.x,
            _12: 0.0, _22: 1.0, _32: v.y,
            _13: 0.0, _23: 0.0, _33: 1.0,
        }
    }

    pub fn rotation(rad: f32) -> Matrix3x3<U> {
        return Matrix3x3 {
            _11: rad.cos(), _21: -rad.sin(), _31: 0.0,
            _12: rad.sin(), _22: rad.cos(),  _32: 0.0,
            _13: 0.0,       _23: 0.0,        _33: 1.0,
        }
    }

    pub fn row_1<'l>(&'l self) -> &'l Vector3D<U> {
        unsafe { mem::transmute(&self._11 as *const f32) }
    }

    pub fn row_2<'l>(&'l self) -> &'l Vector3D<U> {
        unsafe { mem::transmute(&self._12 as *const f32) }
    }

    pub fn row_3<'l>(&'l self) -> &'l Vector3D<U> {
        unsafe { mem::transmute(&self._13 as *const f32) }
    }

    #[inline]
    pub fn identity() -> Matrix3x3<U> {
        Matrix3x3 {
            _11: 1.0, _21: 0.0, _31: 0.0,
            _12: 0.0, _22: 1.0, _32: 0.0,
            _13: 0.0, _23: 0.0, _33: 1.0,
        }
    }

    #[inline]
    pub fn set_indentity(&mut self) {
        self._11 = 1.0; self._21 = 0.0; self._31 = 0.0;
        self._12 = 0.0; self._22 = 1.0; self._32 = 0.0;
        self._13 = 0.0; self._23 = 0.0; self._33 = 1.0;
    }
}

#[allow(dead_code)]
impl<U> Matrix4x4<U> {

    pub fn from_slice(from: &[f32]) -> Matrix4x4<U> {
        assert!(from.len() >= 16);
        return Matrix4x4 {
            _11: from[0],  _21: from[1],  _31: from[2],  _41: from[3],
            _12: from[4],  _22: from[5],  _32: from[6],  _42: from[7],
            _13: from[8],  _23: from[9],  _33: from[10], _43: from[11],
            _14: from[12], _24: from[13], _34: from[14], _44: from[15],
        };
    }

    pub fn as_slice<'l>(&'l self) -> &'l [f32] {
        unsafe {
            return mem::transmute((&self._11 as *const f32, 16 as usize ));
        }
    }

    pub fn as_mut_slice<'l>(&'l mut self) -> &'l mut [f32] {
        unsafe {
            return mem::transmute((&mut self._11 as *mut f32, 16 as usize ));
        }
    }

    pub fn transform(&self, p: &Vector4D<U>) -> Vector4D<U> {
        Vector4D {
            x: p.x * self._11 + p.y * self._21 + p.z * self._31 + p.w * self._41,
            y: p.x * self._12 + p.y * self._22 + p.z * self._32 + p.w * self._42,
            z: p.x * self._13 + p.y * self._23 + p.z * self._33 + p.w * self._43,
            w: p.x * self._14 + p.y * self._24 + p.z * self._34 + p.w * self._44,
        }
    }

    pub fn row_1<'l>(&'l self) -> &'l Vector4D<U> {
        unsafe { mem::transmute(&self._11 as *const f32) }
    }

    pub fn row_2<'l>(&'l self) -> &'l Vector4D<U> {
        unsafe { mem::transmute(&self._12 as *const f32) }
    }

    pub fn row_3<'l>(&'l self) -> &'l Vector4D<U> {
        unsafe { mem::transmute(&self._13 as *const f32) }
    }

    pub fn row_4<'l>(&'l self) -> &'l Vector4D<U> {
        unsafe { mem::transmute(&self._14 as *const f32) }
    }

    #[inline]
    pub fn identity() -> Matrix4x4<U> {
        Matrix4x4 {
            _11: 1.0, _21: 0.0, _31: 0.0, _41: 0.0,
            _12: 0.0, _22: 1.0, _32: 0.0, _42: 0.0,
            _13: 0.0, _23: 0.0, _33: 1.0, _43: 0.0,
            _14: 0.0, _24: 0.0, _34: 0.0, _44: 1.0,
        }
    }

    pub fn scale(v: &Vector3D<U>) -> Matrix4x4<U> {
        return Matrix4x4 {
            _11: v.x, _21: 1.0, _31: 0.0, _41: 0.0,
            _12: 0.0, _22: v.y, _32: 0.0, _42: 0.0,
            _13: 0.0, _23: 0.0, _33: v.z, _43: 0.0,
            _14: 0.0, _24: 0.0, _34: 0.0, _44: 1.0,
        }
    }

    pub fn translation(v: &Vector3D<U>) -> Matrix4x4<U> {
        return Matrix4x4 {
            _11: 1.0, _21: 1.0, _31: 0.0, _41: v.x,
            _12: 0.0, _22: 1.0, _32: 0.0, _42: v.y,
            _13: 0.0, _23: 0.0, _33: 1.0, _43: v.z,
            _14: 0.0, _24: 0.0, _34: 0.0, _44: 1.0,
        }
    }

    #[inline]
    pub fn set_indentity(&mut self) {
        self._11 = 1.0; self._21 = 0.0; self._31 = 0.0; self._41 = 0.0;
        self._12 = 0.0; self._22 = 1.0; self._32 = 0.0; self._42 = 0.0;
        self._13 = 0.0; self._23 = 0.0; self._33 = 1.0; self._43 = 0.0;
        self._14 = 0.0; self._24 = 0.0; self._34 = 0.0; self._44 = 1.0;
    }
}

impl<U> Matrix4x4<U> {
    pub fn rotate(&mut self, rad: f32, axis: &Vector3D<U>) {
        let len = (axis.x * axis.x + axis.y * axis.y + axis.z * axis.z).sqrt();

        if len.abs() < EPSILON { return; }

        let len = 1.0 / len;
        let x = axis.x * len;
        let y = axis.y * len;
        let z = axis.z * len;

        let s = rad.sin();
        let c = rad.cos();
        let t = 1.0 - c;

        let a00 = self._11;
        let a01 = self._21;
        let a02 = self._31;
        let a03 = self._41;
        let a10 = self._12;
        let a11 = self._22;
        let a12 = self._32;
        let a13 = self._42;
        let a20 = self._13;
        let a21 = self._23;
        let a22 = self._33;
        let a23 = self._43;

        // Construct the elements of the rotation matrix
        let b00 = x * x * t + c;
        let b01 = y * x * t + z * s;
        let b02 = z * x * t - y * s;
        let b10 = x * y * t - z * s;
        let b11 = y * y * t + c;
        let b12 = z * y * t + x * s;
        let b20 = x * z * t + y * s;
        let b21 = y * z * t - x * s;
        let b22 = z * z * t + c;

        // Perform rotation-specific matrix multiplication
        self._11 = a00 * b00 + a10 * b01 + a20 * b02;
        self._21 = a01 * b00 + a11 * b01 + a21 * b02;
        self._31 = a02 * b00 + a12 * b01 + a22 * b02;
        self._41 = a03 * b00 + a13 * b01 + a23 * b02;
        self._12 = a00 * b10 + a10 * b11 + a20 * b12;
        self._22 = a01 * b10 + a11 * b11 + a21 * b12;
        self._32 = a02 * b10 + a12 * b11 + a22 * b12;
        self._42 = a03 * b10 + a13 * b11 + a23 * b12;
        self._13 = a00 * b20 + a10 * b21 + a20 * b22;
        self._23 = a01 * b20 + a11 * b21 + a21 * b22;
        self._33 = a02 * b20 + a12 * b21 + a22 * b22;
        self._43 = a03 * b20 + a13 * b21 + a23 * b22;
    }

    pub fn translate(&mut self, v: &Vector3D<U>) {
        self._14 = self._11 * v.x + self._12 * v.y + self._13 * v.z + self._14;
        self._24 = self._21 * v.x + self._22 * v.y + self._23 * v.z + self._24;
        self._34 = self._31 * v.x + self._32 * v.y + self._33 * v.z + self._34;
        self._44 = self._41 * v.x + self._42 * v.y + self._43 * v.z + self._44;
    }

    pub fn scale_by(&mut self, v: &Vector3D<U>) {
        self._11 = self._11 * v.x;
        self._21 = self._21 * v.x;
        self._31 = self._31 * v.x;
        self._41 = self._41 * v.x;
        self._12 = self._12 * v.y;
        self._22 = self._22 * v.y;
        self._32 = self._32 * v.y;
        self._42 = self._42 * v.y;
        self._13 = self._13 * v.z;
        self._23 = self._23 * v.z;
        self._33 = self._33 * v.z;
        self._43 = self._43 * v.z;
    }

    pub fn invert(&self, out: &mut Matrix4x4<U>) {
        let a00 = self._11;
        let a01 = self._21;
        let a02 = self._31;
        let a03 = self._41;
        let a10 = self._12;
        let a11 = self._22;
        let a12 = self._32;
        let a13 = self._42;
        let a20 = self._13;
        let a21 = self._23;
        let a22 = self._33;
        let a23 = self._43;
        let a30 = self._14;
        let a31 = self._24;
        let a32 = self._34;
        let a33 = self._44;

        let b00 = a00 * a11 - a01 * a10;
        let b01 = a00 * a12 - a02 * a10;
        let b02 = a00 * a13 - a03 * a10;
        let b03 = a01 * a12 - a02 * a11;
        let b04 = a01 * a13 - a03 * a11;
        let b05 = a02 * a13 - a03 * a12;
        let b06 = a20 * a31 - a21 * a30;
        let b07 = a20 * a32 - a22 * a30;
        let b08 = a20 * a33 - a23 * a30;
        let b09 = a21 * a32 - a22 * a31;
        let b10 = a21 * a33 - a23 * a31;
        let b11 = a22 * a33 - a23 * a32;

        let det = b00 * b11 - b01 * b10 + b02 * b09 + b03 * b08 - b04 * b07 + b05 * b06;

        if det.abs() < EPSILON {
            panic!(); // TODO
        }

        let det = 1.0 / det;

        out._11 = (a11 * b11 - a12 * b10 + a13 * b09) * det;
        out._21 = (a02 * b10 - a01 * b11 - a03 * b09) * det;
        out._31 = (a31 * b05 - a32 * b04 + a33 * b03) * det;
        out._41 = (a22 * b04 - a21 * b05 - a23 * b03) * det;
        out._12 = (a12 * b08 - a10 * b11 - a13 * b07) * det;
        out._22 = (a00 * b11 - a02 * b08 + a03 * b07) * det;
        out._32 = (a32 * b02 - a30 * b05 - a33 * b01) * det;
        out._42 = (a20 * b05 - a22 * b02 + a23 * b01) * det;
        out._13 = (a10 * b10 - a11 * b08 + a13 * b06) * det;
        out._23 = (a01 * b08 - a00 * b10 - a03 * b06) * det;
        out._33 = (a30 * b04 - a31 * b02 + a33 * b00) * det;
        out._43 = (a21 * b02 - a20 * b04 - a23 * b00) * det;
        out._14 = (a11 * b07 - a10 * b09 - a12 * b06) * det;
        out._24 = (a00 * b09 - a01 * b07 + a02 * b06) * det;
        out._34 = (a31 * b01 - a30 * b03 - a32 * b00) * det;
        out._44 = (a20 * b03 - a21 * b01 + a22 * b00) * det;
    }
}

#[allow(dead_code)]
impl<U> ops::Mul<Matrix4x4<U>> for Matrix4x4<U> {

    type Output = Matrix4x4<U>;

    #[inline]
    fn mul(self, rhs: Matrix4x4<U>) -> Matrix4x4<U> {
        return Matrix4x4 {
            _11: self._11 * rhs._11 + self._12 * rhs._21 + self._13 * rhs._31 + self._14 * rhs._41,
            _21: self._21 * rhs._11 + self._22 * rhs._21 + self._23 * rhs._31 + self._24 * rhs._41,
            _31: self._31 * rhs._11 + self._32 * rhs._21 + self._33 * rhs._31 + self._34 * rhs._41,
            _41: self._41 * rhs._11 + self._42 * rhs._21 + self._43 * rhs._31 + self._44 * rhs._41,
            _12: self._11 * rhs._12 + self._12 * rhs._22 + self._13 * rhs._32 + self._14 * rhs._42,
            _22: self._21 * rhs._12 + self._22 * rhs._22 + self._23 * rhs._32 + self._24 * rhs._42,
            _32: self._31 * rhs._12 + self._32 * rhs._22 + self._33 * rhs._32 + self._34 * rhs._42,
            _42: self._41 * rhs._12 + self._42 * rhs._22 + self._43 * rhs._32 + self._44 * rhs._42,
            _13: self._11 * rhs._13 + self._12 * rhs._23 + self._13 * rhs._33 + self._14 * rhs._43,
            _23: self._21 * rhs._13 + self._22 * rhs._23 + self._23 * rhs._33 + self._24 * rhs._43,
            _33: self._31 * rhs._13 + self._32 * rhs._23 + self._33 * rhs._33 + self._34 * rhs._43,
            _43: self._41 * rhs._13 + self._42 * rhs._23 + self._43 * rhs._33 + self._44 * rhs._43,
            _14: self._11 * rhs._14 + self._12 * rhs._24 + self._13 * rhs._34 + self._14 * rhs._44,
            _24: self._21 * rhs._14 + self._22 * rhs._24 + self._23 * rhs._34 + self._24 * rhs._44,
            _34: self._31 * rhs._14 + self._32 * rhs._24 + self._33 * rhs._34 + self._34 * rhs._44,
            _44: self._41 * rhs._14 + self._42 * rhs._24 + self._43 * rhs._34 + self._44 * rhs._44,
        };
    }
}

#[allow(dead_code)]
impl<U> ops::Mul<Matrix3x3<U>> for Matrix3x3<U> {

    type Output = Matrix3x3<U>;

    #[inline]
    fn mul(self, rhs: Matrix3x3<U>) -> Matrix3x3<U> {
        return Matrix3x3 {
            _11: self._11 * rhs._11 + self._12 * rhs._21 + self._13 * rhs._31,
            _21: self._21 * rhs._11 + self._22 * rhs._21 + self._23 * rhs._31,
            _31: self._31 * rhs._11 + self._32 * rhs._21 + self._33 * rhs._31,
            _12: self._11 * rhs._12 + self._12 * rhs._22 + self._13 * rhs._32,
            _22: self._21 * rhs._12 + self._22 * rhs._22 + self._23 * rhs._32,
            _32: self._31 * rhs._12 + self._32 * rhs._22 + self._33 * rhs._32,
            _13: self._11 * rhs._13 + self._12 * rhs._23 + self._13 * rhs._33,
            _23: self._21 * rhs._13 + self._22 * rhs._23 + self._23 * rhs._33,
            _33: self._31 * rhs._13 + self._32 * rhs._23 + self._33 * rhs._33,
        };
    }
}

#[allow(dead_code)]
impl<U> ops::Mul<Matrix2x2<U>> for Matrix2x2<U> {

    type Output = Matrix2x2<U>;

    #[inline]
    fn mul(self, rhs: Matrix2x2<U>) -> Matrix2x2<U> {
        return Matrix2x2 {
            _11: self._11 * rhs._11 + self._12 * rhs._21,
            _21: self._21 * rhs._11 + self._22 * rhs._21,
            _12: self._11 * rhs._12 + self._12 * rhs._22,
            _22: self._21 * rhs._12 + self._22 * rhs._22,
        };
    }
}

impl<U> Rectangle<U> {
    pub fn new(x: f32, y: f32, w: f32, h:f32) -> Rectangle<U> {
        let mut rect = Rectangle { x: x, y: y, w: w, h: h };
        rect.ensure_invariant();
        return rect;
    }

    pub fn origin(&self) -> Vector2D<U> { Vector2D { x: self.x, y: self.y } }

    pub fn size(&self) -> Vector2D<U> { Vector2D { x: self.w, y: self.h } }

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
        }
    }
    pub fn top_right(&self) -> Vector2D<U> {
        Vector2D {
            x: self.x + self.w,
            y: self.y,
        }
    }

    pub fn bottom_right(&self) -> Vector2D<U> {
        Vector2D {
            x: self.x + self.w,
            y: self.y + self.h,
        }
    }

    pub fn bottom_left(&self) -> Vector2D<U> {
        Vector2D {
            x: self.x,
            y: self.y + self.h,
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
