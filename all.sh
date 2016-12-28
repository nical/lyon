#!/bin/sh

echo "building all crates..."
cd ./core && cargo $1 &&
cd ../tessellation && cargo $1 &&
cd ../extra && cargo $1 &&
cd ../path_builder && cargo $1 &&
cd ../path_iterator && cargo $1 &&
cd ../path && cargo $1 &&
cd ../examples/gfx_logo && cargo build &&
echo "...done"
