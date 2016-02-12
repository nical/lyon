
use std::mem::transmute;
use std::ops;
use std::default::Default;
use std::marker::PhantomData;

use common::Untyped;

use super::vec2::Vector2D;
use super::vec4::Vector4D;

pub type Vec3 = Vector3D<Untyped>;

pub fn vec3(x: f32, y: f32, z: f32) -> Vec3 { Vector3D { x: x, y: y, z: z, _unit: PhantomData } }

#[derive(Copy, Clone, Debug)]
pub struct Vector3D<Unit> {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    _unit: PhantomData<Unit>,
}

impl<U> AsRef<[f32; 3]> for Vector3D<U> {
    fn as_ref(&self) -> &[f32; 3] { unsafe { transmute(self) } }
}

impl<U> AsRef<(f32, f32, f32)> for Vector3D<U> {
    fn as_ref(&self) -> &(f32, f32, f32) { unsafe { transmute(self) } }
}

impl<U> AsMut<[f32; 3]> for Vector3D<U> {
    fn as_mut(&mut self) -> &mut [f32; 3] { unsafe { transmute(self) } }
}

impl<U> AsMut<(f32, f32, f32)> for Vector3D<U> {
    fn as_mut(&mut self) -> &mut (f32, f32, f32) { unsafe { transmute(self) } }
}

impl<U> AsRef<Vector3D<U>> for [f32; 3] {
    fn as_ref(&self) -> &Vector3D<U> { unsafe { transmute(self) } }
}

impl<U> AsRef<Vector3D<U>> for (f32, f32, f32) {
    fn as_ref(&self) -> &Vector3D<U> { unsafe { transmute(self) } }
}

impl<U> AsMut<Vector3D<U>> for [f32; 3] {
    fn as_mut(&mut self) -> &mut Vector3D<U> { unsafe { transmute(self) } }
}

impl<U> AsMut<Vector3D<U>> for (f32, f32, f32) {
    fn as_mut(&mut self) -> &mut Vector3D<U> { unsafe { transmute(self) } }
}

impl<U> Default for Vector3D<U> {
    fn default() -> Vector3D<U> { Vector3D { x: 0.0, y: 0.0, z: 0.0, _unit: PhantomData } }
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
            return transmute((&self.x as *const f32, 3 as usize ));
        }
    }

    pub fn as_mut_slice<'l>(&'l mut self) -> &'l mut [f32] {
        unsafe {
            return transmute((&mut self.x as *mut f32, 3 as usize ));
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

    pub fn tuple(&self) -> (f32, f32, f32) { (self.x, self.y, self.z) }

    pub fn array(&self) -> [f32; 3] { [self.x, self.y, self.z] }

    pub fn xy(&self) -> Vector2D<U> { Vector2D::new(self.x, self.y) }
    pub fn xz(&self) -> Vector2D<U> { Vector2D::new(self.x, self.z) }
    pub fn yz(&self) -> Vector2D<U> { Vector2D::new(self.y, self.z) }
    pub fn yx(&self) -> Vector2D<U> { Vector2D::new(self.y, self.x) }
    pub fn xyz(&self) -> Vector3D<U> { Vector3D::new(self.x, self.y, self.z) }
    pub fn zxy(&self) -> Vector3D<U> { Vector3D::new(self.z, self.x, self.y) }
    pub fn yzx(&self) -> Vector3D<U> { Vector3D::new(self.y, self.z, self.x) }
    pub fn xzy(&self) -> Vector3D<U> { Vector3D::new(self.x, self.z, self.y) }
    pub fn yxz(&self) -> Vector3D<U> { Vector3D::new(self.y, self.x, self.z) }

    pub fn to_vec4(&self, w: f32) -> Vector4D<U> {
        Vector4D::new(self.x, self.y, self.z, w)
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

impl<U> ops::Mul<f32> for Vector3D<U> {

    type Output = Vector3D<U>;

    #[inline]
    fn mul(self, rhs: f32) -> Vector3D<U> {
        return Vector3D::new(self.x * rhs, self.y * rhs, self.z * rhs);
    }
}

#[allow(dead_code)]
impl<U> ops::Div<Vector3D<U>> for Vector3D<U> {

    type Output = Vector3D<U>;

    #[inline]
    fn div(self, rhs: Vector3D<U>) -> Vector3D<U> {
        return Vector3D::new(self.x / rhs.x, self.y / rhs.y, self.z / rhs.z);
    }
}

impl<U> ops::Div<f32> for Vector3D<U> {

    type Output = Vector3D<U>;

    #[inline]
    fn div(self, rhs: f32) -> Vector3D<U> {
        return Vector3D::new(self.x / rhs, self.y / rhs, self.z / rhs);
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
