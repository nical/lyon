#![doc(html_logo_url = "https://nical.github.io/lyon-doc/lyon-logo.svg")]

//! #Lyon SVG
//!
//! Utilities to facilitate interfacing with SVG.
//! At the moment this is mostly a wrapper around the [svgparser](https://crates.io/crates/svgparser)
//! crate.

#![allow(dead_code)]

extern crate lyon_path as path;

pub extern crate svgparser;

pub mod parser;
pub mod serializer;
