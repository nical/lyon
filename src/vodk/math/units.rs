
macro_rules! declare_unit (
    ($module:ident) => (
        pub mod $module {
            use math::vector;

            pub struct Unit;
            pub type Rect = vector::Rectangle2D<f32, Unit>;
            pub type Vec2 = vector::Vector2D<f32, Unit>;
            pub type Vec3 = vector::Vector3D<f32, Unit>;
            pub type Vec4 = vector::Vector4D<f32, Unit>;
            pub type Mat4 = vector::Matrix4D<f32, Unit>;

            pub fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
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

            pub mod Mat4 {
                use super::Mat4;
                use math::vector;
                pub fn identity() -> Mat4 {
                    vector::Matrix4D {
                        _11: 1.0, _21: 0.0, _31: 0.0, _41: 0.0,
                        _12: 0.0, _22: 1.0, _32: 0.0, _42: 0.0,
                        _13: 0.0, _23: 0.0, _33: 1.0, _43: 0.0,
                        _14: 0.0, _24: 0.0, _34: 0.0, _44: 1.0,
                    }
                }

                pub fn perspective(
                    fovy: f32, aspect: f32,
                    near: f32, far: f32,
                    mat: &mut Mat4
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
        }
    )
)

// In texture space (0 .. 1) origin: top-left
declare_unit!(texels)
// In ui units (0 .. X) origin: top-left
declare_unit!(pixels)
// In world space (-X .. Y)
declare_unit!(world)
