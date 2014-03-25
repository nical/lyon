
pub type TextureFlags = i32;
pub static TEXTURE_REPEAT_S          : TextureFlags = 1;
pub static TEXTURE_REPEAT_T          : TextureFlags = 2;
pub static TEXTURE_REPEAT            : TextureFlags = 3;
pub static TEXTURE_CLAMP_S           : TextureFlags = 4;
pub static TEXTURE_CLAMP_T           : TextureFlags = 8;
pub static TEXTURE_CLAMP             : TextureFlags = 12;
pub static TEXTURE_MIN_FILTER_LINEAR : TextureFlags = 16;
pub static TEXTURE_MAG_FILTER_LINEAR : TextureFlags = 32;
pub static TEXTURE_FILTER_LINEAR     : TextureFlags = 48;
pub static TEXTURE_MIN_FILTER_NEAREST: TextureFlags = 64;
pub static TEXTURE_MAG_FILTER_NEAREST: TextureFlags = 128;
pub static TEXTURE_SAMPLE_NEAREST    : TextureFlags = 192;
pub static TEXTURE_FLAGS_DEFAULT     : TextureFlags = TEXTURE_CLAMP|TEXTURE_FILTER_LINEAR;

#[deriving(Eq, Clone, Show)]
pub enum BackendType {
    GL_BACKEND,
    INVALID_NACKEND,
}

#[deriving(Eq, Clone, Show)]
pub enum ShaderType {
    FRAGMENT_SHADER,
    VERTEX_SHADER,
    GEOMETRY_SHADER,
}

#[deriving(Eq, Clone, Show)]
pub enum DrawMode {
    LINES,
    LINE_LOOP,
    LINE_STRIP,
    TRIANGLES,
    TRIANGLE_STRIP
}

#[deriving(Eq, Clone, Show)]
pub enum Feature {
    FRAGMENT_SHADING,
    VERTEX_SHADING,
    GEOMETRY_SHADING,
    RENDER_TO_TEXTURE,
    MULTIPLE_RENDER_TARGETS,
    INSTANCED_RENDERING,
}

#[deriving(Eq, Clone, Show)]
pub enum AttributeType {
    F32,
    F64,
    I32,
    U32,
}

#[deriving(Eq, Clone, Show)]
pub enum PixelFormat {
    FORMAT_R8G8B8A8,
    FORMAT_R8G8B8X8,
    FORMAT_B8G8R8A8,
    FORMAT_B8G8R8X8,
    FORMAT_A8,
}

#[deriving(Eq, Clone, Show)]
pub enum UpdateHint {
    STATIC_UPDATE,
    STREAM_UPDATE,
    DYNAMIC_UPDATE
}

type Handle = u32;

#[deriving(Eq, Clone, Show)]
pub struct Shader { handle: Handle }
#[deriving(Eq, Clone, Show)]
pub struct ShaderProgram { handle: Handle }
#[deriving(Eq, Clone, Show)]
pub struct Texture { handle: Handle }
#[deriving(Eq, Clone, Show)]
pub struct VertexBuffer { handle: Handle }
#[deriving(Eq, Clone, Show)]
pub struct ElementBuffer { handle: Handle }
#[deriving(Eq, Clone, Show)]
pub struct RenderTarget { handle: Handle }

#[deriving(Eq, Clone, Show)]
pub struct GeometryRange {
    vertices: VertexBuffer,
    elements: ElementBuffer,
    first: i32,
    count: i32,
    layout: ~[VertexAttribute],
}

impl GeometryRange {
    pub fn new(vertices: VertexBuffer, count: i32) -> GeometryRange {
        GeometryRange {
            vertices: vertices,
            elements: ElementBuffer { handle: 0 },
            first: 0,
            count: count,
            layout: ~[],
        }
    }

    pub fn new_with_elements(vertices: VertexBuffer,
                             elements: ElementBuffer,
                             count: i32) -> GeometryRange {
        GeometryRange {
            vertices: vertices,
            elements: elements,
            first: 0,
            count: count,
            layout: ~[],
        }
    }

    pub fn add_vertex_attribute(&mut self, t: AttributeType,
                                components: i32,
                                stride: i32,
                                offset: i32) {
        self.layout.push(VertexAttribute{
            attrib_type: t,
            components: components,
            stride: stride,
            offset: offset,
        });
    }
}

#[deriving(Eq, Clone, Show)]
pub struct VertexAttribute {
    attrib_type: AttributeType,
    components: i32,
    stride: i32,
    offset: i32,
}

#[deriving(Eq, Clone, Show)]
pub enum ShaderInputValue {
    INPUT_FLOATS(~[f32]),
    INPUT_TEXTURE(Texture),
}

#[deriving(Eq, Clone, Show)]
pub struct ShaderInput {
    location: i32,
    value: ShaderInputValue,
}

#[deriving(Eq, Clone, Show)]
pub struct DrawCommand {
    mode: DrawMode,
    target: RenderTarget,
    geometry: GeometryRange,
    shader_program: ShaderProgram,
    shader_inputs: ~[ShaderInput],
}

pub enum RenderingCommand {
    OpDraw(DrawCommand),
    OpFlush,
    OpClear,
}

pub trait RenderingContext {
    fn is_supported(&mut self, f: Feature) -> bool;
    fn flush(&mut self);
    fn set_viewport(&mut self, x:i32, y:i32, w:i32, h:i32);
    fn set_clear_color(&mut self, r: f32, g: f32, b: f32, a: f32);
    fn clear(&mut self);

    fn reset_state(&mut self);

    fn make_current(&mut self) -> bool;
    fn check_error(&mut self) -> Option<~str>;

    fn create_texture(&mut self) -> Texture;
    fn destroy_texture(&mut self, tex: Texture);
    fn set_texture_flags(&mut self, tex: Texture, flags: TextureFlags);
    fn upload_texture_data(&mut self, dest: Texture,
                           data: &[u8], format: PixelFormat,
                           w:u32, h:u32, stride: u32) -> bool;
    /**
     * Specifies the texture's size and format
     * Does not need to be called if some data will be uploaded
     * through upload_texture_data.
     */
    fn allocate_texture(&mut self, dest: Texture,
                        format: PixelFormat,
                        w:u32, h:u32, stride: u32) -> bool;

    fn create_shader(&mut self, t: ShaderType) -> Shader;
    fn destroy_shader(&mut self, s: Shader);
    fn compile_shader(&mut self, shader: Shader, src: &str) -> Result<Shader, ~str>;

    fn create_shader_program(&mut self) -> ShaderProgram;
    fn destroy_shader_program(&mut self, s: ShaderProgram);
    fn link_shader_program(&mut self, p: ShaderProgram, shaders: &[Shader]) -> Result<ShaderProgram, ~str>;

    fn create_vertex_buffer(&mut self) -> VertexBuffer;
    fn destroy_vertex_buffer(&mut self, buffer: VertexBuffer);
    fn upload_vertex_data(&mut self, buffer: VertexBuffer,
                          data: &[f32], update: UpdateHint);

    fn get_shader_input_location(&mut self, program: ShaderProgram, name: &str) -> i32;

    fn define_vertex_attribute(attrib_index: u32,
                               attrib_type: AttributeType,
                               component_per_vertex: i32,
                               stride: i32, // zero means tightly packed attributes
                               offset: i32);

    fn create_render_target(&mut self,
                            color_attachments: &[Texture],
                            depth: Option<Texture>,
                            stencil: Option<Texture>) -> Result<RenderTarget, ~str>;
    fn destroy_render_target(&mut self, fbo: RenderTarget);

    fn get_default_render_target(&mut self) -> RenderTarget;

    fn render(&mut self, commands: &[RenderingCommand]);
}

impl RenderTarget {
    fn default() -> RenderTarget { RenderTarget { handle: 0 } }
}
