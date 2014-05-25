use gl;
use glfw;
use glfw::Context;
use gfx::opengl;
use gfx::renderer;
use std::rc::Rc;

use time;
use std::io::timer::sleep;

pub struct Window {
    glfw_win: Rc<glfw::Window>,
    glfw: glfw::Glfw,
    events: Receiver<(f64, glfw::WindowEvent)>,
}

impl Window {
    pub fn create(w: u32, h: u32, title: &str) -> Window {
        let glfw = glfw::init(glfw::FAIL_ON_ERRORS).unwrap();

        glfw.window_hint(glfw::ContextVersion(3, 1));
        glfw.window_hint(glfw::OpenglForwardCompat(true));

        let (glfw_win, events) = glfw.create_window(w, h, "OpenGL", glfw::Windowed)
            .expect("Failed to create GLFW window.");

        glfw_win.set_pos_polling(true);
        glfw_win.set_all_polling(true);
        glfw_win.set_size_polling(true);
        glfw_win.set_close_polling(true);
        glfw_win.set_refresh_polling(true);
        glfw_win.set_focus_polling(true);
        glfw_win.set_iconify_polling(true);
        glfw_win.set_framebuffer_size_polling(true);
        glfw_win.set_key_polling(true);
        glfw_win.set_char_polling(true);
        glfw_win.set_mouse_button_polling(true);
        glfw_win.set_cursor_pos_polling(true);
        glfw_win.set_cursor_enter_polling(true);
        glfw_win.set_scroll_polling(true);

        return Window {
            glfw_win: Rc::new(glfw_win),
            glfw: glfw,
            events: events
        };
    }

    pub fn create_rendering_context(&mut self) -> Box<renderer::RenderingContext> {
        // It is essential to make the context current before calling gl::load_with.
        self.glfw_win.make_current();
        // Load the OpenGL function pointers
        gl::load_with(|s| self.glfw.get_proc_address(s));

        let win = self.glfw_win.clone();
        return box opengl::RenderingContextGL::new(win) as Box<renderer::RenderingContext>;
    }

    pub fn swap_buffers(&mut self) {
        self.glfw_win.swap_buffers();
    }

    pub fn should_close(&self) -> bool {
        return self.glfw_win.should_close();
    }

    pub fn poll_events(&self) {
        self.glfw.poll_events();
        for event in glfw::flush_messages(&self.events) {
            //handle_window_event(&window, event);
        }

    }
}


pub trait InputEventListener {
    fn on_event(&self, InputEvent);
}

pub type Key = glfw::Key;

pub enum Action {
    Press,
    Release,
    Repeat,
}

pub enum MouseButton {
    MouseButtonLeft,
    MouseButtonRight,
    MouseButtonMiddle,
}

pub enum InputEvent {
    CursorPosEvent(i32, i32),
    MouseButtonEvent(MouseButton, bool),
    ScrollEvent(i32, i32),
    FocusEvent(bool),
    CloseEvent,
    FrameBufferSizeEvent(i32, i32),
}

pub type EventMask = u32;
pub static CURSOR_POS_EVENT: EventMask = 1 << 0;
pub static MOUSE_BUTTON_EVENT: EventMask = 1 << 0;
pub static SCROLL_EVENT: EventMask = 1 << 0;
pub static FOCUS_EVENT: EventMask = 1 << 0;
pub static CLOSE_EVENT: EventMask = 1 << 0;
pub static FRAME_BUFFER_SIZE_EVENT: EventMask = 1 << 0;
