#!/bin/sh
echo " -- Building glfw-rs..."
mkdir -p glfw-rs/lib
rustc --out-dir glfw-rs/lib glfw-rs/src/lib/lib.rs
echo " -- Building gl-rs..."
mkdir -p gl-rs/lib
rustc --out-dir gl-rs/lib gl-rs/src/gl/lib.rs
