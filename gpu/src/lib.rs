#![feature(macro_rules)]
#![feature(libc)]

extern crate gl;
extern crate libc;
extern crate vodk_data;

pub mod device;
pub mod objects;
pub mod constants;
pub mod std140;
pub mod opengl;
pub mod logging;
