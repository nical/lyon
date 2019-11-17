#!/bin/sh
glslangValidator -V ./shaders/geometry.glsl.vert -o ./shaders/geometry.vert.spv
glslangValidator -V ./shaders/geometry.glsl.frag -o ./shaders/geometry.frag.spv
glslangValidator -V ./shaders/background.glsl.vert -o ./shaders/background.vert.spv
glslangValidator -V ./shaders/background.glsl.frag -o ./shaders/background.frag.spv

