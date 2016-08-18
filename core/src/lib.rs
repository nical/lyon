extern crate euclid;

pub mod math;
pub mod math_utils;
pub mod path_state;
pub mod events;

pub use path_state::*;
pub use events::*;

/// Flag parameters for arcs as described by the SVG specification.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ArcFlags {
    pub large_arc: bool,
    pub sweep: bool,
}
