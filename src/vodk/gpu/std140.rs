
#[repr(C)]
#[derive(Clone, Show)]
pub struct Mat2 {
    pub _11: f32, pub _21: f32,
    pub _12: f32, pub _22: f32,
}

#[repr(C)]
#[derive(Clone, Show)]
pub struct Mat3 {
    pub _11: f32, pub _21: f32, pub _31: f32, pub _pad1: u32,
    pub _12: f32, pub _22: f32, pub _32: f32, pub _pad2: u32,
    pub _13: f32, pub _23: f32, pub _33: f32, pub _pad3: u32,
}

#[repr(C)]
#[derive(Clone, Show)]
pub struct Mat4 {
    pub _11: f32, pub _21: f32, pub _31: f32, pub _pad1: u32,
    pub _12: f32, pub _22: f32, pub _32: f32, pub _pad2: u32,
    pub _13: f32, pub _23: f32, pub _33: f32, pub _pad3: u32,
    pub _14: f32, pub _24: f32, pub _34: f32, pub _pad4: u32,
}

pub type Float = f32;

#[repr(C)]
#[derive(Clone, Show)]
pub struct Vec2 {
    pub x: f32, pub y: f32
}

#[repr(C)]
#[derive(Clone, Show)]
pub struct Vec3 {
    pub x: f32, pub y: f32, pub z: f32, pub pad: u32
}

#[repr(C)]
#[derive(Clone, Show)]
pub struct Vec4 {
    pub x: f32, pub y: f32, pub z: f32, pub w: f32
}
