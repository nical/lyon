#![doc(html_logo_url = "https://nical.github.io/lyon-doc/lyon-logo.svg")]

//! Utilities to facilitate interfacing lyon with SVG.
//!
//! At the moment this is mostly a wrapper around the [svgparser](https://crates.io/crates/svgparser)
//! crate.

#![allow(dead_code)]

pub extern crate lyon_path as path;
pub extern crate svgparser as parser;

pub mod path_utils;
