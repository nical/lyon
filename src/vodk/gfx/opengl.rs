use gl;
use glfw;
use gpu = gfx::renderer;
use std::str;
use std::cast;
use std::libc::c_void;
use std::rc::Rc;
use std::mem::size_of;

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

pub struct RenderingContextGL {
    //window: Rc<glfw::Window>,
    current_texture: Texture,
    current_render_target: RenderTarget,
    current_program: ShaderProgram,
    current_geometry: Geometry,
    ignore_errors: bool,
}

impl RenderingContextGL {
    pub fn new() -> RenderingContextGL {
        RenderingContextGL {
            current_texture: Texture { handle: 0 },
            current_program: ShaderProgram { handle: 0 },
            current_render_target: RenderTarget { handle: 0 },
            current_geometry: Geometry { handle: 0 },
            ignore_errors: false,
        }
    }

    fn use_render_target(&mut self, fbo: RenderTarget) {
        if self.current_render_target == fbo {
            return;
        }
        gl::BindFramebuffer(gl::FRAMEBUFFER, fbo.handle);
        self.current_render_target = fbo;
    }

    fn render_command(&mut self, command: &RenderingCommand) -> RendererResult {
        match *command {
            OpFlush => { gl::Flush(); }
            OpClear => { gl::Clear(gl::COLOR_BUFFER_BIT); }
            OpDraw(ref draw) => {
                if self.current_program != draw.shader_program {
                    self.current_program = draw.shader_program;
                    gl::UseProgram(draw.shader_program.handle);
                    check_err!("glUseProgram({})",draw.shader_program.handle);
                }
                let mut num_tex = 0;
                for input in draw.shader_inputs.iter() {
                    match input.value {
                        INPUT_FLOATS(ref f) => {
                            match f.len() {
                                1 => { gl::Uniform1f(input.location, f[0]); }
                                2 => { gl::Uniform2f(input.location, f[0], f[1]); }
                                3 => { gl::Uniform3f(input.location, f[0], f[1], f[2]); }
                                4 => { gl::Uniform4f(input.location, f[0], f[1], f[2], f[3]); }
                                x => { return Err(Error{
                                    code: 0,
                                    detail: Some(~"Unsupported unform size > 4") });
                                }
                            }
                        }
                        INPUT_TEXTURE(tex) => {
                            gl::ActiveTexture(gl_texture_unit(num_tex));
                            check_err!("glActiveTexture({})", gl_texture_unit(num_tex));
                            gl::BindTexture(gl::TEXTURE_2D, tex.handle);
                            check_err!("glBindTexture(GL_TEXTURE_2D, {})", tex.handle);
                            gl::Uniform1i(input.location, num_tex as i32);
                            check_err!("glUniform1i({}, {})", input.location, num_tex);
                            num_tex += 1;
                        }
                        // TODO matrices
                    }
                }

                if (draw.geometry != self.current_geometry) {
                    self.current_geometry = draw.geometry;
                    gl::BindVertexArray(draw.geometry.handle);
                };

                if draw.flags & INDEXED != 0 {
                    unsafe {
                        gl::DrawElements(gl_draw_mode(draw.flags),
                                         draw.count as i32,
                                         gl::UNSIGNED_INT,
                                         cast::transmute(0));
                        check_err!("glDrawElements({}, {}, GL_UNSIGNED_INT, 0)",
                                   gl_draw_mode(draw.flags), draw.count);

                    }
                } else {
                    gl::DrawArrays(gl_draw_mode(draw.flags),
                                   draw.first as i32,
                                   draw.count as i32);
                    check_err!("glDrawArrays({}, {}, {})",
                               gl_draw_mode(draw.flags), draw.first, draw.count);
                }
            }
        }
        return Ok(());
    }
}

impl RenderingContext for RenderingContextGL {
    fn make_current(&mut self) -> bool {
        return true; // TODO
    }

    fn reset_state(&mut self) {
        self.current_render_target = self.get_default_render_target();
        self.current_texture = Texture { handle: 0 };
        self.current_program = ShaderProgram { handle: 0 };
        self.current_geometry = Geometry { handle: 0 };
    }

    fn check_error(&mut self) -> Option<~str> {
        match gl::GetError() {
            gl::NO_ERROR            => None,
            gl::INVALID_ENUM        => Some(~"Invalid enum."),
            gl::INVALID_VALUE       => Some(~"Invalid value."),
            gl::INVALID_OPERATION   => Some(~"Invalid operation."),
            gl::OUT_OF_MEMORY       => Some(~"Out of memory."),
            _ => Some(~"Unknown error."),
        }
    }

    fn get_error_str(&mut self, err: ErrorCode) -> &'static str {
        return gl_error_str(err);
    }

    fn is_supported(&mut self, f: Feature) -> bool {
        match f {
            FRAGMENT_SHADING => true,
            VERTEX_SHADING => true,
            GEOMETRY_SHADING => false,
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

    fn clear(&mut self) {
        gl::Clear(gl::COLOR_BUFFER_BIT);
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
        gl::BindTexture(gl::TEXTURE_2D, tex.handle);
        if flags&TEXTURE_REPEAT_S != 0 {
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::REPEAT as i32);
        }
        if flags&TEXTURE_REPEAT_T != 0 {
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::REPEAT as i32);
        }
        if flags&TEXTURE_CLAMP_S != 0 {
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
        }
        if flags&TEXTURE_CLAMP_T != 0 {
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
        }
        if flags&TEXTURE_MIN_FILTER_LINEAR != 0 {
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
        }
        if flags&TEXTURE_MAG_FILTER_LINEAR != 0 {
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
        }
        if flags&TEXTURE_MIN_FILTER_NEAREST != 0 {
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
        }
        if flags&TEXTURE_MAG_FILTER_NEAREST != 0 {
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
        }
        gl::BindTexture(gl::TEXTURE_2D, 0);
    }

    fn upload_texture_data(&mut self, dest: Texture,
                           data: &[u8], format: PixelFormat,
                           w:u32, h:u32, stride: u32) -> RendererResult {
        gl::BindTexture(gl::TEXTURE_2D, dest.handle);
        let fmt = gl_format(format);
        unsafe {
            gl::TexImage2D(
                gl::TEXTURE_2D, 0, fmt as i32, w as i32, h as i32,
                0, fmt, gl::UNSIGNED_BYTE, cast::transmute(data.unsafe_ref(0))
            );
        }
        gl::BindTexture(gl::TEXTURE_2D, 0);
        return Ok(());
    }

    fn allocate_texture(&mut self, dest: Texture,
                        w:u32, h:u32, format: PixelFormat) -> RendererResult {
        gl::BindTexture(gl::TEXTURE_2D, dest.handle);
        let fmt = gl_format(format);
        unsafe {
            gl::TexImage2D(
                gl::TEXTURE_2D, 0, fmt as i32, w as i32, h as i32,
                0, fmt, gl::UNSIGNED_BYTE, cast::transmute(0)
            );
        }
        gl::BindTexture(gl::TEXTURE_2D, 0);
        return Ok(());
    }

    fn read_back_texture(&mut self, tex: Texture,
                         x: u32, y:u32, w: u32, h: u32,
                         format: PixelFormat,
                         dest: &[u8]) -> RendererResult {
        gl::BindTexture(gl::TEXTURE_2D, tex.handle);
        unsafe {
            gl::ReadPixels(x as i32, y as i32, w as i32, h as i32,
                           gl_format(format), gl::UNSIGNED_BYTE,
                           cast::transmute(dest.unsafe_ref(0)));
            check_err!("glReadPixels({}, {}, {}, {}, {}, GL_UNSIGNED_BYTE, <array of lenght {}>)",
                       x, y, w, h, gl_format(format), dest.len());
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

    fn compile_shader(&mut self, shader: Shader, src: &str) -> RendererResult {
        unsafe {
            src.with_c_str(|mut c_src| {
                let len = src.len() as i32;
                gl::ShaderSource(shader.handle, 1, &c_src, &len);
            });
            gl::CompileShader(shader.handle);

            let mut buffer = ~[0 as u8, ..512];
            let mut length: i32 = 0;
            gl::GetShaderInfoLog(shader.handle, 512, &mut length,
                                 cast::transmute(buffer.unsafe_mut_ref(0)));

            let mut status : i32 = 0;
            gl::GetShaderiv(shader.handle, gl::COMPILE_STATUS, &mut status);
            if status != gl::TRUE as i32 {
                return Err( Error {
                    code: 0,
                    detail: Some(str::raw::from_utf8_owned(buffer)),
                });
            }
            return Ok(());
        }
    }

    fn link_shader_program(&mut self, p: ShaderProgram,
                           shaders: &[Shader]) -> RendererResult {
        unsafe {
            for s in shaders.iter() {
                gl::AttachShader(p.handle, s.handle);
            }

            gl::LinkProgram(p.handle);
            let mut buffer = ~[0 as u8, ..512];
            let mut length = 0;
            gl::GetProgramInfoLog(p.handle, 512, &mut length,
                                  cast::transmute(buffer.unsafe_mut_ref(0)));

            gl::ValidateProgram(p.handle);
            let mut status = 0;
            gl::GetProgramiv(p.handle, gl::VALIDATE_STATUS, cast::transmute(&status));
            if (status != gl::TRUE) {
                return Err( Error {
                    code: 0,
                    detail: Some(str::raw::from_utf8_owned(buffer)),
                });
            }
            return Ok(());
        }
    }

    fn create_vertex_buffer(&mut self) -> VertexBuffer {
        let mut b: u32 = 0;
        unsafe {
            gl::GenBuffers(1, &mut b);
        }
        return VertexBuffer { handle: b };
    }

    fn destroy_vertex_buffer(&mut self, buffer: VertexBuffer) {
        unsafe {
            gl::DeleteBuffers(1, &buffer.handle);
        }
    }

    fn upload_vertex_data(&mut self, buffer: VertexBuffer,
                          data: &[f32], update: UpdateHint) -> RendererResult {

        unsafe {
            gl::BindBuffer(gl::ARRAY_BUFFER, buffer.handle);
            check_err!("glBindBuffer({}, {})", gl::ARRAY_BUFFER, buffer.handle);
            gl::BufferData(gl::ARRAY_BUFFER, (data.len() * size_of::<f32>()) as i64,
                           cast::transmute(data.unsafe_ref(0)),
                           gl_update_hint(update));
            check_err!("glBufferData(GL_ARRAY_BUFFER, {}, {}, {})",
                        (data.len() * size_of::<f32>()) as i64,
                        data.unsafe_ref(0),
                        gl_update_hint(update));
        }
        return Ok(());
    }

    fn allocate_vertex_buffer(&mut self, buffer: VertexBuffer,
                              size: u32, update: UpdateHint) -> RendererResult {
        unsafe {
            gl::BindBuffer(gl::ARRAY_BUFFER, buffer.handle);
            check_err!("glBindBuffer(GL_ARRAY_BUFFER, {})", buffer.handle);

            gl::BufferData(gl::ARRAY_BUFFER, size as i64,
                           cast::transmute(0),
                           gl_update_hint(update));
            check_err!("glBufferData(GL_ARRAY_BUFFER, {}, 0, {})",
                       size, gl_update_hint(update));
        }
        return Ok(());
    }

    fn create_geometry(&mut self) -> Geometry {
        let mut b: u32 = 0;
        unsafe {
            gl::GenVertexArrays(1, &mut b);
        }
        return Geometry { handle: b };
    }

    fn destroy_geometry(&mut self, obj: Geometry) {
        unsafe {
            gl::DeleteVertexArrays(1, &obj.handle);
        }
    }

    fn define_geometry(&mut self, geom: Geometry,
                       attributes: &[VertexAttribute],
                       elements: Option<VertexBuffer>) -> RendererResult {
        gl::BindVertexArray(geom.handle);

        let mut i :u32 = 0;
        for attr in attributes.iter() {
            gl::BindBuffer(gl::ARRAY_BUFFER, attr.buffer.handle);
            unsafe {
            gl::VertexAttribPointer(attr.location as u32,
                                    attr.components as i32,
                                    gl_attribue_type(attr.attrib_type),
                                    gl_bool(attr.normalize),
                                    attr.stride as i32,
                                    cast::transmute(attr.offset as uint));
            gl::EnableVertexAttribArray(i);
            }
            i += 1;
        }

        match elements {
            Some(elts) => {
                gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, elts.handle);
            }
            None => {}
        }

        gl::BindVertexArray(0);

        return Ok(());
    }

    fn get_shader_input_location(&mut self, program: ShaderProgram,
                                 name: &str) -> ShaderInputLocation {
        let mut location: i32 = 0;
        name.with_c_str(|c_name| unsafe {
            location = gl::GetUniformLocation(program.handle, c_name);
        });
        return location;
    }

    fn get_vertex_attribute_location(&mut self, program: ShaderProgram,
                                     name: &str) -> ShaderInputLocation {
        let mut location: i32 = 0;
        name.with_c_str(|c_name| unsafe {
            location = gl::GetAttribLocation(program.handle, c_name);
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
            gl::FramebufferTexture2D(gl::FRAMEBUFFER,
                                     gl_attachement(i as u32),
                                     gl::TEXTURE_2D,
                                     color_attachments[i].handle,
                                     0);
            check_err!("glFramebufferTexture2D(GL_FRAMEBUFFER, {}, GL_TEXTURE_2D, {}, 0)",
                       gl_attachement(i as u32), color_attachments[i].handle);
        }

        match depth {
            Some(d) => {
                gl::FramebufferTexture2D(gl::FRAMEBUFFER,
                                         gl::DEPTH_ATTACHMENT,
                                         gl::TEXTURE_2D,
                                         d.handle,
                                         0);
                check_err!("glFramebufferTexture2D(GL_FRAMEBUFFER, G:_DEPTH_ATTACHMENT, GL_TEXTURE_2D, {}, 0)",
                           d.handle);
            }
            _ => {}
        }

        match stencil {
            Some(s) => {
                gl::FramebufferTexture2D(gl::FRAMEBUFFER,
                                         gl::STENCIL_ATTACHMENT,
                                         gl::TEXTURE_2D,
                                         s.handle,
                                         0);
                check_err!("glFramebufferTexture2D(GL_FRAMEBUFFER, G:_DEPTH_ATTACHMENT, GL_TEXTURE_2D, {}, 0)",
                           s.handle);
            }
            _ => {}
        }

        let status = gl::CheckFramebufferStatus(gl::FRAMEBUFFER);
        gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
        if (status != gl::FRAMEBUFFER_COMPLETE) {
            return Err(Error{code: status, detail: None });
        }
        return Ok(RenderTarget{ handle: fbo });
    }

    fn destroy_render_target(&mut self, fbo: RenderTarget) {
        if self.current_render_target == fbo {
            let rt = self.get_default_render_target();
            self.use_render_target(rt);
        }
        unsafe {
            gl::DeleteFramebuffers(1, &fbo.handle);
        }
    }

    fn get_default_render_target(&mut self) -> RenderTarget {
        return RenderTarget { handle: 0 };
    }

    fn render(&mut self, commands: &[RenderingCommand]) -> RendererResult {
        for command in commands.iter() {
            match self.render_command(command) {
                Ok(()) => {}
                Err(e) => { return Err(e); }
            }
        }
        return Ok(());
    }
}

fn gl_format(format: PixelFormat) -> u32 {
    match format {
        FORMAT_R8G8B8A8 => gl::RGBA,
        FORMAT_R8G8B8X8 => gl::RGB,
        FORMAT_B8G8R8A8 => gl::BGRA,
        FORMAT_B8G8R8X8 => gl::BGR,
        FORMAT_A8 => gl::RED,
    }
}

fn gl_shader_type(target: ShaderType) -> u32 {
    match target {
        VERTEX_SHADER => gl::VERTEX_SHADER,
        FRAGMENT_SHADER => gl::FRAGMENT_SHADER,
        GEOMETRY_SHADER => gl::GEOMETRY_SHADER,
    }
}

fn gl_draw_mode(flags: RenderFlags) -> u32 {
    if flags & LINES != 0 {
        return if flags & STRIP != 0 { gl::LINE_STRIP }
               else if flags & LOOP != 0 { gl::LINE_LOOP }
               else { gl::LINES }
    }
    return if flags & STRIP != 0 { gl::TRIANGLE_STRIP }
           else { gl::TRIANGLES }
}

fn gl_update_hint(hint: UpdateHint) -> u32 {
    match hint {
        STATIC_UPDATE => gl::STATIC_DRAW,
        STREAM_UPDATE => gl::STREAM_DRAW,
        DYNAMIC_UPDATE => gl::DYNAMIC_DRAW,
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

fn gl_bool(b: bool) -> u8 {
    return if b { gl::TRUE } else { gl::FALSE };
}

pub fn gl_error_str(err: u32) -> &'static str {
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
