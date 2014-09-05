
macro_rules! declare_unit (
    ($module:ident) => (
        pub mod $module {
            use math::vector;

            #[deriving(Clone, Show)]
            pub struct Unit;

            pub type Rectangle = vector::Rectangle2D<Unit>;
            pub type Vec2 = vector::Vector2D<Unit>;
            pub type Vec3 = vector::Vector3D<Unit>;
            pub type Vec4 = vector::Vector4D<Unit>;
            pub type Mat4 = vector::Matrix4D<Unit>;
            pub type Mat3 = vector::Matrix3D<Unit>;
            pub type Mat2 = vector::Matrix2D<Unit>;

            pub fn rect(x: f32, y: f32, w: f32, h: f32) -> Rectangle {
                vector::Rectangle2D { x: x, y: y, w: w, h: h }
            }

            pub fn vec2(x: f32, y: f32) -> Vec2 {
                vector::Vector2D { x: x, y: y }
            }

            pub fn vec3(x: f32, y: f32, z: f32) -> Vec3 {
                vector::Vector3D { x: x, y: y, z: z }
            }

            pub fn vec4(x: f32, y: f32, z: f32, w: f32) -> Vec4 {
                vector::Vector4D { x: x, y: y, z: z, w: w }
            }

            pub fn mat2(a11: f32, a21: f32, a12: f32, a22:f32) -> Mat2 {
                vector::Matrix2D {
                    _11: a11, _21: a21,
                    _12: a12, _22: a22
                }
            }

            pub fn mat3(
                a11: f32, a21: f32, a31: f32,
                a12: f32, a22: f32, a32: f32,
                a13: f32, a23: f32, a33: f32
            ) -> Mat3 {
                vector::Matrix3D {
                    _11: a11, _21: a21, _31: a31,
                    _12: a12, _22: a22, _32: a32,
                    _13: a13, _23: a23, _33: a33,
                }
            }

            pub mod Mat4 {
                use super::Mat4;
                use math::vector;
                use std::num::One;
                pub fn identity() -> Mat4 { One::one() }
                pub fn perspective(
                    fovy: f32, aspect: f32,
                    near: f32, far: f32,
                    mat: &mut Mat4
                ) {
                    vector::Matrix4D::perspective(fovy, aspect, near, far, mat);
                }
            }

            pub mod Mat3 {
                use super::Mat3;
                use math::vector;
                use std::num::One;
                pub fn identity() -> Mat3 { One::one() }
            }

            pub mod Mat2 {
                use super::Mat2;
                use math::vector;
                use std::num::One;
                pub fn identity() -> Mat2 { One::one() }
            }
        }
    )
)

// In texture space (0 .. 1) origin: top-left
declare_unit!(texels)
// In ui units (0 .. X) origin: top-left
declare_unit!(pixels)
// In world space (-X .. Y)
declare_unit!(world)
