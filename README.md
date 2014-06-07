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

Vodk currently depends on this Rust crates.

-    https://github.com/nical/gl-rs
-    https://github.com/nical/glfw-rs
-    https://github.com/nical/rust-png
-    ~~https://github.com/kballard/rust-lua.git~~ not yet.

And the 3.x version of `glfw` downloadable
[here](http://www.glfw.org/download.html).

#### Building deps

To build for the first time, you will have to fetch and build every
dependancies of vodk.rs, that what `SETUP.sh` and `BUILD.sh` do.

~~~~ {.bash}
cd extern
./SETUP.sh
./BUILD.sh
~~~~

Building Vodk
-------------

### Building

    ./BUILD.sh

Troubleshooting(s)
------------------

### Linking with `glfw`

If linking with `glfw` fails, it probably means you don't have the right
version of glfw installed, probably because your distro packages
an older version like Ubuntu 14.04 LTS.

Download the glfw sources from [glfw](http://www.glfw.org/).
(On Linux, build with the cmake argument `-DCMAKE_C_FLAGS=-fPIC`).

