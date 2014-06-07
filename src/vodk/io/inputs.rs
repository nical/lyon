
pub trait EventListener {
    fn on_event(&self, Event);
}

#[deriving(Show, Eq)]
pub enum Action {
    Press,
    Release,
    Repeat,
}

#[deriving(Show, Eq)]
pub enum MouseButton {
    MouseButtonLeft,
    MouseButtonRight,
    MouseButtonMiddle,
}

#[deriving(Show, Eq)]
pub enum Event {
    CursorPosEvent(f32, f32),
    MouseButtonEvent(MouseButton, Action),
    ScrollEvent(f32, f32),
    FocusEvent(bool),
    CloseEvent,
    FramebufferSizeEvent(i32, i32),
    DummyEvent,
}

pub type EventMask = u32;
pub static CURSOR_POS_EVENT: EventMask = 1 << 0;
pub static MOUSE_BUTTON_EVENT: EventMask = 1 << 1;
pub static SCROLL_EVENT: EventMask = 1 << 2;
pub static FOCUS_EVENT: EventMask = 1 << 3;
pub static CLOSE_EVENT: EventMask = 1 << 4;
pub static FRAME_BUFFER_SIZE_EVENT: EventMask = 1 << 5;
