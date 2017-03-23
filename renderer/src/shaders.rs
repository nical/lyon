pub static PRIM_BUFFER_LEN: usize = 1024;

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
