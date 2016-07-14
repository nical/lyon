#!/bin/sh

echo "building all crates..."
cd ./core && cargo $1 &&
cd ../tessellation && cargo $1 &&
cd ../extra && cargo $1 &&
cd ../examples/glium_tess && cargo $1 &&
echo "...done"
