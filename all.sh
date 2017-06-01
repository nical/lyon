#!/bin/sh

echo "building all crates..."
cd ./core && cargo $1 &&
cd ../tessellation && cargo $1 &&
cd ../extra && cargo $1 &&
cd ../path_builder && cargo $1 &&
cd ../path_iterator && cargo $1 &&
cd ../path && cargo $1 &&
cd ../renderer && cargo $1 &&
cd ../cli && cargo $1 &&
cd ../examples/gfx_basic && cargo build &&
cd ../gfx_advanced && cargo build &&
echo "...done"
