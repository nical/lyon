//! Move at a defined speed along a path.
//!
//! # Path walking
//!
//! ## Overview
//!
//! In principle, walking a path is similar to iterating over it,
//! but instead of going from receiving path segments (of varying
//! sizes), the path walker makes it possible to advance by a certain
//! distance along the path.
//!
//! ## Example
//!
//! ```
//! use lyon_algorithms::walk::{RegularPattern, walk_along_path};
//! use lyon_algorithms::path::PathSlice;
//! use lyon_algorithms::path::iterator::*;
//! use lyon_algorithms::path::math::Point;
//!
//! fn dots_along_path(path: PathSlice, dots: &mut Vec<Point>) {
//!     let mut pattern = RegularPattern {
//!         callback: &mut |position, _tangent, _distance, _attributes: &[f32]| {
//!             dots.push(position);
//!             true // Return true to continue walking the path.
//!         },
//!         // Invoke the callback above at a regular interval of 3 units.
//!         interval: 3.0,
//!     };
//!
//!     let tolerance = 0.01; // The path flattening tolerance.
//!     let start_offset = 0.0; // Start walking at the beginning of the path.
//!     walk_along_path(
//!         path.iter().flattened(tolerance),
//!         start_offset,
//!         &mut pattern
//!     );
//! }
//!
//! ```
//!

use crate::geom::{CubicBezierSegment, QuadraticBezierSegment};
use crate::math::*;
use crate::path::builder::*;
use crate::path::{EndpointId, PathEvent};

use std::f32;

/// Walks along the path staring at offset `start` and applies a `Pattern`.
pub fn walk_along_path<Iter>(path: Iter, start: f32, pattern: &mut dyn Pattern)
where
    Iter: IntoIterator<Item = PathEvent>,
{
    let mut walker = PathWalker::new(start, pattern);
    for evt in path {
        walker.path_event(evt, &[]);
        if walker.done {
            return;
        }
    }
}

/// Types implementing the `Pattern` can be used to walk along a path
/// at constant speed.
///
/// At each step, the pattern receives the position, tangent and already
/// traversed distance along the path and returns the distance until the
/// next step.
///
/// See the `RegularPattern` and `RepeatedPattern` implementations.
/// This trait is also implemented for all functions/closures with signature
/// `FnMut(Point, Vector, f32, &[f32]) -> Option<f32>`.
pub trait Pattern {
    /// This method is invoked at each step along the path.
    ///
    /// If this method returns None, path walking stops. Otherwise the returned
    /// value is the distance along the path to the next element in the pattern.
    fn next(&mut self, position: Point, tangent: Vector, distance: f32, attributes: &[f32]) -> Option<f32>;

    /// Invoked at the start each sub-path.
    ///
    /// Takes the leftover requested distance from the previous sub-path path,
    /// if any.
    ///
    /// If this method returns None, path walking stops. Otherwise the returned
    /// value is the distance along the path to the next element in the pattern.
    fn begin(&mut self, distance: f32) -> Option<f32> {
        Some(distance)
    }
}

/// A helper struct to walk along a flattened path using a builder API.
pub struct PathWalker<'l> {
    prev: Point,
    advancement: f32,
    leftover: f32,
    next_distance: f32,
    first: Point,
    need_moveto: bool,
    done: bool,
    prev_attributes: Vec<f32>,
    attribute_buffer: Vec<f32>,
    first_attributes: Vec<f32>,
    num_attributes: usize,

    pattern: &'l mut dyn Pattern,
}

impl<'l> PathWalker<'l> {
    pub fn new(start: f32, pattern: &'l mut dyn Pattern) -> PathWalker<'l> {
        Self::with_attributes(0, start, pattern)
    }
    pub fn with_attributes(num_attributes: usize, start: f32, pattern: &'l mut dyn Pattern) -> PathWalker<'l> {
        let start = f32::max(start, 0.0);
        PathWalker {
            prev: point(0.0, 0.0),
            first: point(0.0, 0.0),
            advancement: 0.0,
            leftover: 0.0,
            next_distance: start,
            need_moveto: true,
            done: false,
            pattern,
            prev_attributes: vec![0.0; num_attributes],
            attribute_buffer: vec![0.0; num_attributes],
            first_attributes: vec![0.0; num_attributes],
            num_attributes,
        }
    }

    // TODO: interpolate the custom attributes and pass them to the pattern.
    fn edge(&mut self, to: Point, t: f32, attributes: &[f32]) {
        debug_assert!(!self.need_moveto);

        let v = to - self.prev;
        let d = v.length();

        if d < 1e-5 {
            return;
        }

        let tangent = v / d;

        let mut distance = self.leftover + d;
        while distance >= self.next_distance {
            if self.num_attributes > 0 {
                let t2 = t * self.next_distance / distance;
                for i in 0..self.num_attributes {
                    self.attribute_buffer[i] = self.prev_attributes[i] * (1.0 - t2) + attributes[i] * t2;
                }
            }
            let position = self.prev + tangent * (self.next_distance - self.leftover);
            self.prev = position;
            self.leftover = 0.0;
            self.advancement += self.next_distance;
            distance -= self.next_distance;

            if let Some(distance) = self.pattern.next(position, tangent, self.advancement, &self.attribute_buffer[..]) {
                self.next_distance = distance;
            } else {
                self.done = true;
                return;
            }
        }

        self.prev = to;
        self.leftover = distance;
    }
}

impl<'l> PathBuilder for PathWalker<'l> {
    fn num_attributes(&self) -> usize { self.num_attributes }

    fn begin(&mut self, to: Point, attributes: &[f32]) -> EndpointId {
        self.need_moveto = false;
        self.first = to;
        self.prev = to;

        if let Some(distance) = self.pattern.begin(self.next_distance) {
            self.next_distance = distance;
        } else {
            self.done = true;
        }

        self.prev_attributes.copy_from_slice(attributes);
        self.first_attributes.copy_from_slice(attributes);

        EndpointId::INVALID
    }

    fn line_to(&mut self, to: Point, attributes: &[f32]) -> EndpointId {
        debug_assert!(!self.need_moveto);

        self.edge(to, 1.0, attributes);

        self.prev_attributes.copy_from_slice(attributes);

        EndpointId::INVALID
    }

    fn end(&mut self, close: bool) {
        if close {
            let first = self.first;
            let attributes = std::mem::take(&mut self.first_attributes);
            self.edge(first, 1.0, &attributes);
            self.first_attributes = attributes;
            self.need_moveto = true;
        }
    }

    fn quadratic_bezier_to(&mut self, ctrl: Point, to: Point, attributes: &[f32]) -> EndpointId {
        let curve = QuadraticBezierSegment {
            from: self.prev,
            ctrl,
            to,
        };
        curve.for_each_flattened_with_t(0.01, &mut |p, t| {
            self.edge(p, t, attributes);
        });

        self.prev_attributes.copy_from_slice(attributes);

        EndpointId::INVALID
    }

    fn cubic_bezier_to(&mut self, ctrl1: Point, ctrl2: Point, to: Point, attributes: &[f32]) -> EndpointId {
        let curve = CubicBezierSegment {
            from: self.prev,
            ctrl1,
            ctrl2,
            to,
        };

        curve.for_each_flattened_with_t(0.01, &mut |p, t| {
            self.edge(p, t, attributes);
        });

        self.prev_attributes.copy_from_slice(attributes);

        EndpointId::INVALID
    }
}

/// A simple pattern that invokes a callback at regular intervals.
///
/// If the callback returns false, path walking stops.
pub struct RegularPattern<Cb> {
    /// The function to call at each step.
    pub callback: Cb,
    /// A constant interval between each step.
    pub interval: f32,
}

// TODO: lambdas get much less ergonomic when unused arguments have a lifetime.
// For example `RegularPattern { callback: &mut |position, _, _, _| { }, .. }` does not
// compile today because "one type is more general than the other".
// The error message is a bit confusing and it doesn't help the user figure out that they
// have to add an explicit type to the last unused argument argument.
//
// Maybe a more ergonomic solution would be to pass a single argument with multiple members.

impl<Cb> Pattern for RegularPattern<Cb>
where
    Cb: FnMut(Point, Vector, f32, &[f32]) -> bool,
{
    #[inline]
    fn next(&mut self, position: Point, tangent: Vector, distance: f32, attributes: &[f32]) -> Option<f32> {
        if !(self.callback)(position, tangent, distance, attributes) {
            return None;
        }
        Some(self.interval)
    }
}

/// A pattern that invokes a callback at a repeated sequence of
/// constant intervals.
///
/// If the callback returns false, path walking stops.
pub struct RepeatedPattern<'l, Cb> {
    /// The function to call at each step.
    pub callback: Cb,
    /// The repeated interval sequence.
    pub intervals: &'l [f32],
    /// The index of the next interval in the sequence.
    pub index: usize,
}

impl<'l, Cb> Pattern for RepeatedPattern<'l, Cb>
where
    Cb: FnMut(Point, Vector, f32, &[f32]) -> bool,
{
    #[inline]
    fn next(&mut self, position: Point, tangent: Vector, distance: f32, attributes: &[f32]) -> Option<f32> {
        if !(self.callback)(position, tangent, distance, attributes) {
            return None;
        }
        let idx = self.index % self.intervals.len();
        self.index += 1;
        Some(self.intervals[idx])
    }
}

impl<Cb> Pattern for Cb
where
    Cb: FnMut(Point, Vector, f32, &[f32]) -> Option<f32>,
{
    #[inline]
    fn next(&mut self, position: Point, tangent: Vector, distance: f32, attributes: &[f32]) -> Option<f32> {
        (self)(position, tangent, distance, attributes)
    }
}

#[test]
fn walk_square() {
    let expected = [
        (point(0.0, 0.0), vector(1.0, 0.0), 0.0),
        (point(2.0, 0.0), vector(1.0, 0.0), 2.0),
        (point(4.0, 0.0), vector(1.0, 0.0), 4.0),
        (point(6.0, 0.0), vector(1.0, 0.0), 6.0),
        (point(6.0, 2.0), vector(0.0, 1.0), 8.0),
        (point(6.0, 4.0), vector(0.0, 1.0), 10.0),
        (point(6.0, 6.0), vector(0.0, 1.0), 12.0),
        (point(4.0, 6.0), vector(-1.0, 0.0), 14.0),
        (point(2.0, 6.0), vector(-1.0, 0.0), 16.0),
        (point(0.0, 6.0), vector(-1.0, 0.0), 18.0),
        (point(0.0, 4.0), vector(0.0, -1.0), 20.0),
        (point(0.0, 2.0), vector(0.0, -1.0), 22.0),
        (point(0.0, 0.0), vector(0.0, -1.0), 24.0),
    ];

    let mut i = 0;
    let mut pattern = RegularPattern {
        interval: 2.0,
        callback: |pos, n, d, _a: &[f32]| {
            println!("p:{:?} n:{:?} d:{:?}", pos, n, d);
            assert_eq!(pos, expected[i].0);
            assert_eq!(n, expected[i].1);
            assert_eq!(d, expected[i].2);
            i += 1;
            true
        },
    };

    let mut walker = PathWalker::new(0.0, &mut pattern);

    walker.begin(point(0.0, 0.0), &[]);
    walker.line_to(point(6.0, 0.0), &[]);
    walker.line_to(point(6.0, 6.0), &[]);
    walker.line_to(point(0.0, 6.0), &[]);
    walker.close();
}

#[test]
fn walk_with_leftover() {
    let expected = [
        (point(1.0, 0.0), vector(1.0, 0.0), 1.0),
        (point(4.0, 0.0), vector(1.0, 0.0), 4.0),
        (point(5.0, 2.0), vector(0.0, 1.0), 7.0),
        (point(5.0, 5.0), vector(0.0, 1.0), 10.0),
        (point(2.0, 5.0), vector(-1.0, 0.0), 13.0),
        (point(0.0, 4.0), vector(0.0, -1.0), 16.0),
        (point(0.0, 1.0), vector(0.0, -1.0), 19.0),
    ];

    let mut i = 0;
    let mut pattern = RegularPattern {
        interval: 3.0,
        callback: |pos, n, d, _a: &[f32]| {
            println!("p:{:?} n:{:?} d:{:?}", pos, n, d);
            assert_eq!(pos, expected[i].0);
            assert_eq!(n, expected[i].1);
            assert_eq!(d, expected[i].2);
            i += 1;
            true
        },
    };

    let mut walker = PathWalker::new(1.0, &mut pattern);

    walker.begin(point(0.0, 0.0), &[]);
    walker.line_to(point(5.0, 0.0), &[]);
    walker.line_to(point(5.0, 5.0), &[]);
    walker.line_to(point(0.0, 5.0), &[]);
    walker.close();
}

#[test]
fn walk_starting_after() {
    // With a starting distance that is greater than the path, the
    // callback should never be called.
    let cb = &mut |_, _, _, _a: &[f32]| -> Option<f32> { panic!() };
    let mut walker = PathWalker::new(10.0, cb);

    walker.begin(point(0.0, 0.0), &[]);
    walker.line_to(point(5.0, 0.0), &[]);
    walker.end(false);
}

#[test]
fn walk_abort_early() {
    let mut callback_counter = 0;
    let mut pattern = RegularPattern {
        interval: 3.0,
        callback: |_pos, _n, _d, _a: &[f32]| {
            callback_counter += 1;
            false
        },
    };

    let mut walker = PathWalker::new(1.0, &mut pattern);

    walker.begin(point(0.0, 0.0), &[]);
    walker.line_to(point(100.0, 0.0), &[]);

    assert_eq!(callback_counter, 1);
}
