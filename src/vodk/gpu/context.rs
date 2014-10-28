
use data;
use libc::c_void;
use std::mem;

pub type BufferData<'l> = data::DynamicallyTypedSlice<'l>;

pub type TextureFlags = i32;
pub const REPEAT_S          : TextureFlags = 1 << 0;
pub const REPEAT_T          : TextureFlags = 1 << 1;
pub const REPEAT            : TextureFlags = 1 << (REPEAT_S | REPEAT_T) as uint;
pub const CLAMP_S           : TextureFlags = 1 << 2;
pub const CLAMP_T           : TextureFlags = 1 << 3;
pub const CLAMP             : TextureFlags = 1 << (CLAMP_S | CLAMP_T) as uint;
pub const MIN_FILTER_LINEAR : TextureFlags = 1 << 4;
pub const MAG_FILTER_LINEAR : TextureFlags = 1 << 5;
pub const FILTER_LINEAR     : TextureFlags = MIN_FILTER_LINEAR | MAG_FILTER_LINEAR;
pub const MIN_FILTER_NEAREST: TextureFlags = 1 << 6;
pub const MAG_FILTER_NEAREST: TextureFlags = 1 << 7;
pub const FILTER_NEAREST    : TextureFlags = MIN_FILTER_NEAREST | MAG_FILTER_NEAREST;
pub const FLAGS_DEFAULT     : TextureFlags = CLAMP | FILTER_LINEAR;

// TODO this mixes flags that are about the geometry and flags that are about
// pipeline features. Should proably sperate it.
pub type GeometryFlags = u32;
pub const TRIANGLES             : GeometryFlags = 1 << 3;
pub const LINES                 : GeometryFlags = 1 << 4;
pub const STRIP                 : GeometryFlags = 1 << 5;
pub const LOOP                  : GeometryFlags = 1 << 5;
pub const TRIANGLE_STRIP        : GeometryFlags = TRIANGLES | STRIP;
pub const LINE_STRIP            : GeometryFlags = LINES | STRIP;
pub const LINE_LOOP             : GeometryFlags = LINES | LOOP;

pub type TargetTypes = u32;
pub const COLOR  : TargetTypes = 1 << 0;
pub const DEPTH  : TargetTypes = 1 << 1;
pub const STENCIL: TargetTypes = 1 << 2;

#[deriving(PartialEq, Clone, Show)]
pub enum ShaderType {
    FRAGMENT_SHADER,
    VERTEX_SHADER,
    GEOMETRY_SHADER,
    COMPUTE_SHADER,
}

#[deriving(PartialEq, Clone, Show)]
pub enum Feature {
    FRAGMENT_SHADING,
    VERTEX_SHADING,
    GEOMETRY_SHADING,
    COMPUTE,
    DEPTH_TEXTURE,
    RENDER_TO_TEXTURE,
    MULTIPLE_RENDER_TARGETS,
    INSTANCED_RENDERING,
}

pub type AttributeType = data::Type;

#[deriving(PartialEq, Clone, Show)]
pub enum PixelFormat {
    R8G8B8A8,
    R8G8B8X8,
    B8G8R8A8,
    B8G8R8X8,
    A8,
    A_F32,
}

#[deriving(PartialEq, Clone, Show)]
pub enum UpdateHint {
    STATIC_UPDATE,
    STREAM_UPDATE,
    DYNAMIC_UPDATE,
}

#[deriving(PartialEq, Clone, Show)]
pub enum BufferType {
    VERTEX_BUFFER,
    INDEX_BUFFER,
    UNIFORM_BUFFER,
    DRAW_INDIRECT_BUFFER,
    TRANSFORM_FEEDBACK_BUFFER,
}

#[deriving(PartialEq, Clone, Show)]
pub enum BlendMode {
    NO_BLENDING,
    ALPHA_BLENDING,
    ADD_BLENDING,
    SUB_BLENDING,
    MUL_BLENDING,
}

//#[deriving(PartialEq, Clone, Show)]
//pub struct VertexRange {
//    pub first: u32,
//    pub count: u32,
//}
//
//#[deriving(PartialEq, Clone, Show)]
//pub struct IndexRange {
//    pub first: u32,
//    pub count: u32,
//}

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

pub enum Range {
    VertexRange(u16, u16),
    IndexRange(u16, u16),
}

#[deriving(PartialEq, Clone, Show)]
pub struct Error {
    pub code: String,
    pub detail: Option<String>,
}

pub type RendererResult = Result<(), Error>;

pub type ShaderInputLocation = i16;
pub type VertexAttributeLocation = i16;

#[deriving(PartialEq, Clone, Show)]
pub type ErrorCode = u32;

#[deriving(Clone, Show)]
pub struct VertexAttribute {
    pub buffer: Buffer, // TODO: move out of this struct
    pub attrib_type: AttributeType,
    pub location: VertexAttributeLocation,
    pub stride: u16,
    pub offset: u16,
    pub normalize: bool,
}

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
    fn destroy_buffer(&mut self, buffer: Buffer);
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

#[deriving(Show)]
pub struct SyncObject { pub handle: u32 }
#[deriving(Show)]
pub struct BufferObject {
    pub handle: u32,
    pub size: u32,
    pub buffer_type: BufferType
}
#[deriving(Show)]
pub struct TextureObject { pub handle: u32 }
#[deriving(Show)]
pub struct GeometryObject { pub handle: u32 }
#[deriving(Show)]
pub struct ShaderStageObject { pub handle: u32 }
#[deriving(Show)]
pub struct ShaderPipelineObject { pub handle: u32 }
#[deriving(Show)]
pub struct RenderTargetObject { pub handle: u32 }
pub type BufferFlags = u32;

impl SyncObject { pub fn new() -> SyncObject { SyncObject { handle: 0 } } }
impl TextureObject { pub fn new() -> TextureObject { TextureObject { handle: 0 } } }
impl GeometryObject { pub fn new() -> GeometryObject { GeometryObject { handle: 0 } } }
impl ShaderStageObject { pub fn new() -> ShaderStageObject { ShaderStageObject { handle: 0 } } }
impl ShaderPipelineObject { pub fn new() -> ShaderPipelineObject { ShaderPipelineObject { handle: 0 } } }
impl BufferObject {
    pub fn new() -> BufferObject {
        BufferObject { handle: 0, size: 0, buffer_type: VERTEX_BUFFER }
    }
}

pub struct Device<DeviceBackend> {
    pub backend: DeviceBackend,
}

pub type MapFlags = u8;
pub const READ_MAP      : MapFlags = 1;
pub const WRITE_MAP     : MapFlags = 2;
pub const PERSISTENT_MAP: MapFlags = 3;
pub const COHERENT_MAP  : MapFlags = 4;

pub type ErrorFlags = u8;
pub const IGNORE_ERRORS : ErrorFlags = 0;
pub const LOG_ERRORS    : ErrorFlags = 1;
pub const CRASH_ERRORS  : ErrorFlags = 2;

pub type ResultCode = u32;
pub const OK                            : ResultCode = 0;
pub const UNKNOWN_ERROR                 : ResultCode = 1;
pub const UNKNOWN_COMMAND_ERROR         : ResultCode = 2;
pub const INVALID_ARGUMENT_ERROR        : ResultCode = 3;
pub const OUT_OF_MEMORY_ERROR           : ResultCode = 4;
pub const INVALID_OBJECT_HANDLE_ERROR   : ResultCode = 5;
pub const SHADER_COMPILATION_ERROR      : ResultCode = 6;
pub const SHADER_LINK_ERROR             : ResultCode = 7;
pub const DEVICE_LOST_ERROR             : ResultCode = 8;
pub const RT_MISSING_ATTACHMENT_ERROR   : ResultCode = 16;
pub const RT_INCOMPLETE_ATTACHMENT_ERROR: ResultCode = 17;
pub const RT_UNSUPPORTED_ERROR          : ResultCode = 18;

#[deriving(Show)]
pub struct TextureDescriptor {
    pub format: PixelFormat,
    pub width: u16,
    pub height: u16,
    pub mip_levels: u16,
    pub flags: TextureFlags,
}

#[deriving(Show)]
pub struct BufferDescriptor {
    pub size: u32,
    pub update_hint: UpdateHint,
    pub buffer_type: BufferType,
}

#[deriving(Show)]
pub struct GeometryDescriptor<'l> {
    pub index_buffer: Option<Buffer>,
    pub attributes: &'l[VertexAttribute]
}

#[deriving(Show)]
pub struct ShaderStageDescriptor<'l> {
    pub stage_type: ShaderType,
    pub src: &'l[&'l str],
}

#[deriving(Show)]
pub struct ShaderStageResult {
    pub code: ResultCode,
    pub details: String,
}

#[deriving(Show)]
pub struct ShaderPipelineResult {
    pub code: ResultCode,
    pub details: String,
}

#[deriving(Show)]
pub struct ShaderPipelineDescriptor<'l> {
    pub stages: &'l[ShaderStageObject],
    pub attrib_locations: &'l[(&'l str, VertexAttributeLocation)],
}

#[deriving(Show)]
pub struct RenderTargetDescriptor<'l> {
    pub color_attachments: &'l[TextureObject],
    pub depth: Option<TextureObject>,
    pub stencil: Option<TextureObject>
}

#[deriving(Show)]
pub struct DrawCommand {
    pub first: u16,
    pub count: u16,
}

#[deriving(Show)]
pub enum Command {
    CopyBufferToBuffer(BufferObject, BufferObject),
    CopyBufferToTexture(BufferObject, TextureObject),
    CopyTextureToBuffer(TextureObject, BufferObject),
    CopyTextureToTexture(TextureObject, TextureObject),
    SetGeometry(GeometryObject),
    SetShaderPipeline(ShaderPipelineObject),
    SetRenderTarget(RenderTargetObject),
    Draw(DrawCommand),
    SetViewport(u16, u16, u16, u16),
    Wait(SyncObject),
    Signal(SyncObject),
    SetClearColor(f32, f32, f32, f32),
    Clear(TargetTypes),
    Flush,
}

pub trait DeviceBackend {
    fn is_supported(
        &mut self,
        feature: Feature
    ) -> bool;

    fn execute_command_list(
        &mut self,
        commands: &[Command]
    ) -> ResultCode;

    fn create_texture(&mut self,
        descriptor: &TextureDescriptor,
        output: &mut TextureObject
    ) -> ResultCode;

    fn destroy_texture(
        &mut self,
        tex: TextureObject
    );

    fn set_texture_flags(
        &mut self,
        tex: TextureObject,
        flags: TextureFlags
    ) -> ResultCode;

    fn create_shader_stage(
        &mut self,
        descriptor: &ShaderStageDescriptor,
        shader: &mut ShaderStageObject
    ) -> ResultCode;

    fn get_shader_stage_result(
        &mut self,
        shader: ShaderStageObject,
        result: &mut ShaderStageResult,
    ) -> ResultCode;

    fn destroy_shader_stage(
        &mut self,
        stage: ShaderStageObject
    );

    fn create_shader_pipeline(
        &mut self,
        descriptor: &ShaderPipelineDescriptor,
        shader: &mut ShaderPipelineObject
    ) -> ResultCode;

    fn get_shader_pipeline_result(
        &mut self,
        shader: ShaderPipelineObject,
        result: &mut ShaderPipelineResult,
    ) -> ResultCode;

    fn destroy_shader_pipeline(
        &mut self,
        shader: ShaderPipelineObject
    );

    fn create_buffer(
        &mut self,
        descriptor: &BufferDescriptor,
        buffer: &mut BufferObject,
    ) -> ResultCode;

    fn destroy_buffer(
        &mut self,
        buffer: BufferObject
    );

    unsafe fn map_buffer(
        &mut self,
        buffer: BufferObject,
        target: BufferType,
        flags: MapFlags,
        data: *mut *mut u8
    ) -> ResultCode;

    fn unmap_buffer(
        &mut self,
        buffer: BufferObject
    );

    fn destroy_geometry(
        &mut self,
        geom: GeometryObject
    );

    fn create_geometry(
        &mut self,
        descriptor: &GeometryDescriptor,
        geometry: &mut GeometryObject
    ) -> ResultCode;

    fn get_shader_input_location(
        &mut self,
        shader: ShaderPipelineObject,
        name: &str
    ) -> ShaderInputLocation;

    fn get_vertex_attribute_location(
        &mut self,
        shader: ShaderPipelineObject,
        name: &str
    ) -> VertexAttributeLocation;

    fn create_render_target(
        &mut self,
        descriptor: &RenderTargetDescriptor,
        target: &mut RenderTargetObject,
    ) -> ResultCode;

    fn destroy_render_target(
        &mut self,
        target: RenderTargetObject
    );

    fn get_default_render_target(&mut self) -> RenderTargetObject;
}

impl<Backend: DeviceBackend> Device<Backend> {
    pub fn is_supported(
        &mut self,
        feature: Feature
    ) -> bool {
        return self.backend.is_supported(feature);
    }

    pub fn execute_command_list(
        &mut self,
        commands: &[Command]
    ) -> ResultCode {
        return self.backend.execute_command_list(commands);
    }

    pub fn create_texture(&mut self,
        descriptor: &TextureDescriptor,
        output: &mut TextureObject
    ) -> ResultCode {
        return self.backend.create_texture(descriptor, output);
    }

    pub fn destroy_texture(
        &mut self,
        texture: TextureObject
    ) {
        self.backend.destroy_texture(texture);
    }

    pub fn set_texture_flags(
        &mut self,
        texture: TextureObject,
        flags: TextureFlags
    ) -> ResultCode {
        return self.backend.set_texture_flags(texture, flags);
    }

    pub fn create_shader_stage(
        &mut self,
        descriptor: &ShaderStageDescriptor,
        output: &mut ShaderStageObject
    ) -> ResultCode {
        return self.backend.create_shader_stage(descriptor, output);
    }

    pub fn get_shader_stage_result(
        &mut self,
        shader: ShaderStageObject,
        result: &mut ShaderStageResult,
    ) -> ResultCode {
        return self.backend.get_shader_stage_result(shader, result);
    }

    pub fn destroy_shader_stage(
        &mut self,
        stage: ShaderStageObject
    ) {
        self.backend.destroy_shader_stage(stage);
    }

    pub fn create_shader_pipeline(
        &mut self,
        descriptor: &ShaderPipelineDescriptor,
        output: &mut ShaderPipelineObject
    ) -> ResultCode {
        return self.backend.create_shader_pipeline(descriptor, output);
    }

    pub fn get_shader_pipeline_result(
        &mut self,
        shader: ShaderPipelineObject,
        result: &mut ShaderPipelineResult,
    ) -> ResultCode {
        return self.backend.get_shader_pipeline_result(shader, result);
    }

    pub fn destroy_shader_pipeline(
        &mut self,
        shader: ShaderPipelineObject
    ) {
        self.backend.destroy_shader_pipeline(shader);
    }

    pub fn create_buffer(
        &mut self,
        descriptor: &BufferDescriptor,
        buffer: &mut BufferObject,
    ) -> ResultCode {
        return self.backend.create_buffer(descriptor, buffer);
    }

    pub fn destroy_buffer(
        &mut self,
        buffer: BufferObject
    ) {
        self.backend.destroy_buffer(buffer);
    }

    pub fn map_buffer<T>(
        &mut self,
        buffer: BufferObject,
        target: BufferType,
        flags: MapFlags,
        data: &mut &mut[T]
    ) -> ResultCode {
        unsafe {
            let mut ptr = 0 as *mut u8;
            let result = self.backend.map_buffer(buffer, target, flags, &mut ptr);
            if result != OK {
                return result;
            }
            if ptr == 0 as *mut u8 {
                return UNKNOWN_ERROR;
            }
            *data = mem::transmute((
                ptr,
                buffer.size as uint
            ));
        }
        return OK;
    }

    pub fn unmap_buffer(
        &mut self,
        buffer: BufferObject
    ) {
        self.backend.unmap_buffer(buffer);
    }

    pub fn destroy_geometry(
        &mut self,
        geom: GeometryObject
    ) {
        self.backend.destroy_geometry(geom);
    }

    pub fn create_geometry(
        &mut self,
        descriptor: &GeometryDescriptor,
        geometry: &mut GeometryObject
    ) -> ResultCode {
        return self.backend.create_geometry(descriptor, geometry);
    }

    pub fn get_shader_input_location(
        &mut self,
        shader: ShaderPipelineObject,
        name: &str
    ) -> ShaderInputLocation {
        return self.backend.get_shader_input_location(shader, name);
    }

    pub fn get_vertex_attribute_location(
        &mut self,
        shader: ShaderPipelineObject,
        name: &str
    ) -> VertexAttributeLocation {
        return self.backend.get_vertex_attribute_location(shader, name);
    }

    pub fn create_render_target(
        &mut self,
        descriptor: &RenderTargetDescriptor,
        target: &mut RenderTargetObject,
    ) -> ResultCode {
        return self.backend.create_render_target(descriptor, target);
    }

    pub fn destroy_render_target(
        &mut self,
        target: RenderTargetObject
    ) {
        self.backend.destroy_render_target(target);
    }

    pub fn get_default_render_target(&mut self) -> RenderTargetObject {
        return self.backend.get_default_render_target();
    }
}

pub struct LoggingProxy<Backend> {
    pub backend: Backend,
}

impl<Backend: DeviceBackend> DeviceBackend for LoggingProxy<Backend> {
    fn is_supported(
        &mut self,
        feature: Feature
    ) -> bool {
        println!("device.is_supported({})", feature);
        let result = self.backend.is_supported(feature);
        println!("-> {}", result);
        return result;
    }

    fn execute_command_list(
        &mut self,
        commands: &[Command]
    ) -> ResultCode {
        println!("device.execute_command_list({})", commands);
        let result = self.backend.execute_command_list(commands);
        println!("-> {}", result);
        return result;
    }

    fn create_texture(&mut self,
        descriptor: &TextureDescriptor,
        output: &mut TextureObject
    ) -> ResultCode {
        println!("device.create_texture({})", descriptor);
        let result = self.backend.create_texture(descriptor, output);
        println!("-> {}", result);
        return result;
    }

    fn destroy_texture(
        &mut self,
        texture: TextureObject
    ) {
        println!("device.destroy_texture({})", texture);
        self.backend.destroy_texture(texture);
    }

    fn set_texture_flags(
        &mut self,
        texture: TextureObject,
        flags: TextureFlags
    ) -> ResultCode {
        println!("device.set_texture_flags({}, {})", texture, flags);
        let result = self.backend.set_texture_flags(texture, flags);
        println!("-> {}", result);
        return result;
    }

    fn create_shader_stage(
        &mut self,
        descriptor: &ShaderStageDescriptor,
        output: &mut ShaderStageObject
    ) -> ResultCode {
        println!("device.create_shader_stage({})", descriptor);
        let result = self.backend.create_shader_stage(descriptor, output);
        println!("-> {}", result);
        return result;
    }

    fn get_shader_stage_result(
        &mut self,
        shader: ShaderStageObject,
        result: &mut ShaderStageResult,
    ) -> ResultCode {
        println!("device.get_shader_stage_result({}, [out])", shader);
        let result = self.backend.get_shader_stage_result(shader, result);
        println!("-> {}", result);
        return result;
    }

    fn destroy_shader_stage(
        &mut self,
        stage: ShaderStageObject
    ) {
        println!("device.destroy_shader_stage({})", stage);
        self.backend.destroy_shader_stage(stage);
    }

    fn create_shader_pipeline(
        &mut self,
        descriptor: &ShaderPipelineDescriptor,
        output: &mut ShaderPipelineObject
    ) -> ResultCode {
        println!("device.create_shader_pipeline({}, [out])", descriptor);
        let result = self.backend.create_shader_pipeline(descriptor, output);
        println!("-> {}", result);
        return result;
    }

    fn get_shader_pipeline_result(
        &mut self,
        shader: ShaderPipelineObject,
        result: &mut ShaderPipelineResult,
    ) -> ResultCode {
        println!("device.get_shader_pipeline_result({}, [out])", shader);
        let result = self.backend.get_shader_pipeline_result(shader, result);
        println!("-> {}", result);
        return result;
    }

    fn destroy_shader_pipeline(
        &mut self,
        shader: ShaderPipelineObject
    ) {
        println!("device.destroy_shader_pipeline({})", shader);
        self.backend.destroy_shader_pipeline(shader);
    }

    fn create_buffer(
        &mut self,
        descriptor: &BufferDescriptor,
        buffer: &mut BufferObject,
    ) -> ResultCode {
        println!("device.create_buffer({}, [out])", descriptor);
        let result = self.backend.create_buffer(descriptor, buffer);
        println!("-> {}", result);
        return result;
    }

    fn destroy_buffer(
        &mut self,
        buffer: BufferObject
    ) {
        self.backend.destroy_buffer(buffer);
    }

    fn map_buffer(
        &mut self,
        buffer: BufferObject,
        target: BufferType,
        flags: MapFlags,
        data: *mut *mut u8
    ) -> ResultCode {
        println!("device.map_buffer({}, {}, {}, [out])", buffer, target, flags);
        let result = unsafe {
            self.backend.map_buffer(buffer, target, flags, data)
        };
        println!("-> {}", result);
        return result;
    }

    fn unmap_buffer(
        &mut self,
        buffer: BufferObject
    ) {
        println!("device.unmap_buffer({})", buffer);
        self.backend.unmap_buffer(buffer);
    }

    fn destroy_geometry(
        &mut self,
        geom: GeometryObject
    ) {
        println!("device.destroy_geometry({})", geom);
        self.backend.destroy_geometry(geom);
    }

    fn create_geometry(
        &mut self,
        descriptor: &GeometryDescriptor,
        geometry: &mut GeometryObject
    ) -> ResultCode {
        println!("device.create_geometry({}, [out])", descriptor);
        let result = self.backend.create_geometry(descriptor, geometry);
        println!("-> {}", result);
        return result;
    }

    fn get_shader_input_location(
        &mut self,
        shader: ShaderPipelineObject,
        name: &str
    ) -> ShaderInputLocation {
        println!("device.get_shader_input_location({}, {})", shader, name);
        let result = self.backend.get_shader_input_location(shader, name);
        println!("-> {}", result);
        return result;
    }

    fn get_vertex_attribute_location(
        &mut self,
        shader: ShaderPipelineObject,
        name: &str
    ) -> VertexAttributeLocation {
        println!("get_vertex_attribute_location({}, {})", shader, name);
        let result = self.backend.get_vertex_attribute_location(shader, name);
        println!("-> {}", result);
        return result;
    }

    fn create_render_target(
        &mut self,
        descriptor: &RenderTargetDescriptor,
        target: &mut RenderTargetObject,
    ) -> ResultCode {
        println!("device.create_render_target({}, [out])", descriptor);
        let result = self.backend.create_render_target(descriptor, target);
        println!("-> {}", result);
        return result;
    }

    fn destroy_render_target(
        &mut self,
        target: RenderTargetObject
    ) {
        println!("device.destroy_render_target({})", target);
        self.backend.destroy_render_target(target);
    }

    fn get_default_render_target(&mut self) -> RenderTargetObject {
        println!("device.get_default_render_target()");
        let result = self.backend.get_default_render_target();
        println!("-> {}", result);
        return result;
    }
}
