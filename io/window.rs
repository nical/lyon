use gl;
use glfw;
use glfw::Context;
use std::rc::Rc;
use super::inputs;

use time;
use std::io::timer::sleep;

pub struct Window {
    glfw_win: Rc<glfw::Window>,
    glfw: glfw::Glfw,
    events: Receiver<(f64, glfw::WindowEvent)>,
}

impl Window {
    pub fn new(w: u32, h: u32, title: &str) -> Window {
        let glfw = glfw::init(glfw::FAIL_ON_ERRORS).unwrap();

        glfw.window_hint(glfw::ContextVersion(3, 1));
        glfw.window_hint(glfw::OpenglForwardCompat(true));

        let (glfw_win, events) = glfw.create_window(w, h, title, glfw::Windowed)
            .expect("Failed to create GLFW window.");

        glfw_win.set_size_polling(true);
        glfw_win.set_close_polling(true);
        glfw_win.set_refresh_polling(true);
        glfw_win.set_focus_polling(true);
        glfw_win.set_framebuffer_size_polling(true);
        glfw_win.set_mouse_button_polling(true);
        glfw_win.set_cursor_pos_polling(true);
        glfw_win.set_scroll_polling(true);
        //glfw_win.set_key_polling(true);
        //glfw_win.set_char_polling(true);
        //glfw_win.set_cursor_enter_polling(true);

        return Window {
            glfw_win: Rc::new(glfw_win),
            glfw: glfw,
            events: events
        };
    }

    pub fn init_opengl(&mut self) -> bool {
        // make the context current before calling gl::load_with.
        self.glfw_win.make_current();
        gl::load_with(|s| self.glfw.get_proc_address(s));
        return true;
    }

    pub fn swap_buffers(&mut self) {
        self.glfw_win.swap_buffers();
    }

    pub fn should_close(&self) -> bool {
        return self.glfw_win.should_close();
    }

    pub fn poll_events(&self, evts: &mut Vec<inputs::Event>) {
        self.glfw.poll_events();
        for (_, event) in glfw::flush_messages(&self.events) {
            evts.push(from_glfw_event(event));
        }
    }
}

fn from_glfw_mouse_button(b: glfw::MouseButton) -> inputs::MouseButton {
    match b {
        glfw::MouseButtonLeft => inputs::MouseButtonLeft,
        glfw::MouseButtonMiddle => inputs::MouseButtonMiddle,
        glfw::MouseButtonRight => inputs::MouseButtonRight,
        _ => inputs::MouseButtonLeft,
    }
}

fn from_glfw_action(a: glfw::Action) -> inputs::Action {
    match a {
        glfw::Press => inputs::Press,
        glfw::Release => inputs::Release,
        glfw::Repeat => inputs::Repeat,
    }
}

fn from_glfw_event(e: glfw::WindowEvent) -> inputs::Event {
    match e {
        glfw::CursorPosEvent(x, y) => inputs::CursorPosEvent(x as f32, y as f32),
        glfw::MouseButtonEvent(button, action, _) => inputs::MouseButtonEvent(
            from_glfw_mouse_button(button),
            from_glfw_action(action)
        ),
        glfw::FocusEvent(focus) => inputs::FocusEvent(focus),
        glfw::CloseEvent => inputs::CloseEvent,
        glfw::ScrollEvent(dx, dy) => inputs::ScrollEvent(dx as f32, dy as f32),
        glfw::FramebufferSizeEvent(w, h) => inputs::FramebufferSizeEvent(w, h),
        _ => inputs::DummyEvent,
    }
}
