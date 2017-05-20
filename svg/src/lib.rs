//! #Lyon SVG
//!
//! Utilities to facilitate interfacing with SVG.
//! At the moment this is mostly a wrapper around the [svgparser](https://crates.io/crates/svgparser)
//! crate.

// TODO: doc

#![allow(dead_code)]
extern crate lyon_core as core;
extern crate lyon_tessellation as tessellation;

#[cfg(test)]
extern crate lyon_extra as extra;

extern crate svgparser;

pub mod parser;
