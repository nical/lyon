
use std::cast;
use std::ops;
use std::fmt;
use std::kinds::Copy;

#[deriving(Eq)]
struct Vector2D<T, Unit> {
    x: T,
    y: T,
}

#[deriving(Eq)]
struct Vector3D<T, Unit> {
    x: T,
    y: T,
    z: T,
}

#[deriving(Eq)]
pub struct Vector4D<T, Unit> {
    pub x: T,
    pub y: T,
    pub z: T,
    pub w: T,
}

pub struct Untyped;
pub type Vec2 = Vector2D<f32, Untyped>;
pub type Vec3 = Vector3D<f32, Untyped>;
pub type Vec4 = Vector4D<f32, Untyped>;

pub fn vec2(x: f32, y: f32) -> Vec2 { Vector2D { x: x, y: y } }
pub fn vec3(x: f32, y: f32, z: f32) -> Vec3 { Vector3D { x: x, y: y, z: z } }
pub fn vec4(x: f32, y: f32, z: f32, w: f32) -> Vec4 { Vector4D { x: x, y: y, z: z, w: w } }


impl<T: Copy + Add<T,T> + Mul<T,T>, U> Vector4D<T, U> {
    pub fn new(x: T, y: T, z: T, w: T) -> Vector4D<T,U> {
        Vector4D { x: x, y: y, z: z, w: w }
    }

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

    #[inline]
    pub fn dot(&self, other: &Vector4D<T,U>) -> T {
        return self.x*other.x + self.y*other.y + self.z*other.z + self.w*other.w;
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
}

impl<T: ops::Add<T,T>, U>
    ops::Add<Vector4D<T,U>, Vector4D<T,U>>
    for Vector4D<T,U> {

    #[inline]
    fn add(&self, other: &Vector4D<T,U>) -> Vector4D<T, U> {
        return Vector4D {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
            w: self.w + other.w
        };
    }
}

impl<T: ops::Sub<T,T>, U>
    ops::Sub<Vector4D<T,U>, Vector4D<T,U>>
    for Vector4D<T,U> {

    #[inline]
    fn sub(&self, other: &Vector4D<T,U>) -> Vector4D<T, U> {
        return Vector4D {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
            w: self.w - other.w
        };
    }
}

impl<T: ops::Mul<T,T>, U>
    ops::Mul<Vector4D<T,U>, Vector4D<T,U>>
    for Vector4D<T,U> {

    #[inline]
    fn mul(&self, other: &Vector4D<T,U>) -> Vector4D<T, U> {
        return Vector4D {
            x: self.x * other.x,
            y: self.y * other.y,
            z: self.z * other.z,
            w: self.w * other.w
        };
    }
}

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



impl<T: Copy + Add<T,T> + Sub<T,T> + Mul<T,T>, U> Vector3D<T, U> {
    pub fn new(x: T, y: T, z: T) -> Vector3D<T,U> {
        Vector3D { x: x, y: y, z: z}
    }

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
            return cast::transmute((&'l self.x as *T, 4 as uint ));
        }
    }

    #[inline]
    pub fn dot(&self, other: &Vector3D<T,U>) -> T {
        return self.x*other.x + self.y*other.y + self.z*other.z;
    }

    #[inline]
    pub fn cross(&self, other: &Vector3D<T,U>) -> Vector3D<T,U> {
        return Vector3D {
            x: (self.y * other.z) - (self.z * other.y),
            y: (self.z * other.x) - (self.x * other.z),
            z: (self.x * other.y) - (self.y * other.x)
        }
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

impl<T: ops::Add<T,T>, U>
    ops::Add<Vector3D<T,U>, Vector3D<T,U>>
    for Vector3D<T,U> {

    #[inline]
    fn add(&self, other: &Vector3D<T,U>) -> Vector3D<T, U> {
        return Vector3D {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        };
    }
}

impl<T: ops::Sub<T,T>, U>
    ops::Sub<Vector3D<T,U>, Vector3D<T,U>>
    for Vector3D<T,U> {

    #[inline]
    fn sub(&self, other: &Vector3D<T,U>) -> Vector3D<T, U> {
        return Vector3D {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        };
    }
}

impl<T: ops::Mul<T,T>, U>
    ops::Mul<Vector3D<T,U>, Vector3D<T,U>>
    for Vector3D<T,U> {

    #[inline]
    fn mul(&self, other: &Vector3D<T,U>) -> Vector3D<T, U> {
        return Vector3D {
            x: self.x * other.x,
            y: self.y * other.y,
            z: self.z * other.z,
        };
    }
}

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




impl<T: Copy + Add<T,T> + Sub<T,T> + Mul<T,T>, U> Vector2D<T, U> {
    pub fn new(x: T, y: T, z: T) -> Vector2D<T,U> {
        Vector2D { x: x, y: y}
    }

    pub fn from_slice(from: &[T]) -> Vector2D<T,U> {
        assert!(from.len() >= 2);
        return Vector2D {
            x: from[0],
            y: from[1],
        };
    }

    pub fn as_slice<'l>(&'l self) -> &'l [T] {
        unsafe {
            return cast::transmute((&'l self.x as *T, 4 as uint ));
        }
    }

    #[inline]
    pub fn dot(&self, other: &Vector2D<T,U>) -> T {
        return self.x*other.x + self.y*other.y;
    }

    pub fn xy(&self) -> Vector2D<T,U> { Vector2D { x: self.x, y:self.y } }
    pub fn xz(&self) -> Vector2D<T,U> { Vector2D { x: self.x, y:self.z } }
    pub fn yz(&self) -> Vector2D<T,U> { Vector2D { x: self.y, y:self.z } }
    pub fn yx(&self) -> Vector2D<T,U> { Vector2D { x: self.y, y:self.x } }
}

impl<T: ops::Add<T,T>, U>
    ops::Add<Vector2D<T,U>, Vector2D<T,U>>
    for Vector2D<T,U> {

    #[inline]
    fn add(&self, other: &Vector2D<T,U>) -> Vector2D<T, U> {
        return Vector2D {
            x: self.x + other.x,
            y: self.y + other.y,
        };
    }
}

impl<T: ops::Sub<T,T>, U>
    ops::Sub<Vector2D<T,U>, Vector2D<T,U>>
    for Vector2D<T,U> {

    #[inline]
    fn sub(&self, other: &Vector2D<T,U>) -> Vector2D<T, U> {
        return Vector2D {
            x: self.x - other.x,
            y: self.y - other.y,
        };
    }
}

impl<T: ops::Mul<T,T>, U>
    ops::Mul<Vector2D<T,U>, Vector2D<T,U>>
    for Vector2D<T,U> {

    #[inline]
    fn mul(&self, other: &Vector2D<T,U>) -> Vector2D<T, U> {
        return Vector2D {
            x: self.x * other.x,
            y: self.y * other.y,
        };
    }
}

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

//impl<T: fmt::Show, U> fmt::Show for Vector2D<T, U> {
//    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//        write!(f, "[{} {}]", self.x, self.y)
//    }
//}

#[test]
fn test_vec4() {
    let p1 = vec4(1.0, 2.0, 3.0, 4.0);
    let p2 = -p1;
    let p3 = p1 + p2;
    let d = p1.dot(p2);
    let p4 = p1.cross(p2);
}
