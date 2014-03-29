use gl;
use glfw;
use gpu = gfx::renderer;
use std::str;
use std::cast;
use std::libc::c_void;
use std::rc::Rc;
use std::mem::size_of;

macro_rules! check_err (
    ($($arg:tt)*) => (
        if !self.ignore_errors {
            match gl::GetError() {
                gl::NONE => {}
                e => {
                    return Err(gpu::Error{
                        code: gpu::ErrorCode(e),
                        detail: Some(format!($($arg)*))
                    });
                }
            }
        }
    )
)

fn gl_format(format: gpu::PixelFormat) -> u32 {
    match format {
        gpu::FORMAT_R8G8B8A8 => gl::RGBA,
        gpu::FORMAT_R8G8B8X8 => gl::RGB,
        gpu::FORMAT_B8G8R8A8 => gl::BGRA,
        gpu::FORMAT_B8G8R8X8 => gl::BGR,
        gpu::FORMAT_A8 => gl::RED,
    }
}

fn gl_shader_type(target: gpu::ShaderType) -> u32 {
    match target {
        gpu::VERTEX_SHADER => gl::VERTEX_SHADER,
        gpu::FRAGMENT_SHADER => gl::FRAGMENT_SHADER,
        gpu::GEOMETRY_SHADER => gl::GEOMETRY_SHADER,
    }
}

fn gl_draw_mode(flags: gpu::RenderFlags) -> u32 {
    if flags & gpu::LINES != 0 {
        return if flags & gpu::STRIP != 0 { gl::LINE_STRIP }
               else if flags & gpu::LOOP != 0 { gl::LINE_LOOP }
               else { gl::LINES }
    }
    return if flags & gpu::STRIP != 0 { gl::TRIANGLE_STRIP }
           else { gl::TRIANGLES }
}

fn gl_update_hint(hint: gpu::UpdateHint) -> u32 {
    match hint {
        gpu::STATIC_UPDATE => gl::STATIC_DRAW,
        gpu::STREAM_UPDATE => gl::STREAM_DRAW,
        gpu::DYNAMIC_UPDATE => gl::DYNAMIC_DRAW,
    }
}

fn gl_attribue_type(attribute: gpu::AttributeType) -> u32 {
    match attribute {
        gpu::F32 => gl::FLOAT,
        gpu::F64 => gl::DOUBLE,
        gpu::I32 => gl::INT,
        gpu::U32 => gl::UNSIGNED_INT,
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

fn frame_buffer_error(error: u32) -> ~str {
    match error {
        gl::FRAMEBUFFER_INCOMPLETE_MISSING_ATTACHMENT => ~"Missing attachment.",
        gl::FRAMEBUFFER_INCOMPLETE_ATTACHMENT => ~"Incomplete attachment.",
        gl::FRAMEBUFFER_INCOMPLETE_DRAW_BUFFER => ~"Incomplete draw buffer.",
        gl::FRAMEBUFFER_INCOMPLETE_MULTISAMPLE => ~"Incomplete multisample.",
        gl::FRAMEBUFFER_UNSUPPORTED => ~"Unsupported.",
        _ => ~"Unknown Error",
    }
}

pub struct RenderingContextGL {
    //window: Rc<glfw::Window>,
    current_texture: gpu::Texture,
    current_vbo: gpu::VertexBuffer,
    current_render_target: gpu::RenderTarget,
    current_program: gpu::ShaderProgram,
    ignore_errors: bool,
}

fn gl_error_str(err: u32) -> ~str {
    match gl::GetError() {
        gl::NO_ERROR            => { ~"" }
        gl::INVALID_ENUM        => { ~"Invalid enum" },
        gl::INVALID_VALUE       => { ~"Invalid value" },
        gl::INVALID_OPERATION   => { ~"Invalid operation" },
        gl::OUT_OF_MEMORY       => { ~"Out of memory" },
        _ => { ~"Unknown error" }
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

impl RenderingContextGL {
    pub fn new() -> RenderingContextGL {
        RenderingContextGL {
            current_texture: gpu::Texture { handle: 0 },
            current_vbo: gpu::VertexBuffer { handle: 0 },
            current_program: gpu::ShaderProgram { handle: 0 },
            current_render_target: gpu::RenderTarget { handle: 0 },
            ignore_errors: false,
        }
    }

    fn use_render_target(&mut self, fbo: gpu::RenderTarget) {
        if self.current_render_target == fbo {
            return;
        }
        gl::BindFramebuffer(gl::FRAMEBUFFER, fbo.handle);
        self.current_render_target = fbo;
    }

    fn bind_vertex_buffer(&mut self, vbo: gpu::VertexBuffer) {
        if self.current_vbo == vbo {
            return;
        }
        gl::BindBuffer(gl::ARRAY_BUFFER, vbo.handle);
        self.current_vbo = vbo;
    }

    fn render_command(&mut self, command: &gpu::RenderingCommand) -> gpu::Status {
        match *command {
            gpu::OpFlush => { gl::Flush(); }
            gpu::OpClear => { gl::Clear(gl::COLOR_BUFFER_BIT); }
            gpu::OpDraw(ref draw) => {
                gl::UseProgram(draw.shader_program.handle);
                check_err!("glUseProgram({})",draw.shader_program.handle);
                let mut num_tex = 0;
                for input in draw.shader_inputs.iter() {
                    match input.value {
                        gpu::INPUT_FLOATS(ref f) => {
                            match f.len() {
                                1 => { gl::Uniform1f(input.location, f[0]); }
                                2 => { gl::Uniform2f(input.location, f[0], f[1]); }
                                3 => { gl::Uniform3f(input.location, f[0], f[1], f[2]); }
                                4 => { gl::Uniform4f(input.location, f[0], f[1], f[2], f[3]); }
                                x => { return Err(gpu::Error{
                                    code: gpu::ErrorCode(0),
                                    detail: Some(~"Unsupported unform size > 4") });
                                }
                            }
                        }
                        gpu::INPUT_TEXTURE(tex) => {
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

                gl::BindVertexArray(draw.geometry.handle);

                if draw.flags & gpu::INDEXED != 0 {
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

impl gpu::RenderingContext for RenderingContextGL {
    fn make_current(&mut self) -> bool {
        return true; // TODO
    }

    fn reset_state(&mut self) {
        self.current_render_target = self.get_default_render_target();
        self.current_texture = gpu::Texture { handle: 0 };
        self.current_program = gpu::ShaderProgram { handle: 0 };
        self.current_vbo = gpu::VertexBuffer { handle: 0 };
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

    fn is_supported(&mut self, f: gpu::Feature) -> bool {
        match f {
            gpu::FRAGMENT_SHADING => true,
            gpu::VERTEX_SHADING => true,
            gpu::GEOMETRY_SHADING => false,
            gpu::RENDER_TO_TEXTURE => false,
            gpu::MULTIPLE_RENDER_TARGETS => false,
            gpu::INSTANCED_RENDERING => false,
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

    fn create_texture(&mut self) -> gpu::Texture {
        let mut tex = 0;
        unsafe {
            gl::GenTextures(1, &mut tex);
        }
        return gpu::Texture { handle: tex };
    }

    fn destroy_texture(&mut self, tex: gpu::Texture) {
        unsafe {
            gl::DeleteTextures(1, &tex.handle);
        }
    }

    fn set_texture_flags(&mut self, tex: gpu::Texture, flags: gpu::TextureFlags) {
        gl::BindTexture(gl::TEXTURE_2D, tex.handle);
        if flags&gpu::TEXTURE_REPEAT_S != 0 {
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::REPEAT as i32);
        }
        if flags&gpu::TEXTURE_REPEAT_T != 0 {
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::REPEAT as i32);
        }
        if flags&gpu::TEXTURE_CLAMP_S != 0 {
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
        }
        if flags&gpu::TEXTURE_CLAMP_T != 0 {
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
        }
        if flags&gpu::TEXTURE_MIN_FILTER_LINEAR != 0 {
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
        }
        if flags&gpu::TEXTURE_MAG_FILTER_LINEAR != 0 {
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
        }
        if flags&gpu::TEXTURE_MIN_FILTER_NEAREST != 0 {
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
        }
        if flags&gpu::TEXTURE_MAG_FILTER_NEAREST != 0 {
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
        }
        gl::BindTexture(gl::TEXTURE_2D, 0);
    }

    fn upload_texture_data(&mut self, dest: gpu::Texture,
                           data: &[u8], format: gpu::PixelFormat,
                           w:u32, h:u32, stride: u32) -> gpu::Status {
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

    fn allocate_texture(&mut self, dest: gpu::Texture,
                    format: gpu::PixelFormat,
                    w:u32, h:u32, stride: u32) -> gpu::Status {
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

    fn create_shader(&mut self, t: gpu::ShaderType) -> gpu::Shader {
        return gpu::Shader { handle: gl::CreateShader(gl_shader_type(t)) };
    }

    fn destroy_shader(&mut self, s: gpu::Shader) {
        gl::DeleteShader(s.handle);
    }

    fn create_shader_program(&mut self) -> gpu::ShaderProgram {
        return gpu::ShaderProgram { handle: gl::CreateProgram() };
    }

    fn destroy_shader_program(&mut self, p: gpu::ShaderProgram) {
        gl::DeleteProgram(p.handle);
    }

    fn compile_shader(&mut self, shader: gpu::Shader, src: &str) -> gpu::Status {
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
                return Err( gpu::Error {
                    code: gpu::ErrorCode(0),
                    detail: Some(str::raw::from_utf8_owned(buffer)),
                });
            }
            return Ok(());
        }
    }

    fn link_shader_program(&mut self, p: gpu::ShaderProgram,
                           shaders: &[gpu::Shader]) -> gpu::Status {
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
                return Err( gpu::Error {
                    code: gpu::ErrorCode(0),
                    detail: Some(str::raw::from_utf8_owned(buffer)),
                });
            }
            return Ok(());
        }
    }

    fn create_vertex_buffer(&mut self) -> gpu::VertexBuffer {
        let mut b: u32 = 0;
        unsafe {
            gl::GenBuffers(1, &mut b);
        }
        return gpu::VertexBuffer { handle: b };
    }

    fn destroy_vertex_buffer(&mut self, buffer: gpu::VertexBuffer) {
        unsafe {
            gl::DeleteBuffers(1, &buffer.handle);
        }
    }

    fn upload_vertex_data(&mut self, buffer: gpu::VertexBuffer,
                          data: &[f32], update: gpu::UpdateHint) -> gpu::Status {

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

    fn allocate_vertex_buffer(&mut self, buffer: gpu::VertexBuffer,
                              size: u32, update: gpu::UpdateHint) -> gpu::Status {
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

    fn create_geometry(&mut self) -> gpu::Geometry {
        let mut b: u32 = 0;
        unsafe {
            gl::GenVertexArrays(1, &mut b);
        }
        return gpu::Geometry { handle: b };
    }

    fn destroy_geometry(&mut self, obj: gpu::Geometry) {
        unsafe {
            gl::DeleteVertexArrays(1, &obj.handle);
        }
    }

    fn define_geometry(&mut self, geom: gpu::Geometry,
                       attributes: &[gpu::VertexAttribute],
                       elements: Option<gpu::VertexBuffer>) -> gpu::Status {
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

    fn get_shader_input_location(&mut self, program: gpu::ShaderProgram,
                                 name: &str) -> i32 {
        let mut location: i32 = 0;
        name.with_c_str(|c_name| unsafe {
            location = gl::GetUniformLocation(program.handle, c_name);
        });
        return location;
    }

    fn get_vertex_attribute_location(&mut self, program: gpu::ShaderProgram,
                                     name: &str) -> i32 {
        let mut location: i32 = 0;
        name.with_c_str(|c_name| unsafe {
            location = gl::GetAttribLocation(program.handle, c_name);
        });
        return location;
    }

    fn create_render_target(&mut self,
                            color_attachments: &[gpu::Texture],
                            depth: Option<gpu::Texture>,
                            stencil: Option<gpu::Texture>) -> Result<gpu::RenderTarget, ~str> {
        let mut fbo: u32 = 0;
        unsafe {
            gl::GenFramebuffers(1, &mut fbo);
        }

        gl::BindFramebuffer(gl::FRAMEBUFFER, fbo);
        for i in range(0,color_attachments.len()) {
            gl::FramebufferTexture2D(gl::DRAW_FRAMEBUFFER,
                                     gl_attachement(i as u32),
                                     gl::TEXTURE_2D,
                                     color_attachments[i].handle,
                                     0);
        }
        let status = gl::CheckFramebufferStatus(gl::FRAMEBUFFER);
        gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
        if (status != gl::FRAMEBUFFER_COMPLETE) {
            return Err(frame_buffer_error(status));
        }
        return Ok(gpu::RenderTarget{ handle: fbo });
    }

    fn destroy_render_target(&mut self, fbo: gpu::RenderTarget) {
        if self.current_render_target == fbo {
            let rt = self.get_default_render_target();
            self.use_render_target(rt);
        }
        unsafe {
            gl::DeleteFramebuffers(1, &fbo.handle);
        }
    }

    fn get_default_render_target(&mut self) -> gpu::RenderTarget {
        return gpu::RenderTarget { handle: 0 };
    }

    fn render(&mut self, commands: &[gpu::RenderingCommand]) -> gpu::Status {
        for command in commands.iter() {
            match self.render_command(command) {
                Ok(()) => {}
                Err(e) => { return Err(e); }
            }
        }
        return Ok(());
    }
}

