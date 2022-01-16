// This module contains a few helpers that should not be considered as part of the public API,
// but are exposed for use by other lyon crates.
// Changing them doesn't necessarily imply semver breaking bumps.

pub use crate::geom::{CubicBezierSegment, QuadraticBezierSegment};
pub use crate::math::Point;
pub use crate::traits::PathBuilder;
pub use crate::{Attributes, EndpointId};

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct DebugValidator {
    #[cfg(debug_assertions)]
    in_subpath: bool,
}

impl Default for DebugValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl DebugValidator {
    #[inline(always)]
    pub fn new() -> Self {
        DebugValidator {
            #[cfg(debug_assertions)]
            in_subpath: false,
        }
    }

    #[inline(always)]
    pub fn begin(&mut self) {
        #[cfg(debug_assertions)]
        {
            assert!(!self.in_subpath);
            self.in_subpath = true;
        }
    }

    #[inline(always)]
    pub fn end(&mut self) {
        #[cfg(debug_assertions)]
        {
            assert!(self.in_subpath);
            self.in_subpath = false;
        }
    }

    #[inline(always)]
    pub fn edge(&self) {
        #[cfg(debug_assertions)]
        {
            assert!(self.in_subpath);
        }
    }

    #[inline(always)]
    pub fn build(&self) {
        #[cfg(debug_assertions)]
        {
            assert!(!self.in_subpath);
        }
    }
}

pub fn flatten_quadratic_bezier(
    tolerance: f32,
    from: Point,
    ctrl: Point,
    to: Point,
    attributes: Attributes,
    prev_attributes: Attributes,
    builder: &mut impl PathBuilder,
    buffer: &mut [f32],
) -> EndpointId {
    let curve = QuadraticBezierSegment { from, ctrl, to };
    let n = attributes.len();
    let mut id = EndpointId::INVALID;
    curve.for_each_flattened_with_t(tolerance, &mut |point, t| {
        let attr = if t == 1.0 {
            attributes
        } else {
            for i in 0..n {
                buffer[i] = prev_attributes[i] * (1.0 - t) + attributes[i] * t;
            }
            Attributes(&buffer[..])
        };
        id = builder.line_to(point, attr);
    });

    id
}

pub fn flatten_cubic_bezier(
    tolerance: f32,
    from: Point,
    ctrl1: Point,
    ctrl2: Point,
    to: Point,
    attributes: Attributes,
    prev_attributes: Attributes,
    builder: &mut impl PathBuilder,
    buffer: &mut [f32],
) -> EndpointId {
    let curve = CubicBezierSegment {
        from,
        ctrl1,
        ctrl2,
        to,
    };
    let n = attributes.len();
    let mut id = EndpointId::INVALID;
    curve.for_each_flattened_with_t(tolerance, &mut |point, t| {
        let attr = if t == 1.0 {
            attributes
        } else {
            for i in 0..n {
                buffer[i] = prev_attributes[i] * (1.0 - t) + attributes[i] * t;
            }
            Attributes(&buffer[..])
        };
        id = builder.line_to(point, attr);
    });

    id
}
