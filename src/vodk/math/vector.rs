
use std::cast;
use std::ops;
//use std::fmt;
use std::kinds::Copy;
use std::num;

pub static EPSILON: f32 = 0.000001;
pub static PI: f32 = 3.14159265359;

#[deriving(Eq, Show)]
pub struct Untyped;

pub type Vec2 = Vector2D<f32, Untyped>;
pub type Vec3 = Vector3D<f32, Untyped>;
pub type Vec4 = Vector4D<f32, Untyped>;

pub fn vec2(x: f32, y: f32) -> Vec2 { Vector2D { x: x, y: y } }
pub fn vec3(x: f32, y: f32, z: f32) -> Vec3 { Vector3D { x: x, y: y, z: z } }
pub fn vec4(x: f32, y: f32, z: f32, w: f32) -> Vec4 { Vector4D { x: x, y: y, z: z, w: w } }

pub type Mat4 = Matrix4D<f32, Untyped>;
pub type Mat3 = Matrix3D<f32, Untyped>;
pub type Mat2 = Matrix2D<f32, Untyped>;

#[allow(dead_code)]
pub mod Mat4 {
    use super::{Mat4, Matrix4D, Vec3};
    pub fn identity() -> Mat4 {
        Matrix4D {
            _11: 1.0, _21: 0.0, _31: 0.0, _41: 0.0,
            _12: 0.0, _22: 1.0, _32: 0.0, _42: 0.0,
            _13: 0.0, _23: 0.0, _33: 1.0, _43: 0.0,
            _14: 0.0, _24: 0.0, _34: 0.0, _44: 1.0,
        }

    }

    pub fn perspective(fovy: f32, aspect: f32, near: f32, far: f32, mat: &mut Mat4) {
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

    pub fn from_slice(s: &[f32]) -> Mat4 {
        return Matrix4D::from_slice(s);
    }

    pub fn scale(s: &Vec3) -> Mat4 {
        let mut m = identity();
        m.scale(s);
        return m;
    }

    pub fn rotation(rad: f32, s: &Vec3) -> Mat4 {
        let mut m = identity();
        m.rotate(rad, s);
        return m;
    }

    pub fn translation(v: &Vec3) -> Mat4 {
        let mut m = identity();
        m.translate(v);
        return m;
    }
}

#[allow(dead_code)]
pub mod Mat3 {
    use super::{Mat3, Matrix3D};
    pub fn identity() -> Mat3 {
        Matrix3D {
            _11: 1.0, _21: 0.0, _31: 0.0,
            _12: 0.0, _22: 1.0, _32: 0.0,
            _13: 0.0, _23: 0.0, _33: 1.0,
        }
    }

    pub fn from_slice(s: &[f32]) -> Mat3 {
        return Matrix3D::from_slice(s);
    }
}

#[allow(dead_code)]
pub mod Mat2 {
    use super::{Mat2, Matrix2D};
    pub fn identity() -> Mat2 {
        Matrix2D {
            _11: 1.0, _21: 0.0,
            _12: 0.0, _22: 1.0,
        }
    }

    pub fn from_slice(s: &[f32]) -> Mat2 {
        return Matrix2D::from_slice(s);
    }
}

#[deriving(Show)]
pub struct Vector2D<T, Unit = Untyped> {
    pub x: T,
    pub y: T,
}

#[deriving(Show)]
pub struct Vector3D<T, Unit = Untyped> {
    pub x: T,
    pub y: T,
    pub z: T,
}

#[deriving(Show)]
pub struct Vector4D<T, Unit = Untyped> {
    pub x: T,
    pub y: T,
    pub z: T,
    pub w: T,
}

#[allow(dead_code)]
impl<T: Copy + Float, U> Vector4D<T, U> {
    pub fn from_slice(from: &[T]) -> Vector4D<T,U> {
        assert!(from.len() >= 4);
        return Vector4D {
            x: from[0],
            y: from[1],
            z: from[2],
            w: from[3]
        };
    }

    pub fn as_slice<'l>(&'l self) -> &'l [T] {
        unsafe {
            return cast::transmute((&'l self.x as *T, 4 as uint ));
        }
    }

    pub fn as_mut_slice<'l>(&'l mut self) -> &'l mut [T] {
        unsafe {
            return cast::transmute((&'l self.x as *T, 4 as uint ));
        }
    }

    #[inline]
    pub fn dot(&self, rhs: &Vector4D<T,U>) -> T {
        return self.x*rhs.x + self.y*rhs.y + self.z*rhs.z + self.w*rhs.w;
    }

    pub fn length(&self) -> T {
        return self.square_length().sqrt();
    }

    pub fn square_length(&self) -> T {
        return self.x * self.x + self.y * self.y + self.z * self.z + self.w * self.w;
    }

    pub fn xy(&self) -> Vector2D<T,U> { Vector2D { x: self.x, y:self.y } }
    pub fn xz(&self) -> Vector2D<T,U> { Vector2D { x: self.x, y:self.z } }
    pub fn yz(&self) -> Vector2D<T,U> { Vector2D { x: self.y, y:self.z } }
    pub fn yx(&self) -> Vector2D<T,U> { Vector2D { x: self.y, y:self.x } }
    pub fn xyz(&self) -> Vector3D<T,U> { Vector3D { x: self.x, y:self.y, z: self.z } }
    pub fn zxy(&self) -> Vector3D<T,U> { Vector3D { x: self.z, y:self.x, z: self.y } }
    pub fn yzx(&self) -> Vector3D<T,U> { Vector3D { x: self.y, y:self.z, z: self.x } }
    pub fn xzy(&self) -> Vector3D<T,U> { Vector3D { x: self.x, y:self.z, z: self.y } }
    pub fn yxz(&self) -> Vector3D<T,U> { Vector3D { x: self.y, y:self.x, z: self.z } }
    pub fn wxyz(&self) -> Vector4D<T,U> { Vector4D { x: self.w, y:self.x, z: self.y, w:self.z } }
}

impl<T: Float+EpsilonEq, U> Eq for Vector4D<T, U> {
    fn eq(&self, rhs:&Vector4D<T,U>) -> bool {
        let d = *self - *rhs;
        return self.x.epsilon_eq(&rhs.x)
            && self.y.epsilon_eq(&rhs.y)
            && self.z.epsilon_eq(&rhs.z)
            && self.w.epsilon_eq(&rhs.w);
    }
}

impl<T: Float+EpsilonEq, U> Eq for Vector3D<T, U> {
    fn eq(&self, rhs:&Vector3D<T,U>) -> bool {
        return self.x.epsilon_eq(&rhs.x)
            && self.y.epsilon_eq(&rhs.y)
            && self.z.epsilon_eq(&rhs.z);
    }
}

impl<T: Float+EpsilonEq, U> Eq for Vector2D<T, U> {
    fn eq(&self, rhs:&Vector2D<T,U>) -> bool {
        return self.x.epsilon_eq(&rhs.x)
            && self.y.epsilon_eq(&rhs.y);
    }
}


#[allow(dead_code)]
impl<T: ops::Add<T,T>, U>
    ops::Add<Vector4D<T,U>, Vector4D<T,U>>
    for Vector4D<T,U> {

    #[inline]
    fn add(&self, rhs: &Vector4D<T,U>) -> Vector4D<T, U> {
        return Vector4D {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
            w: self.w + rhs.w
        };
    }
}

#[allow(dead_code)]
impl<T: ops::Sub<T,T>, U>
    ops::Sub<Vector4D<T,U>, Vector4D<T,U>>
    for Vector4D<T,U> {

    #[inline]
    fn sub(&self, rhs: &Vector4D<T,U>) -> Vector4D<T, U> {
        return Vector4D {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            z: self.z - rhs.z,
            w: self.w - rhs.w
        };
    }
}

#[allow(dead_code)]
impl<T: ops::Mul<T,T>, U>
    ops::Mul<Vector4D<T,U>, Vector4D<T,U>>
    for Vector4D<T,U> {

    #[inline]
    fn mul(&self, rhs: &Vector4D<T,U>) -> Vector4D<T, U> {
        return Vector4D {
            x: self.x * rhs.x,
            y: self.y * rhs.y,
            z: self.z * rhs.z,
            w: self.w * rhs.w
        };
    }
}

#[allow(dead_code)]
impl<T : ops::Neg<T>, U>
    ops::Neg<Vector4D<T,U>>
    for Vector4D<T,U> {

    #[inline]
    fn neg(&self) -> Vector4D<T, U> {
        return Vector4D {
            x: -self.x,
            y: -self.y,
            z: -self.z,
            w: -self.w
        };
    }
}


#[allow(dead_code)]
impl<T: Copy + Float, U> Vector3D<T, U> {
    pub fn from_slice(from: &[T]) -> Vector3D<T,U> {
        assert!(from.len() >= 3);
        return Vector3D {
            x: from[0],
            y: from[1],
            z: from[2],
        };
    }

    pub fn as_slice<'l>(&'l self) -> &'l [T] {
        unsafe {
            return cast::transmute((&'l self.x as *T, 3 as uint ));
        }
    }

    pub fn as_mut_slice<'l>(&'l mut self) -> &'l mut [T] {
        unsafe {
            return cast::transmute((&'l self.x as *T, 3 as uint ));
        }
    }

    #[inline]
    pub fn dot(&self, rhs: &Vector3D<T,U>) -> T {
        return self.x*rhs.x + self.y*rhs.y + self.z*rhs.z;
    }

    #[inline]
    pub fn cross(&self, rhs: &Vector3D<T,U>) -> Vector3D<T,U> {
        return Vector3D {
            x: (self.y * rhs.z) - (self.z * rhs.y),
            y: (self.z * rhs.x) - (self.x * rhs.z),
            z: (self.x * rhs.y) - (self.y * rhs.x)
        }
    }

    pub fn length(&self) -> T {
        return self.square_length().sqrt();
    }

    pub fn square_length(&self) -> T {
        return self.x * self.x + self.y * self.y + self.z * self.z;
    }

    pub fn xy(&self) -> Vector2D<T,U> { Vector2D { x: self.x, y:self.y } }
    pub fn xz(&self) -> Vector2D<T,U> { Vector2D { x: self.x, y:self.z } }
    pub fn yz(&self) -> Vector2D<T,U> { Vector2D { x: self.y, y:self.z } }
    pub fn yx(&self) -> Vector2D<T,U> { Vector2D { x: self.y, y:self.x } }
    pub fn xyz(&self) -> Vector3D<T,U> { Vector3D { x: self.x, y:self.y, z: self.z } }
    pub fn zxy(&self) -> Vector3D<T,U> { Vector3D { x: self.z, y:self.x, z: self.y } }
    pub fn yzx(&self) -> Vector3D<T,U> { Vector3D { x: self.y, y:self.z, z: self.x } }
    pub fn xzy(&self) -> Vector3D<T,U> { Vector3D { x: self.x, y:self.z, z: self.y } }
    pub fn yxz(&self) -> Vector3D<T,U> { Vector3D { x: self.y, y:self.x, z: self.z } }

    pub fn to_vec4(&self, w: T) -> Vector4D<T, U> {
        Vector4D {
            x: self.x,
            y: self.y,
            z: self.z,
            w: w,
        }
    }
}

#[allow(dead_code)]
impl<T: ops::Add<T,T>, U>
    ops::Add<Vector3D<T,U>, Vector3D<T,U>>
    for Vector3D<T,U> {

    #[inline]
    fn add(&self, rhs: &Vector3D<T,U>) -> Vector3D<T, U> {
        return Vector3D {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
        };
    }
}

#[allow(dead_code)]
impl<T: ops::Sub<T,T>, U>
    ops::Sub<Vector3D<T,U>, Vector3D<T,U>>
    for Vector3D<T,U> {

    #[inline]
    fn sub(&self, rhs: &Vector3D<T,U>) -> Vector3D<T, U> {
        return Vector3D {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            z: self.z - rhs.z,
        };
    }
}

#[allow(dead_code)]
impl<T: ops::Mul<T,T>, U>
    ops::Mul<Vector3D<T,U>, Vector3D<T,U>>
    for Vector3D<T,U> {

    #[inline]
    fn mul(&self, rhs: &Vector3D<T,U>) -> Vector3D<T, U> {
        return Vector3D {
            x: self.x * rhs.x,
            y: self.y * rhs.y,
            z: self.z * rhs.z,
        };
    }
}

#[allow(dead_code)]
impl<T : ops::Neg<T>, U>
    ops::Neg<Vector3D<T,U>>
    for Vector3D<T,U> {

    #[inline]
    fn neg(&self) -> Vector3D<T, U> {
        return Vector3D {
            x: -self.x,
            y: -self.y,
            z: -self.z,
        };
    }
}



#[allow(dead_code)]
impl<T: Copy + Num, U> Vector2D<T, U> {
    pub fn from_slice(from: &[T]) -> Vector2D<T,U> {
        assert!(from.len() >= 2);
        return Vector2D {
            x: from[0],
            y: from[1],
        };
    }

    pub fn as_slice<'l>(&'l self) -> &'l [T] {
        unsafe {
            return cast::transmute((&'l self.x as *T, 2 as uint ));
        }
    }

    pub fn as_mut_slice<'l>(&'l mut self) -> &'l mut [T] {
        unsafe {
            return cast::transmute((&'l self.x as *T, 2 as uint ));
        }
    }

    #[inline]
    pub fn dot(&self, rhs: &Vector2D<T,U>) -> T {
        return self.x*rhs.x + self.y*rhs.y;
    }

    pub fn xy(&self) -> Vector2D<T,U> { Vector2D { x: self.x, y:self.y } }
    pub fn yx(&self) -> Vector2D<T,U> { Vector2D { x: self.y, y:self.x } }
}

#[allow(dead_code)]
impl<T: ops::Add<T,T>, U>
    ops::Add<Vector2D<T,U>, Vector2D<T,U>>
    for Vector2D<T,U> {

    #[inline]
    fn add(&self, rhs: &Vector2D<T,U>) -> Vector2D<T, U> {
        return Vector2D {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        };
    }
}

#[allow(dead_code)]
impl<T: ops::Sub<T,T>, U>
    ops::Sub<Vector2D<T,U>, Vector2D<T,U>>
    for Vector2D<T,U> {

    #[inline]
    fn sub(&self, rhs: &Vector2D<T,U>) -> Vector2D<T, U> {
        return Vector2D {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        };
    }
}

#[allow(dead_code)]
impl<T: ops::Mul<T,T>, U>
    ops::Mul<Vector2D<T,U>, Vector2D<T,U>>
    for Vector2D<T,U> {

    #[inline]
    fn mul(&self, rhs: &Vector2D<T,U>) -> Vector2D<T, U> {
        return Vector2D {
            x: self.x * rhs.x,
            y: self.y * rhs.y,
        };
    }
}

#[allow(dead_code)]
impl<T : ops::Neg<T>, U>
    ops::Neg<Vector2D<T,U>>
    for Vector2D<T,U> {

    #[inline]
    fn neg(&self) -> Vector2D<T, U> {
        return Vector2D {
            x: -self.x,
            y: -self.y,
        };
    }
}

#[deriving(Eq)]
pub struct Matrix2D<T, Unit> {
    pub _11: T, pub _21: T,
    pub _12: T, pub _22: T,
}

#[deriving(Eq)]
pub struct Matrix3D<T, Unit> {
    pub _11: T, pub _21: T, pub _31: T,
    pub _12: T, pub _22: T, pub _32: T,
    pub _13: T, pub _23: T, pub _33: T,
}

#[deriving(Eq)]
pub struct Matrix4D<T, Unit> {
    pub _11: T, pub _21: T, pub _31: T, pub _41: T,
    pub _12: T, pub _22: T, pub _32: T, pub _42: T,
    pub _13: T, pub _23: T, pub _33: T, pub _43: T,
    pub _14: T, pub _24: T, pub _34: T, pub _44: T,
}



impl<T: Copy + Num, U> Matrix2D<T, U> {

    pub fn from_slice(from: &[T]) -> Matrix2D<T,U> {
        assert!(from.len() >= 4);
        return Matrix2D {
            _11: from[0], _21: from[1],
            _12: from[2], _22: from[3],
        };
    }

    pub fn as_slice<'l>(&'l self) -> &'l [T] {
        unsafe {
            return cast::transmute((&'l self._11 as *T, 4 as uint ));
        }
    }

    pub fn as_mut_slice<'l>(&'l mut self) -> &'l mut [T] {
        unsafe {
            return cast::transmute((&'l self._11 as *T, 4 as uint ));
        }
    }

    pub fn row_1<'l>(&'l self) -> &'l Vector2D<T,U> {
        unsafe { cast::transmute(&'l self._11 as *T) }
    }

    pub fn row_2<'l>(&'l self) -> &'l Vector2D<T,U> {
        unsafe { cast::transmute(&'l self._12 as *T) }
    }
}

#[allow(dead_code)]
impl<T: Copy + Add<T,T> + Sub<T,T> + Mul<T,T>, U> Matrix3D<T, U> {

    pub fn from_slice(from: &[T]) -> Matrix3D<T,U> {
        assert_eq!(from.len(), 9);
        return Matrix3D {
            _11: from[0], _21: from[1], _31: from[2],
            _12: from[3], _22: from[4], _32: from[5],
            _13: from[6], _23: from[7], _33: from[8],
        };
    }

    pub fn as_slice<'l>(&'l self) -> &'l [T] {
        unsafe {
            return cast::transmute((&'l self._11 as *T, 9 as uint ));
        }
    }

    pub fn as_mut_slice<'l>(&'l mut self) -> &'l mut [T] {
        unsafe {
            return cast::transmute((&'l self._11 as *T, 9 as uint ));
        }
    }

    pub fn transform(&self, p: &Vector3D<T,U>) -> Vector3D<T,U> {
        Vector3D {
            x: p.x * self._11 + p.y * self._21 + p.z * self._31,
            y: p.x * self._12 + p.y * self._22 + p.z * self._32,
            z: p.x * self._13 + p.y * self._23 + p.z * self._33,
        }
    }

    pub fn row_1<'l>(&'l self) -> &'l Vector3D<T,U> {
        unsafe { cast::transmute(&'l self._11 as *T) }
    }

    pub fn row_2<'l>(&'l self) -> &'l Vector3D<T,U> {
        unsafe { cast::transmute(&'l self._12 as *T) }
    }

    pub fn row_3<'l>(&'l self) -> &'l Vector3D<T,U> {
        unsafe { cast::transmute(&'l self._13 as *T) }
    }
}

#[allow(dead_code)]
impl<T: Copy + Num, U> Matrix4D<T, U> {

    pub fn from_slice(from: &[T]) -> Matrix4D<T,U> {
        assert!(from.len() >= 16);
        return Matrix4D {
            _11: from[0],  _21: from[1],  _31: from[2],  _41: from[3],
            _12: from[4],  _22: from[5],  _32: from[6],  _42: from[7],
            _13: from[8],  _23: from[9],  _33: from[10], _43: from[11],
            _14: from[12], _24: from[13], _34: from[14], _44: from[15],
        };
    }

    pub fn as_slice<'l>(&'l self) -> &'l [T] {
        unsafe {
            return cast::transmute((&'l self._11 as *T, 16 as uint ));
        }
    }

    pub fn as_mut_slice<'l>(&'l mut self) -> &'l mut [T] {
        unsafe {
            return cast::transmute((&'l self._11 as *T, 16 as uint ));
        }
    }

    pub fn transform(&self, p: &Vector4D<T,U>) -> Vector4D<T,U> {
        Vector4D {
            x: p.x * self._11 + p.y * self._21 + p.z * self._31 + p.w * self._41,
            y: p.x * self._12 + p.y * self._22 + p.z * self._32 + p.w * self._42,
            z: p.x * self._13 + p.y * self._23 + p.z * self._33 + p.w * self._43,
            w: p.x * self._14 + p.y * self._24 + p.z * self._34 + p.w * self._44,
        }
    }

    pub fn row_1<'l>(&'l self) -> &'l Vector4D<T,U> {
        unsafe { cast::transmute(&'l self._11 as *T) }
    }

    pub fn row_2<'l>(&'l self) -> &'l Vector4D<T,U> {
        unsafe { cast::transmute(&'l self._12 as *T) }
    }

    pub fn row_3<'l>(&'l self) -> &'l Vector4D<T,U> {
        unsafe { cast::transmute(&'l self._13 as *T) }
    }

    pub fn row_4<'l>(&'l self) -> &'l Vector4D<T,U> {
        unsafe { cast::transmute(&'l self._14 as *T) }
    }
}

impl<U> Matrix4D<f32,U> {
    pub fn rotate(&mut self, rad: f32, axis: &Vec3) {
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

    pub fn translate(&mut self, v: &Vec3) {
        self._14 = self._11 * v.x + self._12 * v.y + self._13 * v.z + self._14;
        self._24 = self._21 * v.x + self._22 * v.y + self._23 * v.z + self._24;
        self._34 = self._31 * v.x + self._32 * v.y + self._33 * v.z + self._34;
        self._44 = self._41 * v.x + self._42 * v.y + self._43 * v.z + self._44;
    }

    pub fn scale(&mut self, v: &Vec3) {
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

    pub fn invert(&self, out: &mut Matrix4D<f32,U>) {
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
            fail!(); // TODO
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
impl<T: ops::Mul<T,T> + ops::Add<T,T>, U>
    ops::Mul<Matrix4D<T,U>, Matrix4D<T,U>>
    for Matrix4D<T,U> {

    #[inline]
    fn mul(&self, rhs: &Matrix4D<T,U>) -> Matrix4D<T, U> {
        return Matrix4D {
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
impl<T: ops::Mul<T,T> + ops::Add<T,T>, U>
    ops::Mul<Matrix3D<T,U>, Matrix3D<T,U>>
    for Matrix3D<T,U> {

    #[inline]
    fn mul(&self, rhs: &Matrix3D<T,U>) -> Matrix3D<T, U> {
        return Matrix3D {
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
impl<T: ops::Mul<T,T> + ops::Add<T,T>, U>
    ops::Mul<Matrix2D<T,U>, Matrix2D<T,U>>
    for Matrix2D<T,U> {

    #[inline]
    fn mul(&self, rhs: &Matrix2D<T,U>) -> Matrix2D<T, U> {
        return Matrix2D {
            _11: self._11 * rhs._11 + self._12 * rhs._21,
            _21: self._21 * rhs._11 + self._22 * rhs._21,
            _12: self._11 * rhs._12 + self._12 * rhs._22,
            _22: self._21 * rhs._12 + self._22 * rhs._22,
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
