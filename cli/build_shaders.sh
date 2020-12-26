#!/bin/sh
glslangValidator -V ./shaders/geometry.vert.glsl -o ./shaders/geometry.vert.spv
glslangValidator -V ./shaders/geometry.frag.glsl -o ./shaders/geometry.frag.spv
glslangValidator -V ./shaders/background.vert.glsl -o ./shaders/background.vert.spv
glslangValidator -V ./shaders/background.frag.glsl -o ./shaders/background.frag.spv

