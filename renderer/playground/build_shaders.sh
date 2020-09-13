#!/bin/sh
cd ../gpu/
./build_shaders.sh &&
cd ../playground &&
cargo run


