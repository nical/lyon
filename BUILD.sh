#!/bin/sh
mkdir -p bin
rustc $* --out-dir bin src/vodk/main.rs -L extern/glfw-rs/lib -L extern/gl-rs/lib
