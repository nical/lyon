
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

#[deriving(Clone, Show)]
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
    pub buffer: BufferObject,
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

#[deriving(Show, Clone, PartialEq)]
pub struct SyncObject { pub handle: u32 }
#[deriving(Show, Clone, PartialEq)]
pub struct BufferObject {
    pub handle: u32,
    pub size: u32,
    pub buffer_type: BufferType
}
#[deriving(Show, Clone, PartialEq)]
pub struct TextureObject { pub handle: u32 }
#[deriving(Show, Clone, PartialEq)]
pub struct GeometryObject { pub handle: u32 }
#[deriving(Show, Clone, PartialEq)]
pub struct ShaderStageObject { pub handle: u32 }
#[deriving(Show, Clone, PartialEq)]
pub struct ShaderPipelineObject { pub handle: u32 }
#[deriving(Show, Clone, PartialEq)]
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

#[deriving(Clone, PartialEq, Show)]
pub enum ResultCode {
    OK,
    UNKNOWN_ERROR,
    UNKNOWN_COMMAND_ERROR,
    INVALID_ARGUMENT_ERROR,
    OUT_OF_MEMORY_ERROR,
    INVALID_OBJECT_HANDLE_ERROR,
    SHADER_COMPILATION_ERROR,
    SHADER_LINK_ERROR,
    DEVICE_LOST_ERROR,
    RT_MISSING_ATTACHMENT_ERROR,
    RT_INCOMPLETE_ATTACHMENT_ERROR,
    RT_UNSUPPORTED_ERROR,
}

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
    pub attributes: &'l[VertexAttribute],
    pub index_buffer: Option<Buffer>,
}

#[deriving(Show)]
pub struct ShaderStageDescriptor<'l> {
    pub stage_type: ShaderType,
    pub src: &'l[&'l str],
}

#[deriving(Show)]
pub struct ShaderBuildResult {
    pub code: ResultCode,
    pub details: String,
}

impl ShaderBuildResult {
    pub fn new() -> ShaderBuildResult { ShaderBuildResult { code: OK, details: String::new() } }
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

#[deriving(Clone, Show)]
pub struct DrawCommand {
    pub range: Range,
    pub flags: GeometryFlags,
}

pub trait DeviceBackend {
    fn is_supported(
        &mut self,
        feature: Feature
    ) -> bool;

    fn set_viewport(&mut self, x:i32, y:i32, w:i32, h:i32);

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
        result: &mut ShaderBuildResult,
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
        result: &mut ShaderBuildResult,
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

    fn copy_buffer_to_texture(
        &mut self,
        buffer: BufferObject,
        texture: TextureObject
    ) -> ResultCode;

    fn copy_texture_to_buffer(
        &mut self,
        texture: TextureObject,
        buffer: BufferObject
    ) -> ResultCode;

    fn copy_buffer_to_buffer(
        &mut self,
        src_buffer: BufferObject,
        dest_buffer: BufferObject,
        src_offset: u16,
        dest_offset: u16,
        size: u16
    ) -> ResultCode;

    fn set_shader(
        &mut self,
        pipeline: ShaderPipelineObject
    ) -> ResultCode;

    fn draw(&mut self,
        geom: GeometryObject,
        range: Range,
        flags: GeometryFlags,
        blend: BlendMode,
        targets: TargetTypes
    ) -> ResultCode;

    fn flush(&mut self) -> ResultCode;

    fn clear(&mut self, targets: TargetTypes) -> ResultCode;

    fn set_clear_color(&mut self, r:f32, g: f32, b: f32, a: f32);
}

impl<Backend: DeviceBackend> Device<Backend> {
    pub fn is_supported(
        &mut self,
        feature: Feature
    ) -> bool {
        return self.backend.is_supported(feature);
    }

    pub fn set_viewport(&mut self, x:i32, y:i32, w:i32, h:i32) {
        self.backend.set_viewport(x, y, w, h);
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
        result: &mut ShaderBuildResult,
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
        result: &mut ShaderBuildResult,
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

    pub fn copy_buffer_to_texture(
        &mut self,
        buffer: BufferObject,
        texture: TextureObject
    ) -> ResultCode {
        return self.backend.copy_buffer_to_texture(buffer, texture);
    }

    pub fn copy_texture_to_buffer(
        &mut self,
        texture: TextureObject,
        buffer: BufferObject
    ) -> ResultCode {
        return self.backend.copy_texture_to_buffer(texture, buffer);
    }

    pub fn set_shader(
        &mut self,
        pipeline: ShaderPipelineObject
    ) -> ResultCode {
        return self.backend.set_shader(pipeline);
    }

    pub fn draw(&mut self,
        geom: GeometryObject,
        range: Range,
        flags: GeometryFlags,
        blend: BlendMode,
        targets: TargetTypes
    ) -> ResultCode {
        return self.backend.draw(geom, range, flags, blend, targets);
    }

    pub fn flush(&mut self) -> ResultCode {
        return self.backend.flush();
    }

    pub fn clear(&mut self, targets: TargetTypes) -> ResultCode {
        return self.backend.clear(targets);
    }

    pub fn set_clear_color(&mut self, r:f32, g: f32, b: f32, a: f32) {
        self.backend.set_clear_color(r, g, b, a);
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

    fn set_viewport(
        &mut self,
        x: i32, y: i32,
        w: i32, h: i32
    ) {
        println!("device.set_viewport({}, {}, {}, {})", x, y, w, h);
        self.backend.set_viewport(x, y, w, h);
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
        result: &mut ShaderBuildResult,
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
        result: &mut ShaderBuildResult,
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

    fn copy_buffer_to_texture(
        &mut self,
        buffer: BufferObject,
        texture: TextureObject
    ) -> ResultCode {
        println!("device.copy_buffer_to_texture({}, {})", buffer, texture);
        let result = self.backend.copy_buffer_to_texture(buffer, texture);
        println!("-> {}", result);
        return result;
    }

    fn copy_texture_to_buffer(
        &mut self,
        texture: TextureObject,
        buffer: BufferObject
    ) -> ResultCode {
        println!("device.copy_texture_to_buffer({}, {})", texture, buffer);
        let result = self.backend.copy_texture_to_buffer(texture, buffer);
        println!("-> {}", result);
        return result;
    }

    fn copy_buffer_to_buffer(
        &mut self,
        src_buffer: BufferObject,
        dest_buffer: BufferObject,
        src_offset: u16,
        dest_offset: u16,
        size: u16
    ) -> ResultCode {
        println!(
            "device.copy_buffer_to_buffer({}, {}, {}, {}, {})",
            src_buffer, dest_buffer, src_offset, dest_offset, size
        );
        let result = self.backend.copy_buffer_to_buffer(
            src_buffer, dest_buffer, src_offset, dest_offset, size
        );
        println!("-> {}", result);
        return result;
    }

    fn set_shader(
        &mut self,
        pipeline: ShaderPipelineObject
    ) -> ResultCode {
        println!("device.set_shader({})", pipeline);
        let result = self.backend.set_shader(pipeline);
        println!("-> {}", result);
        return result;
    }

    fn draw(&mut self,
        geom: GeometryObject,
        range: Range,
        flags: GeometryFlags,
        blend: BlendMode,
        targets: TargetTypes
    ) -> ResultCode {
        println!(
            "device.draw({}, {}, {}, {}, {})",
            geom, range, flags, blend, targets
        );
        let result = self.backend.draw(geom, range, flags, blend, targets);
        println!("-> {}", result);
        return result;
    }

    fn flush(&mut self) -> ResultCode {
        println!("device.flush()");
        let result = self.backend.flush();
        println!("-> {}", result);
        return result;
    }

    fn clear(&mut self, targets: TargetTypes) -> ResultCode {
        println!("device.clear({})", targets);
        let result = self.backend.clear(targets);
        println!("-> {}", result);
        return result;
    }

    fn set_clear_color(&mut self, r:f32, g: f32, b: f32, a: f32) {
        println!("device.set_clear_color({}, {}, {}, {}) -> ()", r, g, b, a);
        self.backend.set_clear_color(r, g, b, a);
    }
}
