#!/bin/sh
glslangValidator -V ./shaders/quad.vert.glsl -o ./shaders/quad.vert.spv
glslangValidator -V ./shaders/quad.frag.glsl -o ./shaders/quad.frag.spv
glslangValidator -V ./shaders/mesh2d.vert.glsl -o ./shaders/mesh2d.vert.spv
glslangValidator -V ./shaders/mesh2d.frag.glsl -o ./shaders/mesh2d.frag.spv
