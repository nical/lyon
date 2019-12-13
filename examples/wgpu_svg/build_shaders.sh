#!/bin/sh
glslangValidator -V ./shaders/geometry.vert.glsl -o ./shaders/geometry.vert.spv
glslangValidator -V ./shaders/geometry.frag.glsl -o ./shaders/geometry.frag.spv
