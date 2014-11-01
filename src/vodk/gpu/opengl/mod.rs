use gl;
use super::context::*;
use super::logging::LoggingProxy;

use std::str;
use std::string::raw;
use std::mem;
use libc::c_void;

use data;

macro_rules! check_err (
    ($($arg:tt)*) => (
        match gl::GetError() {
            gl::NONE => {}
            e => {
                return Err(Error{
                    code: gl_error_str(e).to_string(),
                    detail: Some(format!($($arg)*))
                });
            }
        }
    )
)

pub type DriverBugs = u64;
pub const DRIVER_DEFAULT : DriverBugs = 0;
pub const MISSING_INDEX_BUFFER_VAO : DriverBugs = 1;

pub struct RenderingContextGL {
    workaround: DriverBugs,
    current_render_target: RenderTarget,
    current_program: Shader,
    current_geometry: Geometry,
    current_target_types: TargetTypes,
    current_blend_mode: BlendMode,
    ignore_errors: bool,
}

impl RenderingContextGL {
    pub fn new() -> RenderingContextGL {
        RenderingContextGL {
            workaround: DRIVER_DEFAULT,
            current_program: Shader { handle: 0 },
            current_render_target: RenderTarget { handle: 0 },
            current_geometry: Geometry { handle: 0, ibo: 0 },
            current_target_types: 0,
            current_blend_mode: NO_BLENDING,
            ignore_errors: false,
        }
    }

    fn update_targets(&mut self, targets: TargetTypes) {
        if (targets & DEPTH != 0) && (self.current_target_types & DEPTH == 0) {
            gl::Enable(gl::DEPTH_TEST);
            self.current_target_types |= DEPTH;
        } else if (targets & DEPTH == 0) && (self.current_target_types & DEPTH != 0) {
            gl::Disable(gl::DEPTH_TEST);
            self.current_target_types &= COLOR | STENCIL;
        }
    }

    fn update_blend_mode(&mut self, blend: BlendMode) {
        if blend == self.current_blend_mode {
            return;
        }
        if blend == NO_BLENDING {
            gl::Disable(gl::BLEND);
        } else {
            gl::Enable(gl::BLEND);
            if blend == ALPHA_BLENDING {
                gl::BlendFunc(gl::SRC_ALPHA,gl::ONE_MINUS_SRC_ALPHA);
            } else {
                fail!("Unimplemented");
            }
        }
    }
}

impl RenderingContext for RenderingContextGL {
    fn reset_state(&mut self) {
        gl::BindTexture(gl::TEXTURE_2D, 0);
        gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
        gl::UseProgram(0);
        gl::BindVertexArray(0);
        gl::BindBuffer(gl::ARRAY_BUFFER, 0);
        gl::ClearColor(0.0, 0.0, 0.0, 0.0);
        self.current_render_target = self.get_default_render_target();
        self.current_program = Shader { handle: 0 };
        self.current_geometry = Geometry { handle: 0, ibo: 0 };

        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::SRC_ALPHA,gl::ONE);
    }

    //fn check_error(&mut self) -> Option<String> {
    //    match gl::GetError() {
    //        gl::NO_ERROR            => None,
    //        gl::INVALID_ENUM        => Some("Invalid enum.".to_string()),
    //        gl::INVALID_VALUE       => Some("Invalid value.".to_string()),
    //        gl::INVALID_OPERATION   => Some("Invalid operation.".to_string()),
    //        gl::OUT_OF_MEMORY       => Some("Out of memory.".to_string()),
    //        _ => Some("Unknown error.".to_string()),
    //    }
    //}

    //fn get_error_str(&mut self, err: ErrorCode) -> &'static str {
    //    return gl_error_str(err);
    //}

    fn is_supported(&mut self, f: Feature) -> bool {
        // TODO
        match f {
            FRAGMENT_SHADING => true,
            VERTEX_SHADING => true,
            GEOMETRY_SHADING => false,
            COMPUTE => false,
            DEPTH_TEXTURE => false,
            RENDER_TO_TEXTURE => false,
            MULTIPLE_RENDER_TARGETS => false,
            INSTANCED_RENDERING => false,
        }
    }

    fn flush(&mut self) {
        gl::Flush();
    }

    fn set_viewport(&mut self, x:i32, y:i32, w:i32, h:i32) {
        gl::Viewport(x,y,w,h);
    }

    fn set_clear_color(&mut self, r: f32, g: f32, b: f32, a: f32) {
        gl::ClearColor(r,g,b,a);
    }

    fn clear(&mut self, buffers: TargetTypes) {
        gl::Clear(gl_clear_targets(buffers));
    }

    fn create_texture(&mut self, flags: TextureFlags) -> Texture {
        let mut tex = 0;
        unsafe {
            gl::GenTextures(1, &mut tex);
        }
        let tex = Texture { handle: tex };
        self.set_texture_flags(tex, flags);
        return tex;
    }

    fn destroy_texture(&mut self, tex: Texture) {
        unsafe {
            gl::DeleteTextures(1, &tex.handle);
        }
    }

    fn set_texture_flags(&mut self, tex: Texture, flags: TextureFlags) {
        set_texture_flags(tex.handle, flags);
    }

    fn upload_texture_data(&mut self,
        dest: Texture,
        data: &BufferData,
        w:u32, h:u32,
        format: PixelFormat
    ) -> RendererResult {
        gl::BindTexture(gl::TEXTURE_2D, dest.handle);

        let fmt = gl_format(format);
        unsafe {
            gl::TexImage2D(
                gl::TEXTURE_2D, 0, fmt as i32, w as i32, h as i32,
                0, fmt, gl_data_type_from_format(format),
                mem::transmute(data.as_byte_slice().unsafe_get(0))
            );
        }

        gl::BindTexture(gl::TEXTURE_2D, 0);
        return Ok(());
    }

    fn allocate_texture(&mut self, dest: Texture,
                        w:u32, h:u32, format: PixelFormat) -> RendererResult {
        gl::BindTexture(gl::TEXTURE_2D, dest.handle);
        print_gl_error("upload_texture_data after bind");
        let fmt = gl_format(format);
        unsafe {
            gl::TexImage2D(
                gl::TEXTURE_2D, 0, fmt as i32, w as i32, h as i32,
                0, fmt, gl::UNSIGNED_BYTE, 0 as *const c_void
            );
        }
        print_gl_error("upload_texture_data after TexImage2D");
        gl::BindTexture(gl::TEXTURE_2D, 0);
        return Ok(());
    }

    fn read_back_texture(&mut self, tex: Texture,
                         format: PixelFormat,
                         dest: &mut [u8]) -> RendererResult {
        gl::BindTexture(gl::TEXTURE_2D, tex.handle);
        unsafe {
            gl::GetTexImage(gl::TEXTURE_2D, 0, gl_format(format), 
                            gl::UNSIGNED_BYTE, mem::transmute(dest.unsafe_get(0)));
            check_err!("glGetTexImage(...) on texture {}", tex.handle);
        }
        return Ok(());
    }

    fn create_shader_stage(&mut self, t: ShaderType) -> ShaderStage {
        return ShaderStage { handle: gl::CreateShader(gl_shader_type(t)) };
    }

    fn destroy_shader_stage(&mut self, s: ShaderStage) {
        gl::DeleteShader(s.handle);
    }

    fn create_shader(&mut self) -> Shader {
        return Shader { handle: gl::CreateProgram() };
    }

    fn destroy_shader(&mut self, p: Shader) {
        gl::DeleteProgram(p.handle);
    }

    fn compile_shader_stage(&mut self, shader: ShaderStage, src: &[&str]) -> RendererResult {
        unsafe {
            let mut lines: Vec<*const i8> = Vec::new();
            let mut lines_len: Vec<i32> = Vec::new();

            for line in src.iter() {
                lines.push(mem::transmute(line.as_ptr()));
                lines_len.push(line.len() as i32);
            }

            gl::ShaderSource(shader.handle, 1, lines.as_ptr(), lines_len.as_ptr());
            gl::CompileShader(shader.handle);

            let mut status : i32 = 0;
            gl::GetShaderiv(shader.handle, gl::COMPILE_STATUS, &mut status);
            if status != gl::TRUE as i32 {
                let mut buffer = [0u8, ..512];
                let mut length: i32 = 0;
                gl::GetShaderInfoLog(shader.handle, 512, &mut length,
                                     mem::transmute(buffer.as_mut_ptr()));

                return Err( Error {
                    code: "Shader compilation error".to_string(),
                    detail: Some(raw::from_buf(buffer.as_ptr())),
                });
            }
            return Ok(());
        }
    }

    fn link_shader(&mut self, p: Shader,
                   stages: &[ShaderStage],
                   attrib_locations: &[(&str, VertexAttributeLocation)]) -> RendererResult {
        unsafe {
            for s in stages.iter() {
                gl::AttachShader(p.handle, s.handle);
            }

            for &(ref name, loc) in attrib_locations.iter() {
                if loc < 0 {
                    return Err(Error {
                        code: "Shader link error".to_string(),
                        detail: Some("Invalid negative vertex attribute location".to_string())
                    });
                }
                name.with_c_str(|c_name| {
                    gl::BindAttribLocation(p.handle, loc as u32, c_name);
                });
            }

            gl::LinkProgram(p.handle);
            gl::ValidateProgram(p.handle);
            let mut status: i32 = 0;
            gl::GetProgramiv(p.handle, gl::VALIDATE_STATUS, &mut status);

            if status != gl::TRUE as i32 {
                let mut buffer = [0u8, ..512];
                let mut length: i32 = 0;
                gl::GetProgramInfoLog(p.handle, 512, &mut length,
                                      mem::transmute(buffer.as_mut_ptr()));

                return Err( Error {
                    code: "Shader link error".to_string(),
                    detail: Some(raw::from_buf(buffer.as_ptr())),
                });
            }
            return Ok(());
        }
    }

    fn create_buffer(&mut self, buffer_type: BufferType) -> Buffer {
        let mut b: u32 = 0;
        unsafe {
            gl::GenBuffers(1, &mut b);
        }
        return Buffer {
            handle: b,
            buffer_type: buffer_type
        };
    }

    fn destroy_buffer(&mut self, buffer: BufferObject) {
        unsafe {
            gl::DeleteBuffers(1, &buffer.handle);
        }
    }

    fn upload_buffer(&mut self,
        buffer: Buffer, buf_type: BufferType,
        update: UpdateHint,
        data: &BufferData
    ) -> RendererResult {

        unsafe {
            let gl_buf_type = gl_buffer_type(buf_type);
            gl::BindBuffer(gl_buf_type, buffer.handle);
            check_err!("glBindBuffer({}, {})", buf_type, buffer.handle);
            gl::BufferData(gl_buf_type, data.byte_len() as i64,
                           mem::transmute(data.as_byte_slice().unsafe_get(0)),
                           gl_update_hint(update));
            check_err!("glBufferData({}, {}, {}, {})", buf_type,
                        data.len(), data.as_byte_slice().unsafe_get(0),
                        gl_update_hint(update));
        }
        return Ok(());
    }

    fn allocate_buffer(&mut self, buffer: Buffer, buf_type: BufferType,
                       update: UpdateHint, size: u32) -> RendererResult {
        unsafe {
            let gl_buf_type = gl_buffer_type(buf_type);
            gl::BindBuffer(gl_buf_type, buffer.handle);
            check_err!("glBindBuffer({}, {})", buf_type, buffer.handle);
            gl::BufferData(gl_buf_type, size as i64,
                           0 as *const c_void,
                           gl_update_hint(update));
            check_err!("glBufferData({}, {}, 0, {})", buf_type,
                       size, gl_update_hint(update));
        }
        return Ok(());
    }

    fn destroy_geometry(&mut self, obj: Geometry) {
        unsafe {
            gl::DeleteVertexArrays(1, &obj.handle);
        }
    }

    fn create_geometry(&mut self, attributes: &[VertexAttribute],
                       elements: Option<Buffer>) -> Result<Geometry, Error> {
        let mut handle: u32 = 0;
        unsafe {
            gl::GenVertexArrays(1, &mut handle);
        }

        gl::BindVertexArray(handle);

        for attr in attributes.iter() {
            gl::BindBuffer(gl::ARRAY_BUFFER, attr.buffer.handle);
            unsafe {
            gl::VertexAttribPointer(
                attr.location as u32,
                data::num_components(attr.attrib_type) as i32,
                gl_data_type(attr.attrib_type),
                gl_bool(attr.normalize),
                attr.stride as i32,
                mem::transmute(attr.offset as uint)
            );
            check_err!("glVertexAttribPointer(...)");
            gl::EnableVertexAttribArray(attr.location as u32);
            check_err!("glEnableVertexAttribArray({})", attr.location);
            }
        }

        let ibo =  match elements {
            Some(elts) => { elts.handle }
            None => { 0 }
        };
        // The OpenGL spec indicates that the index buffer binding
        // is part of the VAO state, but some drivers don't follow
        // this, so we'll have to store the ibo in the Geometry to
        // rebind it when rendering
        if ibo != 0 {
            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, ibo);
        }

        gl::BindVertexArray(0);

        return Ok(Geometry {
            handle: handle,
            ibo: ibo
        });
    }

    fn get_shader_input_location(&mut self, program: Shader,
                                 name: &str) -> ShaderInputLocation {
        let mut location = 0;
        name.with_c_str(|c_name| unsafe {
            location = gl::GetUniformLocation(program.handle, c_name) as ShaderInputLocation;
        });
        return location;
    }

    fn get_vertex_attribute_location(&mut self, program: Shader,
                                     name: &str) -> VertexAttributeLocation {
        let mut location = 0;
        name.with_c_str(|c_name| unsafe {
            location = gl::GetAttribLocation(program.handle, c_name) as VertexAttributeLocation;
        });
        return location;
    }

    fn create_render_target(&mut self,
                            color_attachments: &[Texture],
                            depth: Option<Texture>,
                            stencil: Option<Texture>) -> Result<RenderTarget, Error> {
        let mut fbo: u32 = 0;
        unsafe {
            gl::GenFramebuffers(1, &mut fbo);
        }

        gl::BindFramebuffer(gl::FRAMEBUFFER, fbo);
        check_err!("glBindFrameBuffer(GL_FRAMEBUFFER, {})", fbo);

        for i in range(0,color_attachments.len()) {
            gl::FramebufferTexture2D(
                gl::FRAMEBUFFER,
                gl_attachement(i as u32),
                gl::TEXTURE_2D,
                color_attachments[i].handle,
                0
            );
            check_err!("glFramebufferTexture2D(GL_FRAMEBUFFER, {}, GL_TEXTURE_2D, {}, 0)",
                       gl_attachement(i as u32), color_attachments[i].handle);
        }

        match depth {
            Some(d) => {
                gl::FramebufferTexture2D(
                    gl::FRAMEBUFFER,
                    gl::DEPTH_ATTACHMENT,
                    gl::TEXTURE_2D,
                    d.handle,
                    0
                );
                check_err!("glFramebufferTexture2D(GL_FRAMEBUFFER, G:_DEPTH_ATTACHMENT, GL_TEXTURE_2D, {}, 0)",
                           d.handle);
            }
            _ => {}
        }

        match stencil {
            Some(s) => {
                gl::FramebufferTexture2D(
                    gl::FRAMEBUFFER,
                    gl::STENCIL_ATTACHMENT,
                    gl::TEXTURE_2D,
                    s.handle,
                    0
                );
                check_err!("glFramebufferTexture2D(GL_FRAMEBUFFER, G:_DEPTH_ATTACHMENT, GL_TEXTURE_2D, {}, 0)",
                           s.handle);
            }
            _ => {}
        }

        let status = gl::CheckFramebufferStatus(gl::FRAMEBUFFER);
        gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
        if status != gl::FRAMEBUFFER_COMPLETE {
            return Err(Error{code: gl_error_str(status).to_string(), detail: None });
        }
        return Ok(RenderTarget{ handle: fbo });
    }

    fn destroy_render_target(&mut self, fbo: RenderTarget) {
        if self.current_render_target == fbo {
            let rt = self.get_default_render_target();
            self.set_render_target(rt);
        }
        unsafe {
            gl::DeleteFramebuffers(1, &fbo.handle);
        }
    }

    fn get_default_render_target(&mut self) -> RenderTarget {
        return RenderTarget { handle: 0 };
    }

    fn set_render_target(&mut self, target: RenderTarget) {
        if self.current_render_target == target {
            return;
        }
        gl::BindFramebuffer(gl::FRAMEBUFFER, target.handle);
        self.current_render_target = target;
    }

    fn set_shader_input_float(&mut self, location: ShaderInputLocation, input: &[f32]) {
        match input.len() {
            1 => { gl::Uniform1f(location as i32, input[0]); }
            2 => { gl::Uniform2f(location as i32, input[0], input[1]); }
            3 => { gl::Uniform3f(location as i32, input[0], input[1], input[2]); }
            4 => { gl::Uniform4f(location as i32, input[0], input[1], input[2], input[3]); }
            _ => { fail!("trying to send an invalid number of float uniforms"); }
        }
    }

    fn set_shader_input_int(&mut self, location: ShaderInputLocation, input: &[i32]) {
        match input.len() {
            1 => { gl::Uniform1i(location as i32, input[0]); }
            2 => { gl::Uniform2i(location as i32, input[0], input[1]); }
            3 => { gl::Uniform3i(location as i32, input[0], input[1], input[2]); }
            4 => { gl::Uniform4i(location as i32, input[0], input[1], input[2], input[3]); }
            _ => { fail!("trying to send an invalid number of float uniforms"); }
        }
    }
    fn set_shader_input_matrix(&mut self, location: ShaderInputLocation, input: &[f32], dimension: u32, transpose: bool) {
        unsafe {
            match dimension {
                2 => { gl::UniformMatrix2fv(location as i32, 1, gl_bool(transpose), mem::transmute(input.unsafe_get(0))); }
                3 => { gl::UniformMatrix3fv(location as i32, 1, gl_bool(transpose), mem::transmute(input.unsafe_get(0))); }
                4 => { gl::UniformMatrix4fv(location as i32, 1, gl_bool(transpose), mem::transmute(input.unsafe_get(0))); }
                _ => { fail!("Invalid matrix dimension"); }
            }
        }
    }

    fn set_shader_input_texture(&mut self, location: ShaderInputLocation, texture_unit: u32, tex: Texture) {
        gl::ActiveTexture(gl_texture_unit(texture_unit));
        gl::BindTexture(gl::TEXTURE_2D, tex.handle);
        gl::Uniform1i(location as i32, texture_unit as i32);
    }

    fn set_shader(&mut self, program: Shader) -> RendererResult {
        //if self.current_program != program {
            println!("current_program: {}", self.current_program);
            self.current_program = program;
            gl::UseProgram(program.handle);
            check_err!("glUseProgram({})", program.handle);
        //}
        return Ok(());
    }

    fn draw(&mut self,
        geom: Geometry,
        range: Range,
        flags: GeometryFlags,
        blend: BlendMode,
        targets: TargetTypes
    ) -> RendererResult {
        self.update_targets(targets);
        self.update_blend_mode(blend);

        //if geom != self.current_geometry {
            self.current_geometry = geom;
            gl::BindVertexArray(geom.handle);
            check_err!("glBindVertexArray({})", geom.handle);
        //  };

        match range {
            VertexRange(first, count) => {
                gl::DrawArrays(
                    gl_draw_mode(flags),
                    first as i32,
                    count as i32
                );
                check_err!("glDrawArrays(...)");
            }
            IndexRange(first, count) => {
                assert!(geom.ibo != 0);

                if self.workaround & MISSING_INDEX_BUFFER_VAO != 0 {
                    gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, geom.ibo);
                }
                unsafe {
                    gl::DrawElements(
                        gl_draw_mode(flags),
                        count as i32,
                        gl::UNSIGNED_SHORT,
                        0 as *const c_void
                    );
                }
                check_err!("glDrawElements(...)");
            }
        }
        Ok(())
    }

    fn multi_draw(&mut self,
        geom: Geometry,
        indirect_buffer: Buffer,
        flags: GeometryFlags,
        targets: TargetTypes,
        commands: &[DrawCommand]
    ) -> RendererResult {
        self.update_targets(targets);

        gl::BindBuffer(gl::DRAW_INDIRECT_BUFFER, indirect_buffer.handle);

        if geom.ibo != 0 {
            unsafe {
                gl::MultiDrawElementsIndirect(
                    gl_draw_mode(flags),
                    gl::UNSIGNED_SHORT,
                    mem::transmute(commands.as_ptr()),
                    commands.len() as i32,
                    mem::size_of::<DrawCommand>() as i32
                );
            }
        } else {
            unsafe {
                gl::MultiDrawArraysIndirect(
                    gl_draw_mode(flags),
                    mem::transmute(commands.as_ptr()),
                    commands.len() as i32,
                    mem::size_of::<DrawCommand>() as i32
                );
            }
        }

        fail!("Not implemented");
    }
}

fn print_gl_error(msg: &str) {
    match gl::GetError() {
        gl::NO_ERROR            => {}
        gl::INVALID_ENUM        => { println!("{}: Invalid enum.", msg); },
        gl::INVALID_VALUE       => { println!("{}: Invalid value., ", msg); },
        gl::INVALID_OPERATION   => { println!("{}: Invalid operation.", msg); },
        gl::OUT_OF_MEMORY       => { println!("{}: Out of memory.", msg); },
        _ => { println!("Unknown error."); }
    }
}


fn gl_format(format: PixelFormat) -> u32 {
    match format {
        R8G8B8A8 => gl::RGBA,
        R8G8B8X8 => gl::RGB,
        B8G8R8A8 => gl::BGRA,
        B8G8R8X8 => gl::BGR,
        A8 => gl::RED,
        A_F32 => gl::RED,
    }
}

fn gl_shader_type(target: ShaderType) -> u32 {
    match target {
        VERTEX_SHADER => gl::VERTEX_SHADER,
        FRAGMENT_SHADER => gl::FRAGMENT_SHADER,
        GEOMETRY_SHADER => gl::GEOMETRY_SHADER,
        COMPUTE_SHADER => gl::COMPUTE_SHADER,
    }
}

fn gl_draw_mode(flags: GeometryFlags) -> u32 {
    if flags & LINES != 0 {
        return if flags & STRIP != 0 { gl::LINE_STRIP }
               else if flags & LOOP != 0 { gl::LINE_LOOP }
               else { gl::LINES }
    }
    return if flags & STRIP != 0 { gl::TRIANGLE_STRIP }
           else { gl::TRIANGLES }
}

fn gl_buffer_type(t: BufferType) -> u32 {
    return match t {
        VERTEX_BUFFER => gl::ARRAY_BUFFER,
        INDEX_BUFFER => gl::ELEMENT_ARRAY_BUFFER,
        UNIFORM_BUFFER => gl::UNIFORM_BUFFER,
        TRANSFORM_FEEDBACK_BUFFER => gl::TRANSFORM_FEEDBACK_BUFFER,
        DRAW_INDIRECT_BUFFER => gl::DRAW_INDIRECT_BUFFER,
    }
}

fn gl_update_hint(hint: UpdateHint) -> u32 {
    match hint {
        STATIC_UPDATE => gl::STATIC_DRAW,
        STREAM_UPDATE => gl::STREAM_DRAW,
        DYNAMIC_UPDATE => gl::DYNAMIC_DRAW,
    }
}

fn gl_access_flags(flags: MapFlags) -> u32 {
    return match flags {
        READ_MAP => { gl::READ_ONLY }
        WRITE_MAP => { gl::WRITE_ONLY }
        _ => { gl::READ_WRITE }
    };
}

fn gl_texture_unit(unit: u32) -> u32 {
    return gl::TEXTURE0 + unit;
}

fn gl_attachement(i: u32) -> u32 {
    return gl::COLOR_ATTACHMENT0 + i;
}

fn gl_clear_targets(t: TargetTypes) -> u32 {
    let mut res = 0;
    if t & COLOR != 0 { res |= gl::COLOR_BUFFER_BIT; }
    if t & DEPTH != 0 { res |= gl::DEPTH_BUFFER_BIT; }
    if t & STENCIL != 0 { res |= gl::STENCIL_BUFFER_BIT; }
    return res;
}

fn gl_bool(b: bool) -> u8 {
    return if b { gl::TRUE } else { gl::FALSE };
}

fn gl_data_type(t: data::Type) -> u32 {
    match data::scalar_type_of(t) {
        data::F32 => gl::FLOAT,
        data::F64 => gl::DOUBLE,
        data::U32 => gl::UNSIGNED_INT,
        data::I32 => gl::INT,
        data::U16 => gl::UNSIGNED_SHORT,
        data::I16 => gl::SHORT,
        data::U8 =>  gl::UNSIGNED_BYTE,
        data::I8 =>  gl::BYTE,
        _ => 0
    }
}

fn gl_data_type_from_format(fmt: PixelFormat) -> u32 {
    match fmt {
        A_F32 => gl::FLOAT,
        _ => gl::UNSIGNED_BYTE,
    }
}

pub fn gl_error_str(err: ErrorCode) -> &'static str {
    return match err {
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

fn from_gl_error(err: u32) -> ResultCode {
    match err {
        gl::NO_ERROR            => { OK }
        gl::INVALID_ENUM        => { INVALID_ARGUMENT_ERROR }
        gl::INVALID_VALUE       => { INVALID_ARGUMENT_ERROR }
        gl::INVALID_OPERATION   => { INVALID_ARGUMENT_ERROR }
        gl::OUT_OF_MEMORY       => { OUT_OF_MEMORY_ERROR }
        gl::FRAMEBUFFER_INCOMPLETE_MISSING_ATTACHMENT => { RT_MISSING_ATTACHMENT_ERROR }
        gl::FRAMEBUFFER_INCOMPLETE_ATTACHMENT => { RT_INCOMPLETE_ATTACHMENT_ERROR }
        gl::FRAMEBUFFER_INCOMPLETE_DRAW_BUFFER => { UNKNOWN_ERROR }  // TODO
        gl::FRAMEBUFFER_INCOMPLETE_MULTISAMPLE => { UNKNOWN_ERROR }, // TODO
        gl::FRAMEBUFFER_UNSUPPORTED => { RT_UNSUPPORTED_ERROR }
        _ => { UNKNOWN_ERROR }
    }
}

fn set_texture_flags(tex_handle: u32, flags: TextureFlags) {
    if flags == 0 { return; }
    gl::BindTexture(gl::TEXTURE_2D, tex_handle);
    if flags&REPEAT_S != 0 {
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::REPEAT as i32);
    }
    if flags&REPEAT_T != 0 {
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::REPEAT as i32);
    }
    if flags&CLAMP_S != 0 {
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
    }
    if flags&CLAMP_T != 0 {
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
    }
    if flags&MIN_FILTER_LINEAR != 0 {
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
    }
    if flags&MAG_FILTER_LINEAR != 0 {
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
    }
    if flags&MIN_FILTER_NEAREST != 0 {
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
    }
    if flags&MAG_FILTER_NEAREST != 0 {
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
    }
    gl::BindTexture(gl::TEXTURE_2D, 0);
}

struct DrawArraysIndirectCommand {
    count: u32,
    primitive_count: u32,
    first_vertex: u32,
    base_instance: u32,
}

pub struct OpenGLDeviceBackend {
    current_render_target: RenderTargetObject,
    current_program: ShaderPipelineObject,
    current_geometry: GeometryObject,
    current_target_types: TargetTypes,
    current_blend_mode: BlendMode,
    error_flags: ErrorFlags,
}

impl OpenGLDeviceBackend {
    fn check_errors(&mut self) -> ResultCode {
        let gl_error = gl::GetError();
        if gl_error == gl::NO_ERROR {
            return OK;
        }
        if self.error_flags & LOG_ERRORS != 0 {
            println!("GL Error: 0x{:x} ({})", gl_error, gl_error_str(gl_error));
        }
        if self.error_flags & CRASH_ERRORS !=0 {
            fail!("Aborted due to GL error.");
        }
        return from_gl_error(gl_error);
    }

    fn update_targets(&mut self, targets: TargetTypes) {
        if (targets & DEPTH != 0) && (self.current_target_types & DEPTH == 0) {
            gl::Enable(gl::DEPTH_TEST);
            self.current_target_types |= DEPTH;
        } else if (targets & DEPTH == 0) && (self.current_target_types & DEPTH != 0) {
            gl::Disable(gl::DEPTH_TEST);
            self.current_target_types &= COLOR | STENCIL;
        }
    }

    fn update_blend_mode(&mut self, blend: BlendMode) {
        if blend == self.current_blend_mode {
            return;
        }
        if blend == NO_BLENDING {
            gl::Disable(gl::BLEND);
        } else {
            gl::Enable(gl::BLEND);
            if blend == ALPHA_BLENDING {
                gl::BlendFunc(gl::SRC_ALPHA,gl::ONE_MINUS_SRC_ALPHA);
            } else {
                fail!("Unimplemented");
            }
        }
    }
}

impl DeviceBackend for OpenGLDeviceBackend {
    fn is_supported(
        &mut self,
        feature: Feature
    ) -> bool {
        return match feature {
            FRAGMENT_SHADING => true,
            VERTEX_SHADING => true,
            GEOMETRY_SHADING => false,
            COMPUTE => false,
            DEPTH_TEXTURE => false,
            RENDER_TO_TEXTURE => false,
            MULTIPLE_RENDER_TARGETS => false,
            INSTANCED_RENDERING => false,
        };
    }

    fn set_viewport(&mut self, x:i32, y:i32, w:i32, h:i32) {
        gl::Viewport(x,y,w,h);
    }

    fn create_texture(
        &mut self,
        descriptor: &TextureDescriptor,
        texture: &mut TextureObject
    ) -> ResultCode {
        unsafe {
            gl::GenTextures(1, &mut texture.handle);
        }
        set_texture_flags(texture.handle, descriptor.flags);
        return OK;
    }

    fn destroy_texture(
        &mut self,
        texture: TextureObject
    ) {
        unsafe {
            gl::DeleteTextures(1, &texture.handle);
        }
    }

    fn set_texture_flags(
        &mut self,
        texture: TextureObject,
        flags: TextureFlags
    ) -> ResultCode {
        if texture.handle != 0 {
            set_texture_flags(texture.handle, flags);
            return OK;
        }
        return INVALID_OBJECT_HANDLE_ERROR;
    }

    fn create_shader_stage(
        &mut self,
        descriptor: &ShaderStageDescriptor,
        shader: &mut ShaderStageObject
    ) -> ResultCode {
        shader.handle = gl::CreateShader(gl_shader_type(descriptor.stage_type));
        unsafe {
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
            match self.check_errors() {
                OK => {}
                error => { return error; }
            }

            gl::CompileShader(shader.handle);

            match self.check_errors() {
                OK => {}
                error => { return error; }
            }
        }
        return OK;
    }

    fn get_shader_stage_result(
        &mut self,
        shader: ShaderStageObject,
        result: &mut ShaderBuildResult,
    ) -> ResultCode {
        unsafe {
            let mut status : i32 = 0;
            gl::GetShaderiv(shader.handle, gl::COMPILE_STATUS, &mut status);
            match self.check_errors() {
                OK => {}
                error => { return error; }
            }
            if status == gl::TRUE as i32 {
                result.code = OK;
                return OK;
            }
            let mut buffer = [0u8, ..512];
            let mut length: i32 = 0;
            gl::GetShaderInfoLog(
                shader.handle, 512, &mut length,
                mem::transmute(buffer.as_mut_ptr())
            );
            result.code = SHADER_COMPILATION_ERROR;
            result.details = raw::from_buf(buffer.as_ptr());
            return SHADER_COMPILATION_ERROR;
        }
    }

    fn destroy_shader_stage(
        &mut self,
        stage: ShaderStageObject
    ) {
        gl::DeleteShader(stage.handle);
    }

    fn create_shader_pipeline(
        &mut self,
        descriptor: &ShaderPipelineDescriptor,
        pipeline: &mut ShaderPipelineObject
    ) -> ResultCode {
        pipeline.handle = gl::CreateProgram();
        for stage in descriptor.stages.iter() {
            gl::AttachShader(pipeline.handle, stage.handle);
        }

        for &(ref name, loc) in descriptor.attrib_locations.iter() {
            if loc < 0 {
                gl::DeleteProgram(pipeline.handle);
                return INVALID_ARGUMENT_ERROR;
            }
            unsafe {
                name.with_c_str(|c_name| {
                    gl::BindAttribLocation(pipeline.handle, loc as u32, c_name);
                });
            }
        }

        gl::LinkProgram(pipeline.handle);
        return OK;
    }

    fn get_shader_pipeline_result(
        &mut self,
        shader: ShaderPipelineObject,
        result: &mut ShaderBuildResult,
    ) -> ResultCode {
        if shader.handle == 0 {
            return INVALID_OBJECT_HANDLE_ERROR;
        }

        unsafe {
            gl::ValidateProgram(shader.handle);
            match self.check_errors() {
                OK => {}
                error => { println!("validate error {}", error); }
            }

            let mut status: i32 = 0;
            gl::GetProgramiv(shader.handle, gl::VALIDATE_STATUS, &mut status);
            match self.check_errors() {
                OK => {}
                error => { println!("GetProgramiv error {}", error); }
            }

            if status == gl::TRUE as i32 {
                return OK;
            }

            let mut buffer = [0u8, ..512];
            let mut length: i32 = 0;
            gl::GetProgramInfoLog(
                shader.handle, 512, &mut length,
                mem::transmute(buffer.as_mut_ptr())
            );

            result.code = SHADER_LINK_ERROR;
            result.details = raw::from_buf(buffer.as_ptr());
            return SHADER_LINK_ERROR;
        }
    }

    fn destroy_shader_pipeline(
        &mut self,
        pipeline: ShaderPipelineObject
    ) {
        gl::DeleteProgram(pipeline.handle);
    }

    fn create_buffer(
        &mut self,
        descriptor: &BufferDescriptor,
        buffer: &mut BufferObject,
    ) -> ResultCode {
        unsafe {
            gl::GenBuffers(1, &mut buffer.handle);
            match self.check_errors() {
                OK => {}
                error => { return error; }
            }

            buffer.size = descriptor.size;
            buffer.buffer_type = descriptor.buffer_type;

            if descriptor.size == 0 {
                return OK;
            }

            gl::BindBuffer(
                gl_buffer_type(descriptor.buffer_type),
                buffer.handle
            );
            match self.check_errors() {
                OK => {}
                error => { return error; }
            }

            gl::BufferData(
                gl_buffer_type(descriptor.buffer_type),
                descriptor.size as i64,
                0 as *const c_void,
                gl_update_hint(descriptor.update_hint)
            );
            match self.check_errors() {
                OK => {}
                error => { return error; }
            }
        }

        return OK;
    }

    fn destroy_buffer(
        &mut self,
        buffer: BufferObject
    ) {
        unsafe {
            gl::DeleteBuffers(1, &buffer.handle);
            self.check_errors();
        }
    }

    unsafe fn map_buffer(
        &mut self,
        buffer: BufferObject,
        flags: MapFlags,
        data: *mut *mut u8
    ) -> ResultCode {
        if buffer.handle == 0 {
            return INVALID_OBJECT_HANDLE_ERROR;
        }

        let gl_target = gl_buffer_type(buffer.buffer_type);

        gl::BindBuffer(gl_target, buffer.handle);
        match self.check_errors() {
            OK => {}
            error => { return error; }
        }

        *data = gl::MapBuffer(
            gl_target,
            gl_access_flags(flags)
        ) as *mut u8;
        match self.check_errors() {
            OK => {}
            error => { return error; }
        }

        return OK;
    }

    fn unmap_buffer(
        &mut self,
        buffer: BufferObject
    ) {
        gl::UnmapBuffer(gl_buffer_type(buffer.buffer_type));
        self.check_errors();
    }

    fn destroy_geometry(
        &mut self,
        geom: GeometryObject
    ) {
        unsafe {
            gl::DeleteVertexArrays(1, &geom.handle);
            self.check_errors();
        }
    }

    fn create_geometry(
        &mut self,
        descriptor: &GeometryDescriptor,
        output: &mut GeometryObject
    ) -> ResultCode {
        let mut handle: u32 = 0;
        unsafe {
            gl::GenVertexArrays(1, &mut handle);
            match self.check_errors() {
                OK => {}
                error => { return error; }
            }
        }

        gl::BindVertexArray(handle);
        match self.check_errors() {
            OK => {}
            error => { return error; }
        }

        for attr in descriptor.attributes.iter() {
            gl::BindBuffer(gl::ARRAY_BUFFER, attr.buffer.handle);
            match self.check_errors() {
                OK => {}
                error => { return error; }
            }
            unsafe {
                gl::VertexAttribPointer(
                    attr.location as u32,
                    data::num_components(attr.attrib_type) as i32,
                    gl_data_type(attr.attrib_type),
                    gl_bool(attr.normalize),
                    attr.stride as i32,
                    mem::transmute(attr.offset as uint)
                );
                match self.check_errors() {
                    OK => {}
                    error => { return error; }
                }
                gl::EnableVertexAttribArray(attr.location as u32);
                match self.check_errors() {
                    OK => {}
                    error => { return error; }
                }
            }
        }

        match descriptor.index_buffer {
            Some(ibo) => {
                gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, ibo.handle);
                match self.check_errors() {
                    OK => {}
                    error => { return error; }
                }
            }
            None => {}
        }

        gl::BindVertexArray(0);

        *output = GeometryObject {
            handle: handle,
        };
        return OK;
    }

    fn get_shader_input_location(
        &mut self,
        shader: ShaderPipelineObject,
        name: &str
    ) -> ShaderInputLocation {
        if shader.handle == 0 {
            return -1;
        }
        let mut location = 0;
        name.with_c_str(|c_name| unsafe {
            location = gl::GetUniformLocation(shader.handle, c_name) as ShaderInputLocation;
        });
        self.check_errors();
        return location;
    }

    fn get_vertex_attribute_location(
        &mut self,
        shader: ShaderPipelineObject,
        name: &str
    ) -> VertexAttributeLocation {
        if shader.handle == 0 {
            return -1;
        }
        let mut location = 0;
        name.with_c_str(|c_name| unsafe {
            location = gl::GetAttribLocation(shader.handle, c_name) as VertexAttributeLocation;
        });
        self.check_errors();
        return location;
    }

    fn create_render_target(
        &mut self,
        descriptor: &RenderTargetDescriptor,
        target: &mut RenderTargetObject,
    ) -> ResultCode {
        let mut fbo: u32 = 0;
        unsafe {
            gl::GenFramebuffers(1, &mut fbo);
        }

        gl::BindFramebuffer(gl::FRAMEBUFFER, fbo);
        match self.check_errors() {
            OK => {}
            error => { return error; }
        }

        for i in range(0, descriptor.color_attachments.len()) {
            gl::FramebufferTexture2D(
                gl::FRAMEBUFFER,
                gl_attachement(i as u32),
                gl::TEXTURE_2D,
                descriptor.color_attachments[i].handle,
                0
            );
            match self.check_errors() {
                OK => {}
                error => { return error; }
            }
        }

        match descriptor.depth {
            Some(d) => {
                gl::FramebufferTexture2D(
                    gl::FRAMEBUFFER,
                    gl::DEPTH_ATTACHMENT,
                    gl::TEXTURE_2D,
                    d.handle,
                    0
                );
                match self.check_errors() {
                    OK => {}
                    error => { return error; }
                }
            }
            _ => {}
        }

        match descriptor.stencil {
            Some(s) => {
                gl::FramebufferTexture2D(
                    gl::FRAMEBUFFER,
                    gl::STENCIL_ATTACHMENT,
                    gl::TEXTURE_2D,
                    s.handle,
                    0
                );
                match self.check_errors() {
                    OK => {}
                    error => { return error; }
                }
            }
            _ => {}
        }

        let status = gl::CheckFramebufferStatus(gl::FRAMEBUFFER);
        gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
        if status != gl::FRAMEBUFFER_COMPLETE {
            unsafe {
                gl::DeleteFramebuffers(1, &fbo);
            }
            return from_gl_error(status);
        }

        target.handle = fbo;
        return OK;
    }

    fn destroy_render_target(
        &mut self,
        target: RenderTargetObject
    ) {
        unsafe {
            gl::DeleteFramebuffers(1, &target.handle);
            self.check_errors();
        }
    }

    fn get_default_render_target(&mut self) -> RenderTargetObject {
        RenderTargetObject { handle: 0 }
    }

    fn copy_buffer_to_texture(
        &mut self,
        buffer: BufferObject,
        texture: TextureObject
    ) -> ResultCode {
        unsafe {
            let mut width: i32 = 0;
            let mut height: i32 = 0;
            let mut format: i32 = 0;

            gl::BindTexture(gl::TEXTURE_2D, texture.handle);
            match self.check_errors() {
                OK => {}
                error => { return error; }
            }
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
            match self.check_errors() {
                OK => {}
                error => { return error; }
            }

            gl::BindBuffer(gl::PIXEL_UNPACK_BUFFER, buffer.handle);
            match self.check_errors() {
                OK => {}
                error => { return error; }
            }
            // TODO: support other formats
            gl::TexSubImage2D(gl::TEXTURE_2D, 0, 0, 0, width, height,
                            gl::RGBA, gl::UNSIGNED_BYTE, 0 as *const c_void);
            match self.check_errors() {
                OK => {}
                error => { return error; }
            }

            gl::BindBuffer(gl::PIXEL_UNPACK_BUFFER, 0);
            gl::BindTexture(gl::TEXTURE_2D, 0);
        }
        return OK;
    }

    fn copy_texture_to_buffer(
        &mut self,
        texture: TextureObject,
        buffer: BufferObject
    ) -> ResultCode {
        unsafe {
            gl::BindTexture(gl::TEXTURE_2D, texture.handle);
            match self.check_errors() {
                OK => {}
                error => { return error; }
            }
            gl::BindBuffer(gl::PIXEL_PACK_BUFFER, buffer.handle);
            match self.check_errors() {
                OK => {}
                error => { return error; }
            }
            gl::GetTexImage(
                gl::TEXTURE_2D, 0,              // TODO: mip levels
                gl::RGBA, gl::UNSIGNED_BYTE,    // TODO: support more formats
                0 as *mut c_void                // offset in the buffer
            );
            match self.check_errors() {
                OK => {}
                error => { return error; }
            }

            gl::BindBuffer(gl::PIXEL_PACK_BUFFER, 0);
            gl::BindTexture(gl::TEXTURE_2D, 0);
        }
        return OK;
    }

    fn copy_buffer_to_buffer(
        &mut self,
        src_buffer: BufferObject,
        dest_buffer: BufferObject,
        src_offset: u16,
        dest_offset: u16,
        size: u16
    ) -> ResultCode {
        gl::BindBuffer(gl::COPY_READ_BUFFER, src_buffer.handle);
        gl::BindBuffer(gl::COPY_WRITE_BUFFER, dest_buffer.handle);
        match self.check_errors() {
            OK => {}
            error => { return error; }
        }

        gl::CopyBufferSubData(
            gl::COPY_READ_BUFFER, gl::COPY_WRITE_BUFFER,
            src_offset as i64,
            dest_offset as i64,
            size as i64
        );
        match self.check_errors() {
            OK => {}
            error => { return error; }
        }

        gl::BindBuffer(gl::COPY_READ_BUFFER, 0);
        gl::BindBuffer(gl::COPY_WRITE_BUFFER, 0);
        match self.check_errors() {
            OK => {}
            error => { return error; }
        }
        return OK;
    }

    fn bind_uniform_buffer(
        &mut self,
        binding_index: UniformBindingIndex,
        ubo: BufferObject,
        range: Option<(u16, u16)>
    ) -> ResultCode {

        match range {
            Some((start, size)) => {
                gl::BindBufferRange(
                    gl::UNIFORM_BUFFER,
                    binding_index as u32,
                    ubo.handle,
                    start as i64,
                    size as i64
                );
            }
            None => {
                gl::BindBufferBase(
                    gl::UNIFORM_BUFFER,
                    binding_index as u32,
                    ubo.handle
                );
            }
        }
        return self.check_errors();
    }

    fn set_uniform_block(
        &mut self,
        shader: ShaderPipelineObject,
        block_index: UniformBlockLocation,
        binding_index: UniformBindingIndex,
    ) -> ResultCode {
        gl::UniformBlockBinding(
            shader.handle,
            block_index.index as u32,
            binding_index as u32
        );
        return self.check_errors();
    }

    fn get_uniform_block_location(
        &mut self,
        shader: ShaderPipelineObject,
        name: &str
    ) -> UniformBlockLocation {
        let mut result = UniformBlockLocation { index: -1 };
        name.with_c_str(|c_name| unsafe {
            result.index = gl::GetUniformBlockIndex(shader.handle, c_name) as i16;
        });
        return result;
    }

    fn set_shader(&mut self, shader: ShaderPipelineObject) -> ResultCode {
        println!("current_program: {}", self.current_program);
        self.check_errors();
        println!("meh");
        if self.current_program == shader {
            println!("skip");
            return OK;
        }
        self.current_program = shader;
        gl::UseProgram(shader.handle);
        return self.check_errors();
    }

    fn draw(&mut self,
        geom: GeometryObject,
        range: Range,
        flags: GeometryFlags,
        blend: BlendMode,
        targets: TargetTypes
    ) -> ResultCode {
        self.update_targets(targets);
        self.update_blend_mode(blend);

        if geom != self.current_geometry {
            self.current_geometry = geom;
            gl::BindVertexArray(geom.handle);
        };

        match range {
            VertexRange(first, count) => {
                gl::DrawArrays(
                    gl_draw_mode(flags),
                    first as i32,
                    count as i32
                );
                return self.check_errors();
            }
            IndexRange(first, count) => {
                unsafe {
                    gl::DrawElements(
                        gl_draw_mode(flags),
                        count as i32,
                        gl::UNSIGNED_SHORT,
                        // /2 because offset in bytes with u16
                        (first / 2) as *const c_void
                    );
                }
                return self.check_errors();
            }
        }
    }

    fn flush(&mut self) -> ResultCode {
        gl::Flush();
        return self.check_errors();
    }

    fn clear(&mut self, targets: TargetTypes) -> ResultCode {
        gl::Clear(gl_clear_targets(targets));
        return self.check_errors();
    }

    fn set_clear_color(&mut self, r:f32, g: f32, b: f32, a: f32) {
        gl::ClearColor(r, g, b, a);
        self.check_errors();
    }
}

pub fn create_device() -> Device<OpenGLDeviceBackend> {
    Device {
        backend: OpenGLDeviceBackend {
            current_program: ShaderPipelineObject { handle: 0 },
            current_render_target: RenderTargetObject { handle: 0 },
            current_geometry: GeometryObject { handle: 0 },
            current_target_types: 0,
            current_blend_mode: NO_BLENDING,
            error_flags: IGNORE_ERRORS,
        }
    }
}

pub fn create_debug_device(err_flags: ErrorFlags) -> Device<LoggingProxy<OpenGLDeviceBackend>> {
    Device {
        backend: LoggingProxy {
            backend: OpenGLDeviceBackend {
                current_program: ShaderPipelineObject { handle: 0 },
                current_render_target: RenderTargetObject { handle: 0 },
                current_geometry: GeometryObject { handle: 0 },
                current_target_types: 0,
                current_blend_mode: NO_BLENDING,
                error_flags: err_flags,
            }
        }
    }
}
