use crate::geom::math::Point;

use std::sync::mpsc::{channel, Sender, Receiver};

pub const RED: Color = Color { r: 255, g: 0, b: 0, a: 255 };
pub const GREEN: Color = Color { r: 0, g: 255, b: 0, a: 255 };
pub const BLUE: Color = Color { r: 0, g: 0, b: 255, a: 255 };
pub const DARK_RED: Color = Color { r: 128, g: 0, b: 0, a: 255 };
pub const DARK_GREEN: Color = Color { r: 0, g: 128, b: 0, a: 255 };
pub const DARK_BLUE: Color = Color { r: 0, g: 0, b: 128, a: 255 };
pub const BLACK: Color = Color { r: 0, g: 0, b: 0, a: 255 };
pub const WHITE: Color = Color { r: 0, g: 0, b: 0, a: 255 };

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DebuggerMsg {
    Point { position: Point, color: Color, flags: u32 },
    Edge { from: Point, to: Point, color: Color, flags: u32 },
    NewFrame { flags: u32 },
    String { string: String, flags: u32 },
    Error { flags: u32 },
}

impl DebuggerMsg {
    pub fn flags(&self) -> u32 {
        match *self {
            DebuggerMsg::Point { flags, .. } => flags,
            DebuggerMsg::Edge { flags, .. } => flags,
            DebuggerMsg::NewFrame { flags, .. } => flags,
            DebuggerMsg::String { flags, .. } => flags,
            DebuggerMsg::Error { flags, .. } => flags,
        }
    }
}

pub trait Debugger2D {
    fn point(&self, position: &Point, color: Color, flags: u32);
    fn edge(&self, from: &Point, to: &Point, color: Color, flags: u32);
    fn new_frame(&self, flags: u32);
    fn string(&self, string: String, flags: u32);
    fn error(&self, flags: u32);
}

pub struct EmptyDebugger2D;

impl Debugger2D for EmptyDebugger2D {
    fn point(&self, _position: &Point, _color: Color, _flags: u32) {}
    fn edge(&self, _from: &Point, _to: &Point, _color: Color, _flags: u32) {}
    fn new_frame(&self, _flags: u32) {}
    fn string(&self, _s: String, _flags: u32) {}
    fn error(&self, _flags: u32) {}
}


pub struct Trace {
    pub messages: Vec<DebuggerMsg>,
}

impl Trace {
    pub fn new() -> Self {
        Trace {
            messages: Vec::new(),
        }
    }
}

pub struct SenderDebugger2D {
    tx: Sender<DebuggerMsg>,
}

impl Debugger2D for SenderDebugger2D {
    fn point(&self, position: &Point, color: Color, flags: u32) {
        let _ = self.tx.send(DebuggerMsg::Point { position: *position, color, flags });
    }
    fn edge(&self, from: &Point, to: &Point, color: Color, flags: u32) {
        let _ = self.tx.send(DebuggerMsg::Edge { from: *from, to: *to, color, flags });
    }
    fn string(&self, string: String, flags: u32) {
        let _ = self.tx.send(DebuggerMsg::String { string, flags });
    }
    fn new_frame(&self, flags: u32) {
        let _ = self.tx.send(DebuggerMsg::NewFrame { flags });
    }
    fn error(&self, flags: u32) {
        let _ = self.tx.send(DebuggerMsg::Error { flags });
    }
}

pub struct ReceiverDebugger2D {
    rx: Receiver<DebuggerMsg>,
}

pub fn debugger_channel() -> (SenderDebugger2D, ReceiverDebugger2D) {
    let (tx, rx) = channel();
    (
        SenderDebugger2D { tx },
        ReceiverDebugger2D { rx },
    )
}

impl ReceiverDebugger2D {
    pub fn collect(&self) -> Trace {
        let mut trace = Trace {
            messages: Vec::new(),
        };

        self.write_trace(&mut trace);

        trace
    }

    pub fn collect_with_filter(&self, flags: u32) -> Trace {
        let mut trace = Trace {
            messages: Vec::new(),
        };

        self.write_trace_with_filter(&mut trace, flags);

        trace
    }

    pub fn write_trace(&self, trace: &mut Trace) {
        self.write_trace_with_filter(trace, 0xffff);
    }

    pub fn write_trace_with_filter(&self, trace: &mut Trace, flags: u32) {
        while let Ok(msg) = self.rx.try_recv() {
            if msg.flags() & flags != 0 {
                trace.messages.push(msg);
            }
        }
    }
}

pub struct Filter<T> {
    flags: u32,
    dbg: T,
}

impl<T> Filter<T> {
    pub fn new(flags: u32, dbg: T) -> Self {
        Filter { flags, dbg }
    }

    fn filter_out(&self, flags: u32) -> bool {
        self.flags & flags == 0
    }
}

impl<T: Debugger2D> Debugger2D for Filter<T> {
    fn point(&self, position: &Point, color: Color, flags: u32) {
        if self.filter_out(flags) {
            return;
        }
        self.dbg.point(position, color, flags);
    }
    fn edge(&self, from: &Point, to: &Point, color: Color, flags: u32) {
        if self.filter_out(flags) {
            return;
        }
        self.dbg.edge(from, to, color, flags);
    }
    fn string(&self, string: String, flags: u32) {
        if self.filter_out(flags) {
            return;
        }
        self.dbg.string(string, flags);
    }
    fn new_frame(&self, flags: u32) {
        if self.filter_out(flags) {
            return;
        }
        self.dbg.new_frame(flags);
    }
    fn error(&self, flags: u32) {
        if self.filter_out(flags) {
            return;
        }
        self.dbg.error(flags);
    }
}
