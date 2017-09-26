use gl;
use std::mem;
use gl::types::*;
use std::ffi::CString;
use std;
use gpu_data::{DataType, ScalarType, GpuFillVertex};
use glsl;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Texture { pub handle: GLuint }

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Buffer {
    pub handle: GLuint,
    pub buffer_type: BufferType,
}

impl Buffer {
    fn view_as(&self, buffer_type: BufferType) -> Buffer {
        Buffer {
            handle: self.handle,
            buffer_type,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Framebuffer { pub handle: GLuint }

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Geometry { pub handle: GLuint }

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ShaderPipeline { pub handle: GLuint }

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ShaderStage { pub handle: GLuint }

#[derive(Debug, Clone)]
pub struct TextureDescriptor {
    pub format: PixelFormat,
    pub width: u16,
    pub height: u16,
    pub mip_levels: u16,
    pub flags: TextureFlags,
}

#[derive(Copy, Clone, Debug)]
pub struct BufferDescriptor {
    pub size: u32,
    pub update_hint: UpdateHint,
    pub buffer_type: BufferType,
}

#[derive(Copy, Clone, Debug)]
pub enum Range {
    VertexRange(u16, u16),
    IndexRange(u16, u16),
}

pub type UniformBindingIndex = i32;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct UniformBlockLocation { pub index: i16 }
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct VertexAttributeLocation { pub index: i16 }

#[derive(Copy, Clone, Debug)]
pub struct VertexAttribute {
    pub buffer: Buffer,
    pub attrib_type: DataType,
    pub location: VertexAttributeLocation,
    pub stride: u16,
    pub offset: u16,
    pub normalize: bool,
}

pub type TargetTypes = u32;
pub const COLOR  : TargetTypes = 1 << 0;
pub const DEPTH  : TargetTypes = 1 << 1;
pub const STENCIL: TargetTypes = 1 << 2;

pub type GeometryFlags = u32;
pub const TRIANGLES             : GeometryFlags = 1 << 3;
pub const LINES                 : GeometryFlags = 1 << 4;
pub const STRIP                 : GeometryFlags = 1 << 5;
pub const LOOP                  : GeometryFlags = 1 << 5;
pub const TRIANGLE_STRIP        : GeometryFlags = TRIANGLES | STRIP;
pub const LINE_STRIP            : GeometryFlags = LINES | STRIP;
pub const LINE_LOOP : GeometryFlags = LINES | LOOP;

pub type TextureFlags = i32;
pub const REPEAT_S          : TextureFlags = 1 << 0;
pub const REPEAT_T          : TextureFlags = 1 << 1;
pub const REPEAT            : TextureFlags = 1 << (REPEAT_S | REPEAT_T) as usize;
pub const CLAMP_S           : TextureFlags = 1 << 2;
pub const CLAMP_T           : TextureFlags = 1 << 3;
pub const CLAMP             : TextureFlags = 1 << (CLAMP_S | CLAMP_T) as usize;
pub const MIN_FILTER_LINEAR : TextureFlags = 1 << 4;
pub const MAG_FILTER_LINEAR : TextureFlags = 1 << 5;
pub const FILTER_LINEAR     : TextureFlags = MIN_FILTER_LINEAR | MAG_FILTER_LINEAR;
pub const MIN_FILTER_NEAREST: TextureFlags = 1 << 6;
pub const MAG_FILTER_NEAREST: TextureFlags = 1 << 7;
pub const FILTER_NEAREST    : TextureFlags = MIN_FILTER_NEAREST | MAG_FILTER_NEAREST;
pub const FLAGS_DEFAULT : TextureFlags = CLAMP | FILTER_LINEAR;

pub mod map {
    pub const READ       : Flags = Flags(READ_BIT);
    pub const WRITE      : Flags = Flags(WRITE_BIT);
    pub const READ_WRITE : Flags = Flags(READ_BIT | WRITE_BIT);
    pub const PERSISTENT : Flags = Flags(PERSISTENT_BIT);
    pub const COHERENT   : Flags = Flags(COHERENT_BIT);

    pub const READ_BIT       : u8 = 1 << 0;
    pub const WRITE_BIT      : u8 = 1 << 1;
    pub const READ_WRITE_BITS: u8 = READ_BIT | WRITE_BIT;
    pub const PERSISTENT_BIT : u8 = 1 << 2;
    pub const COHERENT_BIT   : u8 = 1 << 3;
    pub const RW_MASK        : u8 = READ_WRITE_BITS;

    #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
    pub struct Flags(pub u8);

    impl Flags {
        pub fn can_read(&self) -> bool { self.0 | READ_BIT != 0 }
        pub fn can_write(&self) -> bool { self.0 | WRITE_BIT != 0 }
        pub fn is_persistent(&self) -> bool { self.0 | PERSISTENT_BIT != 0 }
        pub fn is_coherent(&self) -> bool { self.0 | COHERENT_BIT != 0 }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Error(GLuint);

impl Error {
    pub fn to_str(&self) -> &'static str { gl_error_str(*self) }

    pub fn to_string(&self) -> String { self.to_str().to_string() }
}

#[derive(Debug)]
pub struct GeometryDescriptor<'l> {
    pub attributes: &'l[VertexAttribute],
    pub index_buffer: Option<Buffer>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Blending {
    None,
    Alpha,
    Add,
    Sub,
    Mul,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum BufferType {
    Vertex,
    Index,
    Uniform,
    ShaderStorage,
    DrawIndirect,
    TransformFeedback,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ShaderType {
    Vertex,
    Fragment,
    Geometry,
    Compute,
}

#[derive(Debug)]
pub struct ShaderStageDescriptor<'l> {
    pub stage_type: ShaderType,
    pub src: &'l[&'l str],
}

#[derive(Debug)]
pub struct ShaderPipelineDescriptor<'l> {
    pub stages: &'l[ShaderStage],
    pub attrib_locations: &'l[(&'l str, VertexAttributeLocation)],
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum PixelFormat {
    RgbaU8,
    AlphaU8,
    RgbaF32,
    AlphaF32,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum UpdateHint {
    Static,
    Stream,
    Dynamic,
}

pub struct Device {
    data_texture: Texture,

    current_shader: Option<ShaderPipeline>,
    current_geometry: Option<Geometry>,
    current_target_types: TargetTypes,
    current_blend_mode: Option<Blending>,

    pub ignore_errors: bool,
    pub log_errors: bool,
    pub crash_on_errors: bool,
}

impl Device {

    pub fn new() -> Self {
        Device {
            data_texture: create_data_texture(),
            current_shader: None,
            current_target_types: 0,
            current_blend_mode: None,
            current_geometry: None,
            ignore_errors: false,
            log_errors: true,
            crash_on_errors: false,
        }
    }

    pub fn set_viewport(&mut self, x:i32, y:i32, w:i32, h:i32) {
        unsafe {
            gl::Viewport(x, y, w, h);
        }
    }

    pub fn create_texture(&mut self, descriptor: &TextureDescriptor) -> Texture {
        unsafe {
            let mut texture = Texture { handle: 0 };

            gl::GenTextures(1, &mut texture.handle);

            let flags = descriptor.flags;

            if flags == 0 {
                return texture;
            }

            gl::BindTexture(gl::TEXTURE_2D, texture.handle);

            if flags & REPEAT_S != 0 {
                gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::REPEAT as i32);
            }
            if flags & REPEAT_T != 0 {
                gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::REPEAT as i32);
            }
            if flags & CLAMP_S != 0 {
                gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
            }
            if flags & CLAMP_T != 0 {
                gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
            }
            if flags & MIN_FILTER_LINEAR != 0 {
                gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
            }
            if flags & MAG_FILTER_LINEAR != 0 {
                gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
            }
            if flags & MIN_FILTER_NEAREST != 0 {
                gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
            }
            if flags & MAG_FILTER_NEAREST != 0 {
                gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
            }

            gl::BindTexture(gl::TEXTURE_2D, 0);

            texture
        }
    }

    pub fn destroy_texture(&mut self, texture: Texture) {
        unsafe {
            gl::DeleteTextures(1, &texture.handle);
        }
    }

    pub fn get_vertex_attribute_location(
        &mut self,
        shader: ShaderPipeline,
        name: &str
    ) -> VertexAttributeLocation {
        if shader.handle == 0 {
            return VertexAttributeLocation { index: -1 };
        }
        let c_name = CString::new(name.as_bytes()).unwrap();
        let location = unsafe {
            gl::GetAttribLocation(
                shader.handle,
                c_name.as_bytes_with_nul().as_ptr() as *const i8
            )
        };

        self.check_errors();

        return VertexAttributeLocation { index: location as i16 };
    }

    pub fn copy_buffer_to_buffer(
        &mut self,
        src_buffer: Buffer,
        dest_buffer: Buffer,
        src_offset: u16,
        dest_offset: u16,
        size: u16
    ) -> Result<(), Error> {
        unsafe {
            gl::BindBuffer(gl::COPY_READ_BUFFER, src_buffer.handle);
            gl::BindBuffer(gl::COPY_WRITE_BUFFER, dest_buffer.handle);

            self.check_errors();

            gl::CopyBufferSubData(
                gl::COPY_READ_BUFFER, gl::COPY_WRITE_BUFFER,
                src_offset as isize,
                dest_offset as isize,
                size as isize
            );

            self.check_errors();

            gl::BindBuffer(gl::COPY_READ_BUFFER, 0);
            gl::BindBuffer(gl::COPY_WRITE_BUFFER, 0);

            self.check_errors();
        }
        return Ok(());
    }

    pub fn copy_buffer_to_texture(
        &mut self,
        buffer: Buffer,
        texture: Texture
    ) {
        unsafe {
            let mut width: i32 = 0;
            let mut height: i32 = 0;
            let mut format: i32 = 0;

            gl::BindTexture(gl::TEXTURE_2D, texture.handle);

            gl::GetTexLevelParameteriv(
                gl::TEXTURE_2D, 0, gl::TEXTURE_WIDTH,
                &mut width
            );
            gl::GetTexLevelParameteriv(
                gl::TEXTURE_2D, 0, gl::TEXTURE_HEIGHT,
                &mut height
            );
            gl::GetTexLevelParameteriv(
                gl::TEXTURE_2D, 0, gl::TEXTURE_INTERNAL_FORMAT,
                &mut format
            );

            gl::BindBuffer(gl::PIXEL_UNPACK_BUFFER, buffer.handle);

            // TODO: support other formats
            gl::TexSubImage2D(
                gl::TEXTURE_2D, 0, 0, 0, width, height,
                gl::RGBA, gl::UNSIGNED_BYTE, mem::transmute(0 as usize)
            );

            gl::BindBuffer(gl::PIXEL_UNPACK_BUFFER, 0);
            gl::BindTexture(gl::TEXTURE_2D, 0);
        }
    }

    pub fn create_shader_stage(
        &mut self,
        descriptor: &ShaderStageDescriptor,
    ) -> Result<ShaderStage, Error> {
        unsafe {
            let shader = ShaderStage {
                handle: gl::CreateShader(gl_shader_type(descriptor.stage_type))
            };
            let mut lines: Vec<*const i8> = Vec::new();
            let mut lines_len: Vec<i32> = Vec::new();

            for line in descriptor.src.iter() {
                lines.push(mem::transmute(line.as_ptr()));
                lines_len.push(line.len() as i32);
            }

            gl::ShaderSource(
                shader.handle,
                lines.len() as i32,
                lines.as_ptr(),
                lines_len.as_ptr()
            );

            if let Some(error) = self.check_errors() {
                return Err(error);
            }

            gl::CompileShader(shader.handle);

            if let Some(error) = self.check_errors() {
                return Err(error);
            }

            return Ok(shader);
        }
    }

    pub fn get_shader_stage_result(
        &mut self,
        shader: ShaderStage,
    ) -> Result<(), String> {
        unsafe {
            let mut status : i32 = 0;
            gl::GetShaderiv(shader.handle, gl::COMPILE_STATUS, &mut status);

            if let Some(e) = self.check_errors() {
                return Err(e.to_string());
            }

            if status == gl::TRUE as i32 {
                return Ok(());
            }

            let mut buffer = vec![0u8; 512];
            let mut length: i32 = 0;
            gl::GetShaderInfoLog(
                shader.handle, 512, &mut length,
                mem::transmute(buffer.as_mut_ptr())
            );

            return Err(match String::from_utf8(buffer) {
                Ok(msg) => { msg }
                Err(_) => { "Unknown shader error".to_string() }
            });
        }
    }

    pub fn create_shader_pipeline(
        &mut self,
        descriptor: &ShaderPipelineDescriptor,
    ) -> Result<ShaderPipeline, Error> {
        unsafe {
            let pipeline = ShaderPipeline {
                handle: gl::CreateProgram()
            };
            for stage in descriptor.stages.iter() {
                gl::AttachShader(pipeline.handle, stage.handle);
            }

            for &(ref name, loc) in descriptor.attrib_locations.iter() {
                if loc.index < 0 {
                    gl::DeleteProgram(pipeline.handle);
                    return Err(Error(gl::INVALID_VALUE));
                }
                let c_name = CString::new(name.as_bytes()).unwrap();
                gl::BindAttribLocation(
                    pipeline.handle, loc.index as u32,
                    c_name.as_bytes_with_nul().as_ptr() as *const i8
                );
            }

            gl::LinkProgram(pipeline.handle);

            return Ok(pipeline);
        }
    }

    pub fn get_shader_pipeline_result(&mut self, shader: ShaderPipeline) -> Result<(), String> {
        if shader.handle == 0 {
            return Err(format!("Invalid handle"));
        }
        unsafe {
            gl::ValidateProgram(shader.handle);
            if let Some(error) = self.check_errors() {
                return Err(error.to_string());
            }

            let mut status: i32 = 0;
            gl::GetProgramiv(shader.handle, gl::VALIDATE_STATUS, &mut status);
            if let Some(error) = self.check_errors() {
                return Err(error.to_string());
            }

            if status == gl::TRUE as i32 {
                return Ok(());
            }

            let mut buffer = [0u8; 512];
            let mut length: i32 = 0;
            gl::GetProgramInfoLog(
                shader.handle, 512, &mut length,
                mem::transmute(buffer.as_mut_ptr())
            );

            let err_msg = match String::from_utf8(buffer.to_vec()) {
                Ok(msg) => { msg }
                Err(_) => { "Unknown shader error".to_string() }
            };
            return Err(err_msg);
        }
    }

    pub fn create_buffer(
        &mut self,
        descriptor: &BufferDescriptor,
    ) -> Result<Buffer, Error> {
        unsafe {
            let mut buffer = Buffer {
                handle: 0,
                buffer_type: descriptor.buffer_type
            };

            gl::GenBuffers(1, &mut buffer.handle);

            self.check_errors();

            if descriptor.size == 0 {
                return Ok(buffer);
            }

            // allocate the buffer

            gl::BindBuffer(
                gl_buffer_type(descriptor.buffer_type),
                buffer.handle
            );

            gl::BufferData(
                gl_buffer_type(descriptor.buffer_type),
                descriptor.size as isize,
                mem::transmute(0 as usize), // :(
                gl_update_hint(descriptor.update_hint)
            );

            if let Some(error) = self.check_errors() {
                return Err(error);
            }

            return Ok(buffer);
        }
    }

    pub fn destroy_buffer(
        &mut self,
        buffer: Buffer
    ) {
        unsafe {
            gl::DeleteBuffers(1, &buffer.handle);
            self.check_errors();
        }
    }

    // TODO: try returning a struc MappedBuffer<'l, T> { slice: &'l[T], buffer, &'mut device }
    // to auto-unmap and prevent interleaving maps.
    pub fn map_buffer<T>(
        &mut self,
        buffer: Buffer,
        range: Option<(u16, u16)>, // start/end in elements
        flags: map::Flags,
        data: &mut &mut[T]
    ) -> Result<(), Error> {
        unsafe {
            let size_of = mem::size_of::<T>();

            let sz = size_of as u16;
            let range = range.map(|(start, end)|{(start*sz, end*sz)});

            let mut ptr = 0 as *mut u8;
            let result = self.map_buffer_raw(buffer, range, flags, &mut ptr)?;

            assert!(ptr != 0 as *mut u8);

            let mut size: GLint = 0;
            gl::GetBufferParameteriv(
                gl_buffer_type(buffer.buffer_type),
                gl::BUFFER_SIZE,
                &mut size
            );

            if let Some((start, end)) = range {
                size = std::cmp::min(size, (start - end) as i32);
            }

            *data = std::slice::from_raw_parts_mut(
                mem::transmute(ptr),
                size as usize / size_of
            );
        }
        Ok(())
    }

    unsafe fn map_buffer_raw(
        &mut self,
        buffer: Buffer,
        range: Option<(u16, u16)>, // start/end in bytes
        flags: map::Flags,
        data: *mut *mut u8
    ) -> Result<(), Error> {
        if buffer.handle == 0 {
            return Err(Error(gl::INVALID_VALUE));
        }

        let gl_target = gl_buffer_type(buffer.buffer_type);

        gl::BindBuffer(gl_target, buffer.handle);

        if let Some(error) = self.check_errors() {
            return Err(error);
        }

        if let Some((start, end)) = range {
            *data = gl::MapBufferRange(
                gl_target,
                start as GLintptr,
                (end - start) as isize,
                gl_access_flags(flags)
            ) as *mut u8;
        } else {
            *data = gl::MapBuffer(
                gl_target,
                gl_access_flags(flags)
            ) as *mut u8;
        }

        if let Some(error) = self.check_errors() {
            return Err(error);
        }

        return Ok(());
    }

    pub fn unmap_buffer(&mut self, buffer: Buffer) {
        unsafe {
            gl::UnmapBuffer(gl_buffer_type(buffer.buffer_type));
        }
        self.check_errors();
    }

    pub fn destroy_geometry(&mut self, geom: Geometry) {
        unsafe {
            gl::DeleteVertexArrays(1, &geom.handle);
            self.check_errors();
        }
    }

    pub fn create_geometry(
        &mut self,
        descriptor: &GeometryDescriptor,
    ) -> Result<Geometry, Error> {
        unsafe {
            let mut handle: u32 = 0;
            gl::GenVertexArrays(1, &mut handle);

            if let Some(error) = self.check_errors() {
                return Err(error);
            }

            gl::BindVertexArray(handle);

            if let Some(error) = self.check_errors() {
                return Err(error);
            }

            for attr in descriptor.attributes.iter() {
                gl::BindBuffer(gl::ARRAY_BUFFER, attr.buffer.handle);

                if let Some(error) = self.check_errors() {
                    return Err(error);
                }

                gl::VertexAttribPointer(
                    attr.location.index as u32,
                    attr.attrib_type.size as i32,
                    gl_data_type(attr.attrib_type),
                    gl_bool(attr.normalize),
                    attr.stride as i32,
                    mem::transmute(attr.offset as usize)
                );

                if let Some(error) = self.check_errors() {
                    return Err(error);
                }

                gl::EnableVertexAttribArray(attr.location.index as u32);

                if let Some(error) = self.check_errors() {
                    return Err(error);
                }
            }

            match descriptor.index_buffer {
                Some(ibo) => {
                    gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, ibo.handle);

                    if let Some(error) = self.check_errors() {
                        return Err(error);
                    }
                }
                None => {}
            }

            gl::BindVertexArray(0);

            return Ok(Geometry { handle: handle });
        }
    }

    pub fn bind_uniform_buffer(
        &mut self,
        shader: ShaderPipeline,
        location: UniformBlockLocation,
        binding_index: UniformBindingIndex,
        ubo: Buffer,
        range: Option<(u16, u16)>
    ) -> Result<(), Error> {
        unsafe {
            match range {
                Some((start, size)) => {
                    gl::BindBufferRange(
                        gl::UNIFORM_BUFFER,
                        binding_index as GLuint,
                        ubo.handle,
                        start as isize,
                        size as isize
                    );
                }
                None => {
                    gl::BindBufferBase(
                        gl::UNIFORM_BUFFER,
                        binding_index as GLuint,
                        ubo.handle
                    );
                }
            }

            gl::UniformBlockBinding(
                shader.handle,
                location.index as GLuint,
                binding_index as GLuint,
            );
        }

        return self.check_error_result();
    }

    pub fn get_uniform_block_location(
        &mut self,
        shader: ShaderPipeline,
        name: &str
    ) -> UniformBlockLocation {
        let mut result = UniformBlockLocation { index: -1 };
        let c_name = CString::new(name.as_bytes()).unwrap();
        unsafe {
            result.index = gl::GetUniformBlockIndex(
                shader.handle,
                c_name.as_bytes_with_nul().as_ptr() as *const i8
            ) as i16;
        }
        return result;
    }

    pub fn set_shader(&mut self, shader: ShaderPipeline) -> Result<(), Error> {
        self.check_errors();
        if self.current_shader == Some(shader) {
            return Ok(());
        }
        self.current_shader = Some(shader);
        unsafe {
            gl::UseProgram(shader.handle);
        }
        return self.check_error_result();
    }

    pub fn draw(&mut self,
        geom: Geometry,
        range: Range,
        flags: GeometryFlags,
        blend: Blending,
        targets: TargetTypes
    ) -> Result<(), Error> {
        unsafe {
            self.update_targets(targets);
            self.update_blend_mode(blend);

            if Some(geom) != self.current_geometry {
                self.current_geometry = Some(geom);
                gl::BindVertexArray(geom.handle);
            };

            match range {
                Range::VertexRange(first, count) => {
                    gl::DrawArrays(
                        gl_draw_mode(flags),
                        first as i32,
                        count as i32
                    );
                }
                Range::IndexRange(first, count) => {
                    gl::DrawElements(
                        gl_draw_mode(flags),
                        count as i32,
                        gl::UNSIGNED_SHORT,
                        // /2 because offset in bytes with u16
                        mem::transmute((first / 2) as usize)
                    );
                }
            }
            return self.check_error_result();
        }
    }

    fn update_targets(&mut self, targets: TargetTypes) {
        unsafe {
            if (targets & DEPTH != 0) && (self.current_target_types & DEPTH == 0) {
                gl::Enable(gl::DEPTH_TEST);
                self.current_target_types |= DEPTH;
            } else if (targets & DEPTH == 0) && (self.current_target_types & DEPTH != 0) {
                gl::Disable(gl::DEPTH_TEST);
                self.current_target_types &= COLOR | STENCIL;
            }
        }
    }

    fn update_blend_mode(&mut self, blend: Blending) {
        unsafe {
            if Some(blend) == self.current_blend_mode {
                return;
            }
            if blend == Blending::None {
                gl::Disable(gl::BLEND);
            } else {
                gl::Enable(gl::BLEND);
                if blend == Blending::Alpha {
                    gl::BlendFunc(gl::SRC_ALPHA,gl::ONE_MINUS_SRC_ALPHA);
                } else {
                    panic!("Unimplemented");
                }
            }
        }
    }

    fn flush(&mut self) {
        unsafe {
            gl::Flush();
        }
    }

    fn clear(&mut self, targets: TargetTypes) {
        unsafe {
            gl::Clear(gl_clear_targets(targets));
        }
    }

    fn set_clear_color(&mut self, r:f32, g: f32, b: f32, a: f32) {
        unsafe {
            gl::ClearColor(r, g, b, a);
        }
    }

    fn check_error_result(&mut self) -> Result<(), Error> {
        match self.check_errors() {
            Some(e) => { Err(e) }
            None => { Ok(()) }
        }
    }

    fn check_errors(&mut self) -> Option<Error> {
        if self.ignore_errors {
            return None;
        }
        let error = unsafe { Error(gl::GetError()) };
        if error.0 == gl::NO_ERROR {
            return None;
        }

        if self.log_errors {
            println!("GL Error: 0x{:x} ({})", error.0, error.to_str());
        }
        if self.crash_on_errors {
            panic!();
        }
        return Some(error);
    }
}

fn create_data_texture() -> Texture {
    unsafe {
        let mut handle = 0;
        gl::GenTextures(1, &mut handle);
        gl::BindTexture(gl::TEXTURE_2D, handle);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as GLint);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as GLint);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as GLint);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as GLint);
        gl::BindTexture(gl::TEXTURE_2D, 0);

        Texture { handle }
    }
}

fn gl_error_str(err: Error) -> &'static str {
    return match err.0 {
        gl::NO_ERROR            => { "(No error)" }
        gl::INVALID_ENUM        => { "Invalid enum" },
        gl::INVALID_VALUE       => { "Invalid value" },
        gl::INVALID_OPERATION   => { "Invalid operation" },
        gl::OUT_OF_MEMORY       => { "Out of memory" },
        gl::FRAMEBUFFER_INCOMPLETE_MISSING_ATTACHMENT => "Missing attachment.",
        gl::FRAMEBUFFER_INCOMPLETE_ATTACHMENT => "Incomplete attachment.",
        gl::FRAMEBUFFER_INCOMPLETE_DRAW_BUFFER => "Incomplete draw buffer.",
        gl::FRAMEBUFFER_INCOMPLETE_MULTISAMPLE => "Incomplete multisample.",
        gl::FRAMEBUFFER_UNSUPPORTED => "Unsupported.",
        _ => { "Unknown error" }
    }
}

fn gl_shader_type(target: ShaderType) -> GLuint {
    match target {
        ShaderType::Vertex => gl::VERTEX_SHADER,
        ShaderType::Fragment => gl::FRAGMENT_SHADER,
        ShaderType::Geometry => gl::GEOMETRY_SHADER,
        ShaderType::Compute => gl::COMPUTE_SHADER,
    }
}

fn gl_bool(b: bool) -> u8 {
    return if b { gl::TRUE } else { gl::FALSE };
}

fn gl_texture_unit(unit: u32) -> GLuint {
    return gl::TEXTURE0 + unit;
}

fn gl_attachement(i: u32) -> GLuint {
    return gl::COLOR_ATTACHMENT0 + i;
}

fn gl_data_type(ty: DataType) -> GLuint {
    match ty.scalar {
        ScalarType::F32 => gl::FLOAT,
        ScalarType::F64 => gl::DOUBLE,
        ScalarType::U32 => gl::UNSIGNED_INT,
        ScalarType::I32 => gl::INT,
        ScalarType::U16 => gl::UNSIGNED_SHORT,
        ScalarType::I16 => gl::SHORT,
        ScalarType::U8 =>  gl::UNSIGNED_BYTE,
        ScalarType::I8 =>  gl::BYTE,
        _ => panic!("unsupported scalar type")
    }
}

fn gl_data_type_from_format(fmt: PixelFormat) -> GLuint {
    match fmt {
        PixelFormat::AlphaF32 | PixelFormat::RgbaF32 => gl::FLOAT,
        _ => gl::UNSIGNED_BYTE,
    }
}

fn gl_clear_targets(t: TargetTypes) -> GLuint {
    let mut res = 0;
    if t & COLOR != 0 { res |= gl::COLOR_BUFFER_BIT; }
    if t & DEPTH != 0 { res |= gl::DEPTH_BUFFER_BIT; }
    if t & STENCIL != 0 { res |= gl::STENCIL_BUFFER_BIT; }
    return res;
}

fn gl_draw_mode(flags: GeometryFlags) -> GLuint {
    if flags & LINES != 0 {
        return if flags & STRIP != 0 { gl::LINE_STRIP }
               else if flags & LOOP != 0 { gl::LINE_LOOP }
               else { gl::LINES }
    }
    return if flags & STRIP != 0 { gl::TRIANGLE_STRIP }
           else { gl::TRIANGLES }
}

fn gl_buffer_type(t: BufferType) -> GLuint {
    return match t {
        BufferType::Vertex => gl::ARRAY_BUFFER,
        BufferType::Index => gl::ELEMENT_ARRAY_BUFFER,
        BufferType::Uniform => gl::UNIFORM_BUFFER,
        BufferType::TransformFeedback => gl::TRANSFORM_FEEDBACK_BUFFER,
        BufferType::DrawIndirect => gl::DRAW_INDIRECT_BUFFER,
        BufferType::ShaderStorage => gl::SHADER_STORAGE_BUFFER,
    }
}

fn gl_update_hint(hint: UpdateHint) -> GLuint {
    match hint {
        UpdateHint::Static => gl::STATIC_DRAW,
        UpdateHint::Stream => gl::STREAM_DRAW,
        UpdateHint::Dynamic => gl::DYNAMIC_DRAW,
    }
}

fn gl_access_flags(flags: map::Flags) -> GLuint {
    return match flags.0 & map::RW_MASK {
        map::READ_BIT => { gl::READ_ONLY }
        map::WRITE_BIT => { gl::WRITE_ONLY }
        map::READ_WRITE_BITS => { gl::READ_WRITE }
        _ => { panic!() }
    };
}

pub struct Renderer {
    pub device: Device,

    a_position: VertexAttributeLocation,
    a_normal: VertexAttributeLocation,
    a_prim_id: VertexAttributeLocation,
    a_advancement: VertexAttributeLocation,
}

impl Renderer {
    pub fn new() -> Result<Renderer, Error> {

        let mut device = Device::new();
        device.log_errors = true;
        device.crash_on_errors = true;

        let a_position = VertexAttributeLocation { index: 0 };
        let a_normal = VertexAttributeLocation { index: 1 };
        let a_prim_id = VertexAttributeLocation { index: 2 };
        let a_advancement = VertexAttributeLocation { index: 3 };

        let vs = device.create_shader_stage(
            &ShaderStageDescriptor {
                stage_type: ShaderType::Vertex,
                src: &[glsl::FILL_VERTEX_SHADER],
            },
        ).unwrap();
        device.get_shader_stage_result(vs).unwrap();

        let fs = device.create_shader_stage(
            &ShaderStageDescriptor {
                stage_type: ShaderType::Fragment,
                src: &[glsl::FILL_FRAGMENT_SHADER],
            },
        ).unwrap();
        device.get_shader_stage_result(fs).unwrap();

        let pipeline = device.create_shader_pipeline(
            &ShaderPipelineDescriptor {
                stages: &[vs, fs],
                attrib_locations: &[
                    ("a_position", a_position),
                    ("a_normal", a_normal),
                    ("a_prim_id", a_prim_id),
                ]
            }
        )?;

        match device.get_shader_pipeline_result(pipeline) {
            Err(msg) => {
                panic!("Shader link failed - {}\n", msg);
            }
            _ => {}
        }

        device.set_clear_color(0.0, 0.0, 0.0, 0.0);

        Ok(Renderer {
            device,
            a_position,
            a_normal,
            a_prim_id,
            a_advancement,
        })
    }

    pub fn alloc_fill_geometry(
        &mut self,
        vbo_size: u32,
        ibo_size: u32,
    ) -> FillGeometry {
        let stride = mem::size_of::<GpuFillVertex>() as u16;

        let vbo = self.device.create_buffer(
            &BufferDescriptor {
                size: stride as u32 * vbo_size,
                buffer_type: BufferType::Vertex,
                update_hint: UpdateHint::Static,
            }
        ).ok().unwrap();

        let ibo = self.device.create_buffer(
            &BufferDescriptor {
                size: ibo_size * (mem::size_of::<u16>() as u32),
                buffer_type: BufferType::Index,
                update_hint: UpdateHint::Static,
            }
        ).ok().unwrap();

        let geom = self.device.create_geometry(
            &GeometryDescriptor {
                attributes: &[
                    VertexAttribute {
                        buffer: vbo,
                        attrib_type: DataType::vec2(),
                        location: self.a_position,
                        stride,
                        offset: 0,
                        normalize: false,
                    },
                    VertexAttribute {
                        buffer: vbo,
                        attrib_type: DataType::vec2(),
                        location: self.a_normal,
                        stride,
                        offset: 8,
                        normalize: false,
                    },
                    VertexAttribute {
                        buffer: vbo,
                        attrib_type: DataType::int(),
                        location: self.a_prim_id,
                        stride,
                        offset: 16,
                        normalize: false,
                    }
                ],
                index_buffer: Some(ibo)
            }
        ).ok().unwrap();

        return FillGeometry { geom, vbo, ibo };
    }
}

pub struct FillGeometry {
    pub geom: Geometry,
    pub vbo: Buffer,
    pub ibo: Buffer,
}
