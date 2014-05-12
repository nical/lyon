#!/bin/sh
mkdir -p lib
rm lib/*

echo " -- Building glfw-rs..."
rustc --out-dir lib/ glfw-rs/src/lib/lib.rs

echo " -- Building gl-rs..."
rustc --out-dir lib gl-rs/src/gl/lib.rs

echo " -- Building png..."
rustc --out-dir lib --opt-level=3 png/lib.rs -L png/
cp png/libshim.a lib/