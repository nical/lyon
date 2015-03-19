
macro_rules! declare_unit (
    ($module:ident) => (
        pub mod $module {
            use vector;
            use matrix;

            #[derive(Copy, Clone, Debug)]
            pub struct Unit;

            pub type Rectangle = vector::Rectangle<Unit>;
            pub type Vec2 = vector::Vector2D<Unit>;
            pub type Vec3 = vector::Vector3D<Unit>;
            pub type Vec4 = vector::Vector4D<Unit>;
            pub type Mat4 = matrix::Matrix4x4<Unit>;
            pub type Mat3 = matrix::Matrix3x3<Unit>;
            pub type Mat2 = matrix::Matrix2x2<Unit>;

            pub fn rect(x: f32, y: f32, w: f32, h: f32) -> Rectangle {
                vector::Rectangle::new(x, y, w, h)
            }

            pub fn vec2(x: f32, y: f32) -> Vec2 {
                vector::Vector2D::new(x, y)
            }

            pub fn vec3(x: f32, y: f32, z: f32) -> Vec3 {
                vector::Vector3D::new(x, y, z)
            }

            pub fn vec4(x: f32, y: f32, z: f32, w: f32) -> Vec4 {
                vector::Vector4D::new(x, y, z, w)
            }

            pub fn mat2(a11: f32, a21: f32, a12: f32, a22:f32) -> Mat2 {
                matrix::Matrix2x2::new(
                    a11, a21,
                    a12, a22
                )
            }

            pub fn mat3(
                a11: f32, a21: f32, a31: f32,
                a12: f32, a22: f32, a32: f32,
                a13: f32, a23: f32, a33: f32
            ) -> Mat3 {
                matrix::Matrix3x3::new(
                    a11, a21, a31,
                    a12, a22, a32,
                    a13, a23, a33
                )
            }

            pub fn mat4(
                a11: f32, a21: f32, a31: f32, a41: f32,
                a12: f32, a22: f32, a32: f32, a42: f32,
                a13: f32, a23: f32, a33: f32, a43: f32,
                a14: f32, a24: f32, a34: f32, a44: f32
            ) -> Mat4 {
                matrix::Matrix4x4::new(
                    a11, a21, a31, a41,
                    a12, a22, a32, a42,
                    a13, a23, a33, a43,
                    a14, a24, a34, a44
                )
            }
        }
    )
);

// In texture space (0 .. 1) origin: top-left
declare_unit!(texels);
// In ui units (0 .. X) origin: top-left
declare_unit!(pixels);
// In world space (-X .. Y)
declare_unit!(world);
