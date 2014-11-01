

pub type BufferData<'l> = data::DynamicallyTypedSlice<'l>;


pub type Handle = u32;
pub const INVALID_HANDLE: Handle = 0;

#[deriving(PartialEq, Clone, Show)]
pub struct ShaderStage { pub handle: Handle }

#[deriving(PartialEq, Clone, Show)]
pub struct Shader { pub handle: Handle }

#[deriving(PartialEq, Clone, Show)]
pub struct Texture { pub handle: Handle }

/// Equivalent of a Buffer object in OpenGL
#[deriving(PartialEq, Clone, Show)]
pub struct Buffer {
    pub handle: Handle,
    pub buffer_type: BufferType
}

/// Equivalent of a VAO in OpenGL
#[deriving(PartialEq, Clone, Show)]
pub struct Geometry {
    pub handle: Handle,
    // To work around some drivers not storing the index buffer
    // binding in the VAO state
    pub ibo: Handle
}

/// Equivalent of a FBO in OpenGL
#[deriving(PartialEq, Clone, Show)]
pub struct RenderTarget { pub handle: Handle }

impl Texture { pub fn invalid_handle() -> Texture { Texture { handle: INVALID_HANDLE } } }
impl Shader { pub fn invalid_handle() -> Shader { Shader { handle: INVALID_HANDLE } } }
impl Buffer { pub fn invalid_handle() -> Buffer { Buffer { handle: INVALID_HANDLE, buffer_type: VERTEX_BUFFER } } }
impl Geometry { pub fn invalid_handle() -> Geometry { Geometry { handle: INVALID_HANDLE, ibo: INVALID_HANDLE } } }
impl ShaderStage { pub fn invalid_handle() -> ShaderStage { ShaderStage { handle: INVALID_HANDLE } } }


#[deriving(PartialEq, Clone, Show)]
pub struct Error {
    pub code: String,
    pub detail: Option<String>,
}

pub type RendererResult = Result<(), Error>;

#[deriving(PartialEq, Clone, Show)]
pub type ErrorCode = u32;

pub trait RenderingContext {
    fn is_supported(&mut self, f: Feature) -> bool;
    fn flush(&mut self);
    fn set_viewport(&mut self, x:i32, y:i32, w:i32, h:i32);
    fn set_clear_color(&mut self, r: f32, g: f32, b: f32, a: f32);
    fn clear(&mut self, targets: TargetTypes);

    fn reset_state(&mut self);

    fn create_texture(&mut self, flags: TextureFlags) -> Texture;
    fn destroy_texture(&mut self, tex: Texture);
    fn set_texture_flags(&mut self, tex: Texture, flags: TextureFlags);
    fn upload_texture_data(&mut self,
        dest: Texture,
        data: &BufferData,
        w:u32, h:u32,
        format: PixelFormat
    ) -> RendererResult;
    /**
     * Specifies the texture's size and format
     * Does not need to be called if some data will be uploaded
     * through upload_texture_data.
     */
    fn allocate_texture(&mut self, dest: Texture,
                        w:u32, h:u32, format: PixelFormat) -> RendererResult;

    fn read_back_texture(&mut self, tex: Texture,
                         format: PixelFormat,
                         dest: &mut [u8]) -> RendererResult;

    fn create_shader_stage(&mut self, t: ShaderType) -> ShaderStage;
    fn destroy_shader_stage(&mut self, s: ShaderStage);
    fn compile_shader_stage(&mut self, shader: ShaderStage, src: &[&str]) -> RendererResult;

    fn create_shader(&mut self) -> Shader;
    fn destroy_shader(&mut self, s: Shader);
    fn link_shader(&mut self, p: Shader, stages: &[ShaderStage],
                   attrib_locations: &[(&str, VertexAttributeLocation)])  -> RendererResult;

    fn create_buffer(&mut self, buffer_type: BufferType) -> Buffer;
    fn destroy_buffer(&mut self, buffer: BufferObject);
    fn upload_buffer(&mut self,
        buffer: Buffer,
        buf_type: BufferType,
        update: UpdateHint,
        data: &BufferData
    ) -> RendererResult;
    fn allocate_buffer(&mut self, dest: Buffer, buf_type: BufferType,
                       update: UpdateHint, size: u32) -> RendererResult;

    fn destroy_geometry(&mut self, geom: Geometry);
    fn create_geometry(&mut self,
                       attributes: &[VertexAttribute],
                       indices: Option<Buffer>) -> Result<Geometry, Error>;

    fn get_shader_input_location(&mut self, program: Shader,
                                 name: &str) -> ShaderInputLocation;
    fn get_vertex_attribute_location(&mut self, program: Shader,
                                     name: &str) -> VertexAttributeLocation;

    fn create_render_target(&mut self,
                            color_attachments: &[Texture],
                            depth: Option<Texture>,
                            stencil: Option<Texture>) -> Result<RenderTarget, Error>;
    fn destroy_render_target(&mut self, fbo: RenderTarget);

    fn set_render_target(&mut self, target: RenderTarget);

    fn get_default_render_target(&mut self) -> RenderTarget;

    fn set_shader(&mut self, program: Shader) -> RendererResult;

    fn set_shader_input_float(&mut self, location: ShaderInputLocation, input: &[f32]);
    fn set_shader_input_int(&mut self, location: ShaderInputLocation, input: &[i32]);
    fn set_shader_input_matrix(&mut self, location: ShaderInputLocation, input: &[f32], dimension: u32, transpose: bool);
    fn set_shader_input_texture(&mut self, location: ShaderInputLocation, texture_unit: u32, input: Texture);

    fn draw(&mut self,
        geom: Geometry,
        range: Range,
        flags: GeometryFlags,
        blend: BlendMode,
        targets: TargetTypes
    ) -> RendererResult;

    fn multi_draw(&mut self,
        geom: Geometry,
        indirect_buffer: Buffer,
        flags: GeometryFlags,
        targets: TargetTypes,
        commands: &[DrawCommand]
    ) -> RendererResult;
}



#[deriving(Clone, Show)]
pub struct DrawCommand {
    pub range: Range,
    pub flags: GeometryFlags,
}
