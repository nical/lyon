
pub struct Vec2 { x: f32, y: f32 }
pub struct Vec3 { x: f32, y: f32, z: f32 }
pub struct Vec4 { x: f32, y: f32, z: f32, w: f32 }

pub fn vec2(x: f32, y: f32) -> Vec2 { Vec2 { x: x, y: y } }
pub fn vec3(x: f32, y: f32, z:f32) -> Vec3 { Vec3 { x: x, y: y, z: z } }
pub fn vec4(x: f32, y: f32, z:f32, w: f32) -> Vec4 { Vec4{ x: x, y: y, z: z, w: w } }
