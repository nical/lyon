pub static PRIM_BUFFER_LEN: usize = 1024;

pub static PRIM_BUFFER_LOCATION: u32 = 0;
pub static TRANSFORM_BUFFER_LOCATION: u32 = 1;

pub static FILL_PRIM_BUFFER_DECL: &'static str = &"
    struct Primitive {
        int z_index;
        int color;
        int local_transform;
        int view_transform;
    };
    layout(std430, location = 0) buffer _primitives {
        Primitive primitives[];
    };
";

pub static STROKE_PRIM_BUFFER_DECL: &'static str = &"
    struct Primitive {
        int z_index;
        int color;
        int local_transform;
        int view_transform;
        float width;
        float _padding0;
        float _padding1;
        float _padding2;
    };
    layout(std430, location = 0) buffer _primitives { Primitive primitives[]; };
";

pub static TRANFORM2D_BUFFER_DECL: &'static str = &"
    struct Transform2D {
        float m11, float m12,
        float m21, float m22,
        float m31, float m32,
    };
    layout(std430, location = 1) buffer _transforms { Transform2D transforms[]; };
    mat3 get_transform(int index) {
        Transform2D t = transforms[index];
        return mat3(
            t.m11, t.m12, 0.0
            t.m21, t.m22, 0.0
            t.m31, t.m32, 1.0
        );
    }
";

pub static TRANFORM3D_BUFFER_DECL: &'static str = &"
    layout(location = 1) buffer _transforms { mat4 transforms[]; };
    mat4 get_transform(int index) {
        return transforms[index];
    }
";

pub static VERTEX_ATRIBUTES_DECL: &'static str = &"
    in vec2 a_position;
    in vec2 a_normal;
    in int a_prim_id;
    in int a_advancement;
";

// The vertex shader for the tessellated geometry.
// The transform, color and stroke width are applied instead of during tessellation. This makes
// it possible to change these parameters without having to modify/upload the geometry.
// Per-prim data is stored in uniform buffer objects to keep the vertex buffer small.
pub static FILL_VERTEX_SHADER: &'static str = &"
    #version 140
    #line 266

    #define PRIM_BUFFER_LEN 64

    uniform Globals {
        vec2 u_resolution;
    };

    struct GpuTransform { mat4 transform; };
    uniform u_transforms { GpuTransform transforms[PRIM_BUFFER_LEN]; };

    struct Primitive {
        vec4 color;
        float z_index;
        int local_transform;
        int view_transform;
        float width;
    };
    uniform u_primitives { Primitive primitives[PRIM_BUFFER_LEN]; };

    in vec2 a_position;
    in vec2 a_normal;
    in int a_prim_id;

    out vec4 v_color;

    void main() {
        int id = a_prim_id + gl_InstanceID;
        Primitive prim = primitives[id];

        vec4 local_pos = vec4(a_position + a_normal * prim.width, 0.0, 1.0);
        vec4 world_pos = transforms[prim.view_transform].transform
            * transforms[prim.local_transform].transform
            * local_pos;

        vec2 transformed_pos = world_pos.xy / (vec2(0.5, -0.5) * u_resolution * world_pos.w);

        gl_Position = vec4(transformed_pos, 1.0 - prim.z_index, 1.0);
        v_color = prim.color;
    }
";

pub static STROKE_VERTEX_SHADER: &'static str = &"
    #version 140
    #line 53

    #define PRIM_BUFFER_LEN 64

    uniform Globals {
        vec2 u_resolution;
    };

    struct GpuTransform { mat4 transform; };
    uniform u_transforms { GpuTransform transforms[PRIM_BUFFER_LEN]; };

    struct Primitive {
        vec4 color;
        float z_index;
        int local_transform;
        int view_transform;
        float width;
    };
    uniform u_primitives { Primitive primitives[PRIM_BUFFER_LEN]; };

    in vec2 a_position;
    in vec2 a_normal;
    in float a_advancement;
    in int a_prim_id;

    out vec4 v_color;
    out float v_advancement;

    void main() {
        int id = a_prim_id + gl_InstanceID;
        Primitive prim = primitives[id];

        vec4 local_pos = vec4(a_position + a_normal * prim.width, 0.0, 1.0);
        vec4 world_pos = transforms[prim.view_transform].transform
            * transforms[prim.local_transform].transform
            * local_pos;

        vec2 transformed_pos = world_pos.xy / (vec2(0.5, -0.5) * u_resolution * world_pos.w);

        gl_Position = vec4(transformed_pos, 1.0 - prim.z_index, 1.0);
        v_color = prim.color;
        v_advancement = a_advancement;
    }
";

// The fragment shader is dead simple. It just applies the color computed in the vertex shader.
// A more advanced renderer would probably compute texture coordinates in the vertex shader and
// sample the color from a texture here.
pub static FILL_FRAGMENT_SHADER: &'static str = &"
    #version 140
    in vec4 v_color;
    out vec4 out_color;

    void main() {
        out_color = v_color;
    }
";

pub static STROKE_FRAGMENT_SHADER: &'static str = &"
    #version 140
    in vec4 v_color;
    in float v_advancement;
    out vec4 out_color;

    void main() {
        //float a = mod(v_advancement * 1.0, 1.0);
        //out_color = vec4(a, a, a, 1.0);
        out_color = v_color;
    }
";
