vodk.rs
=======

Rust experiments, mostly around game programming related stuff.

Compatible with rust:

    rustc 0.11.0-pre (0935beb 2014-05-29 14:41:42 -0700)

Building
========

First build
-----------

### Dependancies

Vodk currently depends on:

-   https://github.com/bjz/glfw-rs
-   https://github.com/bjz/gl-rs
-   https://github.com/mozilla-servo/rust-png

To build for the first time, you will have to fetch and build every
dependancies of vodk.rs

~~~~ {.bash}
cd extern
./SETUP.sh
./BUILD.sh
~~~~

Building Vodk
-------------

### Building

    ./BUILD.sh

Troubleshooting
---------------

### Linking with `glfw-rs`

If linking `glfw-rs` fails, it probably means you don't have the right
version of glfw installed, probably because your distro packages
an older version. Download the glfw sources from http://www.glfw.org/ .
(On Linux, build with the cmake argument `-DCMAKE_C_FLAGS=-fPIC`).

