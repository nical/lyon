vodk.rs
=======

Rust experiments, mostly around game programming related stuff.


# First build

To build for the first time, go into the extern directory and run SETUP.sh.
This scripto will fetch the dependencies and build them.
Vodk currently depends on:
- https://github.com/bjz/glfw-rs
- https://github.com/bjz/gl-rs
- https://github.com/mozilla-servo/rust-png

If linking glfw-rs fails, it probably means you don't have the rightservo's  version of glfw installed, probably because your distro packages an older version. Download the glfw sources from http://www.glfw.org/ (On Linux, build with the cmake argument -DCMAKE_C_FLAGS=-fPIC).

# Build

run ./BUILD.sh