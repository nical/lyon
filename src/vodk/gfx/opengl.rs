use gl;
use gpu = gfx::renderer;
use std::str;
use std::cast;
use std::libc::c_void;

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

fn gl_draw_mode(mode: gpu::DrawMode) -> u32 {
    match mode {
        gpu::TRIANGLES => gl::TRIANGLES,
        gpu::TRIANGLE_STRIP => gl::TRIANGLE_STRIP,
        gpu::LINES => gl::LINES,
        gpu::LINE_STRIP => gl::LINE_STRIP,
        gpu::LINE_LOOP => gl::LINE_LOOP,
    }
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

struct RenderingContextGL {
    current_texture: gpu::Texture,
    current_vbo: gpu::VertexBuffer,
    current_program: gpu::ShaderProgram,
}

impl gpu::RenderingContext for RenderingContextGL {
    fn make_current(&mut self) -> bool {
        return true; // TODO
    }

    fn is_supported(&mut self, f: gpu::Feature) -> bool {
        match f {
            gpu::FRAGMENT_SHADING => true,
            gpu::VERTEX_SHADING => true,
            gpu::GEOMETRY_SHADING => false,
            gpu::RENDER_TO_TEXTURE => false,
            gpu::MULTIPLE_RENDER_TARGETS => false,
        }
    }

    fn set_viewport(&mut self, x:i32, y:i32, w:i32, h:i32) {
        gl::Viewport(x,y,w,h);
    }

    fn set_clear_color(&mut self, r: f32, g: f32, b: f32, a: f32) {
        gl::ClearColor(r,g,b,a);
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
                           w:u32, h:u32, stride: u32) {
        gl::BindTexture(gl::TEXTURE_2D, dest.handle);
        let fmt = gl_format(format);
        unsafe {
            gl::TexImage2D(
                gl::TEXTURE_2D, 0, fmt as i32, w as i32, h as i32,
                0, fmt, gl::UNSIGNED_BYTE, cast::transmute(data.unsafe_ref(0))
            );
        }
        gl::BindTexture(gl::TEXTURE_2D, 0);
    }

    fn allocate_texture(&mut self, dest: gpu::Texture,
                    format: gpu::PixelFormat,
                    w:u32, h:u32, stride: u32) {
        gl::BindTexture(gl::TEXTURE_2D, dest.handle);
        let fmt = gl_format(format);
        unsafe {
            gl::TexImage2D(
                gl::TEXTURE_2D, 0, fmt as i32, w as i32, h as i32,
                0, fmt, gl::UNSIGNED_BYTE, cast::transmute(0)
            );
        }
        gl::BindTexture(gl::TEXTURE_2D, 0);
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

    fn bind_shader_program(&mut self, p: gpu::ShaderProgram) {
        gl::UseProgram(p.handle);
    }

    fn unbind_shader_program(&mut self, p: gpu::ShaderProgram) {
        gl::UseProgram(0);
    }

    fn compile_shader(&mut self, shader: gpu::Shader, src: &str) -> gpu::ShaderResult {
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
                println("error while compiling shader\n");
                return gpu::COMPILE_ERROR(str::raw::from_utf8_owned(buffer));
            }
            return gpu::COMPILE_SUCCESS;
        }
    }

    fn link_shader_program(&mut self, p: gpu::ShaderProgram) -> gpu::ProgramResult {
        unsafe {
            gl::LinkProgram(p.handle);
            let mut buffer = ~[0 as u8, ..512];
            let mut length = 0;
            gl::GetProgramInfoLog(p.handle, 512, &mut length,
                                  cast::transmute(buffer.unsafe_mut_ref(0)));

            gl::ValidateProgram(p.handle);
            let mut status = 0;
            gl::GetProgramiv(p.handle, gl::VALIDATE_STATUS, cast::transmute(&status));
            if (status == gl::FALSE) {
                return gpu::LINK_ERROR(str::raw::from_utf8_owned(buffer));
            }
            return gpu::LINK_SUCCESS;
        }
    }

    fn attach_shader(&mut self, p: gpu::ShaderProgram, s: gpu::Shader) {
        gl::AttachShader(p.handle, s.handle);
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

    fn bind_vertex_buffer(&mut self, vbo: gpu::VertexBuffer) {
        gl::BindBuffer(gl::ARRAY_BUFFER, vbo.handle);
    }

    fn unbind_vertex_buffer(&mut self, buffer: gpu::VertexBuffer) {
        gl::BindBuffer(gl::ARRAY_BUFFER, 0);
    }

    fn upload_vertex_data(&mut self, buffer: gpu::VertexBuffer,
                          data: &[u8], update: gpu::UpdateHint) {
        unsafe {
            gl::BufferData(gl::ARRAY_BUFFER, data.len() as i64,
                           cast::transmute(data.unsafe_ref(0)),
                           gl_update_hint(update));
        }
    }

    fn get_uniform_location(&mut self, shader: gpu::Shader, name: &str) -> i32 {
        let mut location: i32 = 0;
        name.with_c_str(|c_name| unsafe {
            location = gl::GetUniformLocation(shader.handle, c_name);
        });
        return location;
    }

    fn define_vertex_attribute(attrib_index: u32,
                               attrib_type: gpu::AttributeType,
                               components_per_vertex: i32,
                               stride: i32,
                               offset: i32) {
        unsafe {
            gl::VertexAttribPointer(attrib_index,
                                    components_per_vertex,
                                    gl_attribue_type(attrib_type),
                                    gl::FALSE, // TODO: normalize
                                    stride,
                                    offset as *c_void);
        }
    }

    fn draw_arrays(&mut self, mode: gpu::DrawMode, first: i32, count: i32) {
        gl::DrawArrays(gl_draw_mode(mode), first, count);
    }

    fn draw(&mut self,
            _mode: gpu::DrawMode,
            _p: gpu::ShaderProgram,
            _inputs: &Iterator<gpu::ShaderInput>) {
        fail!("Not implemented");
    }
}
