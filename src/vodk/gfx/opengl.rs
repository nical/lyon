use gl;
use glfw;
use gpu = gfx::renderer;
use std::str;
use std::mem;
use glfw;
use std::rc::Rc;

use super::renderer::*;

macro_rules! check_err (
    ($($arg:tt)*) => (
        if !self.ignore_errors {
            match gl::GetError() {
                gl::NONE => {}
                e => {
                    return Err(Error{
                        code: e,
                        detail: Some(format!($($arg)*))
                    });
                }
            }
        }
    )
)

type DriverBugs = u64;
pub static DRIVER_DEFAULT : DriverBugs = 0;
pub static MISSING_INDEX_BUFFER_VAO : DriverBugs = 1;

pub struct RenderingContextGL {
    window: Rc<glfw::Window>,
    workaround: DriverBugs,
    current_render_target: RenderTarget,
    current_program: ShaderProgram,
    current_geometry: Geometry,
    current_target_types: TargetTypes,
    ignore_errors: bool,
}

impl RenderingContextGL {
    pub fn new(window: Rc<glfw::Window>) -> RenderingContextGL {
        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::SRC_ALPHA,gl::ONE_MINUS_SRC_ALPHA);

        RenderingContextGL {
            window: window,
            workaround: DRIVER_DEFAULT,
            current_program: ShaderProgram { handle: 0 },
            current_render_target: RenderTarget { handle: 0 },
            current_geometry: Geometry { handle: 0, ibo: 0 },
            current_target_types: 0,
            ignore_errors: false,
        }
    }
}

impl RenderingContext for RenderingContextGL {
    fn make_current(&mut self) -> bool {
        (self.window.deref() as &glfw::Context).make_current();
        return true;
    }

    fn swap_buffers(&mut self) {
        (self.window.deref() as &glfw::Context).swap_buffers();
    }

    fn reset_state(&mut self) {
        gl::BindTexture(gl::TEXTURE_2D, 0);
        gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
        gl::UseProgram(0);
        gl::BindVertexArray(0);
        gl::BindBuffer(gl::ARRAY_BUFFER, 0);
        gl::ClearColor(0.0, 0.0, 0.0, 0.0);
        self.current_render_target = self.get_default_render_target();
        self.current_program = ShaderProgram { handle: 0 };
        self.current_geometry = Geometry { handle: 0, ibo: 0 };

        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::SRC_ALPHA,gl::ONE);
    }

    fn check_error(&mut self) -> Option<String> {
        match gl::GetError() {
            gl::NO_ERROR            => None,
            gl::INVALID_ENUM        => Some("Invalid enum.".to_string()),
            gl::INVALID_VALUE       => Some("Invalid value.".to_string()),
            gl::INVALID_OPERATION   => Some("Invalid operation.".to_string()),
            gl::OUT_OF_MEMORY       => Some("Out of memory.".to_string()),
            _ => Some("Unknown error.".to_string()),
        }
    }

    fn get_error_str(&mut self, err: ErrorCode) -> &'static str {
        return gl_error_str(err);
    }

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
        if flags == 0 { return; }
        gl::BindTexture(gl::TEXTURE_2D, tex.handle);
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

    fn upload_texture_data(&mut self, dest: Texture, data: &[u8],
                           w:u32, h:u32, format: PixelFormat) -> RendererResult {
        gl::BindTexture(gl::TEXTURE_2D, dest.handle);

        let fmt = gl_format(format);
        unsafe {
            gl::TexImage2D(
                gl::TEXTURE_2D, 0, fmt as i32, w as i32, h as i32,
                0, fmt, gl::UNSIGNED_BYTE, mem::transmute(data.unsafe_ref(0))
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
                0, fmt, gl::UNSIGNED_BYTE, mem::transmute(0)
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
                            gl::UNSIGNED_BYTE, mem::transmute(dest.unsafe_ref(0)));
            check_err!("glGetTexImage(...) on texture {}", tex.handle);
        }
        return Ok(());
    }

    fn create_shader(&mut self, t: ShaderType) -> Shader {
        return Shader { handle: gl::CreateShader(gl_shader_type(t)) };
    }

    fn destroy_shader(&mut self, s: Shader) {
        gl::DeleteShader(s.handle);
    }

    fn create_shader_program(&mut self) -> ShaderProgram {
        return ShaderProgram { handle: gl::CreateProgram() };
    }

    fn destroy_shader_program(&mut self, p: ShaderProgram) {
        gl::DeleteProgram(p.handle);
    }

    fn compile_shader(&mut self, shader: Shader, src: &[&str]) -> RendererResult {
        unsafe {
            let mut lines: Vec<*i8> = Vec::new();
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
                let mut buffer : Vec<u8> = Vec::from_fn(512, |_|{0});
                let mut length: i32 = 0;
                gl::GetShaderInfoLog(shader.handle, 512, &mut length,
                                     mem::transmute(buffer.as_mut_slice().unsafe_mut_ref(0)));

                return Err( Error {
                    code: 0,
                    detail: Some(str::raw::from_utf8_owned(buffer)),
                });
            }
            return Ok(());
        }
    }

    fn link_shader_program(&mut self, p: gpu::ShaderProgram,
                           shaders: &[gpu::Shader],
                           attrib_locations: &[(&str, VertexAttributeLocation)]) -> RendererResult {
        unsafe {
            for s in shaders.iter() {
                gl::AttachShader(p.handle, s.handle);
            }

            for &(ref name, loc) in attrib_locations.iter() {
                if loc < 0 {
                    return Err(Error {
                        code: 0,
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
                let mut buffer :Vec<u8> = Vec::from_fn(512, |_|{0});
                let mut length = 0;
                gl::GetProgramInfoLog(p.handle, 512, &mut length,
                                      mem::transmute(buffer.as_mut_slice().unsafe_mut_ref(0)));

                return Err( Error {
                    code: 0,
                    detail: Some(str::raw::from_utf8_owned(buffer)),
                });
            }
            return Ok(());
        }
    }

    fn create_buffer(&mut self) -> Buffer {
        let mut b: u32 = 0;
        unsafe {
            gl::GenBuffers(1, &mut b);
        }
        return Buffer { handle: b };
    }

    fn destroy_buffer(&mut self, buffer: Buffer) {
        unsafe {
            gl::DeleteBuffers(1, &buffer.handle);
        }
    }

    fn upload_buffer(&mut self, buffer: Buffer, buf_type: BufferType,
                     update: UpdateHint, data: &[u8]) -> RendererResult {

        unsafe {
            let gl_buf_type = gl_buffer_type(buf_type);
            gl::BindBuffer(gl_buf_type, buffer.handle);
            check_err!("glBindBuffer({}, {})", buf_type, buffer.handle);
            gl::BufferData(gl_buf_type, data.len() as i64,
                           mem::transmute(data.unsafe_ref(0)),
                           gl_update_hint(update));
            check_err!("glBufferData({}, {}, {}, {})", buf_type,
                        data.len(), data.unsafe_ref(0),
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
                           mem::transmute(0),
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
            gl::VertexAttribPointer(attr.location as u32,
                                    attr.components as i32,
                                    gl_attribue_type(attr.attrib_type),
                                    gl_bool(attr.normalize),
                                    attr.stride as i32,
                                    mem::transmute(attr.offset as uint));
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

    fn get_shader_input_location(&mut self, program: ShaderProgram,
                                 name: &str) -> ShaderInputLocation {
        let mut location = 0;
        name.with_c_str(|c_name| unsafe {
            location = gl::GetUniformLocation(program.handle, c_name) as ShaderInputLocation;
        });
        return location;
    }

    fn get_vertex_attribute_location(&mut self, program: ShaderProgram,
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
            return Err(Error{code: status, detail: None });
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

    fn set_render_target(&mut self, target: gpu::RenderTarget) {
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
                2 => { gl::UniformMatrix2fv(location as i32, 1, gl_bool(transpose), mem::transmute(input.unsafe_ref(0))); }
                3 => { gl::UniformMatrix3fv(location as i32, 1, gl_bool(transpose), mem::transmute(input.unsafe_ref(0))); }
                4 => { gl::UniformMatrix4fv(location as i32, 1, gl_bool(transpose), mem::transmute(input.unsafe_ref(0))); }
                _ => { fail!("Invalid matrix dimension"); }
            }
        }
    }

    fn set_shader_input_texture(&mut self, location: ShaderInputLocation, texture_unit: u32, tex: gpu::Texture) {
        gl::ActiveTexture(gl_texture_unit(texture_unit));
        gl::BindTexture(gl::TEXTURE_2D, tex.handle);
        gl::Uniform1i(location as i32, texture_unit as i32);
        //gl::BindTexture(gl::TEXTURE_2D, 0);
    }

    fn set_shader(&mut self, program: gpu::ShaderProgram) -> RendererResult {
        if self.current_program != program {
            self.current_program = program;
            gl::UseProgram(program.handle);
            check_err!("glUseProgram({})", program.handle);
        }
        return Ok(());
    }

    fn draw(&mut self, geom: gpu::GeometryRange, targets: TargetTypes) -> RendererResult {
        if (targets & DEPTH != 0) && (self.current_target_types & DEPTH == 0) {
            gl::Enable(gl::DEPTH_TEST);
            self.current_target_types |= DEPTH;
        } else if (targets & DEPTH == 0) && (self.current_target_types & DEPTH != 0) {
            gl::Disable(gl::DEPTH_TEST);
            self.current_target_types &= COLOR | STENCIL;
        }

        if geom.geometry != self.current_geometry {
            self.current_geometry = geom.geometry;
            gl::BindVertexArray(geom.geometry.handle);
            check_err!("glBindVertexArray({})", geom.geometry.handle);
        };

        if geom.geometry.ibo != 0 {
            if self.workaround & MISSING_INDEX_BUFFER_VAO != 0 {
                gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, geom.geometry.ibo);
            }
            unsafe {
                gl::DrawElements(gl_draw_mode(geom.flags),
                                 geom.to as i32,
                                 gl::UNSIGNED_SHORT,
                                 mem::transmute(0));
            }
            check_err!("glDrawElements(...)");
        } else {
            gl::DrawArrays(gl_draw_mode(geom.flags),
                           geom.from as i32,
                           geom.to as i32);
            check_err!("glDrawArrays(...)");
        }
        Ok(())
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
    }
}

fn gl_update_hint(hint: UpdateHint) -> u32 {
    match hint {
        STATIC => gl::STATIC_DRAW,
        STREAM => gl::STREAM_DRAW,
        DYNAMIC => gl::DYNAMIC_DRAW,
    }
}

fn gl_attribue_type(attribute: AttributeType) -> u32 {
    match attribute {
        F32 => gl::FLOAT,
        F64 => gl::DOUBLE,
        I32 => gl::INT,
        U32 => gl::UNSIGNED_INT,
    }
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
