#![allow(non_snake_case)]

pub mod units;
pub mod vec2;
pub mod vec3;
pub mod vec4;
pub mod matrix;
pub mod constants;

pub use vec2::{ Vector2D, Vec2, vec2, Rectangle, Size2D, IntVector2D, IntRectangle, IntSize2D, Rect };
pub use vec3::{ Vector3D, Vec3, vec3 };
pub use vec4::{ Vector4D, Vec4, vec4 };
pub use matrix::{ Matrix2x2, Matrix3x3, Matrix4x4, Mat2, Mat3, Mat4 };
pub use units::{ Unit, Untyped };
