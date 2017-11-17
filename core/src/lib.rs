#![doc(html_logo_url = "https://nical.github.io/lyon-doc/lyon-logo.svg")]

extern crate euclid;

pub mod math;
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

impl Default for ArcFlags {
    fn default() -> Self {
        ArcFlags {
            large_arc: false,
            sweep: false,
        }
    }
}
