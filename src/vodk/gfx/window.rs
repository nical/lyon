use gl;
use glfw;
use glfw::Context;
use gfx::opengl;
use gfx::renderer;

use time;
use std::io::timer::sleep;

pub struct Window {
    glfw_win: glfw::Window,
    glfw: glfw::Glfw,
}

impl Window {
    pub fn create(w: u32, h: u32, title: &str) -> Window {
        let glfw = glfw::init(glfw::FAIL_ON_ERRORS).unwrap();

        glfw.window_hint(glfw::ContextVersion(3, 1));
        glfw.window_hint(glfw::OpenglForwardCompat(true));

        let (glfw_win, _) = glfw.create_window(w, h, "OpenGL", glfw::Windowed)
            .expect("Failed to create GLFW window.");

        // It is essential to make the context current before calling gl::load_with.
        glfw_win.make_current();
        // Load the OpenGL function pointers
        gl::load_with(|s| glfw.get_proc_address(s));

        return Window {
            glfw_win: glfw_win,
            glfw: glfw,
        };
    }

    pub fn create_rendering_context(&mut self) -> ~renderer::RenderingContext {
        return ~opengl::RenderingContextGL::new() as ~renderer::RenderingContext;
    }

    pub fn swap_buffers(&mut self) {
        self.glfw_win.swap_buffers();
    }

    pub fn should_close(&self) -> bool {
        return self.glfw_win.should_close();
    }

    pub fn poll_events(&self) {
        self.glfw.poll_events();
    }
}
